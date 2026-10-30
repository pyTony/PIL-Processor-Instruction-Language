use super::*;
#[test]
fn dos_vectors_round_trip_all_entries_and_preserve_flags_and_upper_bx(){
 for vector in 0..=255u32{
  let mut p=load(&[0xcd,0x21],"vectors.com").unwrap();
  let ds=SegmentRegister::Ds.index();p.image.selectors[ds]=0xfedc;p.selectors[ds]=0xfedc;
  p.image.segments[ds]=SegmentCache::real_mode(0xfedc);
  p.cpu.regs[0]=0xabcd2500|vector;p.cpu.regs[2]=0x9876ffff;p.cpu.regs[3]=0x12345678;
  p.cpu.eflags=0xad7;p.cpu.undefined_flags=u32::MAX;
  let old=p.cpu.clone();let image=p.image.clone();let mut out=vec![65];
  assert_eq!(service(&mut p,0x21,&mut out).unwrap(),None);
  assert_eq!(p.cpu,old);assert_eq!(p.image,image);
  assert_eq!(p.ram.load(vector*4,MemWidth::Dword).unwrap(),0xfedcffff);
  p.cpu.regs[0]=0xabcd3500|vector;let mut want=p.cpu.clone();want.regs[3]=0x1234ffff;
  let ram=p.ram.clone();assert_eq!(service(&mut p,0x21,&mut out).unwrap(),None);
  assert_eq!(p.cpu,want);assert_eq!(p.ram,ram);assert_eq!(out,vec![65]);
  let es=SegmentRegister::Es.index();assert_eq!(p.selectors[es],0xfedc);
  assert_eq!(p.image.selectors[es],0xfedc);assert_eq!(p.image.segments[es],SegmentCache::real_mode(0xfedc));
 }
}
#[test]
fn dos_vector_faults_leave_cpu_ram_and_segments_unchanged(){
 for ah in [0x25,0x35]{for length in [1020,1021,1022,1023]{
  let mut p=load(&[0xcd,0x21],"vectors.com").unwrap();p.cpu.regs[0]=(ah<<8)|255;p.ram=Ram::new(length);
  let old=p.clone();let mut out=vec![65];assert!(service(&mut p,0x21,&mut out).is_err());
  assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);assert_eq!(p.image,old.image);assert_eq!(p.selectors,old.selectors);assert_eq!(out,vec![65]);
 }}
 for byte in 0..4{
  let mut p=load(&[0xcd,0x21],"vectors.com").unwrap();p.cpu.regs[0]=0x25ff;p.cpu.regs[2]=0x1234;
  p.ram.protect(1020+byte,1).unwrap();let old=p.clone();let mut out=vec![];
  assert!(service(&mut p,0x21,&mut out).is_err());assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);assert_eq!(p.image,old.image);
 }
}
#[test]
fn dos_vector_bridge_retires_and_reads_guest_ivt_edits(){
 let mut p=load(&[0xcd,0x21,0x62],"vectors.com").unwrap();p.cpu.regs[0]=0x35ff;
 p.ram.store(1020,MemWidth::Dword,0x43218765).unwrap();
 let r=run(p,10).unwrap();assert!(matches!(r.stop,Stop::MissingEncoding{..}));
 assert_eq!((r.completed,r.pil_steps,r.dos_calls),(1,0,1));assert_eq!(r.loaded.cpu.eip,0x102);
 assert_eq!(r.loaded.cpu.regs[3],0x8765);assert_eq!(r.loaded.image.selectors[0],0x4321);
 let mut p=load(&[0xcd,0x21,0xb8,0,0x4c,0xcd,0x21],"vectors.com").unwrap();p.cpu.regs[0]=0x2500;p.cpu.regs[2]=0x5678;
 let r=run(p,10).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!((r.completed,r.pil_steps,r.dos_calls),(3,1,2));
 assert_eq!(r.loaded.ram.load(0,MemWidth::Dword).unwrap(),0x10005678);
}
fn exe(module:&[u8],relocs:&[(u16,u16)])->Vec<u8>{
    let hsize=((28+relocs.len()*4+15)/16)*16;let mut h=vec![0;hsize];h[..2].copy_from_slice(b"MZ");let size=hsize+module.len();
    put_word(&mut h,2,(size%512)as u16);put_word(&mut h,4,((size+511)/512)as u16);
    put_word(&mut h,6,relocs.len()as u16);put_word(&mut h,8,(hsize/16)as u16);put_word(&mut h,12,0xffff);put_word(&mut h,16,0xfffe);put_word(&mut h,24,28);
    for(i,(offset,seg))in relocs.iter().enumerate(){put_word(&mut h,28+i*4,*offset);put_word(&mut h,30+i*4,*seg);}
    h.extend(module);h
}
#[test]
fn original_com_hello_loads_psp_stack_and_runs_to_dos_exit(){
    let bytes=hello("com").unwrap();let p=load(&bytes,"hello.com").unwrap();assert_eq!(p.format,Format::Com);
    assert_eq!(p.selectors,[PSP;6]);assert_eq!(p.cpu.eip,0x100);assert_eq!(p.cpu.regs[4],0xfffe);
    let base=u32::from(PSP)*16;assert_eq!(p.ram.load(base,MemWidth::Word).unwrap(),0x20cd);
    assert_eq!(p.ram.load(base+0xfffe,MemWidth::Word).unwrap(),0);assert_eq!(p.ram.load(base+0x80,MemWidth::Word).unwrap(),0x0d00);
    let r=run(p,100).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"Hello, DOS!\r\n");assert_eq!((r.completed,r.pil_steps,r.dos_calls),(5,3,2));
}
#[test]
fn original_exe_hello_uses_cs_ip_zero_and_psp_data_segment(){
    let p=load(&hello("exe").unwrap(),"hello.exe").unwrap();assert_eq!(p.format,Format::Mz);
    assert_eq!(p.cpu.eip,0);assert_eq!(p.selectors[1],LOAD);assert_eq!(p.selectors[3],PSP);assert_eq!(p.image.start,0);
    let r=run(p,100).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"Hello, DOS!\r\n");assert_eq!(r.pil_steps,3);
}
#[test]
fn mz_relocations_are_word_wrapping_ordered_and_relative_to_load_module(){
    let mut module=vec![0x90;64];put_word(&mut module,0,0xfff0);put_word(&mut module,18,0x1234);
    let mut b=exe(&module,&[(0,0),(2,1),(0,0)]);put_word(&mut b,22,1);put_word(&mut b,20,3);put_word(&mut b,14,2);put_word(&mut b,16,0x40);
    let before=b.clone();let p=load(&b,"reloc.exe").unwrap();assert_eq!(b,before);assert_eq!(p.relocations,3);
    assert_eq!(p.ram.load(p.module_base,MemWidth::Word).unwrap(),0x2010);assert_eq!(p.ram.load(p.module_base+18,MemWidth::Word).unwrap(),0x2244);
    assert_eq!((p.selectors[1],p.cpu.eip,p.selectors[2],p.cpu.regs[4]),(LOAD+1,3,LOAD+2,0x40));
}
#[test]
fn malformed_mz_headers_and_relocations_are_rejected(){
    let good=exe(&[0x90;32],&[(0,0)]);let mut bad=Vec::new();
    for(at,n)in [(2,512),(4,0),(4,10),(8,0),(8,0xffff),(24,27),(24,0xfffe),(28,32),(26,1),(20,33),(22,0xffff)]{
        let mut b=good.clone();put_word(&mut b,at,n);bad.push(b);
    }
    bad.push(b"MZ".to_vec());
    for b in bad{assert!(load(&b,"bad.exe").is_err(),"{b:02x?}");}
    assert!(load(&[0x90],"bad.EXE").is_err());assert!(load(&[],"empty.com").is_err());assert!(load(&vec![0;MAX_FILE+1],"big.com").is_err());
}
#[test]
fn mz_allocation_and_stack_are_bounded_and_trailing_bytes_are_not_loaded(){
    let good=exe(&[0x90;32],&[]);
    for(min,max,ss,sp)in [(0xffff,0xffff,0,0),(2,1,0,0),(0,0,0,0),(0,1,0x9000,0),(0,1,0,0xfffe)]{
        let mut b=good.clone();put_word(&mut b,10,min);put_word(&mut b,12,max);put_word(&mut b,14,ss);put_word(&mut b,16,sp);assert!(load(&b,"bad.exe").is_err());
    }
    let mut b=good;put_word(&mut b,12,1);put_word(&mut b,16,0x20);b.extend([0xcc;8]);let p=load(&b,"extra.exe").unwrap();assert_eq!(p.trailing_bytes,8);assert_eq!(p.module_bytes,32);assert_eq!(p.allocated_end,u32::from(LOAD)*16+48);
    let mut full=vec![0x90;512];full[..32].copy_from_slice(&exe(&[0x90],&[])[..32]);put_word(&mut full,2,0);put_word(&mut full,4,1);assert_eq!(load(&full,"page.exe").unwrap().module_bytes,480);
}
#[test]
fn windows_new_format_signatures_are_rejected_and_zm_is_accepted(){
    for sig in [b"NE",b"LE",b"LX",b"PE",b"W3",b"W4"]{
        let mut b=vec![0;128];b[..32].copy_from_slice(&exe(&[0x90],&[])[..32]);put_word(&mut b,2,128);put_word(&mut b,8,4);b[60..64].copy_from_slice(&64u32.to_le_bytes());b[64..66].copy_from_slice(sig);assert!(load(&b,"win.exe").unwrap_err().contains("Windows"));
    }
    let mut b=hello("exe").unwrap();b[..2].copy_from_slice(b"ZM");assert!(load(&b,"old.exe").is_ok());
}
#[test]
fn missing_cpu_encoding_stops_without_skipping_or_modifying_state(){
    // Typical startup now loads DS before reaching an intentionally unsupported opcode.
    let mut b=exe(&[0xb8,0,0,0x8e,0xd8,0x62],&[(1,0)]);put_word(&mut b,16,0x100);
    let r=run(load(&b,"startup.exe").unwrap(),100).unwrap();assert!(matches!(r.stop,Stop::MissingEncoding{ref key,..}if key=="opcode 62"));assert_eq!(r.completed,2);assert_eq!(r.loaded.cpu.eip,5);assert_eq!(r.loaded.cpu.regs[0],u32::from(LOAD));assert_eq!(r.file_offset,Some(37));assert_eq!(r.loaded.selectors[3],LOAD);assert_eq!(r.stop_bytes[0],0x62);
}
#[test]
fn unsupported_dos_calls_and_monitor_stops_are_not_cpu_gaps(){
    let b=[0xb4,0x5f,0xcd,0x21];let r=run(load(&b,"service.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Service(_)));assert_eq!(r.loaded.cpu.eip,0x102);assert_eq!(r.completed,1);assert_eq!(r.dos_calls,0);
    let r=run(load(&[0xfa,0xf4],"halt.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)));
    let r=run(load(&[0x9a,0,0,0,0xa0],"boundary.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Boundary(_)));assert_eq!(r.completed,1);
}
#[test]
fn code_written_past_the_file_runs_inside_the_program_block(){
    // Store MOV AX,4C00 / INT 21 at 200h (past the 20-byte file) and jump there, as unpackers do.
    let code=[0xc7,0x06,0x00,0x02,0xb8,0x00,0xc7,0x06,0x02,0x02,0x4c,0xcd,0xc6,0x06,0x04,0x02,0x21,0xe9,0xec,0x00];
    let r=run(load(&code,"unpack.com").unwrap(),20).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.completed,6);
    // An MZ program's fetch window is its whole block, PSP through allocation end.
    let p=load(&hello("exe").unwrap(),"hello.exe").unwrap();assert_eq!((p.image.fetch_base,p.image.fetch_base+p.image.end),(u32::from(PSP)*16,p.allocated_end));
}
#[test]
fn loops_are_budgeted_and_psp_int20_can_be_reached(){
    let p=load(&[0xeb,0xfe],"loop.com").unwrap();let r=run(p,37).unwrap();assert_eq!(r.stop,Stop::Budget);assert_eq!(r.completed,37);assert_eq!(r.instructions.get("JMP"),Some(&37));
    let r=run(load(&[0xe9,0xfd,0xfe],"psp.com").unwrap(),10).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.completed,2);
    assert!(run(load(&[0x90],"budget.com").unwrap(),0).is_err());
}
#[test]
fn unterminated_or_oversized_dos_output_is_all_or_none(){
    let b=[0xba,0,2,0xb4,9,0xcd,0x21];let r=run(load(&b,"badstring.com").unwrap(),100).unwrap();assert!(matches!(r.stop,Stop::Service(_)));assert!(r.output.is_empty());assert_eq!(r.loaded.cpu.eip,0x105);assert_eq!(r.loaded.cpu.regs[0],0x900);
    let b=[0xba,0xff,0xff,0xb4,9,0xcd,0x21];let mut p=load(&b,"end.com").unwrap();p.ram.store(u32::from(PSP)*16+0xffff,MemWidth::Byte,b'$' as u32).unwrap();assert_eq!(run(p,100).unwrap().output.len(),0);
}
#[test]
fn corpus_report_ranks_first_blockers_and_keeps_service_failures_separate(){
    use std::time::{SystemTime,UNIX_EPOCH};let dir=std::env::temp_dir().join(format!("pil-dos-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&dir).unwrap();let mut paths=Vec::new();
    for(name,bytes)in [("hello world.com",hello("com").unwrap()),("gap1.com",vec![0x62]),("gap2.com",vec![0x62]),("service.com",vec![0xcd,0x10])]{let p=dir.join(name);std::fs::write(&p,bytes).unwrap();paths.push(p.to_str().unwrap().to_owned());}
    let text=report(&paths,100).unwrap();assert!(text.contains(&format!("DOS executable probe v{}; step budget=100 per program",env!("CARGO_PKG_VERSION"))));assert!(text.contains("SUMMARY imported=4/4 exited=1"));assert!(text.contains("2 opcode 62"));assert!(text.contains("Hello, DOS!\\r\\n"));assert!(text.contains("Service("));assert!(report(&[],100).is_err());assert!(report(&paths,0).is_err());std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mz_negative_relative_segments_wrap_before_entry_and_stack_validation(){
    let mut bytes=hello("exe").unwrap();put_word(&mut bytes,22,0xfff0);put_word(&mut bytes,20,0x100);put_word(&mut bytes,14,0xfff0);
    // With CS=DS=PSP, hello's data offset10C still names the original string.
    let p=load(&bytes,"wrapped.exe").unwrap();assert_eq!(p.selectors[1],PSP);assert_eq!(p.selectors[2],PSP);assert_eq!((p.image.fetch_base,p.image.start),(u32::from(PSP)*16,0));assert_eq!(p.cpu.eip,0x100);
    let r=run(p,100).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"Hello, DOS!\r\n");
    put_word(&mut bytes,20,0xff);assert!(load(&bytes,"outside.exe").is_err());
}

#[test]
fn real_mode_cs_writes_work_but_ram_permissions_and_segment_limits_remain(){
 // CS:C6 changes a following INC AX into INC CX, then it executes.
 let bytes=[0x2e,0xc6,0x06,8,1,0x41,0xeb,0,0x40,0xb8,0,0x4c,0xcd,0x21];
 let r=run(load(&bytes,"selfmod.com").unwrap(),30).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.loaded.cpu.regs[1],1);assert_eq!(r.loaded.ram.load(0x10108,MemWidth::Byte).unwrap(),0x41);
 let mut p=load(&bytes,"readonly.com").unwrap();p.ram.protect(0x10108,1).unwrap();let old=p.cpu.clone();let ram=p.ram.clone();let r=run(p,30).unwrap();assert!(matches!(r.stop,Stop::Fault(_)));assert_eq!(r.loaded.cpu,old);assert_eq!(r.loaded.ram,ram);
 let mut mz_code=bytes;mz_code[4]=0;let r=run(load(&exe(&mz_code,&[]),"selfmod.exe").unwrap(),30).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.loaded.cpu.regs[1],1);assert_eq!(r.loaded.ram.load(0x10108,MemWidth::Byte).unwrap(),0x41);
 let bytes=[0x2e,0xc7,0x06,0xff,0xff,1,0];let r=run(load(&bytes,"limit.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Fault(_)));assert_eq!(r.completed,0);
}

#[test]
fn dos_version_queries_preserve_upper_registers_flags_and_use_psp_override(){
 for selector in [0u8,1,2,0x3f,0xff]{for version in [0x1606u32,0x0705]{
  let mut p=load(&[0xcd,0x21],"version.com").unwrap();assert_eq!(p.ram.load(u32::from(PSP)*16+0x40,MemWidth::Word).unwrap(),0x1606);p.ram.store(u32::from(PSP)*16+0x40,MemWidth::Word,version).unwrap();p.cpu.regs=[0xabcd3000|u32::from(selector),0x12345678,0x87654321,0xdeadbeef,0xfffe,5,6,7];p.cpu.eflags=0xad7;p.cpu.undefined_flags=u32::MAX;let old=p.cpu.clone();let oldram=p.ram.clone();let mut out=vec![];
  assert_eq!(service(&mut p,0x21,&mut out).unwrap(),None);let mut want=old;want.regs[0]=0xabcd0000|version;want.regs[1]=0x12340000;want.regs[3]=0xdead0000|if selector==1{0}else{0xff00};assert_eq!(p.cpu,want);assert_eq!(p.ram,oldram);assert!(out.is_empty());
 }}
}
#[test]
fn dos_version_query_retires_bridge_then_reaches_next_blocker_or_exit(){
 let r=run(load(&[0xb8,0,0x30,0xcd,0x21,0x62],"version.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::MissingEncoding{ref key,..}if key=="opcode 62"));assert_eq!((r.completed,r.pil_steps,r.dos_calls),(2,1,1));assert_eq!(r.loaded.cpu.regs[0],0x1606);
 let r=run(load(&[0xb8,1,0x30,0xcd,0x21,0xb8,0,0x4c,0xcd,0x21],"version.com").unwrap(),10).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!((r.completed,r.pil_steps,r.dos_calls),(4,2,2));
}
#[test]
fn dos_version_failed_psp_read_changes_no_registers_or_output(){
 let mut p=load(&[0xcd,0x21],"version.com").unwrap();p.cpu.regs[0]=0x3000;p.ram=Ram::new(u32::from(PSP)as usize*16+0x41);let old=p.cpu.clone();let oldram=p.ram.clone();let mut out=vec![65];assert!(service(&mut p,0x21,&mut out).is_err());assert_eq!(p.cpu,old);assert_eq!(p.ram,oldram);assert_eq!(out,vec![65]);
}

/// RAM equal outside the active text page and its cursor word.
fn same_outside_screen(a:&Ram,b:&Ram)->bool{
 let(x,y)=(a.bytes(),b.bytes());x.len()==y.len()&&x.iter().zip(y).enumerate().all(|(i,(u,v))|u==v||(0xb8000..0xb8fa0).contains(&i)||(0x450..0x452).contains(&i))
}
#[test]
fn dos_character_output_preserves_other_state_and_returns_character(){
 for byte in 0..=255u32{
  let mut p=load(&[0xcd,0x21],"putchar.com").unwrap();p.cpu.regs[0]=0xabcd02ff;p.cpu.regs[2]=0x12345600|byte;p.cpu.eflags=0xed7;p.cpu.undefined_flags=u32::MAX;
  let mut want=p.cpu.clone();want.regs[0]=0xabcd0200|if byte==9{32}else{byte};let ram=p.ram.clone();let image=p.image.clone();let mut out=vec![];
  service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu,want);assert!(same_outside_screen(&p.ram,&ram));assert_eq!(p.image,image);assert_eq!(out,if byte==9{vec![32;8]}else{vec![byte as u8]});
  if !matches!(byte,7..=10|13){assert_eq!(p.ram.load(0xb8000,MemWidth::Word).unwrap(),0x0700|byte);assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),1);}
 }
}
#[test]
fn dos_tabs_use_column_after_text_cr_lf_backspace_and_bell(){
 for (prior,count) in [(b"".as_slice(),8),(b"a".as_slice(),7),(b"1234567".as_slice(),1),(b"12345678".as_slice(),8),(b"abc\r".as_slice(),8),(b"abc\n".as_slice(),5),(b"abc\x08".as_slice(),6),(b"abc\x07".as_slice(),5),(b"a\t".as_slice(),8)]{
  let mut p=load(&[0xcd,0x21],"tab.com").unwrap();p.cpu.regs[0]=0x0200;p.cpu.regs[2]=9;let mut out=prior.to_vec();let mut want=out.clone();want.extend(vec![32;count]);service(&mut p,0x21,&mut out).unwrap();assert_eq!(out,want);assert_eq!(p.cpu.regs[0],0x0220);
 }
}
#[test]
fn dos_character_output_limit_is_all_or_none_including_tab_expansion(){
 for (length,byte,ok) in [(MAX_OUTPUT-1,b'A',true),(MAX_OUTPUT,b'A',false),(MAX_OUTPUT-1,9,true),(MAX_OUTPUT,9,false)]{
  let mut p=load(&[0xcd,0x21],"limit.com").unwrap();p.cpu.regs[0]=0x02ff;p.cpu.regs[2]=u32::from(byte);let old=p.cpu.clone();let image=p.image.clone();let ram=p.ram.clone();let mut out=vec![b'x';length];let before=out.clone();
  assert_eq!(service(&mut p,0x21,&mut out).is_ok(),ok);if ok{assert_eq!(out.len(),MAX_OUTPUT);assert!(same_outside_screen(&p.ram,&ram));}else{assert_eq!(out,before);assert_eq!(p.cpu,old);assert_eq!(p.ram,ram);}assert_eq!(p.image,image);
 }
 // A tab after CR needs8 bytes even with only one remaining output byte.
 let mut p=load(&[0xcd,0x21],"limit.com").unwrap();p.cpu.regs[0]=0x0200;p.cpu.regs[2]=9;let mut out=vec![b'x';MAX_OUTPUT-2];out.push(13);let before=out.clone();let old=p.cpu.clone();assert!(service(&mut p,0x21,&mut out).is_err());assert_eq!(out,before);assert_eq!(p.cpu,old);
}
#[test]
fn dos_switch_query_returns_fixed_slash_and_rejects_other_subfunctions(){
 for sub in 0..=255u32{
  let mut p=load(&[0xcd,0x21],"switch.com").unwrap();p.cpu.regs[0]=0xabcd3700|sub;p.cpu.regs[2]=0x12345678;p.cpu.eflags=0xed7;p.cpu.undefined_flags=u32::MAX;let old=p.cpu.clone();let image=p.image.clone();let ram=p.ram.clone();let mut out=vec![65];
  let result=service(&mut p,0x21,&mut out);if sub==0{assert!(result.is_ok());let mut want=old;want.regs[2]=0x1234562f;assert_eq!(p.cpu,want);}else{assert!(result.is_err());assert_eq!(p.cpu,old);}assert_eq!(p.ram,ram);assert_eq!(p.image,image);assert_eq!(out,vec![65]);
 }
}
#[test]
fn dos_switch_and_putchar_bridge_reach_exit_and_account_for_budget(){
 let bytes=[0xb8,0,0x37,0xcd,0x21,0xb4,2,0xcd,0x21,0xb2,9,0xcd,0x21,0xb8,0,0x4c,0xcd,0x21];
 let r=run(load(&bytes,"switch.com").unwrap(),20).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"/       ");assert_eq!((r.completed,r.pil_steps,r.dos_calls),(8,4,4));
 let r=run(load(&bytes,"switch.com").unwrap(),2).unwrap();assert_eq!(r.stop,Stop::Budget);assert_eq!((r.completed,r.pil_steps,r.dos_calls),(2,1,1));assert_eq!(r.loaded.cpu.regs[2]&255,47);
}

#[test]
fn host_output_provider_failures_do_not_retire_or_modify_cpu_or_ram(){
 use crate::host_io::{CaptureOutput,Output};
 for character in [false,true]{for disconnected in [false,true]{
  let bytes=if character{vec![0xb4,2,0xb2,b'A',0xcd,0x21]}else{vec![0xba,7,1,0xb4,9,0xcd,0x21,b'A',b'$',0]};
  let mut p=load(&bytes,"provider.com").unwrap();p.cpu.regs[0]=if character{0x0200}else{0x0900};p.cpu.regs[2]=if character{65}else{0x107};
  p.cpu.eip=if character{0x104}else{0x105};let old=p.clone();let mut output=CaptureOutput::new(if disconnected{16}else{0});if disconnected{output.disconnect();}
  let r=run_with_output(p,20,&mut output).unwrap();assert!(matches!(r.stop,Stop::Service(_)));assert_eq!((r.completed,r.pil_steps,r.dos_calls),(0,0,0));assert_eq!(r.loaded.cpu,old.cpu);assert_eq!(r.loaded.ram,old.ram);assert_eq!(r.loaded.image,old.image);assert!(r.output.is_empty());assert!(output.bytes().is_empty());
 }}
}
#[test]
fn custom_headless_provider_matches_default_dos_run(){
 use crate::host_io::{CaptureOutput,Output};
 let p=load(&hello("com").unwrap(),"hello.com").unwrap();let expected=run(p.clone(),100).unwrap();let mut io=CaptureOutput::new(64);let actual=run_with_output(p,100,&mut io).unwrap();assert_eq!(actual.stop,expected.stop);assert_eq!(actual.loaded.cpu,expected.loaded.cpu);assert_eq!(actual.loaded.ram,expected.loaded.ram);assert_eq!(actual.output,expected.output);assert_eq!(io.bytes(),expected.output);
}
#[test]
fn bios_timer_advances_deterministically_updates_bda_and_rolls_over(){
 let mut p=load(&[0xcd,0x1a],"timer.com").unwrap();
 p.cpu.regs=[0xabcd00ff,0x12345678,0x87654321,3,4,5,6,7];p.cpu.eflags=0x203;p.cpu.undefined_flags=1;
 let mut out=vec![];
 service(&mut p,0x1a,&mut out).unwrap();
 assert_eq!((p.cpu.regs[0],p.cpu.regs[1],p.cpu.regs[2]),(0xabcd0000,0x12340000,0x87650000));
 assert_eq!(p.cpu.eflags&1,0);assert_eq!(p.cpu.undefined_flags&1,0);
 assert_eq!(p.ram.load(0x046c,MemWidth::Dword).unwrap(),0);assert_eq!(p.timer_ticks,1);
 service(&mut p,0x1a,&mut out).unwrap();
 assert_eq!((p.cpu.regs[1],p.cpu.regs[2]),(0x12340000,0x87650001));
 assert_eq!(p.ram.load(0x046c,MemWidth::Dword).unwrap(),1);assert_eq!(p.timer_ticks,2);
 p.timer_ticks=0x0018_00af;
 service(&mut p,0x1a,&mut out).unwrap();
 assert_eq!((p.cpu.regs[0]&255,p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff,p.timer_ticks),(1,0x0018,0x00af,0));
}
#[test]
fn bios_timer_write_fault_preserves_entire_service_state(){
 for fault in [0x046c,0x046f,0x0470]{
  let mut p=load(&[0xcd,0x1a],"timer.com").unwrap();
  p.timer_ticks=123;p.cpu.regs[0]=0xabcd00ff;p.cpu.eflags=0xad7;p.cpu.undefined_flags=u32::MAX;
  p.ram.store(0x046c,MemWidth::Dword,0xdeadbeef).unwrap();
  p.ram.store(0x0470,MemWidth::Byte,0x55).unwrap();
  p.ram.protect(fault,1).unwrap();
  let old=p.clone();let mut out=vec![65];
  assert!(service(&mut p,0x1a,&mut out).is_err());
  assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);assert_eq!(p.image,old.image);
  assert_eq!(p.timer_ticks,old.timer_ticks);assert_eq!(p.open_handles,old.open_handles);assert_eq!(out,vec![65]);
 }
 // A truncated BDA must also fail before touching its still-accessible tick cell.
 let mut p=load(&[0xcd,0x1a],"timer.com").unwrap();p.timer_ticks=123;
 p.ram=Ram::new(0x0470);p.ram.store(0x046c,MemWidth::Dword,0xdeadbeef).unwrap();
 let old=p.clone();let mut out=vec![];
 assert!(service(&mut p,0x1a,&mut out).is_err());
 assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);assert_eq!(p.timer_ticks,old.timer_ticks);
}
#[test]
fn dos_open_and_close_validate_paths_modes_and_tracked_psp_handles(){
 let mut p=load(&[0xcd,0x21],"files.com").unwrap();let mut out=vec![];
 let psp_base=u32::from(PSP)*16;
 for h in 0..5u32{assert_eq!(p.ram.load(psp_base+0x18+h,MemWidth::Byte).unwrap(),h);}
 // Invalid access mode (>2) returns error 0x000c
 p.cpu.regs[0]=0xabcd3d03;p.cpu.eflags=0x202;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.cpu.eflags&1,p.cpu.regs[0]),(1,0xabcd000c));
 // Empty ASCIIZ path at DS:0200 returns path-not-found (0x0003)
 p.ram.store(psp_base+0x200,MemWidth::Byte,0).unwrap();
 p.cpu.regs[0]=0xabcd3d00;p.cpu.regs[2]=0x200;p.cpu.eflags=0x202;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.cpu.eflags&1,p.cpu.regs[0]),(1,0xabcd0003));
 // Non-empty ASCIIZ path returns file-not-found (0x0002)
 p.ram.write_bytes(psp_base+0x200,b"EMMXXXX0\0").unwrap();
 p.cpu.regs[0]=0xabcd3d00;p.cpu.regs[2]=0x200;p.cpu.eflags=0x202;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.cpu.eflags&1,p.cpu.regs[0]),(1,0xabcd0002));
 // Unterminated path near segment end errors without modifying CPU
 p.cpu.regs[0]=0x3d00;p.cpu.regs[2]=0xffff;p.ram.store(psp_base+0xffff,MemWidth::Byte,b'A' as u32).unwrap();
 let old=p.cpu.clone();assert!(service(&mut p,0x21,&mut out).is_err());assert_eq!(p.cpu,old);
 // Closing open standard handle 0 succeeds once, then fails with invalid handle (0x0006) on repeat or unopened handle
 p.cpu.regs[0]=0xabcd3e00;p.cpu.regs[3]=0x12340000;p.cpu.eflags=0x203;
 service(&mut p,0x21,&mut out).unwrap();
 assert_eq!(p.cpu.eflags&1,0);assert_eq!(p.ram.load(psp_base+0x18,MemWidth::Byte).unwrap(),0xff);
 service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.cpu.eflags&1,p.cpu.regs[0]),(1,0xabcd0006));
 p.cpu.regs[0]=0xabcd3e00;p.cpu.regs[3]=5;p.cpu.eflags=0x202;
 service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.cpu.eflags&1,p.cpu.regs[0]),(1,0xabcd0006));
}

