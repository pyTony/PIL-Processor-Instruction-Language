//! Raw disk images as BIOS drives: INT 13h on floppies (00h, 01h) and
//! fixed disks (80h, 81h), the diskette parameter table INT 1Eh points to, and the
//! drive types the CMOS reports. This is the machine's firmware, used by a
//! boot sector and the real-mode code it loads, not by Dine's DOS (whose
//! drives are file systems, not sector devices).
//!
//! Function numbers, register use, status codes and the 11-byte parameter
//! table follow the IBM PC AT BIOS technical reference and RBIL INT 13h.
//! Diskette writes change the image held in memory only; a fixed disk read
//! from a file writes each changed sector back to that file.
use super::*;
use crate::pc::disks::Disk;

/// Where the diskette parameter table lives in the BIOS area (F000:EFC7, the
/// address the IBM AT BIOS used).
const DPT:u32=0xfefc7;
const DPT_SEGMENT:u16=0xf000;
const DPT_OFFSET:u16=0xefc7;

/// The diskette parameter table for drive A:'s format (IBM AT values: step
/// rate and head unload DFh, head load and DMA 02h, motor off 25h, 512-byte
/// sectors 02h, sectors per track, gap 1Bh, data length FFh, format gap 54h,
/// fill F6h, head settle 0Fh, motor start 08h), written into the BIOS area
/// and pointed to by INT 1Eh.
pub(super) fn install_table(p:&mut Loaded)->Result<(),String>{
    let sectors=p.disks.floppy[0].as_ref().map_or(18,|d|d.sectors) as u8;
    // The table is BIOS ROM contents: written while the bytes are
    // writable, then ROM again, so a guest store there is dropped.
    let table=[0xdf,0x02,0x25,0x02,sectors,0x1b,0xff,0x54,0xf6,0x0f,0x08];
    let rom=|e|format!("diskette parameter table: {e:?}");
    p.ram.unrom(DPT,table.len()).map_err(rom)?;
    p.ram.write_bytes(DPT,&table).map_err(rom)?;
    p.ram.rom(DPT,table.len()).map_err(rom)?;
    p.ram.store(0x1e*4,MemWidth::Dword,u32::from(DPT_SEGMENT)<<16|u32::from(DPT_OFFSET)).map_err(|e|format!("INT 1Eh vector: {e:?}"))?;
    Ok(())
}

/// The fixed disk parameter tables (F000:E401, where the IBM AT BIOS kept
/// its drive-type table, and the next 16 bytes for the second disk) behind
/// INT 41h (80h) and INT 46h (81h), and the number of fixed disks at
/// 0040:0075. Layout: cylinders (word), heads, reduced write current
/// cylinder (word), write precompensation cylinder (word, FFFFh none), ECC
/// burst, control byte (bit 3: more than 8 heads), three time-outs, landing
/// zone (word), sectors per track, reserved.
const FDPT:u32=0xfe401;
pub(super) fn install_fixed_table(p:&mut Loaded)->Result<(),String>{
    for (unit,vector) in [(0usize,0x41u32),(1,0x46)]{
        let Some((cylinders,heads,sectors))=p.disks.fixed[unit].as_ref().map(Disk::geometry)else{continue};
        let c=cylinders as u16;let mut table=[0u8;16];
        table[0..2].copy_from_slice(&c.to_le_bytes());table[2]=heads as u8;
        table[3..5].copy_from_slice(&c.to_le_bytes());table[5..7].copy_from_slice(&0xffffu16.to_le_bytes());
        table[8]=if heads>8{8}else{0};table[12..14].copy_from_slice(&c.to_le_bytes());table[14]=sectors as u8;
        let at=FDPT+16*unit as u32;
        let rom=|e|format!("fixed disk parameter table: {e:?}");
        p.ram.unrom(at,16).map_err(rom)?;p.ram.write_bytes(at,&table).map_err(rom)?;p.ram.rom(at,16).map_err(rom)?;
        p.ram.store(vector*4,MemWidth::Dword,0xf000_0000|(at-0xf0000)).map_err(|e|format!("INT {vector:02x}h vector: {e:?}"))?;
    }
    let count=p.disks.fixed_count();
    if count>0{p.ram.store(0x475,MemWidth::Byte,u32::from(count)).map_err(|e|format!("BIOS fixed disk count: {e:?}"))?;}
    Ok(())
}

