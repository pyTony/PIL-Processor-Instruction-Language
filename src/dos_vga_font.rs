//! VGA font memory in the text modes. A VGA keeps the character generator in
//! plane 2; programs reach it by switching the sequencer and graphics
//! controller to plane addressing at A0000h (map mask 04h, odd/even off,
//! read map 2, memory map A0000h), copying 32-byte glyph slots, and switching
//! back to the text values, as programs that draw their own window frames
//! and check boxes do.
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
/// Write `count` glyphs of `height` bytes starting at `first` into plane 2
/// `block` (0..=7), zeroing unused rows in each 32-byte slot.
fn write_font_block(p:&mut Loaded,block:u32,first:u32,count:u32,height:u32,bytes:&[u8]){
    let mut font=p.ram.planar().cloned().or_else(||p.vga.font.clone()).unwrap_or_else(Planar::new);
    let base=(block as usize)*0x2000;
    for glyph in 0..count as usize{
        let slot=base+(first as usize+glyph)*32;
        for row in 0..32usize{
            let value=if row<height as usize{bytes[glyph*height as usize+row]}else{0};
            font.poke(2,slot+row,value);
        }
    }
    if p.ram.planar().is_some(){p.ram.set_planar(Some(font));}else{p.vga.font=Some(font);}
}
fn plan_recalc(p:&Loaded,height:u32)->Result<([(u32,MemWidth,u32);4],u16),String>{
    let flags=p.ram.load(0x489,MemWidth::Byte).map_err(|e|format!("VGA flags: {e:?}"))?;
    let scanlines=if flags&0x10!=0{400}else if flags&0x80!=0{200}else{350};
    let rows=(scanlines/height).max(1);
    let cols=p.ram.load(0x44a,MemWidth::Word).map_err(|e|format!("BIOS columns: {e:?}"))?.clamp(1,80);
    let page_bytes=(rows*cols*2+0xff)&!0xff;
    let cursor=(height.saturating_sub(2).min(31)<<8)|height.saturating_sub(1).min(31);
    let writes=[(0x44c,MemWidth::Word,page_bytes),(0x460,MemWidth::Word,cursor),(0x484,MemWidth::Byte,rows-1),(0x485,MemWidth::Word,height)];
    for &(at,width,_) in &writes{p.ram.check_write(at,width).map_err(|e|format!("font BDA write: {e:?}"))?;}
    Ok((writes,cursor as u16))
}
fn apply_recalc(p:&mut Loaded,writes:[(u32,MemWidth,u32);4],height:u32,cursor:u16)->Result<(),String>{
    for &(at,width,value) in &writes{p.ram.store(at,width,value).map_err(|e|format!("font BDA write: {e:?}"))?;}
    p.vga.set_char_height(height as u8,cursor);
    p.dos.mouse.sync_text_rows((writes[2].2+1) as u8);
    Ok(())
}
/// INT 10/AX=1100h (load only) and AX=1110h (load and recalculate text mode).
pub(super) fn load_user_patterns(p:&mut Loaded,recalc:bool)->Result<(),String>{
    let mode=p.ram.load(0x449,MemWidth::Byte).map_err(|e|format!("font mode: {e:?}"))?;
    let height=(p.cpu.regs[3]>>8)&255;let block=p.cpu.regs[3]&255;
    let current=p.ram.load(0x485,MemWidth::Word).map_err(|e|format!("font height: {e:?}"))?;
    if !matches!(mode,0..=3|7)||p.vga.idle.text_map.is_none()||block>=8||height==0||height>32||(!recalc&&height!=current){
        return Err(format!("INT 10h AX=11{:02X}h requires text mode, font block 0..7 and valid cell height",p.cpu.regs[0]&255));
    }
    let(count,first,offset)=(p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff,p.cpu.regs[5]&0xffff);
    if first+count>256||offset+count*height>0x10000{return Err("BIOS font table outside glyph or source bounds".into());}
    let source=MemoryContext{segment:SegmentRegister::Es,cache:p.image.segments[SegmentRegister::Es.index()]};
    let mut bytes=Vec::with_capacity((count*height)as usize);
    for i in 0..count*height{let at=source.translate(offset+i,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("font source: {e:?}"))?;bytes.push(p.ram.load(at,MemWidth::Byte).map_err(|e|format!("font source: {e:?}"))?as u8);}
    let plan=if recalc{Some(plan_recalc(p,height)?)}else{None};
    write_font_block(p,block,first,count,height,&bytes);
    if let Some((writes,cursor))=plan{apply_recalc(p,writes,height,cursor)?;}
    Ok(())
}
/// INT 10/AX=1101h/1102h/1104h (load ROM 8x14, 8x8, 8x16) and
/// AX=1111h/1112h/1114h (load ROM font and recalculate text mode).
pub(super) fn load_rom_font(p:&mut Loaded,height:u32,recalc:bool)->Result<(),String>{
    let mode=p.ram.load(0x449,MemWidth::Byte).map_err(|e|format!("font mode: {e:?}"))?;
    let block=p.cpu.regs[3]&255;
    if !matches!(mode,0..=3|7)||p.vga.idle.text_map.is_none()||block>=8{
        return Err(format!("INT 10h AX=11{:02X}h requires text mode and font block 0..7",p.cpu.regs[0]&255));
    }
    let bytes=match height{
        8=>super::console::graphics_text::font()?,
        14=>super::vga_text::font14()?,
        16=>super::vga_text::font()?,
        _=>return Err(format!("ROM font height {height} is not present")),
    };
    let plan=if recalc{Some(plan_recalc(p,height)?)}else{None};
    write_font_block(p,block,0,256,height,&bytes);
    if let Some((writes,cursor))=plan{apply_recalc(p,writes,height,cursor)?;}
    Ok(())
}
/// INT 10/AX=1103: BL is the character map select value (sequencer register
/// 03h: the font blocks for attribute bit 3 clear and set), as the VGA BIOS
/// writes it. Text modes only. It is recorded like a program's own write to
/// that register; the host-font frontend draws the same glyphs whichever
/// block is selected, as it does for glyphs loaded with AX=1100h.
pub(super) fn select_blocks(p:&mut Loaded)->Result<(),String>{
    if p.vga.idle.text_map.is_none(){return Err("INT 10h AX=1103h outside a text mode is not modelled".into());}
    let bl=p.cpu.regs[3]as u8;
    p.vga.idle.seq[3]=bl;
    if let Some(planes)=p.ram.planar_mut(){planes.seq[3]=bl;}
    Ok(())
}
#[cfg(test)]
#[path="../tests/unit/dos_vga_font.rs"]
mod tests;
