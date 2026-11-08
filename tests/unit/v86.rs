//! Original expectations for protected-mode increment 6: virtual-8086
//! mode. IRETD at CPL 0 with VM in the frame enters it; segment loads,
//! PUSHF/POPF, CLI/STI, IRET and INT n follow the 386's V86 rules (IOPL 3
//! needed, IOPL itself fixed); HLT and I/O outside the bitmap are #GP(0);
//! interrupts and exceptions leave V86 mode for a nonconforming ring 0
//! handler with GS, FS, DS and ES pushed and nulled, and IRETD returns.
//! Frames are written out by hand from the 386 manual's V86 frame layout.
use super::*;
use crate::guest::{self,Image,Operation};
use crate::interrupt::{deliver,Event};
use address::{SegmentAccess,SegmentCache,SystemSegment};

const GDT:u32=0x1000;
const IDT:u32=0x2000;
const TSS:u32=0x3000;
fn gate(selector:u16,offset:u32,access:u8)->u64{
    u64::from(offset&0xffff)|(u64::from(selector)<<16)|(u64::from(access)<<40)|(u64::from(offset>>16)<<48)
}
/// Null; ring 0 flat code (08h) and data (10h); ring 3 flat code (18h);
/// the 386 TSS (20h, busy, limit 6Fh); conforming ring 0 code (28h).
const DESCRIPTORS:[u64;6]=[0,0x00cf_9a00_0000_ffff,0x00cf_9200_0000_ffff,0x00cf_fa00_0000_ffff,0x0000_8b00_3000_006f,0x00cf_9e00_0000_ffff];
fn ring0()->SegmentCache{SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0}}}
fn data0()->SegmentCache{SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Data{writable:true,expand_down:false,big:true,dpl:0}}}
fn v86(selector:u16)->SegmentCache{SegmentCache{base:u32::from(selector)*16,limit:0xffff,access:SegmentAccess::Data{writable:true,expand_down:false,big:false,dpl:3}}}
const NULL:SegmentCache=SegmentCache{base:0,limit:0,access:SegmentAccess::Unusable};

/// Protected mode with paging off, TR = the 386 TSS (ESP0 9000h, SS0 10h,
/// I/O bitmap at 68h with only port 11h's bit set) and these IDT gates:
/// 0Dh (#GP) to 0008:5000, 40h (DPL 3) to 0008:5100, 41h (DPL 3) to the
/// ring 3 code 0018:5200, 42h (DPL 3) to the conforming 0028:5300, 43h
/// (DPL 0) to 0008:5400. All 386 interrupt gates.
fn machine()->(Image,Ram){
    let (mut i,_)=Image::load(&[0x90]).unwrap();
    let mut r=Ram::new(0x20000);
    for (n,d) in DESCRIPTORS.iter().enumerate(){r.write_bytes(GDT+8*n as u32,&d.to_le_bytes()).unwrap();}
    for (v,g) in [(0x0d,gate(0x08,0x5000,0x8e)),(0x40,gate(0x08,0x5100,0xee)),(0x41,gate(0x18,0x5200,0xee)),(0x42,gate(0x28,0x5300,0xee)),(0x43,gate(0x08,0x5400,0x8e))]{
        r.write_bytes(IDT+v*8,&g.to_le_bytes()).unwrap();
    }
    r.store(TSS+4,MemWidth::Dword,0x9000).unwrap();r.store(TSS+8,MemWidth::Word,0x10).unwrap();
    r.store(TSS+0x66,MemWidth::Word,0x68).unwrap();r.store(TSS+0x68+2,MemWidth::Byte,0x02).unwrap();
    i.real_mode=false;i.msw=0x11;i.start=0;i.end=0x20000;
    i.tables=[(8*6-1,GDT),(0x44*8-1,IDT)];
    i.ldt_tr[1]=SystemSegment{selector:0x20,base:TSS,limit:0x6f,kind:0x0b};
    (i,r)
}
/// Already in V86 mode: CS 0060h (code at linear 600h), SS 0070h with
/// SP=0100h, DS 0080h, ES 0081h, FS 0082h, GS 0083h; IF=1, IOPL `iopl`.
fn in_v86(bytes:&[u8],iopl:u32)->(Image,Ram,Cpu){
    let (mut i,mut r)=machine();
    r.write_bytes(0x600,bytes).unwrap();
    i.selectors=[0x81,0x60,0x70,0x80,0x82,0x83];
    i.segments=i.selectors.map(v86);
    (i,r,Cpu{regs:[0,0,0,0,0x100,0,0,0],eflags:0x20202|(iopl<<12),undefined_flags:0,eip:0})
}
/// One instruction: PIL, or INT n through the IDT; a fault is delivered
/// as the runners do. Returns the fault, if there was one.
fn step(i:&mut Image,r:&mut Ram,c:&mut Cpu)->Option<ExecError>{
    let d=match i.decode(r,c.eip){Ok(d)=>d,Err(guest::GuestError::Execution(e))=>{fault(i,r,c,&e);return Some(e);},Err(e)=>panic!("{e:?}")};
    match d.operation{
        Operation::Pil{program,dst,src,flow,address}=>{
            let mut m=i.segment_state();
            match execute_machine(&program,c,dst,src,flow,r,address,&mut m){Ok(_)=>{i.set_segment_state(m);None},Err(e)=>{fault(i,r,c,&e);Some(e)}}
        }
        Operation::Interrupt{vector,next,..}=>{deliver(c,i,r,Event::software(vector,next&0xffff)).unwrap();None}
        _=>panic!("unexpected operation at {:x}",c.eip),
    }
}
fn fault(i:&mut Image,r:&mut Ram,c:&mut Cpu,e:&ExecError){deliver(c,i,r,Event::fault(e,c.eip).expect("an exception")).unwrap();}
fn dword(r:&Ram,at:u32)->u32{r.load(at,MemWidth::Dword).unwrap()}
fn word(r:&Ram,at:u32)->u32{r.load(at,MemWidth::Word).unwrap()}
/// The ring 0 side after an interrupt or exception from V86 mode: CS:EIP,
/// SS:ESP, the four data segments null and VM, TF, NT and IF (interrupt
/// gate) clear.
fn assert_left_v86(i:&Image,c:&Cpu,eip:u32,esp:u32){
    assert_eq!((i.selectors[1],i.segments[1],c.eip),(0x08,ring0(),eip));
    assert_eq!((i.selectors[2],i.segments[2],c.regs[4]),(0x10,data0(),esp));
    for reg in [0,3,4,5]{assert_eq!((i.selectors[reg],i.segments[reg]),(0,NULL),"segment register {reg}");}
    assert_eq!(c.eflags&0x24300,0);
    assert!(!i.segment_state().v86());assert_eq!(i.segment_state().cpl(),0);
}

