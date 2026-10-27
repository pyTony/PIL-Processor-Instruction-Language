//! Original far CALL frame, pointer and transactional-fault expectations.
use super::*;
use address::{SegmentAccess,SegmentCache};
fn direct(w:u8,selector:u16,target:u32)->Vec<u8>{
 let mut b=vec![];if w==32{b.push(0x66);}b.push(0x9a);b.extend(&target.to_le_bytes()[..(w/8)as usize]);b.extend(selector.to_le_bytes());b
}
fn setup(code:&[u8],big:bool)->(guest::Image,Ram,Cpu){
 let(mut i,_)=guest::Image::load(code).unwrap();i.segments[2].access=SegmentAccess::Data{writable:true,expand_down:false,big};
 let mut r=Ram::new(0x22000);r.write_bytes(0x100,code).unwrap();
 let c=Cpu{regs:[0x12345678,0xabcdef01,2,0x400,if big{0x800}else{0xabcd0800},0x400,8,0],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};(i,r,c)
}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
 let d=i.decode(r,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{
 let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);i.set_segment_state(m);result}else{panic!()}
}
fn pointer(r:&mut Ram,at:u32,w:u8,target:u32,selector:u16){
 r.store(at,MemWidth::from_bits(w).unwrap(),target).unwrap();r.store(at+(w/8)as u32,MemWidth::Word,u32::from(selector)).unwrap();
}
#[test]
fn far_call_immediate_frame_widths_and_new_cs_preserve_unrelated_state(){
 for w in [16,32]{for big in [false,true]{for selector in [0u16,1,0x1234,0xffff]{
  let b=direct(w,selector,0x345);let(mut i,mut r,mut c)=setup(&b,big);
  i.selectors[1]=0x1234; // deliberately supplied old selector; fetch cache stays at zero
  i.ss_shadow=1;i.sti_shadow=1;let oldi=i.clone();let mut want=c.clone();let mut wantr=r.clone();
  let n=(w/8)as u32;want.regs[4]-=2*n;want.eip=0x345;
  wantr.store(0xa00-n,MemWidth::from_bits(w).unwrap(),0x1234).unwrap();
  wantr.store(0xa00-2*n,MemWidth::from_bits(w).unwrap(),0x100+b.len()as u32).unwrap();
  step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c,want);assert_eq!(r,wantr);
  let mut wanti=oldi;wanti.selectors[1]=selector;wanti.segments[1]=SegmentCache::real_mode(selector);wanti.ss_shadow=0;wanti.sti_shadow=0;assert_eq!(i,wanti);
 }}}
}
#[test]
fn far_call_memory_pointer_sizes_segments_and_old_esp_are_independent(){
 for w in [16,32]{for big in [false,true]{for kind in 0..5{
  let mut b=vec![];if w==32{b.push(0x66);}
  let at=match kind{0=>{b.extend([0xff,0x1f]);0x500},1=>{b.extend([0xff,0x5e,0]);0x600},2=>{b.extend([0x64,0xff,0x1f]);0x700},3=>{b.extend([0x67,0xff,0x5c,0x73,0xfc]);0x50c},_=>{b.extend([0x67,0xff,0x1c,0x24]);0xa00}};
  let(mut i,mut r,mut c)=setup(&b,big);if kind==4{c.regs[4]=0x800;}
  pointer(&mut r,at,w,0x345,0x1234);r.protect(at,(w/8+2)as usize).unwrap();
  let old=c.clone();step(&mut i,&mut r,&mut c).unwrap();let n=(w/8)as u32;
  let mut want=old;want.regs[4]-=2*n;want.eip=0x345;assert_eq!(c,want);assert_eq!(i.selectors[1],0x1234);
  assert_eq!(r.load(0xa00-2*n,MemWidth::from_bits(w).unwrap()).unwrap(),0x100+b.len()as u32);
 }}}
}
#[test]
fn far_call_reads_complete_overlapping_pointer_before_staging_frame(){
 for w in [16,32]{
  let mut b=vec![];if w==32{b.push(0x66);}b.extend([0xff,0x1f]);
  let(mut i,mut r,mut c)=setup(&b,false);let n=(w/8)as u32;
  let at=0xa00-2*n;c.regs[3]=at-0x100;pointer(&mut r,at,w,0x345,0x1234);
  step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.selectors[1],c.eip),(0x1234,0x345));
  assert_eq!(r.load(at,MemWidth::from_bits(w).unwrap()).unwrap(),0x100+b.len()as u32);
 }
}
#[test]
fn far_call_faults_never_commit_either_slot_or_cs_ip_sp(){
 for w in [16,32]{for fault in 0..7{
  let b=if fault==4||fault==5{let mut b=vec![];if w==32{b.push(0x66);}b.extend([0xff,0x1f]);b}else{direct(w,1,if fault==6{0x10000}else{0x345})};
  if w==16&&fault==6{continue;}
  let(mut i,mut r,mut c)=setup(&b,false);pointer(&mut r,0x500,w,0x345,1);i.ss_shadow=1;i.sti_shadow=1;
  let n=(w/8)as u32;
  match fault{0=>r.protect(0xa00-n,n as usize).unwrap(),1=>r.protect(0xa00-2*n,n as usize).unwrap(),2=>i.segments[2].limit=0x800-n-1,3=>{c.regs[4]=n;},4=>i.segments[3].limit=0x400+n,5=>{let end=0x500+n as usize+1;let mut short=Ram::new(end);short.write_bytes(0,&r.bytes()[..end]).unwrap();r=short;},_=>{}}
  let old=c.clone();let oldi=i.clone();let oldr=r.clone();assert!(step(&mut i,&mut r,&mut c).is_err(),"w={w} fault={fault}");assert_eq!(c,old);assert_eq!(i,oldi);assert_eq!(r,oldr);
 }}
}
#[test]
fn far_call_requires_real_mode_and_valid_memory_encoding(){
 for vm in [false,true]{let(mut i,mut r,mut c)=setup(&[0xff,0x1f],false);i.real_mode=vm;if vm{c.eflags|=0x20000;}i.segments[3].access=SegmentAccess::Unusable;
  let old=c.clone();let oldi=i.clone();let oldr=r.clone();assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::InvalidContext));assert_eq!(c,old);assert_eq!(r,oldr);assert_eq!(i,oldi);
 }
 for b in [vec![0x9a],vec![0x9a,0,0,0],vec![0x66,0x9a,0,0,0,0,0],vec![0xff,0xd8],vec![0xff,0x1e,0],vec![0x67,0xff,0x1c],vec![0xf0,0x9a,0,0,0,0],vec![0xf3,0xff,0x1f]]{
 let(i,r,_)=setup(&b,false);assert!(i.decode(&r,0x100).is_err());}
 for w in [16,32]{assert!(compile(&instantiate(description("call_far").unwrap(),w),w).is_ok());}
}
#[test]
fn far_call_and_retf_round_trip_through_cs_alias_and_original_dos_exit(){
 for w in [16,32]{
  let mut b=direct(w,0x1011,0);b.extend([0xb8,0,0x4c,0xcd,0x21]);while b.len()<16{b.push(0x90);}
  b.push(0x41);if w==32{b.push(0x66);}b.push(0xcb);
  let result=dos::run(dos::load(&b,"far-call.com").unwrap(),30).unwrap();
  assert_eq!(result.stop,dos::Stop::Exit(0));assert_eq!(result.loaded.selectors[1],0x1000);assert_eq!(result.loaded.cpu.regs[4],0xfffe);assert_eq!(result.loaded.cpu.regs[1],1);
 }
}
#[test]
fn two_stores_commit_in_order_and_later_fault_cancels_both(){
 for segment in [false,true]{
  let store=if segment{"store_segment 3"}else{"store"};
  let text=format!("const a 32 0\nconst b 32 1\nconst first 16 48879\nconst last 16 4660\n{store} a first\n{store} b last\npreserve_flags all\nretire");
  let p=compile(&text,16).unwrap();let v=view_for_test();let mut c=Cpu::default();let mut r=Ram::new(1024);let mut machine=guest::Image::load(&[0x90]).unwrap().0.segment_state();
  // The monitor's DS base is 0x100.
  let base=if segment{0x100}else{0};let flow=flow::FlowContext{next_eip:1,displacement:0,displacement_bits:8,code_limit:65535};
  let mut expected=r.clone();expected.store(base,MemWidth::Word,0xbeef).unwrap();expected.store(base+1,MemWidth::Word,0x1234).unwrap();
  execute_machine(&p,&mut c,v,v,flow,&mut r,None,&mut machine).unwrap();assert_eq!(r,expected);
  r=Ram::new(1024);r.protect(base+2,1).unwrap();let oldr=r.clone();let old=c.clone();let oldm=machine.clone();assert!(execute_machine(&p,&mut c,v,v,flow,&mut r,None,&mut machine).is_err());assert_eq!(r,oldr);assert_eq!(c,old);assert_eq!(machine,oldm);
 }
 fn view_for_test()->View{View{register:0,low:0,width:16}}
}

