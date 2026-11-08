//! Original expectations for protected-mode increment 4: delivery through
//! IDT interrupt and trap gates (286 and 386), error codes and the EXT bit,
//! CR2, the double fault and the triple fault, and IRET at the same level.
//! Expected stack bytes are written out by hand from the 386 manual's frame
//! layout, not computed with the code under test.
use super::*;
use crate::guest::Image;
use crate::interrupt::{deliver,Event};
use address::{SegmentAccess,SegmentCache};

const GDT:u32=0x1000;
const IDT:u32=0x2000;
/// Null, flat 32-bit code (08h), flat 32-bit data (10h), 16-bit code with
/// base 10000h (18h), DPL 3 flat 32-bit code (20h), not-present code (28h),
/// conforming DPL 0 code (30h).
const DESCRIPTORS:[u64;7]=[0,0x00cf_9a00_0000_ffff,0x00cf_9200_0000_ffff,0x0000_9a01_0000_ffff,0x00cf_fa00_0000_ffff,0x00cf_1a00_0000_ffff,0x00cf_9e00_0000_ffff];

/// A gate: selector, offset, access byte (P, DPL, type).
fn gate(selector:u16,offset:u32,access:u8)->u64{
    u64::from(offset&0xffff)|(u64::from(selector)<<16)|(u64::from(access)<<40)|(u64::from(offset>>16)<<48)
}
const INT386:u8=0x8e;const TRAP386:u8=0x8f;const INT286:u8=0x86;

/// 32-bit protected mode at CPL 0, flat SS with ESP=8000h, IDT covering
/// vectors 0-21h. Vectors used: 00h (386 interrupt gate), 03h (386 trap gate, DPL 3), 05h (286 gate to
/// 18h), 06h (not present), 07h (task gate), 08h (#DF), 0Bh (#NP), 0Dh (#GP),
/// 0Eh (#PF), 20h (DPL 0), 21h (gate to a data segment), 1Fh (to the
/// not-present code segment), 1Eh (conforming code).
fn machine()->(Cpu,Image,Ram){
    let (mut image,_)=Image::load(&[0x90]).unwrap();
    let mut ram=Ram::new(0x20000);
    for (i,d) in DESCRIPTORS.iter().enumerate(){ram.store(GDT+8*i as u32,MemWidth::Dword,*d as u32).unwrap();ram.store(GDT+8*i as u32+4,MemWidth::Dword,(*d>>32) as u32).unwrap();}
    let mut put=|v:u32,g:u64|{ram.store(IDT+8*v,MemWidth::Dword,g as u32).unwrap();ram.store(IDT+8*v+4,MemWidth::Dword,(g>>32) as u32).unwrap();};
    put(0x00,gate(0x08,0x3000,INT386));
    put(0x03,gate(0x08,0x0001_3100,TRAP386|0x60));
    put(0x05,gate(0x18,0x0000_1234,INT286));
    put(0x06,gate(0x08,0x3600,INT386&0x7f));
    put(0x07,gate(0x40,0,0x85));
    put(0x08,gate(0x08,0x3800,INT386));
    put(0x0b,gate(0x08,0x3b00,INT386));
    put(0x0d,gate(0x08,0x3d00,INT386));
    put(0x0e,gate(0x08,0x3e00,INT386));
    put(0x1e,gate(0x30,0x3e80,INT386));
    put(0x1f,gate(0x28,0x3f00,INT386));
    put(0x20,gate(0x08,0x4000,INT386));
    put(0x21,gate(0x10,0x4100,INT386));
    image.real_mode=false;image.msw=0x11;image.end=0x20000;image.start=0;
    image.tables=[(8*DESCRIPTORS.len() as u16-1,GDT),(0x21*8+7,IDT)];
    image.selectors=[0x10,0x08,0x10,0x10,0x10,0x10];
    let flat=SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Data{writable:true,expand_down:false,big:true,dpl:0}};
    image.segments=[flat;6];
    image.segments[1]=SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0}};
    let cpu=Cpu{regs:[0,0,0,0,0x8000,0,0,0],eflags:0x0000_0346,undefined_flags:0,eip:0x0500};
    (cpu,image,ram)
}
fn dword(ram:&Ram,at:u32)->u32{ram.load(at,MemWidth::Dword).unwrap()}
fn word(ram:&Ram,at:u32)->u32{ram.load(at,MemWidth::Word).unwrap()}
fn fault(e:ExecError,eip:u32)->Event{Event::fault(&e,eip).unwrap()}

