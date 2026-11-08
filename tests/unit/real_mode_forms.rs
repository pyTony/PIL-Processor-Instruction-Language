//! Original expectations for real-mode forms found by SingleStepTests/80386:
//! F2 STOS/LODS, FWAIT, CLTS, SALC, BOUND, ENTER with a nesting level and
//! open-bus port I/O.
//! Image::load gives ES base 0, DS base 100h and SS base 200h.
use super::*;
fn machine(code:&[u8],regs:[u32;8],flags:u32,ram:&[(u32,u8,u32)])->(guest::Image,Ram,Cpu){
 let(image,_)=guest::Image::load(code).unwrap();let mut r=Ram::new(0x21000);r.write_bytes(0x100,code).unwrap();
 for &(at,w,v) in ram{r.store(at,MemWidth::from_bits(w).unwrap(),v).unwrap();}
 (image,r,Cpu{regs,eflags:flags,undefined_flags:0,eip:0x100})
}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
 let d=i.decode(r,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{let mut m=i.segment_state();let result=execute_machine(&program,c,dst,src,flow,r,address,&mut m);i.set_segment_state(m);result}else{panic!("PIL expected")}
}
/// Steps a REP form until it leaves its own address.
fn run_out(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu){for _ in 0..16{step(i,r,c).unwrap();if c.eip!=0x100{return;}}panic!("REP did not finish");}
#[test]
fn f2_stos_and_lods_repeat_by_count_without_testing_zf(){
 // F2 AA = REP STOSB with CX=3, AL=5Ah, DI=10h, ZF clear: stores at ES:10h..12h, CX=0, DI=13h.
 let(mut i,mut r,mut c)=machine(&[0xf2,0xaa],[0x5a,3,0,0,0x800,0,0,0x10],2,&[]);run_out(&mut i,&mut r,&mut c);
 assert_eq!((c.regs[1],c.regs[7],c.eip),(0,0x13,0x102));assert_eq!([0x10,0x11,0x12,0x13].map(|a|r.load(a,MemWidth::Byte).unwrap()),[0x5a,0x5a,0x5a,0]);
 // F2 66 AD = REP LODSD with CX=2, ZF set: reads DS:20h then DS:24h, EAX is the last, SI=28h.
 let(mut i,mut r,mut c)=machine(&[0xf2,0x66,0xad],[0,2,0,0,0x800,0,0x20,0],0x42,&[(0x120,32,0x1111_1111),(0x124,32,0x2222_2222)]);run_out(&mut i,&mut r,&mut c);
 assert_eq!((c.regs[0],c.regs[1],c.regs[6],c.eip),(0x2222_2222,0,0x28,0x103));
}
#[test]
fn fwait_is_a_no_op_unless_ts_and_mp_are_both_set(){
 for (msw,ok) in [(0x10u16,true),(0x18,true),(0x12,true),(0x1a,false)]{
  let(mut i,mut r,mut c)=machine(&[0x9b],[1,2,3,4,0x800,6,7,8],0x8d7,&[]);i.msw=msw;let old=c.clone();let result=step(&mut i,&mut r,&mut c);
  if ok{result.unwrap();assert_eq!(c.eip,0x101);assert_eq!((c.regs,c.eflags),(old.regs,old.eflags));}
  else{assert_eq!(result,Err(ExecError::InvalidContext));assert_eq!(c,old);}
 }
}
#[test]
fn clts_clears_only_cr0_ts(){
 for (msw,want) in [(0x18u16,0x10u16),(0x1e,0x16),(0x10,0x10)]{
  let(mut i,mut r,mut c)=machine(&[0x0f,0x06],[1,2,3,4,0x800,6,7,8],0x8d7,&[]);i.msw=msw;let old=c.clone();
  step(&mut i,&mut r,&mut c).unwrap();assert_eq!(i.msw,want);assert_eq!((c.regs,c.eflags,c.eip),(old.regs,old.eflags,0x102));
 }
}
#[test]
fn salc_sets_al_from_cf_and_preserves_flags(){
 for (flags,al) in [(0x8d7u32,0xffu32),(0x8d6,0)]{
  let(mut i,mut r,mut c)=machine(&[0xd6],[0x1234_5678,2,3,4,0x800,6,7,8],flags,&[]);step(&mut i,&mut r,&mut c).unwrap();
  assert_eq!((c.regs[0],c.eflags,c.eip),(0x1234_5600|al,flags,0x101));
 }
}
#[test]
fn bound_raises_br_outside_the_signed_pair_and_changes_nothing(){
 // 62 06 00 04 = BOUND AX,[0400h]: lower bound at DS:0400h (linear 500h), upper at 502h.
 for (ax,inside) in [(0xfffbu32,true),(0xa,true),(0,true),(0xb,false),(0xfffa,false),(0x8000,false)]{
  let(mut i,mut r,mut c)=machine(&[0x62,0x06,0x00,0x04],[ax,2,3,4,0x800,6,7,8],0x8d7,&[(0x500,16,0xfffb),(0x502,16,0x000a)]);let old=c.clone();
  let result=step(&mut i,&mut r,&mut c);
  if inside{result.unwrap();assert_eq!(c.eip,0x104);assert_eq!((c.regs,c.eflags),(old.regs,old.eflags));}
  else{assert_eq!(result,Err(ExecError::Exception{eip:0x100,vector:5}),"AX={ax:x}");assert_eq!(c,old);}
 }
 // 32-bit form: bounds -2 and 7FFFFFFFh as dwords.
 for (eax,inside) in [(0xffff_fffeu32,true),(0x7fff_ffff,true),(0x8000_0000,false),(0xffff_fffd,false)]{
  let(mut i,mut r,mut c)=machine(&[0x66,0x62,0x06,0x00,0x04],[eax,2,3,4,0x800,6,7,8],2,&[(0x500,32,0xffff_fffe),(0x504,32,0x7fff_ffff)]);
  let result=step(&mut i,&mut r,&mut c);assert_eq!(result.is_ok(),inside,"EAX={eax:x}");
 }
 // 26 62 07 = BOUND AX,ES:[BX] with BX=FFFEh: lower at ES:FFFEh, upper wraps to ES:0000h.
 let(mut i,mut r,mut c)=machine(&[0x26,0x62,0x07],[5,2,3,0xfffe,0x800,6,7,8],2,&[(0xfffe,16,0),(0,16,4)]);
 assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::Exception{eip:0x100,vector:5}));
 r.store(0,MemWidth::Word,5).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.eip,0x103);
 // The register form is not an encoding of BOUND.
 let(i,r,_)=machine(&[0x62,0xc0],[0;8],2,&[]);assert!(i.decode(&r,0x100).is_err());
}
#[test]
fn enter_with_a_nesting_level_copies_enclosing_frame_pointers(){
 // 66 C8 08 00 03 = ENTER 8,3 (32-bit) on the 16-bit stack, SP=0100h, EBP=1234_0080h.
 // Copies [SS:7Ch] and [SS:78h], then pushes the frame pointer 00FCh.
 let(mut i,mut r,mut c)=machine(&[0x66,0xc8,0x08,0x00,0x03],[0,0,0,0,0xabcd_0100,0x1234_0080,0,0],2,&[(0x27c,32,0x1111_1111),(0x278,32,0x2222_2222)]);
 step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!((c.regs[5],c.regs[4],c.eip),(0xfc,0xabcd_00e8,0x105));
 assert_eq!([0x2fc,0x2f8,0x2f4,0x2f0].map(|a|r.load(a,MemWidth::Dword).unwrap()),[0x1234_0080,0x1111_1111,0x2222_2222,0xfc]);
 // C8 04 00 01 = ENTER 4,1: push BP, push the frame pointer; BP=00FEh, SP=00FEh-2-4.
 let(mut i,mut r,mut c)=machine(&[0xc8,0x04,0x00,0x01],[0,0,0,0,0x100,0x5555_4444,0,0],2,&[]);
 step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!((c.regs[5],c.regs[4]),(0x5555_00fe,0xf8));assert_eq!([0x2fe,0x2fc].map(|a|r.load(a,MemWidth::Word).unwrap()),[0x4444,0xfe]);
 // Level 0 with a 32-bit operand on the 16-bit stack writes all of EBP.
 let(mut i,mut r,mut c)=machine(&[0x66,0xc8,0x00,0x00,0x00],[0,0,0,0,0x100,0x5555_4444,0,0],2,&[]);
 step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.regs[5],c.regs[4]),(0xfc,0xfc));
 // Level 32 is level 0 (the level is taken modulo 32).
 let(mut i,mut r,mut c)=machine(&[0xc8,0x00,0x00,0x20],[0,0,0,0,0x100,0x4444,0,0],2,&[]);step(&mut i,&mut r,&mut c).unwrap();assert_eq!((c.regs[5],c.regs[4]),(0xfe,0xfe));
}
#[test]
fn ports_without_a_device_read_all_ones_and_ignore_writes(){
 // IN reads FFh/FFFFh/FFFFFFFFh into AL/AX/EAX; OUT changes nothing.
 for (bytes,eax,want) in [(vec![0xe4,0x80],0x1234_5600u32,0x1234_56ffu32),(vec![0xe5,0x80],0x1234_0000,0x1234_ffff),(vec![0x66,0xe5,0x80],0,0xffff_ffff),
  (vec![0xec],0x1234_5600,0x1234_56ff),(vec![0xed],0x1234_0000,0x1234_ffff),(vec![0x66,0xed],0,0xffff_ffff),(vec![0xe6,0x80],0x1234_5678,0x1234_5678),(vec![0x66,0xef],0x1234_5678,0x1234_5678)]{
  let(mut i,mut r,mut c)=machine(&bytes,[eax,0,0x3f8,0,0x800,0,0,0],0x8d7,&[]);let oldr=r.clone();step(&mut i,&mut r,&mut c).unwrap();
  assert_eq!((c.regs[0],c.eflags,c.eip),(want,0x8d7,0x100+bytes.len() as u32),"{bytes:02x?}");assert_eq!(r,oldr);
 }
 // F3 6D = REP INSW, CX=2, DI=10h: writes FFFFh at ES:10h and 12h. F2 6E = REP OUTSB (F2 repeats too), CX=3, SI=20h.
 let(mut i,mut r,mut c)=machine(&[0xf3,0x6d],[0,2,0x3f8,0,0x800,0,0,0x10],2,&[]);run_out(&mut i,&mut r,&mut c);
 assert_eq!((c.regs[1],c.regs[7]),(0,0x14));assert_eq!([0x10,0x12,0x14].map(|a|r.load(a,MemWidth::Word).unwrap()),[0xffff,0xffff,0]);
 let(mut i,mut r,mut c)=machine(&[0xf2,0x6e],[0,3,0x3f8,0,0x800,0,0x20,0],0x402,&[]);run_out(&mut i,&mut r,&mut c);
 assert_eq!((c.regs[1],c.regs[6]),(0,0x1d));
}
#[test]
fn enter_reads_frame_pointers_it_has_just_pushed(){
 // C8 00 00 03 = ENTER 0,3 with SP=0102h and BP=0102h: BP is pushed at 0100h, then the
 // copies read [BP-2] (the BP just pushed, 0102h) and [BP-4] (the first copy, 0102h).
 let(mut i,mut r,mut c)=machine(&[0xc8,0x00,0x00,0x03],[0,0,0,0,0x102,0x102,0,0],2,&[]);step(&mut i,&mut r,&mut c).unwrap();
 assert_eq!([0x300,0x2fe,0x2fc,0x2fa].map(|a|r.load(a,MemWidth::Word).unwrap()),[0x102,0x102,0x102,0x100]);
 assert_eq!((c.regs[5],c.regs[4]),(0x100,0xfa));
}
