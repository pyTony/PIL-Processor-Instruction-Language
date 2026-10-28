//! DOS compatibility contracts. CPU execution and host output are separate.
use super::*;
use crate::host_io::Output;
pub(super) fn dispatch(p:&mut Loaded,vector:u8,out:&mut impl Output)->Result<Option<u8>,String>{
    match(vector,(p.cpu.regs[0]>>8)as u8){
        (0x20,_)=>if p.selectors[SegmentRegister::Cs.index()]==PSP{Ok(Some(0))}else{Err("INT20 requires CS=PSP in this shim".into())},
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
                    out.write_atomic(&text).map_err(|e|format!("DOS string output provider: {e:?}"))?;p.cpu.regs[0]=(p.cpu.regs[0]&!255)|0x24;return Ok(None);
                }
                if text.len()>=out.remaining(){return Err("DOS output limit reached before string terminator".into());}
                text.push(b);
            }Err("DOS string has no $ terminator before segment end".into())
        },
        (0x10,0x08)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x0720;
            Ok(None)
        },
        (0x10,0x03)=>{
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|0x0607;
            p.cpu.regs[2]&=0xffff0000;
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
