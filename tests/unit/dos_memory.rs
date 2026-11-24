use super::*;
use super::super::{load,run,hello,Stop,service};
use super::super::LoadedOps;
fn call(p:&mut Loaded,ah:u8,bx:u16,es:u16){p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|(u32::from(ah)<<8);low(&mut p.cpu.regs[3],bx);p.image.selectors[0]=es;let mut out=Vec::new();assert_eq!(service(p,0x21,&mut out),Ok(None));assert!(out.is_empty());}
/// Conventional memory alone (no XMS, so no UMBs): the arena as before DOS=UMB.
fn plain(mut p:Loaded)->Loaded{p.select_xms(false).unwrap();p}
fn fresh()->Loaded{plain(load(&[0x90],"arena.com").unwrap())}
#[test]
fn guest_split_mcbs_are_used_for_allocation_and_owner_release(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);
 // Split a free block into a guest-owned resident block and a free remainder.
 let first=0x2000u16;let second=first+0x21;
 p.ram.write_bytes(u32::from(first)*16,&[b'M',0x67,0x45,0x20,0]).unwrap();
 let left=END-second-1;let [lo,hi]=left.to_le_bytes();
 p.ram.write_bytes(u32::from(second)*16,&[b'Z',0,0,lo,hi]).unwrap();
 call(&mut p,0x48,0x10,0);assert!(!cf(&p));assert_eq!(p.cpu.regs[0]as u16,second+1);
 assert_eq!(p.dos.arena.block_owner(first+1),Some(0x4567));assert!(p.dos.arena.valid(&p.ram));
 // Insert a second block for that owner, unknown to the host's old shape.
 let tail=second+0x11;let size=END-tail-1;
 p.ram.write_bytes(u32::from(tail)*16,&[b'M',0x67,0x45,0x10,0]).unwrap();
 let end=tail+0x11;let [lo,hi]=(size-0x11).to_le_bytes();
 p.ram.write_bytes(u32::from(end)*16,&[b'Z',0,0,lo,hi]).unwrap();
 p.dos.arena.release(&mut p.ram,0x4567,None).unwrap();
 assert_eq!(p.dos.arena.block_owner(first+1),Some(0));
 assert_eq!(p.dos.arena.block_owner(tail+1),Some(0));assert!(p.dos.arena.valid(&p.ram));
}
#[test]
fn guest_mcb_chain_rejects_out_of_range_and_missing_terminators(){
 for (kind,size) in [(b'M',0xffffu16),(b'Z',0xffff),(b'M',0x9000)]{
  let mut p=fresh();let [lo,hi]=size.to_le_bytes();
  p.ram.write_bytes(u32::from(PSP-1)*16,&[kind,0,0,lo,hi]).unwrap();
  let old=(p.ram.clone(),p.dos.arena.clone());call(&mut p,0x48,1,0);
  assert!(cf(&p));assert_eq!(p.cpu.regs[0]as u16,7);assert_eq!(p.ram,old.0);assert_eq!(p.dos.arena,old.1);
 }
}
#[test]
fn guest_mcb_chain_count_is_bounded_and_release_preflights_all_owners(){
 let mut p=fresh();
 for i in 0..=MAX_BLOCKS{p.ram.write_bytes((u32::from(ENV-1)+i as u32)*16,&[b'M',0,0,0,0]).unwrap();}
 let old=(p.ram.clone(),p.dos.arena.clone());call(&mut p,0x48,1,0);
 assert!(cf(&p));assert_eq!(p.cpu.regs[0]as u16,7);assert_eq!(p.ram,old.0);assert_eq!(p.dos.arena,old.1);
 let mut p=fresh();p.ram.protect(u32::from(PSP-1)*16+1,2).unwrap();
 let old=(p.ram.clone(),p.dos.arena.clone());
 assert!(p.dos.arena.release(&mut p.ram,PSP,None).is_err());
 assert_eq!(p.ram,old.0);assert_eq!(p.dos.arena,old.1);
}
fn cf(p:&Loaded)->bool{p.cpu.eflags&1!=0}
#[test]
fn loader_creates_mcb_chain_and_ram_is_distinct_from_allocation(){
 let p=fresh();assert_eq!(p.ram.bytes().len(),super::super::PHYSICAL_RAM);assert!(!p.ram.a20_enabled());assert_eq!(p.dos.allocated_end,LIMIT as u32);assert_eq!(p.ram.load(u32::from(PSP-1)*16,MemWidth::Byte).unwrap(),u32::from(b'Z'));assert_eq!(p.ram.load(u32::from(ENV-1)*16,MemWidth::Byte).unwrap(),u32::from(b'M'));assert_eq!(p.dos.arena.blocks,vec![Block{header:ENV-1,size:ENV_PARAS,owner:PSP},Block{header:PSP-1,size:0x9000,owner:PSP}]);assert!(p.dos.arena.valid(&p.ram));
 let mut b=hello("exe").unwrap();super::super::put_word(&mut b,12,1);super::super::put_word(&mut b,16,0x10);let p=plain(load(&b,"small.exe").unwrap());assert!(p.dos.allocated_end<LIMIT as u32);assert_eq!(p.dos.arena.blocks.len(),3);assert_eq!(p.dos.arena.blocks[2].owner,0);assert!(p.dos.arena.valid(&p.ram));
}
#[test]
fn shrink_alloc_free_and_first_fit_reuse_have_correct_headers(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);assert!(!cf(&p));assert_eq!(p.dos.allocated_end,0x20000);assert_eq!(p.ram.load(u32::from(PSP)*16+2,MemWidth::Word).unwrap(),0xa000);assert_eq!(p.cpu.regs[0]as u16,PSP);assert_eq!(p.dos.arena.blocks[2],Block{header:0x2000,size:0x7fff,owner:0});
 call(&mut p,0x48,0x20,0);assert!(!cf(&p));assert_eq!(p.cpu.regs[0]as u16,0x2001);assert_eq!(p.dos.arena.blocks[2],Block{header:0x2000,size:0x20,owner:PSP});call(&mut p,0x49,0,0x2001);assert!(!cf(&p));assert_eq!(p.dos.arena.blocks[2].owner,0);call(&mut p,0x48,0x10,0);assert_eq!(p.cpu.regs[0]as u16,0x2001);assert!(p.dos.arena.valid(&p.ram));
}
#[test]
fn allocation_failure_returns_largest_and_preserves_high_registers_flags(){
 let mut p=fresh();p.cpu.regs[0]=0xabcd0000;p.cpu.regs[3]=0xdead0000;p.cpu.eflags=0xed6;p.cpu.undefined_flags=u32::MAX;let oldr=p.ram.clone();let olda=p.dos.arena.clone();call(&mut p,0x48,1,0);assert_eq!(p.cpu.regs[0],0xabcd0008);assert_eq!(p.cpu.regs[3],0xdead0000);assert_eq!(p.cpu.eflags,0xed7);assert_eq!(p.cpu.undefined_flags,u32::MAX&!1);assert_eq!(p.ram,oldr);assert_eq!(p.dos.arena,olda);
 call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0xffff,0);assert_eq!(p.cpu.regs[0]as u16,8);assert_eq!(p.cpu.regs[3]as u16,0x7fff);
}
#[test]
fn resize_failure_grows_to_maximum_before_returning_dos8(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0x100,0);let block=p.cpu.regs[0]as u16;call(&mut p,0x4a,0xffff,PSP);assert!(cf(&p));assert_eq!((p.cpu.regs[0]as u16,p.cpu.regs[3]as u16),(8,0x1000));assert_eq!(p.dos.allocated_end,0x20000);
 call(&mut p,0x49,0,block);call(&mut p,0x4a,0xffff,PSP);assert!(cf(&p));assert_eq!((p.cpu.regs[0]as u16,p.cpu.regs[3]as u16),(8,0x9000));assert_eq!(p.dos.allocated_end,LIMIT as u32);assert_eq!(p.dos.arena.blocks.len(),2);assert!(p.dos.arena.valid(&p.ram));
}
#[test]
fn free_does_not_zero_data_and_coalescing_waits_until_allocate(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0x10,0);let a=p.cpu.regs[0]as u16;call(&mut p,0x48,0x10,0);let b=p.cpu.regs[0]as u16;p.ram.store(u32::from(a)*16,MemWidth::Word,0xbeef).unwrap();call(&mut p,0x49,0,a);call(&mut p,0x49,0,b);assert_eq!(p.dos.arena.blocks.len(),5);assert_eq!(p.ram.load(u32::from(a)*16,MemWidth::Word).unwrap(),0xbeef);call(&mut p,0x48,0x30,0);assert_eq!(p.cpu.regs[0]as u16,a);assert_eq!(p.dos.arena.blocks.len(),4);assert!(p.dos.arena.valid(&p.ram));
}
#[test]
fn invalid_block_double_free_and_corrupted_mcb_return_dos_errors(){
 let mut p=fresh();for ah in [0x49,0x4a]{let old=p.dos.arena.clone();call(&mut p,ah,0,PSP+1);assert!(cf(&p));assert_eq!(p.cpu.regs[0]as u16,9);assert_eq!(p.dos.arena,old);}
 call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0x10,0);let seg=p.cpu.regs[0]as u16;call(&mut p,0x49,0,seg);call(&mut p,0x49,0,seg);assert_eq!(p.cpu.regs[0]as u16,9);
 p.ram.store(u32::from(PSP-1)*16,MemWidth::Byte,0).unwrap();let old=p.dos.arena.clone();let oldr=p.ram.clone();call(&mut p,0x48,1,0);assert_eq!(p.cpu.regs[0]as u16,7);assert_eq!(p.dos.arena,old);assert_eq!(p.ram,oldr);
}
#[test]
fn zero_size_and_exact_fit_keep_a_valid_terminating_chain(){
 let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0,0);let seg=p.cpu.regs[0]as u16;assert!(!cf(&p));assert_eq!(seg,0x2001);assert_eq!(p.dos.arena.blocks[2].size,0);call(&mut p,0x49,0,seg);call(&mut p,0x48,0x7fff,0);assert!(!cf(&p));assert_eq!(p.dos.arena.blocks.len(),3);assert!(p.dos.arena.valid(&p.ram));
}
#[test]
fn protected_old_or_new_mcb_write_stops_service_without_any_commit(){
 for protect_new in [false,true]{let mut p=fresh();p.ram.protect(if protect_new{0x20000}else{u32::from(PSP-1)*16},2).unwrap();p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x1000;let oldc=p.cpu.clone();let oldr=p.ram.clone();let olda=p.dos.arena.clone();let end=p.dos.allocated_end;assert!(service(&mut p,0x21,&mut Vec::new()).is_err());assert_eq!(p.cpu,oldc);assert_eq!(p.ram,oldr);assert_eq!(p.dos.arena,olda);assert_eq!(p.dos.allocated_end,end);}
}
#[test]
fn original_guest_calls_reach_resize_allocate_free_and_exit(){
 let code=[0xbb,0,0x10,0xb4,0x4a,0xcd,0x21,0xbb,0x10,0,0xb4,0x48,0xcd,0x21,0x8e,0xc0,0xb4,0x49,0xcd,0x21,0xb8,0,0x4c,0xcd,0x21];let result=run(load(&code,"allocate.com").unwrap(),50).unwrap();assert_eq!(result.stop,Stop::Exit(0));assert_eq!(result.dos_calls,4);assert_eq!(result.loaded.dos.allocated_end,0x20000);assert!(result.loaded.dos.arena.valid(&result.loaded.ram));
}
#[test]
fn best_and_last_fit_strategies_pick_their_blocks(){
 // Free blocks after shrinking the PSP: 0x100 (between two allocations) and the tail.
 let setup=|strategy:u8|{let mut p=fresh();call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0x100,0);let a=p.cpu.regs[0]as u16;call(&mut p,0x48,0x10,0);call(&mut p,0x49,0,a);p.dos.alloc_strategy=strategy;p};
 // First fit and best fit both choose the 0x100 hole for 0x80 paragraphs.
 for strategy in [0,1,0x81]{let mut p=setup(strategy);call(&mut p,0x48,0x80,0);assert!(!cf(&p));assert_eq!(p.cpu.regs[0]as u16,0x2001);assert!(p.dos.arena.valid(&p.ram));}
 // Last fit takes the top of the tail block: it ends at A000.
 let mut p=setup(2);call(&mut p,0x48,0x80,0);assert!(!cf(&p));assert_eq!(p.cpu.regs[0]as u16,0xa000-0x80);assert!(p.dos.arena.valid(&p.ram));
 // For 0x100 paragraphs the old hole is an exact fit, smaller than the tail.
 let mut p=setup(1);call(&mut p,0x48,0x100,0);assert_eq!(p.cpu.regs[0]as u16,0x2001);
}
/// INT 21h with AX and BX; returns (AX, CF).
fn int21(p:&mut Loaded,ax:u16,bx:u16)->(u16,bool){
 low(&mut p.cpu.regs[0],ax);low(&mut p.cpu.regs[3],bx);let mut out=Vec::new();
 assert_eq!(service(p,0x21,&mut out),Ok(None));(p.cpu.regs[0]as u16,cf(p))
}
#[test]
fn dos_umb_layout_with_the_xms_driver(){
 let p=load(&[0x90],"umb.com").unwrap();
 // Conventional memory ends at 9FFFh, where the system MCB ("SC", owner 8)
 // ends the visible chain and spans A000-C7FF; upper memory is one free block.
 assert_eq!(&p.dos.arena.blocks[1..],&[Block{header:PSP-1,size:0x8fff,owner:PSP},Block{header:SC,size:0x2800,owner:8},Block{header:UMB_FIRST,size:0x27ff,owner:0}]);
 assert!(p.dos.arena.valid(&p.ram));assert!(!p.dos.arena.linked());
 let byte=|a:u32|p.ram.load(a,MemWidth::Byte).unwrap();let word=|a:u32|p.ram.load(a,MemWidth::Word).unwrap();
 assert_eq!((byte(0x9fff0),word(0x9fff1),&p.ram.bytes()[0x9fff8..0x9fffa]),(u32::from(b'Z'),8,&b"SC"[..]));
 assert_eq!((byte(0xc8000),word(0xc8001),word(0xc8003)),(u32::from(b'Z'),0,0x27ff));
 assert_eq!((byte(LINK_FLAG),word(UPPER_ROOT)),(0,0x9fff));
 assert_eq!((p.dos.allocated_end,word(u32::from(PSP)*16+2)),(0x9fff0,0x9fff));
 assert_eq!(p.image.system[UMB_CODE_SLOT],(0xc8000,0xf0000));
 // Upper memory is RAM; the video BIOS area and F000 stay ROM.
 let mut p=p;p.ram.store(0xc9000,MemWidth::Byte,0x5a).unwrap();assert_eq!(p.ram.bytes()[0xc9000],0x5a);
 p.ram.store(0xc7fff,MemWidth::Byte,0x5a).unwrap();assert_ne!(p.ram.bytes()[0xc7fff],0x5a);
 p.ram.store(0xf0000,MemWidth::Byte,0x5a).unwrap();assert_ne!(p.ram.bytes()[0xf0000],0x5a);
}
#[test]
fn dos_umb_link_and_allocation_strategies(){
 let mut p=load(&[0x90],"umb.com").unwrap();call(&mut p,0x4a,0x1000,PSP);
 // Unlinked: high-first and high-only strategies still allocate low.
 for strategy in [0x80,0x40]{assert_eq!(int21(&mut p,0x5801,strategy),(0x5801,false));call(&mut p,0x48,0x10,0);assert!(!cf(&p));assert!((p.cpu.regs[0]as u16)<SC,"{strategy:x}");}
 assert_eq!(int21(&mut p,0x5802,0),(0x5800,false));
 // Link: the system MCB becomes 'M' and the list of lists says so.
 assert_eq!(int21(&mut p,0x5803,1),(0x5803,false));assert_eq!(int21(&mut p,0x5802,0),(0x5801,false));
 assert_eq!((p.ram.bytes()[0x9fff0],p.ram.bytes()[LINK_FLAG as usize]),(b'M',1));assert!(p.dos.arena.valid(&p.ram));
 // 80h: upper first; 40h: upper only (too big: error 8, BX = largest upper);
 // 00h: address order, so low first; 82h: last fit in upper memory.
 int21(&mut p,0x5801,0x80);call(&mut p,0x48,0x100,0);assert_eq!(p.cpu.regs[0]as u16,UMB_FIRST+1);
 int21(&mut p,0x5801,0x40);call(&mut p,0x48,0x3000,0);assert!(cf(&p));assert_eq!((p.cpu.regs[0]as u16,p.cpu.regs[3]as u16),(8,0x27ff-0x101));
 int21(&mut p,0x5801,0x00);call(&mut p,0x48,0x10,0);assert!((p.cpu.regs[0]as u16)<SC);
 int21(&mut p,0x5801,0x82);call(&mut p,0x48,0x10,0);assert_eq!(p.cpu.regs[0]as u16,UMB_END-0x10);
 // 80h falls back to conventional memory when no upper block fits.
 int21(&mut p,0x5801,0x80);call(&mut p,0x48,0x3000,0);assert!(!cf(&p));assert!((p.cpu.regs[0]as u16)<SC);
 assert!(p.dos.arena.valid(&p.ram));
 // Unlink: 'Z' again; the upper blocks keep their owners.
 assert_eq!(int21(&mut p,0x5803,0),(0x5803,false));assert_eq!((p.ram.bytes()[0x9fff0],p.ram.bytes()[LINK_FLAG as usize]),(b'Z',0));
 assert!(p.dos.arena.valid(&p.ram));assert_eq!(p.dos.arena.block_owner(UMB_FIRST+1),Some(PSP));
 // BX other than 0 or 1 is invalid function.
 assert_eq!(int21(&mut p,0x5803,2),(1,true));
}
#[test]
fn dos_umb_removed_without_the_xms_driver(){
 let mut p=load(&[0x90],"umb.com").unwrap();p.select_xms(false).unwrap();
 assert_eq!(p.dos.arena.blocks,vec![Block{header:ENV-1,size:ENV_PARAS,owner:PSP},Block{header:PSP-1,size:0x9000,owner:PSP}]);assert!(p.dos.arena.valid(&p.ram));
 assert_eq!((p.dos.allocated_end,p.ram.load(u32::from(PSP)*16+2,MemWidth::Word).unwrap()),(0xa0000,0xa000));
 assert_eq!(p.image.system[UMB_CODE_SLOT],(0,0));
 // C800-EFFF is the empty FFh bus again; stores there are dropped.
 p.ram.store(0xc9000,MemWidth::Byte,0x5a).unwrap();assert_eq!(p.ram.bytes()[0xc9000],0xff);
 assert_eq!((p.ram.bytes()[LINK_FLAG as usize],p.ram.load(UPPER_ROOT,MemWidth::Word).unwrap()),(0,0xffff));
 assert_eq!(int21(&mut p,0x5803,1),(1,true));
 // And back: the same layout as a fresh load.
 p.select_xms(true).unwrap();assert_eq!(p.dos.arena,load(&[0x90],"umb.com").unwrap().dos.arena);
 // Upper memory in use cannot be taken away.
 int21(&mut p,0x5803,1);int21(&mut p,0x5801,0x40);call(&mut p,0x4a,0x1000,PSP);call(&mut p,0x48,0x10,0);assert!(!cf(&p));
 assert!(p.select_xms(false).is_err());
}
