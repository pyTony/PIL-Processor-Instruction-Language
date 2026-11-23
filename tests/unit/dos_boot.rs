use super::*;
use crate::dos::{run,Stop};

/// A 1.44 MB image whose boot sector prints 'B' through INT 10h, reads
/// sector 2 (CHS 0/0/2) to 0000:8000 and prints its first byte, then asks
/// for two sectors at 0000:FF00 (across a 64 KiB DMA page) and stores the
/// AX it gets at 0000:9000, then stops with CLI; HLT.
fn image()->Vec<u8>{
    let code:&[u8]=&[
        0xb4,0x0e,0xb0,b'B',0xcd,0x10,             // mov ah,0Eh; mov al,'B'; int 10h
        0xb8,0x01,0x02,0xb9,0x02,0x00,             // mov ax,0201h; mov cx,0002h
        0xba,0x00,0x00,0xbb,0x00,0x80,0xcd,0x13,   // mov dx,0; mov bx,8000h; int 13h
        0xa0,0x00,0x80,0xb4,0x0e,0xcd,0x10,        // mov al,[8000h]; mov ah,0Eh; int 10h
        0xb8,0x02,0x02,0xb9,0x02,0x00,             // mov ax,0202h; mov cx,0002h
        0xbb,0x00,0xff,0xcd,0x13,                  // mov bx,0FF00h; int 13h
        0xa3,0x00,0x90,0xfa,0xf4,                  // mov [9000h],ax; cli; hlt
    ];
    let mut disk=vec![0u8;1_474_560];
    disk[..code.len()].copy_from_slice(code);
    disk[510]=0x55;disk[511]=0xaa;
    disk[512]=b'X';
    disk
}

#[test]
fn a_boot_sector_runs_with_bios_disk_and_video_services(){
    let p=boot(&image(),0).unwrap();
    assert!(p.booted);
    assert_eq!((p.cpu.eip,p.cpu.regs[2]&0xff,p.cpu.regs[4]&0xffff),(0x7c00,0,0x7c00));
    // DOS vectors are a real 0000:0000 once a disk has booted.
    assert_eq!(p.ram.load(0x21*4,MemWidth::Dword).unwrap(),0);
    assert_eq!(p.ram.load(0x13*4,MemWidth::Dword).unwrap(),vectors::entry(0x13));
    let r=run(p,1000).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let p=r.loaded;
    let screen=screen_lines(&p);
    assert!(screen[0].starts_with("BX"),"{screen:?}");
    // The transfer across the DMA page fails with status 09h and no sectors.
    assert_eq!(p.ram.load(0x9000,MemWidth::Word).unwrap(),0x0900);
    assert_eq!(p.ram.load(0x441,MemWidth::Byte).unwrap(),0x09);
}

#[test]
fn drive_parameters_cmos_type_and_equipment_follow_the_image(){
    let mut disk=vec![0u8;1_228_800];disk[510]=0x55;disk[511]=0xaa;
    let mut p=boot(&disk,0).unwrap();
    // AH=08h: CH max cylinder 79, CL 15 sectors, DH max head 1, DL one drive,
    // BL drive type 2 (1.2M), ES:DI the parameter table.
    p.cpu.regs[0]=0x0800;p.cpu.regs[2]=0;
    bios_disk::int13(&mut p).unwrap();
    assert_eq!((p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff,p.cpu.regs[3]&0xff,p.cpu.eflags&1),(0x4f0f,0x0101,2,0));
    assert_eq!((p.selectors[SegmentRegister::Es.index()],p.cpu.regs[7]&0xffff),(0xf000,0xefc7));
    assert_eq!(p.ram.load(0xfefc7+4,MemWidth::Byte).unwrap(),15);
    // CMOS register 10h: A: is a 1.2M drive, B: absent.
    assert_eq!(p.disks.cmos_floppy_types(),0x20);
    // Equipment word: one diskette drive.
    assert_eq!(p.ram.load(0x410,MemWidth::Word).unwrap()&0xc1,0x01);
    // No second drive: a time-out, CF set.
    p.cpu.regs[0]=0x0201;p.cpu.regs[1]=1;p.cpu.regs[2]=1;
    bios_disk::int13(&mut p).unwrap();
    assert_eq!(((p.cpu.regs[0]>>8)&0xff,p.cpu.eflags&1),(0x80,1));
}

