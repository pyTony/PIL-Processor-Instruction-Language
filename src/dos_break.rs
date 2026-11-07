//! Ctrl-C and Ctrl-Break during DOS calls, after FreeDOS break.c (handle_break)
//! and procsupt.asm (spawn_int23): DOS echoes ^C, empties the keyboard, and
//! calls INT 23h through the vector table with the registers the program
//! passed to INT 21h and CF clear. A handler that returns with IRET or RETF 2
//! leaves SP where it was: DOS then runs the interrupted INT 21h call again
//! with the registers the handler left. Any other SP (a plain RETF, which
//! leaves the flags word on the stack) aborts the program as AH00 would,
//! with return code 0 and termination type 1 (Ctrl-C) for AH4D.
use super::*;
use crate::host_io::Output;
/// Offsets in DOSDATA of Dine's default INT 23h handler (STC; RETF: abort the
/// program, as COMMAND.COM and FreeCOM's handler for external programs do),
/// and of the INT 23h instruction DOS calls handlers from. The handler returns
/// to the byte after that instruction, which Dine watches.
pub(super) const HANDLER:u16=0x0074;
const CALL:u16=0x0076;
const RETURN:u16=CALL+2;
/// Bytes at HANDLER: STC; RETF; then INT 23h at CALL.
pub(super) const CODE:[u8;4]=[0xf9,0xcb,0xcd,0x23];
/// An INT 23h call that has not returned yet.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Frame{cs:u16,eip:u32,sp:u16,depth:usize,window:Option<(u32,u32,u32)>}
fn word_at(p:&Loaded,segment:SegmentRegister,offset:u32,write:bool)->Result<u32,String>{
    let context=MemoryContext{segment,cache:p.image.segments[segment.index()]};
    context.translate(offset,MemWidth::Word,write,p.cpu.eip).map_err(|e|format!("INT 23h stack access: {e:?}"))
}
fn set_cs(p:&mut Loaded,selector:u16){
    let cs=SegmentRegister::Cs.index();
    p.selectors[cs]=selector;p.image.selectors[cs]=selector;p.image.segments[cs]=SegmentCache::real_mode(selector);
}
/// The ^C at CS:IP's INT 21h: echo, flush and enter the INT 23h handler, all
/// or nothing. CS:IP is left at the handler's first instruction.
pub(super) fn start(p:&mut Loaded,out:&mut impl Output)->Result<(),String>{
    let echo=b"^C\r\n";
    if out.remaining()<echo.len(){return Err("DOS output limit reached before ^C echo".into());}
    let vector=p.ram.load(0x23*4,MemWidth::Dword).map_err(|e|format!("INT 23h vector read: {e:?}"))?;
    let(segment,offset)=((vector>>16)as u16,vector&0xffff);
    let flag=p.ram.load(0x471,MemWidth::Byte).map_err(|e|format!("BIOS break flag: {e:?}"))?;
    let sp=p.cpu.regs[4]&0xffff;let flags=(p.cpu.eflags&0xffff)&!1;
    let mut writes=console::dos_flush_writes(p)?;
    writes.extend(console::dos_screen(p,echo)?);
    writes.push((0x471,MemWidth::Byte,flag&0x7f));
    for(n,value)in [(2,flags),(4,u32::from(DOSDATA)),(6,u32::from(RETURN))]{
        writes.push((word_at(p,SegmentRegister::Ss,sp.wrapping_sub(n)&0xffff,true)?,MemWidth::Word,value));
    }
    console::check(p,&writes)?;
    out.write_atomic(echo).map_err(|e|format!("DOS ^C echo provider: {e:?}"))?;
    console::store(p,&writes)?;
    // A handler in a parent's block (COMMAND.COM's, inherited by a program it
    // started) runs there; the program's block comes back when it returns.
    let target=u32::from(segment)*16+offset;
    let image=&p.image;
    let inside=|base:u32,start:u32,end:u32|target>=base+start&&target<base+end;
    let window=if inside(image.fetch_base,image.start,image.end)||image.system.iter().any(|&(s,e)|target>=s&&target<e){None}
        else{p.exec.iter().rev().map(|f|&f.image).find(|i|inside(i.fetch_base,i.start,i.end)).map(|i|(i.fetch_base,i.start,i.end))};
    let frame=Frame{cs:p.selectors[SegmentRegister::Cs.index()],eip:p.cpu.eip,sp:sp as u16,depth:p.exec.len(),
        window:window.map(|_|(p.image.fetch_base,p.image.start,p.image.end))};
    if let Some((base,start,end))=window{p.image.fetch_base=base;p.image.start=start;p.image.end=end;}
    p.breaks.push(frame);
    p.cpu.regs[4]=(p.cpu.regs[4]&0xffff0000)|(sp.wrapping_sub(6)&0xffff);
    // CLC, then INT 23h clears IF and TF.
    p.cpu.eflags&=!0x301;p.cpu.undefined_flags&=!0x301;
    set_cs(p,segment);p.cpu.eip=offset;
    p.pending_scan=None;p.line=None;p.flushed=false;p.input_wait=false;
    p.image.retired();
    Ok(())
}
/// What an INT 23h handler asked for when it returned.
pub(super) enum Return{Resume,Abort}
/// At CS:IP == the return point: resume the INT 21h call or abort the program.
pub(super) fn returned(p:&mut Loaded)->Result<Option<Return>,String>{
    let at=p.image.segments[SegmentRegister::Cs.index()].base.wrapping_add(p.cpu.eip);
    if at!=u32::from(DOSDATA)*16+u32::from(RETURN){return Ok(None);}
    let frame=p.breaks.pop().ok_or("INT 23h return point reached without a Ctrl-C break")?;
    if let Some((base,start,end))=frame.window{p.image.fetch_base=base;p.image.start=start;p.image.end=end;}
    if p.cpu.regs[4]&0xffff!=u32::from(frame.sp){return Ok(Some(Return::Abort));}
    set_cs(p,frame.cs);p.cpu.eip=frame.eip;p.image.retired();
    Ok(Some(Return::Resume))
}
/// A program ended: its unreturned INT 23h calls go with it.
pub(super) fn drop_program(p:&mut Loaded){let depth=p.exec.len();p.breaks.retain(|f|f.depth<depth);}
/// Ctrl+Break as the PC BIOS and DOS take it (keyboard IRQ, then DOS's INT 1Bh
/// handler): the keyboard buffer is emptied and holds one 0000h word, and bit 7
/// of 0040:0071 is set for DOS to see at its next break check. A program's own
/// INT 1Bh handler is not called, so a hooked INT 1Bh is refused.
pub(super) fn ctrl_break(p:&mut Loaded)->Result<(),String>{
    if !vectors::answered(p,0x1b)?{
        return Err("the program hooked INT 1Bh; calling it for Ctrl+Break is not modelled".into());
    }
    bios_break(p)
}
/// The BIOS keyboard handler's Ctrl+Break: empty the ring, store a 0000h
/// word and set bit 7 of 0040:0071, all or nothing.
pub(super) fn bios_break(p:&mut Loaded)->Result<(),String>{
    // The flush makes head equal tail; the 0000h word goes in at tail.
    let mut writes=console::dos_flush_writes(p)?;
    let tail=p.ram.load(0x41c,MemWidth::Word).map_err(|e|format!("BIOS keyboard read: {e:?}"))?;
    let flag=p.ram.load(0x471,MemWidth::Byte).map_err(|e|format!("BIOS break flag: {e:?}"))?;
    let next=if tail==0x3c{0x1e}else{tail+2};
    writes.extend([(0x400+tail,MemWidth::Word,0),(0x41c,MemWidth::Word,next),(0x471,MemWidth::Byte,flag|0x80)]);
    console::check(p,&writes)?;console::store(p,&writes)?;
    p.pending_scan=None;Ok(())
}
#[cfg(test)]
#[path="../tests/unit/dos_break.rs"]
mod tests;
