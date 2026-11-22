//! Text-mode font memory: plane 2 at A0000h while the registers select it.
use super::*;
use super::super::{load,run,Stop};
// MOV DX,3C4h; map mask 04h; memory mode 07h; MOV DX,3CEh; mode 00h;
// miscellaneous 04h (A0000h, 64 KiB); read map 02h.
const OPEN:&[u8]=&[0xba,0xc4,0x03,0xb8,0x02,0x04,0xef,0xb8,0x04,0x07,0xef,
    0xba,0xce,0x03,0xb8,0x05,0x00,0xef,0xb8,0x06,0x04,0xef,0xb8,0x04,0x02,0xef];
// Back to the text values: map mask 03h, memory mode 03h, mode 10h,
// miscellaneous 0Eh (B8000h), read map 00h.
const CLOSE:&[u8]=&[0xba,0xc4,0x03,0xb8,0x02,0x03,0xef,0xb8,0x04,0x03,0xef,
    0xba,0xce,0x03,0xb8,0x05,0x10,0xef,0xb8,0x06,0x0e,0xef,0xb8,0x04,0x00,0xef];
fn program(parts:&[&[u8]])->Vec<u8>{
    // MOV AX,A000h; MOV ES,AX first; CLI; HLT last.
    let mut code=vec![0xb8,0x00,0xa0,0x8e,0xc0];for part in parts{code.extend_from_slice(part);}code.extend_from_slice(&[0xfa,0xf4]);code
}
#[test]
fn a_text_mode_program_writes_and_reads_a_glyph_in_plane_two_and_a000_is_ram_again_after(){
    // In the window: MOV CH,[ES:0820h] (glyph 41h, row 0); MOV byte [ES:0820h],7Eh;
    // MOV BL,[ES:0820h]. After CLOSE: MOV CL,[ES:0820h]; MOV byte [ES:0820h],55h.
    let code=program(&[OPEN,&[0x26,0x8a,0x2e,0x20,0x08,0x26,0xc6,0x06,0x20,0x08,0x7e,0x26,0x8a,0x1e,0x20,0x08],
        CLOSE,&[0x26,0x8a,0x0e,0x20,0x08,0x26,0xc6,0x06,0x20,0x08,0x55]]);
    let mut p=load(&code,"font.com").unwrap();p.ram.store(0xa0820,MemWidth::Byte,0x99).unwrap();
    let r=run(p,60).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let p=&r.loaded;let c=&p.cpu;
    assert_eq!(((c.regs[1]>>8)&255,c.regs[3]&255,c.regs[1]&255),(0,0x7e,0x99),"plane 2 starts empty; RAM again after");
    assert!(p.ram.planar().is_none());
    let font=p.vga.font.as_ref().unwrap();
    assert_eq!([0,1,2,3].map(|n|font.peek(n,0x820)),[0,0,0x7e,0],"only plane 2 (map mask 04h) was written");
    assert_eq!(p.ram.load(0xa0820,MemWidth::Byte).unwrap(),0x55);
    assert_eq!((p.vga.idle.seq[4],p.vga.idle.gc[5],p.vga.idle.gc[6]),(3,0x10,0x0e));
}
#[test]
fn the_glyph_stays_across_windows_and_a_text_mode_set_clears_the_font(){
    // Write the glyph, close, open again and read it into BL; then INT 10h AX=0003h.
    let code=program(&[OPEN,&[0x26,0xc6,0x06,0x20,0x08,0x7e],CLOSE,OPEN,&[0x26,0x8a,0x1e,0x20,0x08],CLOSE,&[0xb8,0x03,0x00,0xcd,0x10]]);
    let r=run(load(&code,"font2.com").unwrap(),80).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.cpu.regs[3]&255,0x7e);
    assert!(r.loaded.vga.font.is_none()&&r.loaded.ram.planar().is_none());
    // The window only opens once odd/even is off and A0000h is selected.
    let code=program(&[&[0xba,0xce,0x03,0xb8,0x06,0x04,0xef]]);
    let r=run(load(&code,"half.com").unwrap(),20).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert!(r.loaded.ram.planar().is_none());
}
#[test]
fn chain4_graphics_and_the_mono_map_stay_refused_in_text_mode_with_nothing_changed(){
    // Memory mode 0Eh (chain-4); miscellaneous 05h (graphics); 0Ah (B0000h).
    for (port,ax) in [(0x3c4u16,0x0e04u16),(0x3ce,0x0506),(0x3ce,0x0a06)]{
        let [p0,p1]=port.to_le_bytes();let [a0,a1]=ax.to_le_bytes();
        let code=program(&[&[0xba,p0,p1,0xb8,a0,a1,0xef]]);
        let r=run(load(&code,"bad.com").unwrap(),20).unwrap();
        assert!(matches!(&r.stop,Stop::Service(e) if e.contains("outside the 16-colour planar modes")),"{ax:04x}: {:?}",r.stop);
        assert_eq!((r.loaded.vga.idle.seq[4],r.loaded.vga.idle.gc[6]),(2,0x0e));
        assert!(r.loaded.ram.planar().is_none());
    }
}
#[test]
fn bios_font_block_select_records_the_character_map_in_text_modes(){
 let mut p=load(&[0xcd,0x10],"block.com").unwrap();
 // AX=1103h BL=00h (the standard blocks) and BL=05h; AX and BX unchanged.
 for bl in [0u32,5]{
  p.cpu.regs[0]=0x1103;p.cpu.regs[3]=bl;service(&mut p,0x10,&mut Vec::new()).unwrap();
  assert_eq!((p.vga.idle.seq[3],p.cpu.regs[0],p.cpu.regs[3]),(bl as u8,0x1103,bl));
 }
 // A program may write the register itself in a text mode (3C4h/3C5h).
 let code=[0xba,0xc4,0x03,0xb8,0x03,0x00,0xef,0xfa,0xf4];
 let r=run(load(&code,"seq3.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert_eq!(r.loaded.vga.idle.seq[3],0);
 // Outside the text modes it stops, as before.
 let mut p=load(&[0xcd,0x10],"block13.com").unwrap();p.cpu.regs[0]=0x0013;service(&mut p,0x10,&mut Vec::new()).unwrap();
 p.cpu.regs[0]=0x1103;p.cpu.regs[3]=0;assert!(service(&mut p,0x10,&mut Vec::new()).unwrap_err().contains("1103h"));
}
#[test]
fn rom_font_load_1112_switches_to_80x50_and_1114_restores_80x25(){
 let mut p=load(&[0xcd,0x10],"font50.com").unwrap();
 // AX=1112h BL=00h: 8x8 ROM font + 80x50 recalculation.
 p.cpu.regs[0]=0x1112;p.cpu.regs[3]=0;
 service(&mut p,0x10,&mut Vec::new()).unwrap();
 assert_eq!(p.ram.load(0x484,MemWidth::Byte).unwrap(),49);
 assert_eq!(p.ram.load(0x485,MemWidth::Word).unwrap(),8);
 assert_eq!(p.ram.load(0x44c,MemWidth::Word).unwrap(),0x2000);
 assert_eq!(p.ram.load(0x460,MemWidth::Word).unwrap(),0x0607);
 // Write a character on row 49 via INT 10h AH=02h + AH=09h.
 p.cpu.regs[0]=0x0200;p.cpu.regs[3]=0;p.cpu.regs[2]=(49<<8)|5;
 service(&mut p,0x10,&mut Vec::new()).unwrap();
 p.cpu.regs[0]=0x0941;p.cpu.regs[3]=0x000e;p.cpu.regs[1]=1;
 service(&mut p,0x10,&mut Vec::new()).unwrap();
 assert_eq!(p.ram.load(0xb8000+(49*80+5)*2,MemWidth::Word).unwrap(),0x0e41);
 // Plane 2 holds the 8x8 ROM glyph for 'A' (41h) with rows 8..31 zeroed.
 let font=p.vga.font.as_ref().unwrap();
 assert!((0..8).any(|r|font.peek(2,0x41*32+r)!=0));
 assert!((8..32).all(|r|font.peek(2,0x41*32+r)==0));
 // AX=1114h BL=00h restores 80x25 (height 16).
 p.cpu.regs[0]=0x1114;p.cpu.regs[3]=0;
 service(&mut p,0x10,&mut Vec::new()).unwrap();
 assert_eq!(p.ram.load(0x484,MemWidth::Byte).unwrap(),24);
 assert_eq!(p.ram.load(0x485,MemWidth::Word).unwrap(),16);
 assert_eq!(p.ram.load(0x44c,MemWidth::Word).unwrap(),0x1000);
}
#[test]
fn rom_font_load_01_02_04_keeps_rows_and_1111_sets_28_rows(){
 let mut p=load(&[0xcd,0x10],"font28.com").unwrap();
 // AX=1102h BL=01h loads 8x8 into block 1 without changing 80x25 BDA.
 p.cpu.regs[0]=0x1102;p.cpu.regs[3]=1;
 service(&mut p,0x10,&mut Vec::new()).unwrap();
 assert_eq!(p.ram.load(0x484,MemWidth::Byte).unwrap(),24);
 assert_eq!(p.ram.load(0x485,MemWidth::Word).unwrap(),16);
 let font=p.vga.font.as_ref().unwrap();
 assert!((0..8).any(|r|font.peek(2,0x2000+0x41*32+r)!=0));
 // AX=1111h BL=00h loads 8x14 -> 400/14 = 28 rows.
 p.cpu.regs[0]=0x1111;p.cpu.regs[3]=0;
 service(&mut p,0x10,&mut Vec::new()).unwrap();
 assert_eq!(p.ram.load(0x484,MemWidth::Byte).unwrap(),27);
 assert_eq!(p.ram.load(0x485,MemWidth::Word).unwrap(),14);
}
