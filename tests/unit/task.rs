//! Original expectations for protected-mode increment 7: task switches by
//! far JMP and CALL (to a TSS and through a task gate), by exceptions and
//! INT n through IDT task gates, and IRET with NT set. Expected TSS bytes,
//! busy bits, back links and registers are written out by hand from the
//! 386 manual's TSS layouts, not computed with the code under test.
use super::*;
use crate::guest::Image;
use crate::interrupt::{deliver,Event};
use address::{SegmentAccess,SegmentCache,SystemSegment};

const GDT:u32=0x1000;
const IDT:u32=0x2000;
const TSS_A:u32=0x3000;
const TSS_B:u32=0x3100;
const TSS_C:u32=0x3200;
const LDT:u32=0x3400;
/// 00h null, 08h flat 32-bit code, 10h flat 32-bit data, 18h busy 386 TSS
/// A (the current task), 20h available 386 TSS B, 28h available 286 TSS C
/// (DPL 3), 30h task gate to 20h, 38h LDT, 40h DPL 3 16-bit code at
/// 10000h, 48h DPL 3 16-bit data at 10000h, 50h a TSS that is not present,
/// 58h a 386 TSS with limit 60h.
const DESCRIPTORS:[u64;12]=[0,0x00cf_9a00_0000_ffff,0x00cf_9200_0000_ffff,0x0000_8b00_3000_0067,0x0000_8900_3100_0067,0x0000_e100_3200_002b,0x0000_8500_0020_0000,0x0000_8200_3400_000f,0x0000_fa01_0000_ffff,0x0000_f201_0000_ffff,0x0000_0900_3300_0067,0x0000_8900_3300_0060];

fn put64(ram:&mut Ram,at:u32,v:u64){ram.store(at,MemWidth::Dword,v as u32).unwrap();ram.store(at+4,MemWidth::Dword,(v>>32) as u32).unwrap();}
fn dword(ram:&Ram,at:u32)->u32{ram.load(at,MemWidth::Dword).unwrap()}
fn word(ram:&Ram,at:u32)->u32{ram.load(at,MemWidth::Word).unwrap()}
fn byte(ram:&Ram,at:u32)->u32{ram.load(at,MemWidth::Byte).unwrap()}

