// EGA/VGA graphics controller behaviour, with expected bytes worked out by
// hand from the register definitions (FreeVGA graphics registers), and the
// Ram window/alias routing.
use super::*;
use crate::memory::{Ram,MemWidth,PLANE_ALIAS};
fn filled(values:[u8;4])->Planar{let mut p=Planar::new();for n in 0..4{p.poke(n,7,values[n]);}p}
#[test]
fn reads_load_latches_and_select_a_plane_or_compare_colours(){
    let mut p=filled([0x0f,0x33,0x55,0xff]);
    p.gc[4]=2;assert_eq!(p.read(7),0x55);assert_eq!(p.latches(),[0x0f,0x33,0x55,0xff]);
    // Read mode 1, colour compare 0Fh: bits set in all planes,
    // 0Fh & 33h & 55h & FFh = 01h. Colour 1011b: p0 & p1 & !p2 & p3 = 02h.
    p.gc[5]=8;p.gc[2]=0x0f;assert_eq!(p.read(7),0x01);
    p.gc[2]=0x0b;assert_eq!(p.read(7),0x02);
    // Planes 2 and 3 don't care: compare planes 0/1 with 11b, 0Fh & 33h = 03h.
    p.gc[7]=0x03;p.gc[2]=0x03;assert_eq!(p.read(7),0x03);
}
#[test]
fn write_mode_zero_rotates_uses_set_reset_function_and_bit_mask(){
    let mut p=filled([0xff,0x00,0xf0,0x0f]);p.read(7);
    p.gc[3]=1;assert_eq!(p.peek(0,7),0xff);
    p.write(7,0x81);            // ROR 1 -> C0
    assert_eq!([0,1,2,3].map(|n|p.peek(n,7)),[0xc0;4]);
    // Set/reset 0101b enabled on planes 0 and 1; AND with latches; mask F0h.
    let mut p=filled([0xff,0xff,0x0f,0xaa]);p.read(7);
    p.gc[0]=0b0101;p.gc[1]=0b0011;p.gc[3]=1<<3;p.gc[8]=0xf0;
    p.write(7,0x3c);
    assert_eq!([0,1,2,3].map(|n|p.peek(n,7)),[0xff,0x0f,0x0f,0x2a]);
    // The map mask leaves disabled planes untouched.
    let mut p=filled([1,2,3,4]);p.read(7);p.seq[2]=0b1010;p.write(7,0xee);
    assert_eq!([0,1,2,3].map(|n|p.peek(n,7)),[1,0xee,3,0xee]);
}
#[test]
fn write_modes_one_two_and_three(){
    // Mode 1 copies the latches from offset 7 to offset 9, ignoring the mask.
    let mut p=filled([0x11,0x22,0x33,0x44]);p.read(7);p.gc[5]=1;p.gc[8]=0;p.write(9,0);
    assert_eq!([0,1,2,3].map(|n|p.peek(n,9)),[0x11,0x22,0x33,0x44]);
    // Mode 2: colour 0110b into the masked pixels, latches elsewhere.
    let mut p=filled([0xff,0x00,0xff,0x00]);p.read(7);p.gc[5]=2;p.gc[8]=0x18;p.write(7,0x06);
    assert_eq!([0,1,2,3].map(|n|p.peek(n,7)),[0xe7,0x18,0xff,0x00]);
    assert_eq!(p.pixel(7,3),6);assert_eq!(p.pixel(7,0),5);
    // Mode 3: set/reset colour, mask = bit mask AND rotated data, XOR function.
    let mut p=filled([0x0f,0,0,0]);p.read(7);p.gc[5]=3;p.gc[0]=0b0001;p.gc[3]=3<<3;p.gc[8]=0x3c;p.write(7,0xff);
    assert_eq!([0,1,2,3].map(|n|p.peek(n,7)),[0x33,0,0,0]);
}
#[test]
fn ram_routes_the_window_only_while_planar_memory_is_mapped(){
    let mut ram=Ram::new(0x110000);
    ram.store(0xa0005,MemWidth::Byte,0x5a).unwrap();assert_eq!(ram.bytes()[0xa0005],0x5a);
    assert!(ram.load(PLANE_ALIAS,MemWidth::Byte).is_err());
    ram.set_planar(Some(Planar::new()));
    ram.store(0xa0005,MemWidth::Word,0x1234).unwrap();
    assert_eq!(ram.bytes()[0xa0005],0x5a,"RAM behind the window is not written");
    for n in 0..4{assert_eq!(ram.planar().unwrap().peek(n,5),0x34);assert_eq!(ram.planar().unwrap().peek(n,6),0x12);}
    ram.planar_mut().unwrap().gc[4]=1;ram.store(PLANE_ALIAS+0x10000+5,MemWidth::Byte,0x77).unwrap();
    assert_eq!(ram.load(0xa0005,MemWidth::Byte).unwrap(),0x77);
    assert_eq!(ram.planar().unwrap().latches(),[0x34,0x77,0x34,0x34]);
    assert_eq!(ram.load(0xb0000,MemWidth::Byte).unwrap(),0,"outside the 64 KiB window");
    ram.set_planar(None);assert_eq!(ram.load(0xa0005,MemWidth::Byte).unwrap(),0x5a);
}
