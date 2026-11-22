//! Keyboard IRQ1 for programs that own INT 09h. While INT 09h leads to Dine,
//! keys go straight into the BIOS ring as before. Once a program has put its
//! own handler there (to see Ctrl+Break or trap keys itself), each
//! key arrives as an AT keyboard would deliver it: a scan code in port 60h,
//! IRQ1 through the PIC, and INT 09h between instructions with IF=1. The
//! program's handler reads port 60h and usually chains to the vector it
//! saved, Dine's INT 09h entry, which acts as the BIOS handler: it stores the
//! key's BIOS word in the ring, keeps the Ctrl/Shift/Alt bits of 0040:0017,
//! handles Ctrl+Break (empty ring, 0000h word, 0040:0071 bit 7, INT 1Bh) and
//! sends EOI. The host gives each key as its BIOS word, so the BIOS handler
//! takes that word rather than translating scan codes itself; a scan code
//! that arrives with no word (a software INT 09h, a release) only changes the
//! shift bits. Ctrl+Break arrives as the XT codes Ctrl+Scroll Lock (46h with
//! Ctrl down), without the AT E0h prefixes. Native host repeats are repeated
//! makes, and extended physical edges retain E0h. AT keyboard command F3h
//! (typematic rate/delay with FAh ACK via output buffer/IRQ1 and BDA
//! 0040:0097 bit 4) and INT 16h AH=03h are modelled; LEDs and the rest of
//! the keyboard command set and 0040:0096 status byte are not modelled.
use super::*;
use std::collections::VecDeque;
/// IBM PC/AT default typematic byte: 500 ms delay (bits 6..5 = 01), 10.9 cps (bits 4..0 = 0Bh).
pub(super) const DEFAULT_TYPEMATIC:u8=0x2b;
/// One scan code on its way to port 60h, and the BIOS word the BIOS handler
/// stores for it.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(super) struct Code{pub raw:u8,pub word:Option<u16>}
/// Scan codes the keyboard has sent that the program has not taken yet.
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub(super) struct Keyboard{
    queue:VecDeque<Code>,
    /// The code in port 60h (the last one IRQ1 announced) and whether the
    /// controller's output buffer still holds it (port 64h bit 0).
    latch:Option<Code>,unread:bool,
    /// A device reply already latched in `latch` that still owes one IRQ1
    /// unless software polls port 60h first.
    reply_irq:bool,
    /// Guest-programmed typematic byte (`0ddrrrrr`), or None for the AT default.
    typematic:Option<u8>,
}
/// Delay and repeat interval for an AT typematic byte (`0ddrrrrr`).
pub(super) fn typematic_duration(byte:u8)->(std::time::Duration,std::time::Duration){
    let delay_ms=(u64::from((byte>>5)&3)+1)*250;
    let ticks=u64::from(8+(byte&7))<<((byte>>3)&3);
    let period_us=ticks*1_000_000/240;
    (std::time::Duration::from_millis(delay_ms),std::time::Duration::from_micros(period_us))
}
/// Codes queued at most: 16 keys with their releases.
const LIMIT:usize=32;
const CTRL:u8=0x1d;
const BREAK:u8=0x46;
impl Keyboard{
    pub(super) fn pending(&self)->bool{!self.queue.is_empty()||self.reply_irq}
    /// Port 60h: the latched code (0 before any key).
    pub(super) fn read(&mut self)->u8{self.unread=false;self.reply_irq=false;self.latch.map_or(0,|c|c.raw)}
    /// Port 64h bit 0: the output buffer holds a code.
    pub(super) fn full(&self)->bool{self.unread}
    /// Stage an AT keyboard reply byte in the output buffer and pend one IRQ1.
    pub(super) fn reply(&mut self,raw:u8){
        self.latch=Some(Code{raw,word:None});
        self.unread=true;
        self.reply_irq=true;
    }
    pub(super) fn set_typematic(&mut self,value:u8){self.typematic=Some(value&0x7f);}
    pub(super) fn reset_typematic(&mut self){self.typematic=None;}
    pub(super) fn typematic_byte(&self)->u8{self.typematic.unwrap_or(DEFAULT_TYPEMATIC)}
    pub(super) fn typematic_timing(&self)->Option<(std::time::Duration,std::time::Duration)>{
        self.typematic.map(typematic_duration)
    }
    /// Non-destructive state for host diagnostics. Never consume port 60h.
    pub(super) fn diagnostic(&self)->(usize,bool,Option<u8>){
        (self.queue.len(),self.unread,self.latch.map(|c|c.raw))
    }
}
/// Whether keys go through IRQ1 to a program's INT 09h handler. A handler
/// left behind in memory Dine cannot run (a resident program's) is passed
/// by, as before: keys then go straight into the ring.
pub(super) fn hooked(p:&Loaded)->Result<bool,String>{Ok(!p.rom_mode&&(!p.image.real_mode||vectors::reachable(p,p.speaker.pic.keyboard_vector())?))}
/// The make code of the key that produced a BIOS scan code: Ctrl, Alt and
/// Shift variants (F11/F12, Ctrl+arrows, Alt+digits, ...) come from the same
/// physical key. 0 is an Alt+keypad character, stored when Alt goes up.
fn make(scan:u8)->u8{
    match scan{
        0=>0xb8,
        0x54..=0x5d|0x5e..=0x67|0x68..=0x71=>0x3b+(scan-0x54)%10,
        0x73=>0x4b,0x74=>0x4d,0x75=>0x4f,0x76=>0x51,0x77=>0x47,0x84=>0x49,
        0x78..=0x83=>scan-0x76,
        0x85|0x87|0x89|0x8b=>0x57,0x86|0x88|0x8a|0x8c=>0x58,
        0x8d=>0x48,0x8e=>0x4a,0x8f=>0x4c,0x90=>0x4e,0x91=>0x50,0x92=>0x52,0x93=>0x53,
        0x94|0xa5=>0x0f,0x95|0xa4=>0x35,0x96=>0x37,
        0x97..=0xa3=>scan-0x50,0xa6=>0x1c,
        // E0h-prefixed keys of the enhanced keyboard keep their scan in the low byte.
        s if s>=0x80=>s&0x7f,
        s=>s,
    }
}
fn push(p:&mut Loaded,codes:&[Code])->Result<bool,String>{
    let k=&mut p.speaker.kbd;
    if k.queue.len()+codes.len()>LIMIT{return Ok(false);}
    k.queue.extend(codes.iter().copied());Ok(true)
}
/// A key from the host as its BIOS word: the make code with the word, then
/// the release. Alt+keypad characters come with Alt's release instead.
/// One physical press or release. A press remains held until its matching
/// release arrives, so a guest that samples a key-state table can observe it.
pub(super) fn edge(p:&mut Loaded,word:u16,down:bool)->Result<bool,String>{
    let raw=make((word>>8)as u8);
    if raw==0xb8&&!down{return Ok(true);}
    push(p,&[Code{raw:raw|if down{0}else{0x80},word:down.then_some(word)}])
}
/// An actual set-1 edge. Prefix and code enter the bounded queue together.
/// The optional BIOS word belongs only to a make, for handlers chaining to BIOS.
pub(super) fn physical(p:&mut Loaded,scan:u8,extended:bool,down:bool,word:Option<u16>)->Result<bool,String>{
    if scan==0||scan>=0x80{return Ok(true);}
    let code=Code{raw:scan|if down{0}else{0x80},word:if down{word}else{None}};
    if extended{push(p,&[Code{raw:0xe0,word:None},code])}else{push(p,&[code])}
}
pub(super) fn key(p:&mut Loaded,word:u16)->Result<bool,String>{
    let raw=make((word>>8)as u8);
    if raw==0xb8{return push(p,&[Code{raw,word:Some(word)}]);}
    push(p,&[Code{raw,word:Some(word)},Code{raw:raw|0x80,word:None}])
}
/// Ctrl+Break: Scroll Lock with Ctrl down, pressing Ctrl first if the host
/// has not already reported it held.
pub(super) fn ctrl_break(p:&mut Loaded)->Result<bool,String>{
    let flags=p.ram.load(0x417,MemWidth::Byte).map_err(|e|format!("keyboard flags: {e:?}"))?;
    let codes:&[Code]=if flags&4!=0{&[Code{raw:BREAK,word:None},Code{raw:BREAK|0x80,word:None}]}
        else{&[Code{raw:CTRL,word:None},Code{raw:BREAK,word:None},Code{raw:BREAK|0x80,word:None},Code{raw:CTRL|0x80,word:None}]};
    push(p,codes)
}
/// Between instructions: the next code to port 60h and IRQ1 to INT 09h, if
/// IRQ1 is unmasked, nothing of equal or higher priority is in service, and
/// interrupts are enabled (`force` stands for the BIOS's own STI while a
/// blocking INT 16h or DOS read waits). The frame returns to the current IP.
pub(super) fn interrupt(p:&mut Loaded,force:bool)->Result<bool,String>{
    if !p.speaker.kbd.pending(){return Ok(false);}
    if !force&&(p.cpu.eflags&0x200==0||p.image.sti_shadow!=0||p.image.ss_shadow!=0){return Ok(false);}
    if !p.speaker.pic.keyboard_ready(){return Ok(false);}
    vectors::deliver_irq(p,p.speaker.pic.keyboard_vector())?;
    let k=&mut p.speaker.kbd;
    if k.reply_irq{k.reply_irq=false;}else{k.latch=k.queue.pop_front();}
    k.unread=true;
    p.speaker.pic.acknowledge_keyboard();
    Ok(true)
}
/// What the BIOS INT 09h handler at its entry decided.
pub(super) enum Bios{Done,Break}
/// Modifier state shared by the IRQ1 BIOS handler and native input delivered
/// directly to the BIOS ring. A modifier/release never adds a character.
pub(super) fn bios_modifiers(p:&mut Loaded,raw:u8)->Result<u8,String>{
    let flags=p.ram.load(0x417,MemWidth::Byte).map_err(|e|format!("keyboard flags: {e:?}"))?as u8;
    let bit=match raw&0x7f{0x36=>1,0x2a=>2,CTRL=>4,0x38=>8,_=>0};
    let flags=if raw&0x80!=0{flags&!bit}else{flags|bit};
    if bit!=0{p.ram.store(0x417,MemWidth::Byte,u32::from(flags)).map_err(|e|format!("keyboard flags: {e:?}"))?;}
    Ok(flags)
}
/// The BIOS INT 09h handler at its entry, for the code in port 60h.
pub(super) fn bios(p:&mut Loaded)->Result<Bios,String>{
    let code=p.speaker.kbd.latch;p.speaker.kbd.unread=false;p.speaker.kbd.reply_irq=false;
    let Some(code)=code else{p.speaker.pic.end_keyboard();return Ok(Bios::Done);};
    if code.raw==0xfa&&code.word.is_none(){
        let flags=p.ram.load(0x497,MemWidth::Byte).map_err(|e|format!("keyboard status flags: {e:?}"))?as u8;
        p.ram.store(0x497,MemWidth::Byte,u32::from(flags|0x10)).map_err(|e|format!("keyboard status flags: {e:?}"))?;
        p.speaker.pic.end_keyboard();return Ok(Bios::Done);
    }
    let flags=bios_modifiers(p,code.raw)?;
    if code.raw==BREAK&&flags&4!=0{
        brk::bios_break(p)?;
        if !vectors::answered(p,0x1b)?{return Ok(Bios::Break);}
        p.speaker.pic.end_keyboard();return Ok(Bios::Done);
    }
    // A full ring drops the key, as the BIOS does after its beep.
    if let Some(word)=code.word{let _=console::enqueue(p,word)?;}
    p.speaker.pic.end_keyboard();
    Ok(Bios::Done)
}
#[cfg(test)]
#[path="../tests/unit/dos_keyboard.rs"]
mod tests;
