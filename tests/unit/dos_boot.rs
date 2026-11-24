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
/// kernel at physical 0); the front ends lay the screen out by the mode the
/// hardware was programmed for. The boot sector prints 'L', fills
/// 0000:0400-04FF with 01h and stops.
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
    let display=console::shown_mode(&p).unwrap();
    assert_eq!((display.mode,display.columns),(3,80));
    let session=session::Session::new(p);
    let (cells,columns,cursor)=session.screen().unwrap();
    assert_eq!(columns,80);
    assert_eq!(cells[0],b'L');
    // The cursor after 'L', from the CRTC cursor location register.
    assert_eq!(cursor,(0,1));
}

/// The cursor shape the window draws comes from the CRTC cursor registers.
#[test]
fn the_cursor_shape_survives_an_os_writing_over_the_bios_data_area(){
    let code:&[u8]=&[
        0x31,0xdb,0x8e,0xc3,                       // xor bx,bx; mov es,bx
        0xbf,0x00,0x04,0xb9,0x00,0x01,0xb0,0x01,   // mov di,400h; mov cx,100h; mov al,1
        0xfc,0xf3,0xaa,0xfa,0xf4,                  // cld; rep stosb; cli; hlt
    ];
    let mut disk=vec![0u8;1_474_560];
    disk[..code.len()].copy_from_slice(code);
    disk[510]=0x55;disk[511]=0xaa;
    let r=run(boot(&disk,0).unwrap(),2000).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    // BIOS shape 06h-07h in 8 lines is 13-14 in a 16-line cell.
    assert_eq!(host_console::cursor_shape(&r.loaded),Some((13,14)));
}

/// IN or OUT through DX in a booted machine: EC/ED in, EE out; AL or AX.
fn port_io(p:&mut Loaded,first:u8,port:u16,value:u32)->u32{
    p.cpu.regs[0]=value;ports::transfer(p,first,port,if first&1==0{1}else{2}).unwrap();p.cpu.regs[0]
}

#[test]
fn probes_of_absent_isa_cards_read_the_open_bus(){
    let mut disk=vec![0u8;1_228_800];disk[510]=0x55;disk[511]=0xaa;
    let mut p=boot(&disk,0).unwrap();
    // A word probe of an Ethernet card's data port: FFFFh, writes go nowhere.
    assert_eq!(port_io(&mut p,0xed,0x300,0x1234_0000),0x1234_ffff);
    port_io(&mut p,0xef,0x300,0x5555);
    // Sound Blaster addresses the card does not decode: FFh, writes ignored,
    // and its DSP still answers a reset.
    for port in [0x220,0x223,0x227,0x22b,0x22d]{
        assert_eq!(port_io(&mut p,0xec,port,0)&0xff,0xff,"{port:03x}h");
        port_io(&mut p,0xee,port,0x5a);
    }
    port_io(&mut p,0xee,0x226,1);port_io(&mut p,0xee,0x226,0);
    assert_eq!(port_io(&mut p,0xec,0x22a,0)&0xff,0xaa);
    // Outside a booted machine an unassigned word port still stops.
    let mut q=boot(&disk,0).unwrap();q.booted=false;q.cpu.regs[0]=0;
    assert!(ports::transfer(&mut q,0xed,0x300,2).is_err());
}

#[test]
fn the_keyboard_interface_holds_keys_while_the_controller_disables_it(){
    let mut disk=vec![0u8;1_228_800];disk[510]=0x55;disk[511]=0xaa;
    let mut p=boot(&disk,0).unwrap();
    // Interrupts off: a port 64h read brings a waiting code in (polling).
    p.cpu.eflags&=!0x200;
    port_io(&mut p,0xee,0x64,0xad);
    assert!(keyboard::key(&mut p,0x1e61).unwrap());
    assert_eq!(port_io(&mut p,0xec,0x64,0)&1,0,"held while the interface is off");
    assert!(!keyboard::interrupt(&mut p,true).unwrap(),"no IRQ1 for a held code");
    port_io(&mut p,0xee,0x64,0xae);
    assert_eq!(port_io(&mut p,0xec,0x64,0)&1,1);
    assert_eq!(port_io(&mut p,0xec,0x60,0)&0xff,0x1e);
}

