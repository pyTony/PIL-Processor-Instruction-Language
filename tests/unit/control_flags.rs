//! Independent state checks for real-mode flags, STI delay and counter branches.
use super::*;
use address::{SegmentAccess,SegmentState};
fn setup(bytes:&[u8],big:bool)->(guest::Image,Ram,Cpu){let(mut i,_)=guest::Image::load(bytes).unwrap();i.segments[2].access=SegmentAccess::Data{writable:true,expand_down:false,big};let mut r=Ram::new(0x21000);r.write_bytes(0x100,bytes).unwrap();(i,r,Cpu{regs:[0x12345678,0,0,0,if big{0x800}else{0xabcd0800},0,0x800,0xa00],eflags:0xad7,undefined_flags:u32::MAX,eip:0x100})}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{let d=i.decode(r,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);if result.is_ok(){i.set_segment_state(m);}result}else{panic!()}}
fn op(w:u8,code:u8)->Vec<u8>{if w==32{vec![0x66,code]}else{vec![code]}}
#[test]
fn push_flags_has_386_fixed_bits_and_independent_stack_size(){
 for w in [16,32]{for big in [false,true]{for flags in [0,0x7fd7,0xfffdffff,0x10000]{
  let bytes=op(w,0x9c);let(mut i,mut r,mut c)=setup(&bytes,big);c.eflags=flags;let old=c.clone();let am=if big{u32::MAX}else{65535};let off=c.regs[4].wrapping_sub((w/8)as u32)&am;
  step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=bytes.len()as u32;want.regs[4]=(want.regs[4]&!am)|off;assert_eq!(c,want);assert_eq!(r.load(0x200+off,MemWidth::from_bits(w).unwrap()).unwrap(),(flags&0x7fd5)|2);
 }}}
}
#[test]
fn pop_flags_restores_only_writable_386_bits_and_metadata(){
 for w in [16,32]{for big in [false,true]{for packed in [0,0x7fd7,u32::MAX,0x30000]{
  let bytes=op(w,0x9d);let(mut i,mut r,mut c)=setup(&bytes,big);c.eflags=0x81010002;r.store(0xa00,MemWidth::from_bits(w).unwrap(),packed).unwrap();let old=c.clone();let oldram=r.clone();
  step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=bytes.len()as u32;want.regs[4]+=(w/8)as u32;want.eflags=(want.eflags&!0x7fd5)|(packed&0x7fd5);want.undefined_flags&=!0x7fd5;assert_eq!(c,want);assert_eq!(r,oldram);assert_eq!(i.sti_shadow,0);
 }}}
}
#[test]
fn stack_flag_faults_preserve_cpu_ram_and_both_shadows(){
 for w in [16,32]{for pop in [false,true]{let(mut i,mut r,mut c)=setup(&op(w,if pop{0x9d}else{0x9c}),false);i.ss_shadow=1;i.sti_shadow=1;if pop{c.regs[4]=0xffff;}else{r.protect(0x9fc,4).unwrap();}let old=c.clone();let oldr=r.clone();let oldm=i.segment_state();assert!(step(&mut i,&mut r,&mut c).is_err());assert_eq!(c,old);assert_eq!(r,oldr);assert_eq!(i.segment_state(),oldm);}}
}
#[test]
fn real_mode_guards_reject_protected_and_vm_context_without_effects(){
 for code in [0xfa,0xfb,0x9c,0x9d]{for vm in [false,true]{let(mut i,mut r,mut c)=setup(&[code],false);if vm{c.eflags|=0x20000;}else{i.real_mode=false;}let old=c.clone();let oldr=r.clone();let oldm=i.segment_state();assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::InvalidContext));assert_eq!(c,old);assert_eq!(r,oldr);assert_eq!(i.segment_state(),oldm);}}
}
#[test]
fn counter_branch_uses_address_size_without_changing_count_or_flags(){
 for wide in [false,true]{for a32 in [false,true]{for count in [0,1,0xabcd0000]{for disp in [1u8,0xfe]{let mut bytes=Vec::new();if wide{bytes.push(0x66);}if a32{bytes.push(0x67);}bytes.extend([0xe3,disp]);let(mut i,mut r,mut c)=setup(&bytes,false);c.regs[1]=count;let mut want=c.clone();want.eip+=bytes.len()as u32;if if a32{count==0}else{count&65535==0}{want.eip=want.eip.wrapping_add(disp as i8 as i32 as u32)&if wide{u32::MAX}else{65535};}let oldr=r.clone();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c,want);assert_eq!(r,oldr);}}}}
}
#[test]
fn counter_branch_target_wrap_and_taken_only_limit_check(){
 for w in [16,32]{let p=compile(&instantiate(description("jcxz").unwrap(),w),w).unwrap();let mut c=Cpu::default();c.eip=0xfffe;let v=View{register:0,low:0,width:w};let flow=FlowContext{next_eip:0x10000,displacement:1,displacement_bits:8,code_limit:0x1ffff};execute_flow(&p,&mut c,v,v,flow).unwrap();assert_eq!(c.eip,if w==16{1}else{0x10001});}
 for count in [0,1]{let(mut i,mut r,mut c)=setup(&[0xe3,0x7f],false);i.segments[1].limit=0x110;c.regs[1]=count;let old=c.clone();if count==0{assert!(matches!(step(&mut i,&mut r,&mut c),Err(ExecError::GeneralProtection{..})));assert_eq!(c,old);}else{step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.eip,0x102);}}
 let(i,r,_)=setup(&[0xe3],false);assert!(i.decode(&r,0x100).is_err());
}
#[test]
fn sti_delay_does_not_extend_on_second_sti_and_survives_rep_fault(){
 let(mut i,mut r,mut c)=setup(&[0xfb,0xfb,0xfa],false);c.eflags&=!0x200;let flags=c.eflags;step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.eflags,flags|0x200);assert_eq!(c.undefined_flags,u32::MAX&!0x200);assert_eq!(i.sti_shadow,1);step(&mut i,&mut r,&mut c).unwrap();assert_eq!(i.sti_shadow,0);step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.eflags,flags);
 let(mut i,mut r,mut c)=setup(&[0xfb,0xf3,0xaa],false);c.eflags&=!0x200;c.regs[1]=2;r.protect(0xa01,1).unwrap();step(&mut i,&mut r,&mut c).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.sti_shadow,c.eip,c.regs[1]),(1,0x101,1));let old=c.clone();assert!(step(&mut i,&mut r,&mut c).is_err());assert_eq!(c,old);assert_eq!(i.sti_shadow,1);let mut repaired=Ram::new(r.bytes().len());repaired.write_bytes(0,r.bytes()).unwrap();r=repaired;step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.sti_shadow,c.eip,c.regs[1]),(0,0x103,0));
}
#[test]
fn new_machine_effects_validate_types_and_trace_includes_sti_shadow(){
 for text in ["const x 16 1\ninterrupt_shadow_if x\npreserve_flags all\nretire","const x 1 1\ninterrupt_shadow_if x\ninterrupt_shadow_if x\npreserve_flags all\nretire","const x 1 1\ninterrupt_shadow_if x\nretire_if x\npreserve_flags all\nretire"]{assert!(compile(text,16).is_err());}
 let(i,_,c)=setup(&[0x90],false);let m=i.segment_state();let mut n=m;n.sti_shadow=1;let record=|m:&SegmentState|{let mut out=Vec::new();trace::record_machine(&mut out,0,[0;5],&c.regs,c.eflags,c.undefined_flags,c.eip,&[],&[],&[],m);out};assert!(native::verify_traces(&record(&m),&record(&n)).is_err());
}
#[test]
fn dos_bridge_retires_sti_shadow_but_partial_rep_keeps_it(){
 let bytes=[0xb4,9,0xba,10,1,0xfa,0xfb,0xcd,0x21,0xf4,b'$',0];let p=dos::load(&bytes,"shadow.com").unwrap();let result=dos::run(p,5).unwrap();assert_eq!(result.loaded.image.sti_shadow,0);assert_eq!(result.dos_calls,1);
 let bytes=[0xfa,0xfb,0xf3,0xaa];let mut p=dos::load(&bytes,"shadowrep.com").unwrap();p.cpu.regs[1]=2;let result=dos::run(p,3).unwrap();assert_eq!(result.stop,dos::Stop::Budget);assert_eq!(result.loaded.image.sti_shadow,1);assert_eq!(result.loaded.cpu.regs[1],1);
}
