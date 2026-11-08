//! Original expectations for protected-mode increment 2, second part: LDTR
//! and TR (LLDT, LTR, SLDT, STR), LDT selectors in segment loads, SGDT and
//! SIDT, SMSW r32, the descriptor queries LAR, LSL, VERR and VERW, and ARPL.
//! Image::load gives CS base 0, DS base 100h, SS base 200h and FS base 300h.
use super::*;
use address::{SegmentAccess,SegmentCache,SystemSegment};

const GDT:u32=0x1000;
const LDT:u32=0x3000;
/// 00 null, 08 flat 32-bit code, 10 flat data, 18 16-bit code, 20 LDT at
/// 3000h (limit 17h), 28 available 386 TSS at 4000h (limit 67h), 30 call
/// gate, 38 DPL 3 data with byte limit 1234h, 40 DPL 3 conforming
/// execute-only code, 48 interrupt gate, 50 not-present LDT, 58 execute-only
/// code, 60 DPL 0 conforming readable code.
const DESCRIPTORS:[u64;13]=[0,0x00cf_9a00_0000_ffff,0x00cf_9200_0000_ffff,0x0000_9a00_0000_ffff,0x0000_8200_3000_0017,0x0000_8900_4000_0067,
    0x0000_8c00_0008_0000,0x0000_f200_0000_1234,0x00cf_fc00_0000_ffff,0x0000_8e00_0008_0000,0x0000_0200_3000_0017,0x00cf_9800_0000_ffff,0x00cf_9e00_0000_ffff];
/// LDT: 04 flat data, 0C 16-bit data at 20000h, 14 DPL 3 data.
const LOCAL:[u64;3]=[0x00cf_9200_0000_ffff,0x0000_9202_0000_ffff,0x00cf_f200_0000_ffff];
const ZF:u32=0x40;

fn machine(code:&[u8])->(guest::Image,Ram,Cpu){
 let(image,_)=guest::Image::load(code).unwrap();let mut r=Ram::new(0x21000);r.write_bytes(0x100,code).unwrap();
 for (i,d) in DESCRIPTORS.iter().enumerate(){r.write_bytes(GDT+8*i as u32,&d.to_le_bytes()).unwrap();}
 for (i,d) in LOCAL.iter().enumerate(){r.write_bytes(LDT+8*i as u32,&d.to_le_bytes()).unwrap();}
 (image,r,Cpu{regs:[0,0,0,0,0x800,0,0,0],eflags:2,undefined_flags:0,eip:0x100})
}
/// Protected mode at `cpl` with the GDT loaded and a 16-bit code segment.
fn protected_at(code:&[u8],cpl:u8)->(guest::Image,Ram,Cpu){
 let(mut i,r,c)=machine(code);i.real_mode=false;i.msw=0x11;i.tables[0]=(8*DESCRIPTORS.len() as u16-1,GDT);
 i.segments[1].access=SegmentAccess::Code{readable:true,conforming:false,big:false,dpl:cpl};(i,r,c)
}
fn protected(code:&[u8])->(guest::Image,Ram,Cpu){protected_at(code,0)}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
 let d=i.decode(r,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);if result.is_ok(){i.set_segment_state(m);}result}else{panic!("PIL expected at {:x}",c.eip)}
}
/// Runs one instruction and checks that a fault leaves everything as it was.
fn run(code:&[u8],setup:impl Fn(&mut guest::Image,&mut Cpu))->(Result<(),ExecError>,guest::Image,Ram,Cpu){
 let(mut i,mut r,mut c)=protected(code);setup(&mut i,&mut c);let(old_i,old_r,old_c)=(i.clone(),r.clone(),c.clone());
 let result=step(&mut i,&mut r,&mut c);
 if result.is_err(){assert_eq!((&i,&r,&c),(&old_i,&old_r,&old_c),"{code:02x?} must not change state on a fault");}
 (result,i,r,c)
}
fn gp(code:u16)->Result<(),ExecError>{Err(ExecError::GeneralProtection{eip:0x100,error_code:code})}