#[test]
fn generated_far_frames_match_literal_expected_states_including_second_store_fault(){
 use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
 let dir=std::env::temp_dir().join(format!("pil-far-frame-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&dir).unwrap();
 let mut source=String::from("#![allow(dead_code,unused_mut,unused_variables,unused_assignments)]\n");let mut checks=String::new();
 for w in [16,32]{for stack in [16,32]{
  let name=format!("w{w}s{stack}");let text=description("call_far").unwrap().replace("$FAR_TARGET",&format!("const target_value {w} 837\nconst selector 16 1")).replace("$S",&stack.to_string());
  let p=compile(&instantiate(text,w),w).unwrap();source.push_str(&format!("mod {name} {{ {} }}\n",emit_rust(&p)));
  let n=w/8;let sp=if stack==16{0xabcd0800u32}else{0x800};let width=if w==16{"Word"}else{"Dword"};
  checks.push_str(&format!(r#"
  for fault in 0..3{{
   let mut ram={name}::Ram::new(4096);ram.write_bytes(0,&vec![0xa5;4096]).unwrap();
   if fault>0{{ram.protect(0xa00-{}u32,1).unwrap();}}
   let oldram=ram.clone();let mut regs=[0xdeadbeef;8];regs[4]={sp};let oldregs=regs;
   let mut flags=0xed7;let mut undefined=u32::MAX;let mut ip=0x100;
   let mut segments=[{name}::address::SegmentCache::real_mode(0);6];
   segments[2]={name}::address::SegmentCache::real_mode(0x20);
   segments[2].access={name}::address::SegmentAccess::Data{{writable:true,expand_down:false,big:{}}};
   let mut machine={name}::address::SegmentState{{msw:16,segments,selectors:[0,0x1234,0x20,0,0,0],real_mode:true,ss_shadow:1,sti_shadow:1,flag_checkpoint:None}};let oldmachine=machine.clone();
   let result={name}::execute_with_machine(&mut regs,&mut flags,&mut undefined,&mut ip,(0,0),(0,0),{name}::FlowContext{{next_eip:0x108,displacement:0,displacement_bits:8,code_limit:0xffff}},&mut ram,None,&mut machine);
   if fault==0{{
    result.unwrap();let mut want=oldregs;want[4]-={};assert_eq!(regs,want);assert_eq!(ip,0x345);
    assert_eq!(machine.selectors[1],1);assert_eq!(machine.segments[1],{name}::address::SegmentCache::real_mode(1));assert_eq!((machine.ss_shadow,machine.sti_shadow),(0,0));
    let mut want=oldram;want.store(0xa00-{n},{name}::MemWidth::{width},0x1234).unwrap();want.store(0xa00-{}, {name}::MemWidth::{width},0x108).unwrap();assert_eq!(ram,want);
   }}else{{assert!(result.is_err());assert_eq!(regs,oldregs);assert_eq!(ip,0x100);assert_eq!(ram,oldram);assert_eq!(machine,oldmachine);}}
   assert_eq!((flags,undefined),(0xed7,u32::MAX));
  }}
  "#,n,stack==32,2*n,2*n));
  // Exercise both the first (CS) and second (IP) staged store.
  let needle=format!("ram.protect(0xa00-{n}u32,1)");checks=checks.replace(&needle,&format!("ram.protect(0xa00-if fault==1{{{n}}}else{{{}}},1)",2*n));
 }}
 source.push_str(&format!("fn main(){{{checks}}}"));let file=dir.join("proof.rs");fs::write(&file,source).unwrap();let bin=dir.join(if cfg!(windows){"proof.exe"}else{"proof"});
 let out=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).args(["--edition=2021","-O"]).arg(file).arg("-o").arg(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));assert!(Command::new(bin).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
}

#[test]
fn far_call_bounded_frame_wrap_and_exact_pointer_end_are_explicit(){
 for w in [16,32]{
  let b=direct(w,1,0xffff);let(mut i,mut r,mut c)=setup(&b,false);c.regs[4]=0xabcd0000;
  step(&mut i,&mut r,&mut c).unwrap();let n=(w/8)as u32;assert_eq!(c.regs[4],0xabcd0000|(65536-2*n));assert_eq!(c.eip,0xffff);
  assert_eq!(r.load(0x10200-2*n,MemWidth::from_bits(w).unwrap()).unwrap(),0x100+b.len()as u32);
  let offset=65536u32-(n+2);let mut b=vec![];if w==32{b.push(0x66);}b.extend([0xff,0x1e]);b.extend((offset as u16).to_le_bytes());
  let(mut i,mut r,mut c)=setup(&b,false);pointer(&mut r,0x100+offset,w,0x345,1);step(&mut i,&mut r,&mut c).unwrap();assert_eq!((i.selectors[1],c.eip),(1,0x345));
 }
}