#[test]
fn dta_default_and_far_pointer_roundtrip_preserve_cpu_flags_and_memory(){
 let mut p=load(&[0xcd,0x21],"dta.com").unwrap();let mut out=vec![];
 assert_eq!(p.dta,(PSP,0x80));
 p.cpu.regs[0]=0x2f00;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!(p.cpu.regs[3],0x80);assert_eq!(p.image.selectors[0],PSP);
 p.image.selectors[3]=0xf123;p.cpu.regs[2]=0xabcdffff;p.cpu.regs[0]=0x1a00;
 p.cpu.eflags=0xad7;p.cpu.undefined_flags=0x891;let before=p.cpu.clone();let ram=p.ram.clone();
 service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu,before);assert_eq!(p.ram,ram);
 p.cpu.regs[0]=0x2f00;p.cpu.regs[3]=0x98760000;
 service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu.regs[3],0x9876ffff);
 assert_eq!(p.selectors[0],0xf123);assert_eq!(p.image.selectors[0],0xf123);
 assert_eq!(p.image.segments[0],SegmentCache::real_mode(0xf123));
 assert_eq!((p.cpu.eflags,p.cpu.undefined_flags),(0xad7,0x891));
}
#[test]
fn ioctl_device_info_validates_handles_and_preserves_unrelated_state(){
 let mut p=load(&[0xcd,0x21],"ioctl.com").unwrap();let mut out=vec![];
 for h in [0,1,2,3,4,5,19,20,65535]{
  p.cpu.regs[0]=0xabcd4400;p.cpu.regs[3]=h;p.cpu.regs[2]=0x98765432;
  p.cpu.eflags=0xad7;p.cpu.undefined_flags=0x891;
  service(&mut p,0x21,&mut out).unwrap();
  assert_eq!(p.cpu.eflags&!1,0xad6);assert_eq!(p.cpu.undefined_flags,0x890);
  if h<5{assert_eq!(p.cpu.eflags&1,0);assert_eq!(p.cpu.regs[2],0x98760000|if h<3{0xc3}else{0xc0});}
  else{assert_eq!(p.cpu.eflags&1,1);assert_eq!(p.cpu.regs[0],0xabcd0006);assert_eq!(p.cpu.regs[2],0x98765432);}
 }
 p.cpu.regs[3]=1;p.cpu.regs[0]=0x3e00;service(&mut p,0x21,&mut out).unwrap();
 p.cpu.regs[0]=0x4400;service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu.regs[0],6);
 p.cpu.regs[0]=0x4401;let old=p.clone();assert!(service(&mut p,0x21,&mut out).is_err());assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);
}
#[test]
fn cursor_shape_roundtrip_uses_bda_and_faults_atomically(){
 let mut p=load(&[0xcd,0x10],"cursor.com").unwrap();let mut out=vec![];
 assert_eq!(p.ram.load(0x460,MemWidth::Word).unwrap(),0x0607);
 p.cpu.regs[0]=0x0100;p.cpu.regs[1]=0x12342000;let before=p.cpu.clone();
 service(&mut p,0x10,&mut out).unwrap();assert_eq!(p.cpu,before);
 p.cpu.regs[0]=0x0200;p.cpu.regs[3]=0x0700;p.cpu.regs[2]=0x1827;
 let before=p.cpu.clone();service(&mut p,0x10,&mut out).unwrap();assert_eq!(p.cpu,before);
 assert_eq!(p.ram.load(0x45e,MemWidth::Word).unwrap(),0x1827);
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0);
 p.ram.protect(0x45f,1).unwrap();let old=p.clone();
 assert!(service(&mut p,0x10,&mut out).is_err());assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);
 p.cpu.regs[0]=0x0300;p.cpu.regs[3]=0x0700;p.cpu.regs[2]=0x98760000;
 service(&mut p,0x10,&mut out).unwrap();assert_eq!(p.cpu.regs[1],0x12342000);assert_eq!(p.cpu.regs[2],0x98761827);
 p.cpu.regs[0]=0x0100;p.ram.protect(0x461,1).unwrap();let old=p.clone();
 assert!(service(&mut p,0x10,&mut out).is_err());assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);
 p.cpu.regs[0]=0x0300;p.cpu.regs[3]=0x0800;let old=p.cpu.clone();assert!(service(&mut p,0x10,&mut out).is_err());assert_eq!(p.cpu,old);
}
#[test]
fn bios_screen_reads_guest_vram_and_writes_last_cell_without_moving_cursor(){
 let mut p=load(&[0xcd,0x10],"screen.com").unwrap();let mut out=vec![];
 assert_eq!(p.allocated_end,0xa0000); // video is not DOS-allocatable conventional memory
 for page in 0..8u32{
  p.cpu.regs[3]=page<<8;p.cpu.regs[0]=0x0800;service(&mut p,0x10,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0x0720);
  p.ram.store(0x450+page*2,MemWidth::Word,0x184f).unwrap();
  p.cpu.regs[0]=0xabcd0941;p.cpu.regs[1]=1;p.cpu.regs[3]=(page<<8)|0x1e;
  let old=p.cpu.clone();service(&mut p,0x10,&mut out).unwrap();assert_eq!(p.cpu,old);
  let at=0xb8000+page*4096+3998;assert_eq!(p.ram.load(at,MemWidth::Word).unwrap(),0x1e41);
  p.cpu.regs[0]=0xabcd0a42;service(&mut p,0x10,&mut out).unwrap();assert_eq!(p.ram.load(at,MemWidth::Word).unwrap(),0x1e42);
  p.ram.store(at,MemWidth::Word,0x2f58).unwrap();p.cpu.regs[0]=0xabcd0800;
  service(&mut p,0x10,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd2f58);
  assert_eq!(p.ram.load(0x450+page*2,MemWidth::Word).unwrap(),0x184f);
 }assert!(out.is_empty());
}
#[test]
fn bios_screen_repeated_write_preflights_every_cell(){
 for fault in [0xb8000,0xb8003,0xb8005]{
  let mut p=load(&[0xcd,0x10],"screen.com").unwrap();p.cpu.regs[0]=0x0941;p.cpu.regs[1]=3;p.cpu.regs[3]=0x1e;
  p.ram.protect(fault,1).unwrap();let old=p.clone();let mut out=vec![];
  assert!(service(&mut p,0x10,&mut out).is_err());assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);
 }
 for (page,pos,count) in [(8,0,1),(0,0x1900,1),(0,0x0050,1),(7,0x184f,2)]{
  let mut p=load(&[0xcd,0x10],"screen.com").unwrap();p.cpu.regs[0]=0x0941;p.cpu.regs[1]=count;p.cpu.regs[3]=page<<8;
  if page<8{p.ram.store(0x450+page*2,MemWidth::Word,pos).unwrap();}
  let old=p.clone();assert!(service(&mut p,0x10,&mut vec![]).is_err());assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);
 }
}
#[test]
fn bios_keyboard_peek_read_ring_wrap_full_and_zero_are_distinct(){
 let mut p=load(&[0xcd,0x16],"keys.com").unwrap();let mut out=vec![];
 p.cpu.regs[0]=0xabcd0100;p.cpu.eflags=0x203;p.cpu.undefined_flags=0x40;
 service(&mut p,0x16,&mut out).unwrap();assert_eq!(p.cpu.eflags,0x243);assert_eq!(p.cpu.regs[0],0xabcd0100);assert_eq!(p.cpu.undefined_flags,0);
 for round in 0..3{
  for n in 0..15{assert!(p.enqueue_key(if n==0{0}else{0x4800+n+round}).unwrap());}
  let ram=p.ram.clone();assert!(!p.enqueue_key(0x1e61).unwrap());assert_eq!(p.ram,ram);
  for n in 0..15{
   let key=if n==0{0}else{0x4800+n+round};
   for _ in 0..2{p.cpu.regs[0]=0xabcd0100;service(&mut p,0x16,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0000|u32::from(key));assert_eq!(p.cpu.eflags&0x40,0);}
   p.cpu.regs[0]=0xabcd0000;let flags=p.cpu.eflags;service(&mut p,0x16,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0000|u32::from(key));assert_eq!(p.cpu.eflags,flags);
  }
 }
}
#[test]
fn bios_keyboard_faults_and_bad_pointers_preserve_queue_and_cpu(){
 for fault in [0x41e,0x41d]{
  let mut p=load(&[0xcd,0x16],"keys.com").unwrap();p.ram.protect(fault,1).unwrap();let old=p.clone();
  assert!(p.enqueue_key(0x1e61).is_err());assert_eq!(p.ram,old.ram);assert_eq!(p.cpu,old.cpu);
 }
 let mut p=load(&[0xcd,0x16],"keys.com").unwrap();p.enqueue_key(0x4800).unwrap();p.ram.protect(0x41a,1).unwrap();let old=p.clone();
 assert!(service(&mut p,0x16,&mut vec![]).is_err());assert_eq!(p.ram,old.ram);assert_eq!(p.cpu,old.cpu);
 for bad in [0,0x1f,0x3e,0xffff]{
  let mut p=load(&[0xcd,0x16],"keys.com").unwrap();p.ram.store(0x41a,MemWidth::Word,bad).unwrap();let old=p.clone();
  let r=run(p,10).unwrap();assert!(matches!(r.stop,Stop::Service(_)));assert_eq!(r.completed,0);assert_eq!(r.loaded.cpu,old.cpu);assert_eq!(r.loaded.ram,old.ram);
 }
}
#[test]
fn bios_keyboard_wait_resumes_same_interrupt_once(){
 let p=load(&[0xcd,0x16,0x89,0xc3,0xb8,0,0x4c,0xcd,0x21],"keys.com").unwrap();let old=p.clone();
 let r=run(p,10).unwrap();assert_eq!(r.stop,Stop::WaitingForInput);assert_eq!((r.completed,r.pil_steps,r.dos_calls),(0,0,0));assert_eq!(r.loaded.cpu,old.cpu);assert_eq!(r.loaded.ram,old.ram);
 let mut p=r.loaded;assert!(p.enqueue_key(0x4800).unwrap());let r=run(p,10).unwrap();
 assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.loaded.cpu.regs[3],0x4800);assert_eq!((r.completed,r.pil_steps,r.dos_calls),(4,2,2));
}
#[test]
fn bios_keyboard_store_and_shift_status_services(){
 let mut p=load(&[0xcd,0x16],"keys.com").unwrap();let mut out=vec![];
 for n in 0..16{p.cpu.regs[0]=0xabcd0500;p.cpu.regs[1]=0x1e61;service(&mut p,0x16,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0500|u32::from(n==15));}
 p.ram.store(0x417,MemWidth::Byte,0xa5).unwrap();p.cpu.regs[0]=0xabcd0200;service(&mut p,0x16,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd02a5);
}
fn cell(p:&Loaded,page:u32,row:u32,col:u32)->u32{p.ram.load(0xb8000+page*4096+(row*80+col)*2,MemWidth::Word).unwrap()}
fn put(p:&mut Loaded,page:u32,row:u32,col:u32,value:u32){p.ram.store(0xb8000+page*4096+(row*80+col)*2,MemWidth::Word,value).unwrap();}
/// Teletype onto `page` made active; BH deliberately names another page.
fn tty(p:&mut Loaded,page:u32,ch:u8)->Result<Option<u8>,String>{if page<8{p.ram.store(0x462,MemWidth::Byte,page).unwrap();}p.cpu.regs[0]=0xabcd0e00|u32::from(ch);p.cpu.regs[3]=0x1234_0000|(((page+1)&7)<<8)|0x4f;service(p,0x10,&mut vec![])}
#[test]
fn bios_teletype_keeps_attributes_wraps_controls_and_preserves_registers(){
 for page in 0..8u32{
  let mut p=load(&[0xcd,0x10],"tty.com").unwrap();put(&mut p,page,0,0,0x1e00);
  p.cpu.eflags=0xad7;p.cpu.undefined_flags=u32::MAX;
  tty(&mut p,page,b'A').unwrap();let regs=p.cpu.clone();
  assert_eq!(cell(&p,page,0,0),0x1e41);assert_eq!(p.ram.load(0x450+page*2,MemWidth::Word).unwrap(),0x0001);
  for (ch,want) in [(7u8,0x0001),(8,0x0000),(8,0x0000),(10,0x0100),(b'B',0x0101),(13,0x0100)]{
   tty(&mut p,page,ch).unwrap();assert_eq!(p.ram.load(0x450+page*2,MemWidth::Word).unwrap(),want,"{ch}");
  }
  assert_eq!(cell(&p,page,1,0),0x0742);assert_eq!(cell(&p,page,0,1),0x0720);
  p.ram.store(0x450+page*2,MemWidth::Word,0x054f).unwrap();tty(&mut p,page,b'Z').unwrap();
  assert_eq!(cell(&p,page,5,79),0x075a);assert_eq!(p.ram.load(0x450+page*2,MemWidth::Word).unwrap(),0x0600);
  assert_eq!(p.cpu.regs[1..],regs.regs[1..]);assert_eq!(p.cpu.regs[0],0xabcd0e5a);assert_eq!((p.cpu.eflags,p.cpu.undefined_flags),(0xad7,u32::MAX));
  for other in (0..8).filter(|&o|o!=page){assert_eq!(p.ram.load(0x450+other*2,MemWidth::Word).unwrap(),0);assert_eq!(cell(&p,other,0,0),0x0720);}
 }
}
#[test]
fn bios_teletype_scrolls_its_page_with_attribute_under_cursor(){
 for (ch,start) in [(10u8,0x1805u32),(b'Q',0x184f)]{
  let mut p=load(&[0xcd,0x10],"tty.com").unwrap();
  for row in 0..25{put(&mut p,3,row,7,0x0700|(0x30+row));}put(&mut p,3,24,5,0x2f20);put(&mut p,3,24,79,0x4e20);
  p.ram.store(0x456,MemWidth::Word,start).unwrap();let other=p.ram.clone();tty(&mut p,3,ch).unwrap();
  for row in 0..24{assert_eq!(cell(&p,3,row,7),0x0700|(0x31+row));}
  let attr=if ch==10{0x2f}else{0x4e};
  for col in 0..80{assert_eq!(cell(&p,3,24,col),0x20|(attr<<8));}
  if ch==b'Q'{assert_eq!(cell(&p,3,23,79),0x4e51);}
  assert_eq!(p.ram.load(0x456,MemWidth::Word).unwrap(),if ch==10{0x1805}else{0x1800});
  for page in [0,1,2,4,5,6,7]{for c in 0..2000u32{let at=0xb8000+page*4096+c*2;assert_eq!(p.ram.load(at,MemWidth::Word).unwrap(),other.load(at,MemWidth::Word).unwrap());}}
 }
}
#[test]
fn bios_teletype_rejects_tab_bad_page_cursor_and_faults_atomically(){
 let mut p=load(&[0xcd,0x10],"tty.com").unwrap();
 for (page,ch,cursor) in [(0,9u8,0),(8,b'A',0),(0,b'A',0x1900),(0,b'A',0x0050)]{
  p.ram.store(0x462,MemWidth::Byte,0).unwrap();if page<8{p.ram.store(0x450,MemWidth::Word,cursor).unwrap();}else{p.ram.store(0x462,MemWidth::Byte,page).unwrap();}let old=p.clone();
  assert!(tty(&mut p,page,ch).is_err());assert_eq!(p.ram,old.ram);
 }
 // A bottom-row scroll checks every page cell and the cursor before writing.
 for fault in [0xb8000,0xb8f9e,0x450]{
  let mut p=load(&[0xcd,0x10],"tty.com").unwrap();p.ram.store(0x450,MemWidth::Word,0x184f).unwrap();p.ram.protect(fault,1).unwrap();
  let old=p.clone();assert!(tty(&mut p,0,b'A').is_err());assert_eq!(p.cpu.regs[0],0xabcd0e41);assert_eq!(p.ram,old.ram);
 }
}
fn scroll_call(p:&mut Loaded,ah:u32,lines:u32,attr:u32,from:u32,to:u32)->Result<Option<u8>,String>{
 p.cpu.regs[0]=0xabcd0000|(ah<<8)|lines;p.cpu.regs[3]=0x1234_0000|(attr<<8)|0x55;p.cpu.regs[1]=0x9876_0000|from;p.cpu.regs[2]=0x5432_0000|to;service(p,0x10,&mut vec![])
}
#[test]
fn bios_scroll_moves_only_window_on_active_page_and_clamps_corner(){
 for (ah,lines) in [(6u32,1u32),(6,2),(7,1),(7,3),(6,0),(7,9),(6,200)]{
  let mut p=load(&[0xcd,0x10],"scroll.com").unwrap();
  p.ram.store(0x462,MemWidth::Byte,2).unwrap();
  for page in 0..8{for row in 0..25{for col in 0..80{put(&mut p,page,row,col,(page<<12)|(row<<6)|col);}}}
  let before=p.clone();
  scroll_call(&mut p,ah,lines,0x1f,0x0203,0x0605).unwrap();
  assert_eq!(p.cpu.regs[..4],[0xabcd0000|(ah<<8)|lines,0x98760203,0x54320605,0x12341f55]);
  for page in 0..8{for row in 0..25{for col in 0..80{
   let old=cell(&before,page,row,col);let inside=page==2&&(2..=6).contains(&row)&&(3..=5).contains(&col);
   let n=if lines==0||lines>5{5}else{lines};
   let want=if !inside{old}else if ah==6{if row+n<=6{cell(&before,page,row+n,col)}else{0x1f20}}else if row>=2+n{cell(&before,page,row-n,col)}else{0x1f20};
   assert_eq!(cell(&p,page,row,col),want,"{ah} {lines} {page} {row} {col}");
  }}}
 }
 let mut p=load(&[0xcd,0x10],"scroll.com").unwrap();for row in 0..25{put(&mut p,0,row,79,0x0700|row);}
 scroll_call(&mut p,6,1,0x07,0x0000,0xffff).unwrap();for row in 0..24{assert_eq!(cell(&p,0,row,79),0x0700|(row+1));}assert_eq!(cell(&p,0,24,79),0x0720);
 for (from,to) in [(0x0500,0x0400),(0x0005,0x1804)]{let old=p.clone();scroll_call(&mut p,6,1,0x70,from,to).unwrap();assert_eq!(p.ram,old.ram);}
}
#[test]
fn bios_scroll_faults_and_bad_active_page_change_nothing(){
 let mut p=load(&[0xcd,0x10],"scroll.com").unwrap();p.ram.protect(0xb8000+(6*80+5)*2+1,1).unwrap();
 let old=p.clone();assert!(scroll_call(&mut p,6,1,0x1f,0x0203,0x0605).is_err());assert_eq!(p.ram,old.ram);
 let mut p=load(&[0xcd,0x10],"scroll.com").unwrap();p.ram.store(0x462,MemWidth::Byte,8).unwrap();
 let old=p.clone();assert!(scroll_call(&mut p,7,1,0x1f,0,0x184f).is_err());assert_eq!(p.ram,old.ram);assert_eq!(p.cpu.regs[0],0xabcd0701);
}
#[test]
fn bios_mode_page_and_query_services_use_bda_state(){
 let mut p=load(&[0xcd,0x10],"mode.com").unwrap();
 p.cpu.regs[0]=0xabcd0f00;p.cpu.regs[3]=0x12345678;service(&mut p,0x10,&mut vec![]).unwrap();
 assert_eq!((p.cpu.regs[0],p.cpu.regs[3]),(0xabcd5003,0x12340078));
 for page in 0..8u32{
  p.cpu.regs[0]=0xabcd0500|page;service(&mut p,0x10,&mut vec![]).unwrap();
  assert_eq!(p.ram.load(0x462,MemWidth::Byte).unwrap(),page);assert_eq!(p.ram.load(0x44e,MemWidth::Word).unwrap(),page*4096);
  p.cpu.regs[0]=0xabcd0f00;service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[3],0x12340078|(page<<8));
 }
 p.cpu.regs[0]=0x0508;let old=p.clone();assert!(service(&mut p,0x10,&mut vec![]).is_err());assert_eq!(p.ram,old.ram);
 p.ram.store(0x462,MemWidth::Byte,9).unwrap();p.cpu.regs[0]=0x0f00;let old=p.cpu.clone();assert!(service(&mut p,0x10,&mut vec![]).is_err());assert_eq!(p.cpu,old);
}
#[test]
fn bios_set_mode_three_resets_bda_and_clears_unless_bit7(){
 for keep in [false,true]{
  let mut p=load(&[0xcd,0x10],"mode.com").unwrap();
  for page in 0..8{put(&mut p,page,24,79,0x4e41);p.ram.store(0x450+page*2,MemWidth::Word,0x0102).unwrap();}
  for (at,value) in [(0x44e,0x3000),(0x460,0x2000)]{p.ram.store(at,MemWidth::Word,value).unwrap();}p.ram.store(0x462,MemWidth::Byte,3).unwrap();
  p.cpu.regs[0]=0xabcd0003|if keep{0x80}else{0};let regs=p.cpu.clone();service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu,regs);
  for page in 0..8{assert_eq!(cell(&p,page,24,79),if keep{0x4e41}else{0x0720});assert_eq!(p.ram.load(0x450+page*2,MemWidth::Word).unwrap(),0);}
  for (at,w,value) in [(0x449,MemWidth::Byte,3),(0x44a,MemWidth::Word,80),(0x44c,MemWidth::Word,4096),(0x44e,MemWidth::Word,0),(0x460,MemWidth::Word,0x0607),(0x462,MemWidth::Byte,0),(0x484,MemWidth::Byte,24)]{assert_eq!(p.ram.load(at,w).unwrap(),value);}
 }
 for al in [0u32,1,2,7,0x13,0x87]{let mut p=load(&[0xcd,0x10],"mode.com").unwrap();p.cpu.regs[0]=al;let old=p.clone();assert!(service(&mut p,0x10,&mut vec![]).is_err());assert_eq!(p.ram,old.ram);}
 let mut p=load(&[0xcd,0x10],"mode.com").unwrap();p.ram.protect(0xbef9f,1).unwrap();p.cpu.regs[0]=3;let old=p.clone();
 assert!(service(&mut p,0x10,&mut vec![]).is_err());assert_eq!(p.ram,old.ram);
}
fn dos(p:&mut Loaded,ax:u32,dx:u32,out:&mut Vec<u8>)->Result<Option<u8>,String>{p.cpu.regs[0]=0xabcd0000|ax;p.cpu.regs[2]=0x12340000|dx;service(p,0x21,out)}
#[test]
fn dos_output_reaches_active_page_with_screen_tab_and_atomic_faults(){
 let bytes=[0xcd,0x21,b'A',b'B',9,b'C',13,10,b'$'];
 let mut p=load(&bytes,"screen.com").unwrap();p.ram.store(0x462,MemWidth::Byte,2).unwrap();p.ram.store(0x454,MemWidth::Word,0x0003).unwrap();
 let mut out=vec![];dos(&mut p,0x0900,0x102,&mut out).unwrap();assert_eq!(out,b"AB\tC\r\n");
 let row:Vec<u32>=(0..10).map(|c|cell(&p,2,0,c)&255).collect();assert_eq!(row,[32,32,32,65,66,32,32,32,67,32]);
 assert_eq!(p.ram.load(0x454,MemWidth::Word).unwrap(),0x0100);assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0);
 // A protected screen cell or bad cursor stops before capture or registers change.
 for setup in 0..2{
  let mut p=load(&bytes,"screen.com").unwrap();if setup==0{p.ram.protect(0xb8000,1).unwrap();}else{p.ram.store(0x450,MemWidth::Word,0x1900).unwrap();}
  for (ax,dx) in [(0x0900,0x102),(0x0200,0x41),(0x0600,0x41)]{let old=p.clone();let mut out=vec![];assert!(dos(&mut p,ax,dx,&mut out).is_err());assert!(out.is_empty());assert_eq!(p.ram,old.ram);assert_eq!(p.cpu.regs[0],0xabcd0000|ax);}
 }
 // Provider failure leaves the screen untouched as well.
 let mut p=load(&bytes,"screen.com").unwrap();let mut out=vec![b'x';MAX_OUTPUT];let old=p.ram.clone();assert!(dos(&mut p,0x0200,0x41,&mut out).is_err());assert_eq!(p.ram,old);
}
#[test]
fn dos_reads_echo_extended_keys_in_two_calls_and_keep_flags(){
 let mut p=load(&[0xcd,0x21],"read.com").unwrap();let mut out=vec![];
 for key in [0x1e61,0x4800,0x2e63,0x4b00,0x2064]{assert!(p.enqueue_key(key).unwrap());}
 p.cpu.eflags=0xad7;p.cpu.undefined_flags=u32::MAX;
 for (ah,al,echo) in [(1u32,0x61u32,true),(1,0,true),(1,0x48,true),(8,0x63,false),(7,0,false),(7,0x4b,false),(8,0x64,false)]{
  let n=out.len();dos(&mut p,ah<<8,0x55,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0000|(ah<<8)|al);assert_eq!(p.cpu.regs[2],0x12340055);
  assert_eq!(out[n..],if echo{vec![al as u8]}else{vec![]});
 }
 assert_eq!((p.cpu.eflags,p.cpu.undefined_flags),(0xad7,u32::MAX));assert_eq!(p.pending_scan,None);
 assert_eq!(cell(&p,0,0,0)&255,0x61);assert_eq!(cell(&p,0,0,2)&255,0x48);assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),3);
 assert!(dos(&mut p,0x0100,0,&mut out).is_err()); // service needs input; the run loop waits first
}
#[test]
fn dos_ctrl_c_is_returned_raw_or_stops_before_any_effect(){
 for ah in [1u32,2,8,9,0x0b]{
  let mut p=load(&[0xcd,0x21,b'$'],"break.com").unwrap();p.enqueue_key(0x2e03).unwrap();let old=p.clone();let mut out=vec![];
  assert!(dos(&mut p,ah<<8,if ah==9{0x102}else{0x41},&mut out).is_err());assert!(out.is_empty());assert_eq!(p.ram,old.ram);assert_eq!(p.cpu.regs[0],0xabcd0000|(ah<<8));
 }
 for (ah,dl) in [(7u32,0),(6,0xff)]{
  let mut p=load(&[0xcd,0x21],"break.com").unwrap();p.enqueue_key(0x2e03).unwrap();let mut out=vec![];
  dos(&mut p,ah<<8,dl,&mut out).unwrap();assert_eq!(p.cpu.regs[0]&255,3);assert!(out.is_empty());assert!(!console::dos_available(&p).unwrap());
 }
}
#[test]
fn dos_direct_console_and_status_poll_without_waiting(){
 let mut p=load(&[0xcd,0x21],"poll.com").unwrap();let mut out=vec![];p.cpu.eflags=0x2;p.cpu.undefined_flags=0x40;
 dos(&mut p,0x06ff,0xff,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0600);assert_eq!(p.cpu.eflags&0x40,0x40);assert_eq!(p.cpu.undefined_flags,0);
 dos(&mut p,0x0bff,0,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0b00);
 p.enqueue_key(0x4800).unwrap();
 dos(&mut p,0x0b00,0,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0bff);
 for al in [0u32,0x48]{dos(&mut p,0x0600,0xff,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0600|al);assert_eq!(p.cpu.eflags&0x40,0);}
 dos(&mut p,0x0b00,0,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0b00);assert!(out.is_empty());
 dos(&mut p,0x0600,9,&mut out).unwrap();assert_eq!(out,vec![32;8]);assert_eq!(p.cpu.regs[0]&255,32);
 dos(&mut p,0x0600,0x41,&mut out).unwrap();assert_eq!(out[8],0x41);assert_eq!(cell(&p,0,0,8)&255,0x41);
}
#[test]
fn dos_flush_then_read_flushes_once_and_resumes_with_new_key(){
 // Type-ahead 'x' is discarded; the read waits, then returns and echoes 'y'.
 let code=[0xb8,1,0x0c,0xcd,0x21,0x88,0xc3,0xb8,0,0x4c,0xcd,0x21];
 let mut p=load(&code,"flush.com").unwrap();p.enqueue_key(0x2d78).unwrap();
 let r=run(p,20).unwrap();assert_eq!(r.stop,Stop::WaitingForInput);assert!(r.loaded.flushed);assert!(!console::dos_available(&r.loaded).unwrap());
 let mut p=r.loaded;p.enqueue_key(0x1579).unwrap();let r=run(p,20).unwrap();
 assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.loaded.cpu.regs[3]&255,0x79);assert_eq!(r.output,b"y");assert!(!r.loaded.flushed);
 // Other subfunctions flush, then AL=0; AL=06 polls the now-empty input; AL=0A is not implemented.
 for (al,dl,want) in [(0u32,0,0u32),(0x0b,0,0),(0x02,0,0),(0x06,0xff,0)]{
  let mut p=load(&[0xcd,0x21],"flush.com").unwrap();p.enqueue_key(0x2d78).unwrap();p.pending_scan=Some(0x48);let mut out=vec![];
  dos(&mut p,0x0c00|al,dl,&mut out).unwrap();assert_eq!(p.cpu.regs[0]&255,want);assert!(!console::dos_available(&p).unwrap());assert!(out.is_empty());
  if al==6{assert_eq!(p.cpu.eflags&0x40,0x40);}
 }
}
#[test]
fn dos_blocking_reads_wait_without_retiring_then_resume(){
 for ah in [1u8,7,8]{
  let code=[0xb4,ah,0xcd,0x21,0x88,0xc3,0xb8,0,0x4c,0xcd,0x21];
  let p=load(&code,"wait.com").unwrap();let r=run(p,20).unwrap();assert_eq!(r.stop,Stop::WaitingForInput);assert_eq!(r.completed,1);
  let mut p=r.loaded;p.enqueue_key(0x1e61).unwrap();let r=run(p,20).unwrap();
  assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.loaded.cpu.regs[3]&255,0x61);assert_eq!(r.output,if ah==1{b"a".to_vec()}else{vec![]});
 }
}
/// DS:offset of a line buffer with its size byte set.
fn line_buf(p:&mut Loaded,size:u32)->u32{let base=p.image.segments[SegmentRegister::Ds.index()].base+0x200;p.ram.store(base,MemWidth::Byte,size).unwrap();base}
fn line_keys(p:&mut Loaded,keys:&[u16]){for &k in keys{assert!(p.enqueue_key(k).unwrap());}}
fn line_bytes(p:&Loaded,base:u32,n:u32)->Vec<u8>{(0..n).map(|i|p.ram.load(base+i,MemWidth::Byte).unwrap()as u8).collect()}
#[test]
fn dos_line_input_edits_echoes_and_stores_count_without_cr(){
 // 'a' 'b' BS 'c' ^A F6 LF Up ^F CR: control chars as ^X, F6 as ^Z, LF only echoes, Up and ^F ignored.
 let mut p=load(&[0xcd,0x21],"line.com").unwrap();let base=line_buf(&mut p,10);
 line_keys(&mut p,&[0x1e61,0x3062,0x0e08,0x2e63,0x1e01,0x4000,0x1c0a,0x4800,0x2106,0x1c0d]);
 p.cpu.eflags=0xad7;let mut out=vec![];dos(&mut p,0x0a00,0x200,&mut out).unwrap();
 assert!(!p.input_wait);assert_eq!(p.line,None);assert_eq!(p.cpu.regs[0],0xabcd0a00);assert_eq!(p.cpu.eflags,0xad7);
 assert_eq!(out,b"ab\x08 \x08c^A^Z\r\n\r");assert_eq!(line_bytes(&p,base,7),[10,4,b'a',b'c',1,0x1a,13]);
 assert!(!console::dos_available(&p).unwrap());assert_eq!(cell(&p,0,0,0)&255,u32::from(b'a'));
}
#[test]
fn dos_line_input_backspaces_control_and_tab_columns(){
 let mut p=load(&[0xcd,0x21],"line.com").unwrap();let base=line_buf(&mut p,10);
 line_keys(&mut p,&[0x1e01,0x0e08,0x1e61,0x0f09,0x0e08,0x4b00,0x0e08,0x1c0d]);
 let mut out=b"xy".to_vec();dos(&mut p,0x0a00,0x200,&mut out).unwrap();
 let mut want=b"xy^A\x08 \x08\x08 \x08a\t".to_vec();for _ in 0..5{want.extend_from_slice(b"\x08 \x08");}
 want.extend_from_slice(b"\x08 \x08\r");assert_eq!(out,want);assert_eq!(line_bytes(&p,base,3),[10,0,13]);
}
#[test]
fn dos_line_input_full_buffer_beeps_and_escape_restarts_line(){
 let mut p=load(&[0xcd,0x21],"line.com").unwrap();let base=line_buf(&mut p,3);
 line_keys(&mut p,&[0x1e61,0x3062,0x2e63,0x011b,0x2064,0x1c0d]);
 let mut out=b"> ".to_vec();dos(&mut p,0x0a00,0x200,&mut out).unwrap();
 assert_eq!(out,b"> ab\x07\\\r\n  d\r");assert_eq!(line_bytes(&p,base,4),[3,1,b'd',13]);
 // Size 1 only takes CR; size 0 returns without reading or writing.
 let mut p=load(&[0xcd,0x21],"line.com").unwrap();let base=line_buf(&mut p,1);line_keys(&mut p,&[0x1e61,0x1c0d]);
 let mut out=vec![];dos(&mut p,0x0a00,0x200,&mut out).unwrap();assert_eq!(out,b"\x07\r");assert_eq!(line_bytes(&p,base,3),[1,0,13]);
 let mut p=load(&[0xcd,0x21],"line.com").unwrap();line_buf(&mut p,0);line_keys(&mut p,&[0x1e61]);let old=p.clone();
 let mut out=vec![];dos(&mut p,0x0a00,0x200,&mut out).unwrap();assert!(out.is_empty());assert_eq!(p.ram,old.ram);assert!(!p.input_wait);
}
#[test]
fn dos_line_input_rejects_template_keys_and_ctrl_c_keeping_line(){
 for stop in [0x3b00,0x5300,0x2e03]{
  let mut p=load(&[0xcd,0x21],"line.com").unwrap();let base=line_buf(&mut p,10);line_keys(&mut p,&[0x1e61,stop]);
  let mut out=vec![];assert!(dos(&mut p,0x0a00,0x200,&mut out).is_err());assert_eq!(out,b"a");
  assert_eq!(p.line,Some((b"a".to_vec(),0,false)));assert!(console::dos_available(&p).unwrap());assert_eq!(line_bytes(&p,base,2),[10,0]);
 }
 // A buffer crossing the segment end stops before reading a key.
 let mut p=load(&[0xcd,0x21],"line.com").unwrap();line_keys(&mut p,&[0x1e61,0x1c0d]);
 let end=p.image.segments[SegmentRegister::Ds.index()].base+0xfff0;
 p.ram.store(end,MemWidth::Byte,15).unwrap();assert!(dos(&mut p,0x0a00,0xfff0,&mut vec![]).is_err());assert!(console::dos_available(&p).unwrap());
 p.ram.store(end,MemWidth::Byte,14).unwrap();assert!(dos(&mut p,0x0a00,0xfff0,&mut vec![]).is_ok());assert!(!console::dos_available(&p).unwrap());
}
#[test]
fn dos_line_input_waits_between_keys_and_resumes_same_call(){
 // mov dx,200h; mov ah,0Ah; int 21h; mov ax,4C00h; int 21h
 let code=[0xba,0,2,0xb4,0x0a,0xcd,0x21,0xb8,0,0x4c,0xcd,0x21];
 let mut p=load(&code,"line.com").unwrap();let base=line_buf(&mut p,8);line_keys(&mut p,&[0x1e61]);
 let r=run(p,20).unwrap();assert_eq!(r.stop,Stop::WaitingForInput);assert_eq!(r.completed,2);assert_eq!(r.output,b"a");
 let mut p=r.loaded;assert_eq!(p.line,Some((b"a".to_vec(),0,false)));assert!(!p.input_wait);
 line_keys(&mut p,&[0x3062,0x1c0d]);let r=run(p,20).unwrap();
 assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"b\r");assert_eq!(r.loaded.line,None);assert_eq!(line_bytes(&r.loaded,base,5),[8,2,b'a',b'b',13]);
 // AH0C/AL0A flushes the type-ahead once, then edits keys typed after the wait.
 let code=[0xba,0,2,0xb8,0x0a,0x0c,0xcd,0x21,0xb8,0,0x4c,0xcd,0x21];
 let mut p=load(&code,"line.com").unwrap();let base=line_buf(&mut p,8);line_keys(&mut p,&[0x2d78]);
 let r=run(p,20).unwrap();assert_eq!(r.stop,Stop::WaitingForInput);assert!(r.loaded.flushed);
 let mut p=r.loaded;line_keys(&mut p,&[0x1579,0x1c0d]);let r=run(p,20).unwrap();
 assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"y\r");assert!(!r.loaded.flushed);assert_eq!(line_bytes(&r.loaded,base,4),[8,1,b'y',13]);
}
#[test]
fn bios_pointing_device_reports_function_not_supported(){
 for al in [0u32,5,7]{
  let mut p=load(&[0xcd,0x15],"mouse.com").unwrap();p.cpu.regs[0]=0x1234c200|al;p.cpu.regs[3]=0x5678_0100;p.cpu.eflags=0x2;p.cpu.undefined_flags=1;
  let ram=p.ram.clone();service(&mut p,0x15,&mut vec![]).unwrap();
  assert_eq!(p.cpu.regs[0],0x12348600|al);assert_eq!(p.cpu.regs[3],0x5678_0100);assert_eq!(p.cpu.eflags,0x3);assert_eq!(p.cpu.undefined_flags,0);assert_eq!(p.ram,ram);
 }
}
#[test]
fn mul_and_loop_forms_have_literal_results(){
 // MOV AX,FFFF; MOV CX,FFFF; MUL CX -> DX:AX=FFFE:0001, CF=OF=1.
 let mut p=load(&[0xb8,0xff,0xff,0xb9,0xff,0xff,0xf7,0xe1,0xf4],"mul.com").unwrap();p.cpu.regs[2]=0xabcd1234;p.cpu.eflags=0x2;
 let r=run(p,10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x0001);assert_eq!(r.loaded.cpu.regs[2],0xabcdfffe);assert_eq!(r.loaded.cpu.eflags&0x801,0x801);
 // MUL BL with AL=10h, BL=0Fh -> AX=00F0, CF=OF=0, DX untouched.
 let mut p=load(&[0xb0,0x10,0xb3,0x0f,0xf6,0xe3,0xf4],"mul8.com").unwrap();p.cpu.regs[0]=0x1234ff00;p.cpu.regs[2]=0x55;p.cpu.eflags=0x803;
 let r=run(p,10).unwrap();assert_eq!(r.loaded.cpu.regs[0],0x123400f0);assert_eq!(r.loaded.cpu.regs[2],0x55);assert_eq!(r.loaded.cpu.eflags&0x801,0);
 // LOOP with CX=3 runs INC AX three times and leaves the upper ECX half alone.
 let mut p=load(&[0x40,0xe2,0xfd,0xf4],"loop.com").unwrap();p.cpu.regs[1]=0x77770003;p.cpu.regs[0]=0;
 let r=run(p,20).unwrap();assert_eq!(r.loaded.cpu.regs[0],3);assert_eq!(r.loaded.cpu.regs[1],0x77770000);
 // LOOPNE stops on ZF=1 after one pass: CMP AL,AL sets ZF.
 let mut p=load(&[0x40,0x38,0xc0,0xe0,0xfb,0xf4],"loopne.com").unwrap();p.cpu.regs[1]=5;p.cpu.regs[0]=0;
 let r=run(p,20).unwrap();assert_eq!(r.loaded.cpu.regs[0],1);assert_eq!(r.loaded.cpu.regs[1],4);
}
#[test]
fn rotate_through_carry_decimal_adjust_and_pusha_have_literal_results(){
 // STC; MOV AL,81h; RCL AL,1 -> AL=03, CF=1, OF=MSB(0)^CF=1.
 let r=run(load(&[0xf9,0xb0,0x81,0xd0,0xd0,0xfa,0xf4],"rcl.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&255,0x03);assert_eq!(r.loaded.cpu.eflags&0x801,0x801);
 // CLC; MOV AX,0001; MOV CL,17; RCR AX,CL -> count 17 mod 17 = 0: AX and CF unchanged.
 let r=run(load(&[0xf8,0xb8,0x01,0x00,0xb1,0x11,0xd3,0xd8,0xfa,0xf4],"rcr.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,1);assert_eq!(r.loaded.cpu.eflags&1,0);
 // STC; MOV AX,0002; RCR AX,1 -> AX=8001, CF=0, OF=MSB(0002)^1=1.
 let r=run(load(&[0xf9,0xb8,0x02,0x00,0xd1,0xd8,0xfa,0xf4],"rcr1.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x8001);assert_eq!(r.loaded.cpu.eflags&0x801,0x800);
 // MOV AL,79h; ADD AL,35h; DAA -> AL=14h, CF=1 (79+35=114 BCD).
 let r=run(load(&[0xb0,0x79,0x04,0x35,0x27,0xfa,0xf4],"daa.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&255,0x14);assert_eq!(r.loaded.cpu.eflags&1,1);
 // MOV AL,35h; SUB AL,47h; DAS -> AL=88h, CF=1 (35-47 = -12 -> 88 with borrow).
 let r=run(load(&[0xb0,0x35,0x2c,0x47,0x2f,0xfa,0xf4],"das.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&255,0x88);assert_eq!(r.loaded.cpu.eflags&1,1);
 // PUSHA stores AX..DI below SP with the original SP in the fifth slot; POPA skips it.
 let mut p=load(&[0x60,0xfa,0xf4],"pusha.com").unwrap();p.cpu.regs=[0x1111,0x2222,0x3333,0x4444,0xfffe,0x6666,0x7777,0x8888];
 let r=run(p,10).unwrap();let ss=r.loaded.image.segments[SegmentRegister::Ss.index()].base;assert_eq!(r.loaded.cpu.regs[4],0xffee);
 let words:Vec<u32>=(0..8).map(|i|r.loaded.ram.load(ss+0xffee+i*2,MemWidth::Word).unwrap()).collect();
 assert_eq!(words,[0x8888,0x7777,0x6666,0xfffe,0x4444,0x3333,0x2222,0x1111]);
 let mut p=load(&[0x61,0xfa,0xf4],"popa.com").unwrap();p.cpu.regs[4]=0xffee;for (i,w) in words.iter().enumerate(){p.ram.store(ss+0xffee+i as u32*2,MemWidth::Word,*w).unwrap();}
 let r=run(p,10).unwrap();assert_eq!(r.loaded.cpu.regs,[0x1111,0x2222,0x3333,0x4444,0xfffe,0x6666,0x7777,0x8888]);
}
#[test]
fn near_indirect_jump_reads_register_and_cs_memory_targets(){
 // JMP CS:[0107] -> 010B (MOV AL,2A; HLT); the skipped bytes are HLT.
 let r=run(load(&[0x2e,0xff,0x26,0x07,0x01,0xf4,0xf4,0x0b,0x01,0xf4,0xf4,0xb0,0x2a,0xf4],"jmpmem.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&255,0x2a);
 // MOV BX,0106; JMP BX -> MOV AL,2B; HLT.
 let r=run(load(&[0xbb,0x06,0x01,0xff,0xe3,0xf4,0xb0,0x2b,0xf4],"jmpreg.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&255,0x2b);
}
#[test]
fn lds_and_enter_have_literal_results(){
 // LDS SI,[0110] loads SI=1234, DS=5678; then MOV AX,DS.
 let r=run(load(&[0xc5,0x36,0x10,0x01,0x8c,0xd8,0xf4,0,0,0,0,0,0,0,0,0,0x34,0x12,0x78,0x56],"lds.com").unwrap(),10).unwrap();
 assert_eq!(r.loaded.cpu.regs[6]&0xffff,0x1234);assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x5678);
 // ENTER 001E,0 with SP=FFFE, BP=ABCD: [SS:FFFC]=ABCD, BP=FFFC, SP=FFDE.
 let mut p=load(&[0xc8,0x1e,0x00,0x00,0xf4],"enter.com").unwrap();p.cpu.regs[5]=0x1111abcd;
 let sp=p.cpu.regs[4]&0xffff;let r=run(p,10).unwrap();
 assert_eq!(r.loaded.cpu.regs[5],0x11110000|(sp-2));assert_eq!(r.loaded.cpu.regs[4]&0xffff,sp-2-0x1e);
 let ss=u32::from(r.loaded.selectors[SegmentRegister::Ss.index()])*16;
 assert_eq!(r.loaded.ram.load(ss+sp-2,MemWidth::Word).unwrap(),0xabcd);
 // A non-zero nesting level is outside the profile.
 let r=run(load(&[0xc8,0x10,0x00,0x01,0xf4],"enter1.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::MissingEncoding{..}),"{:?}",r.stop);
}
#[test]
fn handle_write_psp_query_and_absent_extensions(){
 // MOV AH,40; MOV BX,1; MOV CX,3; MOV DX,0110; INT 21; HLT; ... "abc" at 0110.
 let mut code=vec![0xb4,0x40,0xbb,0x01,0x00,0xb9,0x03,0x00,0xba,0x10,0x01,0xcd,0x21,0xfa,0xf4];code.resize(0x10,0);code.extend(b"abc");
 let r=run(load(&code,"write.com").unwrap(),10).unwrap();assert_eq!(r.output,b"abc");assert_eq!(r.loaded.cpu.regs[0]&0xffff,3);assert_eq!(r.loaded.cpu.eflags&1,0);
 // A closed handle is error 6; AUX/PRN writes stop as unmodelled.
 code[3]=7;let r=run(load(&code,"write7.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,6);assert_eq!(r.loaded.cpu.eflags&1,1);assert!(r.output.is_empty());
 code[3]=4;let r=run(load(&code,"prn.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
 // MOV AH,51; INT 21 and MOV AH,62; INT 21 both return the PSP in BX.
 for ah in [0x51,0x62]{let r=run(load(&[0xb4,ah,0xcd,0x21,0xfa,0xf4],"psp.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[3]&0xffff,u32::from(r.loaded.selectors[SegmentRegister::Ds.index()]));}
 // INT 2F installation checks find nothing: AX comes back unchanged.
 for ax in [0x0100u16,0x1100,0x1600,0x1680,0x1687,0x1a00,0x4300,0xb710]{
  let [lo,hi]=ax.to_le_bytes();let r=run(load(&[0xb8,lo,hi,0xcd,0x2f,0xfa,0xf4],"mux.com").unwrap(),10).unwrap();
  assert_eq!(r.loaded.cpu.regs[0]&0xffff,u32::from(ax));assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 }
 // AX=4310h (XMS entry point) is only meaningful once XMS is present: still a stop.
 let r=run(load(&[0xb8,0x10,0x43,0xcd,0x2f,0xfa,0xf4],"xms.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Service(_)));
 // MOV AH,12; MOV BL,10; INT 10: no EGA, BL stays 10h. INT 15 AH=C0: CF=1, AH=86h.
 let r=run(load(&[0xb4,0x12,0xb3,0x10,0xcd,0x10,0xfa,0xf4],"ega.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[3]&255,0x10);
 let r=run(load(&[0xb4,0xc0,0xcd,0x15,0xfa,0xf4],"config.com").unwrap(),10).unwrap();assert_eq!((r.loaded.cpu.regs[0]>>8)&255,0x86);assert_eq!(r.loaded.cpu.eflags&1,1);
}
#[test]
fn true_version_default_drive_and_handle_count_answers(){
 // MOV AX,3306; MOV BX,FFFF; MOV DX,FFFF; INT 21: BX=true version 6.22, DX=0.
 let r=run(load(&[0xb8,0x06,0x33,0xbb,0xff,0xff,0xba,0xff,0xff,0xcd,0x21,0xfa,0xf4],"truever.com").unwrap(),10).unwrap();
 assert_eq!((r.loaded.cpu.regs[3]&0xffff,r.loaded.cpu.regs[2]&0xffff,r.loaded.cpu.regs[0]&0xffff),(0x1606,0,0x3306));assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 // Other AH33 subfunctions are not answered yet.
 let r=run(load(&[0xb8,0x00,0x33,0xcd,0x21,0xfa,0xf4],"break.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
 // MOV AX,19FF; INT 21: AL=0 (A:).
 let r=run(load(&[0xb8,0xff,0x19,0xcd,0x21,0xfa,0xf4],"drive.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x1900);
 // MOV AH,67; MOV BX,20; STC; INT 21: 20 handles fit the PSP table (CF=0).
 let r=run(load(&[0xb4,0x67,0xbb,20,0,0xf9,0xcd,0x21,0xfa,0xf4],"jft.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.eflags&1,0);assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 // 50 handles need a new table; a COM owns all memory, so AH48 fails: CF=1, AX=8.
 let r=run(load(&[0xb4,0x67,0xbb,50,0,0xcd,0x21,0xfa,0xf4],"jft50.com").unwrap(),10).unwrap();
 assert_eq!((r.loaded.cpu.regs[0]&0xffff,r.loaded.cpu.regs[3]&0xffff,r.loaded.cpu.eflags&1),(8,50,1));
 let psp=u32::from(PSP)*16;assert_eq!(r.loaded.ram.load(psp+0x32,MemWidth::Word).unwrap(),20);
 // After AH4A shrinks the program to 1000h paragraphs, the table moves: PSP:34h
 // points at the new block, entries 0-4 are copied and the rest are FFh.
 let r=run(load(&[0xb4,0x4a,0xbb,0x00,0x10,0xcd,0x21,0xb4,0x67,0xbb,50,0,0xcd,0x21,0xfa,0xf4],"jftmove.com").unwrap(),10).unwrap();
 assert_eq!(r.loaded.cpu.eflags&1,0);assert_eq!(r.loaded.ram.load(psp+0x32,MemWidth::Word).unwrap(),50);
 let (off,seg)=(r.loaded.ram.load(psp+0x34,MemWidth::Word).unwrap(),r.loaded.ram.load(psp+0x36,MemWidth::Word).unwrap());
 assert_eq!((off,seg),(0,u32::from(PSP)+0x1001));
 let table:Vec<u32>=(0..50).map(|n|r.loaded.ram.load(seg*16+n,MemWidth::Byte).unwrap()).collect();
 assert_eq!(&table[..5],&[0,1,2,3,4]);assert!(table[5..].iter().all(|&b|b==0xff));
}
#[test]
fn far_jump_forms_transfer_cs_and_ip(){
 // JMP FAR 1000:0107 (same place, new encoding) -> MOV AL,2A; HLT.
 let mut code=vec![0xea,0x07,0x01,0,0,0xf4,0xf4,0xb0,0x2a,0xf4];let psp=load(&code,"probe.com").unwrap().selectors[1];
 code[3..5].copy_from_slice(&psp.to_le_bytes());
 let r=run(load(&code,"jmpfar.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&255,0x2a);assert_eq!(r.loaded.selectors[1],psp);
 // JMP FAR [0110] with the pointer CS:0109 in the program.
 let mut code=vec![0xff,0x2e,0x10,0x01,0xf4,0xf4,0xf4,0xf4,0xf4,0xb0,0x2b,0xf4,0,0,0,0,0x09,0x01];code.extend(psp.to_le_bytes());
 let r=run(load(&code,"jmpfarm.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&255,0x2b);
 // FF /5 with a register operand has no far pointer.
 let r=run(load(&[0xff,0xe8,0xf4],"jmpfarr.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::MissingEncoding{..}));
}
#[test]
fn tsr_date_strategy_lfn_and_extended_memory(){
 // MOV AX,3105; MOV DX,0002; INT 21 -> exit 5, 6 paragraphs kept (FreeDOS minimum).
 let r=run(load(&[0xb8,0x05,0x31,0xba,0x02,0x00,0xcd,0x21,0xfa,0xf4],"tsr.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Exit(5)));assert_eq!(r.loaded.resident,Some(6));
 // AH2A: 1980-01-01, Tuesday.
 let r=run(load(&[0xb4,0x2a,0xcd,0x21,0xfa,0xf4],"date.com").unwrap(),10).unwrap();
 assert_eq!(r.loaded.cpu.regs[1]&0xffff,1980);assert_eq!(r.loaded.cpu.regs[2]&0xffff,0x0101);assert_eq!(r.loaded.cpu.regs[0]&255,2);
 // AH0D has nothing to flush.
 let r=run(load(&[0xb4,0x0d,0xcd,0x21,0xfa,0xf4],"reset.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)));
 // AX=5801 BX=0080 accepted, then AX=5800 returns 0080; BX=0003 is invalid; BX=0041 (upper memory only) is accepted and allocates low.
 let r=run(load(&[0xb8,0x01,0x58,0xbb,0x80,0x00,0xcd,0x21,0xb8,0x00,0x58,0xcd,0x21,0xfa,0xf4],"strategy.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x80);assert_eq!(r.loaded.cpu.eflags&1,0);
 let r=run(load(&[0xb8,0x01,0x58,0xbb,0x03,0x00,0xcd,0x21,0xfa,0xf4],"badfit.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,1);assert_eq!(r.loaded.cpu.eflags&1,1);
 let r=run(load(&[0xb8,0x01,0x58,0xbb,0x41,0x00,0xcd,0x21,0xfa,0xf4],"upperonly.com").unwrap(),10).unwrap();assert_eq!((r.loaded.alloc_strategy,r.loaded.cpu.eflags&1),(0x41,0));
 // AX=5803 (link UMBs) fails: no UMB chain.
 let r=run(load(&[0xb8,0x03,0x58,0xbb,0x01,0x00,0xcd,0x21,0xfa,0xf4],"umb.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,1);assert_eq!(r.loaded.cpu.eflags&1,1);
 // AX=7147 (LFN get directory) -> AX=7100, CF=1. INT 15 AH=88 -> AX=0, CF=0.
 let r=run(load(&[0xb8,0x47,0x71,0xcd,0x21,0xfa,0xf4],"lfn.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x7100);assert_eq!(r.loaded.cpu.eflags&1,1);
 let r=run(load(&[0xb4,0x88,0xf9,0xcd,0x15,0xfa,0xf4],"ext.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0);assert_eq!(r.loaded.cpu.eflags&1,0);
}
#[test]
fn bios_rom_identity_and_hlt_idle(){
 let p=load(&[0xf4],"rom.com").unwrap();
 // Empty bus FFh at F000:FFEA (no "COMPAQ"), date at F000:FFF5, model FCh.
 assert_eq!(p.ram.load(0xfffea,MemWidth::Byte).unwrap(),0xff);assert_eq!(p.ram.load(0xc0000,MemWidth::Word).unwrap(),0xffff);
 assert_eq!(&p.ram.bytes()[0xffff5..0xffffd],b"01/10/84");assert_eq!(p.ram.load(0xffffe,MemWidth::Byte).unwrap(),0xfc);
 // MOV AX,F000; MOV ES,AX; MOV AL,ES:[FFFE]; MOV ES:[FFFE],AL faults: the ROM is read-only.
 let r=run(load(&[0xb8,0x00,0xf0,0x8e,0xc0,0x26,0xa0,0xfe,0xff,0x26,0xa2,0xfe,0xff,0xf4],"romw.com").unwrap(),10).unwrap();
 assert_eq!(r.loaded.cpu.regs[0]&255,0xfc);assert!(matches!(r.stop,Stop::Fault(_)),"{:?}",r.stop);
 // With IF=1 (as loaded), HLT; HLT; INT 1A AH=00 sees two idle ticks, then MOV AH,4C; INT 21.
 let r=run(load(&[0xf4,0xf4,0xb4,0x00,0xcd,0x1a,0xb4,0x4c,0xcd,0x21],"idle.com").unwrap(),10).unwrap();
 assert!(matches!(r.stop,Stop::Exit(_)),"{:?}",r.stop);assert_eq!(r.loaded.cpu.regs[2]&0xffff,2);assert_eq!(r.instructions.get("HLT (idle tick)"),Some(&2));
 // With IF=0, HLT never resumes.
 let r=run(load(&[0xfa,0xf4],"hang.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)));
}
#[test]
fn one_operand_imul_and_xlat_have_literal_results(){
 // MOV AX,FFFE; MOV DX,0003; IMUL DX -> DX:AX = FFFF:FFFA (-6), fits: CF=OF=0.
 let r=run(load(&[0xb8,0xfe,0xff,0xba,0x03,0x00,0xf7,0xea,0xfa,0xf4],"imul.com").unwrap(),10).unwrap();
 assert_eq!(r.loaded.cpu.regs[0]&0xffff,0xfffa);assert_eq!(r.loaded.cpu.regs[2]&0xffff,0xffff);assert_eq!(r.loaded.cpu.eflags&0x801,0);
 // MOV AL,80; MOV BL,02; IMUL BL -> AX = FF00 (-256), does not fit AL: CF=OF=1.
 let r=run(load(&[0xb0,0x80,0xb3,0x02,0xf6,0xeb,0xfa,0xf4],"imul8.com").unwrap(),10).unwrap();
 assert_eq!(r.loaded.cpu.regs[0]&0xffff,0xff00);assert_eq!(r.loaded.cpu.eflags&0x801,0x801);
 // MOV BX,0110; MOV AL,03; XLAT -> AL = byte at DS:0113 = 'd'.
 let mut code=vec![0xbb,0x10,0x01,0xb0,0x03,0xd7,0xfa,0xf4];code.resize(0x10,0);code.extend(b"abcd");
 let r=run(load(&code,"xlat.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&255,u32::from(b'd'));
}
#[test]
fn dos_time_scales_bios_ticks_like_the_clock_driver(){
 // 65543 ticks are 359,998 hundredths: 00:59:59.98 (CH=0, CL=59, DH=59, DL=98).
 let mut p=load(&[0xb4,0x2c,0xcd,0x21,0xfa,0xf4],"time.com").unwrap();p.timer_ticks=65543;
 let r=run(p,10).unwrap();assert_eq!(r.loaded.cpu.regs[1]&0xffff,0x003b);assert_eq!(r.loaded.cpu.regs[2]&0xffff,0x3b62);assert_eq!(r.loaded.timer_ticks,65544);
 // INT 15 AX=E801 is unsupported on this AT-class BIOS; INT 2F AX=0600 finds no ASSIGN.
 let r=run(load(&[0xb8,0x01,0xe8,0xcd,0x15,0xfa,0xf4],"e801.com").unwrap(),10).unwrap();assert_eq!((r.loaded.cpu.regs[0]>>8)&255,0x86);assert_eq!(r.loaded.cpu.eflags&1,1);
 let r=run(load(&[0xb8,0x00,0x06,0xcd,0x2f,0xfa,0xf4],"assign.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x0600);
}

#[test]
fn environment_block_names_comspec_and_the_program_and_can_be_freed(){
 // MOV ES,[002C]; MOV AH,49; INT 21; CLI; HLT
 let r=run(load(&[0x8e,0x06,0x2c,0x00,0xb4,0x49,0xcd,0x21,0xfa,0xf4],"images/FD.img::BIN/Self.com").unwrap(),10).unwrap();
 let psp=u32::from(PSP)*16;assert_eq!(r.loaded.ram.load(psp+0x2c,MemWidth::Word).unwrap(),u32::from(ENV));
 let want=b"COMSPEC=A:\\COMMAND.COM\0\0\x01\0A:\\SELF.COM\0";
 let got:Vec<u8>=(0..want.len()as u32).map(|i|r.loaded.ram.load(u32::from(ENV)*16+i,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(got,want);
 // The environment has its own MCB owned by the PSP, so AH49 frees it.
 assert_eq!(r.loaded.cpu.eflags&1,0);assert_eq!(r.loaded.ram.load(u32::from(ENV-1)*16+1,MemWidth::Word).unwrap(),0);
}
#[test]
fn program_reads_its_own_file_and_nothing_else(){
 // MOV AX,3D00; MOV DX,0124; INT 21; MOV BX,AX; MOV AX,4200; XOR CX,CX; MOV DX,2; INT 21;
 // MOV AH,3F; MOV CX,4; MOV DX,0200; INT 21; MOV SI,AX; MOV AH,3E; INT 21; CLI; HLT; "A:\SELF.COM"
 let mut code=vec![0xb8,0x00,0x3d,0xba,0x26,0x01,0xcd,0x21,0x89,0xc3,0xb8,0x00,0x42,0x31,0xc9,0xba,0x02,0x00,0xcd,0x21,
  0xb4,0x3f,0xb9,0x04,0x00,0xba,0x00,0x02,0xcd,0x21,0x89,0xc6,0xb4,0x3e,0xcd,0x21,0xfa,0xf4];
 assert_eq!(code.len(),0x26);code.extend(b"A:\\SELF.COM\0");
 let r=run(load(&code,"self.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert_eq!((r.loaded.cpu.regs[3]&0xffff,r.loaded.cpu.regs[6]&0xffff,r.loaded.cpu.eflags&1),(5,4,0));
 let buffer:Vec<u32>=(0..4).map(|i|r.loaded.ram.load(u32::from(PSP)*16+0x200+i,MemWidth::Byte).unwrap()).collect();assert_eq!(buffer,[0x3d,0xba,0x26,0x01]);
 assert_eq!((r.loaded.open_handles,r.loaded.drive.open_files()),(0x1f,0));
 // Seek from the end gives DX:AX = file size; reading there returns 0 bytes.
 let mut end=code.clone();end[10..18].copy_from_slice(&[0xb8,0x02,0x42,0x31,0xc9,0x31,0xd2,0x90]);end[30..32].copy_from_slice(&[0x89,0xc6]);
 let r=run(load(&end,"self.com").unwrap(),20).unwrap();assert_eq!(r.loaded.cpu.regs[6]&0xffff,0);
 // Another name is not found; the program file opened for writing stops.
 let mut other=code.clone();other[0x29]=b'X';let r=run(load(&other,"self.com").unwrap(),3).unwrap();assert_eq!((r.loaded.cpu.regs[0]&0xffff,r.loaded.cpu.eflags&1),(2,1));
 let mut write=code.clone();write[1]=2;let r=run(load(&write,"self.com").unwrap(),3).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
 // AH3F on a closed handle is error 6.
 let r=run(load(&[0xb4,0x3f,0xbb,0x07,0x00,0xcd,0x21,0xfa,0xf4],"read.com").unwrap(),5).unwrap();assert_eq!((r.loaded.cpu.regs[0]&0xffff,r.loaded.cpu.eflags&1),(6,1));
}

#[test]
fn list_of_lists_leads_to_the_real_mcb_chain_and_indos_is_clear(){
 // MOV AH,52; INT 21; CLI; HLT
 let r=run(load(&[0xb4,0x52,0xcd,0x21,0xfa,0xf4],"lol.com").unwrap(),10).unwrap();
 let es=r.loaded.selectors[SegmentRegister::Es.index()];let bx=r.loaded.cpu.regs[3]&0xffff;assert_eq!((es,bx),(DOSDATA,u32::from(SYSVARS)));
 let base=u32::from(es)*16+bx;let ram=&r.loaded.ram;
 // Walk the chain from ES:[BX-2]: environment, program, then the 'Z' block.
 let mut mcb=ram.load(base-2,MemWidth::Word).unwrap();let mut seen=Vec::new();
 loop{let kind=ram.load(mcb*16,MemWidth::Byte).unwrap()as u8;let owner=ram.load(mcb*16+1,MemWidth::Word).unwrap();let size=ram.load(mcb*16+3,MemWidth::Word).unwrap();seen.push((kind,owner));if kind==b'Z'{break;}mcb+=size+1;}
 assert_eq!(seen,vec![(b'M',u32::from(PSP)),(b'Z',u32::from(PSP))]);
 // Unmodelled structures are FFFF:FFFF, no block devices, and NUL ends the device chain.
 assert_eq!(ram.load(base+0x04,MemWidth::Dword).unwrap(),0xffff_ffff);assert_eq!(ram.load(base+0x20,MemWidth::Byte).unwrap(),0);
 assert_eq!(ram.load(base+0x22,MemWidth::Dword).unwrap(),0xffff_ffff);assert_eq!(ram.load(base+0x26,MemWidth::Word).unwrap(),0x8004);
 let name:Vec<u8>=(0..8).map(|i|ram.load(base+0x2c+i,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(name,b"NUL     ");
 // MOV AH,34; INT 21: ES:BX -> InDOS = 0, critical-error byte before it = 0.
 let r=run(load(&[0xb4,0x34,0xcd,0x21,0xfa,0xf4],"indos.com").unwrap(),10).unwrap();
 let at=u32::from(r.loaded.selectors[SegmentRegister::Es.index()])*16+(r.loaded.cpu.regs[3]&0xffff);
 assert_eq!((r.loaded.ram.load(at,MemWidth::Byte).unwrap(),r.loaded.ram.load(at-1,MemWidth::Byte).unwrap()),(0,0));
}
#[test]
fn session_commands_run_queue_keys_and_report_the_screen(){
 // MOV AH,0; INT 16; MOV AH,4C; INT 21: waits for one key, exits with AL.
 let mut s=session::Session::new(load(&[0xb4,0x00,0xcd,0x16,0xb4,0x4c,0xcd,0x21],"key.com").unwrap());
 let (reply,quit)=s.command(1,"run 100").unwrap();assert!(!quit);
 assert!(reply.contains("\"status\":\"waiting_input\""),"{reply}");assert!(reply.contains("\"columns\":80"));
 assert!(reply.contains(&format!("\"screen\":\"{}\"","2007".repeat(80*25))),"blank page is space on attribute 7");
 // The ring holds 15 keys; the 16th is refused without stopping the session.
 for n in 0..15{assert!(s.key(b'a'+n,0x1e).unwrap());}assert!(!s.key(b'z',0x2c).unwrap());
 let (reply,_)=s.command(2,"key 1 2").unwrap();assert!(reply.contains("\"ok\":false")&&reply.contains("buffer full"),"{reply}");
 let (reply,_)=s.command(3,"run 100").unwrap();assert!(reply.contains("\"status\":\"exit\"")&&reply.contains("exit code 97"),"{reply}");
 for (n,bad) in ["run 0","run x","key 300 1","dance"].iter().enumerate(){let (reply,_)=s.command(4+n as u64,bad).unwrap();assert!(reply.contains("\"ok\":false"),"{bad}: {reply}");}
 assert!(s.command(9,"quit").unwrap().1);
}
fn call(p:&mut Loaded,ax:u32,cx:u32,path:&[u8])->(u32,u32){
 let psp=u32::from(PSP)*16;let mut z=path.to_vec();z.push(0);p.ram.write_bytes(psp+0x200,&z).unwrap();
 p.cpu.regs[0]=ax;p.cpu.regs[1]=cx;p.cpu.regs[2]=0x200;p.cpu.eflags=0x202;service(p,0x21,&mut vec![]).unwrap();
 (p.cpu.regs[0]&0xffff,p.cpu.eflags&1)
}
fn dta_name(p:&Loaded)->(u8,u32,Vec<u8>){
 let base=u32::from(PSP)*16+0x80;let b=|i|p.ram.load(base+i,MemWidth::Byte).unwrap()as u8;
 let name=(0x1e..0x2b).map(b).take_while(|&c|c!=0).collect();(b(0x15),p.ram.load(base+0x1a,MemWidth::Dword).unwrap(),name)
}
#[test]
fn eight_dot_three_names_and_aliases_follow_dos_rules(){
 use files::probe::*;
 assert_eq!(&fcb(b"readme.txt"),b"README  TXT");assert_eq!(&fcb(b"*.*"),b"???????????");assert_eq!(&fcb(b"a*.c*"),b"A???????C??");
 assert_eq!(&fcb(b"longername.html"),b"LONGERNAHTM");assert_eq!(display(&fcb(b"dir")),b"DIR");
 assert_eq!(short("Factor.com").map(|n|display(&n)),Some(b"FACTOR.COM".to_vec()));
 for long in ["mandelbrot.com","two.dots.txt","has space.c",".hidden","x.html"]{assert!(short(long).is_none(),"{long}");}
 let mut taken=std::collections::BTreeSet::new();
 let first=alias("mandelbrot.com",&taken).unwrap();assert_eq!(display(&first),b"MANDEL~1.COM");taken.insert(first);
 assert_eq!(display(&alias("Mandelbrot2.com",&taken).unwrap()),b"MANDEL~2.COM");
 assert_eq!(display(&alias("two.dots.txt",&taken).unwrap()),b"TWODOT~1.TXT");assert_eq!(display(&alias(".bashrc",&taken).unwrap()),b"BASHRC~1");
 // 1980-01-01 00:00:00 and 2026-10-30 16:26:02 UTC; earlier times clamp to 1980.
 use std::time::{Duration,UNIX_EPOCH};
 assert_eq!(stamp(Some(UNIX_EPOCH)),(0,0x0021));
 assert_eq!(stamp(Some(UNIX_EPOCH+Duration::from_secs(1_793_377_562))),(16<<11|26<<5|1,(46<<9|10<<5|30)as u16));
}
#[test]
fn mounted_folder_is_drive_c_for_find_chdir_and_reading_files(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-drive-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(dir.join("Games")).unwrap();std::fs::write(dir.join("factor.com"),b"FACT").unwrap();
 std::fs::write(dir.join("mandelbrot.com"),b"MANDELBROT").unwrap();std::fs::write(dir.join("Games/readme.txt"),b"hello").unwrap();
 let mut p=load_with_drives(&[0xcd,0x21],"COMMAND.COM",vec![Source::Host(dir.clone())],None).unwrap();
 let env:Vec<u8>=(0..22).map(|i|p.ram.load(u32::from(ENV)*16+i,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(env,b"COMSPEC=C:\\COMMAND.COM");
 assert_eq!(call(&mut p,0x1900,0,b"").0&255,2);
 // Files only, then with directories: host order, 8.3 aliases, sizes.
 let mut names=Vec::new();let mut r=call(&mut p,0x4e00,0,b"*.*");
 while r.1==0{names.push(dta_name(&p));r=call(&mut p,0x4f00,0,b"");}
 assert_eq!(r.0,0x12);assert_eq!(names,vec![(0x20,4,b"FACTOR.COM".to_vec()),(0x20,10,b"MANDEL~1.COM".to_vec())]);
 assert_eq!(call(&mut p,0x4e00,0x10,b"C:\\G*").1,0);assert_eq!(dta_name(&p),(0x10,0,b"GAMES".to_vec()));
 assert_eq!(call(&mut p,0x4e00,0x08,b"\\*.*"),(0x12,1),"no volume label");assert_eq!(call(&mut p,0x4e00,0,b"NOPE\\*.*"),(3,1));
 // CD into the directory: . and .. come first, AH47 reports it, .. leads back.
 assert_eq!(call(&mut p,0x3b00,0,b"games").1,0);
 let mut names=Vec::new();let mut r=call(&mut p,0x4e00,0x10,b"*.*");while r.1==0{names.push(dta_name(&p).2);r=call(&mut p,0x4f00,0,b"");}
 assert_eq!(names,vec![b".".to_vec(),b"..".to_vec(),b"README.TXT".to_vec()]);
 p.cpu.regs[6]=0x300;assert_eq!(call(&mut p,0x4700,0,b""),(0x0100,0));
 let cwd:Vec<u8>=(0..6).map(|i|p.ram.load(u32::from(PSP)*16+0x300+i,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(cwd,b"GAMES\0");
 assert_eq!(call(&mut p,0x3b00,0,b"..\\..").1,1);assert_eq!(call(&mut p,0x3b00,0,b"A:\\").1,1);
 // Read a file through its alias from another directory; a read handle cannot write.
 let (handle,cf)=call(&mut p,0x3d00,0,b"\\mandel~1.com");assert_eq!(cf,0);
 p.cpu.regs[3]=handle;p.cpu.regs[2]=0x400;p.cpu.regs[1]=4;p.cpu.regs[0]=0x3f00;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[0]&0xffff,4);let read:Vec<u8>=(0..4).map(|i|p.ram.load(u32::from(PSP)*16+0x400+i,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(read,b"MAND");
 p.cpu.regs[0]=0x4400;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[2]&0xffff,0x42);
 p.cpu.regs[0]=0x3e00;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.drive.open_files(),0);
 assert_eq!(call(&mut p,0x3d00,0,b"missing.txt"),(2,1));assert_eq!(call(&mut p,0x3d00,0,b"\\games"),(5,1));
 assert_eq!(call(&mut p,0x4300,0,b"readme.txt"),(0x4300,0));assert_eq!(p.cpu.regs[1]&0xffff,0x20);assert_eq!(call(&mut p,0x4301,0x20,b"readme.txt"),(5,1));
 // AH36: no free clusters reported; three 512-byte clusters hold the files.
 p.cpu.regs[2]=0;p.cpu.regs[0]=0x3600;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[3]&0xffff,p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff),(1,0,512,3));
 p.cpu.regs[2]=1;p.cpu.regs[0]=0x3600;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,0xffff);
 // Without a mount, A: is an empty drive.
 let mut empty=load(&[0xcd,0x21],"x.com").unwrap();assert_eq!(call(&mut empty,0x4e00,0x10,b"*.*"),(0x12,1));assert_eq!(call(&mut empty,0x1900,0,b"").0&255,0);
 std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn idiv_truncates_toward_zero_and_keeps_the_dividend_sign_in_the_remainder(){
 // MOV AX,-7; MOV BL,2; IDIV BL; MOV SI,AX; MOV DX,FFFE; MOV AX,7960; MOV CX,7; IDIV CX; CLI; HLT
 let code=[0xb8,0xf9,0xff,0xb3,0x02,0xf6,0xfb,0x89,0xc6,0xba,0xfe,0xff,0xb8,0x60,0x79,0xb9,0x07,0x00,0xf7,0xf9,0xfa,0xf4];
 let r=run(load(&code,"idiv.com").unwrap(),20).unwrap();
 // -7/2 = -3 r -1; -100000/7 = -14285 r -5.
 assert_eq!((r.loaded.cpu.regs[6]&0xffff,r.loaded.cpu.regs[0]&0xffff,r.loaded.cpu.regs[2]&0xffff),(0xfffd,0xc833,0xfffb));
 // 128/1 does not fit a signed byte: divide error, which stops the run.
 let r=run(load(&[0xb8,0x80,0x00,0xb1,0x01,0xf6,0xf9,0xfa,0xf4],"idivde.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Fault(_)),"{:?}",r.stop);
}
#[test]
fn country_information_is_us_code_page_437_with_a_working_case_map(){
 let mut p=load(&[0xcd,0x21],"nls.com").unwrap();let psp=u32::from(PSP)*16;let mut out=vec![];
 let bytes=|p:&Loaded,at:u32,n:u32|->Vec<u8>{(0..n).map(|i|p.ram.load(at+i,MemWidth::Byte).unwrap()as u8).collect()};
 // AH38 AL=00: 24 bytes at DS:DX, AX=BX=1.
 p.cpu.regs[0]=0x3800;p.cpu.regs[2]=0x200;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[3]&0xffff,p.cpu.eflags&1),(1,1,0));
 let info=bytes(&p,psp+0x200,24);assert_eq!(&info[..18],b"\0\0$\0\0\0\0,\0.\0-\0:\0\0\x02\0");assert_eq!(info[22],b',');
 // AX=6501: ID 1, size 1Ch, country 1, code page 437, then the same record.
 p.cpu.regs[0]=0x6501;p.cpu.regs[3]=0xffff;p.cpu.regs[2]=0xffff;p.cpu.regs[1]=0x40;p.cpu.regs[7]=0x300;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!(bytes(&p,psp+0x300,7),[1,0x1c,0,1,0,0xb5,1]);assert_eq!(bytes(&p,psp+0x307,24),info);
 // A buffer too small is error 1; information ID 3 does not exist.
 p.cpu.regs[0]=0x6501;p.cpu.regs[1]=30;service(&mut p,0x21,&mut out).unwrap();assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.eflags&1),(1,1));
 p.cpu.regs[0]=0x6503;p.cpu.regs[1]=0x40;service(&mut p,0x21,&mut out).unwrap();assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.eflags&1),(1,1));
 // AX=6502 points at the upper-case table: size 128, then 80h..FFh.
 p.cpu.regs[0]=0x6502;p.cpu.regs[1]=5;service(&mut p,0x21,&mut out).unwrap();let rec=bytes(&p,psp+0x300,5);assert_eq!(rec[0],2);
 let table=u32::from(u16::from_le_bytes([rec[3],rec[4]]))*16+u32::from(u16::from_le_bytes([rec[1],rec[2]]));
 assert_eq!(bytes(&p,table,2),[0x80,0]);assert_eq!(bytes(&p,table+2+(0x84-0x80),1),[0x8e]);assert_eq!(bytes(&p,table+2+(0xa4-0x80),1),[0xa5]);
 // Upper-case calls: a character, a counted buffer and an ASCIIZ string.
 p.cpu.regs[0]=0x6520;p.cpu.regs[2]=0x81;service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu.regs[2]&255,0x9a);
 p.ram.write_bytes(psp+0x400,b"ab\x87d\0zz").unwrap();p.cpu.regs[0]=0x6522;p.cpu.regs[2]=0x400;service(&mut p,0x21,&mut out).unwrap();assert_eq!(bytes(&p,psp+0x400,7),b"AB\x80D\0zz");
 p.cpu.regs[0]=0x6521;p.cpu.regs[1]=6;service(&mut p,0x21,&mut out).unwrap();assert_eq!(bytes(&p,psp+0x400,7),b"AB\x80D\0Zz");
 p.cpu.regs[0]=0x6523;p.cpu.regs[2]=u32::from(b'n');service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,0);
 // The case-map routine from the record runs as code: CALL FAR it with AL=84h (a with diaeresis).
 let ptr=u32::from(u16::from_le_bytes([info[20],info[21]]))<<16|u32::from(u16::from_le_bytes([info[18],info[19]]));
 let mut code=vec![0xb0,0x84,0x9a];code.extend((ptr as u16).to_le_bytes());code.extend(((ptr>>16)as u16).to_le_bytes());code.extend([0xfa,0xf4]);
 let r=run(load(&code,"casemap.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert_eq!(r.loaded.cpu.regs[0]&255,0x8e);
}
#[test]
fn image_is_drive_c_with_its_label_and_the_host_folder_is_drive_d(){
 use crate::containers::{FatEntry,FatNode,FatTree};use std::sync::Arc;
 let entry=|name:&[u8;11],attr,size,node|FatEntry{name:*name,attr,time:0x8000,date:0x5d5e,size,node};
 let tree=FatTree{dirs:vec![vec![entry(b"COMMAND COM",0x20,3,FatNode::File(0)),entry(b"KERNEL  SYS",0x27,4,FatNode::File(1)),entry(b"DOCS       ",0x10,0,FatNode::Dir(1))],vec![entry(b"NOTE    TXT",0x20,4,FatNode::File(2))]],
  files:vec![Arc::new(b"abc".to_vec()),Arc::new(b"kern".to_vec()),Arc::new(b"note".to_vec())],label:Some(*b"FD14-LITE  "),sectors_per_cluster:4,clusters:5000};
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-drives-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(&dir).unwrap();std::fs::write(dir.join("factor.com"),b"F").unwrap();
 let mut p=load_with_drives(&[0xcd,0x21],"img::FREEDOS/COMMAND.COM",vec![Source::Image(Arc::new(tree)),Source::Host(dir.clone())],Some("FREEDOS/COMMAND.COM")).unwrap();
 let env:Vec<u8>=(0..56).map(|i|p.ram.load(u32::from(ENV)*16+i,MemWidth::Byte).unwrap()as u8).collect();
 assert!(env.starts_with(b"COMSPEC=C:\\COMMAND.COM\0\0\x01\0C:\\FREEDOS\\COMMAND.COM\0"),"{env:?}");
 // Label search, then files: hidden/system KERNEL.SYS only with those bits asked for.
 assert_eq!(call(&mut p,0x4e00,0x08,b"C:\\*.*").1,0);assert_eq!(dta_name(&p),(0x08,0,b"FD14-LIT.E".to_vec()));
 let mut names=Vec::new();let mut r=call(&mut p,0x4e00,0x10,b"*.*");while r.1==0{names.push(dta_name(&p).2);r=call(&mut p,0x4f00,0,b"");}
 assert_eq!(names,vec![b"COMMAND.COM".to_vec(),b"DOCS".to_vec()]);
 assert_eq!(call(&mut p,0x4e00,0x16,b"K*.*").1,0);assert_eq!(dta_name(&p),(0x27,4,b"KERNEL.SYS".to_vec()));
 // Read a file from a subdirectory of the image.
 let (handle,cf)=call(&mut p,0x3d00,0,b"c:\\docs\\note.txt");assert_eq!(cf,0);
 p.cpu.regs[3]=handle;p.cpu.regs[2]=0x400;p.cpu.regs[1]=8;p.cpu.regs[0]=0x3f00;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,4);
 // AH36 reports the image geometry with nothing free; AH0E moves to D:, the host folder.
 p.cpu.regs[2]=3;p.cpu.regs[0]=0x3600;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[3]&0xffff,p.cpu.regs[2]&0xffff),(4,0,5000));
 p.cpu.regs[2]=3;p.cpu.regs[0]=0x0e00;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&255,5);
 assert_eq!(call(&mut p,0x1900,0,b"").0&255,3);assert_eq!(call(&mut p,0x4e00,0,b"*.COM").1,0);assert_eq!(dta_name(&p).2,b"FACTOR.COM");
 // E: does not exist: selecting it keeps D:, and its paths are not found.
 p.cpu.regs[2]=4;p.cpu.regs[0]=0x0e00;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(call(&mut p,0x1900,0,b"").0&255,3);
 assert_eq!(call(&mut p,0x3d00,0,b"E:\\X"),(3,1));
 // The image is read-only: no write opens, creates, deletes, renames or directories.
 assert_eq!(call(&mut p,0x3d01,0,b"C:\\DOCS\\NOTE.TXT"),(5,1));assert_eq!(call(&mut p,0x3c00,0,b"C:\\NEW.TXT"),(5,1));
 assert_eq!(call(&mut p,0x5b00,0,b"C:\\NEW.TXT"),(5,1));assert_eq!(call(&mut p,0x4100,0,b"C:\\DOCS\\NOTE.TXT"),(5,1));
 assert_eq!(call(&mut p,0x3900,0,b"C:\\NEWDIR"),(5,1));assert_eq!(call(&mut p,0x3a00,0,b"C:\\DOCS"),(5,1));
 std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn exec_runs_a_com_and_a_relocated_exe_and_returns_their_codes(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-exec-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(&dir).unwrap();
 // CHILD.COM: MOV AL,[80h]; MOV AH,4C; INT 21 (exits with its tail length).
 std::fs::write(dir.join("child.com"),[0xa0,0x80,0x00,0xb4,0x4c,0xcd,0x21]).unwrap();
 // CHILD.EXE: one relocation (MOV AX,seg), then exits with the byte 7 from that segment.
 let mut exe=vec![0x4d,0x5a,0x2d,0x00,0x01,0x00,0x01,0x00,0x02,0x00,0x00,0x00,0xff,0xff,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x00,0x00,0x1c,0x00,0x00,0x00,0x01,0x00,0x00,0x00];
 exe.extend([0xb8,0x00,0x00,0x8e,0xd8,0xa0,0x0c,0x00,0xb4,0x4c,0xcd,0x21,0x07]);std::fs::write(dir.join("child.exe"),exe).unwrap();
 // Shrinks itself to 100h paragraphs, EXECs CHILD.COM with tail " hi", CHILD.EXE and
 // NONE.COM; SI and DI get the AH4D codes, BP the error from the missing file.
 let parent=[0xbb,0x00,0x01,0xb4,0x4a,0xcd,0x21,0x8c,0x0e,0x48,0x01,0x8c,0x0e,0x4c,0x01,0x8c,0x0e,0x50,0x01,0xba,0x67,0x01,0xbb,0x44,0x01,0xb8,0x00,0x4b,0xcd,0x21,0xb4,0x4d,0xcd,0x21,0x89,0xc6,0xba,0x71,0x01,0xbb,0x44,0x01,0xb8,0x00,0x4b,0xcd,0x21,0xb4,0x4d,0xcd,0x21,0x89,0xc7,0xba,0x7b,0x01,0xbb,0x44,0x01,0xb8,0x00,0x4b,0xcd,0x21,0x89,0xc5,0xfa,0xf4,
  0x00,0x00,0x52,0x01,0x00,0x00,0x57,0x01,0x00,0x00,0x57,0x01,0x00,0x00,0x03,0x20,0x68,0x69,0x0d,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
 let mut code=parent.to_vec();code.extend(b"CHILD.COM\0CHILD.EXE\0NONE.COM\0");
 let p=load_with_drives(&code,"PARENT.COM",vec![Source::Host(dir.clone())],None).unwrap();
 let r=run(p,200).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 let regs=&r.loaded.cpu.regs;assert_eq!((regs[6]&0xffff,regs[7]&0xffff,regs[5]&0xffff),(3,7,2));
 // Back in the parent: its PSP again, the children's blocks free, no frames left.
 assert_eq!((r.loaded.psp,r.loaded.exec.len()),(PSP,0));
 let mut again=r.loaded;again.cpu.regs[0]=0x4800;again.cpu.regs[3]=0xffff;service(&mut again,0x21,&mut vec![]).unwrap();
 assert_eq!(again.cpu.regs[3]&0xffff,u32::from(again.arena.free()));assert!(again.arena.free()>0x8000);
 std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn parse_name_fills_an_fcb_like_freedos(){
 let mut p=load_with_drives(&[0xcd,0x21],"x.com",vec![Source::Host(std::env::temp_dir())],None).unwrap();let psp=u32::from(PSP)*16;
 let mut parse=|text:&[u8],al:u32,fcb:&[u8;12]|->(u32,u32,Vec<u8>){
  let mut z=text.to_vec();z.push(0);p.ram.write_bytes(psp+0x200,&z).unwrap();p.ram.write_bytes(psp+0x300,fcb).unwrap();
  p.cpu.regs[0]=0x2900|al;p.cpu.regs[6]=0x200;p.cpu.regs[7]=0x300;service(&mut p,0x21,&mut vec![]).unwrap();
  (p.cpu.regs[0]&255,(p.cpu.regs[6]&0xffff)-0x200,(0..16).map(|i|p.ram.load(psp+0x300+i,MemWidth::Byte).unwrap()as u8).collect())
 };
 let old=*b"\x05OLDNAME OLD";
 assert_eq!(parse(b" c:readme.txt rest",1,&old),(0,13,b"\x03README  TXT\0\0\0\0".to_vec()));
 assert_eq!(parse(b"*.c?",0,&old),(1,4,b"\0????????C? \0\0\0\0".to_vec()));
 // Bit 1 keeps the drive byte and bit 3 the extension; FreeDOS blanks the rest
 // of the name field even with bit 2. Z: does not exist; a name stops at 8.
 assert_eq!(parse(b"",0x0e,&old),(0,0,b"\x05        OLD\0\0\0\0".to_vec()));
 assert_eq!(parse(b"z:longfilename.text",0,&old),(0xff,10,b"\x1aLONGFILE   \0\0\0\0".to_vec()));
 assert_eq!(parse(b"..\\x",0,&old),(0,2,b"\0..         \0\0\0\0".to_vec()));
 // Leading separators are skipped only with bit 0.
 assert_eq!(parse(b";,a",0,&old).1,0);assert_eq!(parse(b";,a",1,&old),(0,3,b"\0A          \0\0\0\0".to_vec()));
}
#[test]
fn console_handle_reads_a_cooked_line_in_pieces(){
 // MOV AH,3F; XOR BX,BX; MOV CX,2; MOV DX,200; INT 21; MOV SI,AX; MOV AH,3F; MOV CX,10; INT 21; CLI; HLT
 let code=[0xb4,0x3f,0x31,0xdb,0xb9,0x02,0x00,0xba,0x00,0x02,0xcd,0x21,0x89,0xc6,0xb4,0x3f,0xb9,0x0a,0x00,0xcd,0x21,0xfa,0xf4];
 let mut s=session::Session::new(load(&code,"con.com").unwrap());
 let (reply,_)=s.command(1,"run 100").unwrap();assert!(reply.contains("\"status\":\"waiting_input\""),"{reply}");
 for(a,sc)in [(b'a',0x1e),(b'x',0x2d),(8,0x0e),(b'b',0x30),(13,0x1c)]{assert!(s.key(a,sc).unwrap());}
 let (reply,_)=s.command(2,"run 100").unwrap();assert!(reply.contains("monitor stop"),"{reply}");
 let (screen,_,cursor)=s.screen().unwrap();assert_eq!((screen[0],screen[2]),(b'a',b'b'));assert_eq!(cursor,(1,0),"CR then LF");
 let p=s.loaded().unwrap();
 // The first read takes "ab", the second the CR LF that ends the line.
 assert_eq!((p.cpu.regs[6]&0xffff,p.cpu.regs[0]&0xffff),(2,2));
 let got:Vec<u8>=(0..2).map(|i|p.ram.load(u32::from(PSP)*16+0x200+i,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(got,b"\r\n");
}
#[test]
fn demo_programs_print_exact_factorials_and_a_text_mandelbrot(){
 let r=run(load(include_bytes!("../../programs/dos/FACTOR.COM"),"FACTOR.COM").unwrap(),200_000).unwrap();
 assert!(matches!(r.stop,Stop::Exit(0)),"{:?}",r.stop);let text=String::from_utf8(r.output).unwrap();
 assert!(text.starts_with(" 0! = 1\r\n 1! = 1\r\n"),"{text}");assert!(text.contains("20! = 2432902008176640000\r\n"));
 assert!(text.ends_with("25! = 15511210043330985984000000\r\n"),"{text}");
 let r=run(load(include_bytes!("../../programs/dos/MANDEL.COM"),"MANDEL.COM").unwrap(),1_000_000).unwrap();
 assert!(matches!(r.stop,Stop::Exit(0)),"{:?}",r.stop);let text=String::from_utf8(r.output).unwrap();
 let lines:Vec<&str>=text.split_terminator("\r\n").collect();assert_eq!(lines.len(),24);assert!(lines.iter().all(|l|l.len()==79));
 // The top row is far outside the set; the row just above the axis crosses it.
 assert_eq!(lines[0],"::::::::::::::::::---------------------------=====---------::::::::::::::::::::");
 assert_eq!(lines[11],":====+%++++++**@#############################################+===------------::");
}
#[test]
fn ega_and_vga_video_functions_return_unchanged_on_the_cga_bios(){
 // AX=1130h (font info), 1A00h (display combination), 1003h (blink), 1B00h, 1C00h, 1210h (EGA info).
 for ax in [0x1130u32,0x1a00,0x1003,0x1b00,0x1c00,0x1210]{
  let mut p=load(&[0xcd,0x10],"vga.com").unwrap();p.cpu.regs[0]=ax;p.cpu.regs[3]=0x0010;p.cpu.regs[2]=0x1234;p.cpu.regs[1]=0x5678;
  let before=p.cpu.clone();service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu.regs,before.regs,"AX={ax:04x}");assert_eq!(p.cpu.eflags,before.eflags);
 }
}
#[test]
fn duplicated_handles_share_the_file_position_and_close_independently(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-dup-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(&dir).unwrap();std::fs::write(dir.join("text.txt"),b"abcdef").unwrap();
 let mut p=load_with_drives(&[0xcd,0x21],"x.com",vec![Source::Host(dir.clone())],None).unwrap();let psp=u32::from(PSP)*16;
 let mut svc=|p:&mut Loaded,ax:u32,bx:u32,cx:u32|->(u32,u32){p.cpu.regs[0]=ax;p.cpu.regs[3]=bx;p.cpu.regs[1]=cx;p.cpu.regs[2]=0x400;service(p,0x21,&mut vec![]).unwrap();(p.cpu.regs[0]&0xffff,p.cpu.eflags&1)};
 let (file,cf)=call(&mut p,0x3d00,0,b"text.txt");assert_eq!((file,cf),(5,0));
 // AH45 takes the lowest free handle; reads through either handle advance one position.
 assert_eq!(svc(&mut p,0x4500,file,0),(6,0));
 assert_eq!(svc(&mut p,0x3f00,file,2),(2,0));assert_eq!(svc(&mut p,0x3f00,6,2),(2,0));
 assert_eq!(p.ram.load(psp+0x400,MemWidth::Word).unwrap(),u32::from(u16::from_le_bytes(*b"cd")));
 // Closing the original keeps the copy open; closing the copy closes the file.
 assert_eq!(svc(&mut p,0x3e00,file,0).1,0);assert_eq!(p.drive.open_files(),1);
 assert_eq!(svc(&mut p,0x3f00,6,9),(2,0));assert_eq!(svc(&mut p,0x3e00,6,0).1,0);assert_eq!(p.drive.open_files(),0);
 // AH46 points handle 9 at CON: device information and writes follow it; writing a file is access denied.
 assert_eq!(svc(&mut p,0x4600,1,9).1,0);assert_eq!(svc(&mut p,0x4400,9,0).1,0);assert_eq!(p.cpu.regs[2]&0xffff,0xc3);
 let (file,_)=call(&mut p,0x3d00,0,b"text.txt");assert_eq!(svc(&mut p,0x4000,file,1),(5,1));
 // Bad source handle, and a target past the 20-entry table.
 assert_eq!(svc(&mut p,0x4500,12,0),(6,1));assert_eq!(svc(&mut p,0x4600,1,20),(6,1));
 std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn boot_drive_is_the_first_drive(){
 let mut p=load(&[0xcd,0x21],"x.com").unwrap();p.cpu.regs[0]=0x3305;p.cpu.regs[2]=0xff;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[2]&255,1,"A: without drives");
 let mut p=load_with_drives(&[0xcd,0x21],"x.com",vec![Source::Host(std::env::temp_dir())],None).unwrap();p.cpu.regs[0]=0x3305;p.cpu.regs[2]=0xff;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[2]&255,3,"C:");
}
#[test]
fn host_folder_takes_creates_writes_deletes_renames_and_directories(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-write-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(&dir).unwrap();std::fs::write(dir.join("old.txt"),b"abcdef").unwrap();
 let mut p=load_with_drives(&[0xcd,0x21],"x.com",vec![Source::Host(dir.clone())],None).unwrap();let psp=u32::from(PSP)*16;
 let mut svc=|p:&mut Loaded,ax:u32,bx:u32,cx:u32,data:&[u8]|->(u32,u32){p.ram.write_bytes(psp+0x400,data).unwrap();p.cpu.regs[0]=ax;p.cpu.regs[3]=bx;p.cpu.regs[1]=cx;p.cpu.regs[2]=0x400;service(p,0x21,&mut vec![]).unwrap();(p.cpu.regs[0]&0xffff,p.cpu.eflags&1)};
 // FreeCOM's redirection: close handle 1, then AH3C takes it; AH02 and AH40 on handle 1 reach the file.
 assert_eq!(svc(&mut p,0x3e00,1,0,b"").1,0);
 assert_eq!(call(&mut p,0x3c00,0,b"out.txt"),(1,0));
 assert_eq!(svc(&mut p,0x4000,1,3,b"DIR"),(3,0));p.cpu.regs[2]=u32::from(b'!');p.cpu.regs[0]=0x0200;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(std::fs::read(dir.join("OUT.TXT")).unwrap(),b"DIR!");
 // Bit 6 of the device information clears once the file is written.
 assert_eq!(svc(&mut p,0x4400,1,0,b"").1,0);assert_eq!(p.cpu.regs[2]&0xffff,2);
 // Seek back and write over, past the end, then CX=0 truncates at the position.
 p.cpu.regs[2]=1;p.cpu.regs[1]=0;p.cpu.regs[3]=1;p.cpu.regs[0]=0x4200;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(svc(&mut p,0x4000,1,5,b"xyzzy"),(5,0));assert_eq!(std::fs::read(dir.join("OUT.TXT")).unwrap(),b"Dxyzzy");
 p.cpu.regs[2]=3;p.cpu.regs[1]=0;p.cpu.regs[3]=1;p.cpu.regs[0]=0x4200;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(svc(&mut p,0x4000,1,0,b""),(0,0));assert_eq!(std::fs::read(dir.join("OUT.TXT")).unwrap(),b"Dxy");
 // AH5B refuses a name that exists; AH3C truncates it; a read handle cannot write, a write handle cannot read.
 assert_eq!(call(&mut p,0x5b00,0,b"OLD.TXT"),(0x50,1));
 let (read,_)=call(&mut p,0x3d00,0,b"old.txt");assert_eq!(svc(&mut p,0x4000,read,1,b"q"),(5,1));
 let (wo,_)=call(&mut p,0x3d01,0,b"old.txt");assert_eq!(svc(&mut p,0x3f00,wo,1,b""),(5,1));assert_eq!(svc(&mut p,0x4000,wo,2,b"AB"),(2,0));
 assert_eq!(std::fs::read(dir.join("old.txt")).unwrap(),b"ABcdef");
 assert_eq!(call(&mut p,0x3c00,0,b"old.txt").1,0);assert_eq!(std::fs::read(dir.join("old.txt")).unwrap(),b"");
 // AH5A appends a new eight-digit name to the path and creates the file.
 let (_,cf)=call(&mut p,0x5a00,0,b"\\");assert_eq!(cf,0);
 let name:Vec<u8>=(0..14).map(|i|p.ram.load(psp+0x200+i,MemWidth::Byte).unwrap()as u8).take_while(|&c|c!=0).collect();
 assert_eq!(name.len(),9);assert!(dir.join(std::str::from_utf8(&name[1..]).unwrap()).is_file());
 // AH57 sets and gets the time and date of an open file.
 let stamp=(12u32<<11|34<<5|28,(45u32<<9|6<<5|15));
 p.cpu.regs[1]=stamp.0;p.cpu.regs[2]=stamp.1;p.cpu.regs[3]=1;p.cpu.regs[0]=0x5701;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.eflags&1,0);
 p.cpu.regs[1]=0;p.cpu.regs[2]=0;p.cpu.regs[0]=0x5700;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!((p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff),stamp);
 // Directories: make, refuse twice, rename into, remove only when empty and not current.
 assert_eq!(call(&mut p,0x3900,0,b"sub").1,0);assert_eq!(call(&mut p,0x3900,0,b"SUB"),(5,1));
 p.ram.write_bytes(psp+0x300,b"SUB\\NEW.TXT\0").unwrap();p.cpu.regs[7]=0x300;
 assert_eq!(call(&mut p,0x5600,0,b"OUT.TXT").1,0);assert!(dir.join("SUB/NEW.TXT").is_file()&&!dir.join("OUT.TXT").exists());
 assert_eq!(call(&mut p,0x3a00,0,b"SUB"),(5,1),"not empty");
 assert_eq!(call(&mut p,0x5600,0,b"MISSING.TXT"),(2,1));
 // AH60 gives the full name; AH41 deletes; AH4E/4F go on after a deleted file.
 p.cpu.regs[6]=0x200;p.cpu.regs[7]=0x300;let (_,cf)=call(&mut p,0x6000,0,b"sub\\..\\x*.txt");assert_eq!(cf,0);
 let full:Vec<u8>=(0..16).map(|i|p.ram.load(psp+0x300+i,MemWidth::Byte).unwrap()as u8).take_while(|&c|c!=0).collect();assert_eq!(full,b"C:\\X???????.TXT");
 for n in ["A.TXT","B.TXT","C.TXT"]{std::fs::write(dir.join("SUB").join(n),b"").unwrap();}
 let mut names=Vec::new();let mut r=call(&mut p,0x4e00,0,b"SUB\\*.TXT");
 while r.1==0{let found=dta_name(&p).2;let mut path=b"SUB\\".to_vec();path.extend(&found);assert_eq!(call(&mut p,0x4100,0,&path).1,0);names.push(found);r=call(&mut p,0x4f00,0,b"");}
 assert_eq!(names,vec![b"A.TXT".to_vec(),b"B.TXT".to_vec(),b"C.TXT".to_vec(),b"NEW.TXT".to_vec()]);
 assert_eq!(call(&mut p,0x3b00,0,b"SUB").1,0);assert_eq!(call(&mut p,0x3a00,0,b"\\SUB"),(0x10,1),"current directory");
 assert_eq!(call(&mut p,0x3b00,0,b"..").1,0);assert_eq!(call(&mut p,0x3a00,0,b"SUB").1,0);assert!(!dir.join("SUB").exists());
 assert_eq!(call(&mut p,0x4100,0,b"SUB"),(2,1));
 std::fs::remove_dir_all(dir).unwrap();
}
