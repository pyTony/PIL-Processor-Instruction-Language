//! Near calls/returns and string elements: independent values, progress and faults.
use super::*;
use address::*;
fn setup(bytes:&[u8],big:bool)->(guest::Image,Ram,Cpu){let (mut image,_)=guest::Image::load(bytes).unwrap();image.segments[2].access=SegmentAccess::Data{writable:true,expand_down:false,big};let mut ram=Ram::new(0x21000);ram.write_bytes(0x100,bytes).unwrap();let c=Cpu{regs:[0x12345678,0,0,0,if big{0x800}else{0xabcd0800},0,0x800,0xa00],eflags:0xad7,undefined_flags:u32::MAX,eip:0x100};(image,ram,c)}
fn step(image:&mut guest::Image,ram:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{let d=image.decode(ram,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=image.segment_state();let result=execute_machine(&program,c,dst,src,flow,ram,address,&mut m);if result.is_ok(){image.set_segment_state(m);}result}else{panic!()}}
#[test]
fn near_call_and_ret_sizes_flags_and_return_offsets(){
 for w in [16,32]{for big in [false,true]{
  let mut bytes=Vec::new();if w==32{bytes.push(0x66);}bytes.push(0xe8);bytes.extend(vec![0;(w/8)as usize]);if w==32{bytes.push(0x66);}bytes.push(0xc3);
  let (mut image,mut ram,mut c)=setup(&bytes,big);let old=c.clone();let return_ip=0x100+if w==16{3}else{6};let mask=if big{u32::MAX}else{65535};let off=c.regs[4].wrapping_sub((w/8)as u32)&mask;
  step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(c.eip,return_ip);assert_eq!(ram.load(0x200+off,MemWidth::from_bits(w).unwrap()).unwrap(),return_ip);assert_eq!(c.regs[4],(old.regs[4]&!mask)|off);
  step(&mut image,&mut ram,&mut c).unwrap();let mut want=old;want.eip=return_ip;assert_eq!(c,want);
 }}
}
#[test]
fn indirect_call_reads_esp_or_memory_before_stack_write_and_ret_cleanup_is_bytes(){
 let (mut image,mut ram,mut c)=setup(&[0xff,0xd4],false);c.regs[4]=0x800;step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(c.eip,0x800);assert_eq!(ram.load(0x9fe,MemWidth::Word).unwrap(),0x102);
 let (mut image,mut ram,mut c)=setup(&[0xff,0x16,0xfe,8],false);ram.store(0x9fe,MemWidth::Word,0x1234).unwrap();step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(c.eip,0x1234);assert_eq!(ram.load(0x9fe,MemWidth::Word).unwrap(),0x104);
 for w in [16,32]{for big in [false,true]{let mut bytes=Vec::new();if w==32{bytes.push(0x66);}bytes.extend([0xc2,7,0]);let (mut image,mut ram,mut c)=setup(&bytes,big);let old=c.regs[4];ram.store(0xa00,MemWidth::from_bits(w).unwrap(),0x1234).unwrap();step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(c.eip,0x1234);assert_eq!(c.regs[4],old+(w/8)as u32+7);}}
}
#[test]
fn near_control_faults_commit_no_stack_register_or_memory_effect(){
 for bytes in [vec![0xe8,0,1],vec![0xc3],vec![0xc2,8,0]]{
  let (mut image,mut ram,mut c)=setup(&bytes,false);image.segments[1].limit=0x200;ram.store(0xa00,MemWidth::Word,0x300).unwrap();let old=c.clone();let oldram=ram.clone();assert!(matches!(step(&mut image,&mut ram,&mut c),Err(ExecError::GeneralProtection{..})));assert_eq!(c,old);assert_eq!(ram,oldram);
 }
 let (mut image,mut ram,mut c)=setup(&[0xe8,0,0],false);ram.protect(0x9fe,2).unwrap();let old=c.clone();let oldram=ram.clone();assert!(step(&mut image,&mut ram,&mut c).is_err());assert_eq!(c,old);assert_eq!(ram,oldram);
}
#[test]
fn string_widths_address_sizes_df_and_untouched_state(){
 for w in [8,16,32]{for a in [16,32]{for backwards in [false,true]{for op in [0xa4u8,0xaa,0xac]{
  let mut bytes=Vec::new();if w==32{bytes.push(0x66);}if a==32{bytes.push(0x67);}bytes.push(op|u8::from(w!=8));let (mut image,mut ram,mut c)=setup(&bytes,false);c.regs[6]=if a==16{0xabcd0800}else{0x800};c.regs[7]=if a==16{0xef010a00}else{0xa00};if backwards{c.eflags|=0x400;}ram.store(0x900,MemWidth::Dword,0xdeadbeef).unwrap();let old=c.clone();let oldram=ram.clone();let n=(w/8)as u32;let delta=if backwards{0u32.wrapping_sub(n)}else{n};let am=if a==16{65535}else{u32::MAX};let wm=mask(w)as u32;
  step(&mut image,&mut ram,&mut c).unwrap();let mut want=old.clone();want.eip+=bytes.len()as u32;if op!=0xaa{want.regs[6]=(old.regs[6]&!am)|(old.regs[6].wrapping_add(delta)&am);}if op!=0xac{want.regs[7]=(old.regs[7]&!am)|(old.regs[7].wrapping_add(delta)&am);}if op==0xac{want.regs[0]=(old.regs[0]&!wm)|(0xdeadbeef&wm);}assert_eq!(c,want);
  if op==0xac{assert_eq!(ram,oldram);}else{assert_eq!(ram.load(0xa00,MemWidth::from_bits(w).unwrap()).unwrap(),if op==0xa4{0xdeadbeef&wm}else{old.regs[0]&wm});}
 }}}}
}
#[test]
fn rep_zero_count_avoids_access_and_prefix_rules_keep_es_destination(){
 for opcode in [0xa4,0xaa,0xac]{let (mut image,mut ram,mut c)=setup(&[0xf3,opcode],false);image.segments[0].access=SegmentAccess::Unusable;image.segments[3].access=SegmentAccess::Unusable;c.regs[1]=0xabcd0000;let mut want=c.clone();want.eip+=2;let oldram=ram.clone();step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(c,want);assert_eq!(ram,oldram);}
 let (mut image,mut ram,mut c)=setup(&[0x64,0xf3,0xa4],false);c.regs[1]=1;ram.store(0xb00,MemWidth::Byte,0x9a).unwrap();step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(ram.load(0xa00,MemWidth::Byte).unwrap(),0x9a);assert_eq!(c.eip,0x103);
}
#[test]
fn rep_retains_prior_progress_on_fault_and_resumes_without_replaying(){
 let (mut image,mut ram,mut c)=setup(&[0xf3,0xa4],false);ram.write_bytes(0x900,&[10,20,30]).unwrap();c.regs[1]=3;image.ss_shadow=1;ram.protect(0xa01,1).unwrap();
 step(&mut image,&mut ram,&mut c).unwrap();assert_eq!((c.eip,c.regs[1],c.regs[6],c.regs[7]),(0x100,2,0x801,0xa01));assert_eq!(image.ss_shadow,1);
 let old=c.clone();let oldram=ram.clone();assert!(step(&mut image,&mut ram,&mut c).is_err());assert_eq!(c,old);assert_eq!(ram,oldram);
 let mut repaired=Ram::new(ram.bytes().len());repaired.write_bytes(0,ram.bytes()).unwrap();ram=repaired;ram.store(0x900,MemWidth::Byte,99).unwrap();
 step(&mut image,&mut ram,&mut c).unwrap();step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(&ram.bytes()[0xa00..0xa03],&[10,20,30]);assert_eq!((c.eip,c.regs[1]),(0x102,0));assert_eq!(image.ss_shadow,0);
}
#[test]
fn rep_overlap_matches_serial_element_order_and_budget_is_bounded(){
 let (mut image,mut ram,mut c)=setup(&[0xf3,0xa4],false);image.segments[0]=image.segments[3];c.regs[7]=0x801;c.regs[1]=3;ram.write_bytes(0x900,&[1,2,3,4]).unwrap();for _ in 0..3{step(&mut image,&mut ram,&mut c).unwrap();}assert_eq!(&ram.bytes()[0x900..0x904],&[1,1,1,1]);
 let bytes=[0xb9,0xff,0xff,0xf3,0xaa];let p=dos::load(&bytes,"repeat.com").unwrap();let result=dos::run(p,4).unwrap();assert_eq!(result.stop,dos::Stop::Budget);assert_eq!(result.completed,1);assert_eq!(result.pil_steps,4);assert_eq!(result.loaded.cpu.regs[1],65532);assert_eq!(result.loaded.cpu.eip,0x103);
}
#[test]
fn new_effects_validate_context_and_early_retirement_placement(){
 for text in ["const x 32 0\nload_segment v 6 x 8\npreserve_flags all\nretire","const x 1 1\nconst v 16 0\nwrite_gpr 0 v\nretire_if x\npreserve_flags all\nretire"]{assert!(compile(text,16).is_err());}
 for bytes in [vec![0xf3,0x90],vec![0xf2,0xa4],vec![0xf3,0xf3,0xa4],vec![0xf0,0xe8,0,0]]{let (image,ram,_)=setup(&bytes,false);assert!(image.decode(&ram,0x100).is_err());}
}
