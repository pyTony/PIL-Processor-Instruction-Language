//! Original expectations for protected-mode increment 3: CR0.PG, the 386
//! two-level page walk (CR3, PDE, PTE), accessed and dirty bits, user and
//! read-only checks, and #PF error codes. The faulting linear address goes
//! with the error (CR2 is written once #PF is delivered, in increment 4).
use super::*;
use address::SegmentAccess;
use memory::Paging;

const PD:u32=0x10000;
const PT0:u32=0x11000;
const PT_HIGH:u32=0x12000;

fn dword(r:&Ram,at:u32)->u32{
 let mut flat=r.clone();flat.set_paging(None);flat.load(at,MemWidth::Dword).unwrap()
}
/// Page tables: linear 0-1FFFh map to themselves (user, writable), linear
/// 2000h maps to frame 9000h (supervisor, writable), 3000h is absent,
/// 4000h maps to frame 4000h (user, read-only), and E0000000h maps to frame
/// 5000h (supervisor, writable) through a second table.
fn tables()->Ram{
 let mut r=Ram::new(0x20000);
 let mut put=|at:u32,v:u32|r.store(at,MemWidth::Dword,v).unwrap();
 put(PD,PT0|7);put(PD+0x380*4,PT_HIGH|7);
 put(PT0,7);put(PT0+4,0x1000|7);put(PT0+8,0x9000|3);put(PT0+16,0x4000|5);
 put(PT_HIGH,0x5000|3);
 r
}

#[test]
fn walk_maps_linear_pages_and_sets_accessed_and_dirty_bits(){
 let mut r=tables();r.set_paging(Some(Paging{directory:PD,user:false}));
 r.store(0xe000_0010,MemWidth::Dword,0x1234_5678).unwrap();
 assert_eq!(r.load(0xe000_0010,MemWidth::Dword).unwrap(),0x1234_5678);
 assert_eq!(dword(&r,0x5010),0x1234_5678,"E0000010h is frame 5000h");
 assert_eq!(dword(&r,PD+0x380*4),PT_HIGH|0x27,"the PDE gets A only");
 assert_eq!(dword(&r,PT_HIGH),0x5000|0x63,"a write sets A and D in the PTE");
 // A read sets A, not D; a page left alone keeps both clear.
 r.load(0x2004,MemWidth::Word).unwrap();
 assert_eq!((dword(&r,PT0+8),dword(&r,PT0+4)),(0x9000|0x23,0x1000|7));
 // A word that crosses from 1FFFh into 2000h takes one byte from each frame.
 r.store(0x1fff,MemWidth::Word,0xbbaa).unwrap();
 assert_eq!((dword(&r,0x1ffc)>>24,dword(&r,0x9000)&0xff),(0xaa,0xbb));
 // Paging off: linear is physical again.
 r.set_paging(None);assert_eq!(r.load(0x2000,MemWidth::Byte).unwrap(),0);
}

#[test]
fn page_faults_name_the_linear_address_and_set_no_accessed_bits(){
 let fault=|r:&Ram,linear:u32,write:bool|r.translate(linear,write).unwrap_err();
 let mut r=tables();r.set_paging(Some(Paging{directory:PD,user:false}));
 // Missing PDE (linear 400000h) and missing PTE (3000h): bit 0 clear.
 for (linear,write,code) in [(0x40_0000,false,0),(0x40_0000,true,2),(0x3000,false,0),(0x3abc,true,2)]{
  let f=fault(&r,linear,write);assert_eq!((f.address,f.page),(linear,Some(code)));
  assert_eq!(f.error(0x100),ExecError::PageFault{eip:0x100,address:linear,error_code:u16::from(code)});
 }
 // CPL 0 writes to a read-only user page (the 386 has no CR0.WP).
 r.store(0x4000,MemWidth::Byte,1).unwrap();
 // User level: a supervisor page is a protection fault, so is a write to
 // a read-only page; error bit 2 is set for user accesses.
 r.set_paging(Some(Paging{directory:PD,user:true}));
 for (linear,write,code) in [(0xe000_0000,false,5),(0x2000,true,7),(0x4000,true,7),(0x3000,false,4)]{
  let f=fault(&r,linear,write);assert_eq!((f.address,f.page),(linear,Some(code)));
 }
 assert_eq!(r.load(0x4000,MemWidth::Byte).unwrap(),1,"user reads of a read-only page work");
 r.set_paging(None);
 assert_eq!(dword(&r,PD+0x380*4),PT_HIGH|7,"a faulting walk leaves the PDE's A bit clear");
 assert_eq!(dword(&r,PT0+8),0x9000|3);
 // A dword from 2FFEh: the fault names 3000h, and a store writes nothing.
 let mut r=tables();r.set_paging(Some(Paging{directory:PD,user:false}));let before=r.clone();
 let f=r.store(0x2ffe,MemWidth::Dword,0xffff_ffff).unwrap_err();assert_eq!((f.address,f.page),(0x3000,Some(2)));
 let (mut a,mut b)=(r,before);a.set_paging(None);b.set_paging(None);assert_eq!(a.bytes()[0x9ff0..0xa000],b.bytes()[0x9ff0..0xa000]);
 // A table outside RAM is a bus fault, not #PF.
 let mut r=tables();r.store(PD+4,MemWidth::Dword,0x00ff_f007).unwrap();r.set_paging(Some(Paging{directory:PD,user:false}));
 assert_eq!(fault(&r,0x40_0000,false).page,None);
}

