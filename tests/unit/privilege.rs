//! Original expectations for protected-mode increment 5: CALL through 386
//! and 286 call gates to an inner level (stack from the TSS, parameters
//! copied), RETF and IRET to an outer level (SS:ESP popped, data segments
//! nulled), interrupts from ring 3 to ring 0, the privilege faults of far
//! transfers, and the IOPL and I/O bitmap checks. Expected stack contents
//! are written out by hand from the 386 manual's frame layouts.
use super::*;
use crate::guest::{self,Image};
use crate::interrupt::{deliver,Event};
use address::{SegmentAccess,SegmentCache,SystemSegment};

const GDT:u32=0x1000;
const IDT:u32=0x2000;
const TSS:u32=0x3000;
fn gate(selector:u16,offset:u32,access:u8,count:u8)->u64{
    u64::from(offset&0xffff)|(u64::from(selector)<<16)|(u64::from(count)<<32)|(u64::from(access)<<40)|(u64::from(offset>>16)<<48)
}
/// Null; ring 0 flat code (08h) and data (10h); ring 3 flat code (18h)
/// and data (20h); the 386 TSS (28h, busy, limit 6Fh); a DPL 3 386 call
/// gate to 0008:4000 copying 2 dwords (30h); a DPL 3 286 call gate to
/// 0008:4100 copying 1 word (38h); conforming ring 0 code (40h); a DPL 0
/// call gate (48h); a DPL 3 gate to the conforming code at 4200h (50h);
/// DPL 2 data (58h).
fn descriptors()->[u64;12]{[0,0x00cf_9a00_0000_ffff,0x00cf_9200_0000_ffff,0x00cf_fa00_0000_ffff,0x00cf_f200_0000_ffff,0x0000_8b00_3000_006f,
    gate(0x08,0x4000,0xec,2),gate(0x08,0x4100,0xe4,1),0x00cf_9e00_0000_ffff,gate(0x08,0x4000,0x8c,0),gate(0x40,0x4200,0xec,0),0x00cf_d200_0000_ffff]}
fn code(big:bool,dpl:u8)->SegmentCache{SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Code{readable:true,conforming:false,big,dpl}}}
fn data(dpl:u8)->SegmentCache{SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Data{writable:true,expand_down:false,big:true,dpl}}}

/// 32-bit protected mode at CPL 3: CS 1Bh, SS/DS/ES/FS/GS 23h, ESP=7000h,
/// EIP=600h with `bytes` there, TR = the 386 TSS (ESP0 9000h, SS0 10h, I/O
/// bitmap at 68h with only port 11h's bit set), IDT vector 40h a DPL 3 386
/// interrupt gate to 0008:5000.
fn user(bytes:&[u8])->(Image,Ram,Cpu){
    let (mut i,_)=Image::load(&[0x90]).unwrap();
    let mut r=Ram::new(0x20000);
    for (n,d) in descriptors().iter().enumerate(){r.write_bytes(GDT+8*n as u32,&d.to_le_bytes()).unwrap();}
    r.write_bytes(IDT+0x40*8,&gate(0x08,0x5000,0xee,0).to_le_bytes()).unwrap();
    r.store(TSS+4,MemWidth::Dword,0x9000).unwrap();r.store(TSS+8,MemWidth::Word,0x10).unwrap();
    r.store(TSS+0x66,MemWidth::Word,0x68).unwrap();r.store(TSS+0x68+2,MemWidth::Byte,0x02).unwrap();
    r.write_bytes(0x600,bytes).unwrap();
    i.real_mode=false;i.msw=0x11;i.start=0;i.end=0x20000;
    i.tables=[(8*12-1,GDT),(0x40*8+7,IDT)];
    i.ldt_tr[1]=SystemSegment{selector:0x28,base:TSS,limit:0x6f,kind:0x0b};
    i.selectors=[0x23,0x1b,0x23,0x23,0x23,0x23];
    i.segments=[data(3);6];i.segments[1]=code(true,3);
    (i,r,Cpu{regs:[0,0,0,0,0x7000,0,0,0],eflags:0x202,undefined_flags:0,eip:0x600})
}
fn step(i:&mut Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
    let d=i.decode(r,c.eip).map_err(|e|match e{guest::GuestError::Execution(e)=>e,e=>panic!("{e:?}")})?;
    if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);if result.is_ok(){i.set_segment_state(m);}result}else{panic!("PIL expected at {:x}",c.eip)}
}
fn dword(r:&Ram,at:u32)->u32{r.load(at,MemWidth::Dword).unwrap()}
fn word(r:&Ram,at:u32)->u32{r.load(at,MemWidth::Word).unwrap()}
const NULL:SegmentCache=SegmentCache{base:0,limit:0,access:SegmentAccess::Unusable};

