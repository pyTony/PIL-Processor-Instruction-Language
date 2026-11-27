//! The BIOS INT 09h handler at its entry (see `pil_pc::keyboard`): the key's
//! BIOS word into the ring, the shift bits of 0040:0017, Ctrl+Break.
use super::*;
use pil_pc::keyboard;

/// What the BIOS INT 09h handler at its entry decided.
pub enum Bios{Done,HandledBreak,Break}

/// Modifier state shared by the IRQ1 BIOS handler and native input delivered
/// directly to the BIOS ring. A modifier/release never adds a character.
pub fn bios_modifiers<F>(p:&mut Machine<F>,raw:u8)->Result<u8,String>{
    let flags=p.ram.load(0x417,MemWidth::Byte).map_err(|e|format!("keyboard flags: {e:?}"))?as u8;
    let bit=match raw&0x7f{0x36=>1,0x2a=>2,keyboard::CTRL=>4,0x38=>8,_=>0};
    let flags=if raw&0x80!=0{flags&!bit}else{flags|bit};
    if bit!=0{p.ram.store(0x417,MemWidth::Byte,u32::from(flags)).map_err(|e|format!("keyboard flags: {e:?}"))?;}
    Ok(flags)
}

/// The BIOS keyboard handler's Ctrl+Break: empty the ring, store a 0000h
/// word and set bit 7 of 0040:0071, all or nothing.
pub fn bios_break<F>(p:&mut Machine<F>)->Result<(),String>{
    let mut writes=console::flush_writes(p)?;
    let tail=p.ram.load(0x41c,MemWidth::Word).map_err(|e|format!("BIOS keyboard read: {e:?}"))?;
    let flag=p.ram.load(0x471,MemWidth::Byte).map_err(|e|format!("BIOS break flag: {e:?}"))?;
    let next=if tail==0x3c{0x1e}else{tail+2};
    writes.extend([(0x400+tail,MemWidth::Word,0),(0x41c,MemWidth::Word,next),(0x471,MemWidth::Byte,flag|0x80)]);
    console::check(p,&writes)?;console::store(p,&writes)
}

/// The BIOS INT 09h handler at its entry, for the code in port 60h.
pub fn bios<F>(p:&mut Machine<F>)->Result<Bios,String>{
    let code=p.speaker.kbd.latch;p.speaker.kbd.unread=false;
    let Some(code)=code else{p.speaker.pic.end_keyboard();return Ok(Bios::Done);};
    let flags=bios_modifiers(p,code.raw)?;
    if code.raw==keyboard::BREAK&&flags&4!=0{
        bios_break(p)?;
        if !vectors::answered(p,0x1b)?{return Ok(Bios::Break);}
        p.speaker.pic.end_keyboard();return Ok(Bios::HandledBreak);
    }
    // A full ring drops the key, as the BIOS does after its beep.
    if let Some(word)=code.word{let _=console::enqueue(p,word)?;}
    p.speaker.pic.end_keyboard();
    Ok(Bios::Done)
}
