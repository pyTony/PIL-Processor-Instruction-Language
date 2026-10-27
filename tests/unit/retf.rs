//! Original real-mode far-return expectations, independent of generated traces.
use super::*;
use address::{SegmentAccess,SegmentCache};
fn setup(w:u8,big:bool,cleanup:Option<u16>)->(guest::Image,Ram,Cpu){
 let mut code=Vec::new();if w==32{code.push(0x66);}if let Some(n)=cleanup{code.push(0xca);code.extend(n.to_le_bytes());}else{code.push(0xcb);}
 let(mut image,_)=guest::Image::load(&code).unwrap();image.segments[2].access=SegmentAccess::Data{writable:true,expand_down:false,big};
 let mut ram=Ram::new(0x21000);ram.write_bytes(0x100,&code).unwrap();let c=Cpu{regs:[0x12345678,0xabcdef01,2,3,0xabcd0800,5,6,7],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};let mut c=c;if big{c.regs[4]=0x800;} (image,ram,c)
}
fn frame(r:&mut Ram,w:u8,target:u32,selector:u32){let width=MemWidth::from_bits(w).unwrap();r.store(0xa00,width,target).unwrap();r.store(0xa00+(w/8)as u32,width,selector).unwrap();}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
 let d=i.decode(r,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);i.set_segment_state(m);result}else{panic!()}
}
#[test]
fn retf_sizes_selectors_cleanup_and_preserved_cpu_ram(){
 for w in [16,32]{for big in [false,true]{for cleanup in [None,Some(0),Some(7),Some(0xffff)]{for selector in [0,1,0x1234,0xffff]{
  let(mut i,mut r,mut c)=setup(w,big,cleanup);frame(&mut r,w,0x345,0xabcd0000|selector);r.protect(0xa00,(w/4)as usize).unwrap();i.ss_shadow=1;i.sti_shadow=1;
  let old=c.clone();let oldr=r.clone();let oldi=i.clone();let mut want=old;want.eip=0x345;let mask=if big{u32::MAX}else{65535};want.regs[4]=(want.regs[4]&!mask)|(want.regs[4].wrapping_add((w/4)as u32+u32::from(cleanup.unwrap_or(0)))&mask);
  step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c,want);assert_eq!(r,oldr);assert_eq!(i.selectors[1],selector as u16);assert_eq!(i.segments[1],SegmentCache::real_mode(selector as u16));assert_eq!(i.segments[2],oldi.segments[2]);assert_eq!(i.selectors[2],oldi.selectors[2]);assert_eq!((i.ss_shadow,i.sti_shadow),(0,0));assert_eq!((i.start,i.end,i.fetch_base),(oldi.start,oldi.end,oldi.fetch_base));
 }}}}
}
#[test]
fn retf_checks_new_code_limit_and_does_not_use_old_limit(){
 let(mut i,mut r,mut c)=setup(32,false,None);i.segments[1].limit=0x101;frame(&mut r,32,0xffff,1);step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.eip,0xffff);assert_eq!(i.segments[1].limit,0xffff);
 for target in [0x10000,u32::MAX]{let(mut i,mut r,mut c)=setup(32,false,None);frame(&mut r,32,target,0x1234);let old=c.clone();let oldr=r.clone();let oldi=i.clone();assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::GeneralProtection{eip:0x100,error_code:0}));assert_eq!(c,old);assert_eq!(r,oldr);assert_eq!(i,oldi);}
}
#[test]
fn retf_stack_segment_and_ram_faults_commit_nothing(){
 for w in [16,32]{for big in [false,true]{for failure in 0..4{
  let(mut i,mut r,mut c)=setup(w,big,Some(7));frame(&mut r,w,0x345,1);i.ss_shadow=1;i.sti_shadow=1;
  match failure{0=>i.segments[2].limit=0x800+(w/4)as u32-2,1=>{i.segments[2].limit=if big{u32::MAX}else{65535};c.regs[4]=if big{u32::MAX-1}else{0xabcdffff};},2=>{i.segments[2].access=SegmentAccess::Data{writable:true,expand_down:true,big};i.segments[2].limit=0x800;},_=>{let n=0xa00+(w/4)as usize-1;let mut short=Ram::new(n);short.write_bytes(0,&r.bytes()[..n]).unwrap();r=short;}}
  let old=c.clone();let oldr=r.clone();let oldi=i.clone();let result=step(&mut i,&mut r,&mut c);assert!(result.is_err(),"w={w} big={big} failure={failure}");if failure==0||failure==2||big&&failure==1{assert!(matches!(result,Err(ExecError::StackSegment{..})));}assert_eq!(c,old);assert_eq!(r,oldr);assert_eq!(i,oldi);
 }}}
}
#[test]
fn retf_exact_stack_end_wraps_only_after_both_slots_are_read(){
 for w in [16,32]{let(mut i,mut r,mut c)=setup(w,false,Some(3));let n=(w/4)as u32;c.regs[4]=0xabcd0000|(0x10000-n);let at=0x200+(c.regs[4]&65535);r.store(at,MemWidth::from_bits(w).unwrap(),0x345).unwrap();r.store(at+(w/8)as u32,MemWidth::from_bits(w).unwrap(),1).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.regs[4],0xabcd0003);assert_eq!(c.eip,0x345);}
}
#[test]
fn retf_rejects_protected_and_vm_context_before_reading_stack(){
 for vm in [false,true]{let(mut i,mut r,mut c)=setup(16,false,None);i.real_mode=vm;if vm{c.eflags|=0x20000;}i.segments[2].access=SegmentAccess::Unusable;let old=c.clone();let oldi=i.clone();let oldr=r.clone();assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::InvalidContext));assert_eq!(c,old);assert_eq!(i,oldi);assert_eq!(r,oldr);}
}
#[test]
fn retf_fetches_new_cs_ip_and_keeps_physical_image_bounds(){
 // Target physical address101 belongs to the original image even though IP=F1.
 let code=[0xcb,0x41,0x90,0xf4];let(mut i,mut r)=guest::Image::load(&code).unwrap();let mut c=Cpu{eip:0x100,..Cpu::default()};c.regs[4]=0x800;r.store(0xa00,MemWidth::Word,0xf1).unwrap();r.store(0xa02,MemWidth::Word,1).unwrap();
 step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.selectors[1],c.eip),(1,0xf1));step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.regs[1],c.eip),(1,0xf2));
 // Same logical offset as old image is no longer sufficient: actual physical
 // byte must still lie inside the original window, even if RAM contains code.
 assert!(matches!(i.decode(&r,0x100),Err(guest::GuestError::Decode(_))));
}
#[test]
fn retf_prefixes_truncation_and_new_effect_validation(){
 for w in [16,32]{assert!(compile(&instantiate(description("retf").unwrap(),w),w).is_ok());}
 for prefix in [vec![],vec![0x67],vec![0x2e],vec![0x64,0x67]]{let(mut i,mut r,mut c)=setup(16,false,None);let mut code=prefix;code.extend([0xca,7,0]);r.write_bytes(0x100,&code).unwrap();i.end=0x100+code.len()as u32;frame(&mut r,16,0x345,1);step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.eip,c.regs[4]),(0x345,0xabcd080b));}
 for bytes in [vec![0xca],vec![0xca,7],vec![0xf0,0xcb],vec![0xf3,0xcb]]{let(i,r)=guest::Image::load(&bytes).unwrap();assert!(i.decode(&r,0x100).is_err());}
 for text in ["const s 32 0\nconst t 32 0\nfar_branch s t\npreserve_flags all\nretire","const s 16 0\nconst t 32 0\nfar_branch s t\nwrite_segment 3 s\npreserve_flags all\nretire","const a 32 0\ncheck_segment 2 a 4\npreserve_flags all\nretire","const a 64 0\ncheck_segment 6 a 4\npreserve_flags all\nretire"]{assert!(compile(text,16).is_err());}
}
#[test]
fn far_return_dos_usage_fixture_reaches_original_exit_after_cs_change(){
 // PUSH 1010; PUSH 0009; RETF -> physical10109 (MOV AX,4C00;INT21).
 let bytes=[0x68,0x10,0x10,0x68,9,0,0xcb,0x90,0x90,0xb8,0,0x4c,0xcd,0x21];let result=dos::run(dos::load(&bytes,"far.com").unwrap(),30).unwrap();assert_eq!(result.stop,dos::Stop::Exit(0));assert_eq!(result.loaded.selectors[1],0x1010);assert_eq!(result.loaded.cpu.regs[4],0xfffe);
}
