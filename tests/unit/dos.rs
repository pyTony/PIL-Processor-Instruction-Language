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