#[test]
fn sgdt_sidt_and_smsw_r32_work_in_real_and_protected_mode(){
 // SGDT [0800h]; SIDT [0806h] (DS base 100h: linear 900h).
 let code=[0x0f,0x01,0x06,0x00,0x08,0x0f,0x01,0x0e,0x06,0x08];
 let(mut i,mut r,mut c)=machine(&code);i.tables[0]=(0x5f,0x00ab_1000);
 step(&mut i,&mut r,&mut c).unwrap();step(&mut i,&mut r,&mut c).unwrap();
 let mut stored=[0u8;12];for (k,b) in stored.iter_mut().enumerate(){*b=r.load(0x900+k as u32,MemWidth::Byte).unwrap() as u8;}
 assert_eq!(stored,[0x5f,0,0,0x10,0xab,0,0xff,0x03,0,0,0,0],"limit word, then the whole base dword even with a 16-bit operand");
 assert_eq!((c.eip,c.eflags),(0x10a,2));
 // A 32-bit SGDT in protected mode, and the register form is refused.
 let(mut i,mut r,mut c)=protected(&[0x66,0x0f,0x01,0x06,0x00,0x08]);i.tables[0].1=0x1234_5678;step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!((r.load(0x900,MemWidth::Word).unwrap(),r.load(0x902,MemWidth::Dword).unwrap()),(0x67,0x1234_5678));
 let(i,r,_)=machine(&[0x0f,0x01,0xc0]);assert!(i.decode(&r,0x100).is_err());
 // A store past the segment limit is #GP(0) and writes nothing.
 let(mut i,mut r,mut c)=machine(&[0x0f,0x01,0x06,0xfc,0xff]);let old=r.clone();
 assert_eq!(step(&mut i,&mut r,&mut c),gp(0));assert_eq!(r,old);
 // SMSW EAX writes all of CR0 (its upper bits are 0 without paging).
 let(mut i,mut r,mut c)=machine(&[0x66,0x0f,0x01,0xe0]);c.regs[0]=0x1234_5678;step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.regs[0],0x10);
 let(mut i,mut r,mut c)=protected(&[0x66,0x0f,0x01,0xe0]);c.regs[0]=u32::MAX;step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.regs[0],0x11);
}

#[test]
fn protected_only_forms_are_refused_in_real_mode(){
 for code in [vec![0x0f,0x00,0xc0],vec![0x0f,0x00,0xd0],vec![0x0f,0x00,0xe0],vec![0x0f,0x02,0xc0],vec![0x0f,0x03,0xc0],vec![0x63,0xc0]]{
  let(i,r,_)=machine(&code);assert!(i.decode(&r,0x100).is_err(),"{code:02x?} is #UD in real mode");
  let(i,r,_)=protected(&code);assert!(i.decode(&r,0x100).is_ok(),"{code:02x?} decodes in protected mode");
 }
 let(i,r,_)=protected(&[0x0f,0x00,0xf0]);assert!(i.decode(&r,0x100).is_err(),"0F00 /6 is undefined");
}

#[test]
fn lldt_and_ltr_check_the_descriptor_and_mark_the_tss_busy(){
 // 0F 00 D0 = LLDT AX; 0F 00 D8 = LTR AX.
 let(result,i,_,_)=run(&[0x0f,0x00,0xd0],|_,c|c.regs[0]=0x20);result.unwrap();
 assert_eq!(i.ldt_tr[0],SystemSegment{selector:0x20,base:LDT,limit:0x17,kind:2});
 let(result,i,r,_)=run(&[0x0f,0x00,0xd8],|_,c|c.regs[0]=0x28);result.unwrap();
 assert_eq!(i.ldt_tr[1],SystemSegment{selector:0x28,base:0x4000,limit:0x67,kind:0xb});
 assert_eq!(r.load(GDT+0x28+5,MemWidth::Byte).unwrap(),0x8b,"LTR marks the TSS busy");
 // A busy TSS, the wrong type, a not-present LDT, an LDT selector and a
 // selector past the GDT limit all fault with nothing changed.
 let mut busy=DESCRIPTORS[5];busy|=0x0200_0000_0000;
 let(mut i,mut r,mut c)=protected(&[0x0f,0x00,0xd8]);r.write_bytes(GDT+0x28,&busy.to_le_bytes()).unwrap();c.regs[0]=0x28;assert_eq!(step(&mut i,&mut r,&mut c),gp(0x28));
 for (op,selector,want) in [(0xd0u8,0x28u32,gp(0x28)),(0xd8,0x20,gp(0x20)),(0xd0,0x50,Err(ExecError::SegmentNotPresent{eip:0x100,error_code:0x50})),
  (0xd0,0x24,gp(0x24)),(0xd8,0x2c,gp(0x2c)),(0xd0,0x68,gp(0x68)),(0xd8,0x00,gp(0)),(0xd8,0x10,gp(0x10))]{
  let(result,_,_,_)=run(&[0x0f,0x00,op],|_,c|c.regs[0]=selector);assert_eq!(result,want,"0F 00 {op:02x} with {selector:x}");
 }
 // A null LLDT empties LDTR; RPL is ignored.
 let(result,i,_,_)=run(&[0x0f,0x00,0xd0],|i,c|{c.regs[0]=3;i.ldt_tr[0]=SystemSegment{selector:0x20,base:LDT,limit:0x17,kind:2};});result.unwrap();
 assert_eq!(i.ldt_tr[0],SystemSegment{selector:3,..SystemSegment::default()});
 let(result,i,_,_)=run(&[0x0f,0x00,0xd0],|_,c|c.regs[0]=0x23);result.unwrap();assert_eq!(i.ldt_tr[0].selector,0x23);
 // A memory operand: LLDT [0800h].
 let(mut i,mut r,mut c)=protected(&[0x0f,0x00,0x16,0x00,0x08]);r.store(0x900,MemWidth::Word,0x20).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(i.ldt_tr[0].base,LDT);
 // Above CPL 0 it is #GP(0).
 let(mut i,mut r,mut c)=protected_at(&[0x0f,0x00,0xd0],3);c.regs[0]=0x20;assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::GeneralProtection{eip:0x100,error_code:0}));
}