#[test]
fn iretd_at_cpl0_with_vm_enters_v86_mode_with_every_segment_from_the_frame(){
    let (mut i,mut r)=machine();
    r.write_bytes(0x600,&[0xcf]).unwrap();
    i.selectors=[0x10,0x08,0x10,0x10,0x10,0x10];i.segments=[data0();6];i.segments[1]=ring0();
    // EIP, CS, EFLAGS (VM, IOPL 3, IF, CF), ESP, SS, ES, DS, FS, GS.
    for (n,v) in [0x0010,0x0060,0x0002_3203,0x0100,0x0070,0x0081,0x0080,0x0082,0x0083].iter().enumerate(){r.store(0x8000+4*n as u32,MemWidth::Dword,*v).unwrap();}
    let mut c=Cpu{regs:[0,0,0,0,0x8000,0,0,0],eflags:0x2,undefined_flags:0,eip:0x600};
    assert_eq!(step(&mut i,&mut r,&mut c),None);
    assert_eq!((c.eip,c.regs[4],c.eflags),(0x10,0x100,0x0002_3203));
    assert_eq!(i.selectors,[0x81,0x60,0x70,0x80,0x82,0x83]);
    assert_eq!(i.segments,[0x81,0x60,0x70,0x80,0x82,0x83].map(v86));
    assert!(i.segment_state().v86());assert_eq!(i.segment_state().cpl(),3);
    // At CPL 3 a VM bit in the frame is ignored (tests/unit/pm_interrupts.rs),
    // and a 16-bit IRET cannot carry it.
    let (mut i,mut r)=machine();
    r.write_bytes(0x600,&[0x66,0xcf]).unwrap();
    i.selectors=[0x10,0x08,0x10,0x10,0x10,0x10];i.segments=[data0();6];i.segments[1]=ring0();
    for (n,v) in [0x0010,0x0008,0x0202].iter().enumerate(){r.store(0x8000+2*n as u32,MemWidth::Word,*v).unwrap();}
    let mut c=Cpu{regs:[0,0,0,0,0x8000,0,0,0],eflags:0x2,undefined_flags:0,eip:0x600};
    assert_eq!(step(&mut i,&mut r,&mut c),None);
    assert_eq!((c.eip,c.regs[4],c.eflags,i.selectors[1]),(0x10,0x8006,0x202,0x08));
    // A V86 return with EIP beyond FFFFh is #GP(0) and changes nothing.
    let (mut i,mut r)=machine();
    r.write_bytes(0x600,&[0xcf]).unwrap();
    i.selectors=[0x10,0x08,0x10,0x10,0x10,0x10];i.segments=[data0();6];i.segments[1]=ring0();
    for (n,v) in [0x1_0000,0x0060,0x0002_0002,0x0100,0x0070,0,0,0,0].iter().enumerate(){r.store(0x8000+4*n as u32,MemWidth::Dword,*v).unwrap();}
    let mut c=Cpu{regs:[0,0,0,0,0x8000,0,0,0],eflags:0x2,undefined_flags:0,eip:0x600};
    let (old_i,old_c)=(i.clone(),c.clone());
    let mut m=i.segment_state();let d=i.decode(&r,0x600).unwrap();
    let Operation::Pil{program,dst,src,flow,address}=d.operation else{panic!()};
    assert_eq!(execute_machine(&program,&mut c,dst,src,flow,&mut r,address,&mut m),Err(ExecError::GeneralProtection{eip:0x600,error_code:0}));
    assert_eq!((c,i),(old_c,old_i));
}

