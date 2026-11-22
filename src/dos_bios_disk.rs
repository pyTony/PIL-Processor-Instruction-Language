//! Raw disk images as BIOS drives: INT 13h on floppies (00h, 01h) and a
//! fixed disk (80h), the diskette parameter table INT 1Eh points to, and the
//! drive types the CMOS reports. This is the machine's firmware, used by a
//! boot sector and the real-mode code it loads, not by Dine's DOS (whose
//! drives are file systems, not sector devices).
//!
//! Function numbers, register use, status codes and the 11-byte parameter
//! table follow the IBM PC AT BIOS technical reference and RBIL INT 13h.
//! Writes change the image held in memory only; the file is never written.
use super::*;

/// Where the diskette parameter table lives in the BIOS area (F000:EFC7, the
/// address the IBM AT BIOS used).
const DPT:u32=0xfefc7;
const DPT_SEGMENT:u16=0xf000;
const DPT_OFFSET:u16=0xefc7;

#[derive(Clone,Debug)]
pub(crate) struct Disk{data:Vec<u8>,cylinders:u32,heads:u32,sectors:u32}
impl Disk{
    /// A floppy image, its geometry from its size (the PC formats).
    pub(crate) fn floppy(data:Vec<u8>)->Result<Self,String>{
        let (cylinders,heads,sectors)=match data.len(){
            163_840=>(40,1,8),184_320=>(40,1,9),327_680=>(40,2,8),368_640=>(40,2,9),
            737_280=>(80,2,9),1_228_800=>(80,2,15),1_474_560=>(80,2,18),2_949_120=>(80,2,36),
            n=>return Err(format!("{n} bytes is not a PC floppy image size (160K-2.88M)")),
        };
        Ok(Self{data,cylinders,heads,sectors})
    }
    /// A fixed-disk image: 16 heads and 63 sectors per track, the usual
    /// BIOS translation, and as many whole cylinders as the image holds.
    pub(crate) fn fixed(data:Vec<u8>)->Result<Self,String>{
        let cylinders=(data.len()/(16*63*512)) as u32;
        if cylinders==0||cylinders>1024{return Err(format!("{} bytes is not a fixed-disk image of 1-1024 cylinders (16 heads, 63 sectors)",data.len()));}
        Ok(Self{data,cylinders,heads:16,sectors:63})
    }
    /// The CMOS drive type of a floppy (register 10h): 1 360K, 2 1.2M,
    /// 3 720K, 4 1.44M, 5 2.88M.
    fn cmos_type(&self)->u8{
        match (self.cylinders,self.sectors){(40,_)=>1,(80,15)=>2,(80,9)=>3,(80,18)=>4,(80,36)=>5,_=>0}
    }
    fn lba(&self,cylinder:u32,head:u32,sector:u32)->Option<u32>{
        (cylinder<self.cylinders&&head<self.heads&&(1..=self.sectors).contains(&sector))
            .then(||(cylinder*self.heads+head)*self.sectors+sector-1)
    }
    fn total(&self)->u32{(self.data.len()/512) as u32}
}

/// The BIOS drives: floppies 00h and 01h, fixed disk 80h.
#[derive(Clone,Debug,Default)]
pub(crate) struct Disks{floppy:[Option<Disk>;2],fixed:Option<Disk>}
impl Disks{
    pub(crate) fn is_empty(&self)->bool{self.floppy.iter().all(Option::is_none)&&self.fixed.is_none()}
    pub(crate) fn attach(&mut self,drive:u8,disk:Disk)->Result<(),String>{
        match drive{0|1=>self.floppy[usize::from(drive)]=Some(disk),0x80=>self.fixed=Some(disk),_=>return Err(format!("BIOS drive {drive:02x}h: only 00h, 01h and 80h")),}
        Ok(())
    }
    fn get(&self,drive:u8)->Option<&Disk>{match drive{0|1=>self.floppy[usize::from(drive)].as_ref(),0x80=>self.fixed.as_ref(),_=>None}}
    fn get_mut(&mut self,drive:u8)->Option<&mut Disk>{match drive{0|1=>self.floppy[usize::from(drive)].as_mut(),0x80=>self.fixed.as_mut(),_=>None}}
    /// CMOS register 10h: drive A: type in the high nibble, B: in the low.
    pub(crate) fn cmos_floppy_types(&self)->u8{
        let t=|d:&Option<Disk>|d.as_ref().map_or(0,Disk::cmos_type);
        t(&self.floppy[0])<<4|t(&self.floppy[1])
    }
    /// The number of floppy drives, for the equipment word.
    pub(crate) fn floppies(&self)->u16{self.floppy.iter().filter(|d|d.is_some()).count() as u16}
    pub(crate) fn has_floppy(&self,drive:u8)->bool{drive<2&&self.floppy[usize::from(drive)].is_some()}
    /// Cylinders, heads and sectors per track of a diskette drive's image.
    pub(crate) fn floppy_geometry(&self,drive:u8)->Option<(u32,u32,u32)>{
        let d=if drive<2{self.floppy[usize::from(drive)].as_ref()}else{None}?;Some((d.cylinders,d.heads,d.sectors))
    }
    pub(crate) fn write_sector(&mut self,drive:u8,lba:u32,data:&[u8])->Result<(),String>{
        let d=self.get_mut(drive).ok_or("no disk in the drive")?;
        let at=lba as usize*512;
        d.data.get_mut(at..at+512).ok_or("sector outside the image")?.copy_from_slice(data);Ok(())
    }
    pub(crate) fn sector(&self,drive:u8,lba:u32)->Option<&[u8]>{
        let d=self.get(drive)?;d.data.get(lba as usize*512..lba as usize*512+512)
    }
}

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
                    p.disks.get_mut(drive).unwrap().data[range].copy_from_slice(&bytes);
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
            let drives=if drive&0x80==0{u32::from(p.disks.floppies())}else{1};
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
        // The disk in the drive has not changed.
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
