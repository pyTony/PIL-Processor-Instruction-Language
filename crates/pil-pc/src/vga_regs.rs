//! VGA registers in a booted machine. An operating system's graphics
//! library (svgalib on Linux) sets a mode by writing the miscellaneous
//! output, sequencer, CRTC, graphics controller and attribute registers
//! itself, without the BIOS. Here every register 0-4 (sequencer), 0-8
//! (graphics controller), 0-18h (CRTC) and the miscellaneous output takes
//! any value, and after each write:
//!
//! - **CPU access to video memory** follows the live sequencer and graphics
//!   controller values, as on the hardware. With the memory map at A0000h
//!   (GC 06h bits 3-2 = 00b or 01b) and no odd/even addressing (sequencer
//!   04h bit 2 set or chain-4 on, GC 05h bit 4 clear), the four 64 KiB
//!   planes are Ram's planar window (vga_planar: map mask, read map, the
//!   four write modes, latches, chain-4). Otherwise A0000h-BFFFFh is
//!   ordinary RAM: the text buffer at B8000h, as the text modes keep it.
//!   The planes are kept while unmapped, so glyphs written to plane 2
//!   survive the return to text, as svgalib's font restore needs.
//! - **The display** (vga::Display, what the front ends draw) is read from
//!   the registers: graphics or text (GC 06h bit 0), 256 or 16 colours
//!   (attribute 10h bit 6), the CRTC horizontal display end and vertical
//!   display end, maximum scan line and scan doubling, the dot clock halving
//!   (sequencer 01h bit 3). A state that matches BIOS mode 13h (320x200,
//!   chain-4), 0Dh, 0Eh, 10h or 12h (16 colours, planar) is shown as that
//!   mode; text is mode 03h (07h with the B0000h map) with its columns and
//!   rows; anything else (mode X, other sizes) is shown as nothing
//!   (mode FFh), not as the last known mode.
//!
//! Not modelled: odd/even graphics addressing at A0000h (CGA-compatible
//! layouts; memory stays RAM there), the text characters in planes 0 and 1
//! (they stay at B8000h), split screens, pixel panning and timings beyond
//! the display size.
use super::*;
use super::vga::{Display,Vga};
use crate::vga_planar::Planar;

/// Sequencer (3C4h/3C5h) and graphics controller (3CEh/3CFh) access in a
/// booted machine; a word OUT to an index port writes the index then the
/// data. Absent extension indices read FFh and ignore writes.
pub fn controller_io<F>(p:&mut Machine<F>,port:u16,input:bool,value:u16,word:bool)->Result<u16,String>{
    if word&&!matches!(port,0x3c4|0x3ce){return Err("VGA word access must start at index port 3C4h or 3CEh".into());}
    let sequencer=port<0x3ce;
    let c=&mut p.vga.idle;
    let mut index=if sequencer{c.seq_index}else{c.gc_index};
    let mut access=|data:bool,value:u8|->u8{
        if !data{if input{return index;}index=value;return 0;}
        let regs:&mut [u8]=if sequencer{&mut c.seq}else{&mut c.gc};
        let Some(slot)=regs.get_mut(usize::from(index)) else{return if input{0xff}else{0};};
        if input{return *slot;}
        *slot=value;0
    };
    let lo=access(port&1!=0,value as u8);
    let hi=if word{access(true,(value>>8)as u8)}else{0};
    if sequencer{c.seq_index=index;}else{c.gc_index=index;}
    if !input{sync(p);}
    Ok(u16::from(lo)|u16::from(hi)<<8)
}

/// Whether the registers put the planes at A0000h with plane (or chain-4)
/// addressing.
pub fn plane_window(v:&Vga)->bool{
    let(seq,gc)=(&v.idle.seq,&v.idle.gc);
    gc[6]&0x0c<=0x04&&(seq[4]&0x08!=0||seq[4]&0x04!=0)&&gc[5]&0x10==0
}

/// Maps or unmaps the planes and recomputes the display after a register
/// write in a booted machine.
pub fn sync<F>(p:&mut Machine<F>){
    if !p.booted{return;}
    let want=plane_window(&p.vga);
    match(p.ram.planar().is_some(),want){
        (false,true)=>{
            let mut planes=p.vga.font.take().unwrap_or_else(Planar::new);
            planes.seq=p.vga.idle.seq;planes.gc=p.vga.idle.gc;p.ram.set_planar(Some(planes));
        },
        (true,true)=>{let(seq,gc)=(p.vga.idle.seq,p.vga.idle.gc);let planes=p.ram.planar_mut().unwrap();planes.seq=seq;planes.gc=gc;},
        (true,false)=>{p.vga.font=p.ram.planar().cloned();p.ram.set_planar(None);},
        (false,false)=>{},
    }
    p.vga.display=classify(&p.vga);
}

/// The display the registers describe (see the module comment).
pub fn classify(v:&Vga)->Display{
    let(seq,gc,c)=(&v.idle.seq,&v.idle.gc,v.crtc());
    let columns=u32::from(c[1])+1;
    let lines=(u32::from(c[0x12])|(u32::from(c[7])&0x02)<<7|(u32::from(c[7])&0x40)<<3)+1;
    let scan=u32::from(c[9]&0x1f)+1;
    let double=if c[9]&0x80!=0{2}else{1};
    if gc[6]&1==0{
        let mode=if gc[6]&0x0c==0x08{7}else{3};
        return Display{mode,columns,rows:lines/(scan*double)};
    }
    let unknown=Display{mode:0xff,columns:80,rows:25};
    let height=lines/(scan*double);
    if v.attr[16]&0x40!=0{
        // 256 colours: four pixels per character clock, chain-4 linear.
        let chained=seq[4]&0x08!=0&&gc[5]&0x40!=0;
        return if chained&&columns*4==320&&height==200{Display{mode:0x13,columns:40,rows:25}}else{unknown};
    }
    let planar=seq[4]&0x0c==0x04&&gc[5]&0x60==0;
    let width=columns*8/if seq[1]&0x08!=0{2}else{1};
    let mode=match(planar,width,height){
        (true,320,200)=>0x0d,(true,640,200)=>0x0e,(true,640,350)=>0x10,(true,640,480)=>0x12,
        _=>return unknown,
    };
    let cell=match height{480=>16,350=>14,_=>8};
    Display{mode,columns:width/8,rows:height/cell}
}

