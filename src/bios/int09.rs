//! The BIOS INT 09h handler at its entry (see crate::pc::keyboard): the key's
//! BIOS word into the ring, the shift bits of 0040:0017, Ctrl+Break.
use super::*;
/// What the BIOS INT 09h handler at its entry decided.
pub(crate) enum Bios{Done,Break}
/// Modifier state shared by the IRQ1 BIOS handler and native input delivered
/// directly to the BIOS ring. A modifier/release never adds a character.
pub(crate) fn bios_modifiers(p:&mut Loaded,raw:u8)->Result<u8,String>{
    let flags=p.ram.load(0x417,MemWidth::Byte).map_err(|e|format!("keyboard flags: {e:?}"))?as u8;
    let bit=match raw&0x7f{0x36=>1,0x2a=>2,keyboard::CTRL=>4,0x38=>8,_=>0};
    let flags=if raw&0x80!=0{flags&!bit}else{flags|bit};
    if bit!=0{p.ram.store(0x417,MemWidth::Byte,u32::from(flags)).map_err(|e|format!("keyboard flags: {e:?}"))?;}
    Ok(flags)
}
/// The BIOS INT 09h handler at its entry, for the code in port 60h.
pub(crate) fn bios(p:&mut Loaded)->Result<Bios,String>{
    let code=p.speaker.kbd.latch;p.speaker.kbd.unread=false;
    let Some(code)=code else{p.speaker.pic.end_keyboard();return Ok(Bios::Done);};
    let flags=bios_modifiers(p,code.raw)?;
    if code.raw==keyboard::BREAK&&flags&4!=0{
        brk::bios_break(p)?;
        if !vectors::answered(p,0x1b)?{return Ok(Bios::Break);}
        p.speaker.pic.end_keyboard();return Ok(Bios::Done);
    }
    // A full ring drops the key, as the BIOS does after its beep.
    if let Some(word)=code.word{let _=console::enqueue(p,word)?;}
    p.speaker.pic.end_keyboard();
    Ok(Bios::Done)
}