#[test]
fn images_of_other_sizes_are_refused(){
    assert!(boot(&vec![0u8;1_000_000],0).is_err());
    assert!(bios_disk::Disk::floppy(vec![0;737_280]).is_ok());
    assert!(bios_disk::Disk::fixed(vec![0;16*63*512*20]).is_ok());
    assert!(bios_disk::Disk::fixed(vec![0;1024]).is_err());
}

#[test]
fn typed_text_reaches_int_16h_with_shift_for_capitals(){
    // Three INT 16h AH=00h reads, their AX words stored at 0000:0600; CLI; HLT.
    let code:&[u8]=&[
        0x31,0xc0,0xcd,0x16,0xa3,0x00,0x06,
        0x31,0xc0,0xcd,0x16,0xa3,0x02,0x06,
        0x31,0xc0,0xcd,0x16,0xa3,0x04,0x06,0xfa,0xf4,
    ];
    let mut disk=vec![0u8;1_474_560];disk[..code.len()].copy_from_slice(code);disk[510]=0x55;disk[511]=0xaa;
    let mut session=session::Session::new(boot(&disk,0).unwrap());
    // More keys than the keyboard queue holds: typing waits for room.
    type_text(&mut session,"aB?").unwrap();
    session.run(10_000).unwrap();
    let p=session.machine().unwrap();
    let words:Vec<u32>=(0..3).map(|i|p.ram.load(0x600+2*i,MemWidth::Word).unwrap()).collect();
    assert_eq!(words,vec![0x1e61,0x3042,0x353f]);
}

/// An operating system may reuse the BIOS data area (Linux 0.99 has its
/// kernel at physical 0); the screen layout the front ends read is the copy
/// the video BIOS kept. The boot sector prints 'L', fills 0000:0400-04FF
/// with 01h and stops.
#[test]
fn the_screen_layout_survives_an_os_writing_over_the_bios_data_area(){
    let code:&[u8]=&[
        0xb4,0x0e,0xb0,b'L',0xcd,0x10,             // mov ah,0Eh; mov al,'L'; int 10h
        0x31,0xdb,0x8e,0xc3,                       // xor bx,bx; mov es,bx
        0xbf,0x00,0x04,0xb9,0x00,0x01,0xb0,0x01,   // mov di,400h; mov cx,100h; mov al,1
        0xfc,0xf3,0xaa,0xfa,0xf4,                  // cld; rep stosb; cli; hlt
    ];
    let mut disk=vec![0u8;1_474_560];
    disk[..code.len()].copy_from_slice(code);
    disk[510]=0x55;disk[511]=0xaa;
    let r=run(boot(&disk,0).unwrap(),2000).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let p=r.loaded;
    assert_eq!(p.ram.load(0x44a,MemWidth::Word).unwrap(),0x0101);
    assert_eq!(console::bios_video(&p,0x449,MemWidth::Byte).unwrap(),3);
    assert_eq!(console::bios_video(&p,0x44a,MemWidth::Word).unwrap(),80);
    let session=session::Session::new(p);
    let (cells,columns,_)=session.screen().unwrap();
    assert_eq!(columns,80);
    assert_eq!(cells[0],b'L');
}

/// `--rtc-year`: the CMOS year and century read through ports 70h/71h are
/// the chosen year's, for kernels that cannot read this century.
#[test]
fn the_rtc_runs_in_the_chosen_year(){
    let code:&[u8]=&[
        0xb0,0x09,0xe6,0x70,0xe4,0x71,0xa2,0x00,0x90,   // year -> [9000h]
        0xb0,0x32,0xe6,0x70,0xe4,0x71,0xa2,0x01,0x90,   // century -> [9001h]
        0xfa,0xf4,                                      // cli; hlt
    ];
    let mut disk=vec![0u8;1_474_560];
    disk[..code.len()].copy_from_slice(code);
    disk[510]=0x55;disk[511]=0xaa;
    let mut p=boot(&disk,0).unwrap();
    p.clock_offset=Some(0);
    assert!(p.set_rtc_year(1979).is_err());
    p.set_rtc_year(1993).unwrap();
    let r=run(p,1000).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.ram.load(0x9000,MemWidth::Word).unwrap(),0x1993);
}