#[test]
fn sldt_and_str_store_the_selectors(){
 let loaded=|i:&mut guest::Image,c:&mut Cpu|{i.ldt_tr=[SystemSegment{selector:0x20,base:LDT,limit:0x17,kind:2},SystemSegment{selector:0x28,base:0x4000,limit:0x67,kind:0xb}];c.regs[3]=0xdead_beef;};
 // SLDT BX keeps the upper half; with 66h the selector is zero-extended.
 let(result,_,_,c)=run(&[0x0f,0x00,0xc3],loaded);result.unwrap();assert_eq!(c.regs[3],0xdead_0020);
 let(result,_,_,c)=run(&[0x66,0x0f,0x00,0xcb],loaded);result.unwrap();assert_eq!(c.regs[3],0x28);
 // Memory always gets a word, even with 66h: STR [0800h].
 let(mut i,mut r,mut c)=protected(&[0x66,0x0f,0x00,0x0e,0x00,0x08]);loaded(&mut i,&mut c);r.store(0x900,MemWidth::Dword,0xffff_ffff).unwrap();
 step(&mut i,&mut r,&mut c).unwrap();assert_eq!(r.load(0x900,MemWidth::Dword).unwrap(),0xffff_0028);
 // Any privilege level may read them.
 let(mut i,mut r,mut c)=protected_at(&[0x0f,0x00,0xc3],3);loaded(&mut i,&mut c);step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.regs[3]&0xffff,0x20);
}

#[test]
fn segment_loads_read_the_ldt(){
 let with_ldt=|i:&mut guest::Image,c:&mut Cpu|{i.ldt_tr[0]=SystemSegment{selector:0x20,base:LDT,limit:0x17,kind:2};c.regs[0]=0x0c;};
 // MOV DS,AX with 0Ch: the 16-bit data segment at 20000h in the LDT.
 let(result,i,r,_)=run(&[0x8e,0xd8],with_ldt);result.unwrap();
 assert_eq!((i.selectors[3],i.segments[3]),(0x0c,SegmentCache{base:0x20000,limit:0xffff,access:SegmentAccess::Data{writable:true,expand_down:false,big:false,dpl:0}}));
 assert_eq!(r.load(LDT+8+5,MemWidth::Byte).unwrap(),0x93,"the LDT entry is marked accessed");
 // Without an LDT, or past its limit, a TI=1 selector is #GP(selector).
 let(result,_,_,_)=run(&[0x8e,0xd8],|_,c|c.regs[0]=0x0c);assert_eq!(result,gp(0x0c));
 let(result,_,_,_)=run(&[0x8e,0xd8],|i,c|{with_ldt(i,c);c.regs[0]=0x1c;});assert_eq!(result,gp(0x1c));
 // LDT entry 0 is an ordinary descriptor (only GDT entry 0 is null).
 let(result,i,_,_)=run(&[0x8e,0xd8],|i,c|{with_ldt(i,c);c.regs[0]=0x04;});result.unwrap();assert_eq!(i.segments[3].limit,u32::MAX);
 // DPL 3 data through an RPL 3 LDT selector at CPL 0.
 let(result,i,_,_)=run(&[0x8e,0xc0],|i,c|{with_ldt(i,c);c.regs[0]=0x17;});result.unwrap();assert_eq!(i.selectors[0],0x17);
}

/// One descriptor query at `cpl`: returns (ZF, destination after).
fn query(code:&[u8],selector:u32,cpl:u8)->(bool,u32){
 let(mut i,mut r,mut c)=protected_at(code,cpl);i.ldt_tr[0]=SystemSegment{selector:0x20,base:LDT,limit:0x17,kind:2};
 c.regs[1]=selector;c.regs[0]=0x5555_5555;c.eflags=2|0x8d5;let old_ram=r.clone();
 step(&mut i,&mut r,&mut c).unwrap();assert_eq!(r,old_ram,"queries never write memory");
 assert_eq!(c.eflags&!ZF,(2|0x8d5)&!ZF,"only ZF changes");(c.eflags&ZF!=0,c.regs[0])
}