#[test]
fn a_fault_with_an_error_code_goes_through_a_386_interrupt_gate(){
    let (mut cpu,mut image,mut ram)=machine();
    deliver(&mut cpu,&mut image,&mut ram,fault(ExecError::GeneralProtection{eip:0x500,error_code:0x1234},0x500)).unwrap();
    // EFLAGS, CS, EIP, then the error code, as dwords below ESP=8000h.
    assert_eq!([dword(&ram,0x7ffc),dword(&ram,0x7ff8),dword(&ram,0x7ff4),dword(&ram,0x7ff0)],[0x346,0x08,0x500,0x1234]);
    assert_eq!((cpu.regs[4],cpu.eip,image.selectors[1]),(0x7ff0,0x3d00,0x08));
    assert_eq!(cpu.eflags,0x046,"an interrupt gate clears IF and TF");
}

#[test]
fn int3_through_a_dpl3_trap_gate_keeps_if_and_returns_after_the_instruction(){
    let (mut cpu,mut image,mut ram)=machine();
    deliver(&mut cpu,&mut image,&mut ram,Event::software(3,0x501)).unwrap();
    assert_eq!([dword(&ram,0x7ffc),dword(&ram,0x7ff8),dword(&ram,0x7ff4)],[0x346,0x08,0x501]);
    assert_eq!((cpu.regs[4],cpu.eip,cpu.eflags),(0x7ff4,0x0001_3100,0x246),"no error code; TF clear, IF kept");
}

#[test]
fn a_286_gate_pushes_words_and_takes_a_16_bit_offset(){
    let (mut cpu,mut image,mut ram)=machine();cpu.eflags|=0x0001_0000;
    deliver(&mut cpu,&mut image,&mut ram,Event::software(5,0x0001_2345)).unwrap();
    assert_eq!([word(&ram,0x7ffe),word(&ram,0x7ffc),word(&ram,0x7ffa)],[0x0346,0x08,0x2345]);
    assert_eq!((cpu.regs[4],cpu.eip,image.selectors[1],image.segments[1].base),(0x7ffa,0x1234,0x18,0x10000));
    assert_eq!(cpu.eflags,0x046,"RF clears too");
}

#[test]
fn a_16_bit_stack_wraps_sp_and_keeps_the_upper_half_of_esp(){
    let (mut cpu,mut image,mut ram)=machine();
    image.segments[2]=SegmentCache{base:0x10000,limit:0xffff,access:SegmentAccess::Data{writable:true,expand_down:false,big:false,dpl:0}};
    cpu.regs[4]=0xabcd_0004;
    deliver(&mut cpu,&mut image,&mut ram,fault(ExecError::Exception{eip:0x500,vector:0},0x500)).unwrap();
    assert_eq!([dword(&ram,0x10000),dword(&ram,0x1fffc),dword(&ram,0x1fff8)],[0x346,0x08,0x500]);
    assert_eq!(cpu.regs[4],0xabcd_fff8);
}

