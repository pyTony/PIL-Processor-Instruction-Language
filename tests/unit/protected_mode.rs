//! Original expectations for protected-mode increment 2: the PE switch,
//! GDT descriptor loads at CPL 0, the CS D bit, and the return to real mode
//! with the protected-mode limits kept ("unreal mode").
//! Image::load gives CS base 0, DS base 100h, SS base 200h and FS base 300h.
use super::*;
use address::{SegmentAccess,SegmentCache};

const GDT:u32=0x1000;
/// Null, flat 32-bit code (08h), flat 32-bit data (10h), 16-bit code (18h),
/// 16-bit data at 10000h (20h), read-only data (28h), not-present data (30h),
/// DPL 3 data (38h), execute-only code (40h).
const DESCRIPTORS:[u64;9]=[0,0x00cf_9a00_0000_ffff,0x00cf_9200_0000_ffff,0x0000_9a00_0000_ffff,0x0000_9201_0000_ffff,0x00cf_9000_0000_ffff,0x00cf_1200_0000_ffff,0x00cf_f200_0000_ffff,0x00cf_9800_0000_ffff];

fn machine(code:&[u8])->(guest::Image,Ram,Cpu){
 let(image,_)=guest::Image::load(code).unwrap();let mut r=Ram::new(0x21000);r.write_bytes(0x100,code).unwrap();
 for (i,d) in DESCRIPTORS.iter().enumerate(){r.write_bytes(GDT+8*i as u32,&d.to_le_bytes()).unwrap();}
 (image,r,Cpu{regs:[0,0,0,0,0x800,0,0,0],eflags:2,undefined_flags:0,eip:0x100})
}
/// Protected mode at CPL 0 with the GDT loaded and a 16-bit code segment.
fn protected(code:&[u8])->(guest::Image,Ram,Cpu){
 let(mut i,r,c)=machine(code);i.real_mode=false;i.msw=0x11;i.tables[0]=(8*DESCRIPTORS.len() as u16-1,GDT);
 i.segments[1].access=SegmentAccess::Code{readable:true,conforming:false,big:false,dpl:0};(i,r,c)
}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
 let d=i.decode(r,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);if result.is_ok(){i.set_segment_state(m);}result}else{panic!("PIL expected at {:x}",c.eip)}
}
fn flat(big:bool,writable:bool)->SegmentAccess{SegmentAccess::Data{writable,expand_down:false,big,dpl:0}}