/// 16-bit protected mode at CPL 0, code at 100h, DS base 100h.
fn protected(code:&[u8])->(guest::Image,Ram,Cpu){
 let(mut i,_)=guest::Image::load(code).unwrap();let mut r=tables();r.write_bytes(0x100,code).unwrap();
 i.real_mode=false;i.msw=0x11;i.segments[1].access=SegmentAccess::Code{readable:true,conforming:false,big:false,dpl:0};
 (i,r,Cpu{regs:[0,0,0,0,0x800,0,0,0],eflags:2,undefined_flags:0,eip:0x100})
}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
 let d=i.decode(r,c.eip).map_err(|e|match e{guest::GuestError::Execution(e)=>e,e=>panic!("{e:?}")})?;
 if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);if result.is_ok(){i.set_segment_state(m);}result}else{panic!("PIL expected at {:x}",c.eip)}
}

#[test]
fn mov_cr0_turns_paging_on_and_lmsw_keeps_it(){
 let code=[
  0x66,0xb8,0x00,0x00,0x01,0x00,   // 100: MOV EAX,10000h
  0x0f,0x22,0xd8,                  // 106: MOV CR3,EAX
  0x66,0xb8,0x11,0x00,0x00,0x80,   // 109: MOV EAX,80000011h
  0x0f,0x22,0xc0,                  // 10F: MOV CR0,EAX
  0xb8,0x34,0x12,                  // 112: MOV AX,1234h
  0xa3,0x00,0x1f,                  // 115: MOV [1F00h],AX (linear 2000h)
  0xb8,0x01,0x00,0x0f,0x01,0xf0,   // 118: MOV AX,1; LMSW AX
  0x0f,0x20,0xc1,                  // 11E: MOV ECX,CR0
  0xa1,0x00,0x2f,                  // 121: MOV AX,[2F00h] (linear 3000h)
 ];
 let(mut i,mut r,mut c)=protected(&code);
 for _ in 0..4{step(&mut i,&mut r,&mut c).unwrap();}
 assert_eq!((i.paging,i.controls[1],c.eip),(true,PD,0x112));
 for _ in 0..2{step(&mut i,&mut r,&mut c).unwrap();}
 assert_eq!(dword(&r,0x9000)&0xffff,0x1234,"linear 2000h is frame 9000h");
 assert_eq!(dword(&r,PT0+8),0x9000|0x63);
 assert_eq!(dword(&r,PT0)&0x60,0x20,"fetching from page 0 sets its A bit only");
 for _ in 0..3{step(&mut i,&mut r,&mut c).unwrap();}
 assert_eq!((c.regs[1],i.paging),(0x8000_0011,true),"MOV r32,CR0 reads PG; LMSW leaves it");
 assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::PageFault{eip:0x121,address:0x3000,error_code:0}));
 assert_eq!((c.eip,c.regs[0]&0xffff),(0x121,1),"the faulting load commits nothing");
}

#[test]
fn clearing_pe_and_pg_together_returns_to_real_mode(){
 let code=[0x66,0xb8,0x10,0,0,0,0x0f,0x22,0xc0]; // MOV EAX,10h; MOV CR0,EAX
 let(mut i,mut r,mut c)=protected(&code);i.paging=true;i.msw=0x11;i.controls[1]=PD;
 for _ in 0..2{step(&mut i,&mut r,&mut c).unwrap();}
 assert_eq!((i.paging,i.real_mode),(false,true));
 assert!(r.translate(0x3000,false).is_ok(),"RAM is unpaged after the instruction");
}

#[test]
fn user_level_data_access_to_a_supervisor_page_is_a_protection_fault(){
 let code=[0xa1,0x00,0x1f];                       // MOV AX,[1F00h] (linear 2000h)
 let(mut i,mut r,mut c)=protected(&code);i.paging=true;i.controls[1]=PD;
 i.segments[1].access=SegmentAccess::Code{readable:true,conforming:false,big:false,dpl:3};
 r.set_paging(i.segment_state().ram_paging());
 assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::PageFault{eip:0x100,address:0x2000,error_code:5}));
 // At CPL 0 the same load works.
 i.segments[1].access=SegmentAccess::Code{readable:true,conforming:false,big:false,dpl:0};
 step(&mut i,&mut r,&mut c).unwrap();
}