#[test]
fn a_diskette_change_raises_the_change_line_until_the_head_steps(){
    let mut disk=vec![0u8;1_228_800];disk[510]=0x55;disk[511]=0xaa;
    let mut p=boot(&disk,0).unwrap();
    let mut other=vec![0u8;1_228_800];other[512]=0x77;
    // Only a diskette of the drive's format, into a drive that is there.
    assert!(p.disks.change_floppy(0,bios_disk::Disk::floppy(vec![0;1_474_560]).unwrap()).is_err());
    assert!(p.disks.change_floppy(1,bios_disk::Disk::floppy(other.clone()).unwrap()).is_err());
    assert!(!p.disks.changed(0));
    p.disks.change_floppy(0,bios_disk::Disk::floppy(other).unwrap()).unwrap();
    assert_eq!(p.disks.sector(0,1).unwrap()[0],0x77,"the new diskette's data");
    // INT 13h AH=16h: 06h (CF set) once, then 00h.
    for expect in [(6,1),(0,0)]{
        p.cpu.regs[0]=0x1600;p.cpu.regs[2]=0;bios_disk::int13(&mut p).unwrap();
        assert_eq!(((p.cpu.regs[0]>>8)&0xff,p.cpu.eflags&1),expect);
    }
    // The controller's digital input register (3F7h bit 7) for the drive
    // the DOR selects, until a seek.
    p.disks.change_floppy(0,bios_disk::Disk::floppy(vec![0;1_228_800]).unwrap()).unwrap();
    fdc::Fdc::io(&mut p,0x3f2,false,0x1c).unwrap();
    assert_eq!(fdc::Fdc::io(&mut p,0x3f7,true,0).unwrap(),Some(0x80));
    fdc::Fdc::io(&mut p,0x3f2,false,0x1d).unwrap();
    assert_eq!(fdc::Fdc::io(&mut p,0x3f7,true,0).unwrap(),Some(0),"drive B: selected");
    fdc::Fdc::io(&mut p,0x3f2,false,0x1c).unwrap();
    for b in [0x0f,0x00,0x01]{fdc::Fdc::io(&mut p,0x3f5,false,b).unwrap();}
    assert_eq!(fdc::Fdc::io(&mut p,0x3f7,true,0).unwrap(),Some(0));
}