#[test]
fn switch_to_protected_mode_run_32_bit_code_and_return_to_unreal_mode(){
 let mut code=vec![0u8;0x60];
 let mut put=|at:usize,bytes:&[u8]|code[at-0x100..at-0x100+bytes.len()].copy_from_slice(bytes);
 put(0x100,&[0x0f,0x01,0x16,0x00,0x08]);            // LGDT [0800h] (DS base 100h: linear 900h)
 put(0x105,&[0x0f,0x20,0xc0]);                      // MOV EAX,CR0
 put(0x108,&[0x66,0x83,0xc8,0x01]);                 // OR EAX,1
 put(0x10c,&[0x0f,0x22,0xc0]);                      // MOV CR0,EAX
 put(0x10f,&[0x66,0xea,0x20,0x01,0,0,0x08,0]);      // JMP FAR 0008:00000120
 put(0x120,&[0x66,0xb8,0x10,0x00]);                 // 32-bit code: MOV AX,10h
 put(0x124,&[0x8e,0xd8,0x8e,0xd0,0x8e,0xe0]);       // MOV DS,AX; MOV SS,AX; MOV FS,AX
 put(0x12a,&[0xc7,0x05,0,0x20,0,0,0x78,0x56,0x34,0x12]); // MOV DWORD [2000h],12345678h
 put(0x134,&[0xea,0x40,0x01,0,0,0x18,0]);           // JMP FAR 0018:00000140
 put(0x140,&[0x0f,0x20,0xc0,0x66,0x83,0xe0,0xfe,0x0f,0x22,0xc0]); // 16-bit: clear PE
 put(0x14a,&[0xea,0x50,0x01,0,0]);                  // JMP FAR 0000:0150
 put(0x150,&[0xb8,0,0,0x8e,0xe0]);                  // real mode: MOV AX,0; MOV FS,AX
 put(0x155,&[0x67,0x64,0x66,0x8b,0x03]);            // MOV EAX,FS:[EBX]
 let(mut i,mut r,mut c)=machine(&code);r.write_bytes(0x900,&[0x47,0,0x00,0x10,0,0]).unwrap();r.store(0x12000,MemWidth::Dword,0xcafe_f00d).unwrap();
 for _ in 0..4{step(&mut i,&mut r,&mut c).unwrap();}
 // PE is set but nothing else changed: CS still holds its real-mode cache.
 assert_eq!((i.real_mode,i.msw&1,c.eip,i.segments[1],i.tables[0]),(false,1,0x10f,SegmentCache::real_mode(0),(0x47,GDT)));
 step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!((i.selectors[1],c.eip,i.segments[1]),(8,0x120,SegmentCache{base:0,limit:u32::MAX,access:SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0}}));
 assert_eq!(r.load(GDT+8+5,MemWidth::Byte).unwrap(),0x9b,"the load sets the accessed bit");
 for _ in 0..4{step(&mut i,&mut r,&mut c).unwrap();}
 let flat_data=SegmentCache{base:0,limit:u32::MAX,access:flat(true,true)};
 assert_eq!((i.selectors[3],i.segments[3],i.segments[2],i.segments[4]),(0x10,flat_data,flat_data,flat_data));
 assert_eq!(c.regs[0]&0xffff,0x10);
 step(&mut i,&mut r,&mut c).unwrap();assert_eq!(r.load(0x2000,MemWidth::Dword).unwrap(),0x1234_5678,"32-bit addressing by default");
 step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.selectors[1],c.eip,i.segments[1].default_32()),(0x18,0x140,false));
 for _ in 0..3{step(&mut i,&mut r,&mut c).unwrap();}assert!(i.real_mode);
 step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.selectors[1],c.eip,i.segments[1]),(0,0x150,SegmentCache::real_mode(0)));
 // A real-mode load changes only the base, so FS keeps its 4 GiB limit.
 c.regs[3]=0x12000;for _ in 0..3{step(&mut i,&mut r,&mut c).unwrap();}
 assert_eq!((i.selectors[4],i.segments[4]),(0,flat_data));assert_eq!(c.regs[0],0xcafe_f00d);
}

#[test]
fn data_segment_loads_check_the_descriptor_and_privilege(){
 // 8E D8 = MOV DS,AX; 8E C0 = MOV ES,AX; 8E D0 = MOV SS,AX.
 let gp=|code:u32|Err(ExecError::GeneralProtection{eip:0x100,error_code:code as u16});
 for (op,selector,want) in [
  (0xd8u8,0x28u32,Ok(())),(0xc0,0x38,Ok(())),(0xd8,0x40,gp(0x40)),(0xd8,0x30,Err(ExecError::SegmentNotPresent{eip:0x100,error_code:0x30})),
  (0xd8,0x48,gp(0x48)),(0xd8,0x0c,gp(0x0c)),(0xd8,0x08,Ok(())),(0xd8,0x00,Ok(())),
  (0xd0,0x28,gp(0x28)),(0xd0,0x38,gp(0x38)),(0xd0,0x00,gp(0)),(0xd0,0x13,gp(0x10)),(0xd0,0x08,gp(0x08)),(0xd0,0x10,Ok(())),
 ]{
  let(mut i,mut r,mut c)=protected(&[0x8e,op]);c.regs[0]=selector;let(old_i,old_r,old_c)=(i.clone(),r.clone(),c.clone());
  let result=step(&mut i,&mut r,&mut c);assert_eq!(result,want,"8E {op:02x} with {selector:x}");
  if result.is_err(){assert_eq!((i,r,c),(old_i,old_r,old_c));continue;}
  let reg=match op{0xd8=>3,0xc0=>0,_=>2};assert_eq!(i.selectors[reg],selector as u16);
  if selector==0{assert_eq!(i.segments[reg].access,SegmentAccess::Unusable);}
 }
 // A read-only data segment loads, but a write through it is #GP(0).
 let(mut i,mut r,mut c)=protected(&[0x8e,0xc0,0x26,0x88,0x07]);c.regs[0]=0x28;step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!(i.segments[0],SegmentCache{base:0,limit:u32::MAX,access:flat(true,false)});assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::GeneralProtection{eip:0x102,error_code:0}));
 // A null DS loads; using it is #GP(0).
 let(mut i,mut r,mut c)=protected(&[0x8e,0xd8,0x88,0x07]);step(&mut i,&mut r,&mut c).unwrap();assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::GeneralProtection{eip:0x102,error_code:0}));
 // The 16-bit data segment at 10000h; the load sets its accessed bit.
 let(mut i,mut r,mut c)=protected(&[0x8e,0xd8]);c.regs[0]=0x20;step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!(i.segments[3],SegmentCache{base:0x10000,limit:0xffff,access:flat(false,true)});assert_eq!(r.load(GDT+0x20+5,MemWidth::Byte).unwrap(),0x93);
}

