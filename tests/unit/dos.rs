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
    let b=[0xb4,0x31,0xcd,0x21];let r=run(load(&b,"service.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Service(_)));assert_eq!(r.loaded.cpu.eip,0x102);assert_eq!(r.completed,1);assert_eq!(r.dos_calls,0);
    let r=run(load(&[0xf4],"halt.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)));
    let r=run(load(&[0x90],"boundary.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Boundary(_)));
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
    let p=load(&bytes,"wrapped.exe").unwrap();assert_eq!(p.selectors[1],PSP);assert_eq!(p.selectors[2],PSP);assert_eq!(p.image.start,0x100);assert_eq!(p.cpu.eip,0x100);
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

#[test]
fn dos_character_output_preserves_other_state_and_returns_character(){
 for byte in 0..=255u32{
  let mut p=load(&[0xcd,0x21],"putchar.com").unwrap();p.cpu.regs[0]=0xabcd02ff;p.cpu.regs[2]=0x12345600|byte;p.cpu.eflags=0xed7;p.cpu.undefined_flags=u32::MAX;
  let mut want=p.cpu.clone();want.regs[0]=0xabcd0200|if byte==9{32}else{byte};let ram=p.ram.clone();let image=p.image.clone();let mut out=vec![];
  service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu,want);assert_eq!(p.ram,ram);assert_eq!(p.image,image);assert_eq!(out,if byte==9{vec![32;8]}else{vec![byte as u8]});
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
  assert_eq!(service(&mut p,0x21,&mut out).is_ok(),ok);if ok{assert_eq!(out.len(),MAX_OUTPUT);}else{assert_eq!(out,before);assert_eq!(p.cpu,old);}assert_eq!(p.image,image);assert_eq!(p.ram,ram);
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
  if h<5{assert_eq!(p.cpu.eflags&1,0);assert_eq!(p.cpu.regs[2],0x98760000|if h<3{0x83}else{0x80});}
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
fn test_bios_get_video_mode() {
    let mut loaded = load(&hello("com").unwrap(), "test.com").unwrap();
    let mut out = crate::host_io::CaptureOutput::new(1024);
    loaded.cpu.regs[0] = 0x0f00; // AH=0Fh
    loaded.cpu.regs[3] = 0x1234; // BH != 0
    let res = service(&mut loaded, 0x10, &mut out);
    assert_eq!(res, Ok(None));
    assert_eq!(loaded.cpu.regs[0] & 0xffff, 0x5003); // 80 cols, mode 3
    assert_eq!((loaded.cpu.regs[3] >> 8) & 0xff, 0); // BH = 0
}

#[test]
fn test_dos_handle_write() {
    let mut loaded = load(&hello("com").unwrap(), "test.com").unwrap();
    let mut out = crate::host_io::CaptureOutput::new(1024);
    let msg = b"Testing INT 21h AH=40h";
    let ds_base = u32::from(loaded.selectors[3]) * 16;
    loaded.ram.write_bytes(ds_base + 0x200, msg).unwrap();
    loaded.cpu.regs[0] = 0x4000; // AH=40h
    loaded.cpu.regs[3] = 1;      // BX=1 (stdout)
    loaded.cpu.regs[1] = msg.len() as u32; // CX
    loaded.cpu.regs[2] = 0x200;  // DX
    let res = service(&mut loaded, 0x21, &mut out);
    assert_eq!(res, Ok(None));
    assert_eq!(loaded.cpu.regs[0] & 0xffff, msg.len() as u32);
    assert_eq!(loaded.cpu.eflags & 1, 0);
    assert_eq!(out.bytes(), msg);
}