fn finish(p:&mut Loaded,drive:u8,status:u8)->Result<(),String>{
    let at=if drive&0x80==0{0x441}else{0x474};
    p.ram.store(at,MemWidth::Byte,u32::from(status)).map_err(|e|format!("BIOS disk status write: {e:?}"))?;
    p.cpu.regs[0]=(p.cpu.regs[0]&!0xff00)|u32::from(status)<<8;
    if status==0{p.cpu.eflags&=!1;}else{p.cpu.eflags|=1;}
    p.cpu.undefined_flags&=!1;
    Ok(())
}

/// INT 13h with BIOS drives attached.
pub(super) fn int13(p:&mut Loaded)->Result<Option<u8>,String>{
    let ah=(p.cpu.regs[0]>>8)as u8;let al=p.cpu.regs[0]as u8;
    let (cx,dx)=(p.cpu.regs[1] as u16,p.cpu.regs[2] as u16);let drive=dx as u8;
    let present=p.disks.get(drive).is_some();
    // No drive: a timeout (80h) on the diskette side, "bad command" (01h)
    // for a fixed disk the BIOS does not have.
    let missing=if drive&0x80==0{0x80}else{0x01};
    match ah{
        0x00=>finish(p,drive,if present||drive&0x80==0{0}else{missing})?,
        0x01=>{
            let at=if drive&0x80==0{0x441}else{0x474};
            let last=p.ram.load(at,MemWidth::Byte).map_err(|e|format!("BIOS disk status read: {e:?}"))? as u8;
            p.cpu.regs[0]=(p.cpu.regs[0]&!0xffff)|u32::from(last)<<8;
            if last==0{p.cpu.eflags&=!1;}else{p.cpu.eflags|=1;}p.cpu.undefined_flags&=!1;
        },
        0x02|0x03|0x04=>{
            if !present{finish(p,drive,missing)?;return Ok(None);}
            let cylinder=u32::from(cx>>8)|u32::from(cx&0xc0)<<2;
            let sector=u32::from(cx&0x3f);let head=u32::from(dx>>8);let count=u32::from(al);
            let disk=p.disks.get(drive).unwrap();
            let Some(first)=disk.lba(cylinder,head,sector).filter(|_|count>0)else{
                p.cpu.regs[0]&=!0xff;finish(p,drive,0x04)?;return Ok(None)};
            if first+count>disk.total(){p.cpu.regs[0]&=!0xff;finish(p,drive,0x04)?;return Ok(None);}
            let es=u32::from(p.selectors[SegmentRegister::Es.index()]);
            let buffer=(es<<4)+(p.cpu.regs[3]&0xffff);
            // The diskette controller's DMA cannot cross a 64 KiB page; the
            // BIOS refuses such a transfer (09h) and reads nothing.
            if drive&0x80==0&&ah!=0x04&&(buffer&0xffff)+count*512>0x10000{
                p.cpu.regs[0]&=!0xff;finish(p,drive,0x09)?;return Ok(None);
            }
            let range=first as usize*512..(first+count) as usize*512;
            match ah{
                0x02=>{let bytes=disk.data[range].to_vec();p.ram.write_bytes(buffer,&bytes).map_err(|e|format!("INT 13h read into {buffer:05x}h: {e:?}"))?;},
                0x03=>{
                    let bytes:Vec<u8>=(0..count*512).map(|i|p.ram.load(buffer+i,MemWidth::Byte).map(|b|b as u8)).collect::<Result<_,_>>().map_err(|e|format!("INT 13h write from {buffer:05x}h: {e:?}"))?;
                    p.disks.get_mut(drive).unwrap().write(first,&bytes)?;
                },
                _=>{},
            }
            p.cpu.regs[0]=(p.cpu.regs[0]&!0xff)|count;finish(p,drive,0)?;
        },
        0x08=>{
            let Some(disk)=p.disks.get(drive).cloned()else{finish(p,drive,missing)?;return Ok(None)};
            let max_cylinder=disk.cylinders-1;
            let cl=(disk.sectors&0x3f)|(max_cylinder>>2&0xc0);
            p.cpu.regs[1]=(p.cpu.regs[1]&!0xffff)|(max_cylinder&0xff)<<8|cl;
            let drives=u32::from(if drive&0x80==0{p.disks.floppies()}else{u16::from(p.disks.fixed_count())});
            p.cpu.regs[2]=(p.cpu.regs[2]&!0xffff)|(disk.heads-1)<<8|drives;
            if drive&0x80==0{
                p.cpu.regs[3]=(p.cpu.regs[3]&!0xff)|u32::from(disk.cmos_type());
                p.cpu.regs[7]=(p.cpu.regs[7]&!0xffff)|u32::from(DPT_OFFSET);
                load_es(p,DPT_SEGMENT);
            }
            p.cpu.regs[0]&=!0xff;finish(p,drive,0)?;
        },
        0x15=>{
            // Diskette with change line (02h) or fixed disk (03h, CX:DX
            // sectors); AH=00h for no drive. CF clear either way.
            let kind=match p.disks.get(drive){None=>0u32,Some(_) if drive&0x80==0=>2,Some(d)=>{
                p.cpu.regs[1]=(p.cpu.regs[1]&!0xffff)|d.total()>>16;p.cpu.regs[2]=(p.cpu.regs[2]&!0xffff)|(d.total()&0xffff);3}};
            p.cpu.regs[0]=(p.cpu.regs[0]&!0xff00)|kind<<8;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
        },
        // Change line: 06h once after another diskette went in (the BIOS
        // then steps the head, which clears the line).
        0x16 if p.disks.changed(drive)=>{p.disks.stepped(drive);finish(p,drive,6)?;},
        0x16=>finish(p,drive,if present{0}else{missing})?,
        0x17|0x18=>{
            if ah==0x18{p.cpu.regs[7]=(p.cpu.regs[7]&!0xffff)|u32::from(DPT_OFFSET);load_es(p,DPT_SEGMENT);}
            finish(p,drive,if present{0}else{missing})?;
        },
        // Recalibrate, ready test, seek, controller diagnostics: nothing to wait for.
        0x0c|0x0d|0x10|0x11|0x14=>finish(p,drive,if present{0}else{missing})?,
        _=>finish(p,drive,0x01)?,
    }
    Ok(None)
}
fn load_es(p:&mut Loaded,selector:u16){
    let es=SegmentRegister::Es.index();
    p.selectors[es]=selector;p.image.selectors[es]=selector;
    p.image.segments[es]=p.image.segments[es].real_mode_reload(selector);
}