#[test]
fn a_386_call_gate_switches_to_the_ring_0_stack_and_retf_returns_to_ring_3(){
    // CALL FAR 0033:00000000 (the RPL is 3; the gate ignores the offset).
    let (mut i,mut r,mut c)=user(&[0x9a,0,0,0,0,0x33,0]);
    r.store(0x7000,MemWidth::Dword,0x1111_1111).unwrap();r.store(0x7004,MemWidth::Dword,0x2222_2222).unwrap();
    step(&mut i,&mut r,&mut c).unwrap();
    assert_eq!((i.selectors[1],i.segments[1],c.eip),(0x08,code(true,0),0x4000));
    assert_eq!((i.selectors[2],i.segments[2],c.regs[4]),(0x10,data(0),0x8fe8));
    // Old SS, old ESP, the two parameters in their order, CS, EIP.
    assert_eq!([0x8ffc,0x8ff8,0x8ff4,0x8ff0,0x8fec,0x8fe8].map(|a|dword(&r,a)),[0x23,0x7000,0x2222_2222,0x1111_1111,0x1b,0x607]);
    assert_eq!((r.load(GDT+0x08+5,MemWidth::Byte).unwrap(),r.load(GDT+0x10+5,MemWidth::Byte).unwrap()),(0x9b,0x93),"accessed bits");
    // The ring 0 code loads DS with its own data segment, then RETF 8.
    r.write_bytes(0x4000,&[0xca,8,0]).unwrap();
    i.selectors[3]=0x10;i.segments[3]=data(0);
    step(&mut i,&mut r,&mut c).unwrap();
    assert_eq!((i.selectors[1],i.segments[1],c.eip),(0x1b,code(true,3),0x607));
    assert_eq!((i.selectors[2],i.segments[2],c.regs[4]),(0x23,data(3),0x7008),"8 bytes of parameters dropped from both stacks");
    assert_eq!((i.selectors[3],i.segments[3]),(0,NULL),"DS (DPL 0) is nulled");
    assert_eq!((i.selectors[0],i.segments[0]),(0x23,data(3)),"ES (DPL 3) stays");
}

#[test]
fn a_286_call_gate_pushes_words_and_copies_one_word(){
    let (mut i,mut r,mut c)=user(&[0x9a,0,0,0,0,0x3b,0]);
    r.store(0x7000,MemWidth::Word,0xbeef).unwrap();
    step(&mut i,&mut r,&mut c).unwrap();
    assert_eq!((i.selectors[1],c.eip,c.regs[4]),(0x08,0x4100,0x8ff6));
    assert_eq!([0x8ffe,0x8ffc,0x8ffa,0x8ff8,0x8ff6].map(|a|word(&r,a)),[0x23,0x7000,0xbeef,0x1b,0x607]);
}

#[test]
fn a_gate_to_conforming_code_stays_at_cpl_3_on_the_same_stack(){
    let (mut i,mut r,mut c)=user(&[0x9a,0,0,0,0,0x53,0]);
    step(&mut i,&mut r,&mut c).unwrap();
    assert_eq!((i.selectors[1],c.eip,i.selectors[2],c.regs[4]),(0x43,0x4200,0x23,0x6ff8));
    assert_eq!(i.segments[1].access,SegmentAccess::Code{readable:true,conforming:true,big:true,dpl:3});
    assert_eq!([dword(&r,0x6ffc),dword(&r,0x6ff8)],[0x1b,0x607]);
}

#[test]
fn privilege_faults_leave_everything_as_it_was(){
    let gp=|code:u16|Err(ExecError::GeneralProtection{eip:0x600,error_code:code});
    let cases:[(&[u8],Result<(),ExecError>);7]=[
        (&[0x9a,0,0,0,0,0x4b,0],gp(0x48)),            // CALL through a DPL 0 gate
        (&[0xea,0,0,0,0,0x0b,0],gp(0x08)),            // JMP straight to ring 0 code
        (&[0xea,0,0,0,0,0x33,0],gp(0x08)),            // JMP through a gate to ring 0 code
        (&[0x9a,0,0,0,0,0x08,0],gp(0x08)),            // CALL with RPL 0 to ring 0 code
        (&[0xcb],gp(0x08)),                           // RETF to ring 0 (RPL below CPL)
        (&[0xf4],gp(0)),                              // HLT
        (&[0xfa],gp(0)),                              // CLI with IOPL 0
    ];
    for (bytes,want) in cases{
        let (mut i,mut r,mut c)=user(bytes);
        r.store(0x7000,MemWidth::Dword,0x700).unwrap();r.store(0x7004,MemWidth::Dword,0x08).unwrap();
        let old=(i.clone(),r.clone(),c.clone());
        assert_eq!(step(&mut i,&mut r,&mut c),want,"{bytes:02x?}");
        assert_eq!((i,r,c),old,"{bytes:02x?}");
    }
    // A TSS stack selector with the wrong RPL and DPL is #TS(selector).
    let (mut i,mut r,mut c)=user(&[0x9a,0,0,0,0,0x33,0]);r.store(TSS+8,MemWidth::Word,0x58).unwrap();
    assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::InvalidTss{eip:0x600,error_code:0x58}));
    // A TSS too short for the ring 0 entry is #TS(TSS selector).
    let (mut i,mut r,mut c)=user(&[0x9a,0,0,0,0,0x33,0]);i.ldt_tr[1].limit=0x0a;
    assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::InvalidTss{eip:0x600,error_code:0x28}));
}

