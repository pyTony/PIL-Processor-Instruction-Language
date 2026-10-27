use super::*;
use address::SegmentAccess;
fn fixture(bytes:&[u8],big:bool)->(guest::Image,Ram,Cpu){let(mut i,_)=guest::Image::load(bytes).unwrap();i.segments[2].access=SegmentAccess::Data{writable:true,expand_down:false,big};let mut r=Ram::new(0x40000);r.write_bytes(0x100,bytes).unwrap();(i,r,Cpu{regs:[0x12345678,0,0,0,0xabcd0400,if big{0x800}else{0xdead0800},0,0],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100})}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{if let guest::Operation::Pil{program,dst,src,flow,address}=i.decode(r,c.eip).unwrap().operation{let mut m=i.segment_state();let res=execute_machine(&program,c,dst,src,flow,r,address,&mut m);if res.is_ok(){i.set_segment_state(m);}res}else{panic!()}}
#[test]
fn leave_independent_operand_stack_sizes_and_readonly_source(){
 for w in [16,32]{for big in [false,true]{let mut code=vec![0x67,0x64];if w==32{code.push(0x66);}code.push(0xc9);let(mut i,mut r,mut c)=fixture(&code,big);r.store(0xa00,MemWidth::Dword,0x56781234).unwrap();r.protect(0xa00,4).unwrap();let old=c.clone();let oldr=r.clone();step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=code.len()as u32;want.regs[4]=if big{0x800+(w/8)as u32}else{0xabcd0800+(w/8)as u32};want.regs[5]=if w==32{0x56781234}else{(want.regs[5]&0xffff0000)|0x1234};assert_eq!(c,want);assert_eq!(r,oldr);}}
}
#[test]
fn leave_big_stack_uses_full_ebp_and_stack_increment_wraps(){
 let(mut i,mut r,mut c)=fixture(&[0xc9],true);i.segments[2].limit=0x1ffff;c.regs[5]=0x10800;r.store(0x10a00,MemWidth::Word,0x1234).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.regs[4],c.regs[5]),(0x10802,0x11234));
 let(mut i,mut r,mut c)=fixture(&[0xc9],false);c.regs[5]=0xaaaafffe;r.store(0x101fe,MemWidth::Word,0x4321).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.regs[4],c.regs[5]),(0xabcd0000,0xaaaa4321));
}
#[test]
fn leave_faults_preserve_all_pending_effects_and_shadows(){
 for ram_fault in [false,true]{let(mut i,mut r,mut c)=fixture(&[0x66,0xc9],false);i.sti_shadow=1;i.ss_shadow=1;if ram_fault{r=Ram::new(0xa01);r.write_bytes(0x100,&[0x66,0xc9]).unwrap();}else{c.regs[5]=0xffff;}let old=c.clone();let oldr=r.clone();let oldm=i.segment_state();assert!(step(&mut i,&mut r,&mut c).is_err());assert_eq!(c,old);assert_eq!(r,oldr);assert_eq!(i.segment_state(),oldm);}
}
#[test]
fn leave_after_real_mode_ss_load_rebinds_stack_size_and_base(){
 let(mut i,mut r,mut c)=fixture(&[0x8e,0xd0,0xc9],true);c.regs[0]=0x30;c.regs[5]=0xdead0800;r.store(0xb00,MemWidth::Word,0x1234).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(i.ss_shadow,1);step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.regs[4],c.regs[5]),(0xabcd0802,0xdead1234));assert_eq!(i.ss_shadow,0);
}
