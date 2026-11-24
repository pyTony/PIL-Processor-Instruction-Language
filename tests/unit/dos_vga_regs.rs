use super::*;
use crate::pc::vga_regs::*;

/// CRTC 00h-18h, attribute 00h-14h, graphics controller 00h-08h and
/// sequencer 00h-04h of the standard VGA modes 13h and 12h (IBM VGA
/// register values), and the miscellaneous output.
const MODE13:([u8;25],[u8;21],[u8;9],[u8;5],u8)=(
    [0x5f,0x4f,0x50,0x82,0x54,0x80,0xbf,0x1f,0x00,0x41,0,0,0,0,0,0,0x9c,0x8e,0x8f,0x28,0x40,0x96,0xb9,0xa3,0xff],
    [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0x41,0,0x0f,0,0],
    [0,0,0,0,0,0x40,0x05,0x0f,0xff],[0x03,0x01,0x0f,0x00,0x0e],0x63);
const MODE12:([u8;25],[u8;21],[u8;9],[u8;5],u8)=(
    [0x5f,0x4f,0x50,0x82,0x54,0x80,0x0b,0x3e,0x00,0x40,0,0,0,0,0,0,0xea,0x8c,0xdf,0x28,0x00,0xe7,0x04,0xe3,0xff],
    [0,1,2,3,4,5,0x14,7,0x38,0x39,0x3a,0x3b,0x3c,0x3d,0x3e,0x3f,0x01,0,0x0f,0,0],
    [0,0,0,0,0,0,0x05,0x0f,0xff],[0x03,0x01,0x0f,0x00,0x06],0xe3);

/// (port, value) pairs a register-level mode set writes, in svgalib's
/// order: miscellaneous output, sequencer under synchronous reset, CRTC
/// (unprotected first), graphics controller, attribute controller, then
/// the attribute index with bit 5 set (display on). A port with bit 15 set
/// is a word OUT of index and data.
fn mode_set(m:&([u8;25],[u8;21],[u8;9],[u8;5],u8))->Vec<(u16,u16)>{
    let (crtc,attr,gc,seq,misc)=m;
    let mut out=vec![(0x3c2,u16::from(*misc)),(0x83c4,0x0100)];
    for i in 1..5{out.push((0x83c4,u16::from(seq[i])<<8|i as u16));}
    out.push((0x83c4,0x0300));
    out.push((0x83d4,0x0e11));
    for (i,v) in crtc.iter().enumerate(){out.push((0x83d4,u16::from(*v)<<8|i as u16));}
    for (i,v) in gc.iter().enumerate(){out.push((0x83ce,u16::from(*v)<<8|i as u16));}
    for (i,v) in attr.iter().enumerate(){out.push((0x3c0,i as u16));out.push((0x3c0,u16::from(*v)));}
    out.push((0x3c0,0x20));
    out
}

/// A 1.44 MB image whose boot sector reads the (port, value) table from
/// sectors 2-3 to 0000:7E00 through INT 13h, plays it, then
/// stores `pixel` bytes (offset, value) through ES=A000h and stops.
fn image(table:&[(u16,u16)],extra:&[(u16,u16)],pixels:&[(u16,u8)])->Vec<u8>{
    let mut code=vec![
        0x31,0xc0,0x8e,0xc0,        // xor ax,ax; mov es,ax
        0xb8,0x02,0x02,0xbb,0x00,0x7e, // mov ax,0202h; mov bx,7E00h
        0xb9,0x02,0x00,0xba,0x00,0x00, // mov cx,0002h; mov dx,0
        0xcd,0x13,                  // int 13h
        0xbe,0x00,0x7e,             // mov si,7E00h
        0xad,                       // next: lodsw (port)
        0x89,0xc2,                  // mov dx,ax
        0x85,0xc0,                  // test ax,ax
        0x74,0x0f,                  // jz done
        0xad,                       // lodsw (value)
        0xf6,0xc6,0x80,             // test dh,80h
        0x75,0x03,                  // jnz word
        0xee,                       // out dx,al
        0xeb,0xf0,                  // jmp next
        0x80,0xe6,0x7f,             // word: and dh,7Fh
        0xef,                       // out dx,ax
        0xeb,0xea,                  // jmp next
        0xb8,0x00,0xa0,0x8e,0xc0,   // done: mov ax,A000h; mov es,ax
    ];
    for &(at,v) in pixels{let [lo,hi]=at.to_le_bytes();code.extend_from_slice(&[0x26,0xc6,0x06,lo,hi,v]);}
    code.extend_from_slice(&[0xfa,0xf4]);
    assert!(code.len()<0x100);
    let mut disk=vec![0u8;1_474_560];
    disk[..code.len()].copy_from_slice(&code);
    let mut at=0x200;
    for &(port,value) in table.iter().chain(extra){
        disk[at..at+2].copy_from_slice(&port.to_le_bytes());disk[at+2..at+4].copy_from_slice(&value.to_le_bytes());at+=4;
    }
    assert!(at<0x600);
    disk[510]=0x55;disk[511]=0xaa;
    disk
}

