//! Keyboard IRQ1 for programs that own INT 09h. While INT 09h leads to Dine,
//! keys go straight into the BIOS ring as before. Once a program has put its
//! own handler there (GW-BASIC does, to see Ctrl+Break and ON KEY traps), each
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
//! Ctrl down), without the AT E0h prefixes. Typematic repeat, LEDs, the
//! keyboard's command set and the 0040:0096/0097 status bytes are not modelled.
use super::*;
use std::collections::VecDeque;
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
}
/// Codes queued at most: 16 keys with their releases.
const LIMIT:usize=32;
const CTRL:u8=0x1d;
const BREAK:u8=0x46;
impl Keyboard{
    pub(super) fn pending(&self)->bool{!self.queue.is_empty()}
    /// Port 60h: the latched code (0 before any key).
    pub(super) fn read(&mut self)->u8{self.unread=false;self.latch.map_or(0,|c|c.raw)}
    /// Port 64h bit 0: the output buffer holds a code.
    pub(super) fn full(&self)->bool{self.unread}
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
    if p.speaker.kbd.queue.is_empty(){return Ok(false);}
    if !force&&(p.cpu.eflags&0x200==0||p.image.sti_shadow!=0||p.image.ss_shadow!=0){return Ok(false);}
    if !p.speaker.pic.keyboard_ready(){return Ok(false);}
    vectors::deliver_irq(p,p.speaker.pic.keyboard_vector())?;
    let k=&mut p.speaker.kbd;
    k.latch=k.queue.pop_front();k.unread=true;
    p.speaker.pic.acknowledge_keyboard();
    Ok(true)
}
/// What the BIOS INT 09h handler at its entry decided.
pub(super) enum Bios{Done,Break}
/// The BIOS INT 09h handler at its entry, for the code in port 60h.
pub(super) fn bios(p:&mut Loaded)->Result<Bios,String>{
    let code=p.speaker.kbd.latch;p.speaker.kbd.unread=false;
    let Some(code)=code else{p.speaker.pic.end_keyboard();return Ok(Bios::Done);};
    let flags=p.ram.load(0x417,MemWidth::Byte).map_err(|e|format!("keyboard flags: {e:?}"))?as u8;
    let bit=match code.raw&0x7f{0x36=>1,0x2a=>2,CTRL=>4,0x38=>8,_=>0};
    if code.raw==BREAK&&flags&4!=0{
        brk::bios_break(p)?;
        if !vectors::answered(p,0x1b)?{return Ok(Bios::Break);}
        p.speaker.pic.end_keyboard();return Ok(Bios::Done);
    }
    if bit!=0{
        let flags=if code.raw&0x80!=0{flags&!bit}else{flags|bit};
        p.ram.store(0x417,MemWidth::Byte,u32::from(flags)).map_err(|e|format!("keyboard flags: {e:?}"))?;
    }
    // A full ring drops the key, as the BIOS does after its beep.
    if let Some(word)=code.word{let _=console::enqueue(p,word)?;}
    p.speaker.pic.end_keyboard();
    Ok(Bios::Done)
}
