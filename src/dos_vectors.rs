//! The interrupt vector table as programs see it. Each vector Dine answers
//! starts at a three-byte entry in the BIOS area (F000:F000 + 3*n): INT n,
//! then IRET. While a vector still holds its entry (or 0, as vectors Dine has
//! no entry for start), INT n is answered by Dine on the spot, as before. When
//! a program has put its own handler there (AH25h or a direct write), INT n
//! is a real-mode interrupt: FLAGS, CS and the return IP are pushed, IF and TF
//! are cleared, and the handler runs. A handler that chains to the address it
//! saved reaches the entry, where Dine answers the call; leaving the entry
//! then returns to the handler's caller with the caller's IF, TF, DF and IOPL
//! and the status flags (CF, PF, AF, ZF, SF, OF) the service left, as DOS and
//! BIOS handlers return them (RETF 2 or a patched IRET frame).
use super::*;
/// Linear address of vector 0's entry; vector n's is 3*n bytes on.
pub(super) const ENTRIES:u32=0xff000;
const SEGMENT:u16=0xf000;
const FIRST:u16=0xf000;
/// The vectors that start at an entry: the timer and its user hook, the
/// keyboard IRQ, the BIOS video, equipment, memory, disk, serial, system, keyboard, printer and
/// clock services, Ctrl-Break, and the DOS calls (20h, 21h, idle 28h, fast
/// console 29h, network 2Ah, multiplex 2Fh). Data pointers (1Dh-1Eh, 41h,
/// 43h, 46h), the DOS-kept 22h-24h and the free and user vectors stay as
/// they were. INT 1Fh points to the upper graphics font.
pub(super) const ANSWERED:[u8;19]=[0x08,0x09,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x1a,0x1b,0x1c,0x20,0x21,0x28,0x29,0x2a,0x2f];
/// The INT stubs end before the small synthetic IRQ0 BIOS wrapper.
const ENTRY_END:u32=ENTRIES+3*256;
const TIMER_OFFSET:u16=0xf300;
// IRQ0 enters here with the interrupted FLAGS/CS/IP on the stack. Save the
// registers BIOS itself uses across INT 1Ch, then return from the hardware IRQ.
const TIMER_CODE:&[u8]=&[0x66,0x50,0x1e,0xcd,0x1c,0x1f,0xb0,0x20,0xe6,0x20,0x66,0x58,0xcf];
// The BIOS keyboard handler's Ctrl+Break, when a program owns INT 1Bh: call
// it, then EOI and return from IRQ1 through the frame already on the stack.
const BREAK_OFFSET:u16=TIMER_OFFSET+TIMER_CODE.len() as u16;
const BREAK_CODE:&[u8]=&[0x50,0xcd,0x1b,0xb0,0x20,0xe6,0x20,0x58,0xcf];
pub(super) const END:u32=ENTRY_END+(TIMER_CODE.len()+BREAK_CODE.len()) as u32;
/// The far pointer of vector n's entry.
pub(super) fn entry(n:u8)->u32{u32::from(SEGMENT)<<16|(u32::from(FIRST)+3*u32::from(n))}
/// Writes the entries (into the read-only BIOS area, before it is protected)
/// and points the answered vectors at them.
pub(super) fn install(ram:&mut Ram)->Result<(),String>{
    let mut code=vec![0u8;3*256];
    for n in 0..=255u8{code[3*usize::from(n)..][..3].copy_from_slice(&[0xcd,n,0xcf]);}
    code.extend_from_slice(TIMER_CODE);code.extend_from_slice(BREAK_CODE);
    ram.write_bytes(ENTRIES,&code).map_err(|e|format!("interrupt entries: {e:?}"))?;
    for n in ANSWERED{ram.store(u32::from(n)*4,MemWidth::Dword,entry(n)).map_err(|e|format!("INT {n:02x}h vector: {e:?}"))?;}
    Ok(())
}
/// Whether vector n still leads to Dine: its entry, or 0 for a vector that
/// has none.
pub(super) fn answered(p:&Loaded,n:u8)->Result<bool,String>{
    // A raw ROM has no Dine behind its vectors: 0000:0000 is an address too.
    if p.rom_mode{return Ok(false);}
    let vector=p.ram.load(u32::from(n)*4,MemWidth::Dword).map_err(|e|format!("INT {n:02x}h vector read: {e:?}"))?;
    Ok(vector==0||vector==entry(n))
}
fn linear(p:&Loaded)->u32{p.image.segments[SegmentRegister::Cs.index()].base.wrapping_add(p.cpu.eip)}
/// Whether CS:IP is at an entry's INT n, so Dine answers whatever the vector holds.
pub(super) fn at_entry(p:&Loaded)->bool{let at=linear(p);(ENTRIES..ENTRY_END).contains(&at)&&(at-ENTRIES)%3==0}
fn set_cs(p:&mut Loaded,selector:u16){
    let cs=SegmentRegister::Cs.index();
    p.selectors[cs]=selector;p.image.selectors[cs]=selector;p.image.segments[cs]=SegmentCache::real_mode(selector);
}
fn stack(p:&Loaded,offset:u32,write:bool)->Result<u32,String>{
    let context=MemoryContext{segment:SegmentRegister::Ss,cache:p.image.segments[SegmentRegister::Ss.index()]};
    context.translate(offset&0xffff,MemWidth::Word,write,p.cpu.eip).map_err(|e|format!("interrupt stack access: {e:?}"))
}
/// INT n at CS:IP (length 2) to a program's handler. All three stack words
/// and the target are checked before anything changes.
pub(super) fn deliver(p:&mut Loaded,n:u8)->Result<(),String>{
    deliver_at(p,n,(p.cpu.eip+2)&0xffff)
}
/// Fault/exception delivery (such as #UD, vector 06h) with the faulting EIP.
pub(super) fn deliver_fault(p:&mut Loaded,n:u8,fault_ip:u32)->Result<(),String>{
    deliver_at(p,n,fault_ip&0xffff)
}
fn target(p:&Loaded,n:u8)->Result<(u16,u32),String>{
    let(segment,offset)=crate::interrupt::real_vector(&p.image,&p.ram,n)?;let offset=u32::from(offset);
    let address=u32::from(segment)*16+offset;let image=&p.image;
    let fetchable=(address>=image.fetch_base+image.start&&address<image.fetch_base+image.end)
        ||image.system.iter().any(|&(start,end)|address>=start&&address<end);
    if !fetchable{return Err(format!("INT {n:02x}h handler at {segment:04x}:{offset:04x} is outside the running program"));}
    Ok((segment,offset))
}
/// Whether vector n leads to a program's handler inside code Dine can run.
pub(super) fn reachable(p:&Loaded,n:u8)->Result<bool,String>{Ok(!answered(p,n)?&&target(p,n).is_ok())}
pub(super) fn deliver_timer(p:&mut Loaded)->Result<(),String>{
    if !p.image.real_mode||p.cpu.eflags&0x20000!=0{return Err("timer hook outside real mode is not modelled".into());}
    target(p,0x1c)?;
    deliver_to(p,SEGMENT,u32::from(TIMER_OFFSET),p.cpu.eip&0xffff)
}
/// A hardware interrupt (IRQ0 as INT 08h) between instructions: the frame
/// returns to the current IP.
pub(super) fn deliver_irq(p:&mut Loaded,n:u8)->Result<(),String>{
    if !p.image.real_mode||p.cpu.eflags&0x20000!=0{return Err(format!("IRQ through INT {n:02x}h outside real mode is not modelled"));}
    deliver_at(p,n,p.cpu.eip&0xffff)
}
/// Continue the BIOS INT 08h handler in its wrapper (INT 1Ch, EOI, IRET),
/// which returns through the frame already on the stack.
pub(super) fn enter_timer_wrapper(p:&mut Loaded){
    set_cs(p,SEGMENT);p.cpu.eip=u32::from(TIMER_OFFSET);p.image.retired();
}
/// Continue the BIOS INT 09h handler's Ctrl+Break in its wrapper (INT 1Bh,
/// EOI, IRET), which returns through the frame already on the stack.
pub(super) fn enter_break_wrapper(p:&mut Loaded){
    set_cs(p,SEGMENT);p.cpu.eip=u32::from(BREAK_OFFSET);p.image.retired();
}
/// Whether CS:IP is INT 09h at its own entry, where the BIOS handler runs.
pub(super) fn at_keyboard_entry(p:&Loaded)->bool{linear(p)==ENTRIES+3*9}
/// Whether CS:IP is INT 08h at its own entry, where the BIOS handler runs.
pub(super) fn at_timer_entry(p:&Loaded)->bool{linear(p)==ENTRIES+3*8}
/// Protected-mode delivery through the IDT (see crate::interrupt::deliver).
pub(super) fn deliver_protected(p:&mut Loaded,event:crate::interrupt::Event)->Result<(),String>{
    let result=crate::interrupt::deliver(&mut p.cpu,&mut p.image,&mut p.ram,event);
    p.selectors=p.image.selectors;
    result
}
fn deliver_at(p:&mut Loaded,n:u8,return_ip:u32)->Result<(),String>{
    let(segment,offset)=target(p,n)?;deliver_to(p,segment,offset,return_ip)
}
fn deliver_to(p:&mut Loaded,segment:u16,offset:u32,return_ip:u32)->Result<(),String>{
    crate::interrupt::real_frame(&mut p.cpu,&mut p.image,&mut p.ram,segment,offset as u16,return_ip)?;
    p.selectors=p.image.selectors;
    Ok(())
}
/// At an entry's IRET: return to the caller with its own control flags and
/// the status flags the service left. `Ok(false)` anywhere else.
pub(super) fn leave(p:&mut Loaded)->Result<bool,String>{
    let at=linear(p);
    if !(ENTRIES..ENTRY_END).contains(&at)||(at-ENTRIES)%3!=2{return Ok(false);}
    let sp=p.cpu.regs[4]&0xffff;let mut words=[0u32;3];
    for(k,word)in words.iter_mut().enumerate(){
        let address=stack(p,sp.wrapping_add(2*k as u32),false)?;
        *word=p.ram.load(address,MemWidth::Word).map_err(|e|format!("interrupt stack read: {e:?}"))?;
    }
    let[ip,cs,flags]=words;
    const STATUS:u32=0x08d5;
    p.cpu.eflags=(p.cpu.eflags&0xffff0000)|(flags&0x7fd5&!STATUS)|(p.cpu.eflags&STATUS)|2;
    p.cpu.regs[4]=(p.cpu.regs[4]&0xffff0000)|(sp.wrapping_add(6)&0xffff);
    set_cs(p,cs as u16);p.cpu.eip=ip;p.image.retired();
    Ok(true)
}
#[cfg(test)]
#[path="../tests/unit/dos_vectors.rs"]
mod tests;
