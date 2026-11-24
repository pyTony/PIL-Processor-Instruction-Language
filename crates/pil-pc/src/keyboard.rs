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
//! makes, and extended physical edges retain E0h. Device-generated typematic, LEDs, the
//! keyboard's command set and the 0040:0096/0097 status bytes are not modelled.
use super::*;
use std::collections::VecDeque;
/// One scan code on its way to port 60h, and the BIOS word the BIOS handler
/// stores for it.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Code{pub raw:u8,pub word:Option<u16>}
/// Scan codes the keyboard has sent that the program has not taken yet.
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct Keyboard{
    pub queue:VecDeque<Code>,
    /// The code in port 60h and whether the controller's output buffer
    /// still holds it (port 64h bit 0), and whether IRQ1 has announced it.
    pub latch:Option<Code>,pub unread:bool,pub signalled:bool,
    /// A keyboard command that waits for its data byte (EDh, F3h, F0h).
    expect:Option<u8>,
    /// The controller's keyboard interface is off (8042 command ADh, until
    /// AEh): the keyboard holds its codes, so none enters the output buffer.
    pub disabled:bool,
}
/// Codes queued at most: 16 keys with their releases.
const LIMIT:usize=32;
pub const CTRL:u8=0x1d;
pub const BREAK:u8=0x46;
impl Keyboard{
    /// A code waits for the output buffer, or the buffer holds one IRQ1 has
    /// not announced yet.
    pub fn pending(&self)->bool{!self.queue.is_empty()||self.unread&&!self.signalled}
    /// A byte the CPU wrote to port 60h that is not for the controller goes
    /// to the keyboard, which answers through the output buffer like a key:
    /// FAh (ACK) to a command it knows, FEh (resend) to one it does not. Set
    /// LEDs (EDh), typematic (F3h) and scan code set (F0h) take a data byte
    /// that is acknowledged in turn. The LEDs, rate and set change nothing
    /// here: the host keyboard supplies the keys (IBM PC AT Technical
    /// Reference, keyboard commands). The first reply enters an empty output
    /// buffer at once; IRQ1 announces it only when `announce` (a guest
    /// handler owns IRQ1, as an OS keyboard driver does), else it waits there
    /// for a program polling port 64h. Further replies follow it.
    pub fn command(&mut self,value:u8,announce:bool){
        let replies:&[u8]=if self.expect.take().is_some(){&[0xfa]}else{match value{
            0xed|0xf3|0xf0=>{self.expect=Some(value);&[0xfa]},
            // Identify: an MF II keyboard (ABh 41h through the controller's
            // scan code translation).
            0xf2=>&[0xfa,0xab,0x41],
            0xee=>&[0xee],
            // Reset: the keyboard drops its buffer and passes its self-test.
            0xff=>{self.queue.clear();&[0xfa,0xaa]},
            0xf4..=0xfd=>&[0xfa],
            _=>&[0xfe],
        }};
        let mut rest=replies;
        if !self.unread{
            if let Some((&first,more))=replies.split_first(){
                self.latch=Some(Code{raw:first,word:None});self.unread=true;self.signalled=!announce;rest=more;
            }
        }
        for &raw in rest.iter().rev(){self.queue.push_front(Code{raw,word:None});}
    }
    /// Port 60h: the latched code (0 before any key).
    pub fn read(&mut self)->u8{self.unread=false;self.latch.map_or(0,|c|c.raw)}
    /// Port 64h bit 0: the output buffer holds a code.
    pub fn full(&self)->bool{self.unread}
    /// Non-destructive state for host diagnostics. Never consume port 60h.
    pub fn diagnostic(&self)->(usize,bool,Option<u8>){
        (self.queue.len(),self.unread,self.latch.map(|c|c.raw))
    }
}
/// Whether keys go through IRQ1 to a program's INT 09h handler. A handler
/// left behind in memory Dine cannot run (a resident program's) is passed
/// by, as before: keys then go straight into the ring.
pub fn hooked<F:Firmware>(p:&Machine<F>)->Result<bool,String>{F::keyboard_irq_owned(p)}
/// The make code of the key that produced a BIOS scan code: Ctrl, Alt and
/// Shift variants (F11/F12, Ctrl+arrows, Alt+digits, ...) come from the same
/// physical key. 0 is an Alt+keypad character, stored when Alt goes up.
pub fn make(scan:u8)->u8{
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
fn push<F>(p:&mut Machine<F>,codes:&[Code])->Result<bool,String>{
    let k=&mut p.speaker.kbd;
    if k.queue.len()+codes.len()>LIMIT{return Ok(false);}
    k.queue.extend(codes.iter().copied());Ok(true)
}
/// A key from the host as its BIOS word: the make code with the word, then
/// the release. Alt+keypad characters come with Alt's release instead.
/// One physical press or release. A press remains held until its matching
/// release arrives, so a guest that samples a key-state table can observe it.
pub fn edge<F>(p:&mut Machine<F>,word:u16,down:bool)->Result<bool,String>{
    let raw=make((word>>8)as u8);
    if raw==0xb8&&!down{return Ok(true);}
    push(p,&[Code{raw:raw|if down{0}else{0x80},word:down.then_some(word)}])
}
/// An actual set-1 edge. Prefix and code enter the bounded queue together.
/// The optional BIOS word belongs only to a make, for handlers chaining to BIOS.
pub fn physical<F>(p:&mut Machine<F>,scan:u8,extended:bool,down:bool,word:Option<u16>)->Result<bool,String>{
    if scan==0||scan>=0x80{return Ok(true);}
    let code=Code{raw:scan|if down{0}else{0x80},word:if down{word}else{None}};
    if extended{push(p,&[Code{raw:0xe0,word:None},code])}else{push(p,&[code])}
}
pub fn key<F>(p:&mut Machine<F>,word:u16)->Result<bool,String>{
    let raw=make((word>>8)as u8);
    if raw==0xb8{return push(p,&[Code{raw,word:Some(word)}]);}
    push(p,&[Code{raw,word:Some(word)},Code{raw:raw|0x80,word:None}])
}
/// Ctrl+Break: Scroll Lock with Ctrl down, pressing Ctrl first if the host
/// has not already reported it held.
pub fn ctrl_break<F>(p:&mut Machine<F>)->Result<bool,String>{
    let flags=p.ram.load(0x417,MemWidth::Byte).map_err(|e|format!("keyboard flags: {e:?}"))?;
    let codes:&[Code]=if flags&4!=0{&[Code{raw:BREAK,word:None},Code{raw:BREAK|0x80,word:None}]}
        else{&[Code{raw:CTRL,word:None},Code{raw:BREAK,word:None},Code{raw:BREAK|0x80,word:None},Code{raw:CTRL|0x80,word:None}]};
    push(p,codes)
}
/// Between instructions: the next code to port 60h and IRQ1 to INT 09h, if
/// IRQ1 is unmasked, nothing of equal or higher priority is in service, and
/// interrupts are enabled (`force` stands for the BIOS's own STI while a
/// blocking INT 16h or DOS read waits). The frame returns to the current IP.
pub fn interrupt<F:Firmware>(p:&mut Machine<F>,force:bool)->Result<bool,String>{
    // A code a polling driver already brought into the output buffer is
    // announced as it stands; otherwise the next code enters it with IRQ1.
    let filled=p.speaker.kbd.unread&&!p.speaker.kbd.signalled;
    if !filled&&(p.speaker.kbd.queue.is_empty()||p.speaker.kbd.disabled){return Ok(false);}
    if !force&&(p.cpu.eflags&0x200==0||p.image.sti_shadow!=0||p.image.ss_shadow!=0){return Ok(false);}
    if !p.speaker.pic.keyboard_ready(){return Ok(false);}
    F::deliver_irq(p,p.speaker.pic.keyboard_vector())?;
    let k=&mut p.speaker.kbd;
    if !filled{k.latch=k.queue.pop_front();k.unread=true;}
    k.signalled=true;
    p.speaker.pic.acknowledge_keyboard();
    Ok(true)
}
impl Keyboard{
    /// Port 64h read while IRQ1 cannot be taken (interrupts off or IRQ1
    /// masked): the next code is in the output buffer by now, as for a
    /// driver that polls the controller.
    pub fn poll(&mut self,irq_possible:bool){
        if irq_possible||self.unread||self.disabled{return;}
        if let Some(code)=self.queue.pop_front(){self.latch=Some(code);self.unread=true;self.signalled=false;}
    }
}