/// Read the boot drive's first sector to 0000:7C00 and start there, with
/// DL the drive, CS=DS=ES=SS=0, SP=7C00h and interrupts on, as the AT BIOS
/// leaves the machine after POST.
pub(super) fn load_boot_sector(p:&mut Loaded,drive:u8)->Result<(),String>{
    let sector=p.disks.sector(drive,0).ok_or("boot drive has no sector 0")?.to_vec();
    p.ram.write_bytes(0x7c00,&sector).map_err(|e|format!("boot sector: {e:?}"))?;
    for i in 0..6{p.selectors[i]=0;p.image.selectors[i]=0;p.image.segments[i]=crate::address::SegmentCache::real_mode(0);}
    p.cpu.regs=[0;8];p.cpu.regs[2]=u32::from(drive);p.cpu.regs[4]=0x7c00;
    p.cpu.eip=0x7c00;p.cpu.eflags=0x202;p.cpu.undefined_flags=0;
    Ok(())
}
/// INT 19h: load the boot sector again and start it. The service returns
/// to the loop like any other, which steps over the two bytes of an INT
/// instruction, so CS:IP is left two bytes before 0000:7C00.
pub(super) fn bootstrap(p:&mut Loaded)->Result<Option<u8>,String>{
    let drive=if p.disks.get(0).is_some(){0}else{0x80};
    load_boot_sector(p,drive)?;
    p.cpu.eip=0x7bfe;
    Ok(None)
}