#[test]
fn far_transfers_check_the_code_descriptor(){
 // EA 00 01 sel = JMP FAR sel:0100 from 16-bit protected-mode code.
 let jmp=|selector:u16|{let s=selector.to_le_bytes();protected(&[0xea,0x00,0x01,s[0],s[1]])};
 let(mut i,mut r,mut c)=jmp(0x18);step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.selectors[1],c.eip),(0x18,0x100));
 for (selector,want) in [(0x10u16,Err(ExecError::GeneralProtection{eip:0x100,error_code:0x10})),(0,Err(ExecError::GeneralProtection{eip:0x100,error_code:0})),(0x1b,Err(ExecError::GeneralProtection{eip:0x100,error_code:0x18}))]{
  let(mut i,mut r,mut c)=jmp(selector);let(old_i,old_r,old_c)=(i.clone(),r.clone(),c.clone());assert_eq!(step(&mut i,&mut r,&mut c),want,"selector {selector:x}");assert_eq!((i,r,c),(old_i,old_r,old_c));
 }
 // An offset past the new limit is #GP(0) after the descriptor checks.
 let(mut i,mut r,mut c)=protected(&[0x66,0xea,0,0,1,0,0x18,0]);assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::GeneralProtection{eip:0x100,error_code:0}));
 // A far CALL pushes the old CS and IP and loads the new CS.
 let(mut i,mut r,mut c)=protected(&[0x9a,0x00,0x02,0x18,0x00]);i.segments[2]=SegmentCache{base:0,limit:0xffff,access:flat(false,true)};
 step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.selectors[1],c.eip,c.regs[4]),(0x18,0x200,0x7fc));assert_eq!(r.load(0x7fc,MemWidth::Dword).unwrap(),0x0000_0105);
}

#[test]
fn cr0_writes_set_and_clear_pe(){
 // MOV CR0,EAX: PE alone is accepted; a CR0 bit the 386 lacks is refused.
 let(mut i,mut r,mut c)=machine(&[0x0f,0x22,0xc0]);c.regs[0]=0x11;step(&mut i,&mut r,&mut c).unwrap();assert!(!i.real_mode);
 let(mut i,mut r,mut c)=machine(&[0x0f,0x22,0xc0]);c.regs[0]=0x4000_0011;assert!(matches!(step(&mut i,&mut r,&mut c),Err(ExecError::UnsupportedControl{..})));
 let(mut i,mut r,mut c)=protected(&[0x0f,0x22,0xc0]);c.regs[0]=0x10;step(&mut i,&mut r,&mut c).unwrap();assert!(i.real_mode);
 // At CPL 3 a control-register access is #GP(0).
 let(mut i,mut r,mut c)=protected(&[0x0f,0x20,0xc0]);i.segments[1].access=SegmentAccess::Code{readable:true,conforming:false,big:false,dpl:3};
 assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::GeneralProtection{eip:0x100,error_code:0}));
}