/// 32-bit protected mode at CPL 0 in task A (TR=18h), flat segments, ESP
/// 8000h, IDT with a task gate to TSS B at vectors 0Dh (DPL 0) and 40h
/// (DPL 3). TSS B: EIP 700h, the given EFLAGS, EAX-EDI B0000000h+n with
/// ESP 9000h, ES 0Ch and GS 04h from the LDT at 38h, CS 08h, SS and DS
/// 10h, FS null, CR3 field 5000h. The LDT has two flat data entries.
/// `code` is placed at EIP 500h; IRETD sits at 700h.
fn machine(code:&[u8],b_flags:u32)->(Cpu,Image,Ram){
    let (mut image,_)=Image::load(&[0x90]).unwrap();
    let mut ram=Ram::new(0x20000);
    for (i,d) in DESCRIPTORS.iter().enumerate(){put64(&mut ram,GDT+8*i as u32,*d);}
    put64(&mut ram,IDT+8*0x0d,0x0000_8500_0020_0000);
    put64(&mut ram,IDT+8*0x40,0x0000_e500_0020_0000);
    put64(&mut ram,LDT,0x00cf_9200_0000_ffff);put64(&mut ram,LDT+8,0x00cf_9200_0000_ffff);
    ram.store(TSS_B+0x1c,MemWidth::Dword,0x5000).unwrap();
    ram.store(TSS_B+0x20,MemWidth::Dword,0x700).unwrap();
    ram.store(TSS_B+0x24,MemWidth::Dword,b_flags).unwrap();
    for i in 0..8u32{ram.store(TSS_B+0x28+4*i,MemWidth::Dword,if i==4{0x9000}else{0xb000_0000+i}).unwrap();}
    for (i,s) in [0x0cu32,0x08,0x10,0x10,0,0x04].iter().enumerate(){ram.store(TSS_B+0x48+4*i as u32,MemWidth::Word,*s).unwrap();}
    ram.store(TSS_B+0x60,MemWidth::Word,0x38).unwrap();
    for (i,b) in code.iter().enumerate(){ram.store(0x500+i as u32,MemWidth::Byte,u32::from(*b)).unwrap();}
    ram.store(0x700,MemWidth::Byte,0xcf).unwrap();
    image.real_mode=false;image.msw=0x11;image.end=0x20000;image.start=0;
    image.tables=[(8*DESCRIPTORS.len() as u16-1,GDT),(0x40*8+7,IDT)];
    image.ldt_tr[1]=SystemSegment{selector:0x18,base:TSS_A,limit:0x67,kind:0x0b};
    image.selectors=[0x10,0x08,0x10,0x10,0x10,0x10];
    let flat=SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Data{writable:true,expand_down:false,big:true,dpl:0}};
    image.segments=[flat;6];
    image.segments[1]=SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0}};
    let cpu=Cpu{regs:[1,2,3,4,0x8000,6,7,8],eflags:0x0000_0246,undefined_flags:0,eip:0x500};
    (cpu,image,ram)
}
fn step(i:&mut Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
    let d=i.decode(r,c.eip).unwrap();
    if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);if result.is_ok(){i.set_segment_state(m);}result}else{panic!("PIL expected at {:x}",c.eip)}
}
/// Task A's state as a switch away from it stores it in its 386 TSS.
fn assert_a_saved(ram:&Ram,eip:u32){
    assert_eq!((dword(ram,TSS_A+0x20),dword(ram,TSS_A+0x24)),(eip,0x246));
    assert_eq!((0..8).map(|i|dword(ram,TSS_A+0x28+4*i)).collect::<Vec<_>>(),vec![1,2,3,4,0x8000,6,7,8]);
    assert_eq!((0..6).map(|i|word(ram,TSS_A+0x48+4*i)).collect::<Vec<_>>(),vec![0x10,0x08,0x10,0x10,0x10,0x10]);
}
fn assert_in_b(cpu:&Cpu,image:&Image,esp:u32,eflags:u32){
    assert_eq!(cpu.regs,[0xb000_0000,0xb000_0001,0xb000_0002,0xb000_0003,esp,0xb000_0005,0xb000_0006,0xb000_0007]);
    assert_eq!((cpu.eip,cpu.eflags),(0x700,eflags));
    assert_eq!(image.selectors,[0x0c,0x08,0x10,0x10,0,0x04]);
    assert_eq!(image.segments[4].access,SegmentAccess::Unusable,"a null FS");
    assert_eq!(image.ldt_tr,[SystemSegment{selector:0x38,base:LDT,limit:0x0f,kind:2},SystemSegment{selector:0x20,base:TSS_B,limit:0x67,kind:0x0b}]);
    assert_eq!(image.msw&8,8,"a task switch sets CR0.TS");
    assert_eq!(image.controls[1],0,"CR3 is loaded only while paging is on");
}

#[test]
fn jmp_to_a_tss_saves_the_old_task_frees_it_and_keeps_nt_as_the_tss_has_it(){
    // JMP FAR 0020:00000000.
    let (mut cpu,mut image,mut ram)=machine(&[0xea,0,0,0,0,0x20,0],0x0000_4002);
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_in_b(&cpu,&image,0x9000,0x4002);
    assert_a_saved(&ram,0x507);
    assert_eq!((byte(&ram,GDT+0x18+5),byte(&ram,GDT+0x20+5)),(0x89,0x8b),"A available, B busy");
    assert_eq!(word(&ram,TSS_B),0,"JMP writes no back link");
    assert_eq!((byte(&ram,LDT+5),byte(&ram,LDT+8+5)),(0x93,0x93),"the LDT data segments are marked accessed");
}

#[test]
fn call_through_a_task_gate_links_back_and_iret_returns_to_the_caller(){
    // CALL FAR 0030:00000000 through the task gate; IRETD in task B.
    let (mut cpu,mut image,mut ram)=machine(&[0x9a,0,0,0,0,0x30,0],0x0000_0002);
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_in_b(&cpu,&image,0x9000,0x4002);
    assert_a_saved(&ram,0x507);
    assert_eq!(word(&ram,TSS_B),0x18,"the back link names task A");
    assert_eq!((byte(&ram,GDT+0x18+5),byte(&ram,GDT+0x20+5)),(0x8b,0x8b),"both busy");
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_eq!((cpu.eip,cpu.eflags,cpu.regs),(0x507,0x246,[1,2,3,4,0x8000,6,7,8]));
    assert_eq!(image.selectors,[0x10,0x08,0x10,0x10,0x10,0x10]);
    assert_eq!(image.ldt_tr[1].selector,0x18);
    assert_eq!(image.ldt_tr[0].selector,0,"task A's LDT field is null");
    assert_eq!((byte(&ram,GDT+0x18+5),byte(&ram,GDT+0x20+5)),(0x8b,0x89),"B is free again, A stays busy");
    assert_eq!((dword(&ram,TSS_B+0x20),dword(&ram,TSS_B+0x24)),(0x701,0x0002),"B is stored with NT clear");
}