#[test]
fn the_conforming_target_keeps_cpl_and_the_gate_type_checks_come_first(){
    let (mut cpu,mut image,mut ram)=machine();
    deliver(&mut cpu,&mut image,&mut ram,Event::software(0x1e,0x502)).unwrap();
    assert_eq!((image.selectors[1],cpu.eip),(0x30,0x3e80));
    let (mut cpu,image,ram)=machine();let m=image.segment_state();
    let mut read=|at:u32|Ok(u64::from(ram.load(at,MemWidth::Dword).unwrap())|(u64::from(ram.load(at+4,MemWidth::Dword).unwrap())<<32));
    // Beyond the limit, and a data-segment target, are #GP naming the entry
    // or the selector; EXT is set unless INT n is being delivered.
    assert_eq!(m.interrupt_gate(0x22,true,0x500,&mut read),Err(ExecError::GeneralProtection{eip:0x500,error_code:0x112}));
    assert_eq!(m.interrupt_gate(0x22,false,0x500,&mut read),Err(ExecError::GeneralProtection{eip:0x500,error_code:0x113}));
    assert_eq!(m.interrupt_gate(0x06,true,0x500,&mut read),Err(ExecError::SegmentNotPresent{eip:0x500,error_code:0x32}));
    assert_eq!(m.interrupt_gate(0x07,true,0x500,&mut read),Err(ExecError::InvalidContext),"task gates are not modelled yet");
    assert_eq!(m.interrupt_gate(0x10,true,0x500,&mut read),Err(ExecError::GeneralProtection{eip:0x500,error_code:0x82}),"an empty entry is not a gate");
    assert_eq!(m.interrupt_target(0x10,false,0x500,&mut read),Err(ExecError::GeneralProtection{eip:0x500,error_code:0x11}));
    assert_eq!(m.interrupt_target(0x28,true,0x500,&mut read),Err(ExecError::SegmentNotPresent{eip:0x500,error_code:0x28}));
    assert_eq!(m.interrupt_target(0x00,false,0x500,&mut read),Err(ExecError::GeneralProtection{eip:0x500,error_code:1}));
    // At CPL 3, INT n through a DPL 0 gate is #GP; the DPL 3 gate passes.
    let mut user=image.clone();user.segments[1].access=SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:3};
    let u=user.segment_state();
    assert_eq!(u.interrupt_gate(0x20,true,0x500,&mut read),Err(ExecError::GeneralProtection{eip:0x500,error_code:0x102}));
    assert!(u.interrupt_gate(0x03,true,0x500,&mut read).is_ok());
    assert!(u.interrupt_gate(0x20,false,0x500,&mut read).is_ok(),"exceptions ignore the gate DPL");
    // A ring 0 handler from CPL 3 switches stacks through the TSS
    // (tests/unit/privilege.rs); with no TSS loaded, #TS is not deliverable here.
    let (mut uimage,mut uram)=(user,ram.clone());
    assert!(deliver(&mut cpu,&mut uimage,&mut uram,Event::software(3,0x501)).is_err());
}

#[test]
fn faults_while_delivering_become_a_double_fault_and_then_a_shutdown(){
    // #NP at the vector 06h gate (benign #UD first): delivered as #NP with
    // EXT set, through vector 0Bh.
    let (mut cpu,mut image,mut ram)=machine();
    deliver(&mut cpu,&mut image,&mut ram,Event{vector:6,error_code:None,return_eip:0x500,software:false,cr2:None}).unwrap();
    assert_eq!((cpu.eip,dword(&ram,0x7ff0),dword(&ram,0x7ff4)),(0x3b00,0x33,0x500));
    // #GP whose handler segment is not present: #NP during a contributory
    // fault is a double fault, with error code 0.
    let (mut cpu,mut image,mut ram)=machine();
    ram.store(IDT+0x0d*8+2,MemWidth::Word,0x28).unwrap();
    deliver(&mut cpu,&mut image,&mut ram,fault(ExecError::GeneralProtection{eip:0x500,error_code:0},0x500)).unwrap();
    assert_eq!((cpu.eip,dword(&ram,0x7ff0),dword(&ram,0x7ff4)),(0x3800,0,0x500));
    // And with #DF's own handler absent too, the processor shuts down.
    let (mut cpu,mut image,mut ram)=machine();
    ram.store(IDT+0x0d*8+2,MemWidth::Word,0x28).unwrap();ram.store(IDT+0x08*8+2,MemWidth::Word,0x28).unwrap();
    let (before,regs)=(ram.clone(),cpu.regs);
    let message=deliver(&mut cpu,&mut image,&mut ram,fault(ExecError::GeneralProtection{eip:0x500,error_code:0},0x500)).unwrap_err();
    assert!(message.starts_with("shutdown"),"{message}");
    assert_eq!((ram,cpu.regs,cpu.eip),(before,regs,0x500),"nothing was pushed");
}

