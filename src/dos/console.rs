//! DOS CON character I/O, Ctrl-C break check and optional ANSI.SYS driver,
//! layered above the pure BIOS console in `crate::bios::console`.
use super::*;
use crate::bios::color_text::{CELLS,COLS,PAGE,ROWS,VIDEO};
pub(crate) use crate::bios::console::*;

#[path="ansi.rs"]
pub(super) mod ansi;

/// Blocking BIOS INT 16h and DOS INT 21h CON reads stop without retiring.
/// AH0C flushes once before its first wait, so resuming after a key arrives
/// does not discard that key.
pub(super) fn waiting(p:&mut Loaded,vector:u8)->Result<bool,String>{
    if crate::bios::console::waiting(p,vector)?{return Ok(true);}
    let(ah,al)=((p.cpu.regs[0]>>8)as u8,p.cpu.regs[0]as u8);
    if vector==0x21&&super::services::stdin_aux(p){return Ok(false);}
    match(vector,ah){
        (0x21,1|7|8)=>Ok(!dos_available(p)?),
        (0x21,0x0c) if matches!(al,1|7|8)=>{
            if !p.dos.flushed{let writes=flush_writes(p)?;store(p,&writes)?;p.dos.pending_scan=None;p.dos.flushed=true;}
            Ok(!dos_available(p)?)
        },
        _=>Ok(false),
    }
}

/// DOS console output also reaches the active page, as through INT29/AH0E;
/// with the ANSI driver loaded its escape sequences are interpreted.
pub(super) fn dos_screen(p:&Loaded,bytes:&[u8])->Result<Writes,String>{
    let mode=byte(p,0x449)?;
    // FreeDOS INT 29h prints with AH0Eh and BX=0007h, so colour 7 in graphics.
    if graphics_text::is_graphics(mode){let writes=graphics_text::teletype(p,bytes,7,true)?;check(p,&writes)?;return Ok(writes);}
    let writes=if ansi::installed(p)?&&mode==3{ansi::plan(p,active_page(p)?,bytes)?}else{teletype_plan(p,active_page(p)?,bytes,true)?};
    check(p,&writes)?;Ok(writes)
}

/// DOS stdin is CON on the BIOS ring. An ASCII-0 key returns 00h, then its scan
/// code on the next read, as single-byte DOS reads do.
pub(super) fn dos_available(p:&Loaded)->Result<bool,String>{
    if p.dos.pending_scan.is_some(){return Ok(true);}
    let(head,tail)=ring(p)?;Ok(head!=tail)
}

/// Plan one DOS character read: the byte, ring writes and the next pending scan.
pub(super) fn dos_take(p:&Loaded)->Result<(u8,Writes,Option<u8>),String>{
    if let Some(scan)=p.dos.pending_scan{return Ok((scan,Vec::new(),None));}
    let(head,tail)=ring(p)?;
    if head==tail{return Err("DOS console read requires input".into());}
    let key=read(p,0x400+head)?;let writes=vec![(0x41a,MemWidth::Word,next(head))];
    if key&255==0{Ok((0,writes,Some((key>>8)as u8)))}else{Ok((key as u8,writes,None))}
}

/// The error a DOS call stops with, before any effect, when it finds Ctrl-C:
/// the run loop then echoes ^C and calls INT 23h (see dos/break.rs).
pub(super) const BREAK:&str="DOS Ctrl-C break";

/// FreeDOS check_handle_break on CON: Ctrl-Break pressed (0040:0071 bit 7) or
/// Ctrl-C (03h) at the head of the keyboard buffer. While standard input is
/// COM1 (CTTY), the keyboard is not the console and is not checked.
pub(super) fn dos_break_check(p:&Loaded)->Result<(),String>{
    if super::services::stdin_aux(p){return Ok(());}
    if byte(p,0x471)?&0x80!=0{return Err(BREAK.into());}
    if p.dos.pending_scan.is_some(){return Ok(());}
    let(head,tail)=ring(p)?;
    if head!=tail&&read(p,0x400+head)?&255==3{return Err(BREAK.into());}
    Ok(())
}

/// One AH0A key: a byte below 256, or scan<<8 for an ASCII-0 key read as the
/// two-byte pair read_line consumes. An owed scan from an earlier single-byte
/// read comes back as a plain byte. None means no input is ready.
pub(super) fn dos_line_key(p:&Loaded)->Result<Option<(u32,Writes)>,String>{
    if let Some(scan)=p.dos.pending_scan{return Ok(Some((u32::from(scan),Vec::new())));}
    let(head,tail)=ring(p)?;
    if head==tail{return Ok(None);}
    let key=read(p,0x400+head)?;let writes=vec![(0x41a,MemWidth::Word,next(head))];
    Ok(Some((if key&255==0{key&0xff00}else{key&255},writes)))
}
