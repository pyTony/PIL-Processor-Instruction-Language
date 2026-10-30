//! DOS compatibility contracts. CPU execution and host output are separate.
use super::*;
use crate::host_io::Output;
pub(super) fn dispatch(p:&mut Loaded,vector:u8,out:&mut impl Output)->Result<Option<u8>,String>{
    match(vector,(p.cpu.regs[0]>>8)as u8){
        (0x20,_)=>if p.selectors[SegmentRegister::Cs.index()]==PSP{Ok(Some(0))}else{Err("INT20 requires CS=PSP in this shim".into())},
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
        (0x21,0x44) if p.cpu.regs[0]as u8==0=>{
            let handle=p.cpu.regs[3]&0xffff;
            if handle>=5 || p.open_handles&(1<<handle)==0{
                p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|6;
                p.cpu.eflags|=1;
            }else{
                // Minimal character-device profile: CON for 0..2, AUX/PRN for 3..4.
                // No raw mode, direct INT29 output or generic IOCTL advertised.
                let info=if handle<3{0x83}else{0x80};
                p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|info;
                p.cpu.eflags&=!1;
            }
            p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,0x30)=>{
            let version=p.ram.load(u32::from(PSP)*16+0x40,MemWidth::Word).map_err(|e|format!("DOS version PSP read: {e:?}"))?;
            let flags=p.cpu.regs[0]as u8==1;
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|version;
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|if flags{0}else{0xff00};
            p.cpu.regs[1]&=0xffff0000;
            Ok(None)
        },
        (0x21,0x4c)=>Ok(Some(p.cpu.regs[0]as u8)),
        (0x21,ah @ (0x48|0x49|0x4a))=>{memory_service(p,ah)?;Ok(None)},
        (0x21,9)=>{
            let offset=p.cpu.regs[2]&0xffff;let mut text=Vec::new();
            let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
            for n in offset..=0xffff{
                let addr=context.translate(n,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS string access: {e:?}"))?;
                let b=p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS string RAM access: {e:?}"))?as u8;
                if b==b'$'{
                    console::dos_break_check(p)?;let screen=console::dos_screen(p,&text)?;
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
        (0x1a,0x00)=>{
            let (ticks,rollover)=bios_ticks(p)?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|rollover;
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|((ticks>>16)&0xffff);
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|(ticks&0xffff);
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,0x3d)=>{
            let mode=(p.cpu.regs[0]&7)as u8;
            if mode>2{
                p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x000c;
                return Ok(None);
            }
            let offset=p.cpu.regs[2]&0xffff;
            let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
            let mut len=0usize;let mut terminated=false;
            for i in 0..128u32{
                let Some(at)=offset.checked_add(i).filter(|&a|a<=0xffff)else{return Err("DOS open path exceeds segment end before NUL terminator".into());};
                let addr=context.translate(at,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS open path access: {e:?}"))?;
                let b=p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS open path RAM access: {e:?}"))?as u8;
                if b==0{terminated=true;break;}
                len+=1;
            }
            if !terminated{return Err("DOS open path has no NUL terminator within 128 bytes".into());}
            let err=if len==0{0x0003}else{0x0002};
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|err;
            Ok(None)
        },
        (0x21,0x40)=>{write_handle(p,out)?;Ok(None)},
        (0x21,0x51|0x62)=>{
            // DOS 3+: AH51 and AH62 both return the current PSP segment.
            p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|u32::from(PSP);
            Ok(None)
        },
        (0x2f,_) if matches!(p.cpu.regs[0]&0xffff,0x0600|0x1100|0x1600|0x1680|0x1687|0x1a00|0x4300)=>{
            // Nothing hooks the multiplex chain, so these installation checks
            // reach the kernel default, which returns AX unchanged (FreeDOS
            // int2f.asm; RBIL): no ASSIGN, redirector, Windows, DPMI host, ANSI.SYS or
            // XMS driver. AX=1680h leaves AL=80h (time slice not released).
            Ok(None)
        },
        (0x10,0x12|0x4f)=>{
            // CGA-class video BIOS: no EGA/VGA alternate functions and no VBE.
            // Such a BIOS ignores these calls, so BL=10h stays 10h (no EGA)
            // and AX=4F00h does not return AL=4Fh.
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
            // No clock driver or CMOS: DOS keeps its 1 January 1980 default, a
            // Tuesday (FreeDOS systime.c: day of week (days+2)%7 with days=0).
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|1980;
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|0x0101;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|2;
            Ok(None)
        },
        (0x21,0x58)=>{
            let al=p.cpu.regs[0]as u8;let bl=p.cpu.regs[3]as u8;
            match al{
                0=>{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|u32::from(p.alloc_strategy);p.cpu.eflags&=!1;},
                1 if bl>0x82||(bl&0x3f)>2=>{p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|1;p.cpu.eflags|=1;},
                // Without UMBs, 8xh (high then low) allocates low with the same fit.
                1 if bl&0x40==0=>{p.alloc_strategy=bl;p.cpu.eflags&=!1;},
                1=>return Err(format!("DOS allocation strategy {bl:02x}h (upper memory only) is not modelled")),
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
        (0x21,0x3e)=>{
            let handle=(p.cpu.regs[3]&0xffff)as u16;
            if handle<20&&(p.open_handles&(1u32<<handle))!=0{
                p.ram.store(u32::from(PSP)*16+0x18+u32::from(handle),MemWidth::Byte,0xff).map_err(|e|format!("DOS close JFT write: {e:?}"))?;
                p.open_handles&=!(1u32<<handle);
                p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            }else{
                p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
                p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x0006;
            }
            Ok(None)
        },
        _=>Err(format!("unimplemented DOS/BIOS service INT {vector:02x}, AH={:02x}",(p.cpu.regs[0]>>8)&255)),
    }
}
/// One INT 1Ah AH=00h read: the current count, then the deterministic clock
/// advances one tick. The rollover flag is consumed by this read.
fn bios_ticks(p:&mut Loaded)->Result<(u32,u32),String>{
    let ticks=p.timer_ticks;
    let next=ticks+1;
    let (next_ticks,rollover)=if next>=TICKS_PER_DAY{(0,1u32)}else{(next,0u32)};
    // Validate the entire service write set before committing either cell.
    p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    p.ram.store(0x046c,MemWidth::Dword,ticks).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.store(0x0470,MemWidth::Byte,0).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    p.timer_ticks=next_ticks;
    Ok((ticks,rollover))
}
/// AH40 on the CON handles 0..2 writes CX bytes from DS:DX like AH09, without
/// a terminator. AUX/PRN (3..4) are not modelled; a closed handle is error 6.
fn write_handle(p:&mut Loaded,out:&mut impl Output)->Result<(),String>{
    let handle=p.cpu.regs[3]&0xffff;
    if handle>=20||p.open_handles&(1<<handle)==0{
        p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|6;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
        return Ok(());
    }
    if handle>=3{return Err(format!("DOS write to handle {handle} (AUX/PRN) is not modelled"));}
    let offset=p.cpu.regs[2]&0xffff;let count=p.cpu.regs[1]&0xffff;
    if offset+count>0x10000{return Err("DOS write buffer exceeds segment end".into());}
    if count as usize>out.remaining(){return Err("DOS output limit reached before handle write".into());}
    let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
    let mut text=Vec::with_capacity(count as usize);
    for n in offset..offset+count{
        let addr=context.translate(n,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS write buffer access: {e:?}"))?;
        text.push(p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS write buffer RAM access: {e:?}"))?as u8);
    }
    console::dos_break_check(p)?;let screen=console::dos_screen(p,&text)?;
    out.write_atomic(&text).map_err(|e|format!("DOS handle output provider: {e:?}"))?;
    console::store(p,&screen)?;
    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|count;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
    Ok(())
}
/// AH02 (and AH06 output): capture and active-page screen change together.
fn put_char(p:&mut Loaded,out:&mut impl Output,byte:u8)->Result<(),String>{
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
    let dx=p.cpu.regs[2]&0xffff;
    let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
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