#[test]
fn a_faulting_switch_changes_nothing(){
    for (selector,expected) in [
        (0x18u8,ExecError::GeneralProtection{eip:0x500,error_code:0x18}),
        (0x23,ExecError::GeneralProtection{eip:0x500,error_code:0x20}),
        (0x50,ExecError::SegmentNotPresent{eip:0x500,error_code:0x50}),
        (0x58,ExecError::InvalidTss{eip:0x500,error_code:0x58}),
    ]{
        let (mut cpu,mut image,mut ram)=machine(&[0xea,0,0,0,0,selector,0],0x0000_0002);
        let before=(cpu.regs,cpu.eflags,image.selectors,image.ldt_tr,image.msw);
        assert_eq!(step(&mut image,&mut ram,&mut cpu),Err(expected),"selector {selector:02x}");
        assert_eq!((cpu.regs,cpu.eflags,image.selectors,image.ldt_tr,image.msw),before);
        assert_eq!((cpu.eip,dword(&ram,TSS_A+0x20),byte(&ram,GDT+0x18+5),byte(&ram,GDT+0x20+5)),(0x500,0,0x8b,0x89));
    }
    // A data segment as the new task's CS is #TS(selector), before any store.
    let (mut cpu,mut image,mut ram)=machine(&[0xea,0,0,0,0,0x20,0],0x0000_0002);
    ram.store(TSS_B+0x4c,MemWidth::Word,0x10).unwrap();
    assert_eq!(step(&mut image,&mut ram,&mut cpu),Err(ExecError::InvalidTss{eip:0x500,error_code:0x10}));
    assert_eq!((image.ldt_tr[1].selector,dword(&ram,TSS_A+0x20),byte(&ram,GDT+0x18+5)),(0x18,0,0x8b));
}

#[test]
fn an_exception_through_a_task_gate_pushes_its_error_code_on_the_new_stack(){
    let (mut cpu,mut image,mut ram)=machine(&[0x90],0x0000_0002);
    deliver(&mut cpu,&mut image,&mut ram,Event::fault(&ExecError::GeneralProtection{eip:0x500,error_code:0x1234},0x500).unwrap()).unwrap();
    assert_in_b(&cpu,&image,0x8ffc,0x4002);
    assert_eq!(dword(&ram,0x8ffc),0x1234);
    assert_a_saved(&ram,0x500);
    assert_eq!(word(&ram,TSS_B),0x18);
    // INT 40h through the DPL 3 task gate: no error code, return after INT.
    let (mut cpu,mut image,mut ram)=machine(&[0x90],0x0000_0002);
    deliver(&mut cpu,&mut image,&mut ram,Event::software(0x40,0x502)).unwrap();
    assert_in_b(&cpu,&image,0x9000,0x4002);
    assert_a_saved(&ram,0x502);
}

#[test]
fn a_286_tss_loads_word_registers_with_ffff_above_and_null_fs_and_gs(){
    // CALL FAR 0028:00000000 to the DPL 3 286 TSS: IP 10h, FLAGS 0202h,
    // AX-DI 1111h*(n+1) with SP 0800h, ES/SS/DS 4Bh, CS 43h, no LDT.
    let (mut cpu,mut image,mut ram)=machine(&[0x9a,0,0,0,0,0x28,0],0x0000_0002);
    ram.store(TSS_C+0x0e,MemWidth::Word,0x10).unwrap();
    ram.store(TSS_C+0x10,MemWidth::Word,0x0202).unwrap();
    for i in 0..8u32{ram.store(TSS_C+0x12+2*i,MemWidth::Word,if i==4{0x0800}else{0x1111*(i+1)}).unwrap();}
    for (i,s) in [0x4bu32,0x43,0x4b,0x4b].iter().enumerate(){ram.store(TSS_C+0x22+2*i as u32,MemWidth::Word,*s).unwrap();}
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_eq!(cpu.regs,[0xffff_1111,0xffff_2222,0xffff_3333,0xffff_4444,0xffff_0800,0xffff_6666,0xffff_7777,0xffff_8888]);
    assert_eq!((cpu.eip,cpu.eflags),(0x10,0x4202));
    assert_eq!(image.selectors,[0x4b,0x43,0x4b,0x4b,0,0]);
    assert_eq!((image.segments[1].base,image.segment_state().cpl()),(0x10000,3));
    assert_eq!(image.ldt_tr[1],SystemSegment{selector:0x28,base:TSS_C,limit:0x2b,kind:3});
    assert_eq!((word(&ram,TSS_C),byte(&ram,GDT+0x28+5)),(0x18,0xe3),"back link and busy bit");
    assert_a_saved(&ram,0x507);
}

