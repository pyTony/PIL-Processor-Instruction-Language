//! Literal expected states for ordinary-RAM XCHG, independent of its PIL.
use super::*;
fn step(bytes:&[u8],cpu:&mut Cpu,ram:&mut Ram,image:&mut guest::Image)->Result<(),ExecError>{
 let d=image.decode(ram,cpu.eip).unwrap();
 if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{
  let mut machine=image.segment_state();let result=execute_machine(&program,cpu,dst,src,flow,ram,address,&mut machine);image.set_segment_state(machine);result
 }else{panic!("not PIL: {bytes:02x?}")}
}
#[test]
fn exchange_memory_widths_address_sizes_lock_and_aliases(){
 for w in [8,16,32]{for a32 in [false,true]{for lock in [false,true]{
  let mut bytes=vec![];if lock{bytes.push(0xf0);}if w==32{bytes.push(0x66);}if a32{bytes.push(0x67);}bytes.extend([if w==8{0x86}else{0x87},7]);
  let(mut image,mut ram)=guest::Image::load(&bytes).unwrap();
  let mut cpu=Cpu{regs:[0x12345678,1,2,0x700,4,5,6,0x700],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};
  ram.store(0x800,MemWidth::Dword,0xdeadbeef).unwrap();let mut want=cpu.clone();let mut want_ram=ram.clone();
  want.regs[0]=(want.regs[0]&!(mask(w)as u32))|(0xdeadbeef&mask(w)as u32);want.eip+=bytes.len()as u32;
  want_ram.store(0x800,MemWidth::from_bits(w).unwrap(),0x12345678).unwrap();let old_image=image.clone();
  step(&bytes,&mut cpu,&mut ram,&mut image).unwrap();assert_eq!(cpu,want);assert_eq!(ram,want_ram);assert_eq!(image,old_image);
 }}}
 // Source BH aliases BX in the address. EA must see incoming BX.
 let bytes=[0x86,0x3f];let(mut image,mut ram)=guest::Image::load(&bytes).unwrap();let mut cpu=Cpu{eip:0x100,..Cpu::default()};cpu.regs[3]=0x700;
 ram.store(0x800,MemWidth::Byte,0xab).unwrap();step(&bytes,&mut cpu,&mut ram,&mut image).unwrap();assert_eq!(cpu.regs[3],0xab00);assert_eq!(ram.load(0x800,MemWidth::Byte).unwrap(),7);
}
#[test]
fn exchange_memory_ss_fs_sib_and_esp_snapshot(){
 for (bytes,at,reg,value) in [
  (vec![0x87,0x46,0],0x900,0,0x5678),
  (vec![0x64,0x87,7],0xa00,0,0x5678),
  (vec![0x66,0x67,0x87,0x64,0x24,0],0xa00,4,0x800),
 ]{
  let(mut image,mut ram)=guest::Image::load(&bytes).unwrap();let mut cpu=Cpu{regs:[0x12345678,0,0,0x700,0x800,0x700,0,0],eip:0x100,..Cpu::default()};
  ram.store(at,MemWidth::Dword,0xdeadbeef).unwrap();step(&bytes,&mut cpu,&mut ram,&mut image).unwrap();
  let wide=bytes[0]==0x66;assert_eq!(cpu.regs[reg],if wide{0xdeadbeef}else{0x1234beef});assert_eq!(ram.load(at,if wide{MemWidth::Dword}else{MemWidth::Word}).unwrap(),value);
 }
}
#[test]
fn exchange_memory_faults_do_not_commit_registers_ram_or_markers(){
 for fault in 0..5{
  let bytes=[0x66,0x87,7];let(mut image,mut ram)=guest::Image::load(&bytes).unwrap();let mut cpu=Cpu{eip:0x100,..Cpu::default()};cpu.regs[0]=0x12345678;cpu.regs[3]=0x700;image.ss_shadow=1;image.sti_shadow=1;
  match fault{0=>ram.protect(0x803,1).unwrap(),1=>image.segments[3].limit=0x702,2=>image.segments[3].access=address::SegmentAccess::Unusable,3=>cpu.regs[3]=0xffff,_=>{ram=Ram::new(0x803);ram.write_bytes(0x100,&bytes).unwrap();}}
  let old=cpu.clone();let old_ram=ram.clone();let old_image=image.clone();assert!(step(&bytes,&mut cpu,&mut ram,&mut image).is_err());assert_eq!(cpu,old);assert_eq!(ram,old_ram);assert_eq!(image,old_image);
 }
}
#[test]
fn exchange_memory_invalid_prefixes_and_truncation_still_reject(){
 for bytes in [vec![0xf0,0x87,0xc0],vec![0xf0,0x90],vec![0xf0,0xf0,0x87,7],vec![0xf3,0x87,7],vec![0x87,6,0],vec![0x67,0x87,4]]{
  let(image,ram)=guest::Image::load(&bytes).unwrap();assert!(image.decode(&ram,0x100).is_err(),"{bytes:02x?}");
 }
}
