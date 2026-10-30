//! DOS compatibility contracts. CPU execution and host output are separate.
use super::*;
use crate::host_io::Output;
pub(super) fn dispatch(p:&mut Loaded,vector:u8,out:&mut impl Output)->Result<Option<u8>,String>{
    match(vector,(p.cpu.regs[0]>>8)as u8){
        (0x20,_)=>if p.selectors[SegmentRegister::Cs.index()]==PSP{Ok(Some(0))}else{Err("INT20 requires CS=PSP in this shim".into())},
        (0x21,0x58)=>{
            let subfunc = p.cpu.regs[0] & 0xff;
            match subfunc {
                0x00 => {
                    // First fit (0)
                    p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | 0x0000;
                    p.cpu.eflags &= !1;
                }
                0x01 | 0x03 => {
                    p.cpu.eflags &= !1;
                }
                0x02 => {
                    // UMB ei linkitetty
                    p.cpu.regs[0] &= !0xff;
                    p.cpu.eflags &= !1;
                }
                _ => {
                    p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | 1;
                    p.cpu.eflags |= 1;
                }
            }
            p.cpu.undefined_flags &= !1;
            Ok(None)
        },
        (0x21,0x52)=>{
            const SYSVARS_SEG: u16 = 0x0050;
            const SYSVARS_OFF: u16 = 0x0020;
            let es = SegmentRegister::Es.index();
            p.selectors[es] = SYSVARS_SEG;
            p.image.selectors[es] = SYSVARS_SEG;
            p.image.segments[es] = SegmentCache::real_mode(SYSVARS_SEG);
            p.cpu.regs[3] = (p.cpu.regs[3] & 0xffff0000) | u32::from(SYSVARS_OFF); // BX = 0x0020

            // Kirjoitetaan ES:[BX - 2] = PSP - 1
            let first_mcb = PSP - 1;
            let mcb_ptr_addr = u32::from(SYSVARS_SEG) * 16 + u32::from(SYSVARS_OFF) - 2;
            p.ram.store(mcb_ptr_addr, MemWidth::Word, u32::from(first_mcb)).map_err(|e| format!("SYSVARS MCB ptr: {e:?}"))?;

            // Nollataan taulukon alku
            let sysvars_base = u32::from(SYSVARS_SEG) * 16 + u32::from(SYSVARS_OFF);
            let zeros = [0u8; 32];
            p.ram.write_bytes(sysvars_base, &zeros).map_err(|e| format!("SYSVARS zeros: {e:?}"))?;

            Ok(None)
        },
        (0x21,0x07)=>{
            // Direct Console Input Without Echo: palautetaan Enter (0x0D)
            p.cpu.regs[0] = (p.cpu.regs[0] & 0xffffff00) | 0x0d;
            Ok(None)
        },
        (0x21,0x0a)=>{
            let offset = p.cpu.regs[2] & 0xffff; // DX
            let context = MemoryContext{segment:SegmentRegister::Ds, cache:p.image.segments[SegmentRegister::Ds.index()]};
            let max_addr = context.translate(offset, MemWidth::Byte, false, p.cpu.eip).map_err(|e| format!("DOS input buf access: {e:?}"))?;
            let max_len = p.ram.load(max_addr, MemWidth::Byte).map_err(|e| format!("DOS input max read: {e:?}"))? as usize;

            let cmd = b"DIR\r";
            let actual_chars = 3usize.min(max_len.saturating_sub(1));

            // 1. Kirjoitetaan luettu pituus tavuun DS:DX + 1
            let count_addr = context.translate(offset.wrapping_add(1) & 0xffff, MemWidth::Byte, true, p.cpu.eip).map_err(|e| format!("DOS input count access: {e:?}"))?;
            p.ram.store(count_addr, MemWidth::Byte, actual_chars as u32).map_err(|e| format!("DOS input count write: {e:?}"))?;

            // 2. Kirjoitetaan merkit DS:DX + 2 alkaen
            for (i, &b) in cmd[..actual_chars].iter().enumerate() {
                let char_addr = context.translate(offset.wrapping_add(2 + i as u32) & 0xffff, MemWidth::Byte, true, p.cpu.eip).map_err(|e| format!("DOS input char access: {e:?}"))?;
                p.ram.store(char_addr, MemWidth::Byte, u32::from(b)).map_err(|e| format!("DOS input char write: {e:?}"))?;
            }

            // 3. Kirjoitetaan päättävä \r (Enter)
            let cr_addr = context.translate(offset.wrapping_add(2 + actual_chars as u32) & 0xffff, MemWidth::Byte, true, p.cpu.eip).map_err(|e| format!("DOS input cr access: {e:?}"))?;
            p.ram.store(cr_addr, MemWidth::Byte, 0x0d).map_err(|e| format!("DOS input cr write: {e:?}"))?;

            // Kaiku konsoliin
            if out.remaining() >= 5 {
                let _ = out.write_atomic(b"DIR\r\n");
            }
            Ok(None)
        },
        (0x21,0x02)=>{
            let byte=p.cpu.regs[2]as u8;
            let count=if byte==9{8-out.column()%8}else{1};
            if count>out.remaining(){return Err("DOS output limit reached before character output".into());}
            let value=if byte==9{b' '}else{byte};
            let spaces=[b' ';8];let single=[byte];
            out.write_atomic(if byte==9{&spaces[..count]}else{&single}).map_err(|e|format!("DOS character output provider: {e:?}"))?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);
            Ok(None)
        },
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
        (0x21,0x40)=>{
            let handle=(p.cpu.regs[3]&0xffff) as usize;
            let count=(p.cpu.regs[1]&0xffff) as usize;
            if handle>=20 || p.open_handles&(1<<handle)==0{
                p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|6;
                p.cpu.eflags|=1;
                p.cpu.undefined_flags&=!1;
                return Ok(None);
            }
            let offset=p.cpu.regs[2]&0xffff;
            let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
            let mut buf=Vec::with_capacity(count);
            for i in 0..count as u32{
                let at=offset.wrapping_add(i)&0xffff;
                let addr=context.translate(at,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS write buffer access: {e:?}"))?;
                let b=p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS write RAM access: {e:?}"))? as u8;
                buf.push(b);
            }
            if !buf.is_empty(){
                if buf.len()>out.remaining(){return Err("DOS output limit reached before handle write".into());}
                out.write_atomic(&buf).map_err(|e|format!("DOS write output provider: {e:?}"))?;
            }
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|(count as u32);
            p.cpu.eflags&=!1;
            p.cpu.undefined_flags&=!1;
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
                    out.write_atomic(&text).map_err(|e|format!("DOS string output provider: {e:?}"))?;p.cpu.regs[0]=(p.cpu.regs[0]&!255)|0x24;return Ok(None);
                }
                if text.len()>=out.remaining(){return Err("DOS output limit reached before string terminator".into());}
                text.push(b);
            }Err("DOS string has no $ terminator before segment end".into())
        },
        (0x10,0x0f)=>{
            // AL = 0x03 (80x25 color text), AH = 80 (columns) -> AX = 0x5003
            // BH = 0 (active display page)
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x5003;
            p.cpu.regs[3]&=0xffff00ff; // BH = 0
            Ok(None)
        },
        (0x10,0x08)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x0720;
            Ok(None)
        },
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
        (0x1a,0x00)=>{
            const TICKS_PER_DAY:u32=0x0018_00b0;
            let ticks=p.timer_ticks;
            let next=ticks+1;
            let (next_ticks,rollover)=if next>=TICKS_PER_DAY{(0,1u32)}else{(next,0u32)};
            // Validate the entire service write set before committing either cell.
            p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
            p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
            p.ram.store(0x046c,MemWidth::Dword,ticks).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
            p.ram.store(0x0470,MemWidth::Byte,0).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
            p.timer_ticks=next_ticks;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|rollover;
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|((ticks>>16)&0xffff);
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|(ticks&0xffff);
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
            Ok(None)
        },
        (0x21,0x3d)=>{
            let mode = (p.cpu.regs[0] & 7) as u8;
            if mode > 2 {
                p.cpu.eflags |= 1;
                p.cpu.undefined_flags &= !1;
                p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | 0x000c;
                return Ok(None);
            }
            let offset = p.cpu.regs[2] & 0xffff;
            let context = MemoryContext{segment:SegmentRegister::Ds, cache:p.image.segments[SegmentRegister::Ds.index()]};
            let mut path_bytes = Vec::new();
            for i in 0..128u32 {
                let Some(at) = offset.checked_add(i).filter(|&a| a <= 0xffff) else { return Err("DOS open path exceeds segment end before NUL terminator".into()); };
                let addr = context.translate(at, MemWidth::Byte, false, p.cpu.eip).map_err(|e| format!("DOS open path access: {e:?}"))?;
                let b = p.ram.load(addr, MemWidth::Byte).map_err(|e| format!("DOS open path RAM access: {e:?}"))? as u8;
                if b == 0 { break; }
                path_bytes.push(b);
            }
                        let path_str = String::from_utf8_lossy(&path_bytes).to_ascii_uppercase();
                        let ds_val = p.selectors[SegmentRegister::Ds.index()];
            eprintln!("DOS OPEN ATTEMPT: DS={:04x} DX={:04x} path='{}'", ds_val, offset, path_str);
            if path_str.ends_with("COMMAND.COM") {
                let handle = 5u16;
                p.open_handles |= 1 << handle;
                p.file_pos = 0;
                p.cpu.eflags &= !1;
                p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | u32::from(handle);
            } else {
                let err = if path_bytes.is_empty() { 0x0003 } else { 0x0002 };
                p.cpu.eflags |= 1;
                p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | err;
            }
            p.cpu.undefined_flags &= !1;
            Ok(None)
        },
        (0x21,0x3f)=>{
            let handle = (p.cpu.regs[3] & 0xffff) as usize;
            let count = (p.cpu.regs[1] & 0xffff) as usize;
            if handle != 5 || p.open_handles & (1 << handle) == 0 {
                p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | 6;
                p.cpu.eflags |= 1;
                p.cpu.undefined_flags &= !1;
                return Ok(None);
            }
            let offset = p.cpu.regs[2] & 0xffff;
            let context = MemoryContext{segment:SegmentRegister::Ds, cache:p.image.segments[SegmentRegister::Ds.index()]};
            let avail = p.image_bytes.len().saturating_sub(p.file_pos);
            let to_read = count.min(avail);
            for i in 0..to_read as u32 {
                let at = offset.wrapping_add(i) & 0xffff;
                let addr = context.translate(at, MemWidth::Byte, true, p.cpu.eip).map_err(|e| format!("DOS read buf access: {e:?}"))?;
                p.ram.store(addr, MemWidth::Byte, u32::from(p.image_bytes[p.file_pos + i as usize])).map_err(|e| format!("DOS read RAM store: {e:?}"))?;
            }
            p.file_pos += to_read;
            p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | (to_read as u32);
            p.cpu.eflags &= !1;
            p.cpu.undefined_flags &= !1;
            Ok(None)
        },
        (0x21,0x42)=>{
            let handle = (p.cpu.regs[3] & 0xffff) as usize;
            if handle != 5 || p.open_handles & (1 << handle) == 0 {
                p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | 6;
                p.cpu.eflags |= 1;
                p.cpu.undefined_flags &= !1;
                return Ok(None);
            }
            let origin = (p.cpu.regs[0] & 0xff) as u8;
            let offset = ((p.cpu.regs[1] as i32) << 16) | (p.cpu.regs[2] as u16 as i32);
            let new_pos: i64 = match origin {
                0 => offset as i64,
                1 => p.file_pos as i64 + offset as i64,
                2 => p.image_bytes.len() as i64 + offset as i64,
                _ => {
                    p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | 1;
                    p.cpu.eflags |= 1;
                    p.cpu.undefined_flags &= !1;
                    return Ok(None);
                }
            };
            if new_pos < 0 {
                p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | 0x19;
                p.cpu.eflags |= 1;
            } else {
                p.file_pos = new_pos as usize;
                p.cpu.regs[2] = (p.cpu.regs[2] & 0xffff0000) | ((new_pos as u32) & 0xffff);
                p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | (((new_pos as u32) >> 16) & 0xffff);
                p.cpu.eflags &= !1;
            }
            p.cpu.undefined_flags &= !1;
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

