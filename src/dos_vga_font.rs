//! VGA font memory in the text modes. A VGA keeps the character generator in
//! plane 2; programs reach it by switching the sequencer and graphics
//! controller to plane addressing at A0000h (map mask 04h, odd/even off,
//! read map 2, memory map A0000h), copying 32-byte glyph slots, and switching
//! back to the text values. Norton SysInfo 8 does this to draw its own window
//! frames and check boxes.
//!
//! While the registers select that addressing (memory map A0000h, odd/even
//! off in both the sequencer and the graphics controller, chain-4 off), the
//! four planes are Ram's planar window, with the same latches, write modes,
//! map mask and read map as in the 16-colour modes (vga_planar). Otherwise
//! the planes are kept here and A0000h is ordinary RAM again, as it was in
//! the text modes before. A text-mode set starts a fresh, cleared set of
//! planes.
//!
//! Not modelled: the BIOS 8x16 font. Dine's text screen is drawn with the
//! host terminal's font, so plane 2 starts as zeros (no glyphs loaded) and
//! glyphs a program writes there are kept and read back but not shown.
//! Planes 0 and 1 are separate from the text buffer at B8000h. With the
//! 128 KiB map (GC 06h bits 2-3 = 00) only A0000h-AFFFFh reaches the planes;
//! B0000h-BFFFFh stays the text buffer.
use super::*;
use crate::vga_planar::Planar;
use super::vga::Controllers;
/// Whether the registers give plane-at-a-time access at A0000h.
pub(super) fn plane_window(c:&Controllers)->bool{
    c.text_map.is_some()&&c.gc[6]&0x0c<=0x04&&c.gc[6]&0x02==0&&c.seq[4]&0x0c==0x04&&c.gc[5]&0x10==0
}
/// Maps the planes at A0000h, keeps their registers in step, or unmaps
/// them, after a sequencer or graphics controller write in a text mode.
pub(super) fn sync(p:&mut Loaded){
    if p.vga.idle.text_map.is_none(){return;}
    let want=plane_window(&p.vga.idle);
    let registers=|planes:&mut Planar,c:&Controllers|{planes.seq=c.seq;planes.gc=c.gc;};
    match(p.ram.planar().is_some(),want){
        (false,true)=>{
            let mut planes=p.vga.font.take().unwrap_or_else(Planar::new);
            registers(&mut planes,&p.vga.idle);p.ram.set_planar(Some(planes));
        },
        (true,true)=>{let c=p.vga.idle.clone();registers(p.ram.planar_mut().unwrap(),&c);},
        (true,false)=>{p.vga.font=p.ram.planar().cloned();p.ram.set_planar(None);},
        (false,false)=>{},
    }
}