#[test]
fn int_from_ring_3_runs_the_ring_0_handler_on_its_stack_and_iretd_returns(){
    let (mut i,mut r,mut c)=user(&[0x90]);
    deliver(&mut c,&mut i,&mut r,Event::software(0x40,0x602)).unwrap();
    assert_eq!([0x8ffc,0x8ff8,0x8ff4,0x8ff0,0x8fec].map(|a|dword(&r,a)),[0x23,0x7000,0x202,0x1b,0x602]);
    assert_eq!((i.selectors[1],i.selectors[2],c.regs[4],c.eip,c.eflags),(0x08,0x10,0x8fec,0x5000,0x002));
    // The handler loads DS, then IRETD.
    r.write_bytes(0x5000,&[0xcf]).unwrap();i.selectors[3]=0x10;i.segments[3]=data(0);
    step(&mut i,&mut r,&mut c).unwrap();
    assert_eq!((i.selectors[1],i.selectors[2],c.regs[4],c.eip,c.eflags),(0x1b,0x23,0x7000,0x602,0x202));
    assert_eq!((i.selectors[3],i.segments[3]),(0,NULL));
    // A fault at ring 3 goes the same way, with its error code.
    let (mut i,mut r,mut c)=user(&[0x90]);
    r.write_bytes(IDT+0x0d*8,&gate(0x08,0x5d00,0x8e,0).to_le_bytes()).unwrap();
    deliver(&mut c,&mut i,&mut r,Event::fault(&ExecError::GeneralProtection{eip:0x600,error_code:0},0x600).unwrap()).unwrap();
    assert_eq!([0x8ffc,0x8ff8,0x8ff4,0x8ff0,0x8fec,0x8fe8].map(|a|dword(&r,a)),[0x23,0x7000,0x202,0x1b,0x600,0]);
    assert_eq!((c.eip,c.regs[4]),(0x5d00,0x8fe8));
}

#[test]
fn iopl_and_the_io_bitmap_decide_port_access_above_iopl(){
    // IN AL,10h: port 10h's bit is clear, so it reads the open bus.
    let (mut i,mut r,mut c)=user(&[0xe4,0x10]);step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.regs[0]&0xff,0xff);
    let gp=Err(ExecError::GeneralProtection{eip:0x600,error_code:0});
    // Port 11h's bit is set; a word at 10h covers it; port 40h is past the TSS limit.
    for bytes in [&[0xe4,0x11][..],&[0x66,0xe5,0x10],&[0xe4,0x40],&[0xe6,0x11]]{
        let (mut i,mut r,mut c)=user(bytes);let old=c.clone();assert_eq!(step(&mut i,&mut r,&mut c),gp,"{bytes:02x?}");assert_eq!(c,old);
    }
    // IOPL 3 allows every port, CLI and STI.
    for bytes in [&[0xe4,0x11][..],&[0xe4,0x40],&[0xfa],&[0xfb]]{
        let (mut i,mut r,mut c)=user(bytes);c.eflags|=0x3000;step(&mut i,&mut r,&mut c).unwrap();
    }
    // A 286 TSS has no bitmap.
    let (mut i,mut r,mut c)=user(&[0xe4,0x10]);i.ldt_tr[1].kind=0x03;assert_eq!(step(&mut i,&mut r,&mut c),gp);
}

#[test]
fn popf_at_ring_3_changes_neither_iopl_nor_if(){
    // POPFD with IOPL 3, IF 0 and CF 1 on the stack.
    let (mut i,mut r,mut c)=user(&[0x9d]);r.store(0x7000,MemWidth::Dword,0x3001).unwrap();
    step(&mut i,&mut r,&mut c).unwrap();
    assert_eq!((c.eflags,c.regs[4]),(0x203,0x7004));
    // With IOPL 3, IF follows the popped value; IOPL still needs CPL 0.
    let (mut i,mut r,mut c)=user(&[0x9d]);c.eflags=0x3202;r.store(0x7000,MemWidth::Dword,0x0001).unwrap();
    step(&mut i,&mut r,&mut c).unwrap();
    assert_eq!(c.eflags,0x3003);
    // PUSHFD works at ring 3.
    let (mut i,mut r,mut c)=user(&[0x9c]);step(&mut i,&mut r,&mut c).unwrap();assert_eq!((dword(&r,0x6ffc),c.regs[4]),(0x202,0x6ffc));
}
