//! BIOS interrupt services (INT 08h, 10h, 11h, 12h, 13h, 14h, 15h, 16h, 18h,
//! 19h, 1Ah, 1Ch) answered from `Machine<F>` state alone.
use super::*;
use pil_pc::{sampled_ticks,timer_edge,TICKS_PER_DAY};

/// Answer a BIOS interrupt vector on `p`. Returns `Ok(Some(exit))` when
/// `vector` is a BIOS service handled here, `Ok(None)` when `vector` belongs
/// to an upper layer (such as DOS), or `Err` on an unmodelled subfunction or fault.
pub fn dispatch<F>(p:&mut Machine<F>,vector:u8)->Result<Option<Option<u8>>,String>{
    let ah=(p.cpu.regs[0]>>8)as u8;
    let handled:Option<u8>=match(vector,ah){
        (0x14,ah) if p.com1.is_some()=>{int14::bios(p,ah)?;None},
        (0x10,ah @ (0x08|0x09|0x0a))=>{console::video(p,ah)?;None},
        // Everex/Trident, Video7 and Chips & Technologies / Realtek probes:
        // retain AL=00 / BX=0000 rather than claiming vendor extensions.
        (0x10,0x70) if p.cpu.regs[0]&0xffff==0x7000&&p.cpu.regs[3]&0xffff==0=>None,
        (0x10,0x6f) if p.cpu.regs[0]&0xffff==0x6f00&&p.cpu.regs[3]&0xffff==0=>None,
        (0x10,0x5f) if p.cpu.regs[0]&0xffff==0x5f00&&p.cpu.regs[3]&0xffff==0=>None,
        (0x10,ah @ (0x00|0x05|0x06|0x07|0x0b|0x0c|0x0d|0x0e|0x0f))=>{console::display(p,ah)?;None},
        (0x10,0x01)=>{
            p.ram.store(0x460,MemWidth::Word,p.cpu.regs[1]&0xffff).map_err(|e|format!("BIOS cursor shape write: {e:?}"))?;
            None
        },
        (0x10,0x02)=>{
            let page=(p.cpu.regs[3]>>8)&255;
            if page>=8{return Err("BIOS cursor page must be 0..7".into());}
            let pos=p.cpu.regs[2]&0xffff;
            p.ram.store(0x450+page*2,MemWidth::Word,pos).map_err(|e|format!("BIOS cursor position write: {e:?}"))?;
            None
        },
        (0x10,0x03)=>{
            let page=(p.cpu.regs[3]>>8)&255;
            let shape=p.ram.load(0x460,MemWidth::Word).map_err(|e|format!("BIOS cursor shape read: {e:?}"))?;
            let position=if page<8{p.ram.load(0x450+page*2,MemWidth::Word).map_err(|e|format!("BIOS cursor position read: {e:?}"))?}else{0};
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|shape;
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|position;
            None
        },
        (0x10,0x10)=>{planar::palette(p)?;None},
        (0x10,0x11)=>{planar::font(p)?;None},
        (0x10,0x12)=>{planar::alternate(p);None},
        (0x10,0x1a)=>{planar::combination(p)?;None},
        (0x10,0x1b|0x1c|0x4f|0xfe|0xff)=>None,
        (0x11,_)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|u32::from(p.fpu.equipment(0x0025));
            None
        },
        (0x12,_)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|0x0280;
            None
        },
        (0x16,ah)=>{console::keyboard(p,ah)?;None},
        (0x1c,_)=>None,
        (0x19,_) if p.booted=>disk::bootstrap(p)?,
        (0x18,_) if p.booted=>return Err("INT 18h: no bootable disk and no ROM BASIC".into()),
        (0x13,_) if !p.disks.is_empty()=>disk::int13(p)?,
        (0x13,0x08)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&!0xff00)|0x0100;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;
            None
        },
        (0x13,ah) if !matches!(ah,0x00..=0x05|0x08..=0x0d|0x10|0x11|0x14..=0x18)=>{
            let status=if p.cpu.regs[2]&0x80==0{0x441}else{0x474};
            p.ram.store(status,MemWidth::Byte,1).map_err(|e|format!("BIOS disk status write: {e:?}"))?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!0xff00)|0x0100;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;None
        },
        (0x15,0xd8) if p.cpu.regs[0]as u8==0=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&!0xff00)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;None
        },
        (0x15,0x84)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;None
        },
        (0x15,0x24) if (p.cpu.regs[0]as u8)<=3=>{
            let op=p.cpu.regs[0]as u8;
            if op<=1{p.speaker.bios_a20(&mut p.ram,op==1);}
            if op==2{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(p.ram.a20_enabled());}
            if op==3{p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|3;}
            p.cpu.regs[0]&=!0xff00;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;None
        },
        (0x15,0xbf) if matches!(p.cpu.regs[0]&0xffff,0xbf00..=0xbf02|0xbfdc|0xbfde)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;None
        },
        (0x15,0xc0|0xc2)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;None
        },
        (0x15,0x87)=>{block_move::block_move(p)?;None},
        (0x15,0xe8) if p.cpu.regs[0]as u8==0x01=>{block_move::memory_size(p);None},
        (0x15,0xe8)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;None
        },
        (0x15,0x88)=>{
            let kb=p.ram.bytes().len().saturating_sub(0x100000)/1024;
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|kb.min(0xffff)as u32;
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;None
        },
        (0x1a,0x00)=>{
            let (ticks,rollover)=bios_ticks(p)?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|rollover;
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|((ticks>>16)&0xffff);
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|(ticks&0xffff);
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;None
        },
        (0x1a,0x01)=>{
            // Set system tick counter from CX:DX (IBM PC/AT BIOS): store
            // 0040:006C and clear the midnight flag at 0040:0070.
            let desired=((p.cpu.regs[1]&0xffff)<<16)|(p.cpu.regs[2]&0xffff);
            p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
            p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
            if p.clock_offset.is_some(){
                let raw=p.speaker.pit_interrupts(p.clock_offset)%u64::from(TICKS_PER_DAY);
                p.timer_tick_adjust=i64::from(desired)-raw as i64;
                p.speaker.irq_edges=Some(desired);
            }
            p.ram.store(0x046c,MemWidth::Dword,desired).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
            p.ram.store(0x0470,MemWidth::Byte,0).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
            p.timer_ticks=desired;
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;None
        },
        (0x1a,ah @ (0x03|0x05))=>{
            let (cx,dx)=(p.cpu.regs[1]as u16,p.cpu.regs[2]as u16);
            let [ch,cl,dh,dl]=[(cx>>8)as u8,cx as u8,(dx>>8)as u8,dx as u8];
            if ah==3{
                if dl!=0{return Err(format!("INT 1Ah AH=03h daylight-saving flag DL={dl:02X}h is not modelled"));}
                p.rtc.set_bios_time(p.clock_offset,ch,cl,dh)?;
            }else{p.rtc.set_bios_date(p.clock_offset,ch,cl,dh,dl)?;}
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;None
        },
        (0x1a,0xb1)=>{
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x8600;
            p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;None
        },
        (0x1a,ah @ (0x02|0x04))=>{
            let [century,year,month,day,hour,minute,second]=p.rtc.bios(p.clock_offset);
            let (cx,dx)=if ah==2{(u32::from(hour)<<8|u32::from(minute),u32::from(second)<<8)}
                else{(u32::from(century)<<8|u32::from(year),u32::from(month)<<8|u32::from(day))};
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|cx;
            p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|dx;
            p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;None
        },
        _=>return Ok(None),
    };
    Ok(Some(handled))
}

