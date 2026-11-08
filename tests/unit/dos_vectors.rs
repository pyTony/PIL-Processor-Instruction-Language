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
    // INT 1Fh points to the upper half of the BIOS graphics font.
    assert_eq!(p.ram.load(0x1f*4,MemWidth::Dword).unwrap(),super::super::console::graphics_text::UPPER_FONT);
    // Other unimplemented BIOS data pointers, mouse and free vectors stay 0.
    for n in [0x1d,0x1e,0x33,0x41,0x60,0x67]{assert_eq!(p.ram.load(n*4,MemWidth::Dword).unwrap(),0,"INT {n:02x}h");}
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
    // A handler outside the program (the BIOS area), and a stack whose words
    // would straddle the end of the segment (SP odd and below 6: shutdown).
    for(vector,sp)in [(0xf000_0000u32,0xfffe),(u32::from(PSP)<<16|0x110,5)]{
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
    let now=(ms*1_193_182/(65_536*1000))as u32%super::super::TICKS_PER_DAY;let got=r.loaded.ram.load(0x46c,MemWidth::Dword).unwrap();
    let delta=got.abs_diff(now);assert!(delta.min(super::super::TICKS_PER_DAY-delta)<=2,"{got} {now}");assert_eq!(r.loaded.cpu.regs[0]&0xffff,got&0xffff);
    // The tick went back from the old count, so midnight passed.
    assert_eq!(r.loaded.ram.load(0x470,MemWidth::Byte).unwrap(),1);
    // A program's INT 08h handler takes IRQ0 instead of this BIOS shortcut:
    // here the vector points at CLI; HLT, which halts inside the handler.
    let mut p=fresh(Some(0),0x202);p.ram.store(0x08*4,MemWidth::Dword,u32::from(PSP)<<16|0x108).unwrap();
    let r=run(p,10).unwrap();assert!(matches!(&r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.ram.load(0x46c,MemWidth::Dword).unwrap(),0x1111,"no chaining, so no BIOS tick");
    assert_eq!(r.loaded.cpu.eip,0x109);assert_eq!(r.loaded.speaker.pic.read(0x20)&1,0,"acknowledged, IRR clear");
}
/// A COM program whose main code is CLI; HLT, with the old INT 08h vector
/// saved at 0200h, a tick counter at 0210h and the handler at 0130h.
fn irq0_program(handler:&[u8])->Loaded{
    let code=code_at(&[(0,&[0xfa,0xf4]),(0x30,handler)]);
    let mut p=load(&code,"irq0.com").unwrap();p.cpu.eflags=0x202;
    let ss=u32::from(PSP)*16;
    p.ram.store(ss+0x200,MemWidth::Dword,entry(8)).unwrap();p.ram.store(ss+0x210,MemWidth::Byte,0).unwrap();
    p.ram.store(0x08*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    p.ram.store(0x46c,MemWidth::Dword,0x1111).unwrap();
    p
}
// INC BYTE [0210h]; MOV AL,20h; OUT 20h,AL; IRET
const OWN_EOI:&[u8]=&[0xfe,0x06,0x10,0x02,0xb0,0x20,0xe6,0x20,0xcf];
#[test]
fn irq0_enters_a_program_int08_handler_with_an_exact_frame_and_its_eoi_ends_service(){
    let mut p=irq0_program(OWN_EOI);p.cpu.eflags=0x2d7;
    let(ip,sp,flags,cs)=(p.cpu.eip,p.cpu.regs[4]&0xffff,p.cpu.eflags,p.selectors[SegmentRegister::Cs.index()]);
    super::super::services::timer_interrupt_at(&mut p,42).unwrap();
    let ss=u32::from(PSP)*16;let word=|p:&Loaded,o:u32|p.ram.load(ss+((sp-o)&0xffff),MemWidth::Word).unwrap();
    assert_eq!((word(&p,2),word(&p,4),word(&p,6)),(flags&0xffff,u32::from(cs),ip));
    assert_eq!((p.cpu.eip,p.cpu.regs[4]&0xffff,p.cpu.eflags&0x300),(0x130,sp-6,0));
    // In service until the EOI; the BIOS count is the handler's business.
    p.speaker.pic.write(0x20,0x0b).unwrap();assert_eq!(p.speaker.pic.read(0x20)&1,1);p.speaker.pic.write(0x20,0x0a).unwrap();
    assert_eq!(p.ram.load(0x46c,MemWidth::Dword).unwrap(),0x1111);assert_eq!(p.timer_ticks,0);
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let mut p=r.loaded;assert_eq!(p.ram.load(ss+0x210,MemWidth::Byte).unwrap(),1);
    assert_eq!(p.cpu.regs[4]&0xffff,sp);assert_eq!(p.cpu.eflags&0xdd5,flags&0xdd5,"IRET restored them; only the final CLI changed IF");
    p.speaker.pic.write(0x20,0x0b).unwrap();assert_eq!(p.speaker.pic.read(0x20)&1,0);
}
#[test]
fn irq0_waits_while_masked_in_service_or_with_if_clear(){
    // A handler without EOI: the next edge stays pending in IRR.
    let mut p=irq0_program(&[0xfe,0x06,0x10,0x02,0xcf]);
    super::super::services::timer_interrupt_at(&mut p,42).unwrap();assert_eq!(p.cpu.eip,0x130);
    let r=run(p,2).unwrap();let mut p=r.loaded;assert_eq!(p.cpu.eip,0x100);
    let before=(p.cpu.clone(),p.ram.clone());
    super::super::services::timer_interrupt_at(&mut p,43).unwrap();
    assert_eq!((p.cpu.clone(),p.ram.clone()),before);assert_eq!(p.speaker.pic.read(0x20)&1,1,"IRR pending");
    // EOI, then the pending request is delivered at the next boundary.
    p.speaker.pic.write(0x20,0x20).unwrap();super::super::services::timer_interrupt_at(&mut p,43).unwrap();assert_eq!(p.cpu.eip,0x130);
    // Masked at the PIC, or IF=0: nothing is delivered, the request waits.
    for(mask,flags)in [(1,0x202),(0,0x002)]{
        let mut p=irq0_program(OWN_EOI);p.cpu.eflags=flags;p.speaker.pic.write(0x21,mask).unwrap();
        let before=(p.cpu.clone(),p.ram.clone());
        super::super::services::timer_interrupt_at(&mut p,42).unwrap();
        assert_eq!((p.cpu.clone(),p.ram.clone()),before);assert_eq!(p.speaker.pic.read(0x20)&1,1);
        p.speaker.pic.write(0x21,0).unwrap();p.cpu.eflags=0x202;
        super::super::services::timer_interrupt_at(&mut p,42).unwrap();assert_eq!(p.cpu.eip,0x130,"the same edge, once allowed");
    }
}
#[test]
fn a_chained_bios_handler_counts_one_tick_runs_int1c_and_sends_the_eoi(){
    // PUSHF; CALL FAR CS:[0200h]; INC BYTE [0210h]; IRET, and the
    // tail-chaining form INC BYTE [0210h]; JMP FAR CS:[0200h].
    let call:&[u8]=&[0x9c,0x2e,0xff,0x1e,0x00,0x02,0xfe,0x06,0x10,0x02,0xcf];
    let jump:&[u8]=&[0xfe,0x06,0x10,0x02,0x2e,0xff,0x2e,0x00,0x02];
    for handler in [call,jump]{
        let mut p=irq0_program(handler);
        // An INT 1Ch hook at 0140h: INC BYTE [0211h]; IRET.
        let ss=u32::from(PSP)*16;
        for(k,b)in [0xfeu32,0x06,0x11,0x02,0xcf].into_iter().enumerate(){p.ram.store(ss+0x140+k as u32,MemWidth::Byte,b).unwrap();}
        p.ram.store(ss+0x211,MemWidth::Byte,0).unwrap();p.ram.store(0x1c*4,MemWidth::Dword,u32::from(PSP)<<16|0x140).unwrap();
        p.cpu.regs[0]=0xdeadbeef;let(sp,flags)=(p.cpu.regs[4],p.cpu.eflags);
        super::super::services::timer_interrupt_at(&mut p,42).unwrap();
        let r=run(p,60).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
        let p=&r.loaded;
        assert_eq!(p.ram.load(0x46c,MemWidth::Dword).unwrap(),0x1112,"one tick, not the host edge total");
        assert_eq!(p.timer_ticks,0x1112);assert_eq!(p.ram.load(0x470,MemWidth::Byte).unwrap(),0);
        assert_eq!((p.ram.load(ss+0x210,MemWidth::Byte).unwrap(),p.ram.load(ss+0x211,MemWidth::Byte).unwrap()),(1,1));
        assert_eq!(r.instructions.get("BIOS INT 08h (timer tick)"),Some(&1));
        assert_eq!((p.cpu.regs[0],p.cpu.regs[4],p.cpu.eflags&0xdd5),(0xdeadbeef,sp,flags&0xdd5));
        let mut pic=p.speaker.pic.clone();pic.write(0x20,0x0b).unwrap();assert_eq!(pic.read(0x20)&1,0,"BIOS EOI");
    }
}
#[test]
fn the_bios_tick_wraps_at_a_day_and_a_software_int08_runs_it_with_a_frame(){
    // INT 08h through the BIOS vector, then CLI; HLT.
    let mut p=load(&[0xcd,0x08,0xfa,0xf4],"soft.com").unwrap();p.cpu.eflags=0x202;
    p.ram.store(0x46c,MemWidth::Dword,0x18_00af).unwrap();let sp=p.cpu.regs[4];
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.ram.load(0x46c,MemWidth::Dword).unwrap(),0);assert_eq!(r.loaded.ram.load(0x470,MemWidth::Byte).unwrap(),1);
    assert_eq!((r.loaded.cpu.eip,r.loaded.cpu.regs[4]),(0x103,sp));
    assert_eq!(r.instructions.get("INT 08h to the BIOS handler"),Some(&1));
    // A read-only tick count stops before anything changes.
    let mut p=irq0_program(&[0x2e,0xff,0x2e,0x00,0x02]);p.ram.protect(0x46c,4).unwrap();
    super::super::services::timer_interrupt_at(&mut p,42).unwrap();
    let r=run(p,1).unwrap();let p=r.loaded;let(cpu,ram)=(p.cpu.clone(),p.ram.clone());
    let r=run(p,5).unwrap();assert!(matches!(&r.stop,Stop::Service(m) if m.contains("BDA")),"{:?}",r.stop);
    assert_eq!((r.loaded.cpu,r.loaded.ram),(cpu,ram));
}
#[test]
fn irq0_delivery_faults_change_nothing(){
    // A handler outside the program, an odd SP below 6 (shutdown), and protected mode.
    for case in 0..3{
        let mut p=irq0_program(OWN_EOI);
        match case{0=>p.ram.store(0x08*4,MemWidth::Dword,0xf000_0000).unwrap(),1=>p.cpu.regs[4]=5,_=>p.image.real_mode=false}
        let(cpu,ram,image)=(p.cpu.clone(),p.ram.clone(),p.image.clone());
        assert!(super::super::services::timer_interrupt_at(&mut p,42).is_err());
        assert_eq!((p.cpu,p.ram,p.image),(cpu,ram,image));
        p.speaker.pic.write(0x20,0x0b).unwrap();assert_eq!(p.speaker.pic.read(0x20)&1,0,"not acknowledged");
    }
}
#[test]
fn hlt_with_a_program_int08_handler_and_the_host_clock_ends_through_the_handler(){
    // STI; HLT; CLI; HLT.
    let code=code_at(&[(0,&[0xfb,0xf4,0xfa,0xf4]),(0x30,OWN_EOI)]);
    let mut p=load(&code,"idle.com").unwrap();p.clock_offset=Some(0);p.cpu.eflags=0x002;
    p.ram.store(0x08*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    let ss=u32::from(PSP)*16;p.ram.store(ss+0x210,MemWidth::Byte,0).unwrap();
    // Take the host edge first, so only HLT can raise the next one.
    let now=super::super::services::sampled_ticks(&mut p);p.speaker.irq_edges=Some(now);p.timer_ticks=7;
    let r=run(p,8).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert!(r.loaded.ram.load(ss+0x210,MemWidth::Byte).unwrap()>=1);assert_eq!(r.loaded.cpu.eip,0x103);
    assert_eq!(r.loaded.timer_ticks,7,"the handler, not Dine, keeps the count");
}
#[test]
fn timer_irq_respects_pic_mask_and_hook_remains_in_service_until_eoi(){
    let code=code_at(&[(0,&[0xfa,0xf4]),(0x30,&[0xcf])]);let mut p=load(&code,"pic.com").unwrap();
    p.cpu.eflags=2;p.speaker.pic.write(0x20,0x0a).unwrap();p.speaker.pic.write(0x21,1).unwrap();
    super::super::services::timer_interrupt_at(&mut p,42).unwrap();assert_eq!(p.speaker.pic.read(0x20)&1,1);assert_eq!(p.timer_ticks,0);
    p.cpu.eflags=0x202;super::super::services::timer_interrupt_at(&mut p,42).unwrap();assert_eq!(p.timer_ticks,0);
    p.speaker.pic.write(0x21,0).unwrap();p.ram.store(0x1c*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    super::super::services::timer_interrupt_at(&mut p,42).unwrap();assert_eq!(p.timer_ticks,42);
    p.speaker.pic.write(0x20,0x0b).unwrap();assert_eq!(p.speaker.pic.read(0x20)&1,1);
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert_eq!(r.loaded.speaker.pic.read(0x20)&1,0);
}
#[test]
fn timer_hook_runs_at_instruction_boundary_and_iret_restores_frame(){
    // Interrupted CLI; HLT. Hook: INC BYTE [0200h]; IRET.
    let code=code_at(&[(0,&[0xfa,0xf4]),(0x30,&[0xfe,0x06,0x00,0x02,0xcf])]);
    let mut p=load(&code,"tick.com").unwrap();p.cpu.eflags=0x2d7;
    p.ram.store(0x1c*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    p.ram.store(u32::from(PSP)*16+0x200,MemWidth::Byte,0).unwrap();
    let (ip,sp,flags)=(p.cpu.eip,p.cpu.regs[4]&0xffff,p.cpu.eflags);
    super::super::services::timer_interrupt_at(&mut p,42).unwrap();
    assert_eq!(p.cpu.eip,u32::from(TIMER_OFFSET));assert_eq!(p.cpu.eflags&0x300,0);
    assert_eq!(p.ram.load(0x46c,MemWidth::Dword).unwrap(),42);
    let ss=u32::from(PSP)*16;
    assert_eq!(p.ram.load(ss+sp-6,MemWidth::Word).unwrap(),ip);
    assert_eq!(p.ram.load(ss+sp-2,MemWidth::Word).unwrap(),flags&0xffff);
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.ram.load(ss+0x200,MemWidth::Byte).unwrap(),1);
    assert_eq!(r.loaded.cpu.regs[4]&0xffff,sp);
    assert_eq!(r.loaded.cpu.eflags&0xd5,flags&0xd5);
}
#[test]
fn host_timer_run_loop_delivers_a_pending_hook_before_the_next_instruction(){
    let code=code_at(&[(0,&[0xfa,0xf4]),(0x30,&[0xfe,0x06,0x00,0x02,0xcf])]);
    let mut p=load(&code,"tick.com").unwrap();p.cpu.eflags=0x202;p.clock_offset=Some(0);p.timer_ticks=u32::MAX;
    p.ram.store(0x1c*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    let ss=u32::from(PSP)*16;p.ram.store(ss+0x200,MemWidth::Byte,0).unwrap();
    let sp=p.cpu.regs[4];let r=run(p,40).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.ram.load(ss+0x200,MemWidth::Byte).unwrap(),1);
    assert_eq!(r.loaded.cpu.regs[4],sp);
}
#[test]
fn timer_bios_wrapper_restores_eax_and_ds_after_the_hook(){
    // BIOS saves EAX and DS around the user hook. This hook changes both.
    let code=code_at(&[(0,&[0xfa,0xf4]),(0x30,&[0x66,0xb8,0x34,0x12,0x78,0x56,0x8e,0xd8,0xcf])]);
    let mut p=load(&code,"clobber.com").unwrap();p.cpu.eflags=0x202;p.cpu.regs[0]=0xdeadbeef;
    p.ram.store(0x1c*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    let original_ds=p.selectors[SegmentRegister::Ds.index()];
    super::super::services::timer_interrupt_at(&mut p,42).unwrap();
    let r=run(p,30).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.cpu.regs[0],0xdeadbeef);assert_eq!(r.loaded.selectors[SegmentRegister::Ds.index()],original_ds);
}
#[test]
fn timer_hook_can_chain_to_the_default_int1c_entry(){
    let old=entry(0x1c);let [lo,hi,sl,sh]=old.to_le_bytes();
    // PUSHF; CALL FAR old; INC BYTE [0200h]; IRET
    let code=code_at(&[(0,&[0xfa,0xf4]),(0x30,&[0x9c,0x9a,lo,hi,sl,sh,0xfe,0x06,0x00,0x02,0xcf])]);
    let mut p=load(&code,"chain.com").unwrap();p.cpu.eflags=0x202;
    p.ram.store(0x1c*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    let ss=u32::from(PSP)*16;p.ram.store(ss+0x200,MemWidth::Byte,0).unwrap();
    super::super::services::timer_interrupt_at(&mut p,42).unwrap();
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.ram.load(ss+0x200,MemWidth::Byte).unwrap(),1);
}
#[test]
fn bios_clock_read_does_not_enter_timer_hook_mid_service(){
    let code=code_at(&[(0,&[0xcd,0x1a,0xfa,0xf4]),(0x30,&[0xcf])]);
    let mut p=load(&code,"clock.com").unwrap();p.cpu.eflags=0x202;p.clock_offset=Some(0);p.timer_ticks=12;
    p.ram.store(0x1c*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    p.cpu.regs[0]=0;let (ip,cs)=(p.cpu.eip,p.selectors[SegmentRegister::Cs.index()]);
    super::super::service(&mut p,0x1a,&mut vec![]).unwrap();
    assert_eq!((p.cpu.eip,p.selectors[SegmentRegister::Cs.index()]),(ip,cs));
    assert_eq!(p.cpu.regs[2]&0xffff,12);
}
#[test]
fn timer_hook_outside_real_mode_is_an_explicit_stop(){
    let code=code_at(&[(0,&[0xfa,0xf4]),(0x30,&[0xcf])]);
    let mut p=load(&code,"tick.com").unwrap();p.cpu.eflags=0x202;p.image.real_mode=false;
    p.ram.store(0x1c*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    let before=p.clone();assert!(super::super::services::timer_interrupt_at(&mut p,42).is_err());
    assert_eq!(p.cpu,before.cpu);assert_eq!(p.ram,before.ram);assert_eq!(p.timer_ticks,before.timer_ticks);
}
#[test]
fn timer_hook_invalid_target_is_transactional_and_masked_ticks_wait(){
    let mut p=load(&[0xfa,0xf4],"tick.com").unwrap();p.cpu.eflags=2;
    p.ram.store(0x1c*4,MemWidth::Dword,0xf000_0000).unwrap();
    let original=p.clone();super::super::services::timer_interrupt_at(&mut p,42).unwrap();assert_eq!(p.cpu,original.cpu);assert_eq!(p.ram,original.ram);
    p.cpu.eflags=0x202;let original=p.clone();
    assert!(super::super::services::timer_interrupt_at(&mut p,42).is_err());
    assert_eq!(p.cpu,original.cpu);assert_eq!(p.ram,original.ram);assert_eq!(p.timer_ticks,original.timer_ticks);
}
#[test]
fn pit_channel0_latches_reads_and_reprograms_binary_modes(){
    // Latch; IN twice into BL, BH; mode 34h; count 0000h; CLI; HLT.
    let code=[0xb0,0x00,0xe6,0x43,0xe4,0x40,0x88,0xc3,0xe4,0x40,0x88,0xc7,0xb0,0x34,0xe6,0x43,0xb0,0x00,0xe6,0x40,0xe6,0x40,0xfa,0xf4];
    let mut p=load(&code,"pit.com").unwrap();p.cpu.regs[3]=0xabcd;
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    // Time moves a whole tick at a time without the host clock: count 0.
    assert_eq!(r.loaded.cpu.regs[3]&0xffff,0);
    let mut p=load(&code,"pit.com").unwrap();p.clock_offset=Some(0);
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    // This 8253-compatible profile holds the previous count during programming.
    let r=run(load(&[0xb0,0x34,0xe6,0x43,0xb0,0x00,0xe6,0x40,0xe4,0x40,0xfa,0xf4],"half.com").unwrap(),20).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
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
    // gives AL=FFh; valid dates change the session clock only.
    for(cx,dx)in [(0x4445,0x5351),(2001,0x021d),(1979,0x0101),(2100,0x0101),(1990,0x0d01),(1990,0x0100)]{
        assert_eq!(dos(&mut p,0x2b00,0,cx,dx).unwrap(),(0x2bff,0,dx,0),"{cx} {dx:04x}");
    }
    for(cx,dx,weekday)in [(2000,0x021d,2),(1980,0x0101,2),(2099,0x0c1f,4)]{
        assert_eq!(dos(&mut p,0x2b00,0,cx,dx).unwrap(),(0x2b00,0,dx,0));
        p.cpu.regs[0]=0x2a00;super::super::service(&mut p,0x21,&mut vec![]).unwrap();
        assert_eq!((p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff,p.cpu.regs[0]&255),(cx,dx,weekday));
        let bcd=|n:u32|((n/10)<<4|n%10)as u8;assert_eq!(p.rtc.read(None,9),Some(bcd(cx%100)));assert_eq!(p.rtc.read(None,0x32),Some(bcd(cx/100)));
    }
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
    // AAM 0 is a divide error: nothing changes (vector 0 cleared, so the
    // fault stops the run instead of reaching the DOS default handler).
    let mut p=load(&[0xd4,0x00,0xf4],"z.com").unwrap();p.cpu.regs[0]=0x55;p.ram.store(0,MemWidth::Dword,0).unwrap();let cpu=p.cpu.clone();
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

// Real-mode exception delivery (interrupt.rs). Handlers are installed by
// writing the vector directly: XOR AX,AX; MOV ES,AX; MOV word ES:[4n],0130h;
// MOV ES:[4n+2],CS. The handler is at 0130h (code offset 30h).
fn hook(n:u8)->Vec<u8>{
    let at=u16::from(n)*4;let [a0,a1]=at.to_le_bytes();let [b0,b1]=(at+2).to_le_bytes();
    vec![0x31,0xc0,0x8e,0xc0,0x26,0xc7,0x06,a0,a1,0x30,0x01,0x26,0x8c,0x0e,b0,b1]
}
fn with_hook(n:u8,test:&[u8],handler:&[u8])->Vec<u8>{
    let mut main=hook(n);main.extend_from_slice(test);main.extend_from_slice(&[0xfa,0xf4]);
    code_at(&[(0,&main),(0x30,handler)])
}
#[test]
fn divide_error_enters_the_handler_with_the_div_address_and_restarts_it(){
    // XOR CX,CX; MOV AX,7; XOR DX,DX; DIV CX (at 0117h). The handler sets
    // CX=1, BX=1234h, SI=the pushed IP, DI=the pushed FLAGS, and IRETs, so
    // DIV runs again and divides by one.
    let code=with_hook(0,&[0x31,0xc9,0xb8,0x07,0x00,0x31,0xd2,0xf7,0xf1],
        &[0x41,0xbb,0x34,0x12,0x89,0xe5,0x8b,0x76,0x00,0x8b,0x7e,0x04,0xcf]);
    let mut p=load(&code,"de.com").unwrap();p.cpu.eflags=0x202;let sp=p.cpu.regs[4];
    let r=run(p,100).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let c=&r.loaded.cpu;
    assert_eq!((c.regs[0]&0xffff,c.regs[2]&0xffff,c.regs[1]&0xffff,c.regs[3]&0xffff),(7,0,1,0x1234));
    assert_eq!(c.regs[6]&0xffff,0x117,"return address is the DIV itself");
    assert_eq!(c.regs[7]&0x200,0x200,"pushed FLAGS keep the caller's IF");
    assert_eq!(c.regs[4],sp);
    assert_eq!(r.instructions.get("#DE (vector 00h)"),Some(&1));
    assert_eq!(r.exception_count,1);assert_eq!(r.exceptions[0].eip,0x117);
}
fn screen_row(p:&Loaded,row:u32)->String{
    (0..80).map(|c|char::from(p.ram.load(0xb8000+(row*80+c)*2,MemWidth::Byte).unwrap()as u8)).collect::<String>().trim_end().to_string()
}
#[test]
fn without_a_program_handler_a_divide_error_reaches_the_dos_default_handler(){
    // MOV BX,5678h; PUSH BX; XOR CX,CX; DIV CX at 0105h. Vector 0 holds its
    // entry, so the FreeDOS default handler prints the message and 13 stack
    // words (IP, CS, FLAGS, then 5678h and the PSP's zero word) and ends the
    // program with errorlevel 127.
    let mut p=load(&[0xbb,0x78,0x56,0x53,0x31,0xc9,0xf7,0xf1,0xfa,0xf4],"de.com").unwrap();p.cpu.eflags=0x202;
    assert_eq!(p.ram.load(0,MemWidth::Dword).unwrap(),entry(0));
    let r=run(p,20).unwrap();
    assert!(matches!(r.stop,Stop::Exit(0x7f)),"{:?}",r.stop);
    assert_eq!(r.instructions.get("#DE (vector 00h)"),Some(&1));
    assert_eq!((r.exception_count,r.exceptions[0].eip,r.exceptions[0].vector),(1,0x106,0));
    assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x4c7f);
    let p=&r.loaded;
    assert_eq!(screen_row(p,1),"Interrupt divide by zero, stack:");
    let words=screen_row(p,2);let words:Vec<&str>=words.split(' ').collect();
    assert_eq!(words.len(),13,"{words:?}");
    assert_eq!(&words[..5],&["0106",&format!("{PSP:04X}"),"0246","5678","0000"]);
    assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),3<<8,"cursor after the final CR LF");
}
#[test]
fn the_default_divide_handler_needs_the_bios_int10_and_a_null_vector_still_stops(){
    // A program that hooked INT 10h: the kernel's print would reach its handler.
    let code=with_hook(0x10,&[0x31,0xc9,0xf7,0xf1],&[0xcf]);
    let r=run(load(&code,"de10.com").unwrap(),40).unwrap();
    assert!(matches!(&r.stop,Stop::Service(m) if m.contains("INT 10h")),"{:?}",r.stop);
    // A vector a program cleared to 0000:0000 is not the kernel's handler.
    let code=[0x31,0xc0,0x8e,0xc0,0x26,0x66,0xc7,0x06,0,0,0,0,0,0,0x31,0xc9,0xf7,0xf1,0xfa,0xf4];
    let r=run(load(&code,"de0.com").unwrap(),40).unwrap();
    assert!(matches!(&r.stop,Stop::Fault(m) if m.contains("vector: 0")),"{:?}",r.stop);
}
#[test]
fn idiv_overflow_and_aam_zero_are_divide_errors_too(){
    // IDIV CL with AX=8000h (quotient -32768 does not fit AL); AAM 0.
    for (test,name) in [(vec![0xb8,0x00,0x80,0xb1,0x01,0xf6,0xf9],"idiv"),(vec![0xd4,0x00],"aam")]{
        let code=with_hook(0,&test,&[0xbb,0x34,0x12,0x89,0xe5,0x8b,0x76,0x00,0x83,0x46,0x00,0x02,0xcf]);
        let r=run(load(&code,"de.com").unwrap(),100).unwrap();
        assert!(matches!(r.stop,Stop::Monitor(_)),"{name}: {:?}",r.stop);
        assert_eq!(r.loaded.cpu.regs[3]&0xffff,0x1234,"{name}");
        assert_eq!(r.instructions.get("#DE (vector 00h)"),Some(&1),"{name}");
    }
}
#[test]
fn segment_limit_faults_are_gp_and_ss_in_real_mode(){
    // MOV AX,[FFFFh] is #GP(0) (vector 0Dh); MOV BP,FFFFh; MOV AX,[BP+0] is
    // #SS(0) (vector 0Ch). The handler skips the 3-byte instruction.
    let skip=[0xbb,0x34,0x12,0x89,0xe5,0x83,0x46,0x00,0x03,0xcf];
    for (n,test,key) in [(0x0du8,vec![0xa1,0xff,0xff],"#GP (vector 0dh)"),(0x0c,vec![0xbd,0xff,0xff,0x8b,0x46,0x00],"#SS (vector 0ch)")]{
        let code=with_hook(n,&test,&skip);
        let r=run(load(&code,"gp.com").unwrap(),100).unwrap();
        assert!(matches!(r.stop,Stop::Monitor(_)),"{key}: {:?}",r.stop);
        assert_eq!(r.loaded.cpu.regs[3]&0xffff,0x1234,"{key}");
        assert_eq!(r.instructions.get(key),Some(&1),"{key}");
    }
}
#[test]
fn int3_and_into_return_to_the_next_instruction(){
    // The handler counts calls in BX and keeps the pushed IP in SI.
    let count=[0x43,0x89,0xe5,0x8b,0x76,0x00,0xcf];
    // INT3 at 0110h.
    let r=run(load(&with_hook(3,&[0x31,0xdb,0xcc],&count),"bp.com").unwrap(),100).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!((r.loaded.cpu.regs[3]&0xffff,r.loaded.cpu.regs[6]&0xffff),(1,0x113));
    // INTO with OF=0 (after XOR) does nothing; MOV AL,7Fh; ADD AL,1 sets OF; INTO again.
    let r=run(load(&with_hook(4,&[0x31,0xdb,0xce,0xb0,0x7f,0x04,0x01,0xce],&count),"of.com").unwrap(),100).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!((r.loaded.cpu.regs[3]&0xffff,r.loaded.cpu.regs[6]&0xffff),(1,0x118));
    assert_eq!(r.instructions.get("INTO"),Some(&2));
}
#[test]
fn int3_without_a_program_handler_stops_as_a_service(){
    let r=run(load(&[0xcc,0xf4],"bp.com").unwrap(),10).unwrap();
    assert!(matches!(&r.stop,Stop::Service(m) if m.contains("INT3")),"{:?}",r.stop);
    assert_eq!(r.loaded.cpu.eip,0x100);
}
#[test]
fn a_rom_takes_its_divide_error_through_the_vector_it_set_up(){
    use crate::dos::run_rom;
    let mut rom=vec![0u8;65536];
    rom[0xfff0..0xfff5].copy_from_slice(&[0xea,0x00,0xe0,0x00,0xf0]);
    // XOR AX,AX; MOV DS,AX; MOV [0],E100h; MOV [2],F000h; MOV AX,1000h;
    // MOV SS,AX; MOV SP,100h; XOR CX,CX; MOV AX,9; XOR DX,DX; DIV CX; CLI; HLT.
    let main=[0x31,0xc0,0x8e,0xd8,0xc7,0x06,0,0,0x00,0xe1,0xc7,0x06,2,0,0x00,0xf0,0xb8,0x00,0x10,0x8e,0xd0,0xbc,0x00,0x01,
        0x31,0xc9,0xb8,0x09,0x00,0x31,0xd2,0xf7,0xf1,0xfa,0xf4];
    rom[0xe000..0xe000+main.len()].copy_from_slice(&main);
    rom[0xe100..0xe102].copy_from_slice(&[0x41,0xcf]); // INC CX; IRET
    let path=std::env::temp_dir().join(format!("pil-rom-de-{}.bin",std::process::id()));
    std::fs::write(&path,&rom).unwrap();
    let r=run_rom(&path,100);std::fs::remove_file(&path).unwrap();let r=r.unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!((r.loaded.cpu.regs[0]&0xffff,r.loaded.cpu.regs[1]&0xffff,r.loaded.cpu.regs[4]&0xffff),(9,1,0x100));
    assert_eq!(r.loaded.ram.load(0x10000+0xfa,MemWidth::Word).unwrap(),0xe01f,"pushed IP = the DIV at E01Fh");
}
#[test]
fn reinitialized_pic_delivers_irq0_through_its_new_vector(){
    // CWSDPMI-style ICW1 11h, ICW2 50h, ICW3 04h, ICW4 01h moves IRQ0 to INT 50h.
    let code=code_at(&[(0,&[0xfa,0xf4]),(0x30,&[0xcf]),(0x40,&[0xcf])]);let mut p=load(&code,"remap.com").unwrap();
    for(port,value)in [(0x20,0x11),(0x21,0x50),(0x21,0x04),(0x21,0x01)]{p.speaker.pic.write(port,value).unwrap();}
    assert_eq!((p.speaker.pic.timer_vector(),p.speaker.pic.keyboard_vector()),(0x50,0x51));
    // A program INT 08h handler is no longer IRQ0's.
    p.ram.store(8*4,MemWidth::Dword,u32::from(PSP)<<16|0x140).unwrap();
    p.cpu.eflags=0x202;let before=(p.cpu.eip,p.cpu.regs[4],p.ram.load(0x46c,MemWidth::Dword).unwrap());
    // INT 50h still 0: no handler behind IRQ0, so the run stops unchanged.
    let e=super::super::services::timer_interrupt_at(&mut p,42).unwrap_err();assert!(e.contains("INT 50h"),"{e}");
    assert_eq!((p.cpu.eip,p.cpu.regs[4],p.ram.load(0x46c,MemWidth::Dword).unwrap()),before);
    // With a handler at INT 50h, IRQ0 enters it; the BIOS tick is not counted.
    p.ram.store(0x50*4,MemWidth::Dword,u32::from(PSP)<<16|0x130).unwrap();
    super::super::services::timer_interrupt_at(&mut p,43).unwrap();
    assert_eq!(p.cpu.eip,0x130);assert_eq!(p.ram.load(0x46c,MemWidth::Dword).unwrap(),before.2);
    p.speaker.pic.write(0x20,0x0b).unwrap();assert_eq!(p.speaker.pic.read(0x20),1);
    // IRQ1 follows the base as well.
    assert!(!super::super::keyboard::hooked(&p).unwrap());
    p.ram.store(0x51*4,MemWidth::Dword,u32::from(PSP)<<16|0x140).unwrap();assert!(super::super::keyboard::hooked(&p).unwrap());
}
