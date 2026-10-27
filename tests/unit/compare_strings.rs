use super::*;
use address::SegmentAccess;
fn setup(code:&[u8])->(guest::Image,Ram,Cpu){let(i,_)=guest::Image::load(code).unwrap();let mut r=Ram::new(0x21000);r.write_bytes(0x100,code).unwrap();(i,r,Cpu{regs:[0x12345678,3,0,0,0x800,0,0x800,0xa00],eflags:0xad7,undefined_flags:u32::MAX,eip:0x100})}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{if let guest::Operation::Pil{program,dst,src,flow,address}=i.decode(r,c.eip).unwrap().operation{let mut m=i.segment_state();let res=execute_machine(&program,c,dst,src,flow,r,address,&mut m);i.set_segment_state(m);res}else{panic!()}}
fn expected_flags(a:u32,b:u32,w:u8)->u32{let mask=mask(w)as u32;let a=a&mask;let b=b&mask;let z=a.wrapping_sub(b)&mask;let sign=1u32<<(w-1);u32::from(a<b)|(u32::from((z as u8).count_ones()%2==0)<<2)|((a^b^z)&16)|(u32::from(z==0)<<6)|(u32::from(z&sign!=0)<<7)|(u32::from((a^b)&(a^z)&sign!=0)<<11)}
fn code(op:u8,w:u8,a:u8,prefix:Option<u8>)->Vec<u8>{let mut c=Vec::new();if w==32{c.push(0x66);}if a==32{c.push(0x67);}if let Some(p)=prefix{c.push(p);}c.push(op|u8::from(w!=8));c}
#[test]
fn compare_strings_all_sizes_df_flags_and_register_preservation(){
 for w in [8,16,32]{for a in [16,32]{for back in [false,true]{for op in [0xa6,0xae]{for(left,right)in [(0,1),(0x7fffffff,0xffffffff),(0x80,0x80),(0x10,1)]{
 let bytes=code(op,w,a,None);let(mut i,mut r,mut c)=setup(&bytes);if a==16{c.regs[6]|=0xabcd0000;c.regs[7]|=0xdead0000;}if back{c.eflags|=0x400;}c.regs[0]=left;r.store(0x900,MemWidth::from_bits(w).unwrap(),left).unwrap();r.store(0xa00,MemWidth::from_bits(w).unwrap(),right).unwrap();let old=c.clone();let oldr=r.clone();step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=bytes.len()as u32;want.eflags=(want.eflags&!ARITH_FLAGS)|expected_flags(left,right,w);want.undefined_flags&=!ARITH_FLAGS;let m=if a==16{65535}else{u32::MAX};let delta=if back{0u32.wrapping_sub((w/8)as u32)}else{(w/8)as u32};for reg in if op==0xa6{vec![6,7]}else{vec![7]}{want.regs[reg]=(want.regs[reg]&!m)|(want.regs[reg].wrapping_add(delta)&m);}assert_eq!(c,want);assert_eq!(r,oldr);
 }}}}}
}
#[test]
fn repeats_test_new_zf_ignore_initial_zf_and_stop_on_match_or_mismatch(){
 for prefix in [0xf2,0xf3]{for op in [0xa6,0xae]{for initial_zf in [false,true]{let bytes=code(op,8,16,Some(prefix));let(mut i,mut r,mut c)=setup(&bytes);c.regs[0]=5;c.eflags=(c.eflags&!64)|(u32::from(initial_zf)<<6);r.write_bytes(0x900,&[5,5,5]).unwrap();r.write_bytes(0xa00,if prefix==0xf3{&[5,4,5]}else{&[4,5,4]}).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.eip,c.regs[1]),(0x100,2));assert!(i.flag_checkpoint.is_some());step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.eip,c.regs[1],c.regs[7]),(0x102,1,0xa02));assert_eq!(c.eflags&64,if prefix==0xf3{0}else{64});assert_eq!(i.flag_checkpoint,None);}}}
}
#[test]
fn count_zero_avoids_all_data_access_and_count_exhaustion_keeps_last_flags(){
 for w in [8,16,32]{for a in [16,32]{for prefix in [0xf2,0xf3]{let bytes=code(0xa6,w,a,Some(prefix));let(mut i,mut r,mut c)=setup(&bytes);i.segments[0].access=SegmentAccess::Unusable;i.segments[3].access=SegmentAccess::Unusable;c.regs[1]=if a==16{0xabcd0000}else{0};let mut want=c.clone();want.eip+=bytes.len()as u32;step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c,want);assert_eq!(i.flag_checkpoint,None);}}}
 let(mut i,mut r,mut c)=setup(&[0xf3,0xae]);c.regs[0]=0;r.write_bytes(0xa00,&[0,0,0]).unwrap();for _ in 0..3{step(&mut i,&mut r,&mut c).unwrap();}assert_eq!((c.regs[1],c.regs[7],c.eip),(0,0xa03,0x102));assert_eq!(c.eflags&ARITH_FLAGS,64|4);
}
#[test]
fn source_override_and_fixed_es_are_independent_and_second_read_fault_is_atomic(){
 let(mut i,mut r,mut c)=setup(&[0x64,0xa6]);r.store(0xb00,MemWidth::Byte,7).unwrap();r.store(0xa00,MemWidth::Byte,7).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.eflags&64,64);
 let(mut i,mut r,mut c)=setup(&[0x64,0xae]);c.regs[0]=7;r.store(0xa00,MemWidth::Byte,7).unwrap();i.segments[4].access=SegmentAccess::Unusable;step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.eflags&64,64);
 let(mut i,mut r,mut c)=setup(&[0xa6]);i.segments[0].access=SegmentAccess::Unusable;let old=c.clone();let oldr=r.clone();assert!(step(&mut i,&mut r,&mut c).is_err());assert_eq!(c,old);assert_eq!(r,oldr);
}
#[test]
fn compare_repeat_fault_restores_entry_flags_keeps_progress_and_retries(){
 for prefix in [0xf2,0xf3]{for op in [0xa6,0xae]{let bytes=code(op,8,16,Some(prefix));let(mut i,mut r,mut c)=setup(&bytes);c.regs[0]=if prefix==0xf3{0}else{1};r.write_bytes(0x900,&[c.regs[0]as u8;3]).unwrap();i.segments[0].limit=0xa00;i.ss_shadow=1;i.sti_shadow=1;let flags=(c.eflags,c.undefined_flags);step(&mut i,&mut r,&mut c).unwrap();assert_ne!((c.eflags,c.undefined_flags),flags);let old_regs=c.regs;assert!(step(&mut i,&mut r,&mut c).is_err());assert_eq!(c.regs,old_regs);assert_eq!((c.eflags,c.undefined_flags),flags);assert_eq!(c.eip,0x100);assert_eq!((i.ss_shadow,i.sti_shadow),(1,1));assert_eq!(i.flag_checkpoint,None);i.segments[0].limit=65535;for _ in 0..2{step(&mut i,&mut r,&mut c).unwrap();}assert_eq!((c.eip,c.regs[1]),(0x102,0));assert_eq!((i.ss_shadow,i.sti_shadow),(0,0));}}
}
#[test]
fn memory_setcc_all_conditions_sizes_preserves_flags_and_registers(){
 for n in 0..16u8{for addr32 in [false,true]{for f in [0,0xed7]{let mut bytes=Vec::new();if addr32{bytes.push(0x67);}bytes.extend([0x0f,0x90+n,if addr32{5}else{6},0,8]);if addr32{bytes.extend([0,0]);}let(mut i,mut r,mut c)=setup(&bytes);c.eflags=f;let old=c.clone();let val=match n{0=>f&0x800!=0,1=>f&0x800==0,2=>f&1!=0,3=>f&1==0,4=>f&64!=0,5=>f&64==0,6=>f&65!=0,7=>f&65==0,8=>f&128!=0,9=>f&128==0,10=>f&4!=0,11=>f&4==0,12=>(f>>7^f>>11)&1!=0,13=>(f>>7^f>>11)&1==0,14=>f&64!=0||(f>>7^f>>11)&1!=0,_=>f&64==0&&(f>>7^f>>11)&1==0};step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=bytes.len()as u32;assert_eq!(c,want);assert_eq!(r.load(0x900,MemWidth::Byte).unwrap(),u32::from(val));}}}
}
#[test]
fn memory_setcc_bp_ss_override_readonly_fault_and_no_false_store_elision(){
 for override_ds in [false,true]{let mut bytes=Vec::new();if override_ds{bytes.push(0x3e);}bytes.extend([0x0f,0x95,0x46,0]);let(mut i,mut r,mut c)=setup(&bytes);c.regs[5]=0x800;c.eflags|=64;let at=if override_ds{0x900}else{0xa00};r.store(at,MemWidth::Byte,0xaa).unwrap();let mut i2=i.clone();let mut r2=r.clone();let mut c2=c.clone();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(r.load(at,MemWidth::Byte).unwrap(),0);r2.protect(at,1).unwrap();let old=c2.clone();assert!(step(&mut i2,&mut r2,&mut c2).is_err());assert_eq!(c2,old);}
}
#[test]
fn compare_checkpoints_are_traced_prefix_rules_and_budgets_are_bounded(){
 for text in ["checkpoint_flags\ncheckpoint_flags\npreserve_flags all\nretire","const x 1 1\ncheckpoint_flags\npreserve_flags all\nretire"]{assert!(compile(text,8).is_err());}
 for bytes in [vec![0xf2,0xa4],vec![0xf2,0xf3,0xae],vec![0xf3,0xf3,0xae],vec![0xf3,0x90]]{let(i,r,_)=setup(&bytes);assert!(i.decode(&r,0x100).is_err());}
 let(i,_,c)=setup(&[0x90]);let m=i.segment_state();let mut n=m;n.flag_checkpoint=Some((c.eip,1,2));let record=|m:&address::SegmentState|{let mut b=Vec::new();trace::record_machine(&mut b,0,[0;5],&c.regs,c.eflags,c.undefined_flags,c.eip,&[],&[],&[],m);b};assert!(native::verify_traces(&record(&m),&record(&n)).is_err());
 let mut p=dos::load(&[0xf3,0xae],"compare.com").unwrap();p.cpu.regs[1]=3;p.cpu.regs[7]=0x200;let result=dos::run(p,1).unwrap();assert_eq!(result.stop,dos::Stop::Budget);assert_eq!((result.completed,result.pil_steps),(0,1));assert!(result.loaded.image.flag_checkpoint.is_some());
}