#[test]
fn a_page_fault_sets_cr2_and_a_fault_on_its_stack_is_a_double_fault(){
    // Identity-map the first 128 KiB, but leave the page under the stack
    // (7000h) absent.
    let setup=||{
        let (cpu,mut image,mut ram)=machine();
        ram.store(0x10000,MemWidth::Dword,0x11007).unwrap();
        for page in 0..32u32{if page!=7{ram.store(0x11000+4*page,MemWidth::Dword,(page<<12)|7).unwrap();}}
        image.paging=true;image.controls[1]=0x10000;
        (cpu,image,ram)
    };
    let (mut cpu,mut image,mut ram)=setup();cpu.regs[4]=0x9000;
    deliver(&mut cpu,&mut image,&mut ram,fault(ExecError::PageFault{eip:0x500,address:0xdead_b000,error_code:6},0x500)).unwrap();
    assert_eq!((image.controls[0],cpu.eip),(0xdead_b000,0x3e00),"CR2 holds the linear address");
    ram.set_paging(None);
    assert_eq!([dword(&ram,0x8ffc),dword(&ram,0x8ff0)],[0x346,6]);
    // ESP=8000h: the frame falls in the absent page, so the #PF raised
    // while delivering #PF is a double fault, whose frame faults again.
    let (mut cpu,mut image,mut ram)=setup();
    let message=deliver(&mut cpu,&mut image,&mut ram,fault(ExecError::PageFault{eip:0x500,address:0x1234,error_code:0},0x500)).unwrap_err();
    assert!(message.starts_with("shutdown"),"{message}");
    assert_eq!(image.controls[0],0x7ffc,"the last #PF recognised wrote CR2");
}

fn step(i:&mut Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
    let d=i.decode(r,c.eip).unwrap();
    if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);if result.is_ok(){i.set_segment_state(m);}result}else{panic!("PIL expected at {:x}",c.eip)}
}
/// IRETD (CF) at 3D00h, with a frame at ESP=7FF4h: EIP 600h, CS 08h and
/// the given EFLAGS.
fn iret(flags:u32,cpl3:bool)->(Cpu,Image,Ram){
    let (mut cpu,mut image,mut ram)=machine();
    ram.store(0x3d00,MemWidth::Byte,0xcf).unwrap();
    for (at,v) in [(0x7ff4,0x600),(0x7ff8,if cpl3{0x23}else{0x08}),(0x7ffc,flags)]{ram.store(at,MemWidth::Dword,v).unwrap();}
    if cpl3{
        image.selectors[1]=0x23;image.segments[1]=SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:3}};
    }
    cpu.eip=0x3d00;cpu.regs[4]=0x7ff4;cpu.eflags=0x0000_0002;
    (cpu,image,ram)
}

#[test]
fn iretd_at_the_same_level_restores_eip_cs_and_eflags(){
    let (mut cpu,mut image,mut ram)=iret(0x0000_3ed7,false);
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_eq!((cpu.eip,image.selectors[1],cpu.regs[4]),(0x600,0x08,0x8000));
    assert_eq!(cpu.eflags,0x0000_3ed7,"CPL 0 takes IOPL and IF from the frame");
    // At CPL 3 with IOPL 0, IOPL and IF keep their values.
    let (mut cpu,mut image,mut ram)=iret(0x0000_32d7,true);
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_eq!((cpu.eip,image.selectors[1]),(0x600,0x23));
    assert_eq!(cpu.eflags,0x0000_00d7);
    // At CPL 3 with IOPL 3, IF comes from the frame but IOPL does not change.
    let (mut cpu,mut image,mut ram)=iret(0x0000_0200,true);cpu.eflags=0x3002;
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_eq!(cpu.eflags,0x0000_3202);
}

#[test]
fn iret_task_returns_stop_and_vm_in_the_frame_counts_only_at_cpl0(){
    let (mut cpu,mut image,mut ram)=iret(0x0000_0002,false);cpu.eflags|=0x4000;
    assert_eq!(step(&mut image,&mut ram,&mut cpu),Err(ExecError::InvalidContext),"NT=1 is a task return");
    // VM=1 at CPL 0 is the return to virtual-8086 mode (tests/unit/v86.rs).
    // At CPL 3 the VM bit in the frame is ignored.
    let (mut cpu,mut image,mut ram)=iret(0x0002_0002,true);
    step(&mut image,&mut ram,&mut cpu).unwrap();assert_eq!(cpu.eflags&0x20000,0);
}