#[test]
fn a_booted_machine_sets_mode_13h_through_the_registers_and_draws_chained_pixels(){
    let disk=image(&mode_set(&MODE13),&[],&[(5,0x2a),(320,0x07)]);
    let mut p=boot(&disk,0).unwrap();
    p.select_fpu(false).ok();
    let r=run(p,4000).unwrap();
    assert!(!matches!(r.stop,Stop::Fault(_)|Stop::Service(_)|Stop::MissingEncoding{..}),"{:?}",r.stop);
    let p=&r.loaded;
    assert_eq!(p.vga.display,vga::Display{mode:0x13,columns:40,rows:25});
    // Chain-4: byte n lands in plane n&3 at n>>2.
    let planes=p.ram.planar().expect("planes at A0000h");
    assert_eq!((planes.peek(1,1),planes.peek(0,80)),(0x2a,0x07));
    let (w,h,frame)=desktop_vga_frame(p).unwrap();
    assert_eq!((w,h),(320,200));
    let rgb=|i:usize|{let [r,g,b]=p.vga.dac(i).map(|c|u32::from((u16::from(c)*255/63)as u8));r<<16|g<<8|b};
    assert_eq!((frame[5],frame[320]),(rgb(0x2a),rgb(7)));
}

#[test]
fn a_booted_machine_sets_mode_12h_writes_planes_and_returns_to_text(){
    // Mode 12h, map mask plane 2 only, then two pixels; then the text
    // registers of mode 3 (GC 06h = 0Eh puts B8000h back as RAM).
    let back=[(0x83c4,0x0402),(0x83c4,0x0204),(0x83ce,0x1005),(0x83ce,0x0e06)];
    let disk=image(&mode_set(&MODE12),&[(0x83c4,0x0402)],&[(0,0x80),(80,0x01)]);
    let mut p=boot(&disk,0).unwrap();p.select_fpu(false).ok();
    let r=run(p,4000).unwrap();
    let p=&r.loaded;
    assert_eq!(p.vga.display,vga::Display{mode:0x12,columns:80,rows:30});
    let planes=p.ram.planar().unwrap();
    assert_eq!((planes.peek(2,0),planes.peek(0,0),planes.peek(2,80)),(0x80,0,0x01));
    let (w,h,frame)=desktop_vga_frame(p).unwrap();
    assert_eq!((w,h),(640,480));
    let colors=p.vga.planar_rgb().map(|[r,g,b]|u32::from(r)<<16|u32::from(g)<<8|u32::from(b));
    assert_eq!((frame[0],frame[1],frame[640+7]),(colors[4],colors[0],colors[4]));
    // Back to text: the planes are kept aside and B8000h is RAM again.
    let disk=image(&mode_set(&MODE12),&back,&[]);
    let mut q=boot(&disk,0).unwrap();q.select_fpu(false).ok();
    let r=run(q,4000).unwrap();
    assert!(r.loaded.ram.planar().is_none());
    assert_eq!(r.loaded.vga.display.mode,3);
}

#[test]
fn register_states_without_a_bios_mode_show_nothing(){
    // Unchained 256 colours at 320x240 (mode X): not one of the modes the
    // front ends draw, so the display is FFh, not the last mode.
    let mut v=vga::Vga::default();
    v.idle.gc=[0,0,0,0,0,0x40,0x05,0x0f,0xff];v.idle.seq=[3,1,0x0f,0,0x06];v.attr[16]=0x41;
    v.free=true;
    for (i,x) in [(0x11u8,0x0e),(6,0x0d),(7,0x3e),(9,0x41),(0x10,0xea),(0x11,0xac),(0x12,0xdf),(0x15,0xe7),(0x16,0x06)]{
        v.io(0x3d4,false,i).unwrap();v.io(0x3d5,false,x).unwrap();
    }
    assert_eq!(classify(&v).mode,0xff);
    assert!(plane_window(&v));
}