#[test]
fn lar_lsl_verr_and_verw_check_type_and_privilege(){
 // 66 0F 02 C1 = LAR EAX,CX; 66 0F 03 C1 = LSL EAX,CX; 0F 00 E1 = VERR CX; 0F 00 E9 = VERW CX.
 let (lar,lsl,verr,verw)=([0x66,0x0f,0x02,0xc1],[0x66,0x0f,0x03,0xc1],[0x0f,0x00,0xe1],[0x0f,0x00,0xe9]);
 let none=0x5555_5555;
 // selector, CPL, LAR, LSL, VERR, VERW (None: ZF=0 and EAX unchanged).
 for (selector,cpl,want_lar,want_lsl,want_r,want_w) in [
  (0x08u32,0u8,Some(0x00cf_9a00u32),Some(u32::MAX),true,false),
  (0x10,0,Some(0x00cf_9200),Some(u32::MAX),true,true),
  (0x20,0,Some(0x0000_8200),Some(0x17),false,false),
  (0x28,0,Some(0x0000_8900),Some(0x67),false,false),
  (0x30,0,Some(0x0000_8c00),None,false,false),
  (0x38,0,Some(0x0000_f200),Some(0x1234),true,true),
  (0x3b,3,Some(0x0000_f200),Some(0x1234),true,true),
  (0x40,0,Some(0x00cf_fc00),Some(u32::MAX),false,false),
  (0x48,0,None,None,false,false),
  (0x50,0,Some(0x0000_0200),Some(0x17),false,false),
  (0x58,0,Some(0x00cf_9800),Some(u32::MAX),false,false),
  (0x14,0,Some(0x00cf_f200),Some(u32::MAX),true,true),
  // RPL 3 or CPL 3 hides a DPL 0 descriptor; conforming code stays visible.
  (0x13,0,None,None,false,false),(0x10,3,None,None,false,false),(0x2b,3,None,None,false,false),(0x40,3,Some(0x00cf_fc00),Some(u32::MAX),false,false),(0x63,3,Some(0x00cf_9e00),Some(u32::MAX),true,false),
  // Null, past the GDT limit, past the LDT limit.
  (0x00,0,None,None,false,false),(0x03,0,None,None,false,false),(0x68,0,None,None,false,false),(0x1c,0,None,None,false,false),
 ]{
  assert_eq!(query(&lar,selector,cpl),(want_lar.is_some(),want_lar.unwrap_or(none)),"LAR {selector:x} at CPL {cpl}");
  assert_eq!(query(&lsl,selector,cpl),(want_lsl.is_some(),want_lsl.unwrap_or(none)),"LSL {selector:x} at CPL {cpl}");
  assert_eq!(query(&verr,selector,cpl).0,want_r,"VERR {selector:x} at CPL {cpl}");
  assert_eq!(query(&verw,selector,cpl).0,want_w,"VERW {selector:x} at CPL {cpl}");
 }
 // 16-bit forms keep EAX's upper half: LAR AX,CX masks to FF00h.
 assert_eq!(query(&[0x0f,0x02,0xc1],0x10,0),(true,0x5555_9200));
 assert_eq!(query(&[0x0f,0x03,0xc1],0x38,0),(true,0x5555_1234));
 // A memory selector operand: LAR EAX,[0800h].
 let(mut i,mut r,mut c)=protected(&[0x66,0x0f,0x02,0x06,0x00,0x08]);r.store(0x900,MemWidth::Word,0x10).unwrap();step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!((c.regs[0],c.eflags&ZF),(0x00cf_9200,ZF));
}

#[test]
fn arpl_raises_the_destination_rpl(){
 // 63 D8 = ARPL AX,BX.
 for (dst,src,want,zf) in [(0x10u32,0x03u32,0x13u32,true),(0x13,0x01,0x13,false),(0x12,0x02,0x12,false),(0xffff_0011,0x0002,0xffff_0012,true)]{
  let(result,_,_,c)=run(&[0x63,0xd8],|_,c|{c.regs[0]=dst;c.regs[3]=src;c.eflags=0x8d5|2;});result.unwrap();
  assert_eq!((c.regs[0],c.eflags&ZF!=0,c.eflags&!ZF),(want,zf,(0x8d5|2)&!ZF),"ARPL {dst:x},{src:x}");
 }
 // ARPL [0800h],BX writes the word back; a read-only segment faults.
 let(mut i,mut r,mut c)=protected(&[0x63,0x1e,0x00,0x08]);r.store(0x900,MemWidth::Word,0x0020).unwrap();c.regs[3]=1;step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!(r.load(0x900,MemWidth::Word).unwrap(),0x21);
 let(mut i,mut r,mut c)=protected(&[0x63,0x1e,0x00,0x08]);i.segments[3].access=SegmentAccess::Data{writable:false,expand_down:false,big:false,dpl:0};c.regs[3]=1;
 assert_eq!(step(&mut i,&mut r,&mut c),gp(0));
}