#[test]
fn with_iopl_3_flags_instructions_and_iret_work_but_iopl_stays(){
    // MOV AX,1234h; MOV DS,AX; PUSHF; PUSH 0; POPF; STI; PUSH 0; PUSH CS; PUSH 40h; IRET.
    let (mut i,mut r,mut c)=in_v86(&[0xb8,0x34,0x12,0x8e,0xd8,0x9c,0x6a,0x00,0x9d,0xfb,0x6a,0x00,0x0e,0x6a,0x40,0xcf],3);
    for _ in 0..3{assert_eq!(step(&mut i,&mut r,&mut c),None);}
    assert_eq!((i.selectors[3],i.segments[3]),(0x1234,v86(0x1234)),"MOV DS loads the V86 cache");
    assert_eq!((word(&r,0x7fe),c.regs[4]),(0x3202,0xfe),"PUSHF: the low FLAGS word at 0070:00FE");
    for _ in 0..2{assert_eq!(step(&mut i,&mut r,&mut c),None);}
    assert_eq!(c.eflags,0x0002_3002,"POPF of 0 clears IF but keeps IOPL 3 and VM");
    assert_eq!(step(&mut i,&mut r,&mut c),None);
    assert_eq!(c.eflags,0x0002_3202,"STI");
    for _ in 0..4{assert_eq!(step(&mut i,&mut r,&mut c),None);}
    assert_eq!((c.eip,i.selectors[1],i.segments[1],c.regs[4]),(0x40,0x60,v86(0x60),0xfe),"IRET pops IP, CS and FLAGS (the PUSHF word stays below)");
    assert_eq!(c.eflags,0x0002_3002,"IRET clears IF from the frame, IOPL and VM stay");
}

#[test]
fn with_iopl_0_sensitive_instructions_are_gp0_with_the_v86_frame_on_the_ring0_stack(){
    for bytes in [vec![0xfa],vec![0xfb],vec![0x9c],vec![0x9d],vec![0x66,0x9c],vec![0xcf],vec![0xcd,0x40],vec![0xcc],vec![0xf4]]{
        let (mut i,mut r,mut c)=in_v86(&bytes,0);
        c.regs[0]=0xaaaa;
        // INT n and INT3 raise the #GP while being delivered, so it is delivered in turn.
        let expected=if bytes[0]==0xcd||bytes[0]==0xcc{None}else{Some(ExecError::GeneralProtection{eip:0,error_code:0})};
        assert_eq!(step(&mut i,&mut r,&mut c),expected,"{bytes:02x?}");
        assert_left_v86(&i,&c,0x5000,0x8fd8);
        // GS, FS, DS, ES, SS, ESP, EFLAGS, CS, EIP, error code.
        assert_eq!([0x8ffc,0x8ff8,0x8ff4,0x8ff0,0x8fec,0x8fe8,0x8fe4,0x8fe0,0x8fdc,0x8fd8].map(|a|dword(&r,a)),[0x83,0x82,0x80,0x81,0x70,0x100,0x20202,0x60,0,0],"{bytes:02x?}");
        assert_eq!(c.regs[0],0xaaaa);
    }
    // HLT is #GP(0) at IOPL 3 too.
    let (mut i,mut r,mut c)=in_v86(&[0xf4],3);
    assert_eq!(step(&mut i,&mut r,&mut c),Some(ExecError::GeneralProtection{eip:0,error_code:0}));
    assert_eq!(dword(&r,0x8fe4),0x23202);
}