#[test]
fn the_console_changes_a_booted_machines_diskette(){
    let mut disk=vec![0u8;1_228_800];disk[510]=0x55;disk[511]=0xaa;
    let dir=std::env::temp_dir().join(format!("pil-diskettes-{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (first,second)=(dir.join("boot.img"),dir.join("root.img"));
    let mut root=vec![0u8;1_228_800];root[0]=0x42;
    std::fs::write(&first,&disk).unwrap();std::fs::write(&second,&root).unwrap();
    let mut session=session::Session::new(boot(&disk,0).unwrap());
    session.boot_diskettes(vec![first,second]).unwrap();
    assert!(session.disk(b'A',Some("next")).unwrap().contains("2/2"));
    let p=session.machine().unwrap();
    assert!(p.disks.changed(0));assert_eq!(p.disks.sector(0,0).unwrap()[0],0x42);
    assert!(session.disk(b'A',Some("next")).is_err(),"no third diskette");
    // Dine's own DOS has no BIOS diskette to change this way.
    let dos=crate::dos::load(&[0xc3],"x.com").unwrap();
    assert!(session::Session::new(dos).boot_diskettes(vec![dir.join("boot.img")]).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
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

/// A boot sector that programs the CRTC itself: cursor off through register
/// 0Ah (bit 5) and the start address through 0Ch/0Dh, then a word written
/// to 0040:004A. The display follows the registers and ignores the data
/// area, as the hardware does.
#[test]
fn a_booted_screen_follows_the_crtc_registers_not_the_bios_data_area(){
    let code:&[u8]=&[
        0xba,0xd4,0x03,                            // mov dx,3D4h
        0xb8,0x0a,0x20,0xef,                       // mov ax,200Ah; out dx,ax (cursor off)
        0xb8,0x0c,0x00,0xef,                       // mov ax,000Ch; out dx,ax (start high 00h)
        0xb8,0x0d,0x50,0xef,                       // mov ax,500Dh; out dx,ax (start low 50h: row 1)
        0x31,0xdb,0x8e,0xc3,                       // xor bx,bx; mov es,bx
        0x26,0xc7,0x06,0x4a,0x04,0x28,0x00,        // mov word [es:44Ah],40
        0xb8,0x00,0xb8,0x8e,0xc0,                  // mov ax,0B800h; mov es,ax
        0x26,0xc6,0x06,0xa0,0x00,b'R',             // mov byte [es:0A0h],'R' (row 1, column 0)
        0xfa,0xf4,                                 // cli; hlt
    ];
    let mut disk=vec![0u8;1_474_560];
    disk[..code.len()].copy_from_slice(code);
    disk[510]=0x55;disk[511]=0xaa;
    let r=run(boot(&disk,0).unwrap(),2000).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(host_console::cursor_shape(&r.loaded),None);
    let session=session::Session::new(r.loaded);
    let (cells,columns,_)=session.screen().unwrap();
    assert_eq!(columns,80);
    assert_eq!(cells[0],b'R');
}

#[test]
fn probe_scripts_parse_their_steps_and_refuse_unknown_ones(){
    let steps=parse_script("# comment\nprompt >\nscreen login:\ntype ls -l\\r\ntype a\\\\b\\s\nrun 1_000\nshow\nmark compile\n").unwrap();
    assert_eq!(steps,vec![
        ScriptStep::Prompt(">".into()),ScriptStep::Screen("login:".into()),
        ScriptStep::Type("ls -l\r".into()),ScriptStep::Type("a\\b ".into()),
        ScriptStep::Run(1000),ScriptStep::Show,ScriptStep::Mark("compile".into()),
    ]);
    assert!(parse_script("wait 5\n").unwrap_err().contains("line 1"));
    assert!(parse_script("run many\n").is_err());
    assert_eq!(parse_script("shot a.ppm\n").unwrap(),vec![ScriptStep::Shot("a.ppm".into())]);
    assert!(parse_script("shot\n").unwrap_err().contains("file name"));
}

#[test]
fn a_script_shot_writes_the_graphics_screen_and_refuses_text(){
    // Mode 13h through INT 10h, pixel (1,0) in colour 4 (red), then a loop.
    let code:&[u8]=&[
        0xb8,0x13,0x00,0xcd,0x10,                  // mov ax,13h; int 10h
        0xb8,0x00,0xa0,0x8e,0xc0,                  // mov ax,A000h; mov es,ax
        0x26,0xc6,0x06,0x01,0x00,0x04,             // mov byte [es:1],4
        0xeb,0xfe,                                 // jmp $
    ];
    let mut disk=vec![0u8;1_474_560];
    disk[..code.len()].copy_from_slice(code);disk[510]=0x55;disk[511]=0xaa;
    let file=std::env::temp_dir().join(format!("pil-shot-{}.ppm",std::process::id()));
    let name=file.to_string_lossy().into_owned();
    let script=parse_script(&format!("run 100000\nshot {name}\n")).unwrap();
    let report=script_probe(boot(&disk,0).unwrap(),1_000_000,&script,None,None).unwrap();
    assert!(report.contains(": 320x200, mode 13h,"),"{report}");
    let ppm=std::fs::read(&file).unwrap();std::fs::remove_file(&file).unwrap();
    let header=b"P6\n320 200\n255\n";
    assert_eq!(&ppm[..header.len()],header);
    assert_eq!(ppm.len(),header.len()+320*200*3);
    assert_eq!(&ppm[header.len()..header.len()+6],&[0,0,0,0xaa,0,0]);
    // A text screen has no picture: the script stops and says so.
    let mut text=vec![0u8;1_474_560];
    text[..2].copy_from_slice(&[0xeb,0xfe]);text[510]=0x55;text[511]=0xaa;
    let script=parse_script(&format!("run 1000\nshot {name}\n")).unwrap();
    let report=script_probe(boot(&text,0).unwrap(),1_000_000,&script,None,None).unwrap();
    assert!(report.contains("no graphics screen (display mode 03h)"),"{report}");
    assert!(!file.exists());
}

#[test]
fn a_script_types_into_a_booted_machine_and_waits_for_the_screen(){
    // A boot sector that echoes each key through INT 16h/INT 10h, after a
    // '>' prompt, forever.
    let code:&[u8]=&[
        0xb4,0x0e,0xb0,b'>',0xcd,0x10,             // mov ah,0Eh; mov al,'>'; int 10h
        0x31,0xc0,0xcd,0x16,                       // xor ax,ax; int 16h
        0xb4,0x0e,0xcd,0x10,                       // mov ah,0Eh; int 10h
        0xeb,0xf6,                                 // jmp back to xor ax,ax
    ];
    let mut disk=vec![0u8;1_474_560];
    disk[..code.len()].copy_from_slice(code);disk[510]=0x55;disk[511]=0xaa;
    let script=parse_script("prompt >\ntype hi\nscreen >hi\nmark typed\nshow\n").unwrap();
    let report=script_probe(boot(&disk,0).unwrap(),2_000_000,&script,None,None).unwrap();
    assert!(report.contains("-- mark typed:"),"{report}");
    assert!(report.lines().any(|l|l==">hi"),"{report}");
    assert!(report.contains("script done"),"{report}");
}

#[test]
fn a_tar_floppy_holds_a_ustar_archive_with_correct_checksums(){
    let dir=std::env::temp_dir().join(format!("pil-tar-floppy-{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file=dir.join("hello.c");std::fs::write(&file,b"main(){}\n").unwrap();
    let image=tar_floppy(&[file.to_string_lossy().into_owned()],1_228_800).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(image.len(),1_228_800);
    assert_eq!(&image[..7],b"hello.c");
    assert_eq!(&image[257..263],b"ustar\0");
    assert_eq!(&image[124..136],b"00000000011\0");
    let mut header=image[..512].to_vec();header[148..156].copy_from_slice(b"        ");
    let sum:u32=header.iter().map(|&b|u32::from(b)).sum();
    assert_eq!(&image[148..156],format!("{sum:06o}\0 ").as_bytes());
    assert_eq!(&image[512..521],b"main(){}\n");
    assert!(image[1024..].iter().all(|&b|b==0));
    let big=dir.with_extension("big");
    assert!(tar_floppy(&[big.to_string_lossy().into_owned()],1_228_800).is_err());
}
