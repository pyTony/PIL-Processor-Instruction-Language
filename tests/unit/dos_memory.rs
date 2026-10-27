use super::*;
use super::super::{load,run,hello,Stop,service};
fn call(p:&mut Loaded,ah:u8,bx:u16,es:u16){p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|(u32::from(ah)<<8);low(&mut p.cpu.regs[3],bx);p.image.selectors[0]=es;let mut out=Vec::new();assert_eq!(service(p,0x21,&mut out),Ok(None));assert!(out.is_empty());}
fn fresh()->Loaded{load(&[0x90],"arena.com").unwrap()}
fn cf(p:&Loaded)->bool{p.cpu.eflags&1!=0}
#[test]
fn loader_creates_mcb_chain_and_ram_is_distinct_from_allocation(){
 let p=fresh();assert_eq!(p.ram.bytes().len(),LIMIT);assert_eq!(p.allocated_end,LIMIT as u32);assert_eq!(p.ram.load(u32::from(PSP-1)*16,MemWidth::Byte).unwrap(),u32::from(b'Z'));assert_eq!(p.arena.blocks,vec![Block{header:PSP-1,size:0x9000,owner:PSP}]);assert!(p.arena.valid(&p.ram));
 let mut b=hello("exe").unwrap();super::super::put_word(&mut b,12,1);super::super::put_word(&mut b,16,0x10);let p=load(&b,"small.exe").unwrap();assert!(p.allocated_end<LIMIT as u32);assert_eq!(p.arena.blocks.len(),2);assert_eq!(p.arena.blocks[1].owner,0);assert!(p.arena.valid(&p.ram));
}
#[test]
fn shrink_alloc_free_and_first_fit_reuse_have_correct_headers(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);assert!(!cf(&p));assert_eq!(p.allocated_end,0x20000);assert_eq!(p.ram.load(u32::from(PSP)*16+2,MemWidth::Word).unwrap(),0x2000);assert_eq!(p.arena.blocks[1],Block{header:0x2000,size:0x7fff,owner:0});
 call(&mut p,0x48,0x20,0);assert!(!cf(&p));assert_eq!(p.cpu.regs[0]as u16,0x2001);assert_eq!(p.arena.blocks[1],Block{header:0x2000,size:0x20,owner:PSP});call(&mut p,0x49,0,0x2001);assert!(!cf(&p));assert_eq!(p.arena.blocks[1].owner,0);call(&mut p,0x48,0x10,0);assert_eq!(p.cpu.regs[0]as u16,0x2001);assert!(p.arena.valid(&p.ram));
}
#[test]
fn allocation_failure_returns_largest_and_preserves_high_registers_flags(){
 let mut p=fresh();p.cpu.regs[0]=0xabcd0000;p.cpu.regs[3]=0xdead0000;p.cpu.eflags=0xed6;p.cpu.undefined_flags=u32::MAX;let oldr=p.ram.clone();let olda=p.arena.clone();call(&mut p,0x48,1,0);assert_eq!(p.cpu.regs[0],0xabcd0008);assert_eq!(p.cpu.regs[3],0xdead0000);assert_eq!(p.cpu.eflags,0xed7);assert_eq!(p.cpu.undefined_flags,u32::MAX&!1);assert_eq!(p.ram,oldr);assert_eq!(p.arena,olda);
 call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0xffff,0);assert_eq!(p.cpu.regs[0]as u16,8);assert_eq!(p.cpu.regs[3]as u16,0x7fff);
}
#[test]
fn resize_failure_grows_to_maximum_before_returning_dos8(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0x100,0);let block=p.cpu.regs[0]as u16;call(&mut p,0x4a,0xffff,PSP);assert!(cf(&p));assert_eq!((p.cpu.regs[0]as u16,p.cpu.regs[3]as u16),(8,0x1000));assert_eq!(p.allocated_end,0x20000);
 call(&mut p,0x49,0,block);call(&mut p,0x4a,0xffff,PSP);assert!(cf(&p));assert_eq!((p.cpu.regs[0]as u16,p.cpu.regs[3]as u16),(8,0x9000));assert_eq!(p.allocated_end,LIMIT as u32);assert_eq!(p.arena.blocks.len(),1);assert!(p.arena.valid(&p.ram));
}
#[test]
fn free_does_not_zero_data_and_coalescing_waits_until_allocate(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0x10,0);let a=p.cpu.regs[0]as u16;call(&mut p,0x48,0x10,0);let b=p.cpu.regs[0]as u16;p.ram.store(u32::from(a)*16,MemWidth::Word,0xbeef).unwrap();call(&mut p,0x49,0,a);call(&mut p,0x49,0,b);assert_eq!(p.arena.blocks.len(),4);assert_eq!(p.ram.load(u32::from(a)*16,MemWidth::Word).unwrap(),0xbeef);call(&mut p,0x48,0x30,0);assert_eq!(p.cpu.regs[0]as u16,a);assert_eq!(p.arena.blocks.len(),3);assert!(p.arena.valid(&p.ram));
}
#[test]
fn invalid_block_double_free_and_corrupted_mcb_return_dos_errors(){
 let mut p=fresh();for ah in [0x49,0x4a]{let old=p.arena.clone();call(&mut p,ah,0,PSP+1);assert!(cf(&p));assert_eq!(p.cpu.regs[0]as u16,9);assert_eq!(p.arena,old);}
 call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0x10,0);let seg=p.cpu.regs[0]as u16;call(&mut p,0x49,0,seg);call(&mut p,0x49,0,seg);assert_eq!(p.cpu.regs[0]as u16,9);
 p.ram.store(u32::from(PSP-1)*16,MemWidth::Byte,0).unwrap();let old=p.arena.clone();let oldr=p.ram.clone();call(&mut p,0x48,1,0);assert_eq!(p.cpu.regs[0]as u16,7);assert_eq!(p.arena,old);assert_eq!(p.ram,oldr);
}
#[test]
fn zero_size_and_exact_fit_keep_a_valid_terminating_chain(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0,0);let seg=p.cpu.regs[0]as u16;assert!(!cf(&p));assert_eq!(seg,0x2001);assert_eq!(p.arena.blocks[1].size,0);call(&mut p,0x49,0,seg);call(&mut p,0x48,0x7fff,0);assert!(!cf(&p));assert_eq!(p.arena.blocks.len(),2);assert!(p.arena.valid(&p.ram));
}
#[test]
fn protected_mcb_or_psp_write_stops_service_without_any_commit(){
 for protect_psp in [false,true]{let mut p=fresh();p.ram.protect(if protect_psp{u32::from(PSP)*16+2}else{u32::from(PSP-1)*16},2).unwrap();p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x1000;let oldc=p.cpu.clone();let oldr=p.ram.clone();let olda=p.arena.clone();let end=p.allocated_end;assert!(service(&mut p,0x21,&mut Vec::new()).is_err());assert_eq!(p.cpu,oldc);assert_eq!(p.ram,oldr);assert_eq!(p.arena,olda);assert_eq!(p.allocated_end,end);}
}
#[test]
fn original_guest_calls_reach_resize_allocate_free_and_exit(){
 let code=[0xbb,0,0x10,0xb4,0x4a,0xcd,0x21,0xbb,0x10,0,0xb4,0x48,0xcd,0x21,0x8e,0xc0,0xb4,0x49,0xcd,0x21,0xb8,0,0x4c,0xcd,0x21];let result=run(load(&code,"allocate.com").unwrap(),50).unwrap();assert_eq!(result.stop,Stop::Exit(0));assert_eq!(result.dos_calls,4);assert_eq!(result.loaded.allocated_end,0x20000);assert!(result.loaded.arena.valid(&result.loaded.ram));
}
