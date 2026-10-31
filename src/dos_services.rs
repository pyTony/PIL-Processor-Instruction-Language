//! DOS compatibility contracts. CPU execution and host output are separate.
use super::*;
use crate::host_io::Output;
use std::time::{SystemTime,UNIX_EPOCH};
pub(super) fn dispatch(p:&mut Loaded,vector:u8,out:&mut impl Output)->Result<Option<u8>,String>{
    if vector==0x21&&aux_console(p)?{return Ok(None);}
    match(vector,(p.cpu.regs[0]>>8)as u8){
        (0x20,_)=>if p.selectors[SegmentRegister::Cs.index()]==p.psp{Ok(Some(0))}else{Err("INT20 requires CS=PSP in this shim".into())},
        (0x21,0x02)=>{console::dos_break_check(p)?;put_char(p,out,p.cpu.regs[2]as u8)?;Ok(None)},
        (0x21,ah @ (0x01|0x07|0x08))=>{read_char(p,out,ah)?;Ok(None)},
        (0x21,0x06)=>{direct_console(p,out)?;Ok(None)},
        (0x21,0x0b)=>{
            console::dos_break_check(p)?;
            let ready=console::dos_available(p)?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|if ready{0xff}else{0};Ok(None)
        },
        (0x21,0x0c)=>{flush_then(p,out)?;Ok(None)},
        (0x21,0x0a)=>{console::dos_break_check(p)?;line_input(p,out)?;Ok(None)},
        (0x21,0x37)=>{
            if p.cpu.regs[0]as u8!=0{return Err("DOS switch-character shim supports AH37/AL00 query only".into());}
            p.cpu.regs[0]&=!255;p.cpu.regs[2]=(p.cpu.regs[2]&!255)|u32::from(b'/');
            Ok(None)
        },
        (0x21,0x25)=>{
            let address=(p.cpu.regs[0]&255)*4;
            let pointer=(u32::from(p.image.selectors[SegmentRegister::Ds.index()])<<16)|(p.cpu.regs[2]&0xffff);
            p.ram.store(address,MemWidth::Dword,pointer).map_err(|e|format!("DOS vector write: {e:?}"))?;
            Ok(None)
        },
        (0x21,0x35)=>{
            let address=(p.cpu.regs[0]&255)*4;
            let pointer=p.ram.load(address,MemWidth::Dword).map_err(|e|format!("DOS vector read: {e:?}"))?;
            let selector=(pointer>>16)as u16;
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|(pointer&0xffff);
            let es=SegmentRegister::Es.index();
            p.image.selectors[es]=selector;p.selectors[es]=selector;
            p.image.segments[es]=SegmentCache::real_mode(selector);
            Ok(None)
        },
        (0x21,0x1a)=>{
            // Store the far pointer without dereferencing the caller's buffer.
            p.dta=(p.image.selectors[SegmentRegister::Ds.index()],p.cpu.regs[2]as u16);
            Ok(None)
        },
        (0x21,0x2f)=>{
            let (segment,offset)=p.dta;
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|u32::from(offset);
            let es=SegmentRegister::Es.index();
            p.selectors[es]=segment;p.image.selectors[es]=segment;
            p.image.segments[es]=SegmentCache::real_mode(segment);
            Ok(None)
        },
        (0x21,0x30)=>{
            let version=p.ram.load(u32::from(p.psp)*16+0x40,MemWidth::Word).map_err(|e|format!("DOS version PSP read: {e:?}"))?;
            let flags=p.cpu.regs[0]as u8==1;
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|version;
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|if flags{0}else{0xff00};
            p.cpu.regs[1]&=0xffff0000;
            Ok(None)
        },
        (0x21,0x33) if p.cpu.regs[0]as u8<=2=>{
            // BREAK flag: AL=0 gets it in DL, AL=1 sets it from DL, AL=2 does
            // both. It is only kept: Dine checks Ctrl-Break on no DOS call.
            let new=p.cpu.regs[2]&1!=0;let old=p.break_check;
            if p.cpu.regs[0]as u8!=0{p.break_check=new;}
            if p.cpu.regs[0]as u8!=1{p.cpu.regs[2]=(p.cpu.regs[2]&!255)|u32::from(old);}
            Ok(None)
        },
        (0x21,0x33) if p.cpu.regs[0]as u8==5=>{
            // Boot drive: the first drive Dine has (C: for an image or a
            // mounted folder, A: when there is neither), as DL=1 for A:.
            p.cpu.regs[2]=(p.cpu.regs[2]&!255)|u32::from(p.drive.first_letter()-b'A'+1);
            Ok(None)
        },
        (0x21,0x33) if p.cpu.regs[0]as u8==6=>{
            // True version: the same 6.22 profile as AH30, revision 0, and
            // DH=0 because Dine is neither in ROM nor in the HMA.
            let version=p.ram.load(u32::from(p.psp)*16+0x40,MemWidth::Word).map_err(|e|format!("DOS true version PSP read: {e:?}"))?;
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|version;
            p.cpu.regs[2]&=0xffff0000;
            Ok(None)
        },
        (0x21,0x67)=>{
            // Set handle count, as FreeDOS SetJFTSize does: a count that fits
            // the current table only updates PSP:32h; a larger one allocates a
            // new table (AH48 rules), copies the old entries, fills the rest
            // with FFh and points PSP:34h at it. No memory: CF=1, AX=8.
            let psp=u32::from(p.psp)*16;let load=|p:&Loaded,at:u32|p.ram.load(at,MemWidth::Word).map_err(|e|format!("DOS JFT PSP read: {e:?}"));
            let wanted=p.cpu.regs[3]&0xffff;let current=load(p,psp+0x32)?;
            if wanted>current{
                let saved=(p.cpu.regs[0],p.cpu.regs[3]);
                p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|(wanted+15)>>4;memory_service(p,0x48)?;
                let segment=p.cpu.regs[0]&0xffff;p.cpu.regs[3]=saved.1;
                if p.cpu.eflags&1!=0{p.cpu.regs[0]=(saved.0&0xffff0000)|8;return Ok(None);}
                p.cpu.regs[0]=saved.0;
                let old=jft(p,0)?;
                for n in 0..wanted{
                    let entry=if n<current{p.ram.load(old+n,MemWidth::Byte).map_err(|e|format!("DOS JFT read: {e:?}"))?}else{0xff};
                    p.ram.store(segment*16+n,MemWidth::Byte,entry).map_err(|e|format!("DOS JFT write: {e:?}"))?;
                }
                p.ram.store(psp+0x34,MemWidth::Word,0).map_err(|e|format!("DOS JFT PSP write: {e:?}"))?;
                p.ram.store(psp+0x36,MemWidth::Word,segment).map_err(|e|format!("DOS JFT PSP write: {e:?}"))?;
            }
            p.ram.store(psp+0x32,MemWidth::Word,wanted).map_err(|e|format!("DOS JFT PSP write: {e:?}"))?;
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,ah @ (0x34|0x52))=>{
            // ES:BX -> InDOS byte (critical-error byte just before it) or the
            // list of lists, in Dine's DOS data segment.
            let es=SegmentRegister::Es.index();
            p.selectors[es]=DOSDATA;p.image.selectors[es]=DOSDATA;p.image.segments[es]=SegmentCache::real_mode(DOSDATA);
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|u32::from(if ah==0x34{INDOS}else{SYSVARS});
            Ok(None)
        },
        (0x21,0x4c)=>Ok(Some(p.cpu.regs[0]as u8)),
        (0x21,0x4b)=>{exec::exec(p)?;Ok(None)},
        (0x21,0x4d)=>{
            // The last child's return code (AH = how it ended), read once.
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|u32::from(std::mem::take(&mut p.return_code));
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,0x3f) if console_handle(p)=>{console_read(p,out)?;Ok(None)},
        (0x21,0x3f) if aux_handle(p)=>{
            let(offset,wanted)=(p.cpu.regs[2]&0xffff,p.cpu.regs[1]&0xffff);
            if offset+wanted>0x10000{return Err("DOS read buffer exceeds segment end".into());}
            if wanted==0{p.cpu.regs[0]&=0xffff0000;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;return Ok(None);}
            match serial::read(p,wanted as usize)?{
                None=>p.input_wait=true,
                Some(bytes)=>{
                    let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
                    for(n,&b)in bytes.iter().enumerate(){
                        let addr=context.translate(offset+n as u32,MemWidth::Byte,true,p.cpu.eip).map_err(|e|format!("DOS read buffer access: {e:?}"))?;
                        p.ram.store(addr,MemWidth::Byte,u32::from(b)).map_err(|e|format!("DOS read buffer write: {e:?}"))?;
                    }
                    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|bytes.len()as u32;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
                },
            }
            Ok(None)
        },
        (0x14,ah) if p.com1.is_some()=>{serial::bios(p,ah)?;Ok(None)},
        (0x21,0x44) if p.cpu.regs[0]as u8==1&&console_handle(p)=>{
            // The console keeps its cooked mode: setting the information it
            // already has (C3h, as CTTY CON does) is taken, raw mode is not.
            if p.cpu.regs[2]&0xffff|0x80!=0xc3{return Err(format!("DOS console device information {:04x}h (raw mode) is not modelled",p.cpu.regs[2]&0xffff));}
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x21,0x44) if p.cpu.regs[0]as u8==1&&aux_handle(p)=>{
            // Set device information: DH must be 0; the device bit stays set.
            if p.cpu.regs[2]&0xff00!=0{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|1;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;return Ok(None);}
            if let Some(port)=p.com1.as_mut(){port.info=p.cpu.regs[2]as u8|0x80;}
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x21,ah @ (0x48|0x49|0x4a))=>{memory_service(p,ah)?;Ok(None)},
        (0x21,9)=>{
            let offset=p.cpu.regs[2]&0xffff;let mut text=Vec::new();
            let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
            for n in offset..=0xffff{
                let addr=context.translate(n,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS string access: {e:?}"))?;
                let b=p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS string RAM access: {e:?}"))?as u8;
                if b==b'$'{
                    console::dos_break_check(p)?;
                    if to_file(p,&text)?{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|0x24;return Ok(None);}
                    let screen=console::dos_screen(p,&text)?;
                    out.write_atomic(&text).map_err(|e|format!("DOS string output provider: {e:?}"))?;
                    console::store(p,&screen)?;p.cpu.regs[0]=(p.cpu.regs[0]&!255)|0x24;return Ok(None);
                }
                if text.len()>=out.remaining(){return Err("DOS output limit reached before string terminator".into());}
                text.push(b);
            }Err("DOS string has no $ terminator before segment end".into())
        },
        (0x10,ah @ (0x08|0x09|0x0a))=>{console::video(p,ah)?;Ok(None)},
        (0x10,ah @ (0x00|0x05|0x06|0x07|0x0e|0x0f))=>{console::display(p,ah)?;Ok(None)},
        (0x16,ah)=>{console::keyboard(p,ah)?;Ok(None)},
        (0x10,0x01)=>{
            p.ram.store(0x460,MemWidth::Word,p.cpu.regs[1]&0xffff).map_err(|e|format!("BIOS cursor shape write: {e:?}"))?;
            Ok(None)
        },
        (0x10,0x02)=>{
            let page=(p.cpu.regs[3]>>8)&255;
            if page>=8{return Err("BIOS cursor page must be 0..7".into());}
            p.ram.store(0x450+page*2,MemWidth::Word,p.cpu.regs[2]&0xffff).map_err(|e|format!("BIOS cursor position write: {e:?}"))?;
            Ok(None)
        },
        (0x10,0x03)=>{
            let page=(p.cpu.regs[3]>>8)&255;
            if page>=8{return Err("BIOS cursor page must be 0..7".into());}
            let shape=p.ram.load(0x460,MemWidth::Word).map_err(|e|format!("BIOS cursor shape read: {e:?}"))?;
            let position=p.ram.load(0x450+page*2,MemWidth::Word).map_err(|e|format!("BIOS cursor position read: {e:?}"))?;
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|shape;
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|position;
            Ok(None)
        },
        (0x11,_)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x0025;
            Ok(None)
        },
        (0x12,_)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x0280;
            Ok(None)
        },
        (0x15,0xc2)=>{
            // No PS/2 pointing device is modelled: answer as a BIOS without one
            // (SeaBIOS: CF=1, AH=86h function not supported).
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x33,_) if p.cpu.regs[0]&0xffff==0=>{
            // No mouse driver is loaded: the reset reports AX=0 (not installed)
            // and BX=0 buttons, as MOUSE.COM-less DOS does where INT 33h points
            // at an IRET and callers have zeroed AX.
            p.cpu.regs[0]&=0xffff0000;p.cpu.regs[3]&=0xffff0000;
            Ok(None)
        },
        (0x1a,0x00)=>{
            let (ticks,rollover)=bios_ticks(p)?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|rollover;
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|((ticks>>16)&0xffff);
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|(ticks&0xffff);
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,0x40)=>{write_handle(p,out)?;Ok(None)},
        (0x21,ah @ (0x38|0x65))=>{nls::service(p,ah)?;Ok(None)},
        (0x21,0x44) if p.cpu.regs[0]as u8==0=>{files::service(p,0x44)?;Ok(None)},
        (0x21,ah @ (0x0e|0x19|0x29|0x36|0x45|0x46|0x39|0x3a|0x3b|0x3c|0x3d|0x3e|0x3f|0x41|0x42|0x43|0x47|0x4e|0x4f|0x56|0x57|0x5a|0x5b|0x60|0x69|0x73))=>{files::service(p,ah)?;Ok(None)},
        (0x21,0x51|0x62)=>{
            // DOS 3+: AH51 and AH62 both return the current PSP segment.
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|u32::from(p.psp);
            Ok(None)
        },
        (0x2f,_) if matches!(p.cpu.regs[0]&0xffff,0x0100|0x0600|0x1100|0x1600|0x1680|0x1687|0x1a00|0x4300|0xae00|0xb700|0xb710)=>{
            // Nothing hooks the multiplex chain, so these installation checks
            // reach the kernel default, which returns AX unchanged (FreeDOS
            // int2f.asm; RBIL): no PRINT, ASSIGN, APPEND, redirector, Windows, DPMI host, ANSI.SYS or
            // XMS driver, and no installable command claims AX=AE00h (AL stays 00h).
            // AX=1680h leaves AL=80h (time slice not released).
            Ok(None)
        },
        (0x10,0x10|0x11|0x12|0x1a|0x1b|0x1c|0x4f)=>{
            // CGA-class video BIOS: no EGA/VGA palette (10h), font (11h),
            // alternate (12h), display combination (1Ah) or state (1Bh/1Ch)
            // functions and no VBE. The AT BIOS returns from these unchanged
            // (10h-12h are reserved entries, the rest are past its table), so
            // BL=10h stays 10h (no EGA), AX=1A00h does not return AL=1Ah (no
            // VGA) and AX=4F00h does not return AL=4Fh.
            Ok(None)
        },
        (0x15,0xe8)=>{
            // E801h/E820h memory maps postdate this AT-class BIOS: unsupported.
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,0x2c)=>{
            // DOS reads the BIOS tick count (one INT 1Ah AH=00h read) and scales it
            // by 65536*5/59659 to hundredths, as the FreeDOS clock driver does.
            let (ticks,_)=bios_ticks(p)?;
            let scaled=5*ticks;let hs=((scaled/59659)<<16)+((scaled%59659)<<16)/59659;
            let (minutes,rest)=(hs/6000,hs%6000);
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|((minutes/60)<<8)|(minutes%60);
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|((rest/100)<<8)|(rest%100);
            Ok(None)
        },
        (0x15,0xc0)=>{
            // No system configuration table: CF=1, AH=86h as for AH=C2.
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,0x0d)=>Ok(None), // No disk buffers are modelled, so the flush has nothing to write.
        (0x21,0x31)=>{
            // Terminate and stay resident: the run ends here, as for AH4C, and the
            // report records the paragraphs kept (at least 6, as FreeDOS does).
            p.resident=Some((p.cpu.regs[2]as u16).max(6));
            Ok(Some(p.cpu.regs[0]as u8))
        },
        (0x21,0x2a)=>{
            // Without the host clock there is no clock driver or CMOS: DOS keeps
            // its 1 January 1980 default, a Tuesday (FreeDOS systime.c: day of
            // week (days+2)%7 with days=0). With it, the host's local date.
            let(year,month,day,weekday)=match p.clock_offset{
                Some(offset)=>{
                    let(days,_)=local_now(offset);
                    let(_,date)=files::stamp(Some(UNIX_EPOCH+std::time::Duration::from_secs(days*86400)));
                    (1980+u32::from(date>>9),u32::from(date>>5&15),u32::from(date&31),((days+4)%7)as u32)
                },
                None=>(1980,1,1,2),
            };
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|year;
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|month<<8|day;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|weekday;
            Ok(None)
        },
        (0x21,0x58)=>{
            let al=p.cpu.regs[0]as u8;let bl=p.cpu.regs[3]as u8;
            match al{
                0=>{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|u32::from(p.alloc_strategy);p.cpu.eflags&=!1;},
                1 if bl>0x82||(bl&0x3f)>2=>{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|1;p.cpu.eflags|=1;},
                // With no UMB chain linked, FreeDOS DosMemAlloc searches low
                // memory for 4xh (high only) and 8xh (high first) with the same fit.
                1=>{p.alloc_strategy=bl;p.cpu.eflags&=!1;},
                2=>{p.cpu.regs[0]&=!255;p.cpu.eflags&=!1;},
                // No UMB chain exists, so linking it is an invalid function (FreeDOS).
                _=>{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|1;p.cpu.eflags|=1;},
            }
            p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x21,0x71)=>{
            // No long-file-name API: AL=00h, CF=1, so AX=7100h (FreeDOS without LFN).
            p.cpu.regs[0]&=!255;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x15,0x88)=>{
            // No memory above 1 MB: 0 KB of extended memory.
            p.cpu.regs[0]&=0xffff0000;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        _=>Err(format!("unimplemented DOS/BIOS service INT {vector:02x}, AH={:02x}",(p.cpu.regs[0]>>8)&255)),
    }
}
/// Days since 1970-01-01 and milliseconds into the day, in the host's local
/// time `offset` seconds east of UTC.
pub(super) fn local_now(offset:i64)->(u64,u64){
    let ms=SystemTime::now().duration_since(UNIX_EPOCH).map_or(0,|d|d.as_millis()as i64)+offset*1000;
    let ms=ms.max(0)as u64;(ms/86_400_000,ms%86_400_000)
}
/// One INT 1Ah AH=00h read. With the host clock, the count is the host's
/// local time of day at 1193180/65536 ticks a second, and rollover is set when
/// midnight passed since the last read. Otherwise it is the current count,
/// then the deterministic clock advances one tick. Either way the rollover
/// flag is consumed by this read.
fn bios_ticks(p:&mut Loaded)->Result<(u32,u32),String>{
    let(ticks,next_ticks,rollover)=match p.clock_offset{
        Some(offset)=>{
            let(_,ms)=local_now(offset);let now=((ms*1_193_180)/(65_536*1000))as u32;
            (now,now,u32::from(now<p.timer_ticks))
        },
        None=>{
            let ticks=p.timer_ticks;let next=ticks+1;
            if next>=TICKS_PER_DAY{(ticks,0,1)}else{(ticks,next,0)}
        },
    };
    // Validate the entire service write set before committing either cell.
    p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    p.ram.store(0x046c,MemWidth::Dword,ticks).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.store(0x0470,MemWidth::Byte,0).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    p.timer_ticks=next_ticks;
    Ok((ticks,rollover))
}
/// AH40 on the CON handles 0..2 writes CX bytes from DS:DX like AH09, without
/// a terminator, and on a file writes them there. AUX/PRN (3..4) are not
/// modelled; a closed handle is error 6.
/// Linear address of a handle's entry in the job file table PSP:34h points at.
pub(super) fn jft(p:&Loaded,handle:u16)->Result<u32,String>{
    let psp=u32::from(p.psp)*16;let word=|at|p.ram.load(at,MemWidth::Word).map_err(|e|format!("DOS JFT pointer read: {e:?}"));
    Ok(word(psp+0x36)?*16+word(psp+0x34)?+u32::from(handle))
}
fn write_handle(p:&mut Loaded,out:&mut impl Output)->Result<(),String>{
    let handle=p.cpu.regs[3]&0xffff;
    let Some(sft)=files::system_file(p,handle as u16)? else{
        p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|6;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
        return Ok(());
    };
    if sft==4||(sft==3&&p.com1.is_none()){return Err(format!("DOS write to handle {handle} (AUX/PRN) is not modelled"));}
    let offset=p.cpu.regs[2]&0xffff;let count=p.cpu.regs[1]&0xffff;
    if offset+count>0x10000{return Err("DOS write buffer exceeds segment end".into());}
    if sft<3&&count as usize>out.remaining(){return Err("DOS output limit reached before handle write".into());}
    let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
    let mut text=Vec::with_capacity(count as usize);
    for n in offset..offset+count{
        let addr=context.translate(n,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS write buffer access: {e:?}"))?;
        text.push(p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS write buffer RAM access: {e:?}"))?as u8);
    }
    if sft==3{
        let written=serial::write(p,&text)?;
        p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|written;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;return Ok(());
    }
    if sft>=5{
        match files::write(p,sft,&text)?{
            Ok(written)=>{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|written;p.cpu.eflags&=!1;},
            Err(code)=>{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|u32::from(code);p.cpu.eflags|=1;},
        }
        p.cpu.undefined_flags&=!1;return Ok(());
    }
    console::dos_break_check(p)?;let screen=console::dos_screen(p,&text)?;
    out.write_atomic(&text).map_err(|e|format!("DOS handle output provider: {e:?}"))?;
    console::store(p,&screen)?;
    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|count;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
    Ok(())
}
/// AH02/06/09 output goes to the file handle 1 was redirected to, if any.
/// DOS has no error return there, so a failed write stops the run.
fn to_file(p:&mut Loaded,bytes:&[u8])->Result<bool,String>{
    if p.com1.is_some()&&files::system_file(p,1)?==Some(3){serial::write(p,bytes)?;return Ok(true);}
    let Some(sft)=files::redirected_output(p)? else{return Ok(false)};
    match files::write(p,sft,bytes)?{Ok(n) if n as usize==bytes.len()=>Ok(true),Ok(_)=>Err("DOS console output to redirected handle 1 found the drive full".into()),Err(code)=>Err(format!("DOS console output to redirected handle 1 failed with error {code}"))}
}
/// AH02 (and AH06 output): capture and active-page screen change together.
fn put_char(p:&mut Loaded,out:&mut impl Output,byte:u8)->Result<(),String>{
    if to_file(p,&[byte])?{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(byte);return Ok(());}
    let count=if byte==9{8-out.column()%8}else{1};
    if count>out.remaining(){return Err("DOS output limit reached before character output".into());}
    let value=if byte==9{b' '}else{byte};
    let spaces=[b' ';8];let single=[byte];let bytes=if byte==9{&spaces[..count]}else{&single[..]};
    let screen=console::dos_screen(p,bytes)?;
    out.write_atomic(bytes).map_err(|e|format!("DOS character output provider: {e:?}"))?;
    console::store(p,&screen)?;
    p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);
    Ok(())
}
/// AH01 echoes (including the 00h of an extended key); AH01/AH08 check Ctrl-C;
/// AH06/AH07 return it as 03h. The caller already waited for input.
fn read_char(p:&mut Loaded,out:&mut impl Output,ah:u8)->Result<(),String>{
    if matches!(ah,1|8){console::dos_break_check(p)?;}
    let(byte,ring,pending)=console::dos_take(p)?;
    let mut writes=ring;
    if ah==1{
        if out.remaining()==0{return Err("DOS output limit reached before input echo".into());}
        writes.extend(console::dos_screen(p,&[byte])?);
    }
    console::check(p,&writes)?;
    if ah==1{out.write_atomic(&[byte]).map_err(|e|format!("DOS echo output provider: {e:?}"))?;}
    console::store(p,&writes)?;p.pending_scan=pending;
    p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(byte);Ok(())
}
/// AH06: DL=FF polls without waiting or a Ctrl-C check. Other DL values output
/// like AH02 without its Ctrl-C check, as the FreeDOS handler does.
fn direct_console(p:&mut Loaded,out:&mut impl Output)->Result<(),String>{
    if p.cpu.regs[2]as u8!=0xff{return put_char(p,out,p.cpu.regs[2]as u8);}
    p.cpu.undefined_flags&=!0x40;
    if !console::dos_available(p)?{p.cpu.regs[0]&=!255;p.cpu.eflags|=0x40;return Ok(());}
    read_char(p,out,6)?;p.cpu.eflags&=!0x40;Ok(())
}
/// AH0C flushes type-ahead, then runs AL=01/06/07/08; other AL values return 00h.
/// Waiting reads were flushed before their first wait (see console::waiting).
fn flush_then(p:&mut Loaded,out:&mut impl Output)->Result<(),String>{
    let al=p.cpu.regs[0]as u8;
    match al{
        1|7|8=>{
            if !p.flushed{return Err("DOS AH0C read reached without its flush".into());}
            read_char(p,out,al)?;p.flushed=false;Ok(())
        },
        0x0a=>{
            if !p.flushed{
                let flush=console::dos_flush_writes(p)?;console::store(p,&flush)?;
                p.pending_scan=None;p.flushed=true;
            }
            line_input(p,out)?;
            if !p.input_wait{p.flushed=false;}Ok(())
        },
        _=>{
            let flush=console::dos_flush_writes(p)?;
            let saved=(p.ram.clone(),p.cpu.clone(),p.pending_scan);
            console::store(p,&flush)?;p.pending_scan=None;
            let result=if al==6{direct_console(p,out)}else{p.cpu.regs[0]&=!255;Ok(())};
            if result.is_err(){(p.ram,p.cpu,p.pending_scan)=saved;}
            result
        },
    }
}
const F1:u32=0x3b00;const F5:u32=0x3f00;const F6:u32=0x4000;
const INS:u32=0x5200;const DEL:u32=0x5300;const LEFT:u32=0x4b00;const RIGHT:u32=0x4d00;
/// Echo `bytes` to capture and screen and apply `writes`, all or nothing.
fn echo(p:&mut Loaded,out:&mut impl Output,bytes:&[u8],mut writes:Vec<(u32,MemWidth,u32)>)->Result<(),String>{
    if bytes.len()>out.remaining(){return Err("DOS output limit reached during line input".into());}
    writes.extend(console::dos_screen(p,bytes)?);console::check(p,&writes)?;
    out.write_atomic(bytes).map_err(|e|format!("DOS line echo provider: {e:?}"))?;
    console::store(p,&writes)
}
/// AH0A buffered input at DS:DX (max, count, bytes), after FreeDOS read_line.
/// Keys are consumed and echoed as they arrive; with none left before CR the
/// line state stays in Loaded and the INT waits. The guest buffer is written
/// once, at CR: the bytes including CR, and a count without it. Template keys
/// (F1-F5, RIGHT, INS, DEL) are rejected; other extended keys are ignored.
fn line_input(p:&mut Loaded,out:&mut impl Output)->Result<(),String>{
    let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
    line_into(p,out,context,p.cpu.regs[2]&0xffff)
}
/// The AH0A editor on the buffer at `context`:`dx`.
fn line_into(p:&mut Loaded,out:&mut impl Output,context:MemoryContext,dx:u32)->Result<(),String>{
    let eip=p.cpu.eip;
    let at=move|n:u32|->Result<u32,String>{
        if dx+n>0xffff{return Err("DOS line buffer exceeds segment end".into());}
        context.translate(dx+n,MemWidth::Byte,false,eip).map_err(|e|format!("DOS line buffer access: {e:?}"))
    };
    let size=p.ram.load(at(0)?,MemWidth::Byte).map_err(|e|format!("DOS line buffer read: {e:?}"))?as usize;
    if size==0{p.line=None;return Ok(());}
    at(1+size as u32)?; // the whole buffer must lie inside DS before any key is taken
    let(mut line,cu_pos,mut first)=p.line.take().unwrap_or((Vec::new(),out.column(),true));
    let result=line_edit(p,out,&at,size,&mut line,cu_pos,&mut first);
    if p.input_wait||result.is_err(){p.line=Some((line,cu_pos,first));}
    result
}

fn line_edit(p:&mut Loaded,out:&mut impl Output,at:&dyn Fn(u32)->Result<u32,String>,size:usize,line:&mut Vec<u8>,cu_pos:usize,first:&mut bool)->Result<(),String>{
    loop{
        console::dos_break_check(p)?;
        let Some((c,ring))=console::dos_line_key(p)? else{p.input_wait=true;return Ok(());};
        let clear=p.pending_scan.is_some();
        let mut c=c;if c==F6{c=0x1a;}
        match c{
            10=>{echo(p,out,if *first{&[]}else{b"\r\n"},ring)?;},
            6=>echo(p,out,&[],ring)?,
            F1..=F5|INS|DEL|RIGHT=>return Err(format!("DOS line template key {:02x} is not modeled",c>>8)),
            8|LEFT=>{
                let mut bs=Vec::new();
                if let Some(last)=line.pop(){
                    let n=if last==9{
                        let mut col=cu_pos;
                        for &b in line.iter(){col=if b==9{(col+8)&!7}else if b<32{col+2}else{col+1};}
                        out.column().saturating_sub(col).max(1)
                    }else if last<32{2}else{1};
                    for _ in 0..n{bs.extend_from_slice(b"\x08 \x08");}
                }
                echo(p,out,&bs,ring)?;
            },
            27=>{
                let mut e=b"\\\r\n".to_vec();e.extend(std::iter::repeat_n(b' ',cu_pos));
                echo(p,out,&e,ring)?;line.clear();
            },
            c if c>=256=>echo(p,out,&[],ring)?,
            13=>{
                line.push(13);
                let mut writes=ring;
                for (i,&b) in line.iter().enumerate(){writes.push((at(2+i as u32)?,MemWidth::Byte,u32::from(b)));}
                writes.push((at(1)?,MemWidth::Byte,(line.len()-1)as u32));
                echo(p,out,b"\r",writes)?;
                if clear{p.pending_scan=None;}
                return Ok(());
            },
            c=>{
                let b=c as u8;
                if line.len()+1<size{
                    let shown:&[u8]=if b<32&&b!=9{&[b'^',b+64]}else{&[b]};
                    echo(p,out,shown,ring)?;line.push(b);
                }else{echo(p,out,&[7],ring)?;}
            },
        }
        if clear{p.pending_scan=None;}
        *first=false;
    }
}
/// Handles on CON: system files 0..2, also through AH45/46 copies.
/// Whether standard input is the AUX device on COM1, as after CTTY COM1.
pub(super) fn stdin_aux(p:&Loaded)->bool{
    p.com1.is_some()&&matches!(files::system_file(p,0),Ok(Some(3)))
}
/// The DOS console input calls while standard input is COM1 (after CTTY
/// COM1): they read the bytes the host terminal sent, echo to standard output
/// where DOS echoes, and wait while none have come. AH0A edits with Backspace
/// only. Returns false for every other call.
fn aux_console(p:&mut Loaded)->Result<bool,String>{
    let(ah,al,dl)=((p.cpu.regs[0]>>8)as u8,p.cpu.regs[0]as u8,p.cpu.regs[2]as u8);
    if !stdin_aux(p)||!matches!(ah,0x01|0x06|0x07|0x08|0x0a|0x0b|0x0c){return Ok(false);}
    if ah==0x06&&dl!=0xff{return Ok(false);}
    let(ah,dx)=if ah==0x0c{
        if !matches!(al,0x01|0x06|0x07|0x08|0x0a){p.cpu.regs[0]&=!255;return Ok(true);}
        // AH0C empties the input, then reads as AL says.
        if !p.flushed{if let Some(port)=p.com1.as_mut(){port.rx.clear();}p.flushed=true;}
        (al,p.cpu.regs[2]&0xffff)
    }else{(ah,p.cpu.regs[2]&0xffff)};
    let set_al=|p:&mut Loaded,v:u8|p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(v);
    let take=|p:&mut Loaded|p.com1.as_mut().and_then(|s|s.rx.pop_front());
    match ah{
        0x0b=>{let ready=p.com1.as_ref().is_some_and(|s|!s.rx.is_empty());set_al(p,if ready{0xff}else{0});},
        0x06=>match take(p){
            Some(b)=>{set_al(p,b);p.cpu.eflags&=!0x40;},
            None=>{set_al(p,0);p.cpu.eflags|=0x40;},
        },
        0x01|0x07|0x08=>match take(p){
            Some(b)=>{set_al(p,b);if ah==1{to_file(p,&[b])?;}p.flushed=false;},
            None=>p.input_wait=true,
        },
        _=>{
            // AH0A: DS:DX holds the size, then the count and the line.
            let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
            let eip=p.cpu.eip;
            let at=|n:u32|->Result<u32,String>{
                if dx+n>0xffff{return Err("DOS line buffer exceeds segment end".into());}
                context.translate(dx+n,MemWidth::Byte,true,eip).map_err(|e|format!("DOS line buffer access: {e:?}"))
            };
            let size=p.ram.load(at(0)?,MemWidth::Byte).map_err(|e|format!("DOS line buffer read: {e:?}"))?as usize;
            if size==0{return Ok(true);}
            at(1+size as u32)?;
            let mut line=p.line.take().map_or(Vec::new(),|(line,_,_)|line);
            loop{
                let Some(b)=take(p)else{p.line=Some((line,0,false));p.input_wait=true;return Ok(true)};
                match b{
                    b'\r'=>{
                        to_file(p,b"\r")?;
                        let mut bytes=vec![line.len()as u8];bytes.extend(&line);bytes.push(b'\r');
                        for(n,&v)in bytes.iter().enumerate(){p.ram.store(at(1+n as u32)?,MemWidth::Byte,u32::from(v)).map_err(|e|format!("DOS line buffer write: {e:?}"))?;}
                        break;
                    },
                    8|0x7f=>if line.pop().is_some(){to_file(p,b"\x08 \x08")?;},
                    // A full line takes only CR, as DOS rings the bell.
                    _ if line.len()+1>=size=>{to_file(p,b"\x07")?;},
                    b'\n'=>{},
                    _=>{line.push(b);to_file(p,&[b])?;},
                }
            }
            p.flushed=false;
        },
    }
    Ok(true)
}
/// A handle on the AUX device (system file 3) while COM1 is attached.
fn aux_handle(p:&Loaded)->bool{
    p.com1.is_some()&&matches!(files::system_file(p,(p.cpu.regs[3]&0xffff)as u16),Ok(Some(3)))
}
fn console_handle(p:&Loaded)->bool{
    matches!(files::system_file(p,(p.cpu.regs[3]&0xffff)as u16),Ok(Some(0..=2)))
}
/// AH3F from CON reads cooked lines, as FreeDOS read_line_handle does: a line
/// is edited as AH0A edits it (127 characters), CR echoes LF, and the line
/// with CR LF is handed out over as many reads as the program makes.
fn console_read(p:&mut Loaded,out:&mut impl Output)->Result<(),String>{
    let wanted=(p.cpu.regs[1]&0xffff)as usize;let dx=p.cpu.regs[2]&0xffff;
    if wanted==0{p.cpu.regs[0]&=0xffff0000;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;return Ok(());}
    if dx as usize+wanted>0x10000{return Err("DOS read buffer exceeds segment end".into());}
    if p.con_pending.is_empty(){
        let base=u32::from(DOSDATA)*16+u32::from(CON_LINE);
        if p.line.is_none(){p.ram.store(base,MemWidth::Byte,128).map_err(|e|format!("DOS console line write: {e:?}"))?;}
        let context=MemoryContext{segment:SegmentRegister::Ds,cache:SegmentCache::real_mode(DOSDATA)};
        console::dos_break_check(p)?;
        line_into(p,out,context,u32::from(CON_LINE))?;
        if p.input_wait{return Ok(());}
        let count=p.ram.load(base+1,MemWidth::Byte).map_err(|e|format!("DOS console line read: {e:?}"))?;
        let mut line=(0..=count).map(|i|p.ram.load(base+2+i,MemWidth::Byte).map(|b|b as u8)).collect::<Result<Vec<_>,_>>().map_err(|e|format!("DOS console line read: {e:?}"))?;line.push(10);
        echo(p,out,b"\n",Vec::new())?;
        p.con_pending=line;
    }
    let n=wanted.min(p.con_pending.len());
    let bytes:Vec<u8>=p.con_pending.drain(..n).collect();
    let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
    let addrs=(0..n as u32).map(|i|context.translate(dx+i,MemWidth::Byte,true,p.cpu.eip).map_err(|e|format!("DOS read buffer access: {e:?}"))).collect::<Result<Vec<_>,_>>()?;
    for &a in &addrs{p.ram.check_write(a,MemWidth::Byte).map_err(|e|format!("DOS read buffer write: {e:?}"))?;}
    for(a,&b)in addrs.into_iter().zip(&bytes){p.ram.store(a,MemWidth::Byte,u32::from(b)).map_err(|e|format!("DOS read buffer write: {e:?}"))?;}
    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|n as u32;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
    Ok(())
}
