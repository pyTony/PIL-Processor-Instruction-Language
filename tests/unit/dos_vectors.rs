//! Interrupt vectors and the 4DOS increment's services, ports and timer.
use super::*;
use super::super::{load,run,Stop,PSP};
fn code_at(bytes:&[(usize,&[u8])])->Vec<u8>{
    let mut code=vec![0x90;0x40];
    for(at,b)in bytes{if code.len()<at+b.len(){code.resize(at+b.len(),0x90);}code[*at..at+b.len()].copy_from_slice(b);}
    code
}
#[test]
fn answered_vectors_start_at_read_only_entries_and_others_keep_their_values(){
    let p=load(&[0xcd,0x20],"v.com").unwrap();
    for n in ANSWERED{
        assert_eq!(p.ram.load(u32::from(n)*4,MemWidth::Dword).unwrap(),0xf000_0000|(0xf000+3*u32::from(n)),"INT {n:02x}h");
        assert!(answered(&p,n).unwrap());
    }
    for n in 0..=255u8{
        let at=ENTRIES+3*u32::from(n);
        let b:Vec<u32>=(0..3).map(|i|p.ram.load(at+i,MemWidth::Byte).unwrap()).collect();
        assert_eq!(b,vec![0xcd,u32::from(n),0xcf]);
        assert!(p.ram.check_write(at,MemWidth::Byte).is_err(),"entries are in the read-only BIOS area");
    }
    // No mouse driver, user and free vectors, BIOS data pointers: still 0.
    for n in [0x1d,0x1e,0x1f,0x33,0x41,0x60,0x67]{assert_eq!(p.ram.load(n*4,MemWidth::Dword).unwrap(),0,"INT {n:02x}h");}
    assert_eq!(p.ram.load(0x23*4,MemWidth::Dword).unwrap(),0x0070_0074);
}
#[test]
fn a_hooked_int21_runs_the_handler_which_chains_to_dine_with_the_service_flags(){
    // Save the old INT 21h vector at 0200h, hook it with 0130h, then CLC and
    // INT 21h AX=4452h (the DR-DOS check: CF=1, AX=1). PUSHF/POP CX keeps the
    // flags the caller sees. The handler sets BX and jumps to the saved vector.
    let code=code_at(&[(0,&[0xb8,0x21,0x35,0xcd,0x21,0x89,0x1e,0x00,0x02,0x8c,0x06,0x02,0x02,0xba,0x30,0x01,0xb8,0x21,0x25,0xcd,0x21,
        0xb8,0x52,0x44,0xf8,0xcd,0x21,0x9c,0x59,0xfa,0xf4]),(0x30,&[0xbb,0x34,0x12,0x2e,0xff,0x2e,0x00,0x02])]);
    let mut p=load(&code,"hook.com").unwrap();p.cpu.eflags=0x202;let sp=p.cpu.regs[4];
    let r=run(p,100).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let c=&r.loaded.cpu;
    assert_eq!((c.regs[0]&0xffff,c.regs[3]&0xffff,c.regs[4]),(1,0x1234,sp));
    assert_eq!(c.regs[1]&0x201,0x201,"CF from the service, IF from the caller");
    assert_eq!(r.instructions.get("INT 21h to the program's handler"),Some(&1));
    assert_eq!(r.instructions.get("IRET (Dine interrupt entry)"),Some(&1));
    assert_eq!(r.loaded.ram.load(0x21*4,MemWidth::Dword).unwrap(),u32::from(PSP)<<16|0x130);
    assert_eq!(r.loaded.ram.load(u32::from(PSP)*16+0x200,MemWidth::Dword).unwrap(),entry(0x21));
}
#[test]
fn a_handler_that_answers_itself_returns_with_iret_and_entry_frames_are_exact(){
    // Hook INT 2Fh with MOV AX,44DDh; IRET; then INT 2Fh AX=D44Dh.
    let code=code_at(&[(0,&[0xba,0x30,0x01,0xb8,0x2f,0x25,0xcd,0x21,0xb8,0x4d,0xd4,0xcd,0x2f,0xfa,0xf4]),(0x30,&[0xb8,0xdd,0x44,0xcf])]);
    let mut p=load(&code,"own.com").unwrap();p.cpu.eflags=0x202;
    let r=run(p,4).unwrap();assert!(matches!(r.stop,Stop::Budget));
    // After MOV DX, MOV AX, INT 21h (direct), MOV AX: the next step delivers.
    let mut p=r.loaded;let sp=p.cpu.regs[4]&0xffff;p.cpu.eflags|=0x100;
    let flags=p.cpu.eflags&0xffff;deliver(&mut p,0x2f).unwrap();
    let ss=u32::from(PSP)*16;let word=|p:&Loaded,o:u32|p.ram.load(ss+((sp-o)&0xffff),MemWidth::Word).unwrap();
    assert_eq!((word(&p,2),word(&p,4),word(&p,6)),(flags,u32::from(PSP),0x10d));
    assert_eq!((p.cpu.regs[4]&0xffff,p.selectors[SegmentRegister::Cs.index()],p.cpu.eip,p.cpu.eflags&0x300),(sp-6,PSP,0x130,0));
    let r=run(p,10).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)));
    assert_eq!((r.loaded.cpu.regs[0]&0xffff,r.loaded.cpu.regs[4]&0xffff),(0x44dd,sp));
}
#[test]
fn delivery_faults_change_nothing(){
    // A handler outside the program (the BIOS area), and a stack that would wrap.
    for(vector,sp)in [(0xf000_0000u32,0xfffe),(u32::from(PSP)<<16|0x110,4)]{
        let mut p=load(&[0xcd,0x2f,0xfa,0xf4],"bad.com").unwrap();
        p.ram.store(0x2f*4,MemWidth::Dword,vector).unwrap();p.cpu.regs[4]=sp;
        let(cpu,ram,image)=(p.cpu.clone(),p.ram.clone(),p.image.clone());
        let r=run(p,5).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
        assert_eq!((r.loaded.cpu,r.loaded.ram,r.loaded.image),(cpu,ram,image));
    }
    // A read-only stack word stops before any change.
    let mut p=load(&[0xcd,0x2f,0xfa,0xf4],"ro.com").unwrap();
    p.ram.store(0x2f*4,MemWidth::Dword,u32::from(PSP)<<16|0x102).unwrap();
    let top=u32::from(PSP)*16+(p.cpu.regs[4]&0xffff);p.ram.protect(top-6,2).unwrap();
    let(cpu,ram)=(p.cpu.clone(),p.ram.clone());
    let r=run(p,5).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
    assert_eq!((r.loaded.cpu,r.loaded.ram),(cpu,ram));
}
#[test]
fn timer_ticks_reach_the_bda_only_with_the_host_clock_if_and_bios_handlers(){
    // XOR AX,AX; MOV ES,AX; MOV AX,[ES:046Ch]; CLI; HLT.
    let code=[0x31,0xc0,0x8e,0xc0,0x26,0xa1,0x6c,0x04,0xfa,0xf4];
    let fresh=|clock:Option<i64>,flags:u32|{let mut p=load(&code,"t.com").unwrap();p.clock_offset=clock;p.cpu.eflags=flags;
        p.ram.store(0x46c,MemWidth::Dword,0x1111).unwrap();p.timer_ticks=u32::MAX;p};
    // Deterministic clock, or IF=0: the word stays as it was.
    for p in [fresh(None,0x202),fresh(Some(0),0x002)]{let r=run(p,10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x1111);}
    let r=run(fresh(Some(0),0x202),10).unwrap();
    let ms=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()as u64%86_400_000;
    let now=(ms*1_193_180/(65_536*1000))as u32;let got=r.loaded.ram.load(0x46c,MemWidth::Dword).unwrap();
    assert!(got<=now&&now-got<=1,"{got} {now}");assert_eq!(r.loaded.cpu.regs[0]&0xffff,got&0xffff);
    // The tick went back from the old count, so midnight passed.
    assert_eq!(r.loaded.ram.load(0x470,MemWidth::Byte).unwrap(),1);
    // A program's INT 08h or 1Ch handler is not called: the run stops.
    for n in [0x08u32,0x1c]{
        let mut p=fresh(Some(0),0x202);p.ram.store(n*4,MemWidth::Dword,u32::from(PSP)<<16|0x108).unwrap();
        let r=run(p,10).unwrap();assert!(matches!(&r.stop,Stop::Service(m) if m.contains("IRQ0")),"{:?}",r.stop);
    }
}
#[test]
fn pit_channel0_latches_reads_and_takes_only_the_bios_mode(){
    // Latch; IN twice into BL, BH; mode 34h; count 0000h; CLI; HLT.
    let code=[0xb0,0x00,0xe6,0x43,0xe4,0x40,0x88,0xc3,0xe4,0x40,0x88,0xc7,0xb0,0x34,0xe6,0x43,0xb0,0x00,0xe6,0x40,0xe6,0x40,0xfa,0xf4];
    let mut p=load(&code,"pit.com").unwrap();p.cpu.regs[3]=0xabcd;
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    // Time moves a whole tick at a time without the host clock: count 0.
    assert_eq!(r.loaded.cpu.regs[3]&0xffff,0);
    let mut p=load(&code,"pit.com").unwrap();p.clock_offset=Some(0);
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    // Reading channel 0 while its new count is half written stops the run.
    let r=run(load(&[0xb0,0x34,0xe6,0x43,0xb0,0x00,0xe6,0x40,0xe4,0x40,0xfa,0xf4],"half.com").unwrap(),20).unwrap();
    assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
    // Mode 3 and a BCD or other-mode command: 36h is taken, 35h and 30h are not.
    let r=run(load(&[0xb0,0x36,0xe6,0x43,0xb0,0x00,0xe6,0x40,0xe6,0x40,0xfa,0xf4],"m3.com").unwrap(),20).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    for mode in [0x35,0x30]{let r=run(load(&[0xb0,mode,0xe6,0x43,0xfa,0xf4],"m.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{mode:02x}");}
}
fn dos(p:&mut Loaded,ax:u32,bx:u32,cx:u32,dx:u32)->Result<(u32,u32,u32,u32),String>{
    p.cpu.regs[0]=ax;p.cpu.regs[3]=bx;p.cpu.regs[1]=cx;p.cpu.regs[2]=dx;p.cpu.eflags=0x202;
    let mut out=Vec::new();super::super::service(p,0x21,&mut out)?;assert!(out.is_empty());
    Ok((p.cpu.regs[0],p.cpu.regs[3],p.cpu.regs[2],p.cpu.eflags&1))
}
fn name(p:&mut Loaded,text:&[u8]){let mut z=text.to_vec();z.push(0);p.ram.write_bytes(u32::from(PSP)*16+0x200,&z).unwrap();}
#[test]
fn nul_takes_writes_reads_empty_and_is_a_character_device(){
    let mut p=load(&[0xcd,0x20],"n.com").unwrap();
    for path in [&b"NUL"[..],b"nul.txt",b"A:\\ANY\\NUL"]{
        name(&mut p,path);let(ax,..)=dos(&mut p,0x3d02,0,0,0x200).unwrap();assert_eq!(ax&0xffff,5,"lowest free handle");
        assert_eq!(dos(&mut p,0x4000,5,7,0x200).unwrap().0&0xffff,7);
        assert_eq!(dos(&mut p,0x3f00,5,7,0x200).unwrap().0&0xffff,0);
        assert_eq!(dos(&mut p,0x4400,5,0,0).unwrap().2&0xffff,0xc4);
        assert_eq!(dos(&mut p,0x3e00,5,0,0).unwrap().3,0);
    }
    // Create opens it too; AH5Bh finds it exists.
    name(&mut p,b"NUL");assert_eq!(dos(&mut p,0x3c00,0,0,0x200).unwrap().0&0xffff,5);
    let(ax,_,_,cf)=dos(&mut p,0x5b00,0,0,0x200).unwrap();assert_eq!((ax&0xffff,cf),(0x50,1));
    // Standard output on NUL drops AH09 text.
    dos(&mut p,0x3e00,1,0,0).unwrap();assert_eq!(dos(&mut p,0x3d01,0,0,0x200).unwrap().0&0xffff,1);
    name(&mut p,b"hidden$");assert_eq!(dos(&mut p,0x0900,0,0,0x200).unwrap().0&0xff,0x24);
}
#[test]
fn code_page_remote_drive_dr_dos_and_date_checks_answer_as_dos_6_22(){
    let mut p=load(&[0xcd,0x20],"q.com").unwrap();
    assert_eq!(dos(&mut p,0x6601,0xffff_ffff,0,0x1234_0000).unwrap(),(0x6601,0xffff_01b5,0x1234_01b5,0));
    assert_eq!(dos(&mut p,0x6602,437,0,437).unwrap().3,0);
    assert!(dos(&mut p,0x6602,850,0,850).is_err());
    assert_eq!(dos(&mut p,0x6603,0,0,0).unwrap(),(1,0,0,1));
    // AX=4409h: A: (BL=1, and BL=0 the default) is local; C: does not exist.
    assert_eq!(dos(&mut p,0x4409,1,0,0xffff).unwrap(),(0x4409,1,0,0));
    assert_eq!(dos(&mut p,0x4409,0,0,0xffff).unwrap().3,0);
    assert_eq!(dos(&mut p,0x4409,3,0,0xffff).unwrap(),(0x0f,3,0xffff,1));
    // IOCTL past 11h is an invalid function (the DR-DOS check); 10h stops.
    for al in [0x12,0x52,0xff]{assert_eq!(dos(&mut p,0x4400|al,0,0,0).unwrap(),(1,0,0,1));}
    assert!(dos(&mut p,0x4410,0,0,0).is_err());
    // AH2Bh: an invalid date (the DESQview check, 29 Feb 2001, 1979, 2100)
    // gives AL=FFh; a valid one would change the clock and stops.
    for(cx,dx)in [(0x4445,0x5351),(2001,0x021d),(1979,0x0101),(2100,0x0101),(1990,0x0d01),(1990,0x0100)]{
        assert_eq!(dos(&mut p,0x2b00,0,cx,dx).unwrap(),(0x2bff,0,dx,0),"{cx} {dx:04x}");
    }
    for(cx,dx)in [(2000,0x021d),(1980,0x0101),(2099,0x0c1f)]{assert!(dos(&mut p,0x2b00,0,cx,dx).is_err());}
    // AH50h: the current PSP may be set to a segment holding a PSP.
    assert_eq!(dos(&mut p,0x5000,u32::from(PSP),0,0).unwrap().0,0x5000);
    assert!(dos(&mut p,0x5000,0x2000,0,0).is_err());assert_eq!(p.psp,PSP);
}
#[test]
fn multiplex_checks_for_4dos_windows_mscdex_and_append_leave_registers(){
    let mut p=load(&[0xcd,0x20],"m.com").unwrap();
    for ax in [0x4680u32,0x168f,0x1500,0x150b,0xb706,0xd44d,0xd44e]{
        p.cpu.regs[0]=ax;p.cpu.regs[3]=0x8d;p.cpu.regs[1]=3;let cpu=p.cpu.clone();
        assert_eq!(super::super::service(&mut p,0x2f,&mut Vec::new()).unwrap(),None);assert_eq!(p.cpu,cpu,"{ax:04x}");
    }
}
#[test]
fn ascii_adjust_instructions_match_the_sdm_and_aam_zero_faults(){
    // AX=02FBh: AAA -> 0401h (CF=AF=1); AAS -> 02FBh-106h=01F5h&FF0Fh=0105h
    // (CF=AF=1); AAM 10 -> AL=5: 0005h; AAD 10 -> 5; AAM 7 -> 0005h; AAD 7.
    for(code,ax,flags,want,cf)in [(&[0x37][..],0x02fb,0,0x0401,1),(&[0x3f],0x02fb,0,0x0105,1),(&[0x37],0x0204,0,0x0204,0),(&[0x37],0x0204,0x10,0x030a,1),
        (&[0xd4,0x0a],0x1263,0,0x0909,0),(&[0xd4,0x07],0x0063,0,0x0e01,0),(&[0xd5,0x0a],0x0909,0,0x0063,0),(&[0xd5,0x07],0x0e01,0,0x0063,0),(&[0xd5,0x0a],0x1999,0,0x0093,0)]{
        let mut bytes=code.to_vec();bytes.extend([0xfa,0xf4]);
        let mut p=load(&bytes,"a.com").unwrap();p.cpu.regs[0]=0xabcd_0000|ax;p.cpu.eflags=0x202|flags;
        let r=run(p,5).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)));
        assert_eq!((r.loaded.cpu.regs[0],r.loaded.cpu.eflags&1),(0xabcd_0000|want,cf),"{code:02x?} {ax:04x}");
    }
    // AAM 0 is a divide error: nothing changes.
    let mut p=load(&[0xd4,0x00,0xf4],"z.com").unwrap();p.cpu.regs[0]=0x55;let cpu=p.cpu.clone();
    let r=run(p,5).unwrap();assert!(matches!(r.stop,Stop::Fault(_)),"{:?}",r.stop);assert_eq!(r.loaded.cpu,cpu);
}
#[test]
fn parent_interrupt_handler_can_be_called_by_child_program(){
    use std::time::{SystemTime,UNIX_EPOCH};
    use super::super::load_with_drives;
    use super::super::files::Source;
    let dir=std::env::temp_dir().join(format!("pil-inthook-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    // CHILD.COM: MOV AX, 0xD44D; INT 2Fh; MOV AH, 4Ch; INT 21h
    std::fs::write(dir.join("child.com"),[0xb8,0x4d,0xd4,0xcd,0x2f,0xb4,0x4c,0xcd,0x21]).unwrap();
    let mut parent=vec![
        0xbb,0x00,0x01,0xb4,0x4a,0xcd,0x21, // AH4A resize to 100h paras
        0xba,0x70,0x01,0xb8,0x2f,0x25,0xcd,0x21, // AH25 hook INT 2Fh to 170h
        0x8c,0x0e,0x44,0x01,0x8c,0x0e,0x48,0x01,0x8c,0x0e,0x4c,0x01, // CS into exec block
        0xba,0x60,0x01,0xbb,0x40,0x01,0xb8,0x00,0x4b,0xcd,0x21, // AH4B EXEC CHILD.COM
        0xb4,0x4d,0xcd,0x21,0x89,0xc6,0xfa,0xf4, // AH4D get return code into SI; HLT
    ];
    parent.resize(0x40,0);parent.extend([0,0,0x50,0x01,0,0,0x50,0x01,0,0,0x50,0x01,0,0]);
    parent.resize(0x50,0);parent.extend([0,0x0d]);
    parent.resize(0x60,0);parent.extend(b"CHILD.COM\0");
    parent.resize(0x70,0);parent.extend([0x3d,0x4d,0xd4,0x75,0x02,0xb0,0x77,0xcf,0xcf]);
    let p=load_with_drives(&parent,"PARENT.COM",vec![Source::Host(dir.clone())],None).unwrap();
    let r=run(p,200).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.cpu.regs[6]&0xffff,0x77,"child called parent INT 2Fh and exited with 77h");
    assert_eq!((r.loaded.psp,r.loaded.exec.len()),(PSP,0));
    std::fs::remove_dir_all(dir).unwrap();
}