#[test]
fn int_n_at_iopl_3_needs_a_dpl_3_gate_to_nonconforming_ring_0_code(){
    // INT 40h: the frame without an error code, then IRETD back to V86 mode.
    let (mut i,mut r,mut c)=in_v86(&[0xcd,0x40],3);
    r.write_bytes(0x5100,&[0xcf]).unwrap();
    assert_eq!(step(&mut i,&mut r,&mut c),None);
    assert_left_v86(&i,&c,0x5100,0x8fdc);
    assert_eq!([0x8ffc,0x8ff8,0x8ff4,0x8ff0,0x8fec,0x8fe8,0x8fe4,0x8fe0,0x8fdc].map(|a|dword(&r,a)),[0x83,0x82,0x80,0x81,0x70,0x100,0x23202,0x60,2]);
    assert_eq!(step(&mut i,&mut r,&mut c),None);
    assert_eq!((c.eip,c.regs[4],c.eflags),(2,0x100,0x23202));
    assert_eq!(i.selectors,[0x81,0x60,0x70,0x80,0x82,0x83]);
    assert_eq!(i.segments,[0x81,0x60,0x70,0x80,0x82,0x83].map(v86));
    // To ring 3 code, to conforming ring 0 code: #GP(selector). A DPL 0
    // gate: #GP(vector*8+2).
    for (vector,code) in [(0x41u8,0x18u32),(0x42,0x28),(0x43,0x43*8+2)]{
        let (mut i,mut r,mut c)=in_v86(&[0xcd,vector],3);
        assert_eq!(step(&mut i,&mut r,&mut c),None,"the #GP is raised during delivery and delivered in turn");
        assert_left_v86(&i,&c,0x5000,0x8fd8);
        assert_eq!((dword(&r,0x8fdc),dword(&r,0x8fd8)),(0,code),"INT {vector:02x}h: return to the INT, error code");
    }
}

#[test]
fn io_in_v86_mode_always_uses_the_bitmap(){
    // IN AL,10h is allowed by the bitmap whatever IOPL is; IN AL,11h is not.
    for iopl in [0,3]{
        let (mut i,mut r,mut c)=in_v86(&[0xe4,0x10],iopl);
        assert_eq!(step(&mut i,&mut r,&mut c),None);
        assert_eq!((c.regs[0]&0xff,c.eip),(0xff,2),"open bus");
        let (mut i,mut r,mut c)=in_v86(&[0xe4,0x11],iopl);
        assert_eq!(step(&mut i,&mut r,&mut c),Some(ExecError::GeneralProtection{eip:0,error_code:0}));
        assert_left_v86(&i,&c,0x5000,0x8fd8);
    }
}

#[test]
fn far_transfers_and_privileged_forms_in_v86_mode(){
    // JMP FAR 0090:0004 loads CS with a V86 cache; CALL FAR pushes as in real mode.
    let (mut i,mut r,mut c)=in_v86(&[0xea,0x04,0x00,0x90,0x00],0);
    assert_eq!(step(&mut i,&mut r,&mut c),None);
    assert_eq!((c.eip,i.selectors[1],i.segments[1]),(4,0x90,v86(0x90)));
    let (mut i,mut r,mut c)=in_v86(&[0x9a,0x00,0x01,0x90,0x00],0);
    assert_eq!(step(&mut i,&mut r,&mut c),None);
    assert_eq!((c.eip,i.selectors[1],c.regs[4]),(0x100,0x90,0xfc));
    assert_eq!((word(&r,0x7fe),word(&r,0x7fc)),(0x60,5));
    // MOV to CR0 is #GP(0); LAR (protected-mode only) is not decoded.
    let (mut i,mut r,mut c)=in_v86(&[0x0f,0x22,0xc0],3);
    assert_eq!(step(&mut i,&mut r,&mut c),Some(ExecError::GeneralProtection{eip:0,error_code:0}));
    let (i,r,_)=in_v86(&[0x0f,0x02,0xc0],3);
    assert!(i.decode(&r,0).is_err());
}