#[test]
fn a_386_tss_with_vm_set_enters_virtual_8086_mode(){
    let (mut cpu,mut image,mut ram)=machine(&[0xea,0,0,0,0,0x20,0],0x0002_3202);
    for (i,s) in [0x1000u32,0xe000,0x2000,0x3000,0x4000,0x5000].iter().enumerate(){ram.store(TSS_B+0x48+4*i as u32,MemWidth::Word,*s).unwrap();}
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_eq!((cpu.eip,cpu.eflags),(0x700,0x0002_3202));
    assert_eq!(image.selectors,[0x1000,0xe000,0x2000,0x3000,0x4000,0x5000]);
    assert_eq!(image.segments[1],SegmentCache::v86(0xe000));
    assert!(image.segment_state().v86());
}

#[test]
fn clts_keeps_paging_and_needs_cpl_0(){
    // CLTS (0F 06) clears TS only; PG stays set.
    let (mut cpu,mut image,mut ram)=machine(&[0x0f,0x06],0x0000_0002);
    image.msw=0x19;image.paging=true;
    // Identity-map the first 4 MiB with a 4 KiB page table at 6000h.
    ram.store(0x5000,MemWidth::Dword,0x6007).unwrap();
    for page in 0..0x20u32{ram.store(0x6000+4*page,MemWidth::Dword,(page<<12)|7).unwrap();}
    image.controls[1]=0x5000;
    ram.set_paging(image.segment_state().ram_paging());
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_eq!((image.msw,image.paging),(0x11,true));
    // At CPL 3 it is #GP(0).
    let (mut cpu,mut image,mut ram)=machine(&[0x0f,0x06],0x0000_0002);
    image.msw=0x19;image.segments[1].access=SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:3};
    assert_eq!(step(&mut image,&mut ram,&mut cpu),Err(ExecError::GeneralProtection{eip:0x500,error_code:0}));
}

#[test]
fn a_switch_before_any_ltr_stores_the_old_task_at_linear_zero(){
    // TR as after reset: selector 0, base 0, limit FFFFh, a busy 386 TSS.
    let (mut cpu,mut image,mut ram)=machine(&[0xea,0,0,0,0,0x20,0],0x0000_0002);
    image.ldt_tr[1]=address::TR_RESET;
    put64(&mut ram,GDT,0x0000_0200_0000_0000);
    assert_eq!(Image::load(&[0x90]).unwrap().0.ldt_tr[1],SystemSegment{selector:0,base:0,limit:0xffff,kind:0x0b});
    step(&mut image,&mut ram,&mut cpu).unwrap();
    assert_in_b(&cpu,&image,0x9000,0x0002);
    assert_eq!((dword(&ram,0x20),dword(&ram,0x24)),(0x507,0x246),"EIP and EFLAGS at linear 20h and 24h");
    assert_eq!((0..8).map(|i|dword(&ram,0x28+4*i)).collect::<Vec<_>>(),vec![1,2,3,4,0x8000,6,7,8]);
    assert_eq!((0..6).map(|i|word(&ram,0x48+4*i)).collect::<Vec<_>>(),vec![0x10,0x08,0x10,0x10,0x10,0x10]);
    assert_eq!(byte(&ram,GDT+0x20+5),0x8b,"the new TSS is busy");
    assert_eq!(byte(&ram,GDT+5),0,"JMP clears the busy bit in GDT entry 0, which TR's selector names");
}
