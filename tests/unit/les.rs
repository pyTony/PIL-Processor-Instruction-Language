//! Literal LES expectations independent of its PIL description.
use super::*;
fn step(cpu:&mut Cpu,ram:&mut Ram,image:&mut guest::Image)->Result<(),ExecError>{
 let d=image.decode(ram,cpu.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{
  let mut machine=image.segment_state();let result=execute_machine(&program,cpu,dst,src,flow,ram,address,&mut machine);image.set_segment_state(machine);result
 }else{panic!("expected PIL")}
}
#[test]
fn les_width_address_segment_and_destination_aliases(){
 for w in [16,32]{for a32 in [false,true]{for prefix in [None,Some(0x26),Some(0x64)]{
  let mut bytes=vec![];if w==32{bytes.push(0x66);}if a32{bytes.push(0x67);}if let Some(p)=prefix{bytes.push(p);}bytes.extend([0xc4,0x1f]);
  let(mut image,mut ram)=guest::Image::load(&bytes).unwrap();let mut cpu=Cpu{regs:[0,1,2,0x700,4,5,6,0x700],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};
  let base=match prefix{Some(0x26)=>0x100,Some(0x64)=>0x300,_=>0x100};let at=base+0x700;
  ram.store(at,MemWidth::Dword,0xdeadbeef).unwrap();ram.store(at+w/8,MemWidth::Word,0xfedc).unwrap();ram.protect(at,(w/8+2)as usize).unwrap();
  let mut want=cpu.clone();want.regs[3]=if w==16{0xbeef}else{0xdeadbeef};want.eip+=bytes.len()as u32;
  let old_ram=ram.clone();let mut want_image=image.clone();want_image.selectors[0]=0xfedc;want_image.segments[0]=address::SegmentCache::real_mode(0xfedc);
  step(&mut cpu,&mut ram,&mut image).unwrap();assert_eq!(cpu,want);assert_eq!(ram,old_ram);assert_eq!(image,want_image);
 }}}
}
#[test]
fn les_ss_and_sib_sources_and_upper_destination_preservation(){
 for (bytes,at) in [(vec![0xc4,0x7e,0],0x900),(vec![0x67,0xc4,0x7c,0x8b,4],0x824)]{
  let(mut image,mut ram)=guest::Image::load(&bytes).unwrap();let mut cpu=Cpu{regs:[0,8,0,0x700,0,0x700,0,0xabcd1234],eip:0x100,..Cpu::default()};
  ram.store(at,MemWidth::Dword,0x4321beef).unwrap();step(&mut cpu,&mut ram,&mut image).unwrap();assert_eq!(cpu.regs[7],0xabcdbeef);assert_eq!(image.selectors[0],0x4321);assert_eq!(image.segments[0].base,0x43210);
 }
}
#[test]
fn les_pointer_faults_leave_registers_segments_ram_and_markers_unchanged(){
 for fault in 0..4{
  let bytes=[0xc4,0x3f];let(mut image,mut ram)=guest::Image::load(&bytes).unwrap();let mut cpu=Cpu{eip:0x100,..Cpu::default()};cpu.regs[3]=0x700;cpu.regs[7]=0xabcd1234;image.ss_shadow=1;image.sti_shadow=1;
  ram.store(0x800,MemWidth::Word,0xbeef).unwrap();match fault{0=>image.segments[3].limit=0x702,1=>{ram=Ram::new(0x803);ram.write_bytes(0x100,&bytes).unwrap();ram.store(0x800,MemWidth::Word,0xbeef).unwrap();},2=>image.segments[3].access=address::SegmentAccess::Unusable,_=>cpu.regs[3]=0xfffe}
  let old=cpu.clone();let old_ram=ram.clone();let old_image=image.clone();assert!(step(&mut cpu,&mut ram,&mut image).is_err());assert_eq!(cpu,old);assert_eq!(ram,old_ram);assert_eq!(image,old_image);
 }
}
#[test]
fn les_rejects_register_prefix_mode_and_truncated_forms(){
 for bytes in [vec![0xc4,0xff],vec![0xf0,0xc4,0x3f],vec![0xf3,0xc4,0x3f],vec![0xc4],vec![0xc4,0x3e,0],vec![0x67,0xc4,0x3c]]{
  let(image,ram)=guest::Image::load(&bytes).unwrap();assert!(image.decode(&ram,0x100).is_err());
 }
 let(mut image,ram)=guest::Image::load(&[0xc4,0x3f]).unwrap();image.real_mode=false;assert!(image.decode(&ram,0x100).is_err());
}