/// The timer interrupt's effect while time runs on the host clock.
pub fn timer_interrupt<F>(p:&mut Machine<F>)->Result<(),String>{
    if p.clock_offset.is_none(){return Ok(());}
    let now=sampled_ticks(p);
    timer_interrupt_at(p,now)
}

/// Commit a sampled PIT IRQ edge total through the PIC and INT 08h.
pub fn timer_interrupt_at<F>(p:&mut Machine<F>,now:u32)->Result<(),String>{
    timer_edge(p,now);
    timer_pending(p,now)
}

fn timer_pending<F>(p:&mut Machine<F>,now:u32)->Result<(),String>{
    if p.cpu.eflags&0x200==0||p.image.sti_shadow!=0||p.image.ss_shadow!=0{return Ok(());}
    if !p.speaker.pic.timer_ready(){return Ok(());}
    let vector=p.speaker.pic.timer_vector();
    if !p.image.real_mode{
        vectors::deliver_irq(p,vector)?;
        p.speaker.pic.acknowledge_timer();
        return Ok(());
    }
    if !vectors::answered(p,vector)?{
        vectors::deliver_irq(p,vector)?;
        p.speaker.pic.acknowledge_timer();
        return Ok(());
    }
    if vector!=0x08{return Err(format!("IRQ0 through INT {vector:02X}h (PIC base moved), which has no program handler, is not modelled"));}
    let hook=!vectors::answered(p,0x1c)?;
    p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    if hook{vectors::deliver_timer(p)?;}
    p.speaker.pic.acknowledge_timer();
    if !hook{p.speaker.pic.finish_timer();}
    p.ram.store(0x046c,MemWidth::Dword,now).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    if now<p.timer_ticks{p.ram.store(0x0470,MemWidth::Byte,1).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;}
    p.timer_ticks=now;
    Ok(())
}

/// HLT with IF=1 under the host clock while a program owns INT 08h.
pub fn halt_tick<F>(p:&mut Machine<F>)->Result<(),String>{
    p.speaker.pic.raise_timer();
    timer_pending(p,p.timer_ticks)
}

/// The BIOS INT 08h handler, reached by INT 08h at its entry.
pub fn bios_timer<F>(p:&mut Machine<F>)->Result<(),String>{
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

/// One INT 1Ah AH=00h read: returns `(ticks, rollover)`.
pub fn bios_ticks<F>(p:&mut Machine<F>)->Result<(u32,u32),String>{
    let(ticks,next_ticks,rollover)=match p.clock_offset{
        Some(_)=>{
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
    p.ram.check_write(0x046c,MemWidth::Dword).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.check_write(0x0470,MemWidth::Byte).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    p.ram.store(0x046c,MemWidth::Dword,ticks).map_err(|e|format!("BIOS timer BDA write: {e:?}"))?;
    p.ram.store(0x0470,MemWidth::Byte,0).map_err(|e|format!("BIOS timer rollover write: {e:?}"))?;
    p.timer_ticks=next_ticks;
    Ok((ticks,rollover))
}
