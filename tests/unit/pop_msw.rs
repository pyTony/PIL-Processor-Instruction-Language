//! Original expected-state tests for memory POP and machine-status reads.
use super::*;
use address::{SegmentAccess};
fn setup(code:&[u8],big:bool)->(guest::Image,Ram,Cpu){
 let(mut i,_)=guest::Image::load(code).unwrap();i.segments[2].access=SegmentAccess::Data{writable:true,expand_down:false,big};
 let mut r=Ram::new(0x22000);r.write_bytes(0x100,code).unwrap();
 let c=Cpu{regs:[0x12345678,0xabcdef01,0x55555555,0x600,0x800,0x800,8,0],eflags:0xad7,undefined_flags:u32::MAX,eip:0x100};(i,r,c)
}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
 let d=i.decode(r,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let res=execute_machine(&program,c,dst,src,flow,r,address,&mut m);i.set_segment_state(m);res}else{panic!()}
}
#[test]
fn memory_pop_sizes_stack_sizes_and_post_increment_esp_address(){
 for w in [16,32]{for big in [false,true]{for sib in [false,true]{
  let mut b=vec![];if w==32{b.push(0x66);}b.push(0x67);b.extend(if sib{vec![0x8f,0x44,0x74,0xfc]}else{vec![0x8f,0x04,0x24]});
  let(mut i,mut r,mut c)=setup(&b,big);r.store(0xa00,MemWidth::from_bits(w).unwrap(),0xdeadbeef).unwrap();let old=c.clone();let oldr=r.clone();let oldi=i.clone();let n=(w/8)as u32;let destination=0xa00+n+if sib{12}else{0};
  step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=b.len()as u32;want.regs[4]+=n;assert_eq!(c,want);assert_eq!(i,oldi);assert_eq!(r.load(destination,MemWidth::from_bits(w).unwrap()).unwrap(),0xdeadbeef&mask(w)as u32);
  assert_eq!(&r.bytes()[0xa00..0xa00+n as usize],&oldr.bytes()[0xa00..0xa00+n as usize]);
 }}}
}
#[test]
fn memory_pop_bp_ds_and_overrides_use_independent_old_ss_source(){
 for prefix in [None,Some(0x3e),Some(0x64),Some(0x2e)]{
  let mut b=vec![];if let Some(p)=prefix{b.push(p);}b.extend([0x8f,0x46,0]);let(mut i,mut r,mut c)=setup(&b,false);c.regs[4]|=0xabcd0000;r.store(0xa00,MemWidth::Word,0xbeef).unwrap();
  let destination=match prefix{None=>0xa00,Some(0x3e)=>0x900,Some(0x64)=>0xb00,_=>0x800};let old=c.clone();step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=b.len()as u32;want.regs[4]=0xabcd0802;assert_eq!(c,want);assert_eq!(r.load(destination,MemWidth::Word).unwrap(),0xbeef);
 }
}
#[test]
fn memory_pop_readonly_source_overlap_and_precise_store_fault(){
 let(mut i,mut r,mut c)=setup(&[0x8f,7],false);r.store(0xa00,MemWidth::Word,0xbeef).unwrap();r.protect(0xa00,2).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(r.load(0x700,MemWidth::Word).unwrap(),0xbeef);assert_eq!(c.regs[4],0x802);
 for fault in 0..4{
  let b=if fault==3{vec![0x67,0x8f,3]}else{vec![0x67,0x8f,4,0x24]};let(mut i,mut r,mut c)=setup(&b,false);r.store(0xa00,MemWidth::Word,0xbeef).unwrap();i.ss_shadow=1;i.sti_shadow=1;
  match fault{0=>r.protect(0xa02,2).unwrap(),1=>i.segments[2].limit=0x802,2=>{let mut short=Ram::new(0xa01);short.write_bytes(0,&r.bytes()[..0xa01]).unwrap();r=short;},_=>{c.regs[3]=0x30000;i.segments[3].limit=0x40000;}}
  let old=c.clone();let oldr=r.clone();let oldi=i.clone();assert!(step(&mut i,&mut r,&mut c).is_err());assert_eq!(c,old);assert_eq!(r,oldr);assert_eq!(i,oldi);
 }
}
#[test]
fn memory_pop_sp_wrap_policy_is_explicit_and_non_esp_destinations_work(){
 for w in [16,32]{let mut b=vec![];if w==32{b.push(0x66);}b.extend([0x67,0x8f,4,0x24]);let(mut i,mut r,mut c)=setup(&b,false);c.regs[4]=65536-(w/8)as u32;r.store(0x200+c.regs[4],MemWidth::from_bits(w).unwrap(),0xbeef).unwrap();let old=c.clone();let oldr=r.clone();assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::InvalidContext));assert_eq!(c,old);assert_eq!(r,oldr);
  let(mut i,mut r,mut c)=setup(&[0x8f,7],false);c.regs[4]=0xabcdfffe;r.store(0x101fe,MemWidth::Word,0xbeef).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.regs[4],0xabcd0000);assert_eq!(r.load(0x700,MemWidth::Word).unwrap(),0xbeef);
 }
}
#[test]
fn memory_pop_prefixes_and_bad_extensions_are_diagnostics(){
 for b in [vec![0x8f,0x0f],vec![0x8f,6,0],vec![0x67,0x8f,4],vec![0xf0,0x8f,7],vec![0xf3,0x8f,7]]{let(i,r)=guest::Image::load(&b).unwrap();assert!(i.decode(&r,0x100).is_err());}
 for text in ["const v 16 0\nread_address_after a 4 v\npreserve_flags all\nretire","const v 32 0\nread_address_after a 8 v\npreserve_flags all\nretire","const v 8 1\nrequire v\npreserve_flags all\nretire"]{assert!(compile(text,16).is_err());}
}
#[test]
fn smsw_word_registers_read_supplied_msw_and_preserve_other_state(){
 for reg in 0..8{for msw in [0u16,0x10,0xa5a0,0xffff]{for real in [false,true]{
  let(mut i,mut r,mut c)=setup(&[0x0f,1,0xe0|reg],false);i.msw=msw;i.real_mode=real;let old=c.clone();let oldi=i.clone();let oldr=r.clone();step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=3;want.regs[reg as usize]=(want.regs[reg as usize]&0xffff0000)|u32::from((msw&!1)|u16::from(!real));assert_eq!(c,want);assert_eq!(i,oldi);assert_eq!(r,oldr);
 }}}
}
#[test]
fn smsw_memory_always_stores_word_with_address_and_segment_overrides(){
 for b in [vec![0x0f,1,0x26,0,6],vec![0x66,0x0f,1,0x26,0,6],vec![0x67,0x0f,1,0x64,0x73,0xfc],vec![0x64,0x0f,1,0x26,0,4]]{
  let(mut i,mut r,mut c)=setup(&b,false);i.msw=0xa5b0;let at=if b[0]==0x67{0x70c}else{0x700};r.store(at,MemWidth::Dword,0xdeadbeef).unwrap();let old=c.clone();step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=b.len()as u32;assert_eq!(c,want);assert_eq!(r.load(at,MemWidth::Dword).unwrap(),0xdeada5b0);
 }
}
#[test]
fn smsw_faults_and_encoding_boundaries_are_transactional(){
 let b=[0x0f,1,0x26,0,6];let(mut i,mut r,mut c)=setup(&b,false);r.protect(0x700,2).unwrap();let old=c.clone();let oldr=r.clone();let oldi=i.clone();assert!(step(&mut i,&mut r,&mut c).is_err());assert_eq!(c,old);assert_eq!(r,oldr);assert_eq!(i,oldi);
 for b in [vec![0x0f,1,0],vec![0x66,0x0f,1,0xe0],vec![0x0f,1,0x26,0],vec![0xf0,0x0f,1,0xe0]]{let(i,r)=guest::Image::load(&b).unwrap();assert!(i.decode(&r,0x100).is_err());}
}
#[test]
fn msw_state_is_retained_across_other_instructions_and_part_of_traces(){
 let(mut i,mut r,mut c)=setup(&[0x90],false);i.msw=0xa5b0;step(&mut i,&mut r,&mut c).unwrap();assert_eq!(i.msw,0xa5b0);
 let a=i.segment_state();let mut b=a;b.msw^=2;let record=|s:&address::SegmentState|{let mut v=Vec::new();trace::record_machine(&mut v,0,[0;5],&c.regs,c.eflags,c.undefined_flags,c.eip,&[],&[],&[],s);v};assert!(native::verify_traces(&record(&a),&record(&b)).is_err());
}
