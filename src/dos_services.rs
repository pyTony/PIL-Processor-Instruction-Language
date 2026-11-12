//! DOS compatibility contracts. CPU execution and host output are separate.
use super::*;
use crate::host_io::Output;
// Capture only DOS services that return errors in AX with carry set. Other
// calls need not define carry, so it cannot be used as a general error signal.
pub(super) fn dispatch(p:&mut Loaded,vector:u8,out:&mut impl Output)->Result<Option<u8>,String>{
    let ah=(p.cpu.regs[0]>>8)as u8;
    let result=dispatch_inner(p,vector,out);
    if result.is_ok() && vector==0x21 && matches!(ah,0x38..=0x4b|0x4e|0x4f|0x56|0x57|0x5a|0x5b|0x5f|0x60|0x65..=0x67|0x69) && p.cpu.eflags&1!=0 {
        p.dos_error=p.cpu.regs[0]as u16;
    }
    result
}
fn dispatch_inner(p:&mut Loaded,vector:u8,out:&mut impl Output)->Result<Option<u8>,String>{
    let _span=crate::profile::span("dos_bios_service");
    if vector==0x21&&aux_console(p)?{return Ok(None);}
    // DOS checks for Ctrl-C on entry to the character calls, and to every
    // call while BREAK is on (FreeDOS int21_service).
    let ah=(p.cpu.regs[0]>>8)as u8;
    if vector==0x21&&(p.break_check||matches!(ah,1..=5|8..=0x0b)){console::dos_break_check(p)?;}
    match(vector,ah){
        (0x00,_)=>divide_error(p),
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
            // both. While it is on, every INT 21h call checks for Ctrl-C.
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
        (0x21,0x59)=>{
            if p.cpu.regs[3]&0xffff!=0{return Err("DOS extended error AH59 requires BX=0000h".into());}
            // Match DOSBox-X's conservative classification: file/path-not-found
            // are class 8; other classes and the locus are unspecified.
            let class=if matches!(p.dos_error,2|3){8}else{0};
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|u32::from(p.dos_error);
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|(class<<8)|1;
            p.cpu.regs[1]&=!0xff00;
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;Ok(None)
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
            let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
            let text=crate::dos_runtime::dollar_string(&p.ram,context,p.cpu.regs[2]&0xffff,p.cpu.eip,out.remaining())?;
            console::dos_break_check(p)?;
            if to_file(p,&text)?{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|0x24;return Ok(None);}
            let screen=console::dos_screen(p,&text)?;
            out.write_atomic(&text).map_err(|e|format!("DOS string output provider: {e:?}"))?;
            console::store(p,&screen)?;p.cpu.regs[0]=(p.cpu.regs[0]&!255)|0x24;Ok(None)
        },
        (0x10,ah @ (0x08|0x09|0x0a))=>{console::video(p,ah)?;Ok(None)},
        // Everex/Trident installation query. This VGA BIOS has no Everex
        // extension: retain AL=00, rather than returning the AL=70 signature.
        // RBIL INT 10/AX=7000h/BX=0000h. Other AH70 functions stay unsupported.
        (0x10,0x70) if p.cpu.regs[0]&0xffff==0x7000&&p.cpu.regs[3]&0xffff==0=>Ok(None),
        // Video7 installation query: BX stays zero, not the 'V7' signature.
        (0x10,0x6f) if p.cpu.regs[0]&0xffff==0x6f00&&p.cpu.regs[3]&0xffff==0=>Ok(None),
        // Chips & Technologies / Realtek controller-information probe:
        // retain AL=00 rather than claiming an AL=5Fh extension signature.
        (0x10,0x5f) if p.cpu.regs[0]&0xffff==0x5f00&&p.cpu.regs[3]&0xffff==0=>Ok(None),
        (0x10,ah @ (0x00|0x05|0x06|0x07|0x0b|0x0c|0x0d|0x0e|0x0f))=>{console::display(p,ah)?;Ok(None)},
        (0x16,ah)=>{console::keyboard(p,ah)?;Ok(None)},
        (0x1c,_)=>Ok(None), // BIOS default timer user hook is an IRET.

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
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|u32::from(p.fpu.equipment(0x0025));
            Ok(None)
        },
        (0x12,_)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x0280;
            Ok(None)
        },
        (0x13,0x08)=>{
            // DOS-mounted images and folders are not BIOS sector devices.
            // Report this unimplemented BIOS operation as an error, not a
            // fabricated geometry or access to the underlying host disk.
            p.cpu.regs[0]=(p.cpu.regs[0]&!0xff00)|0x0100;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x15,0xd8) if p.cpu.regs[0]as u8==0=>{
            // EISA slot-information probe on this non-EISA BIOS.
            p.cpu.regs[0]=(p.cpu.regs[0]&!0xff00)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x15,0x84)=>{
            // No game adapter: report the BIOS joystick service unavailable
            // (CF=1, AH=86h); all other register bits remain unchanged.
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x15,0x24) if (p.cpu.regs[0]as u8)<=3=>{
            let op=p.cpu.regs[0]as u8;
            if op<=1{p.speaker.bios_a20(&mut p.ram,op==1);}
            if op==2{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(p.ram.a20_enabled());}
            if op==3{p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|3;}
            p.cpu.regs[0]&=!0xff00;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x15,0xbf) if matches!(p.cpu.regs[0]&0xffff,0xbf00..=0xbf02|0xbfdc|0xbfde)=>{
            // No resident DOS/16M, DOS/4GW or DESQview/X interface. An
            // unsupported BIOS call leaves DX/BX unchanged (no installation
            // signature), sets CF and returns AH=86h.
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x15,0xc2)=>{
            // No PS/2 pointing device is modelled: answer as a BIOS without one
            // (SeaBIOS: CF=1, AH=86h function not supported).
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x33,_) if p.mouse.view().0=>{mouse::service(p)?;Ok(None)},
        (0x33,_) if p.cpu.regs[0]&0xffff==0=>{
            // Standalone probes keep the no-MOUSE.COM contract.
            p.cpu.regs[0]&=0xffff0000;p.cpu.regs[3]&=0xffff0000;Ok(None)
        },
        (0x33,_)=>Ok(None), // absent driver: conventional no-op
        (0x1a,0x00)=>{
            let (ticks,rollover)=bios_ticks(p)?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|rollover;
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|((ticks>>16)&0xffff);
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|(ticks&0xffff);
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x1a,ah @ (0x02|0x04))=>{
            let [century,year,month,day,hour,minute,second]=p.rtc.bios(p.clock_offset);
            let (cx,dx)=if ah==2{(u32::from(hour)<<8|u32::from(minute),u32::from(second)<<8)}
                else{(u32::from(century)<<8|u32::from(year),u32::from(month)<<8|u32::from(day))};
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|cx;
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|dx;
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x21,0x40)=>{write_handle(p,out)?;Ok(None)},
        (0x21,ah @ (0x38|0x65|0x66))=>{nls::service(p,ah)?;Ok(None)},
        (0x21,0x44) if p.cpu.regs[0]as u8==0x0c&&p.cpu.regs[1]&0xffff==0x036a=>{
            // Generic CON code-page query needs a DISPLAY-style device
            // driver. None is installed; the normal device rejects the
            // request. Invalid handles still get DOS error 6.
            let code=if files::system_file(p,(p.cpu.regs[3]&0xffff)as u16)?.is_some(){1}else{6};
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|code;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;Ok(None)
        },
        (0x21,0x44) if matches!(p.cpu.regs[0]as u8,0|6|7|9|0x0d|0x0e)=>{files::service(p,0x44)?;Ok(None)},
        (0x21,0x44) if p.cpu.regs[0]as u8>0x11=>{
            // IOCTL subfunctions end at 11h in DOS 6.22 and FreeDOS
            // (ioctl.c default case): others fail with CF=1, AX=0001h
            // (invalid function). AX=4452h is the DR-DOS version query, so
            // this is how a non-DR-DOS kernel answers it.
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|1;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,ah @ (0x0e|0x19|0x29|0x36|0x45|0x46|0x39|0x3a|0x3b|0x3c|0x3d|0x3e|0x3f|0x41|0x42|0x43|0x47|0x4e|0x4f|0x56|0x57|0x5a|0x5b|0x60|0x69|0x73))=>{files::service(p,ah)?;Ok(None)},
        (0x21,0x50)=>{
            // Set the current PSP to BX (DOS 2+, no checks in DOS or FreeDOS).
            // Dine reads the handle table and exit vectors through it, so only
            // a segment that holds a PSP (INT 20h at offset 0) is taken.
            let segment=(p.cpu.regs[3]&0xffff)as u16;
            let at=u32::from(segment)*16;
            let signature=p.ram.load(at,MemWidth::Word).map_err(|e|format!("DOS PSP read: {e:?}"))?;
            if signature!=0x20cd{return Err(format!("DOS set PSP to {segment:04x}h, which holds no PSP, is not modelled"));}
            p.psp=segment;
            let mut handles=0u32;
            for h in 0..20u16{
                if let Ok(addr)=jft(p,h){
                    if let Ok(entry)=p.ram.load(addr,MemWidth::Byte){
                        if entry as u8!=0xff{handles|=1<<h;}
                    }
                }
            }
            p.open_handles=handles;
            Ok(None)
        },
        (0x21,0x51|0x62)=>{
            // DOS 3+: AH51 and AH62 both return the current PSP segment.
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|u32::from(p.psp);
            Ok(None)
        },
        (0x21,0x55)=>{exec::create_child_psp(p)?;Ok(None)},
        (0x28|0x2a,_)=>{
            // DOS idle (28h) and the network hook (2Ah) end in the kernel's
            // IRET when nothing has hooked them (FreeDOS kernel.asm): no
            // redirector or network software is present.
            Ok(None)
        },
        (0x2f,_) if p.cpu.regs[0]&0xffff==0x1a00&&console::ansi::installed(p)?=>{
            // The ANSI driver answers its installation check with AL=FFh (RBIL INT 2F/AX=1A00h).
            p.cpu.regs[0]|=0xff;Ok(None)
        },
        (0x2f,_) if (0xc0..=0xff).contains(&ah) && p.cpu.regs[0]&255==0
            && p.cpu.regs[3]&0xffff==0x5144 && p.cpu.regs[1]&0xffff==0x4d45 && p.cpu.regs[2]&0xffff==0x4d30=>{
            // No Quarterdeck RPCI handler claims this signed scan.
            Ok(None)
        },
        (0x2f,_) if p.cpu.regs[0]&0xffff==0x4b02=>{
            // The default multiplex handler leaves the caller's state unchanged.
            Ok(None)
        },
        (0x2f,_) if matches!(p.cpu.regs[0]&0xffff,0x0100|0x0600|0x1000|0x1100|0x1500|0x150b|0x1600|0x160a|0x1680..=0x168f|0x1700..=0x17ff|0x1a00|0x4300|0x4680|0x4a10|0x4a11|0x4a33|0xae00|0xb700|0xb706|0xb707|0xb710|0xb800|0xd44d|0xd44e|0xd44f|0xd701)=>{
            // Nothing hooks the multiplex chain, so these installation checks
            // reach the kernel default, which returns AX unchanged (FreeDOS
            // int2f.asm; RBIL): no PRINT, ASSIGN, APPEND, redirector, Windows, DPMI host, ANSI.SYS (unless loaded) or
            // XMS driver, and no installable command claims AX=AE00h (AL stays 00h).
            // AX=1680h leaves AL=80h (time slice not released). D44Dh is the
            // 4DOS installation check (no resident 4DOS answers 44DDh) and
            // 4680h the Windows 3.0 real/standard-mode check (AX=0 if running);
            // D44Eh/D44Fh are 4DOS broadcasts, which nothing takes.
            // 1700h leaves AX=1700h: Windows clipboard (WinOldAp) is not supported.
            // B706h (APPEND state) keeps BX: no APPEND, so no functions on.
            // 1500h/150Bh keep BX (not ADADh): no MSCDEX, no CD-ROM drives.
            // 1000h keeps AL=00h: SHARE is not installed.
            // D200h keeps AL=00h: Quarterdeck RPCI is not installed.
            // D701h keeps AX nonzero: no Banyan VINES. B800h keeps AL=00h: no
            // MS-Net/LAN Manager network.
            Ok(None)
        },
        // Colour VGA BIOS (Tony's choice, 7 November 2026): palette/DAC (10h),
        // font information (11h), EGA information (12h BL=10h) and display
        // combination (1Ah) are answered in dos_planar.
        (0x10,0x10)=>{planar::palette(p)?;Ok(None)},
        (0x10,0x11)=>{planar::font(p)?;Ok(None)},
        (0x10,0x12)=>{planar::alternate(p);Ok(None)},
        (0x10,0x1a)=>{planar::combination(p)?;Ok(None)},
        (0x10,0x1b|0x1c|0x4f|0xfe|0xff)=>{
            // No functionality/state services (1Bh/1Ch) and no VBE: they return
            // unchanged, so AL stays and callers see them as unsupported.
            // FEh/FFh are DESQview/TopView shadow buffers (not installed, ES:DI
            // unchanged), and AX=4F00h does not return AL=4Fh.
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
        (0x21,0x2d)=>{
            let h=(p.cpu.regs[1]>>8)&255;let m=p.cpu.regs[1]&255;let s=(p.cpu.regs[2]>>8)&255;let hundredths=p.cpu.regs[2]&255;
            if h>23||m>59||s>59||hundredths>99{p.cpu.regs[0]|=255;return Ok(None);}
            let total_hundredths=((h*60+m)*60+s)*100+hundredths;
            let desired=(u64::from(total_hundredths)*59659/327680)as u32;
            p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("DOS set time BDA write: {e:?}"))?;
            p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("DOS set time rollover write: {e:?}"))?;
            if p.clock_offset.is_some(){let raw=p.speaker.pit_interrupts(p.clock_offset)%u64::from(TICKS_PER_DAY);p.timer_tick_adjust=i64::from(desired)-raw as i64;}
            p.dos_date_adjust=p.rtc.set_dos_time(p.clock_offset,p.dos_date_adjust,(h*60+m)*60+s);
            p.ram.store(0x046c,MemWidth::Dword,desired).map_err(|e|format!("DOS set time BDA write: {e:?}"))?;
            p.ram.store(0x0470,MemWidth::Byte,0).map_err(|e|format!("DOS set time rollover write: {e:?}"))?;
            if p.clock_offset.is_some(){p.speaker.irq_edges=Some(desired);}
            p.timer_ticks=desired;p.cpu.regs[0]&=!255;Ok(None)
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
            let(year,month,day,weekday)=rtc::dos_date(p.clock_offset,p.dos_date_adjust);
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|year;
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|month<<8|day;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|weekday;
            Ok(None)
        },
        (0x21,0x2b)=>{
            // Set date: FreeDOS DosSetDate rejects a year outside 1980..2099,
            // a month outside 1..12 or a day outside the month with AL=FFh and
            // changes nothing. That is also the DESQview/TopView installation
            // check (CX='DE', DX='SQ'): no DESQview answers it. Valid writes
            // update the session's DOS date and RTC, never the host clock.
            let year=p.cpu.regs[1]&0xffff;let(month,day)=((p.cpu.regs[2]>>8)&255,p.cpu.regs[2]&255);
            let leap=year%4==0&&(year%100!=0||year%400==0);
            let days=match month{1|3|5|7|8|10|12=>31,4|6|9|11=>30,2 if leap=>29,2=>28,_=>0};
            if (1980..=2099).contains(&year)&&day>=1&&day<=days{
                p.dos_date_adjust=p.rtc.set_dos_date(p.clock_offset,p.dos_date_adjust,year,month,day);p.cpu.regs[0]&=!255;
            }else{p.cpu.regs[0]|=0xff;}
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
        (0x21,0x5f) if !matches!(p.cpu.regs[0]as u8,7|8)=>{
            // Redirector calls go to INT 2Fh AX=111Eh. With no network
            // redirector the FreeDOS kernel's own INT 2Fh answers AX=0001h
            // with CF=1 (int2f.asm, "redirected INT21/5F44"), so DOS returns
            // invalid function: CF=1, AX=0001h (inthndlr.c case 0x5f).
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|1;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,0xdc)=>{
            // Not a DOS function (Novell NetWare's connection number): the
            // FreeDOS kernel answers unsupported functions with AL=00h
            // (inthndlr.c, default case), which callers read as no network.
            p.cpu.regs[0]&=!255;Ok(None)
        },
        (0x15,0x88)=>{
            let kb=p.ram.bytes().len().saturating_sub(0x100000)/1024;
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|kb.min(0xffff)as u32;
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        _=>Err(format!("unimplemented DOS/BIOS service INT {vector:02x}, AH={:02x}",(p.cpu.regs[0]>>8)&255)),
    }
}
/// The timer interrupt's effect while time runs on the host clock: when the
/// PIT IRQ edge total moves on, IRR bit 0 is set; with IF=1, no interrupt
/// shadow, IRQ0 unmasked and not in service, the PIC acknowledges it.
/// While INT 08h is still the BIOS entry, Dine does the BIOS handler's work
/// on the host: it stores the edge total at 0040:006C (and sets 0040:0070
/// when the BIOS day count wraps) and runs a program's INT 1Ch hook through
/// the BIOS wrapper. When a program has its own INT 08h handler, IRQ0 is a
/// real-mode interrupt to that handler, which stays in service until an EOI;
/// a handler that chains to the BIOS entry reaches [`bios_timer`]. The
/// deterministic clock (no host clock) raises no IRQs and moves only when it
/// is read, as before.
pub(super) fn timer_interrupt(p:&mut Loaded)->Result<(),String>{
    if p.clock_offset.is_none(){return Ok(());}
    let now=sampled_ticks(p);
    timer_interrupt_at(p,now)
}
pub(super) fn sampled_ticks(p:&mut Loaded)->u32{
    let raw=p.speaker.pit_interrupts(p.clock_offset)%u64::from(TICKS_PER_DAY);
    (raw as i64+p.timer_tick_adjust).rem_euclid(i64::from(TICKS_PER_DAY))as u32
}
/// A sampled PIT IRQ edge total: a new one sets IRR bit 0. The PIC holds a
/// single request, so edges that pass while one is pending are not counted.
pub(super) fn timer_edge(p:&mut Loaded,now:u32){
    if p.speaker.irq_edges!=Some(now){p.speaker.pic.raise_timer();p.speaker.irq_edges=Some(now);}
}
/// Commit a sampled PIT IRQ edge total through the PIC and INT 08h.
pub(super) fn timer_interrupt_at(p:&mut Loaded,now:u32)->Result<(),String>{
    timer_edge(p,now);
    timer_pending(p,now)
}
fn timer_pending(p:&mut Loaded,now:u32)->Result<(),String>{
    if p.cpu.eflags&0x200==0||p.image.sti_shadow!=0||p.image.ss_shadow!=0{return Ok(());}
    if !p.speaker.pic.timer_ready(){return Ok(());}
    // IRQ0's vector is INT 08h unless the PIC has been reinitialized.
    let vector=p.speaker.pic.timer_vector();
    if !p.image.real_mode{
        vectors::deliver_irq(p,vector)?;
        p.speaker.pic.acknowledge_timer();
        return Ok(());
    }
    if !vectors::answered(p,vector)?{
        // The interrupt arrives between instructions: IRET resumes the
        // current IP. The handler's EOI (its own, or the BIOS wrapper's
        // after chaining) takes IRQ0 out of service.
        vectors::deliver_irq(p,vector)?;
        p.speaker.pic.acknowledge_timer();
        return Ok(());
    }
    // Dine's BIOS timer work stands only behind INT 08h; another vector
    // still at its Dine entry (or 0) has no IRQ0 handler behind it.
    if vector!=0x08{return Err(format!("IRQ0 through INT {vector:02X}h (PIC base moved), which has no program handler, is not modelled"));}
    let hook=!vectors::answered(p,0x1c)?;
    p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    // Keep the BIOS shortcut's existing catch-up policy (one hook per poll).
    if hook{vectors::deliver_timer(p)?;}
    p.speaker.pic.acknowledge_timer();
    if !hook{p.speaker.pic.finish_timer();}
    p.ram.store(0x046c,MemWidth::Dword,now).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    if now<p.timer_ticks{p.ram.store(0x0470,MemWidth::Byte,1).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;}
    p.timer_ticks=now;
    Ok(())
}
/// HLT with IF=1 under the host clock while a program owns INT 08h: the
/// halt ends with the next timer edge, delivered to the handler once HLT
/// has retired. Waiting for the edge is not modelled; it arrives at once.
/// If IRQ0 is masked or still in service, execution continues as the
/// deterministic idle tick does.
pub(super) fn halt_tick(p:&mut Loaded)->Result<(),String>{
    p.speaker.pic.raise_timer();
    timer_pending(p,p.timer_ticks)
}
/// The BIOS INT 08h handler, reached by INT 08h at its entry (a program's
/// handler chaining to the saved vector, or a software INT 08h). It adds one
/// to the tick count at 0040:006C, wrapping at a day with 0040:0070 set, then
/// continues in the BIOS wrapper: INT 1Ch, EOI to port 20h, IRET.
pub(super) fn bios_timer(p:&mut Loaded)->Result<(),String>{
    let ticks=p.ram.load(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA read: {e:?}"))?;
    p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    let next=ticks.wrapping_add(1);let wrapped=next>=TICKS_PER_DAY;let next=if wrapped{0}else{next};
    p.ram.store(0x046c,MemWidth::Dword,next).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    if wrapped{p.ram.store(0x0470,MemWidth::Byte,1).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;}
    p.timer_ticks=next;
    vectors::enter_timer_wrapper(p);
    Ok(())
}
/// One INT 1Ah AH=00h read. With a host clock, BIOS ticks follow channel 0
/// interrupt edges, respecting IF and the programmed divisor. Initially the
/// count matches local time of day; reprogramming changes its rate. Otherwise it is the current count,
/// then the deterministic clock advances one tick. Either way the rollover
/// flag is consumed by this read.
fn bios_ticks(p:&mut Loaded)->Result<(u32,u32),String>{
    let(ticks,next_ticks,rollover)=match p.clock_offset{
        Some(_)=>{
            // Do not redirect CS:IP to a hook while servicing INT 1Ah itself;
            // the run loop delivers pending hooks between guest instructions.
            // Nor into a program's INT 08h handler: its chained BIOS ticks
            // are the count.
            if vectors::answered(p,0x1c)?&&vectors::answered(p,p.speaker.pic.timer_vector())?{timer_interrupt(p)?;}
            let now=p.timer_ticks;
            let flagged=p.ram.load(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover read: {e:?}"))?!=0;
            (now,now,u32::from(flagged))
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
    if p.clock_offset.is_none()&&rollover!=0{p.dos_date_adjust+=86400;}
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
    if sft==files::NUL{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|(p.cpu.regs[1]&0xffff);p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;return Ok(());}
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
    if sft>files::NUL{
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
    // Standard output on NUL: the characters are taken and dropped.
    if files::system_file(p,1)?==Some(files::NUL){return Ok(true);}
    let Some(sft)=files::redirected_output(p)? else{return Ok(false)};
    match files::write(p,sft,bytes)?{Ok(n) if n as usize==bytes.len()=>Ok(true),Ok(_)=>Err("DOS console output to redirected handle 1 found the drive full".into()),Err(code)=>Err(format!("DOS console output to redirected handle 1 failed with error {code}"))}
}
/// AH02 (and AH06 output): capture and active-page screen change together.
fn put_char(p:&mut Loaded,out:&mut impl Output,byte:u8)->Result<(),String>{
    if to_file(p,&[byte])?{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(byte);return Ok(());}
    let (bytes,value)=crate::dos_runtime::character_output(byte,out)?;
    let screen=console::dos_screen(p,&bytes)?;
    out.write_atomic(&bytes).map_err(|e|format!("DOS character output provider: {e:?}"))?;
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
/// where DOS echoes, and wait while none have come. AH0A edits as the FreeDOS
/// kernel's read_line does on any device: Backspace and DEL erase, ESC cancels
/// the line, control characters echo as ^X. Returns false for every other call.
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
            let column=p.com1.as_ref().map_or(0,|s|s.column);
            let(mut line,cu_pos,mut first)=p.line.take().unwrap_or((Vec::new(),column,true));
            loop{
                let Some(b)=take(p)else{p.line=Some((line,cu_pos,first));p.input_wait=true;return Ok(true)};
                // 00h starts a two-byte key, as from the keyboard; template
                // keys are not modelled here either, other extended keys do nothing.
                if b==0{
                    let Some(scan)=take(p)else{
                        if let Some(s)=p.com1.as_mut(){s.rx.push_front(0);}
                        p.line=Some((line,cu_pos,first));p.input_wait=true;return Ok(true)
                    };
                    if matches!(scan,0x3b..=0x3f|0x4d|0x52|0x53){
                        p.line=Some((line,cu_pos,first));
                        return Err(format!("DOS line template key {scan:02x} is not modelled"));
                    }
                    first=false;continue;
                }
                match b{
                    b'\r'=>{
                        to_file(p,b"\r")?;
                        let mut bytes=vec![line.len()as u8];bytes.extend(&line);bytes.push(b'\r');
                        for(n,&v)in bytes.iter().enumerate(){p.ram.store(at(1+n as u32)?,MemWidth::Byte,u32::from(v)).map_err(|e|format!("DOS line buffer write: {e:?}"))?;}
                        break;
                    },
                    // LF is never stored; after the first key it starts a new line.
                    b'\n'=>if !first{to_file(p,b"\r\n")?;},
                    6=>{},
                    // Backspace or DEL takes back the last character, a control
                    // character shown as ^X with two, a tab back to its column.
                    8|0x7f=>if let Some(last)=line.pop(){
                        let n=if last==9{
                            let mut col=cu_pos;
                            for &c in line.iter(){col=if c==9{(col+8)&!7}else if c<32{col+2}else{col+1};}
                            p.com1.as_ref().map_or(0,|s|s.column).saturating_sub(col).max(1)
                        }else if last<32{2}else{1};
                        to_file(p,&b"\x08 \x08".repeat(n))?;
                    },
                    // ESC cancels the line: a backslash, and the next line indented
                    // to where this one started.
                    27=>{
                        let mut e=b"\\\r\n".to_vec();e.extend(std::iter::repeat_n(b' ',cu_pos));
                        to_file(p,&e)?;line.clear();
                    },
                    // A full line takes only CR, as DOS rings the bell.
                    _ if line.len()+1>=size=>{to_file(p,b"\x07")?;},
                    _=>{
                        line.push(b);
                        if b<32&&b!=9{to_file(p,&[b'^',b+64])?;}else{to_file(p,&[b])?;}
                    },
                }
                first=false;
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
/// INT 00h at its entry: the FreeDOS kernel's default divide-error handler
/// (entry.asm, reloc_call_int0_handler). It prints "Interrupt divide by zero,
/// stack:" and the 13 words at SS:SP (the fault's IP, CS and FLAGS, then the
/// program's stack) in hex through INT 10h AH=0Eh with BX=0070h, then ends
/// the program with INT 21h AX=4C7Fh (errorlevel 127).
fn divide_error(p:&mut Loaded)->Result<Option<u8>,String>{
    if !vectors::answered(p,0x10)?{return Err("divide error: the DOS default handler prints through INT 10h, which the program has hooked; that is not modelled".into());}
    let ss=MemoryContext{segment:SegmentRegister::Ss,cache:p.image.segments[SegmentRegister::Ss.index()]};
    let sp=p.cpu.regs[4]&0xffff;
    let mut text=b"\r\nInterrupt divide by zero, stack:\r\n".to_vec();
    for k in 0..13u32{
        let at=ss.translate(sp.wrapping_add(2*k)&0xffff,MemWidth::Word,false,p.cpu.eip).map_err(|e|format!("divide error stack read: {e:?}"))?;
        let word=p.ram.load(at,MemWidth::Word).map_err(|e|format!("divide error stack read: {e:?}"))?;
        text.extend(format!("{word:04X} ").bytes());
    }
    text.extend(b"\r\n");
    let writes=console::bios_teletype(p,&text,0x70)?;
    console::check(p,&writes)?;console::store(p,&writes)?;
    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x4c7f;
    Ok(Some(0x7f))
}
