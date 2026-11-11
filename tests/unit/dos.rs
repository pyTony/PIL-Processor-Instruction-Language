use super::*;
#[test]
fn cga_mode_control_out_retains_latch_without_touching_bios_shadow(){
 // MOV DX,3D8h; MOV AL,21h; OUT DX,AL; IN AL,DX; INT 20h.
 let bytes=[0xba,0xd8,3,0xb0,0x21,0xee,0xec,0xcd,0x20];
 let r=run(load(&bytes,"cga-control.com").unwrap(),20).unwrap();
 assert!(matches!(r.stop,Stop::Exit(0)),"{:?}",r.stop);
 assert_eq!(r.loaded.vga.cga_mode(),0x21);
 assert_eq!(r.loaded.cpu.regs[0]&255,255,"CGA latch is output-only");
 assert_eq!(r.loaded.ram.load(0x465,MemWidth::Byte).unwrap(),0x29);
 let mut session=session::Session::new(r.loaded);
 let (reply,_)=session.command(0,"snapshot").unwrap();
 assert!(reply.contains("\"video_enabled\":false"));
 assert!(reply.contains("\"text_blink\":true"));
}
#[test]
fn cga_mode_control_rejects_layout_changes_atomically_and_bios_resets_it(){
 let mut p=load(&[0xee],"cga-control.com").unwrap();
 for mode in [3,4,5,6,0x12,0x13]{
  int10(&mut p,mode,0,0,0);
  let original=p.vga.cga_mode();let ram=p.ram.clone();
  for bit in [1,2,0x10]{
   let before=p.vga.clone();p.cpu.regs[0]=u32::from(original^bit);p.cpu.regs[2]=0x3d8;
   assert!(ports::io(&mut p).is_err());assert_eq!(p.vga,before);assert_eq!(p.ram,ram);
  }
  p.cpu.regs[0]=u32::from(original^0x28);p.cpu.regs[2]=0x3d8;
  assert_eq!(ports::io(&mut p).unwrap(),Some(1));
  assert_eq!(p.vga.cga_mode(),original^0x28);
  int10(&mut p,mode,0,0,0);assert_eq!(p.vga.cga_mode(),original);
 }
}
#[test]
fn cga_mode_control_color_burst_and_blink_follow_ports_and_bios(){
 let mut p=graphics_mode(4);let ram=p.ram.clone();
 p.vga.io(0x3d8,false,0xee).unwrap();
 assert_eq!(p.vga.cga_mode(),0xee,"retain reserved bits too");
 assert_eq!(p.ram,ram,"direct hardware writes do not change BIOS mode or pixels");
 let mut session=session::Session::new(p);
 let (reply,_)=session.command(0,"snapshot").unwrap();
 assert!(reply.contains("\"cga_mode\":5"));
 let mut p=load(&[0xee],"blink.com").unwrap();
 int10(&mut p,0x1003,0,0,0);assert_eq!(p.vga.cga_mode()&0x20,0);
 int10(&mut p,0x1003,1,0,0);assert_eq!(p.vga.cga_mode()&0x20,0x20);
 p.vga.attribute_io(0x3c0,false,0x10).unwrap();
 p.vga.attribute_io(0x3c0,false,0).unwrap();assert_eq!(p.vga.cga_mode()&0x20,0);
}
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
 let mut p=load(&[0xcd,0x21,0x63],"vectors.com").unwrap();p.cpu.regs[0]=0x35ff;
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
fn mz_allocation_and_stack_are_bounded_and_bytes_past_the_last_page_are_not_loaded(){
    let good=exe(&[0x90;32],&[]);
    for(min,max,ss,sp)in [(0xffff,0xffff,0,0),(2,1,0,0),(0,0,0,0),(0,1,0x9000,0),(0,1,0,0xfffe)]{
        let mut b=good.clone();put_word(&mut b,10,min);put_word(&mut b,12,max);put_word(&mut b,14,ss);put_word(&mut b,16,sp);assert!(load(&b,"bad.exe").is_err());
    }
    // DOS loads the whole last page, not just its declared byte count (BBC
    // BASIC checksums 10 bytes past it); bytes past the last page stay unloaded.
    let mut b=good;put_word(&mut b,12,1);put_word(&mut b,16,0x20);b.extend([0xcc;8]);let p=load(&b,"extra.exe").unwrap();assert_eq!((p.trailing_bytes,p.module_bytes),(0,40));assert_eq!(p.allocated_end,u32::from(LOAD)*16+64);
    assert_eq!(p.ram.load(u32::from(LOAD)*16+32,MemWidth::Byte).unwrap(),0xcc);
    b.resize(520,0xcc);let p=load(&b,"overlay.exe").unwrap();assert_eq!((p.trailing_bytes,p.module_bytes),(8,480));
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
    let mut b=exe(&[0xb8,0,0,0x8e,0xd8,0x63],&[(1,0)]);put_word(&mut b,16,0x100);
    let r=run(load(&b,"startup.exe").unwrap(),100).unwrap();assert!(matches!(r.stop,Stop::MissingEncoding{ref key,..}if key=="opcode 63"));assert_eq!(r.completed,2);assert_eq!(r.loaded.cpu.eip,5);assert_eq!(r.loaded.cpu.regs[0],u32::from(LOAD));assert_eq!(r.file_offset,Some(37));assert_eq!(r.loaded.selectors[3],LOAD);assert_eq!(r.stop_bytes[0],0x63);
}
#[test]
fn unsupported_dos_calls_and_monitor_stops_are_not_cpu_gaps(){
    let b=[0xb4,0xe0,0xcd,0x21];let r=run(load(&b,"service.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Service(_)));assert_eq!(r.loaded.cpu.eip,0x102);assert_eq!(r.completed,1);assert_eq!(r.dos_calls,0);
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
    for(name,bytes)in [("hello world.com",hello("com").unwrap()),("gap1.com",vec![0x63]),("gap2.com",vec![0x63]),("service.com",vec![0xcd,0x10])]{let p=dir.join(name);std::fs::write(&p,bytes).unwrap();paths.push(p.to_str().unwrap().to_owned());}
    let text=report(&paths,100).unwrap();assert!(text.contains(&format!("DOS executable probe v{}; step budget=100 per program",env!("CARGO_PKG_VERSION"))));assert!(text.contains("SUMMARY imported=4/4 exited=1"));assert!(text.contains("2 opcode 63"));assert!(text.contains("Hello, DOS!\\r\\n"));assert!(text.contains("Service("));assert!(report(&[],100).is_err());assert!(report(&paths,0).is_err());std::fs::remove_dir_all(dir).unwrap();
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
 let r=run(load(&[0xb8,0,0x30,0xcd,0x21,0x63],"version.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::MissingEncoding{ref key,..}if key=="opcode 63"));assert_eq!((r.completed,r.pil_steps,r.dos_calls),(2,1,1));assert_eq!(r.loaded.cpu.regs[0],0x1606);
 let r=run(load(&[0xb8,1,0x30,0xcd,0x21,0xb8,0,0x4c,0xcd,0x21],"version.com").unwrap(),10).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!((r.completed,r.pil_steps,r.dos_calls),(4,2,2));
}
#[test]
fn dos_version_failed_psp_read_changes_no_registers_or_output(){
 let mut p=load(&[0xcd,0x21],"version.com").unwrap();p.cpu.regs[0]=0x3000;p.ram=Ram::new(u32::from(PSP)as usize*16+0x41);let old=p.cpu.clone();let oldram=p.ram.clone();let mut out=vec![65];assert!(service(&mut p,0x21,&mut out).is_err());assert_eq!(p.cpu,old);assert_eq!(p.ram,oldram);assert_eq!(out,vec![65]);
}

/// RAM equal outside the active text page and its cursor word.
fn same_outside_screen(a:&Ram,b:&Ram)->bool{
 let(x,y)=(a.bytes(),b.bytes());x.len()==y.len()&&x[..0x450]==y[..0x450]&&x[0x452..0xb8000]==y[0x452..0xb8000]&&x[0xb8fa0..]==y[0xb8fa0..]
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
fn ioctl_status_reports_file_eof_and_device_readiness_without_consuming(){
 let mut p=load_with_drives(&[0xcd,0x21,0xf4],"status.com",vec![Source::Memory(Default::default())],None).unwrap();let mut out=vec![];
 let (handle,cf)=call(&mut p,0x3d00,0,b"status.com");assert_eq!(cf,0);
 fn status(p:&mut Loaded,out:&mut Vec<u8>,sub:u32,handle:u32)->(u32,u32){
  p.cpu.regs[0]=0xabcd4400|sub;p.cpu.regs[3]=handle;p.cpu.regs[2]=0x12345678;
  p.cpu.eflags=0xad7;p.cpu.undefined_flags=0x891;
  service(p,0x21,out).unwrap();
  assert_eq!(p.cpu.regs[2],0x12345678);assert_eq!(p.cpu.eflags&!1,0xad6);assert_eq!(p.cpu.undefined_flags,0x890);
  (p.cpu.regs[0],p.cpu.eflags&1)
 }
 assert_eq!(status(&mut p,&mut out,6,handle),(0xabcd44ff,0));
 assert_eq!(status(&mut p,&mut out,6,handle),(0xabcd44ff,0));
 p.cpu.regs[0]=0x4500;p.cpu.regs[3]=handle;service(&mut p,0x21,&mut out).unwrap();let dup=p.cpu.regs[0]&0xffff;
 p.cpu.regs[0]=0x3f00;p.cpu.regs[3]=dup;p.cpu.regs[1]=3;p.cpu.regs[2]=0x400;service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,3);
 assert_eq!(status(&mut p,&mut out,6,handle),(0xabcd4400,0));
 assert_eq!(status(&mut p,&mut out,7,handle),(0xabcd44ff,0));
 p.cpu.regs[0]=0x4200;p.cpu.regs[3]=handle;p.cpu.regs[1]=0;p.cpu.regs[2]=1;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!(status(&mut p,&mut out,6,dup),(0xabcd44ff,0));
 p.cpu.regs[0]=0x4200;p.cpu.regs[3]=dup;p.cpu.regs[1]=0;p.cpu.regs[2]=9;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!(status(&mut p,&mut out,6,handle),(0xabcd4400,0));
 let (created,cf)=call(&mut p,0x3c00,0,b"Z:\\STATUS.TMP");assert_eq!(cf,0);
 p.cpu.regs[0]=0x3e00;p.cpu.regs[3]=created;service(&mut p,0x21,&mut out).unwrap();
 let (write_only,cf)=call(&mut p,0x3d01,0,b"Z:\\STATUS.TMP");assert_eq!(cf,0);
 assert_eq!(status(&mut p,&mut out,6,write_only),(0xabcd44ff,0));
 assert_eq!(status(&mut p,&mut out,7,write_only),(0xabcd44ff,0));
 assert_eq!(status(&mut p,&mut out,6,19),(0xabcd0006,1));
 assert_eq!(status(&mut p,&mut out,6,0),(0xabcd4400,0));
 p.enqueue_key(0x1e61).unwrap();
 assert_eq!(status(&mut p,&mut out,6,0),(0xabcd44ff,0));
 assert_eq!(status(&mut p,&mut out,6,0),(0xabcd44ff,0));
 assert_eq!(status(&mut p,&mut out,7,1),(0xabcd44ff,0));
 let (nul,cf)=call(&mut p,0x3d00,0,b"NUL");assert_eq!(cf,0);
 assert_eq!(status(&mut p,&mut out,6,nul),(0xabcd44ff,0));
 assert_eq!(status(&mut p,&mut out,7,nul),(0xabcd44ff,0));
 serial::attach(&mut p).unwrap();
 assert_eq!(status(&mut p,&mut out,6,3),(0xabcd4400,0));
 p.com1.as_mut().unwrap().rx.push_back(b'X');
 assert_eq!(status(&mut p,&mut out,6,3),(0xabcd44ff,0));
 assert_eq!(p.com1.as_ref().unwrap().rx.front(),Some(&b'X'));
 assert_eq!(status(&mut p,&mut out,7,3),(0xabcd44ff,0));
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
#[test]
fn bios_enhanced_keyboard_checks_and_reads_preserve_extended_words(){
 let mut p=load(&[0xcd,0x16],"keys.com").unwrap();let mut out=vec![];
 p.cpu.regs[0]=0xabcd1100;p.cpu.eflags=0x203;p.cpu.undefined_flags=0x40;
 service(&mut p,0x16,&mut out).unwrap();assert_eq!(p.cpu.regs[0],0xabcd1100);assert_eq!(p.cpu.eflags,0x243);assert_eq!(p.cpu.undefined_flags,0);
 assert!(p.enqueue_key(0x48e0).unwrap());assert!(p.enqueue_key(0x8500).unwrap());
 for key in [0x48e0,0x8500]{
  let ram=p.ram.clone();p.cpu.regs[0]=0xabcd1100;service(&mut p,0x16,&mut out).unwrap();
  assert_eq!(p.cpu.regs[0],0xabcd0000|key);assert_eq!(p.cpu.eflags&0x40,0);assert_eq!(p.ram,ram);
  p.cpu.regs[0]=0xabcd1000;let flags=p.cpu.eflags;service(&mut p,0x16,&mut out).unwrap();
  assert_eq!(p.cpu.regs[0],0xabcd0000|key);assert_eq!(p.cpu.eflags,flags);
 }
 let mut p=load(&[0xcd,0x16,0x89,0xc3,0xb8,0,0x4c,0xcd,0x21],"keys.com").unwrap();
 p.cpu.regs[0]=0x1000;let old=p.clone();let r=run(p,10).unwrap();
 assert_eq!(r.stop,Stop::WaitingForInput);assert_eq!(r.completed,0);assert_eq!(r.loaded.cpu,old.cpu);assert_eq!(r.loaded.ram,old.ram);
 let mut p=r.loaded;p.enqueue_key(0x48e0).unwrap();let r=run(p,10).unwrap();
 assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.loaded.cpu.regs[3],0x48e0);
}
#[test]
fn bios_compaq_speed_probe_reports_absent_without_touching_state(){
 let mut p=load(&[0xcd,0x16],"probe.com").unwrap();p.enqueue_key(0x1e61).unwrap();
 p.cpu.regs[0]=0xabcd_f1ff;p.cpu.eflags=0xad7;p.cpu.undefined_flags=0x891;let old=p.clone();
 service(&mut p,0x16,&mut vec![]).unwrap();assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);
 p.cpu.regs[0]=0xf100;let old=p.clone();assert!(service(&mut p,0x16,&mut vec![]).is_err());
 assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);
}
#[test]
fn bios_disk_geometry_reports_unsupported_without_fabricating_a_drive(){
 let mut p=load(&[0xcd,0x13],"disk.com").unwrap();p.cpu.regs=[0xabcd0801,0x12345678,0x98766480,0xaaaa5555,0,0,0,0];
 p.cpu.eflags=0x202;p.cpu.undefined_flags=1;let old=p.clone();
 service(&mut p,0x13,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0],0xabcd0101);
 assert_eq!(&p.cpu.regs[1..],&old.cpu.regs[1..]);assert_eq!(p.cpu.eflags,0x203);assert_eq!(p.cpu.undefined_flags,0);assert_eq!(p.ram,old.ram);
}
#[test]
fn ioctl_block_access_flag_checks_drive_and_writes_only_result_byte(){
 let mut p=load(&[0xcd,0x21],"ioctl.com").unwrap();let at=p.image.segments[3].base+0x300;
 p.ram.store(at,MemWidth::Word,0xaa00).unwrap();p.cpu.regs[0]=0x440d;p.cpu.regs[1]=0x0867;p.cpu.regs[2]=0x300;p.cpu.regs[3]=0;
 service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.eflags&1,0);assert_eq!(p.ram.load(at,MemWidth::Word).unwrap(),0x0100);
 p.cpu.regs[0]=0x440d;p.cpu.regs[3]=27;let ram=p.ram.clone();service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[0]&0xffff,15);assert_eq!(p.cpu.eflags&1,1);assert_eq!(p.ram,ram);
 p.cpu.regs[0]=0x440d;p.cpu.regs[3]=0;p.ram.protect(at+1,1).unwrap();let old=p.clone();
 assert!(service(&mut p,0x21,&mut vec![]).is_err());assert_eq!(p.cpu,old.cpu);assert_eq!(p.ram,old.ram);
}
#[test]
fn ioctl_logical_drive_map_has_one_letter_and_rejects_invalid_drive(){
 let mut p=load(&[0xcd,0x21],"map.com").unwrap();p.cpu.regs[0]=0x1234440e;p.cpu.regs[3]=0;
 service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0],0x12344400);assert_eq!(p.cpu.eflags&1,0);
 p.cpu.regs[0]=0x440e;p.cpu.regs[3]=27;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[0]&0xffff,15);assert_eq!(p.cpu.eflags&1,1);
}
#[test]
fn eisa_slot_probe_reports_unsupported_without_modifying_slot_state(){
 let mut p=load(&[0xcd,0x15],"eisa.com").unwrap();p.cpu.regs[0]=0x1234d800;p.cpu.regs[1]=0x5678;
 p.cpu.eflags=0x202;p.cpu.undefined_flags=1;let old=p.clone();service(&mut p,0x15,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[0],0x12348600);assert_eq!(&p.cpu.regs[1..],&old.cpu.regs[1..]);
 assert_eq!(p.cpu.eflags,0x203);assert_eq!(p.cpu.undefined_flags,0);assert_eq!(p.ram,old.ram);
}
#[test]
fn dos_date_advances_when_virtual_clock_crosses_midnight(){
 let mut p=load(&[0xcd,0x21],"midnight.com").unwrap();
 p.cpu.regs[0]=0x2b00;p.cpu.regs[1]=2026;p.cpu.regs[2]=0x0b07;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.rtc.read(None,6),Some(7));
 p.cpu.regs[0]=0x2d00;p.cpu.regs[1]=0x173b;p.cpu.regs[2]=0x3b00;service(&mut p,0x21,&mut vec![]).unwrap();
 p.timer_ticks=TICKS_PER_DAY-1;p.cpu.regs[0]=0;service(&mut p,0x1a,&mut vec![]).unwrap();
 p.cpu.regs[0]=0x2a00;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!((p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff,p.cpu.regs[0]&255),(2026,0x0b08,0));
}
#[test]
fn keyboard_echo_uses_controller_response_without_consuming_bios_keys(){
 // Send EE, inspect output-full, read the response, inspect empty status.
 let code=[0xb0,0xee,0xe6,0x60,0xe4,0x64,0x88,0xc3,0xe4,0x60,0x88,0xc7,0xe4,0x64,0x88,0xc1,0xfa,0xf4];
 let mut p=load(&code,"kbecho.com").unwrap();p.enqueue_key(0x1e61).unwrap();
 let r=run(p,40).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert_eq!(r.loaded.cpu.regs[3]&0xffff,0xee01);assert_eq!(r.loaded.cpu.regs[1]&0xff,0);
 let mut p=r.loaded;p.cpu.regs[0]=0x1000;service(&mut p,0x16,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,0x1e61);
 // An unread response must not be silently replaced.
 let code=[0xb0,0xee,0xe6,0x60,0xe6,0x60,0xfa,0xf4];let r=run(load(&code,"full.com").unwrap(),20).unwrap();
 assert!(matches!(r.stop,Stop::Service(_)));let mut p=r.loaded;
 p.cpu.eip=0x106;p.ram.store(p.image.segments[1].base+0x106,MemWidth::Byte,0xe4).unwrap();
 p.ram.store(p.image.segments[1].base+0x107,MemWidth::Byte,0x60).unwrap();
 assert_eq!(ports::io(&mut p).unwrap(),Some(2));assert_eq!(p.cpu.regs[0]&255,0xee);
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
fn mandelbrot_guest_writes_pixels_into_mode_13_vga_memory(){
    let bytes=crate::mandelbrot::guest_vga_bytes(8,16).unwrap();
    let r=run(load(&bytes,"mandel.com").unwrap(),200_000).unwrap();
    assert!(matches!(r.stop,Stop::WaitingForInput),"{:?}",r.stop);
    assert_eq!(r.loaded.ram.load(0x449,MemWidth::Byte).unwrap(),0x13);
    let reference=crate::mandelbrot::reference(8,16).unwrap();
    for y in 0..8 {for x in 0..8 {assert_eq!(r.loaded.ram.load(0xa0000+(96+y)*320+156+x,MemWidth::Byte).unwrap(),u32::from(reference[(y*8+x) as usize]));}}
    assert_eq!(r.loaded.ram.load(0xa0000+63999,MemWidth::Byte).unwrap(),0);
}

#[test]
fn cga_modes_bios_pixels_and_memory_banks(){
 for (mode,width,cols) in [(4u32,320u32,40u32),(6,640,80)]{
  let mut p=load(&[0xf4],"cga.com").unwrap();
  p.ram.store(0xb8000,MemWidth::Byte,0xaa).unwrap();
  p.cpu.regs[0]=mode|0x80;console::display(&mut p,0).unwrap();
  assert_eq!(p.ram.load(0xb8000,MemWidth::Byte).unwrap(),0xaa);
  p.cpu.regs[0]=mode;console::display(&mut p,0).unwrap();
  assert_eq!(p.ram.load(0xb8000,MemWidth::Byte).unwrap(),0);
  assert_eq!(p.ram.load(0x44a,MemWidth::Word).unwrap(),cols);
  let col=if mode==4{3}else{1};
  for (x,y,addr,shift) in [(0,0,0xb8000,if mode==4{6}else{7}),
                            (width-1,0,0xb8000+79,0),(0,1,0xba000,if mode==4{6}else{7}),
                            (width-1,199,0xba000+99*80+79,0)]{
   p.cpu.regs[0]=0x0c00|col;p.cpu.regs[1]=x;p.cpu.regs[2]=y;p.cpu.regs[3]=0;
   service(&mut p,0x10,&mut vec![]).unwrap();
   assert_eq!((p.ram.load(addr,MemWidth::Byte).unwrap()>>shift)&col,col);
   p.cpu.regs[0]=0x0d00;service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&255,col);
   p.cpu.regs[0]=0x0c80|col;service(&mut p,0x10,&mut vec![]).unwrap();
   p.cpu.regs[0]=0x0d00;service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&255,0);
  }
  p.cpu.regs[0]=0x0c01;p.cpu.regs[1]=width;p.cpu.regs[2]=0;
  assert!(service(&mut p,0x10,&mut vec![]).is_err());
  p.cpu.regs[0]=3;console::display(&mut p,0).unwrap();assert_eq!(p.ram.load(0x449,MemWidth::Byte).unwrap(),3);
 }
}
#[test]
fn cga_palette_bios_and_hardware_register_are_independent(){
 let mut p=load(&[0xf4],"cga.com").unwrap();p.cpu.regs[0]=4;console::display(&mut p,0).unwrap();
 p.cpu.regs[3]=0x0012;console::display(&mut p,0x0b).unwrap();assert_eq!(p.vga.cga_color(),0x32);
 p.cpu.regs[3]=0x0100;console::display(&mut p,0x0b).unwrap();assert_eq!(p.vga.cga_color(),0x12);
 p.vga.io(0x3d9,false,0x2f).unwrap();assert_eq!(p.vga.cga_color(),0x2f);
 assert_eq!(p.ram.load(0x466,MemWidth::Byte).unwrap(),0x12);
}
#[test]
fn cga_border_selection_works_in_colour_text_mode(){
 let mut p=load(&[0xf4],"border.com").unwrap();
 p.ram.store(0x466,MemWidth::Byte,0x30).unwrap();
 p.ram.store(0xb8000,MemWidth::Word,0x1e41).unwrap();
 p.cpu.regs[3]=0x0004;console::display(&mut p,0x0b).unwrap();
 assert_eq!(p.ram.load(0x466,MemWidth::Byte).unwrap(),0x24);
 assert_eq!(p.vga.cga_color(),0x24);
 assert_eq!(p.ram.load(0xb8000,MemWidth::Word).unwrap(),0x1e41);
 p.cpu.regs[3]=0x0101;assert!(console::display(&mut p,0x0b).is_err());
}
#[test]
fn absent_game_port_and_bios_joystick_service_report_no_device(){
 let mut p=load(&[0xf4],"joy.com").unwrap();p.cpu.regs[0]=0x1234_8402;p.cpu.regs[2]=1;p.cpu.eflags=2;
 service(&mut p,0x15,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0],0x1234_8602);assert_eq!(p.cpu.regs[2],1);assert_eq!(p.cpu.eflags&1,1);
 // OUT DX,AL then IN AL,DX, absent bus reads high.
 let code=[0xba,0x01,0x02,0xb0,0x12,0xee,0xec,0xfa,0xf4];
 let r=run(load(&code,"joy.com").unwrap(),100).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)));assert_eq!(r.loaded.cpu.regs[0]&255,255);
}
#[test]
fn cga_color_register_accepts_guest_out_instruction(){
 let guest=[0xba,0xd9,0x03,0xb0,0x2f,0xee,0xf4];
 let r=run(load(&guest,"color.com").unwrap(),100).unwrap();
 assert_eq!(r.loaded.vga.cga_color(),0x2f);
}
#[test]
fn mode13_bios_pixel_is_full_eight_bit_index(){
 let mut p=load(&[0xf4],"pixel.com").unwrap();p.cpu.regs[0]=0x13;console::display(&mut p,0).unwrap();
 p.cpu.regs[0]=0x0caa;p.cpu.regs[1]=319;p.cpu.regs[2]=199;p.cpu.regs[3]=0;
 service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.ram.load(0xafa00-1,MemWidth::Byte).unwrap(),0xaa);
 p.cpu.regs[0]=0x0d00;service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&255,0xaa);
}

#[test]
fn vga_mode13_clears_or_preserves_memory_and_returns_to_text(){
    let mut p=load(&[0xf4],"mode.com").unwrap();
    p.ram.store(0xa0000,MemWidth::Byte,0x55).unwrap();p.ram.store(0xafa00,MemWidth::Byte,0x77).unwrap();
    p.cpu.regs[0]=0x93;console::display(&mut p,0).unwrap();
    assert_eq!(p.ram.load(0xa0000,MemWidth::Byte).unwrap(),0x55);
    p.cpu.regs[0]=0x13;console::display(&mut p,0).unwrap();
    assert_eq!(p.ram.load(0xa0000,MemWidth::Byte).unwrap(),0);assert_eq!(p.ram.load(0xafa00,MemWidth::Byte).unwrap(),0x77);
    p.cpu.regs[0]=3;console::display(&mut p,0).unwrap();assert_eq!(p.ram.load(0x449,MemWidth::Byte).unwrap(),3);
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
 for al in [0u32,1,2,7,0x87]{let mut p=load(&[0xcd,0x10],"mode.com").unwrap();p.cpu.regs[0]=al;let old=p.clone();assert!(service(&mut p,0x10,&mut vec![]).is_err());assert_eq!(p.ram,old.ram);}
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
 // ENTER 0010,2 with BP=ABCD and [SS:ABCB]=5555: push ABCD, copy [ABCB], push the frame
 // pointer FFFC; BP=FFFC, SP=FFFC-4-10h.
 let mut p=load(&[0xc8,0x10,0x00,0x02,0xf4],"enter2.com").unwrap();p.cpu.regs[5]=0x1111abcd;
 let ss=u32::from(p.selectors[SegmentRegister::Ss.index()])*16;p.ram.store(ss+0xabcb,MemWidth::Word,0x5555).unwrap();
 let sp=p.cpu.regs[4]&0xffff;let r=run(p,10).unwrap();
 assert_eq!(r.loaded.cpu.regs[5],0x11110000|(sp-2));assert_eq!(r.loaded.cpu.regs[4]&0xffff,sp-6-0x10);
 assert_eq!([sp-2,sp-4,sp-6].map(|o|r.loaded.ram.load(ss+o,MemWidth::Word).unwrap()),[0xabcd,0x5555,sp-2]);
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
 for ax in [0x0100u16,0x1100,0x1600,0x1680,0x1687,0x1700,0x1a00,0x4300,0x4a11,0xb710]{
  let [lo,hi]=ax.to_le_bytes();let r=run(load(&[0xb8,lo,hi,0xcd,0x2f,0xfa,0xf4],"mux.com").unwrap(),10).unwrap();
  assert_eq!(r.loaded.cpu.regs[0]&0xffff,u32::from(ax));assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 }
 // AX=4310h (XMS entry point) is only meaningful once XMS is present: still a stop.
 let r=run(load(&[0xb8,0x10,0x43,0xcd,0x2f,0xfa,0xf4],"xms.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Service(_)));
 // MOV AH,12; MOV BL,10; INT 10: no EGA, BL stays 10h. INT 15 AH=C0: CF=1, AH=86h.
 // The VGA BIOS answers the EGA information call: colour, 256 KiB.
 let r=run(load(&[0xb4,0x12,0xb3,0x10,0xcd,0x10,0xfa,0xf4],"ega.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[3]&0xffff,0x0003);
 let r=run(load(&[0xb4,0xc0,0xcd,0x15,0xfa,0xf4],"config.com").unwrap(),10).unwrap();assert_eq!((r.loaded.cpu.regs[0]>>8)&255,0x86);assert_eq!(r.loaded.cpu.eflags&1,1);
}
#[test]
fn true_version_default_drive_and_handle_count_answers(){
 // MOV AX,3306; MOV BX,FFFF; MOV DX,FFFF; INT 21: BX=true version 6.22, DX=0.
 let r=run(load(&[0xb8,0x06,0x33,0xbb,0xff,0xff,0xba,0xff,0xff,0xcd,0x21,0xfa,0xf4],"truever.com").unwrap(),10).unwrap();
 assert_eq!((r.loaded.cpu.regs[3]&0xffff,r.loaded.cpu.regs[2]&0xffff,r.loaded.cpu.regs[0]&0xffff),(0x1606,0,0x3306));assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 // BREAK: AL=0 reads DL=0 (off), AL=1 sets it from DL bit 0, AL=2 swaps; AL=3 is not answered.
 let mut p=load(&[0xcd,0x21],"break.com").unwrap();
 let ah33=|p:&mut Loaded,al:u32,dl:u32|{p.cpu.regs[0]=0x3300|al;p.cpu.regs[2]=0xff00|dl;service(p,0x21,&mut vec![]).map(|_|p.cpu.regs[2]&0xffff)};
 assert_eq!(ah33(&mut p,0,0x55),Ok(0xff00));assert_eq!(ah33(&mut p,1,1),Ok(0xff01));assert!(p.break_check);
 assert_eq!(ah33(&mut p,2,0),Ok(0xff01));assert!(!p.break_check);assert!(ah33(&mut p,3,0).is_err());
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
 // AX=7147 (LFN get directory) -> AX=7100, CF=1. INT 15 AH=88 -> AX=15360 KiB, CF=0.
 let r=run(load(&[0xb8,0x47,0x71,0xcd,0x21,0xfa,0xf4],"lfn.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x7100);assert_eq!(r.loaded.cpu.eflags&1,1);
 let r=run(load(&[0xb4,0x88,0xf9,0xcd,0x15,0xfa,0xf4],"ext.com").unwrap(),10).unwrap();assert_eq!(r.loaded.cpu.regs[0]&0xffff,15360);assert_eq!(r.loaded.cpu.eflags&1,0);
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
fn buffered_session_defers_frames_but_keeps_output_and_key_wait(){
 let code=[0xb4,2,0xb2,b'X',0xcd,0x21,0xb4,0,0xcd,0x16,0xcd,0x20];
 let mut s=session::Session::new(load(&code,"BUFFER.COM").unwrap());
 let(reply,_)=s.command(1,"snapshot").unwrap();assert!(reply.contains("\"buffered_run\":true"));
 let(reply,_)=s.command(2,"run 100 buffered").unwrap();
 assert!(reply.contains("\"frame_pending\":true")&&reply.contains("\"status\":\"waiting_input\""),"{reply}");
 assert!(!reply.contains("\"screen\":"));assert!(reply.contains("\"output\":\"X\""));
 let(reply,_)=s.command(3,"snapshot").unwrap();assert!(reply.contains("\"screen\":\"5807"),"{reply}");
 s.key(13,28).unwrap();let(reply,_)=s.command(4,"run 100 buffered").unwrap();assert!(reply.contains("\"status\":\"exit\""),"{reply}");
 let mut p=load(&code,"SERIAL.COM").unwrap();serial::attach(&mut p).unwrap();
 let mut s=session::Session::new(p);s.command(1,"console com1").unwrap();
 let(reply,_)=s.command(2,"run 100 buffered").unwrap();assert!(reply.contains("\"com1\":\"58\""),"{reply}");
 let(reply,_)=s.command(3,"snapshot").unwrap();assert!(reply.contains("\"com1\":\"\""),"{reply}");
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
#[test]
fn keyboard_poll_hint_keeps_guest_running_and_accepts_input(){
 // Poll INT 16h until a key arrives, read it, then exit.
 let code=[0xb4,1,0xcd,0x16,0x74,0xfa,0xb4,0,0xcd,0x16,0xb8,0,0x4c,0xcd,0x21];
 let mut s=session::Session::new(load(&code,"poll.com").unwrap());
 let(reply,_)=s.command(1,"run 20000").unwrap();
 assert!(reply.contains("\"idle_hint\":true")&&reply.contains("\"status\":\"running\""));
 assert!(s.key(b'x',0x2d).unwrap());
 let(reply,_)=s.command(2,"run 20000").unwrap();assert!(reply.contains("\"status\":\"exit\""));
}
#[test]
fn session_routes_console_to_com1_before_guest_execution(){
 let code=[0xb4,2,0xb2,b'H',0xcd,0x21,0xb4,1,0xcd,0x21,0xb4,0x4c,0xcd,0x21];
 let mut p=load(&code,"headless.com").unwrap();serial::attach(&mut p).unwrap();
 let mut s=session::Session::new(p);s.command(1,"console com1").unwrap();
 let(reply,_)=s.command(2,"run 100").unwrap();assert!(reply.contains("\"status\":\"waiting_input\"")&&reply.contains("\"com1\":\"48\""),"{reply}");
 s.command(3,"com1 78").unwrap();let(reply,_)=s.command(4,"run 100").unwrap();
 assert!(reply.contains("exit code 120")&&reply.contains("\"com1\":\"78\""),"{reply}");
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
 for d in [&b"."[..],b"..",b"\\"]{assert_eq!(call(&mut p,0x4300,0,d),(0x4300,0));assert_eq!(p.cpu.regs[1]&0xffff,0x10);}
 assert_eq!(call(&mut p,0x3b00,0,b"..").1,0);assert_eq!(call(&mut p,0x4300,0,b"."),(0x4300,0));assert_eq!(p.cpu.regs[1]&0xffff,0x10);
 assert_eq!(call(&mut p,0x4300,0,b".."),(3,1));assert_eq!(call(&mut p,0x4300,0,b""),(3,1));
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
 // 128/1 does not fit a signed byte: divide error, which the DOS default
 // handler ends with errorlevel 127.
 let r=run(load(&[0xb8,0x80,0x00,0xb1,0x01,0xf6,0xf9,0xfa,0xf4],"idivde.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Exit(0x7f)),"{:?}",r.stop);
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
fn exec_overlay_loads_relocates_and_executes_without_a_child(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-overlay-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(&dir).unwrap();
 // MOV AX,1234h; RETF. The relocation is data following the code and the
 // factor is deliberately different from the destination segment.
 let mut module=vec![0xb8,0x34,0x12,0xcb,0xf0,0xff,0x23,0x01];
 std::fs::write(dir.join("CODE.OVL"),exe(&module,&[(4,0),(6,0)])).unwrap();
 let parent=[0xba,0x00,0x02,0xbb,0x40,0x02,0xb8,0x03,0x4b,0xcd,0x21,0x9a,0,0,0,0x30,0xfa,0xf4];
 let mut p=load_with_drives(&parent,"PARENT.COM",vec![Source::Host(dir.clone())],None).unwrap();let base=u32::from(PSP)*16;
 p.ram.write_bytes(base+0x200,b"CODE.OVL\0").unwrap();p.ram.write_bytes(base+0x240,&[0,0x30,0x35,0x12]).unwrap();
 let arena=p.arena.clone();let r=run(p,30).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x1234);assert_eq!(r.loaded.ram.load(0x30004,MemWidth::Word).unwrap(),0x1225);assert_eq!(r.loaded.ram.load(0x30006,MemWidth::Word).unwrap(),0x1358);
 assert_eq!((r.loaded.psp,r.loaded.exec.len()),(PSP,0));assert_eq!(r.loaded.arena,arena);
 // A raw overlay loads at offset zero as well, without applying the factor.
 module[..4].copy_from_slice(&[0xb8,0x78,0x56,0xcb]);std::fs::write(dir.join("RAW.OVL"),&module).unwrap();
 let mut p=load_with_drives(&[0xcd,0x21],"PARENT.COM",vec![Source::Host(dir.clone())],None).unwrap();
 p.image.end=0x1000; // Overlay goes in a separately allocated executable area.
 p.ram.write_bytes(base+0x200,b"RAW.OVL\0").unwrap();p.ram.write_bytes(base+0x240,&[0,0x30,0x35,0x12]).unwrap();p.cpu.regs[0]=0xbeef4b03;p.cpu.regs[2]=0x200;p.cpu.regs[3]=0x240;p.cpu.eflags|=1;p.cpu.undefined_flags|=1;
 let mut expected=p.cpu.clone();expected.eflags&=!1;expected.undefined_flags&=!1;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu,expected);assert_eq!(&p.ram.bytes()[0x30000..0x30008],&module);
 // A second load invalidates the old decoded code through its signature.
 let mut image=p.image.clone();image.selectors[1]=0x3000;image.segments[1]=SegmentCache::real_mode(0x3000);
 let first=p.cache.fetch(&image,&p.ram,0).unwrap();assert!(first.label.contains("0x5678"),"{}",first.label);
 std::fs::write(dir.join("RAW.OVL"),[0xb8,0xbc,0x9a,0xcb]).unwrap();p.cpu.regs[0]=0x4b03;service(&mut p,0x21,&mut vec![]).unwrap();
 let second=p.cache.fetch(&image,&p.ram,0).unwrap();assert!(second.label.contains("0x9abc"),"{}",second.label);
 // An invalid relocation and a missing file report DOS errors without writing.
 std::fs::write(dir.join("BAD.OVL"),exe(&[1,2,3],&[(3,0)])).unwrap();
 for(name,error)in [(b"BAD.OVL\0".as_slice(),11),(b"MISSING.OVL\0".as_slice(),2)]{
  p.ram.write_bytes(base+0x200,name).unwrap();let before=p.ram.clone();p.cpu.regs[0]=0x4b03;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.eflags&1),(error,1));assert_eq!(p.ram,before);
 }
 // A protected destination fails before any of the overlay is written.
 p.ram.write_bytes(base+0x200,b"CODE.OVL\0").unwrap();p.ram.protect(0x30004,1).unwrap();let before=p.ram.clone();p.cpu.regs[0]=0x4b03;assert!(service(&mut p,0x21,&mut vec![]).is_err());assert_eq!(p.ram,before);
 std::fs::write(dir.join("BIG.OVL"),vec![0x90;0x10020]).unwrap();p.ram.set_a20_enabled(true);p.ram.write_bytes(base+0x200,b"BIG.OVL\0").unwrap();p.ram.write_bytes(base+0x240,&[0xff,0xff,0,0]).unwrap();
 let before=p.ram.clone();p.cpu.regs[0]=0x4b03;assert!(service(&mut p,0x21,&mut vec![]).is_err());assert_eq!(p.ram,before);
 std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn exec_runs_a_com_and_a_relocated_exe_and_returns_their_codes(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-exec-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(&dir).unwrap();
 // CHILD.COM: MOV AL,[80h]; MOV AH,4C; INT 21 (exits with its tail length).
 // Control registers belong to the CPU, so the parent retains child changes.
 std::fs::write(dir.join("child.com"),[0x66,0xb8,0x1e,0,0,0,0x0f,0x22,0xc0,
  0x66,0xb8,0x78,0x56,0x34,0x12,0x0f,0x22,0xd0,
  0x66,0xb8,0,0x40,0,0,0x0f,0x22,0xd8,0xa0,0x80,0x00,0xb4,0x4c,0xcd,0x21]).unwrap();
 // CHILD.EXE: one relocation (MOV AX,seg), then exits with the byte 7 from that segment.
 let mut exe=vec![0x4d,0x5a,0x2d,0x00,0x01,0x00,0x01,0x00,0x02,0x00,0x00,0x00,0xff,0xff,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x00,0x00,0x1c,0x00,0x00,0x00,0x01,0x00,0x00,0x00];
 exe.extend([0xb8,0x00,0x00,0x8e,0xd8,0xa0,0x0c,0x00,0xb4,0x4c,0xcd,0x21,0x07]);std::fs::write(dir.join("child.exe"),exe).unwrap();
 // Shrinks itself to 100h paragraphs, EXECs CHILD.COM with tail " hi", CHILD.EXE and
 // NONE.COM; SI and DI get the AH4D codes, BP the error from the missing file.
 let parent=[0xbb,0x00,0x01,0xb4,0x4a,0xcd,0x21,0x8c,0x0e,0x48,0x01,0x8c,0x0e,0x4c,0x01,0x8c,0x0e,0x50,0x01,0xba,0x67,0x01,0xbb,0x44,0x01,0xb8,0x00,0x4b,0xcd,0x21,0xb4,0x4d,0xcd,0x21,0x89,0xc6,0xba,0x71,0x01,0xbb,0x44,0x01,0xb8,0x00,0x4b,0xcd,0x21,0xb4,0x4d,0xcd,0x21,0x89,0xc7,0xba,0x7b,0x01,0xbb,0x44,0x01,0xb8,0x00,0x4b,0xcd,0x21,0x89,0xc5,0xfa,0xf4,
  0x00,0x00,0x52,0x01,0x00,0x00,0x57,0x01,0x00,0x00,0x57,0x01,0x00,0x00,0x03,0x20,0x68,0x69,0x0d,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
 let mut code=parent.to_vec();code.extend(b"CHILD.COM\0CHILD.EXE\0NONE.COM\0");
 let p=load_with_drives(&code,"PARENT.COM",vec![Source::Host(dir.clone())],None).unwrap();
 struct Identities(Vec<ProgramIdentity>);
 impl InstructionBackend for Identities{
  fn execute(&mut self,id:&ProgramIdentity,_:&Image,_:&std::rc::Rc<crate::guest::Decoded>,_:&mut Cpu,_:&mut Ram,_:&mut crate::address::SegmentState)->Option<Result<(),crate::flow::ExecError>>{
   if !self.0.contains(id){self.0.push(id.clone());}None
  }
 }
 let mut seen=Identities(Vec::new());let mut output=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
 let r=run_with_backend(p,200,&mut output,Some(&mut seen)).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert!(seen.0.iter().any(|id|id.path.as_deref()==Some(b"C:\\CHILD.COM")));
 assert!(seen.0.iter().any(|id|id.path.as_deref()==Some(b"C:\\CHILD.EXE")));
 assert_eq!(r.loaded.active_program.path,None);
 assert_eq!(r.loaded.active_program.bytes.as_slice(),code.as_slice());
 let regs=&r.loaded.cpu.regs;assert_eq!((regs[6]&0xffff,regs[7]&0xffff,regs[5]&0xffff),(3,7,2));
 // Back in the parent: its PSP again, the children's blocks free, no frames left.
 assert_eq!((r.loaded.psp,r.loaded.exec.len()),(PSP,0));
 assert_eq!((r.loaded.image.msw,r.loaded.image.controls),(0x1e,[0x12345678,0x4000]));
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
fn absent_vga_extension_registers_ignore_writes_and_read_ff(){
 for port in [0xcd,0xde,0xdf]{
  let p=load(&[0xba,port,3,0xb0,0xaa,0xee,0xec],"bank.com").unwrap();let before=p.vga.clone();
  let r=run(p,4).unwrap();assert_eq!(r.completed,4);assert_eq!(r.loaded.cpu.regs[0]&255,255);assert_eq!(r.loaded.vga,before);
 }
 for mode in [3,0x12]{for (port,index) in [(0x3c4,6),(0x3ce,9)]{
  let mut p=graphics_mode(mode);planar::io(&mut p,port,false,index,false).unwrap();
  let idle=p.vga.idle.clone();let planes=p.ram.planar().cloned();let ram=p.ram.bytes().to_vec();
  for value in [0xff,0]{
   planar::io(&mut p,port+1,false,value,false).unwrap();
   assert_eq!(planar::io(&mut p,port+1,true,0,false).unwrap(),Some(0xff));
  }
  assert_eq!(p.vga.idle,idle);assert_eq!(p.ram.planar(),planes.as_ref());assert_eq!(p.ram.bytes(),ram);
 }}
}
#[test]
fn bios_user_font_load_stores_glyphs_and_rejects_bad_spans_atomically(){
 let mut p=graphics_mode(3);let es=p.image.segments[0].base;
 for row in 0..16{p.ram.store(es+0x300+row,MemWidth::Byte,row+1).unwrap();}
 p.cpu.regs[5]=0x300;p.cpu.regs[0]=0x1100;p.cpu.regs[3]=0x1000;p.cpu.regs[1]=1;p.cpu.regs[2]=220;
 let cpu=p.cpu.clone();service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu,cpu);
 for row in 0..16{assert_eq!(p.vga.font.as_ref().unwrap().peek(2,220*32+row),row as u8+1);}
 assert_eq!(p.cpu.regs[5],cpu.regs[5]);let before=p.vga.font.clone();
 p.cpu.regs[0]=0x1100;p.cpu.regs[5]=0xfff8;
 assert!(service(&mut p,0x10,&mut vec![]).is_err());assert_eq!(p.vga.font,before);
}
#[test]
fn vendor_video_queries_report_absent_without_altering_cpu_or_ram(){
 for query in [0x7000,0x6f00,0x5f00]{
 let mut p=load(&[0xcd,0x10],"everex.com").unwrap();
 p.cpu.regs=[0xabcd0000|query,0x12340008,0x5678f000,0x9abc0000,0xfc2,0xfc2,0x519e,0x51a6];
 p.cpu.eflags=0xad7;let cpu=p.cpu.clone();let ram=p.ram.clone();
 service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu,cpu);assert_eq!(p.ram.bytes(),ram.bytes());
 p.cpu.regs[3]=4;assert!(service(&mut p,0x10,&mut vec![]).is_err());
 p.cpu.regs[0]=query|1;p.cpu.regs[3]=0;assert!(service(&mut p,0x10,&mut vec![]).is_err());
 }
}
#[test]
fn unsupported_vga_state_and_shadow_buffer_functions_return_unchanged(){
 // AX=1B00h/1C00h (functionality and state, not provided), 4F00h (no VBE),
 // FE00h/FF00h (TopView/DESQview shadow buffers). The palette, font,
 // EGA-information and display-combination calls are covered with the
 // planar modes.
 for ax in [0x1b00u32,0x1c00,0x4f00,0xfe00,0xff00]{
  let mut p=load(&[0xcd,0x10],"vga.com").unwrap();p.cpu.regs[0]=ax;p.cpu.regs[3]=0x0010;p.cpu.regs[2]=0x1234;p.cpu.regs[1]=0x5678;
  let before=p.cpu.clone();service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(p.cpu.regs,before.regs,"AX={ax:04x}");assert_eq!(p.cpu.eflags,before.eflags);
 }
}
#[test]
fn seek_on_character_device_succeeds_at_position_zero(){
 // Ralf Brown INT 21h AH=42h: for character devices (handles 0..4, NUL),
 // lseek succeeds returning position 0 (DX:AX=0, CF=0).
 let mut p=load(&[0xcd,0x21],"seekdev.com").unwrap();
 for handle in [0u32,1,2,3,4]{
  p.cpu.regs[0]=0x4200;p.cpu.regs[3]=handle;p.cpu.regs[1]=0x1234;p.cpu.regs[2]=0x5678;
  service(&mut p,0x21,&mut vec![]).unwrap();
  assert_eq!(p.cpu.regs[0]&0xffff,0,"handle {handle} AX");
  assert_eq!(p.cpu.regs[2]&0xffff,0,"handle {handle} DX");
  assert_eq!(p.cpu.eflags&1,0,"handle {handle} CF");
 }
 // An unopened handle returns error 6 (invalid handle) and CF=1.
 p.cpu.regs[0]=0x4200;p.cpu.regs[3]=5;p.cpu.regs[1]=0;p.cpu.regs[2]=0;
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[0]&0xffff,6);
 assert_eq!(p.cpu.eflags&1,1);
}
#[test]
fn duplicated_handles_share_the_file_position_and_close_independently(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-dup-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(&dir).unwrap();std::fs::write(dir.join("text.txt"),b"abcdef").unwrap();
 let mut p=load_with_drives(&[0xcd,0x21],"x.com",vec![Source::Host(dir.clone())],None).unwrap();let psp=u32::from(PSP)*16;
 let svc=|p:&mut Loaded,ax:u32,bx:u32,cx:u32|->(u32,u32){p.cpu.regs[0]=ax;p.cpu.regs[3]=bx;p.cpu.regs[1]=cx;p.cpu.regs[2]=0x400;service(p,0x21,&mut vec![]).unwrap();(p.cpu.regs[0]&0xffff,p.cpu.eflags&1)};
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
 let svc=|p:&mut Loaded,ax:u32,bx:u32,cx:u32,data:&[u8]|->(u32,u32){p.ram.write_bytes(psp+0x400,data).unwrap();p.cpu.regs[0]=ax;p.cpu.regs[3]=bx;p.cpu.regs[1]=cx;p.cpu.regs[2]=0x400;service(p,0x21,&mut vec![]).unwrap();(p.cpu.regs[0]&0xffff,p.cpu.eflags&1)};
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
 assert_eq!(svc(&mut p,0x3f00,read,6,b""),(6,0));
 assert_eq!((0..6).map(|i|p.ram.load(psp+0x400+i,MemWidth::Byte).unwrap()as u8).collect::<Vec<_>>(),b"ABcdef");
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
 let renamed=call(&mut p,0x5600,0,b"OUT.TXT");assert_eq!(renamed.1,0,"rename returned {renamed:?}");assert!(dir.join("SUB/NEW.TXT").is_file()&&!dir.join("OUT.TXT").exists());
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
#[test]
fn pic_ports_expose_pending_timer_and_independent_masks(){
 // CLI; select IRR; read IRQ0 into BL; set/read master and slave masks.
 let code=[0xfa,0xb0,0x0a,0xe6,0x20,0xe4,0x20,0x88,0xc3,0xb0,0xa5,0xe6,0x21,0xe4,0x21,0x88,0xc7,0xb0,0x5a,0xe6,0xa1,0xe4,0xa1,0xfa,0xf4];
 let mut p=load(&code,"picports.com").unwrap();p.speaker.pic.raise_timer();let r=run(p,30).unwrap();
 assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert_eq!((r.loaded.cpu.regs[3]&0xffff,r.loaded.cpu.regs[0]&255),(0xa501,0x5a));
}
#[test]
fn generic_console_codepage_query_reports_no_display_driver(){
 let mut p=load(&[0xcd,0x21],"ioctl.com").unwrap();let base=u32::from(PSP)*16;p.ram.write_bytes(base+0x200,&[0x34,0x12,0x78,0x56]).unwrap();
 // Duplicate stdout as handle 5, as SYSINFO does when opening CON.
 p.cpu.regs[0]=0x4500;p.cpu.regs[3]=1;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,5);
 for (handle,error) in [(5,1),(19,6)]{
  p.cpu.regs[0]=0xbeef440c;p.cpu.regs[1]=0x036a;p.cpu.regs[3]=handle;p.cpu.regs[2]=0x200;p.cpu.eflags&=!1;p.cpu.undefined_flags|=1;
  let before=p.ram.clone();service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!((p.cpu.regs[0],p.cpu.eflags&1,p.cpu.undefined_flags&1,p.dos_error),(0xbeef0000|error,1,0,error as u16));assert_eq!(p.ram,before);
 }
}
#[test]
fn dos_a20_gate_wraps_real_mode_and_accepts_fast_kbc_and_bios_controls(){
 // DS=FFFF, read FFFF:0510 with A20 disabled, enable port 92, read again,
 // disable it, and read once more. Both physical bytes were set independently.
 let code=[0xb8,0xff,0xff,0x8e,0xd8,0xa0,0x10,0x05,0x88,0xc3,0xb0,2,0xe6,0x92,0xa0,0x10,0x05,0x88,0xc7,0xb0,0,0xe6,0x92,0xa0,0x10,0x05,0xfa,0xf4];
 let mut p=load(&code,"a20.com").unwrap();p.ram.set_a20_enabled(true);p.ram.store(0x500,MemWidth::Byte,0x12).unwrap();p.ram.store(0x100500,MemWidth::Byte,0x34).unwrap();p.ram.set_a20_enabled(false);
 let r=run(p,40).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert_eq!((r.loaded.cpu.regs[3]&0xffff,r.loaded.cpu.regs[0]&255),(0x3412,0x12));
 // The fast and 8042 gates are independent inputs to the address line.
 let code=[0xb0,2,0xe6,0x92,0xb0,0xd1,0xe6,0x64,0xb0,3,0xe6,0x60,0xb0,0,0xe6,0x92,0xb0,0xd0,0xe6,0x64,0xe4,0x64,0x88,0xc3,0xe4,0x60,0x88,0xc7,0xb0,0xd1,0xe6,0x64,0xb0,1,0xe6,0x60,0xfa,0xf4];
 let r=run(load(&code,"kbc.com").unwrap(),50).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert_eq!(r.loaded.cpu.regs[3]&0xffff,0x0301);assert!(!r.loaded.ram.a20_enabled());
 let mut p=load(&[0xcd,0x15],"a20bios.com").unwrap();
 for (op,ax,bx,enabled) in [(0x2401,1,0,true),(0x2402,1,0,true),(0x2403,3,3,true),(0x2400,0,3,false),(0x2402,0,3,false)]{
  p.cpu.regs[0]=op;p.cpu.eflags|=1;p.cpu.undefined_flags|=1;service(&mut p,0x15,&mut vec![]).unwrap();
  assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[3]&0xffff,p.cpu.eflags&1,p.cpu.undefined_flags&1,p.ram.a20_enabled()),(ax,bx,0,0,enabled));
 }
}
#[test]
fn dosbench_absent_quarterdeck_checks_and_pit_latch(){
 let mut p=load(&[0xcd,0x2f],"quarterdeck.com").unwrap();
 p.cpu.regs[0]=0xabcd_d200;p.cpu.regs[3]=0x5144;p.cpu.regs[1]=0x4d45;p.cpu.regs[2]=0x4d30;
 let before=p.cpu.clone();service(&mut p,0x2f,&mut vec![]).unwrap();assert_eq!(p.cpu,before);
 p.cpu.regs[0]=0x4b02;p.cpu.regs[3]=0;p.cpu.regs[7]=0;p.selectors[SegmentRegister::Es.index()]=0;
 let before=p.cpu.clone();service(&mut p,0x2f,&mut vec![]).unwrap();assert_eq!(p.cpu,before);
 p.cpu.regs[7]=1;let before=p.cpu.clone();service(&mut p,0x2f,&mut vec![]).unwrap();assert_eq!(p.cpu,before);
 p.cpu.regs[0]=0xabcd_bfde;p.cpu.regs[3]=8;p.cpu.eflags&=!1;p.cpu.undefined_flags|=1;
 service(&mut p,0x15,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0],0xabcd_86de);assert_eq!(p.cpu.regs[3],8);assert_eq!(p.cpu.eflags&1,1);assert_eq!(p.cpu.undefined_flags&1,0);
 for code in [&[0xb8,0x00,0xd2,0xbb,0x44,0x51,0xb9,0x45,0x4d,0xba,0x30,0x4d,0xcd,0x2f,0xfa,0xf4][..],&[0xb8,0xde,0xbf,0xbb,8,0,0xcd,0x15,0xfa,0xf4]]{
  let r=run(load(code,"probe.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 }
 // The exact channel 2 latch/read sequence used by CHKCPU, after programming.
 let code=[0xb0,0xb6,0xe6,0x43,0xb0,0x34,0xe6,0x42,0xb0,0x12,0xe6,0x42,0xb0,1,0xe6,0x61,0xb0,0x80,0xe6,0x43,0xe4,0x42,0x88,0xc3,0xe4,0x42,0xfa,0xf4];
 let r=run(load(&code,"pit2.com").unwrap(),30).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert_eq!((r.loaded.cpu.regs[0]&255,r.loaded.cpu.regs[3]&255),(0x12,0x34));
}
#[test]
fn speaker_ports_take_watcoms_sound_and_nosound(){
 // sound(900): IN AL,61h by DX; OR AL,3; OUT; mode B6h to 43h; count 052Dh to 42h low then high; CLI; HLT.
 let code=[0xba,0x61,0x00,0xec,0x0c,0x03,0xee,0xb0,0xb6,0xba,0x43,0x00,0xee,0xba,0x42,0x00,0xb0,0x2d,0xee,0xb0,0x05,0xee,0xfa,0xf4];
 let r=run(load(&code,"sound.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert_eq!((r.loaded.speaker.tone(),r.loaded.speaker.beeps),(900,1));
 // nosound(): IN AL,61h; AND AL,FCh; OUT 61h,AL. Bit 4 toggles on each read.
 let code=[0xe4,0x61,0x88,0xc3,0xe4,0x61,0x24,0xfc,0xe6,0x61,0xe4,0x61,0xfa,0xf4];
 let r=run(load(&code,"nosound.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert_eq!((r.loaded.cpu.regs[3]&255,r.loaded.cpu.regs[0]&255,r.loaded.speaker.tone()),(0x10,0x10,0));
}
#[test]
fn video_status_ports_cycle_retrace_and_horizontal_blanking(){
 // Ports 3DAh (color) and 3BAh (monochrome): bit 3 is vertical retrace,
 // bit 0 is horizontal retrace/blanking. Reading port 3DAh in a CGA snow-check
 // loop or vsync-wait loop must advance and exit.
 for port in [0x03dau16, 0x03ba] {
  let [lo, hi] = port.to_le_bytes();
  let code = [0xba, lo, hi, 0xec, 0x88, 0xc3, 0xec, 0x88, 0xc7, 0xfa, 0xf4];
  let mut loaded = load(&code, "vstat.com").unwrap();
  if port==0x03ba { loaded.vga.io(0x3c2,false,0x66).unwrap().unwrap(); }
  let r = run(loaded, 20).unwrap();
  assert!(matches!(r.stop, Stop::Monitor(_)), "{:?}", r.stop);
  let first = r.loaded.cpu.regs[3] & 0xff;
  let second = (r.loaded.cpu.regs[3] >> 8) & 0xff;
  assert_ne!(first & 1, second & 1, "bit 0 toggles between reads on port {port:04x}h");
 }
 // 4HELP.EXE CGA snow check: wait until horizontal blank ends, then wait until blank begins.
 let snow_code = [
  0xba, 0xda, 0x03, 0xec, 0xa8, 0x08, 0x75, 0x09, 0xd0, 0xe8, 0x72, 0xf7,
  0xec, 0xd0, 0xe8, 0x73, 0xfb, 0xfa, 0xf4,
 ];
 let r = run(load(&snow_code, "snow.com").unwrap(), 50).unwrap();
 assert!(matches!(r.stop, Stop::Monitor(_)), "{:?}", r.stop);
 // Standard game/demo vertical retrace wait: wait for vblank end, then wait for vblank start.
 let vsync_code = [
  0xba, 0xda, 0x03,
  0xec, 0xa8, 0x08, 0x75, 0xfb,
  0xec, 0xa8, 0x08, 0x74, 0xfb,
  0xfa, 0xf4,
 ];
 let r = run(load(&vsync_code, "vsync.com").unwrap(), 200).unwrap();
 assert!(matches!(r.stop, Stop::Monitor(_)), "{:?}", r.stop);
 // Other ports, word accesses, channel 1 and a channel 0
 // rate other than the BIOS one stop the run.
 for code in [&[0xe4,0x41,0xfa,0xf4][..],&[0xe5,0x61,0xfa,0xf4],&[0xb0,0x76,0xe6,0x43,0xfa,0xf4],
   &[0xb0,0x36,0xe6,0x43,0xb0,0x01,0xe6,0x40,0xb0,0,0xe6,0x40,0xfa,0xf4]]{
  let r=run(load(code,"port.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
 }
}
#[test]
fn memory_drive_is_z_with_temp_and_keeps_files_for_the_session(){
 let mut p=load_with_drives(&[0xcd,0x21],"x.com",vec![Source::Memory(MemDir::default())],None).unwrap();let psp=u32::from(PSP)*16;
 let env:Vec<u8>=(0..33).map(|i|p.ram.load(u32::from(ENV)*16+i,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(env,b"COMSPEC=A:\\COMMAND.COM\0TEMP=Z:\\\0\0");
 let svc=|p:&mut Loaded,ax:u32,bx:u32,cx:u32,data:&[u8]|->(u32,u32){p.ram.write_bytes(psp+0x400,data).unwrap();p.cpu.regs[0]=ax;p.cpu.regs[3]=bx;p.cpu.regs[1]=cx;p.cpu.regs[2]=0x400;service(p,0x21,&mut vec![]).unwrap();(p.cpu.regs[0]&0xffff,p.cpu.eflags&1)};
 // Create, write, close, reopen and read back; seek and truncate.
 let (h,cf)=call(&mut p,0x3c00,0,b"Z:\\PIPE.TMP");assert_eq!(cf,0);assert_eq!(svc(&mut p,0x4000,h,5,b"hello"),(5,0));assert_eq!(svc(&mut p,0x3e00,h,0,b"").1,0);
 let (h,_)=call(&mut p,0x3d02,0,b"z:pipe.tmp");assert_eq!(svc(&mut p,0x3f00,h,9,b""),(5,0));
 assert_eq!((0..5).map(|i|p.ram.load(psp+0x400+i,MemWidth::Byte).unwrap()as u8).collect::<Vec<_>>(),b"hello");
 p.cpu.regs[2]=2;p.cpu.regs[1]=0;p.cpu.regs[3]=h;p.cpu.regs[0]=0x4200;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(svc(&mut p,0x4000,h,0,b""),(0,0));
 // AH57 on a memory file, then close; the size in a search is the truncated one.
 p.cpu.regs[1]=0x6000;p.cpu.regs[2]=0x5b4f;p.cpu.regs[3]=h;p.cpu.regs[0]=0x5701;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.eflags&1,0);
 assert_eq!(svc(&mut p,0x3e00,h,0,b"").1,0);
 assert_eq!(call(&mut p,0x4e00,0x10,b"Z:\\*.*").1,0);assert_eq!(dta_name(&p),(0x20,2,b"PIPE.TMP".to_vec()));
 // Directories, rename into one, a refused non-empty remove, delete; AH36 counts 2 KiB clusters.
 assert_eq!(call(&mut p,0x3900,0,b"Z:\\SUB").1,0);
 p.ram.write_bytes(psp+0x300,b"Z:\\SUB\\P.TMP\0").unwrap();p.cpu.regs[7]=0x300;assert_eq!(call(&mut p,0x5600,0,b"Z:\\PIPE.TMP").1,0);
 assert_eq!(call(&mut p,0x3a00,0,b"Z:\\SUB"),(5,1));assert_eq!(call(&mut p,0x3d00,0,b"Z:\\PIPE.TMP"),(2,1));
 p.cpu.regs[2]=26;p.cpu.regs[0]=0x3600;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[3]&0xffff,p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff),(4,16383,512,16384));
 assert_eq!(call(&mut p,0x4100,0,b"Z:\\SUB\\P.TMP").1,0);assert_eq!(call(&mut p,0x3a00,0,b"Z:\\SUB").1,0);
 assert_eq!(call(&mut p,0x4e00,0x10,b"Z:\\*.*"),(0x12,1));
 // AH0E reports drives up to Z:.
 p.cpu.regs[2]=0;p.cpu.regs[0]=0x0e00;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&255,26);
}
#[test]
fn mandbbc_draws_the_same_characters_as_the_bbc_basic_floating_point_program(){
 // The rows BBC BASIC prints with floating point (computed in Python from
 // the listing: z0 = c, escape at |z|^2 >= 5, at most 16 iterations).
 let expected=[
  "..............,,,,,,,,,,,,,,,,,,,''''''''~~==;B;#;~'''''',,,,,,................",
  "............,,,,,,,,,,,,,,,,,,,''''''''~~~=+:;O#*+=~~''''',,,,,,,..............",
  "...........,,,,,,,,,,,,,,,,,'''''''''~~~==:O      :=~~~~'''',,,,,,.............",
  ".........,,,,,,,,,,,,,,,,,''''''''~~====++:$      ;++===~~''',,,,,,,...........",
  "........,,,,,,,,,,,,,,,,''''''~~~~=: X%;* $X     B % :++:#=~'',,,,,,,..........",
  ".......,,,,,,,,,,,,,,''''~~~~~~~==+:%                 B   :~~',,,,,,,,.........",
  "......,,,,,,,,,,,'''''~~~~~~~~===+; $                    *+=~'',,,,,,,,........",
  "......,,,,,,,''''''~=$;+==++++++:;&                      &:+~''',,,,,,,,.......",
  ".....,,,,,''''''~~~==;$&&;$#**;;;&                         $=~'',,,,,,,,.......",
  ".....,,,''''''~~~~~=+:*        &&                         &+~~''',,,,,,,.......",
  "....,,''''''~~~~==;:;&                                    %=~~''',,,,,,,,......",
  "....,,'~~~~====++:*  #                                   $+=~~''',,,,,,,,......",
  "....,'                                                  *:==~~''',,,,,,,,......",
  "....,,'~~~~====++:*  #                                   $+=~~''',,,,,,,,......",
  "....,,''''''~~~~==;:;&                                    %=~~''',,,,,,,,......",
  ".....,,,''''''~~~~~=+:*        &&                         &+~~''',,,,,,,.......",
  ".....,,,,,''''''~~~==;$&&;$#**;;;&                         $=~'',,,,,,,,.......",
  "......,,,,,,,''''''~=$;+==++++++:;&                      &:+~''',,,,,,,,.......",
  "......,,,,,,,,,,,'''''~~~~~~~~===+; $                    *+=~'',,,,,,,,........",
  ".......,,,,,,,,,,,,,,''''~~~~~~~==+:%                 B   :~~',,,,,,,,.........",
  "........,,,,,,,,,,,,,,,,''''''~~~~=: X%;* $X     B % :++:#=~'',,,,,,,..........",
  ".........,,,,,,,,,,,,,,,,,''''''''~~====++:$      ;++===~~''',,,,,,,...........",
  "...........,,,,,,,,,,,,,,,,,'''''''''~~~==:O      :=~~~~'''',,,,,,.............",
  "............,,,,,,,,,,,,,,,,,,,''''''''~~~=+:;O#*+=~~''''',,,,,,,..............",
  "..............,,,,,,,,,,,,,,,,,,,''''''''~~==;B;#;~'''''',,,,,,................"];
 let r=run(load(include_bytes!("../../programs/dos/MANDBBC.COM"),"MANDBBC.COM").unwrap(),1_000_000).unwrap();
 assert!(matches!(r.stop,Stop::Exit(0)),"{:?}",r.stop);let text=String::from_utf8(r.output).unwrap();
 let lines:Vec<&str>=text.split_terminator("\r\n").collect();assert_eq!(lines,expected);
}
#[test]
fn host_clock_gives_the_local_date_and_time_of_day(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let mut p=load(&[0xcd,0x21],"clock.com").unwrap();p.clock_offset=Some(3*3600);
 let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()+3*3600;
 p.cpu.regs[0]=0x2a00;service(&mut p,0x21,&mut vec![]).unwrap();
 let(_,date)=files::probe::stamp(Some(UNIX_EPOCH+std::time::Duration::from_secs(now)));
 assert_eq!((p.cpu.regs[1]&0xffff,(p.cpu.regs[2]>>8)&255,p.cpu.regs[2]&255),(1980+u32::from(date>>9),u32::from(date>>5&15),u32::from(date&31)));
 assert_eq!(p.cpu.regs[0]&255,((now/86400+4)%7)as u32,"day of week");
 // AH2C: hours and minutes of the local time of day (within a minute of now).
 p.cpu.regs[0]=0x2c00;service(&mut p,0x21,&mut vec![]).unwrap();
 let got=((p.cpu.regs[1]>>8)&255)*60+(p.cpu.regs[1]&255);let want=(now%86400/60)as u32;
 assert!(got==want||(got+1)%1440==want,"{got} vs {want}");
 // The deterministic clock stays at midnight on 1 January 1980.
 let mut p=load(&[0xcd,0x21],"clock.com").unwrap();p.cpu.regs[0]=0x2a00;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!((p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff,p.cpu.regs[0]&255),(1980,0x0101,2));
}
#[test]
fn command_line_goes_to_the_psp_tail_as_command_com_passes_it(){
    let mut p=load(&[0xcd,0x20],"x.com").unwrap();let tail=u32::from(PSP)*16+0x80;
    let read=|p:&Loaded,n:u32|(0..n).map(|i|p.ram.load(tail+i,MemWidth::Byte).unwrap()as u8).collect::<Vec<_>>();
    assert_eq!(read(&p,2),[0,0x0d],"empty by default");
    set_command_tail(&mut p,"/K D:\\START.BAT").unwrap();
    assert_eq!(read(&p,18),b"\x10 /K D:\\START.BAT\r");
    assert!(set_command_tail(&mut p,&"x".repeat(126)).is_err());
    set_command_tail(&mut p,"\u{e4}").unwrap();assert_eq!(read(&p,4),[2,b' ',0x84,13]);
    assert!(set_command_tail(&mut p,"🙂").is_err());
    set_command_tail(&mut p,&"x".repeat(125)).unwrap();assert_eq!(read(&p,1),[126]);assert_eq!(read(&p,128)[127],0x0d);
}
#[test]
fn dos_set_time_changes_only_virtual_clocks_and_validates_the_complete_write(){
 let mut p=load(&[0xcd,0x21],"time.com").unwrap();p.cpu.regs[0]=0xabcd2d00;p.cpu.regs[1]=0x0c22;p.cpu.regs[2]=0x384e;p.cpu.eflags=0xad7;p.cpu.undefined_flags=0x8d5;
 service(&mut p,0x21,&mut vec![]).unwrap();let hundredths=((12*60+34)*60+56)*100+78;let ticks=hundredths*59659u64/327680;
 assert_eq!((p.cpu.regs[0],p.cpu.eflags,p.cpu.undefined_flags,p.timer_ticks),(0xabcd2d00,0xad7,0x8d5,ticks as u32));assert_eq!(p.ram.load(0x46c,MemWidth::Dword).unwrap(),ticks as u32);
 assert_eq!((p.rtc.read(None,4),p.rtc.read(None,2),p.rtc.read(None,0)),(Some(0x12),Some(0x34),Some(0x56)));
 p.cpu.regs[0]=0x2c00;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[1]&0xffff,0x0c22);assert_eq!((p.cpu.regs[2]>>8)&255,56);assert!((p.cpu.regs[2]&255).abs_diff(78)<=6);
 let mut live=load(&[0xcd,0x21],"time-live.com").unwrap();live.clock_offset=Some(0);live.cpu.regs[0]=0x2d00;live.cpu.regs[1]=0x0102;live.cpu.regs[2]=0x0300;service(&mut live,0x21,&mut vec![]).unwrap();let first=live.timer_ticks;let now=services::sampled_ticks(&mut live);assert!(now.wrapping_sub(first)<20);
 // Out-of-range values return AL=FF without altering either clock or memory.
 p.cpu.regs[0]=0x2d00;p.cpu.regs[1]=0x1800;p.cpu.regs[2]=0;let before=p.clone();service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&255,255);assert_eq!((p.timer_ticks,p.timer_tick_adjust,&p.rtc,&p.ram),(before.timer_ticks,before.timer_tick_adjust,&before.rtc,&before.ram));
 // A failure at either BDA write leaves time and all guest state unchanged.
 let mut p=load(&[0xcd,0x21],"time-ro.com").unwrap();p.ram.protect(0x470,1).unwrap();p.cpu.regs[0]=0x2d00;p.cpu.regs[1]=0x0102;p.cpu.regs[2]=0x0304;let before=p.clone();assert!(service(&mut p,0x21,&mut vec![]).is_err());assert_eq!((p.cpu,p.timer_ticks,p.timer_tick_adjust,p.rtc,p.ram),(before.cpu,before.timer_ticks,before.timer_tick_adjust,before.rtc,before.ram));
}
#[test]
fn cmos_year_writes_are_virtual_readable_and_do_not_change_dos_date(){
 // Read/write the year twice, as a hardware probe does, leaving the RTC restored.
 let code=[0xb0,9,0xe6,0x70,0xb0,0x81,0xe6,0x71,0xe4,0x71,0x88,0xc3,0xb0,0x80,0xe6,0x71,0xe4,0x71,0xfa,0xf4];
 let r=run(load(&code,"rtc.com").unwrap(),30).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert_eq!((r.loaded.cpu.regs[3]&255,r.loaded.cpu.regs[0]&255),(0x81,0x80));
 let mut p=r.loaded;p.cpu.regs[0]=0x2a00;service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!((p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff),(1980,0x0101));
 let code=[0xb0,9,0xe6,0x70,0xb0,0xfa,0xe6,0x71,0xfa,0xf4];let r=run(load(&code,"rtc-invalid.com").unwrap(),20).unwrap();
 assert!(matches!(&r.stop,Stop::Service(m) if m.contains("valid BCD")),"{:?}",r.stop);assert_eq!(r.loaded.rtc.read(None,9),Some(0x80));
}
#[test]
fn guest_mouse_handler_takes_precedence_over_absent_driver_stub(){
 let code=[0xba,0x14,1,0xb8,0x33,0x25,0xcd,0x21,0xb8,0x0a,0,0x31,0xdb,0xcd,0x33,0xb8,0,0x4c,0xcd,0x21,0x43,0xcf];
 let r=run(load(&code,"mousehook.com").unwrap(),30).unwrap();assert!(matches!(r.stop,Stop::Exit(0)));assert_eq!(r.loaded.cpu.regs[3]&65535,1);
}
#[test]
fn no_mouse_driver_and_cmos_reads_as_an_at_with_640k(){
 // INT 33h AX=0 reports no driver; other calls retain the input state.
 let mut p=load(&[0xcd,0x21],"mouse.com").unwrap();p.cpu.regs[0]=0;p.cpu.regs[3]=0x1234;service(&mut p,0x33,&mut vec![]).unwrap();
 assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[3]&0xffff),(0,0));
 for function in [1,2,3,0xa,0x15,0x16,0x17,0x21,0xffff]{
  p.cpu.regs=[0xabcd0000|function,0x7000,0xf04,0x1234,0xfa4,0xfaa,0x100c,0x48];p.cpu.eflags=0xad7;
  let cpu=p.cpu.clone();let ram=p.ram.clone();service(&mut p,0x33,&mut vec![]).unwrap();assert_eq!(p.cpu,cpu);assert_eq!(p.ram,ram);
 }
 // OUT 70h,AL then IN AL,71h for registers 15h, 16h (640K), 17h (no extended
 // memory), 0Bh (24-hour BCD), 09h, 32h (1980) and 06h (Tuesday); CLI; HLT.
 let mut code=Vec::new();
 for reg in [0x15,0x16,0x17,0x0b,0x09,0x32,0x06,0x33]{code.extend([0xb0,reg,0xe6,0x70,0xe4,0x71,0x50]);}
 code.extend([0xfa,0xf4]);
 let r=run(load(&code,"cmos.com").unwrap(),40).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 let sp=r.loaded.cpu.regs[4];let base=u32::from(PSP)*16;
 let pushed:Vec<u32>=(0..8).rev().map(|i|r.loaded.ram.load(base+sp+i*2,MemWidth::Byte).unwrap()).collect();
 assert_eq!(pushed,[0x80,0x02,0x00,0x02,0x80,0x19,0x03,0]);
 // CMOS shutdown byte round trip; its value is state, not a reset by itself.
 let code=[0xb0,0x8f,0xe6,0x70,0xb0,9,0xe6,0x71,0xe4,0x71,0xfa,0xf4];
 let r=run(load(&code,"shutdown.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);assert_eq!((r.loaded.cpu.regs[0]&255,r.loaded.cmos_shutdown),(9,9));
 // Writing the data port and reading the index port stop the run.
 for code in [&[0xb0,0x0b,0xe6,0x70,0xe6,0x71,0xfa,0xf4][..],&[0xe4,0x70,0xfa,0xf4]]{
  let r=run(load(code,"cmosw.com").unwrap(),20).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
 }
}
#[test]
fn finland_358_gives_day_month_year_and_a_24_hour_clock(){
 let mut p=load(&[0xcd,0x21],"fi.com").unwrap();let psp=u32::from(PSP)*16;let mut out=vec![];
 let bytes=|p:&Loaded,at:u32,n:u32|->Vec<u8>{(0..n).map(|i|p.ram.load(at+i,MemWidth::Byte).unwrap()as u8).collect()};
 // AH38 AL=FFh BX=358 DX=FFFFh sets the country; AL=0 then reads it: AL=FFh (code over 254), BX=358.
 p.cpu.regs[0]=0x38ff;p.cpu.regs[3]=358;p.cpu.regs[2]=0xffff;service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.country,358);
 p.cpu.regs[0]=0x3800;p.cpu.regs[2]=0x200;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[3]&0xffff,p.cpu.eflags&1),(0xff,358,0));
 let info=bytes(&p,psp+0x200,24);assert_eq!(&info[..18],b"\x01\0EUR\0\0 \0,\0.\0.\0\x03\x02\x01");assert_eq!(info[22],b';');
 // AX=6501 with DX=358 gives the same record after country 358 and code page 437.
 p.cpu.regs[0]=0x6501;p.cpu.regs[3]=437;p.cpu.regs[2]=358;p.cpu.regs[1]=0x40;p.cpu.regs[7]=0x300;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!(bytes(&p,psp+0x300,7),[1,0x1c,0,0x66,1,0xb5,1]);assert_eq!(bytes(&p,psp+0x307,24),info);
 // Asking for 001 still answers with the US record; an unknown country stops.
 p.cpu.regs[0]=0x3801;p.cpu.regs[2]=0x200;service(&mut p,0x21,&mut out).unwrap();assert_eq!(bytes(&p,psp+0x202,1),b"$");
 p.cpu.regs[0]=0x3831;p.cpu.regs[2]=0x200;assert!(service(&mut p,0x21,&mut out).is_err());
}
#[test]
fn com1_takes_bios_calls_uart_registers_and_dos_aux_handles(){
 let mut p=load(&[0xcd,0x21],"com.com").unwrap();let psp=u32::from(PSP)*16;
 // Without a session there is no port: INT 14h stops, and COM1 is a plain name.
 p.cpu.regs[0]=0x0300;p.cpu.regs[2]=0;assert!(service(&mut p,0x14,&mut vec![]).is_err());
 serial::attach(&mut p).unwrap();
 assert_eq!((p.ram.load(0x400,MemWidth::Word).unwrap(),p.ram.load(0x410,MemWidth::Word).unwrap()&0x0e00),(0x3f8,0x0200));
 // AH01 sends, AH03 reports empty transmitter and CTS/DSR/DCD, AH02 times out at once.
 let int14=|p:&mut Loaded,ax:u32|{p.cpu.regs[0]=ax;p.cpu.regs[2]=0;service(p,0x14,&mut vec![]).unwrap();p.cpu.regs[0]&0xffff};
 assert_eq!(int14(&mut p,0x0141),0x6041);assert_eq!(int14(&mut p,0x0300),0x60b0);assert_eq!(int14(&mut p,0x0200)&0x8000,0x8000);
 p.com1.as_mut().unwrap().rx.extend(b"hi");assert_eq!(int14(&mut p,0x0300),0x61b0);assert_eq!(int14(&mut p,0x0200),u32::from(b'h'));
 // The 8250 registers, polled: LSR shows the waiting byte, RBR takes it, THR sends; IER interrupts stop.
 let code=[0xba,0xfd,0x03,0xec,0x88,0xc3,0xba,0xf8,0x03,0xec,0x88,0xc7,0xb0,0x21,0xee,0xfa,0xf4];
 let mut q=load(&code,"uart.com").unwrap();serial::attach(&mut q).unwrap();q.com1.as_mut().unwrap().rx.push_back(b'z');
 let r=run(q,20).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert_eq!((r.loaded.cpu.regs[3]&0xffff,r.loaded.com1.as_ref().unwrap().tx.clone()),(0x7a61,vec![0x21]));
 let mut q=load(&[0xba,0xf9,0x03,0xb0,0x01,0xee,0xfa,0xf4],"ier.com").unwrap();serial::attach(&mut q).unwrap();
 assert!(matches!(run(q,20).unwrap().stop,Stop::Service(_)));
 // Handle 3 (AUX) and COM1 opened by name write to the port and read what arrived.
 p.com1.as_mut().unwrap().tx.clear();p.com1.as_mut().unwrap().rx.clear();
 p.ram.write_bytes(psp+0x200,b"ok").unwrap();p.cpu.regs[0]=0x4000;p.cpu.regs[3]=3;p.cpu.regs[1]=2;p.cpu.regs[2]=0x200;service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.com1.as_ref().unwrap().tx,b"ok");
 let (h,cf)=call(&mut p,0x3d02,0,b"C:\\DEV\\COM1.TXT");assert_eq!(cf,0);
 p.cpu.regs[0]=0x3f00;p.cpu.regs[3]=h;p.cpu.regs[1]=8;p.cpu.regs[2]=0x300;service(&mut p,0x21,&mut vec![]).unwrap();assert!(p.input_wait,"no byte yet: the read waits");p.input_wait=false;
 p.com1.as_mut().unwrap().rx.extend(b"abc");service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!((p.cpu.regs[0]&0xffff,p.ram.load(psp+0x300,MemWidth::Word).unwrap()),(3,u32::from_le_bytes([b'a',b'b',0,0])));
}
#[test]
fn com1_line_input_cancels_on_esc_and_shows_control_keys_as_freedos_read_line(){
 let mut p=load(&[0xcd,0x21],"ctty.com").unwrap();let psp=u32::from(PSP)*16;serial::attach(&mut p).unwrap();
 for h in [0,1]{p.ram.store(psp+0x18+h,MemWidth::Byte,3).unwrap();}
 let mut out=vec![];
 // The prompt puts the port at column 4; the line starts there.
 p.ram.write_bytes(psp+0x300,b"C:\\>$").unwrap();p.cpu.regs[0]=0x0900;p.cpu.regs[2]=0x300;service(&mut p,0x21,&mut out).unwrap();
 // A VT Up arrow (ESC [ A) arrives in two parts: ESC cancels what was typed.
 p.com1.as_mut().unwrap().rx.extend(b"dir\x1b");
 p.ram.write_bytes(psp+0x200,&[10]).unwrap();p.cpu.regs[0]=0x0a00;p.cpu.regs[2]=0x200;service(&mut p,0x21,&mut out).unwrap();
 assert!(p.input_wait);p.input_wait=false;
 // The rest is ordinary text; Backspace takes it, then Ctrl+A shows as ^A and erases with two.
 p.com1.as_mut().unwrap().rx.extend(b"[A\x08\x08\x01\x08\nver\r");
 p.cpu.regs[0]=0x0a00;service(&mut p,0x21,&mut out).unwrap();assert!(!p.input_wait);
 assert_eq!((0..5).map(|i|p.ram.load(psp+0x201+i,MemWidth::Byte).unwrap()as u8).collect::<Vec<_>>(),b"\x03ver\r");
 assert_eq!(p.com1.as_ref().unwrap().tx,b"C:\\>dir\\\r\n    [A\x08 \x08\x08 \x08^A\x08 \x08\x08 \x08\r\nver\r".to_vec());
 assert!(out.is_empty());
 // A 00h key pair: template keys stay unmodelled, others are ignored.
 p.com1.as_mut().unwrap().rx.extend(b"\0\x48a\r");p.com1.as_mut().unwrap().tx.clear();
 p.cpu.regs[0]=0x0a00;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.ram.load(psp+0x201,MemWidth::Word).unwrap(),p.com1.as_ref().unwrap().tx.clone()),(u32::from_le_bytes([1,b'a',0,0]),b"a\r".to_vec()));
 p.com1.as_mut().unwrap().rx.extend(b"\0\x3d");
 p.cpu.regs[0]=0x0a00;assert!(service(&mut p,0x21,&mut out).unwrap_err().contains("template key 3d"));
}
#[test]
fn ctty_com1_moves_dos_console_input_and_output_to_the_port(){
 let mut p=load(&[0xcd,0x21],"ctty.com").unwrap();let psp=u32::from(PSP)*16;serial::attach(&mut p).unwrap();
 // Handles 0 and 1 on AUX, as CTTY COM1 leaves them.
 for h in [0,1]{p.ram.store(psp+0x18+h,MemWidth::Byte,3).unwrap();}
 let mut out=vec![];
 p.cpu.regs[0]=0x0b00;service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu.regs[0]&255,0);
 p.cpu.regs[0]=0x0100;service(&mut p,0x21,&mut out).unwrap();assert!(p.input_wait);p.input_wait=false;
 p.com1.as_mut().unwrap().rx.extend(b"xDIX\x08R\r");
 p.cpu.regs[0]=0x0100;service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu.regs[0]&255,u32::from(b'x'));
 // AH0A: Backspace edits, CR ends; echo goes to the port, nothing to the screen output.
 p.ram.write_bytes(psp+0x200,&[10]).unwrap();p.cpu.regs[0]=0x0a00;p.cpu.regs[2]=0x200;service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((0..6).map(|i|p.ram.load(psp+0x201+i,MemWidth::Byte).unwrap()as u8).collect::<Vec<_>>(),b"\x03DIR\r\0");
 p.cpu.regs[0]=0x0200;p.cpu.regs[2]=u32::from(b'!');service(&mut p,0x21,&mut out).unwrap();
 assert_eq!(p.com1.as_ref().unwrap().tx,b"xDIX\x08 \x08R\r!");assert!(out.is_empty());
}
#[test]
fn timeit_runs_its_command_through_comspec_and_prints_the_elapsed_time(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-timeit-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(&dir).unwrap();
 // A stand-in COMMAND.COM: writes its command tail with AH40, then exits with the tail's length.
 std::fs::write(dir.join("command.com"),[0xb4,0x40,0xbb,0x01,0x00,0x8a,0x0e,0x80,0x00,0x32,0xed,0xba,0x81,0x00,0xcd,0x21,0xa0,0x80,0x00,0xb4,0x4c,0xcd,0x21]).unwrap();
 let timeit=|tail:&[u8],country:u16,ticks:u32|{
  let mut p=load_with_drives(include_bytes!("../../programs/dos/TIMEIT.COM"),"TIMEIT.COM",vec![Source::Host(dir.clone())],None).unwrap();
  let mut psp_tail=vec![tail.len()as u8];psp_tail.extend(tail);psp_tail.push(13);
  p.ram.write_bytes(u32::from(PSP)*16+0x80,&psp_tail).unwrap();p.country=country;p.timer_ticks=ticks;
  let r=run(p,20_000).unwrap();(r.stop,String::from_utf8(r.output).unwrap(),r.loaded)
 };
 // Each AH2C read advances the deterministic clock one tick (5 hundredths).
 let (stop,text,p)=timeit(b"  FACTOR 3",1,0);
 assert!(matches!(stop,Stop::Exit(12)),"the command's exit code: {stop:?}");
 assert_eq!(text," /C FACTOR 3Command  FACTOR 3\r\nStart    0:00:00.00\r\nEnd      0:00:00.05\r\nElapsed  0:00:00.05\r\n");
 assert_eq!((p.psp,p.exec.len()),(PSP,0));
 // Finnish separators, and a run over midnight still counts forward.
 let (stop,text,_)=timeit(b" DIR",358,0x0018_00af);
 assert!(matches!(stop,Stop::Exit(7)),"{stop:?}");
 assert_eq!(text," /C DIRCommand  DIR\r\nStart    23.59.59,94\r\nEnd      0.00.00,00\r\nElapsed  0.00.00,06\r\n");
 // No command: usage and exit code 1, without running anything.
 let (stop,text,_)=timeit(b"   ",1,0);
 assert!(matches!(stop,Stop::Exit(1)),"{stop:?}");assert!(text.starts_with("Runs a command"),"{text}");
 // A command line that would not fit after " /C ".
 let (stop,text,_)=timeit(&[b'x';123],1,0);
 assert!(matches!(stop,Stop::Exit(1)),"{stop:?}");assert_eq!(text,"TIMEIT: the command line is too long\r\n");
 std::fs::remove_file(dir.join("command.com")).unwrap();
 // COMSPEC names a file that is not there: the DOS error is reported.
 let (stop,text,_)=timeit(b" X",1,0);
 assert!(matches!(stop,Stop::Exit(1)),"{stop:?}");assert_eq!(text,"TIMEIT: cannot run COMSPEC, DOS error 2\r\n");
 std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn run_test386_rom_image() {
    use std::path::Path;
    use crate::dos::{self, MAX_STEPS};

    let bin_path = Path::new("test386.asm/test386.bin");
    if !bin_path.exists() {
        return;
    }

    let result = dos::run_rom(bin_path, MAX_STEPS).expect("Execution engine returned an error");
    println!("{}", dos::format_rom_report(&result));
}

#[test]
fn session_cache_survives_slices_and_rechecks_changed_code(){
 let p=load(&[0xeb,0xfe],"loop.com").unwrap();
 let first=run(p,1).unwrap();
 assert_eq!(first.loaded.cache.stats().misses,1);
 let mut second=run(first.loaded,1).unwrap();
 assert_eq!(second.loaded.cache.stats().hits,1);
 second.loaded.ram.write_bytes(u32::from(PSP)*16+0x100,&[0x90]).unwrap();
 let third=run(second.loaded,1).unwrap();
 assert_eq!(third.loaded.cpu.eip,0x101);
 assert!(third.loaded.cache.stats().invalidations>0);
}

#[test]
fn rom_code_at_dos_stub_addresses_is_executed_as_rom(){
 let path=std::env::temp_dir().join(format!("pil-rom-{}.bin",std::process::id()));
 let mut bytes=vec![0;65536];
 bytes[0xfff0..0xfff5].copy_from_slice(&[0xea,0x02,0xf0,0x00,0xf0]);
 bytes[0xf002..0xf006].copy_from_slice(&[0xb8,0x34,0x12,0xf4]);
 std::fs::write(&path,bytes).unwrap();
 let result=run_rom(&path,10);
 std::fs::remove_file(&path).unwrap();
 let r=result.unwrap();
 assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
 assert_eq!(r.loaded.cpu.regs[0],0x1234);
 assert_eq!((r.loaded.selectors[1],r.loaded.cpu.eip),(0xf000,0xf005));
 assert!(!r.instructions.contains_key("IRET (Dine interrupt entry)"));
 assert!(r.loaded.ram.check_write(0xff002,MemWidth::Byte).is_err());
}

#[test]
fn a_128k_rom_maps_its_lower_half_at_e0000(){
 // Reset at F000:FFF0 (offset 1FFF0h) jumps to E000:0000 in the lower
 // half: MOV AX,5678h; HLT. Sizes other than 64 and 128 KB are refused.
 let path=std::env::temp_dir().join(format!("pil-rom128-{}.bin",std::process::id()));
 let mut bytes=vec![0;0x20000];
 bytes[0x1fff0..0x1fff5].copy_from_slice(&[0xea,0x00,0x00,0x00,0xe0]);
 bytes[0..4].copy_from_slice(&[0xb8,0x78,0x56,0xf4]);
 std::fs::write(&path,&bytes).unwrap();
 let result=run_rom(&path,10);
 std::fs::write(&path,vec![0;0x18000]).unwrap();
 let odd=run_rom(&path,10);
 std::fs::remove_file(&path).unwrap();
 let r=result.unwrap();
 assert_eq!(r.loaded.cpu.regs[0],0x5678);
 assert_eq!((r.loaded.selectors[1],r.loaded.cpu.eip),(0xe000,0x0003));
 assert!(r.loaded.ram.check_write(0xe0000,MemWidth::Byte).is_err(),"the lower half is ROM too");
 assert!(odd.is_err_and(|e|e.contains("64 KB or 128 KB")));
}

#[test]
fn vga_word_io_roundtrips_and_direct_display_start_is_visible(){
    // MOV DX,3D4; MOV AX,010D; OUT DX,AX; IN AX,DX; HLT
    let mut p=load(&[0xba,0xd4,3,0xb8,0x0d,1,0xef,0xed,0xf4],"crtc.com").unwrap();
    p.ram.write_bytes(0xb8002,&[b'V',7]).unwrap();
    let r=run(p,4).unwrap();assert!(matches!(r.stop,Stop::Budget));assert_eq!(r.loaded.cpu.regs[0]&65535,0x010d);
    let s=session::Session::new(r.loaded);let (cells,_,_)=s.screen().unwrap();assert_eq!(cells[0],b'V');
}
#[test]
fn vga_unimplemented_word_data_access_is_atomic(){
    let mut p=load(&[0xef],"crtc.com").unwrap();p.cpu.regs[2]=0x3d5;p.cpu.regs[0]=0x1234;
    let before=p.cpu.clone();let r=run(p,1).unwrap();assert!(matches!(r.stop,Stop::Service(_)));assert_eq!(r.loaded.cpu,before);
}

#[test]
fn inactive_mono_word_probe_returns_ff_and_preserves_active_crtc(){
    let p=load(&[0xba,0xb4,3,0xb8,0x0f,0x55,0xef,0xed,0xf4],"mono.com").unwrap();
    assert_eq!(p.ram.load(0x463,MemWidth::Word).unwrap(),0x3d4);
    let r=run(p,4).unwrap();assert!(matches!(r.stop,Stop::Budget));
    assert_eq!(r.loaded.cpu.regs[0]&65535,0xffff);assert_eq!(r.loaded.vga.cursor(),0);
}
#[test]
fn mono_word_access_uses_selected_bank(){
    let mut p=load(&[0xba,0xb4,3,0xb8,0x0f,0x55,0xef,0xed,0xf4],"mono.com").unwrap();
    p.vga.io(0x3c2,false,0x66).unwrap();let r=run(p,4).unwrap();
    assert_eq!(r.loaded.cpu.regs[0]&65535,0x550f);assert_eq!(r.loaded.vga.cursor(),0x55);
}

#[test]
fn bios_mode_three_restores_color_io_selection_and_bda(){
    let mut p=load(&[0xf4],"mode.com").unwrap();p.vga.io(0x3c2,false,0x66).unwrap();
    p.ram.store(0x463,MemWidth::Word,0x3b4).unwrap();p.cpu.regs[0]=3;
    console::display(&mut p,0).unwrap();assert!(p.vga.color_io());
    assert_eq!(p.ram.load(0x463,MemWidth::Word).unwrap(),0x3d4);
}

#[test]
fn pit_nonzero_reload_and_latch_work_through_guest_ports(){
    // CLI; select mode 2; load E90B; latch; read low/high into BX; HLT.
    let code=[0xfa,0xb0,0x34,0xe6,0x43,0xb0,0x0b,0xe6,0x40,0xb0,0xe9,0xe6,0x40,
        0xb0,0,0xe6,0x43,0xe4,0x40,0x88,0xc3,0xe4,0x40,0x88,0xc7,0xf4];
    let r=run(load(&code,"pit.com").unwrap(),50).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.cpu.regs[3]&65535,0xe90b);
}

#[test]
fn host_bios_clock_read_with_if_clear_keeps_undelivered_pit_ticks_out_of_bda(){
    let mut p=load(&[0xfa,0xf4],"clock.com").unwrap();p.clock_offset=Some(0);p.cpu.eflags=2;p.timer_ticks=123;
    p.ram.store(0x46c,MemWidth::Dword,123).unwrap();p.cpu.regs[0]=0;service(&mut p,0x1a,&mut vec![]).unwrap();
    assert_eq!(p.cpu.regs[2]&65535,123);assert_eq!(p.timer_ticks,123);assert_eq!(p.ram.load(0x46c,MemWidth::Dword).unwrap(),123);
}

#[test]
fn dos_extended_error_reports_failed_open_and_survives_unrelated_calls(){
 let mut p=load(&[0xcd,0x21],"files.com").unwrap();let mut out=vec![];
 let base=u32::from(PSP)*16;
 p.ram.write_bytes(base+0x200,b"MISSING.BAS\0").unwrap();
 p.cpu.regs[0]=0xabcd3d00;p.cpu.regs[2]=0x200;
 service(&mut p,0x21,&mut out).unwrap();
 assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.eflags&1),(2,1));
 // Calling AH59 repeatedly retains the last error and preserves upper halves.
 for _ in 0..2{
  p.cpu.regs[0]=0xabcd5902;p.cpu.regs[3]=0x12340000;p.cpu.regs[1]=0x5678aaff;p.cpu.eflags|=1;
  service(&mut p,0x21,&mut out).unwrap();
  assert_eq!(p.cpu.regs[0],0xabcd0002);assert_eq!(p.cpu.regs[3],0x12340801);
  assert_eq!(p.cpu.regs[1],0x567800ff);assert_eq!(p.cpu.eflags&1,0);
 }
 let mut child=p.clone();child.cpu.regs[0]=0x5900;child.cpu.regs[3]=0;
 service(&mut child,0x21,&mut out).unwrap();assert_eq!(child.cpu.regs[0],2);
 // A later invalid close replaces the error. Nonzero BX is not modelled.
 p.cpu.regs[0]=0x3e00;p.cpu.regs[3]=0xffff;service(&mut p,0x21,&mut out).unwrap();
 p.cpu.regs[0]=0x5900;p.cpu.regs[3]=0;service(&mut p,0x21,&mut out).unwrap();assert_eq!(p.cpu.regs[0],6);
 p.cpu.regs[0]=0x5900;p.cpu.regs[3]=1;assert!(service(&mut p,0x21,&mut out).is_err());
}
/// AH09 with `text` (ending in $) at PSP:0300.
fn ansi_print(p:&mut Loaded,text:&[u8])->Result<(),String>{
 let psp=u32::from(PSP)*16;p.ram.write_bytes(psp+0x300,text).unwrap();
 p.cpu.regs[0]=0x0900;p.cpu.regs[2]=0x300;service(p,0x21,&mut vec![]).map(|_|())
}
#[test]
fn ansi_driver_draws_colours_moves_the_cursor_and_clears(){
 let mut p=load(&[0xcd,0x21],"ansi.com").unwrap();
 // Without the driver the escape is an ordinary character (the arrow glyph).
 ansi_print(&mut p,b"\x1b[1m$").unwrap();assert_eq!((cell(&p,0,0,0),cell(&p,0,0,1)),(0x071b,0x075b));
 p.ram.store(0x450,MemWidth::Word,0).unwrap();
 p.cpu.regs[0]=0x1a00;service(&mut p,0x2f,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,0x1a00,"not installed");
 console::ansi::install(&mut p).unwrap();
 p.cpu.regs[0]=0x1a00;service(&mut p,0x2f,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,0x1aff,"installed");
 // SGR: bright cyan on blue, then reset; ANSI colour 6 (cyan) is CGA 3, 4 (blue) is CGA 1.
 ansi_print(&mut p,b"\x1b[1;36;44mA\x1b[0mB\x1b[31;7mC\x1b[m$").unwrap();
 assert_eq!((cell(&p,0,0,0),cell(&p,0,0,1),cell(&p,0,0,2)),(0x1b41,0x0742,0x4043));
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),3);
 // A sequence split across two calls continues; H is 1-based and clamps.
 ansi_print(&mut p,b"\x1b[10;$").unwrap();assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),3);
 ansi_print(&mut p,b"5HX\x1b[2AY\x1b[99B\x1b[3DZ\x1b[200;200f$").unwrap();
 assert_eq!((cell(&p,0,9,4)&255,cell(&p,0,7,5)&255,cell(&p,0,24,3)&255),(u32::from(b'X'),u32::from(b'Y'),u32::from(b'Z')));
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0x184f);
 // Save and restore; K erases to the line end in the current attribute.
 ansi_print(&mut p,b"\x1b[1;1H\x1b[s\x1b[5;5H\x1b[u\x1b[42m\x1b[K$").unwrap();
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0);
 assert!((0..80).all(|c|cell(&p,0,0,c)==0x2720));
 // An ESC not followed by [ is shown with the next byte.
 ansi_print(&mut p,b"\x1bx$").unwrap();assert_eq!((cell(&p,0,0,0),cell(&p,0,0,1)),(0x271b,0x2778));
 // 2J clears the whole page in the attribute and homes the cursor; LF at the bottom scrolls in it.
 ansi_print(&mut p,b"\x1b[0;44m\x1b[2J$").unwrap();
 assert!((0..25).all(|r|(0..80).all(|c|cell(&p,0,r,c)==0x1720)));assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0);
 ansi_print(&mut p,b"\x1b[25;1H\x1b[0mq\r\n$").unwrap();
 assert_eq!((cell(&p,0,23,0),cell(&p,0,24,0)),(0x0771,0x0720));
}
#[test]
fn ansi_driver_faults_change_nothing(){
 let mut p=load(&[0xcd,0x21],"ansi.com").unwrap();console::ansi::install(&mut p).unwrap();
 ansi_print(&mut p,b"\x1b[3$").unwrap();
 // A read-only screen cell: neither the screen nor the half-read sequence changes.
 let psp=u32::from(PSP)*16;
 p.ram.write_bytes(psp+0x300,b"1mA$").unwrap();p.ram.protect(0xb8000,1).unwrap();let old=p.clone();
 p.cpu.regs[0]=0x0900;p.cpu.regs[2]=0x300;assert!(service(&mut p,0x21,&mut vec![]).is_err());assert!(p.ram==old.ram,"faulting write changed RAM");
 // Status reports, mode sets and key reassignment are refused, with nothing changed.
 let mut p=load(&[0xcd,0x21],"ansi.com").unwrap();console::ansi::install(&mut p).unwrap();
 for seq in [&b"ab\x1b[6n$"[..],b"\x1b[=3h$",b"\x1b[7l$",b"\x1b[0;68;\"dir\";13p$"]{
  p.ram.write_bytes(psp+0x300,seq).unwrap();let old=p.clone();
  p.cpu.regs[0]=0x0900;p.cpu.regs[2]=0x300;let err=service(&mut p,0x21,&mut vec![]).unwrap_err();
  assert!(err.contains("not modelled"),"{err}");assert!(p.ram==old.ram,"refused sequence changed RAM");
 }
}

#[test]
fn ansi_driver_rejects_guest_corrupted_parameter_count_without_effects(){
 let mut p=load(&[0xcd,0x21],"ansi.com").unwrap();console::ansi::install(&mut p).unwrap();
 let state=(DOSDATA as u32)*16+0x390;
 for count in [17,255]{
  p.ram.store(state+1,MemWidth::Byte,2).unwrap();
  p.ram.store(state+5,MemWidth::Byte,count).unwrap();
  for byte in [b'1',b'm']{
   let old=p.clone();
   let err=console::dos_screen(&p,&[byte]).unwrap_err();
   assert!(err.contains("invalid ANSI driver parameter count"),"{err}");
   assert!(p.ram==old.ram,"invalid parser state changed RAM");
  }
 }
}
#[test]
fn quarterdeck_multiplex_scan_reports_absent_without_changing_guest_state(){
 let mut p=load(&[0xcd,0x2f],"rpciscan.com").unwrap();let mut out=vec![];
 for id in 0xc0..=0xffu32{
  p.cpu.regs[0]=0x12340000|(id<<8);p.cpu.regs[3]=0xaaaa5144;
  p.cpu.regs[1]=0xbbbb4d45;p.cpu.regs[2]=0xcccc4d30;
  p.cpu.eflags=0xad7;p.cpu.undefined_flags=0x45;
  let cpu=p.cpu.clone();let ram=p.ram.clone();let image=p.image.clone();
  assert_eq!(service(&mut p,0x2f,&mut out).unwrap(),None);
  assert_eq!(p.cpu,cpu);assert_eq!(p.ram,ram);assert_eq!(p.image,image);
 }
 for (ax,bx,cx,dx) in [(0xd201,0x5144,0x4d45,0x4d30),(0xd200,0x5145,0x4d45,0x4d30),
                        (0xd200,0x5144,0x4d46,0x4d30),(0xd200,0x5144,0x4d45,0x4d31)]{
  p.cpu.regs[0]=ax;p.cpu.regs[3]=bx;p.cpu.regs[1]=cx;p.cpu.regs[2]=dx;
  let cpu=p.cpu.clone();assert!(service(&mut p,0x2f,&mut out).is_err());assert_eq!(p.cpu,cpu);
 }
}
#[test]
fn guest_multiplex_handler_receives_quarterdeck_probe_first(){
 let mut p=load(&[0xcd,0x2f,0xfa,0xf4],"rpcitsr.com").unwrap();
 p.ram.write_bytes(u32::from(PSP)*16+0x120,&[0xb0,0xff,0xcf]).unwrap();
 p.ram.store(0x2f*4,MemWidth::Dword,(u32::from(PSP)<<16)|0x120).unwrap();
 p.cpu.regs[0]=0xd200;p.cpu.regs[3]=0x5144;p.cpu.regs[1]=0x4d45;p.cpu.regs[2]=0x4d30;
 let result=run(p,20).unwrap();assert!(matches!(result.stop,Stop::Monitor(_)),"{:?}",result.stop);
 assert_eq!(result.loaded.cpu.regs[0]&0xffff,0xd2ff);
}

#[test]
fn absent_task_switcher_returns_the_caller_state_unchanged(){
 let mut p=load(&[0xcd,0x2f],"switcher.com").unwrap();
 p.cpu.regs[0]=0x12344b02;p.cpu.regs[3]=0;p.cpu.regs[7]=0;
 p.cpu.eflags=0xad7;p.cpu.undefined_flags=0x45;
 let old=p.cpu.clone();let image=p.image.clone();let ram=p.ram.clone();
 assert_eq!(service(&mut p,0x2f,&mut vec![]).unwrap(),None);
 assert_eq!(p.cpu,old);assert_eq!(p.image,image);assert_eq!(p.ram,ram);
}

// Graphics-mode text: an independent decoding of the glyph file, then the
// framebuffer bytes each mode must hold for one character cell.
fn glyph_rows(ch:u8)->[u8;8]{
 let text=include_str!("../../src/dos_font8x8.txt");
 let lines:Vec<&str>=text.lines().filter(|l|!l.starts_with(';')).collect();
 let at=usize::from(ch)*9;assert!(lines[at].starts_with(&format!("{ch:02X}")));
 let mut rows=[0u8;8];
 for r in 0..8{for(b,c)in lines[at+1+r].chars().enumerate(){if c=='#'{rows[r]|=0x80>>b;}}}
 rows
}
fn graphics_mode(mode:u32)->Loaded{
 let mut p=load(&[0xf4],"gfx.com").unwrap();p.cpu.regs[0]=mode;console::display(&mut p,0).unwrap();p
}
fn int10(p:&mut Loaded,ax:u32,bx:u32,cx:u32,dx:u32){
 p.cpu.regs[0]=ax;p.cpu.regs[3]=bx;p.cpu.regs[1]=cx;p.cpu.regs[2]=dx;service(p,0x10,&mut vec![]).unwrap();
}
/// Byte address of scanline y, byte column `byte` in the CGA banks.
fn cga_line(y:u32,byte:u32)->u32{0xb8000+(y&1)*0x2000+(y/2)*80+byte}
fn two_bit(bits:u8,color:u32)->[u32;2]{
 let half=|nibble:u8|(0..4).fold(0,|n,i|n<<2|if nibble&(8>>i)!=0{color}else{0});
 [half(bits>>4),half(bits&15)]
}
#[test]
fn rom_font_sits_read_only_at_f000_fa6e_and_has_every_low_glyph(){
 let p=load(&[0xf4],"font.com").unwrap();
 for ch in 0..128u8{for(r,row)in glyph_rows(ch).into_iter().enumerate(){
  assert_eq!(p.ram.load(0xffa6e+u32::from(ch)*8+r as u32,MemWidth::Byte).unwrap(),u32::from(row),"glyph {ch:02X} row {r}");
 }}
 assert_eq!(glyph_rows(b' '),[0;8]);assert_ne!(glyph_rows(b'A'),[0;8]);
 // Printable glyphs are distinct, so AH08h can tell them apart.
 let shapes:std::collections::BTreeSet<[u8;8]>=(0x21..0x7f).map(glyph_rows).collect();assert_eq!(shapes.len(),0x7f-0x21);
 assert!(p.ram.check_write(0xffa6e,MemWidth::Byte).is_err());
 assert_eq!(p.ram.load(0xffff5,MemWidth::Byte).unwrap(),u32::from(b'0'));
}
#[test]
fn single_page_graphics_ignores_other_display_pages_and_can_write_text(){
 for mode in [4,5,6,0x13]{
  let mut p=graphics_mode(mode);
  int10(&mut p,0x0501,0,0,0);
  assert_eq!(p.ram.load(0x462,MemWidth::Byte).unwrap(),0);
  assert_eq!(p.ram.load(0x44e,MemWidth::Word).unwrap(),0);
  int10(&mut p,0x0944,0x000e,1,0x0101);
  int10(&mut p,0x0e41,1,0,0);
  assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),1);
  // Even if a guest changed the active-page byte itself, AH=09 uses page 0.
  p.ram.store(0x462,MemWidth::Byte,1).unwrap();
  int10(&mut p,0x0944,0x000e,1,0x0101);
 }
 // Switching from a text page must reset the mode 13h display and cursor.
 let mut p=graphics_mode(3);int10(&mut p,0x0501,0,0,0);
 int10(&mut p,0x0200,0,0,0x0102);int10(&mut p,0x0013,0,0,0);
 assert_eq!(p.ram.load(0x44e,MemWidth::Word).unwrap(),0);
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0);
}
#[test]
fn teletype_draws_glyphs_in_each_graphics_mode_and_moves_the_cursor(){
 // Mode 4: two bits a pixel, so each glyph row is two bytes; BL=2.
 let mut p=graphics_mode(4);int10(&mut p,0x0e41,0x0002,0,0);
 for(r,bits)in glyph_rows(b'A').into_iter().enumerate(){
  let [a,b]=two_bit(bits,2);let y=r as u32;
  assert_eq!((p.ram.load(cga_line(y,0),MemWidth::Byte).unwrap(),p.ram.load(cga_line(y,1),MemWidth::Byte).unwrap()),(a,b),"row {r}");
 }
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0x0001);
 // Mode 6: 80 columns of one byte; the cursor wraps after column 79.
 let mut p=graphics_mode(6);p.ram.store(0x450,MemWidth::Word,0x024f).unwrap();int10(&mut p,0x0e5a,0x0001,0,0);
 for(r,bits)in glyph_rows(b'Z').into_iter().enumerate(){assert_eq!(p.ram.load(cga_line(16+r as u32,79),MemWidth::Byte).unwrap(),u32::from(bits));}
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0x0300);
 // Mode 13h: one byte a pixel in the BL colour, background 0.
 let mut p=graphics_mode(0x13);p.ram.store(0x450,MemWidth::Word,0x0103).unwrap();int10(&mut p,0x0e40,0x002a,0,0);
 for(r,bits)in glyph_rows(b'@').into_iter().enumerate(){for b in 0..8{
  let at=0xa0000+(8+r as u32)*320+24+b;assert_eq!(p.ram.load(at,MemWidth::Byte).unwrap(),if bits&(0x80>>b)!=0{0x2a}else{0});
 }}
 // Control bytes move the cursor without drawing; text RAM rules are not used.
 let mut p=graphics_mode(4);let before=p.ram.clone();
 for ch in [7u32,b'\r' as u32,b'\n' as u32]{int10(&mut p,0x0e00|ch,3,0,0);}
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0x0100);
 assert_eq!(&p.ram.bytes()[0xb8000..0xbc000],&before.bytes()[0xb8000..0xbc000]);
 p.cpu.regs[0]=0x0e09;assert!(service(&mut p,0x10,&mut vec![]).is_err());
}
#[test]
fn dos_output_in_graphics_uses_freedos_colour_seven(){
 // MOV AX,4 / INT 10h / MOV AH,2 / MOV DL,'H' / INT 21h / CLI / HLT
 let code=[0xb8,0x04,0x00,0xcd,0x10,0xb4,0x02,0xb2,b'H',0xcd,0x21,0xfa,0xf4];
 let r=run(load(&code,"dos.com").unwrap(),10).unwrap();assert_eq!(r.output,b"H");
 for(r_,bits)in glyph_rows(b'H').into_iter().enumerate(){
  let [a,b]=two_bit(bits,3);let y=r_ as u32;
  assert_eq!((r.loaded.ram.load(cga_line(y,0),MemWidth::Byte).unwrap(),r.loaded.ram.load(cga_line(y,1),MemWidth::Byte).unwrap()),(a,b));
 }
 // A line feed on row 24 scrolls the screen up one character row (8 lines).
 let mut p=graphics_mode(6);p.ram.store(cga_line(8,5),MemWidth::Byte,0x81).unwrap();p.ram.store(cga_line(199,0),MemWidth::Byte,0xff).unwrap();
 p.ram.store(0x450,MemWidth::Word,0x1800).unwrap();
 let writes=console::dos_screen(&p,b"\n").unwrap();for(at,w,v)in writes{p.ram.store(at,w,v).unwrap();}
 assert_eq!(p.ram.load(cga_line(0,5),MemWidth::Byte).unwrap(),0x81);assert_eq!(p.ram.load(cga_line(8,5),MemWidth::Byte).unwrap(),0);
 assert_eq!(p.ram.load(cga_line(191,0),MemWidth::Byte).unwrap(),0xff);assert_eq!(p.ram.load(cga_line(199,0),MemWidth::Byte).unwrap(),0);
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0x1800);
}
#[test]
fn repeated_graphics_characters_xor_read_back_and_stay_on_screen(){
 let mut p=graphics_mode(4);p.ram.store(0x450,MemWidth::Word,0x0526).unwrap();
 int10(&mut p,0x0957,0x0001,2,0);
 for col in [38u32,39]{for(r,bits)in glyph_rows(b'W').into_iter().enumerate(){
  let [a,b]=two_bit(bits,1);let y=40+r as u32;
  assert_eq!((p.ram.load(cga_line(y,col*2),MemWidth::Byte).unwrap(),p.ram.load(cga_line(y,col*2+1),MemWidth::Byte).unwrap()),(a,b));
 }}
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0x0526);
 int10(&mut p,0x0800,0,0,0);assert_eq!(p.cpu.regs[0]&0xffff,u32::from(b'W'));
 // Bit 7 XORs: drawing the same character again clears both cells.
 int10(&mut p,0x0a57,0x0081,2,0);
 assert!((40..48).all(|y|(76..80).all(|b|p.ram.load(cga_line(y,b),MemWidth::Byte).unwrap()==0)));
 int10(&mut p,0x0800,0,0,0);assert_eq!(p.cpu.regs[0]&0xffff,0);
 // A run past the last cell is refused before anything is drawn.
 p.ram.store(0x450,MemWidth::Word,0x1827).unwrap();let before=p.ram.clone();
 p.cpu.regs[0]=0x0941;p.cpu.regs[1]=2;assert!(service(&mut p,0x10,&mut vec![]).is_err());assert_eq!(p.ram,before);
 // The 40-column modes refuse a cursor in column 40.
 p.ram.store(0x450,MemWidth::Word,0x0028).unwrap();p.cpu.regs[0]=0x0e41;assert!(service(&mut p,0x10,&mut vec![]).is_err());
}
#[test]
fn upper_half_characters_come_from_the_int_1f_table(){
 let mut p=graphics_mode(6);
 // INT 1Fh starts at Dine's own read-only 80h-FFh table (C000:1000), as a
 // VGA BIOS points it at its own; GW-BASIC draws 94h and FFh in graphics.
 assert_eq!(p.ram.load(0x1f*4,MemWidth::Dword).unwrap(),0xc000_1000);
 for ch in 0x80..=0xffu8{for(r,row)in glyph_rows(ch).into_iter().enumerate(){
  assert_eq!(p.ram.load(0xc1000+u32::from(ch-0x80)*8+r as u32,MemWidth::Byte).unwrap(),u32::from(row),"glyph {ch:02X} row {r}");
 }}
 assert!(p.ram.check_write(0xc1000,MemWidth::Byte).is_err());
 let shapes:std::collections::BTreeSet<[u8;8]>=(0x21..0xff).filter(|&c|c!=0x7f).map(glyph_rows).collect();assert_eq!(shapes.len(),0xff-0x21-1,"printable glyphs are distinct");
 int10(&mut p,0x0e94,1,0,0);
 for(r,v)in glyph_rows(0x94).iter().enumerate(){assert_eq!(p.ram.load(cga_line(r as u32,0),MemWidth::Byte).unwrap(),u32::from(*v));}
 p.ram.store(0x450,MemWidth::Word,0).unwrap();int10(&mut p,0x0800,0,0,0);assert_eq!(p.cpu.regs[0]&255,0x94);
 int10(&mut p,0x1130,0x0400,0,0);assert_eq!((p.selectors[SegmentRegister::Es.index()],p.cpu.regs[5]&0xffff),(0xc000,0x1000));
 // A vector set to 0 leaves no table: the call stops rather than drawing the interrupt table.
 p.ram.store(0x1f*4,MemWidth::Dword,0).unwrap();p.ram.store(0x450,MemWidth::Word,0).unwrap();let before=p.ram.clone();
 p.cpu.regs[0]=0x0e80;p.cpu.regs[3]=1;
 let e=service(&mut p,0x10,&mut vec![]).unwrap_err();assert!(e.contains("INT 1Fh"),"{e}");assert_eq!(p.ram,before);
 // A table at 2000:0000 supplies 80h onward, as GRAFTABL would.
 let shape=[0x18u32,0x3c,0x7e,0xff,0xff,0x7e,0x3c,0x18];
 for(n,v)in shape.iter().enumerate(){p.ram.store(0x20008+n as u32,MemWidth::Byte,*v).unwrap();}
 p.ram.store(0x1f*4,MemWidth::Dword,0x2000_0000).unwrap();
 int10(&mut p,0x0e81,1,0,0);
 for(r,v)in shape.iter().enumerate(){assert_eq!(p.ram.load(cga_line(r as u32,0),MemWidth::Byte).unwrap(),*v);}
 p.ram.store(0x450,MemWidth::Word,0).unwrap();int10(&mut p,0x0800,0,0,0);assert_eq!(p.cpu.regs[0]&255,0x81);
}
#[test]
fn graphics_scroll_moves_character_rows_and_fills_with_bh(){
 let mut p=graphics_mode(0x13);
 p.ram.store(0xa0000+16*320+9,MemWidth::Byte,0x55).unwrap();p.ram.store(0xa0000+16*320+100,MemWidth::Byte,0x66).unwrap();
 // Rows 1..3, columns 1..2 (pixels 8..23), up one row, fill 9.
 int10(&mut p,0x0601,0x0900,0x0101,0x0302);
 assert_eq!(p.ram.load(0xa0000+8*320+9,MemWidth::Byte).unwrap(),0x55);
 assert_eq!(p.ram.load(0xa0000+16*320+100,MemWidth::Byte).unwrap(),0x66,"outside the window");
 assert!((24..32).all(|y|(8..24).all(|x|p.ram.load(0xa0000+y*320+x,MemWidth::Byte).unwrap()==9)));
 assert_eq!(p.ram.load(0xa0000+24*320+24,MemWidth::Byte).unwrap(),0);
 // Down one row in mode 5 moves the pixel back and fills the top row.
 let mut p=graphics_mode(5);p.ram.store(cga_line(8,0),MemWidth::Byte,0xc0).unwrap();
 int10(&mut p,0x0701,0x0200,0x0000,0x184f);
 assert_eq!(p.ram.load(cga_line(16,0),MemWidth::Byte).unwrap(),0xc0);
 assert!((0..8).all(|y|(0..80).all(|b|p.ram.load(cga_line(y,b),MemWidth::Byte).unwrap()==0xaa)));
}
#[test]
fn mode_five_is_four_colour_with_its_own_crt_mode_byte(){
 let mut p=graphics_mode(5);
 assert_eq!(p.ram.load(0x465,MemWidth::Byte).unwrap(),0x2e);assert_eq!(p.ram.load(0x44a,MemWidth::Word).unwrap(),40);
 int10(&mut p,0x0f00,0,0,0);assert_eq!(p.cpu.regs[0]&0xffff,0x2805);
 int10(&mut p,0x0c03,0,319,199);assert_eq!(p.ram.load(cga_line(199,79),MemWidth::Byte).unwrap(),3);
 int10(&mut p,0x0b00,0x0100,0,0);assert_eq!(p.ram.load(0x466,MemWidth::Byte).unwrap(),0x10);
 for(mode,value)in [(4u32,0x2au32),(6,0x1e),(3,0x29)]{let p=graphics_mode(mode);assert_eq!(p.ram.load(0x465,MemWidth::Byte).unwrap(),value);}
 assert_eq!(load(&[0xf4],"boot.com").unwrap().ram.load(0x465,MemWidth::Byte).unwrap(),0x29);
}

// EGA/VGA 16-colour planar modes and the VGA BIOS calls (Tony chose a VGA
// BIOS identity on 7 November 2026).
#[test]
fn planar_mode_sets_fill_the_bios_data_area_and_map_the_planes(){
 for(mode,cols,rows,cell,page)in [(0x0du32,40u32,24u32,8u32,0x2000u32),(0x0e,80,24,8,0x4000),(0x10,80,24,14,0x8000),(0x12,80,29,16,0xa000)]{
  let mut p=graphics_mode(mode);
  assert_eq!(p.ram.load(0x449,MemWidth::Byte).unwrap(),mode);assert_eq!(p.ram.load(0x44a,MemWidth::Word).unwrap(),cols);
  assert_eq!(p.ram.load(0x484,MemWidth::Byte).unwrap(),rows);assert_eq!(p.ram.load(0x485,MemWidth::Word).unwrap(),cell);
  assert_eq!(p.ram.load(0x44c,MemWidth::Word).unwrap(),page);
  let planar=p.ram.planar().unwrap();assert_eq!((planar.seq[2],planar.gc[5],planar.gc[8]),(0x0f,0,0xff));
  int10(&mut p,0x0f00,0,0,0);assert_eq!(p.cpu.regs[0]&0xffff,cols<<8|mode);
  // The window is the planes: a write reaches all four, RAM behind stays.
  p.ram.store(0xa0000,MemWidth::Byte,0xa5).unwrap();assert_eq!(p.ram.bytes()[0xa0000],0);
  assert!((0..4).all(|n|p.ram.planar().unwrap().peek(n,0)==0xa5));
  int10(&mut p,0x0003,0,0,0);assert!(p.ram.planar().is_none());assert_eq!(p.ram.load(0xa0000,MemWidth::Byte).unwrap(),0);
  assert_eq!(p.ram.load(0x485,MemWidth::Word).unwrap(),16);
 }
 // Bit 7 keeps the planes across a planar mode set.
 let mut p=graphics_mode(0x12);p.ram.store(0xa0010,MemWidth::Byte,0x3c).unwrap();
 int10(&mut p,0x0090,0,0,0);assert_eq!(p.ram.planar().unwrap().peek(2,0x10),0x3c);
 int10(&mut p,0x0010,0,0,0);assert_eq!(p.ram.planar().unwrap().peek(2,0x10),0);
}
#[test]
fn planar_pages_keep_pixels_text_and_display_separate(){
 let mut p=graphics_mode(0x10);
 int10(&mut p,0x0c03,0,0,0);
 int10(&mut p,0x0c0c,0x0100,0,0);
 assert_eq!(p.ram.planar().unwrap().pixel(0,0),3);
 assert_eq!(p.ram.planar().unwrap().pixel(0x8000,0),12);
 let (_,_,page0,_)=planar::snapshot(&p).unwrap();assert!(page0.starts_with("80"));
 int10(&mut p,0x0501,0,0,0);
 let (_,_,page1,_)=planar::snapshot(&p).unwrap();assert!(page1.starts_with("00"));
 int10(&mut p,0x0e41,7,0,0);
 assert_eq!(p.ram.planar().unwrap().pixel(0,0),3);
 assert_eq!(p.ram.load(0x452,MemWidth::Word).unwrap(),1);
 p.cpu.regs[0]=0x0502;assert!(service(&mut p,0x10,&mut vec![]).is_err());
}
#[test]
fn planar_character_writes_use_the_page_in_bh(){
 // GW-BASIC's SCREEN 9,,1,0 prints on hidden page 1 while page 0 is shown:
 // AH02h and AH09h with BH=1 must leave page 0 alone.
 let mut p=graphics_mode(0x10);
 int10(&mut p,0x0200,0x0100,0,0x0203);
 assert_eq!(p.ram.load(0x452,MemWidth::Word).unwrap(),0x0203);
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0);
 int10(&mut p,0x0941,0x010e,1,0);
 let planar=p.ram.planar().unwrap();let cell=(2*14)*80+3;
 assert!((0..14).any(|r|planar.peek(1,0x8000+cell+r*80)!=0));
 assert!((0..0x8000).all(|i|(0..4).all(|k|planar.peek(k,i)==0)));
 p.cpu.regs[0]=0x0800;p.cpu.regs[3]=0x0100;service(&mut p,0x10,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[0]&255,0x41);
 p.cpu.regs[0]=0x0800;p.cpu.regs[3]=0;service(&mut p,0x10,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[0]&255,0);
}
#[test]
fn planar_bios_pixels_combine_four_planes(){
 let mut p=graphics_mode(0x12);
 int10(&mut p,0x0c0b,0,639,479);
 let at=479*80+79;assert_eq!([0,1,2,3].map(|n|p.ram.planar().unwrap().peek(n,at)),[1,1,0,1]);
 int10(&mut p,0x0d00,0,639,479);assert_eq!(p.cpu.regs[0]&255,0x0b);
 int10(&mut p,0x0c86,0,639,479);int10(&mut p,0x0d00,0,639,479);assert_eq!(p.cpu.regs[0]&255,0x0d,"XOR 0Bh^06h");
 p.cpu.regs[0]=0x0c01;p.cpu.regs[1]=0;p.cpu.regs[2]=480;assert!(service(&mut p,0x10,&mut vec![]).is_err());
 let mut p=graphics_mode(0x10);p.cpu.regs[0]=0x0c01;p.cpu.regs[1]=0;p.cpu.regs[2]=350;assert!(service(&mut p,0x10,&mut vec![]).is_err());
 int10(&mut p,0x0c0f,0,0,349);assert_eq!(p.ram.planar().unwrap().peek(3,349*80),0x80);
}
#[test]
fn guest_programs_the_graphics_controller_with_word_outs(){
 // MOV AX,12h; INT 10h; MOV AX,A000h; MOV ES,AX; MOV DX,3CEh;
 // MOV AX,0205h; OUT DX,AX (write mode 2); MOV AX,3C08h; OUT DX,AX (mask);
 // MOV AL,[ES:0]; MOV AL,9; MOV [ES:0],AL; MOV DX,3C4h; MOV AX,0102h;
 // OUT DX,AX (map mask plane 0); IN AL,DX (index 02h back); CLI; HLT
 let code=[0xb8,0x12,0x00,0xcd,0x10,0xb8,0x00,0xa0,0x8e,0xc0,0xba,0xce,0x03,
  0xb8,0x05,0x02,0xef,0xb8,0x08,0x3c,0xef,0x26,0xa0,0x00,0x00,0xb0,0x09,0x26,0xa2,0x00,0x00,
  0xba,0xc4,0x03,0xb8,0x02,0x01,0xef,0xec,0xfa,0xf4];
 let r=run(load(&code,"gc.com").unwrap(),40).unwrap();
 let planar=r.loaded.ram.planar().unwrap();
 assert_eq!([0,1,2,3].map(|n|planar.peek(n,0)),[0x3c,0,0,0x3c]);
 assert_eq!((planar.gc[5],planar.gc[8],planar.seq[2]),(2,0x3c,1));
 assert_eq!(r.loaded.cpu.regs[0]&255,2);
 // Chain-4 or odd/even addressing is refused with nothing changed.
 let code=[0xb8,0x12,0x00,0xcd,0x10,0xba,0xc4,0x03,0xb8,0x04,0x0e,0xef,0xfa,0xf4];
 let r=run(load(&code,"chain.com").unwrap(),20).unwrap();
 assert!(matches!(&r.stop,Stop::Service(e)|Stop::Fault(e) if e.contains("outside the planar profile")),"{:?}",r.stop);
 assert_eq!(r.loaded.ram.planar().unwrap().seq[4],6);
 // Outside the planar modes the map mask is kept, but chain-4 is refused.
 let code=[0xba,0xc4,0x03,0xb8,0x02,0x0f,0xef,0xb8,0x04,0x0e,0xef,0xfa,0xf4];
 let r=run(load(&code,"text.com").unwrap(),20).unwrap();
 assert!(matches!(&r.stop,Stop::Service(e)|Stop::Fault(e) if e.contains("outside the 16-colour planar modes")),"{:?}",r.stop);
}
#[test]
fn attribute_controller_flip_flop_and_palette_reach_the_display_colours(){
 let mut p=graphics_mode(0x12);
 // Default palette: pixel 6 is attribute 14h, EGA brown (2Ah,15h,00h).
 assert_eq!(p.vga.planar_rgb()[6],[0xaa,0x55,0]);
 // 3DAh read resets the flip-flop; index 01h, data 3Fh (white).
 let code=[0xba,0xda,0x03,0xec,0xba,0xc0,0x03,0xb0,0x01,0xee,0xb0,0x3f,0xee,0xb0,0x01,0xee,0x42,0xec,0xfa,0xf4];
 // The program runs in a fresh load; only the planes and VGA state are copied.
 let mut q=load(&code,"attr.com").unwrap();q.ram.set_planar(p.ram.planar().cloned());q.vga=p.vga.clone();
 q.ram.store(0x449,MemWidth::Byte,0x12).unwrap();
 let r=run(q,20).unwrap();assert_eq!(r.loaded.vga.planar_rgb()[1],[0xff,0xff,0xff]);assert_eq!(r.loaded.cpu.regs[0]&255,0x3f);
 // AH=10h: set and read a palette register, the overscan and a DAC entry.
 int10(&mut p,0x1000,0x2a02,0,0);int10(&mut p,0x1007,0x0002,0,0);assert_eq!((p.cpu.regs[3]>>8)&255,0x2a);
 int10(&mut p,0x1001,0x0500,0,0);int10(&mut p,0x1008,0,0,0);assert_eq!((p.cpu.regs[3]>>8)&255,5);
 int10(&mut p,0x1010,0x002a,0x2030,0x1000);int10(&mut p,0x1015,0x002a,0,0);
 assert_eq!((p.cpu.regs[2]>>8&255,p.cpu.regs[1]&0xffff),(0x10,0x2030));
 assert_eq!(p.vga.planar_rgb()[2],[64,129,194]);
 // Block write and read through ES:DX, 2 entries from 40h.
 p.selectors[SegmentRegister::Es.index()]=0x2000;
 for(n,v)in [1u32,2,3,4,5,6].iter().enumerate(){p.ram.store(0x20000+n as u32,MemWidth::Byte,*v).unwrap();}
 int10(&mut p,0x1012,0x0040,2,0);assert_eq!((p.vga.dac(0x40),p.vga.dac(0x41)),([1,2,3],[4,5,6]));
 int10(&mut p,0x1017,0x0040,2,0x10);assert_eq!(&p.ram.bytes()[0x20010..0x20016],&[1,2,3,4,5,6]);
 p.cpu.regs[0]=0x1013;assert!(service(&mut p,0x10,&mut vec![]).is_err());
 // 200-line modes use CGA colours: attribute 0Eh -> DAC 16h, yellow.
 let p=graphics_mode(0x0d);assert_eq!(p.vga.planar_rgb()[14],[0xff,0xff,0x55]);
}
#[test]
fn vga_bios_identifies_itself_and_reports_its_8x8_font(){
 let mut p=load(&[0xf4],"id.com").unwrap();
 int10(&mut p,0x1a00,0,0,0);assert_eq!((p.cpu.regs[0]&255,p.cpu.regs[3]&0xffff),(0x1a,0x0008));
 int10(&mut p,0x1200,0xff10,0,0);assert_eq!((p.cpu.regs[3]&0xffff,p.cpu.regs[1]&0xffff),(0x0003,0x0009));
 int10(&mut p,0x1200,0x0036,0,0);assert_eq!(p.cpu.regs[0]&0xffff,0x1200,"other AH=12h calls stay unsupported");
 assert_eq!((p.ram.load(0x487,MemWidth::Byte).unwrap(),p.ram.load(0x489,MemWidth::Byte).unwrap()),(0x60,0x11));
 int10(&mut p,0x1130,0x0300,0,0);
 assert_eq!((p.selectors[SegmentRegister::Es.index()],p.cpu.regs[5]&0xffff,p.cpu.regs[1]&0xffff,p.cpu.regs[2]&255),(0xf000,0xfa6e,16,24));
 p.cpu.regs[0]=0x1130;p.cpu.regs[3]=0x0600;assert!(service(&mut p,0x10,&mut vec![]).is_err());
 p.cpu.regs[0]=0x1100;assert!(service(&mut p,0x10,&mut vec![]).is_err());
 p.cpu.regs[0]=0x1a01;assert!(service(&mut p,0x10,&mut vec![]).is_err());
}
#[test]
fn planar_text_uses_tall_cells_and_scrolls_them(){
 let mut p=graphics_mode(0x12);p.ram.store(0x450,MemWidth::Word,0x0102).unwrap();
 int10(&mut p,0x0e41,0x000c,0,0);
 let g=glyph_rows(b'A');
 for r in 0..16u32{let at=(16+r)as usize*80+2;let want=g[(r/2)as usize];
  assert_eq!([0,1,2,3].map(|n|p.ram.planar().unwrap().peek(n,at)),[0,0,want,want],"row {r}");}
 p.ram.store(0x450,MemWidth::Word,0x0102).unwrap();int10(&mut p,0x0800,0,0,0);assert_eq!(p.cpu.regs[0]&255,u32::from(b'A'));
 // Mode 10h: 14-line cells; rows 0 and 7 once, the others twice.
 let mut p=graphics_mode(0x10);int10(&mut p,0x0e48,0x0001,0,0);
 let g=glyph_rows(b'H');let rows:Vec<u8>=(0..14).map(|r|p.ram.planar().unwrap().peek(0,r*80)).collect();
 assert_eq!(rows,vec![g[0],g[1],g[1],g[2],g[2],g[3],g[3],g[4],g[4],g[5],g[5],g[6],g[6],g[7]]);
 // A line feed on the last of 30 rows moves everything up 16 lines.
 let mut p=graphics_mode(0x12);p.ram.planar_mut().unwrap().poke(1,20*80+5,0x81);p.ram.store(0x450,MemWidth::Word,0x1d00).unwrap();
 let writes=console::dos_screen(&p,b"\n").unwrap();for(at,w,v)in writes{p.ram.store(at,w,v).unwrap();}
 assert_eq!(p.ram.planar().unwrap().peek(1,4*80+5),0x81);assert_eq!(p.ram.planar().unwrap().peek(1,20*80+5),0);
 assert_eq!(p.ram.load(0x450,MemWidth::Word).unwrap(),0x1d00);
}
#[test]
fn port_accesses_dine_does_not_answer_stop_instead_of_reading_open_bus(){
 // Dine answers unprefixed IN/OUT itself; a prefixed IN, INS or OUTS reaches the decoder's
 // open bus, which Dine must not use for its devices.
 for bytes in [vec![0x66,0xe5,0x60],vec![0x6c],vec![0xf3,0x6f]]{
  let r=run(load(&bytes,"port.com").unwrap(),10).unwrap();
  assert!(matches!(r.stop,Stop::Service(ref m)if m.contains("not modelled in Dine")),"{bytes:02x?} {:?}",r.stop);assert_eq!(r.loaded.cpu.eip,0x100);
 }
}
#[test]
fn session_mounts_and_unmounts_fat12_floppy_and_host_folders_live(){
 use std::time::{SystemTime,UNIX_EPOCH};
 let dir=std::env::temp_dir().join(format!("pil-mount-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
 let host_d=dir.join("usb folder");std::fs::create_dir_all(&host_d).unwrap();
 std::fs::write(host_d.join("NOTE.TXT"),b"from-usb").unwrap();
 // Build a 360 KiB FAT12 floppy image with DISK1.TXT = "disk-one"
 let mut floppy=vec![0u8;720*512];
 floppy[..3].copy_from_slice(&[0xeb,0x3c,0x90]);
 floppy[11..13].copy_from_slice(&512u16.to_le_bytes());floppy[13]=1;
 floppy[14..16].copy_from_slice(&1u16.to_le_bytes());floppy[16]=2;
 floppy[17..19].copy_from_slice(&112u16.to_le_bytes());
 floppy[19..21].copy_from_slice(&720u16.to_le_bytes());floppy[21]=0xf9;
 floppy[22..24].copy_from_slice(&3u16.to_le_bytes());
 for f in 0..2{
  let at=512+f*3*512;
  floppy[at..at+6].copy_from_slice(&[0xf9,0xff,0xff,0xff,0x0f,0x00]);
 }
 let root=7*512;
 floppy[root..root+11].copy_from_slice(b"DISK1   TXT");
 floppy[root+26..root+28].copy_from_slice(&2u16.to_le_bytes());
 floppy[root+28..root+32].copy_from_slice(&8u32.to_le_bytes());
 floppy[14*512..14*512+8].copy_from_slice(b"disk-one");
 let floppy_path=dir.join("disk1.img");std::fs::write(&floppy_path,&floppy).unwrap();
 let mut s=session::Session::new(load(&[0xf4],"noop.com").unwrap());
 let (reply,_)=s.command(1,&format!("mount A {}",floppy_path.display())).unwrap();
 assert!(reply.contains("\"ok\":true"),"{reply}");
 let (reply,_)=s.command(2,&format!("mount D {}",host_d.display())).unwrap();
 assert!(reply.contains("\"ok\":true"),"{reply}");
 let p=s.loaded().unwrap();
 assert!(files::program(p,b"A:\\DISK1.TXT").unwrap().is_ok());
 assert_eq!(&files::program(p,b"A:\\DISK1.TXT").unwrap().unwrap().0[..],b"disk-one");
 assert_eq!(&files::program(p,b"D:\\NOTE.TXT").unwrap().unwrap().0[..],b"from-usb");
 let (reply,_)=s.command(3,"unmount D").unwrap();
 assert!(reply.contains("\"ok\":true"),"{reply}");
 assert!(files::program(s.loaded().unwrap(),b"D:\\NOTE.TXT").unwrap().is_err());
 let (reply,_)=s.command(4,"unmount A:").unwrap();
 assert!(reply.contains("\"ok\":true"),"{reply}");
 assert!(files::program(s.loaded().unwrap(),b"A:\\DISK1.TXT").unwrap().is_err());
 let (reply,_)=s.command(5,"mount Zinvalid /tmp").unwrap();
 assert!(reply.contains("\"ok\":false"),"{reply}");
 std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn short_file_reads_validate_returned_bytes_instead_of_requested_cx(){
 let mut p=load_with_drives(&[0xcd,0x21,0xf4],"short.com",vec![Source::Memory(Default::default())],None).unwrap();
 let (handle,cf)=call(&mut p,0x3d00,0,b"short.com");assert_eq!(cf,0);
 p.cpu.regs[3]=handle;p.cpu.regs[2]=15;p.cpu.regs[1]=65535;p.cpu.regs[0]=0x3f00;
 service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!((p.cpu.regs[0]&65535,p.cpu.eflags&1),(3,0));
 assert_eq!(p.ram.load(u32::from(PSP)*16+15,MemWidth::Word).unwrap(),0x21cd);
 // EOF: no bytes are written even if CX would cross the segment end.
 p.cpu.regs[2]=65535;p.cpu.regs[1]=65535;p.cpu.regs[0]=0x3f00;
 service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!((p.cpu.regs[0]&65535,p.cpu.eflags&1),(0,0));
 // Seek back, then a genuinely crossing returned span fails without advancing.
 p.cpu.regs[0]=0x4200;p.cpu.regs[1]=0;p.cpu.regs[2]=0;service(&mut p,0x21,&mut vec![]).unwrap();
 p.cpu.regs[0]=0x3f00;p.cpu.regs[1]=3;p.cpu.regs[2]=65535;let ram=p.ram.clone();
 service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!((p.cpu.regs[0]&65535,p.cpu.eflags&1),(5,1));assert_eq!(p.ram,ram);
 p.cpu.regs[0]=0x3f00;p.cpu.regs[2]=0x400;
 service(&mut p,0x21,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0]&65535,3);
}
#[test]
fn absent_dos_extender_probes_preserve_signature_registers_and_report_failure(){
 for ax in [0xbf00,0xbf01,0xbf02,0xbfdc,0xbfde]{
  let mut p=load(&[0xcd,0x15],"probe.com").unwrap();p.cpu.regs=[0xabcd0000|ax,0x12345678,0,0,0xfffe,1,2,3];p.cpu.eflags=0x246;
  let before=p.cpu.clone();service(&mut p,0x15,&mut vec![]).unwrap();
  let mut want=before;want.regs[0]=(want.regs[0]&!0xff00)|0x8600;want.eflags|=1;assert_eq!(p.cpu,want);
 }
}
#[test]
fn extended_memory_size_matches_cmos_and_remains_outside_the_dos_arena(){
 let mut p=load(&[0xcd,0x15],"memory.com").unwrap();p.cpu.regs[0]=0xabcd8800;
 service(&mut p,0x15,&mut vec![]).unwrap();assert_eq!(p.cpu.regs[0],0xabcd3c00);
 p.ram.set_a20_enabled(true);p.ram.store(0xffffff,MemWidth::Byte,0x5a).unwrap();
 assert_eq!(p.ram.load(0xffffff,MemWidth::Byte).unwrap(),0x5a);assert_eq!(p.allocated_end,0xa0000);
 for (index,want) in [(0x17,0),(0x18,0x3c),(0x30,0),(0x31,0x3c)]{
  p.cmos_index=index;p.cpu.eip=0x100;p.ram.write_bytes(0x10100,&[0xe4,0x71]).unwrap();
  ports::io(&mut p).unwrap();assert_eq!(p.cpu.regs[0]&255,want);
 }
}
#[test]
fn prefixed_word_vga_output_in_32bit_code_has_the_correct_width_and_length(){
 let mut p=load(&[0x66,0xef],"ports.com").unwrap();
 p.image.segments[1].access=crate::address::SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0};
 p.cpu.regs[2]=0x3c4;p.cpu.regs[0]=0x0f02;
 assert_eq!(ports::access(&p),Some((0x3c4,2)));assert_eq!(ports::io(&mut p).unwrap(),Some(2));assert_eq!(p.vga.idle.seq[2],15);
 p.ram.write_bytes(0x10100,&[0xef,0x90]).unwrap();assert_eq!(ports::access(&p),Some((0x3c4,4)));assert!(ports::io(&mut p).is_err());
}

#[test]
fn mode13_unchaining_preserves_pixels_plane_masks_and_page_flips(){
 let mut p=load(&[0xf4],"unchain.com").unwrap();p.cpu.regs[0]=0x13;service(&mut p,0x10,&mut vec![]).unwrap();
 p.ram.write_bytes(0xa0000,&[1,2,3,4,5,6,7,8]).unwrap();
 planar::io(&mut p,0x3c4,false,0x0604,true).unwrap();
 assert_eq!(&planar::mode13_snapshot(&p).unwrap()[..16],"0102030405060708");
 for n in 0..4{planar::io(&mut p,0x3c4,false,((1<<n)<<8)|2,true).unwrap();p.ram.store(0xa4000,MemWidth::Byte,0x20+n).unwrap();}
 p.vga.io(0x3d4,false,12).unwrap();p.vga.io(0x3d5,false,0x40).unwrap();
 assert_eq!(&planar::mode13_snapshot(&p).unwrap()[..8],"20212223");
 planar::io(&mut p,0x3c4,false,0x0e04,true).unwrap();assert_eq!(p.ram.load(0xa0001,MemWidth::Byte).unwrap(),2);
 planar::io(&mut p,0x3c4,false,0x0f02,true).unwrap();p.ram.store(0xa0001,MemWidth::Byte,0x77).unwrap();assert_eq!(p.ram.planar().unwrap().peek(1,0),0x77);
 p.cpu.regs[0]=0x93;service(&mut p,0x10,&mut vec![]).unwrap();assert_eq!(&planar::mode13_snapshot(&p).unwrap()[..8],"01770304");
 assert_eq!(p.ram.planar().unwrap().peek(3,0x4000),0x23,"no-clear mode set keeps all plane memory");
 p.cpu.regs[0]=0x13;service(&mut p,0x10,&mut vec![]).unwrap();assert!(p.ram.planar().is_none());assert!(p.ram.bytes()[0xa0000..0xafa00].iter().all(|b|*b==0));
}
#[test]
fn floppy_motor_off_write_does_not_claim_a_disk_controller(){
 for value in [0,4,8,12]{let mut p=load(&[0xee],"motor.com").unwrap();p.cpu.regs[2]=0x3f2;p.cpu.regs[0]=value;assert_eq!(ports::io(&mut p).unwrap(),Some(1));}
 let mut p=load(&[0xee],"motor.com").unwrap();p.cpu.regs[2]=0x3f2;p.cpu.regs[0]=0x1c;assert!(ports::io(&mut p).is_err());
}

#[test]
fn bios_rtc_time_and_date_are_bcd_preserve_registers_and_share_the_session_clock(){
 let mut p=load(&[0xcd,0x1a],"rtc.com").unwrap();
 p.rtc.set_dos_date(None,0,2024,2,29);p.rtc.set_dos_time(None,0,23*3600+59*60+58);
 for (ah,cx,dx) in [(2,0x2359,0x5800),(4,0x2024,0x0229)]{
  p.cpu.regs=[0xabcd0000|(ah<<8)|0x5a,0x1234dead,0x5678beef,3,4,5,6,7];p.cpu.eflags=0xad7;p.cpu.undefined_flags=u32::MAX;
  let mut expected=p.cpu.clone();expected.regs[1]=0x12340000|cx;expected.regs[2]=0x56780000|dx;expected.eflags&=!1;expected.undefined_flags&=!1;
  let ram=p.ram.clone();let image=p.image.clone();let rtc=p.rtc.clone();
  service(&mut p,0x1a,&mut vec![]).unwrap();assert_eq!(p.cpu,expected);assert_eq!(p.ram,ram);assert_eq!(p.image,image);assert_eq!(p.rtc,rtc);
 }
 p.cpu.regs[0]=0x0200;service(&mut p,0x1a,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[1]&65535,(u32::from(p.rtc.read(None,4).unwrap())<<8)|u32::from(p.rtc.read(None,2).unwrap()));
 assert_eq!(p.cpu.regs[2]&65535,u32::from(p.rtc.read(None,0).unwrap())<<8);
}

#[test]
fn child_psp_initializes_fields_and_inherits_parent_state(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 // Shrink initial COM block to 0x40 paragraphs so memory is available for child
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.eflags&1,0);
 // Set custom vectors in IVT
 p.ram.store(0x22*4,MemWidth::Dword,0x1234_5678).unwrap();
 p.ram.store(0x23*4,MemWidth::Dword,0x2345_6789).unwrap();
 p.ram.store(0x24*4,MemWidth::Dword,0x3456_789a).unwrap();
 // Set command tail in parent PSP
 let parent_base=u32::from(PSP)*16;
 p.ram.write_bytes(parent_base+0x80,&[4,b'T',b'E',b'S',b'T',0x0d]).unwrap();
 // Allocate child memory block
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x20;
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.eflags&1,0);
 let child=p.cpu.regs[0]as u16;
 assert!(child>PSP);
 // Call AH=55h to create child PSP
 p.cpu.regs[0]=0x55aa;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(child+0x20);
 p.cpu.eflags=0x202;p.cpu.undefined_flags=0;
 service(&mut p,0x21,&mut vec![]).unwrap();
 // Return conventions: AL destroyed (0xf0), CF unchanged, active PSP switched
 assert_eq!(p.cpu.regs[0]as u8,0xf0);
 assert_eq!(p.cpu.eflags&1,0);
 assert_eq!(p.psp,child);
 assert_eq!(p.dta,(child,0x80));
 // Check child PSP contents in RAM
 let child_base=u32::from(child)*16;
 assert_eq!(&p.ram.bytes()[child_base as usize..child_base as usize+2],&[0xcd,0x20]);
 assert_eq!(p.ram.load(child_base+2,MemWidth::Word).unwrap()as u16,child+0x20); // SI memory end
 assert_eq!(p.ram.load(child_base+0x0a,MemWidth::Dword).unwrap(),0x1234_5678); // INT 22h
 assert_eq!(p.ram.load(child_base+0x0e,MemWidth::Dword).unwrap(),0x2345_6789); // INT 23h
 assert_eq!(p.ram.load(child_base+0x12,MemWidth::Dword).unwrap(),0x3456_789a); // INT 24h
 assert_eq!(p.ram.load(child_base+0x16,MemWidth::Word).unwrap()as u16,PSP); // Parent PSP
 assert_eq!(&p.ram.bytes()[child_base as usize+0x18..child_base as usize+0x1d],&[0,1,2,3,4]); // JFT
 assert_eq!(p.ram.load(child_base+0x2c,MemWidth::Word).unwrap()as u16,ENV); // Env segment
 assert_eq!(p.ram.load(child_base+0x32,MemWidth::Word).unwrap()as u16,20); // JFT size
 assert_eq!(p.ram.load(child_base+0x34,MemWidth::Word).unwrap()as u16,0x18); // JFT offset
 assert_eq!(p.ram.load(child_base+0x36,MemWidth::Word).unwrap()as u16,child); // JFT segment
 assert_eq!(p.ram.load(child_base+0x40,MemWidth::Word).unwrap()as u16,DOS_COMPAT_VERSION);
 assert_eq!(&p.ram.bytes()[child_base as usize+0x50..child_base as usize+0x53],&[0xcd,0x21,0xcb]);
 assert_eq!(&p.ram.bytes()[child_base as usize+0x80..child_base as usize+0x86],&[4,b'T',b'E',b'S',b'T',0x0d]);
}

#[test]
fn child_psp_transfers_mcb_ownership_for_allocated_block(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 // Shrink initial COM block
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 // Allocate block for child
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x10;
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.eflags&1,0);
 let child=p.cpu.regs[0]as u16;
 let mcb_header=u32::from(child-1)*16;
 // Before AH=55h: owner is parent PSP
 assert_eq!(p.ram.load(mcb_header+1,MemWidth::Word).unwrap()as u16,PSP);
 assert_eq!(p.arena.block_owner(child),Some(PSP));
 // Create child PSP
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(child+0x10);
 service(&mut p,0x21,&mut vec![]).unwrap();
 // After AH=55h: owner in RAM and arena is child segment
 assert_eq!(p.ram.load(mcb_header+1,MemWidth::Word).unwrap()as u16,child);
 assert_eq!(p.arena.block_owner(child),Some(child));
}

#[test]
fn child_psp_parent_linkage_and_switching(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 // Shrink initial COM block
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x20;
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.eflags&1,0);
 let child=p.cpu.regs[0]as u16;
 // AH=55h creates child and sets current PSP to child
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(child+0x20);
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.psp,child);
 // Query current PSP via AH=51h and AH=62h
 for ah in [0x51,0x62]{
  p.cpu.regs[0]=ah<<8;
  service(&mut p,0x21,&mut vec![]).unwrap();
  assert_eq!(p.cpu.regs[3]&0xffff,u32::from(child));
 }
 // Switch back to parent via AH=50h
 p.cpu.regs[0]=0x5000;p.cpu.regs[3]=u32::from(PSP);
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.psp,PSP);
 // Query current PSP via AH=51h: returns parent PSP
 p.cpu.regs[0]=0x5100;
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.regs[3]&0xffff,u32::from(PSP));
 // Switch back to child via AH=50h
 p.cpu.regs[0]=0x5000;p.cpu.regs[3]=u32::from(child);
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.psp,child);
}

#[test]
fn child_psp_rejects_invalid_and_boundary_segments(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 // Shrink initial COM block
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 // Segment 0 (IVT): rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=0;p.cpu.regs[6]=0x100;
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // Segment 0x40 (BDA): rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=0x40;p.cpu.regs[6]=0x100;
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // Beyond conventional RAM: rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=0xa000;p.cpu.regs[6]=0xa100;
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=0xffff;p.cpu.regs[6]=0xffff;
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // Same as current PSP: rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(PSP);p.cpu.regs[6]=0x2000;
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // In unallocated free memory: rejected
 let free_seg=p.arena.first_free_segment().unwrap();
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(free_seg);p.cpu.regs[6]=u32::from(free_seg+0x10);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // Read-only protected memory: rejected
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x10;
 service(&mut p,0x21,&mut vec![]).unwrap();
 let seg=p.cpu.regs[0]as u16;
 p.ram.protect(u32::from(seg)*16,256).unwrap();
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(seg);p.cpu.regs[6]=u32::from(seg+0x10);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
}

#[test]
fn child_psp_rejects_allocated_block_smaller_than_psp_and_preserves_memory(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 // Allocate a block of 1 paragraph (16 bytes), which cannot hold 256-byte PSP
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=1;
 service(&mut p,0x21,&mut vec![]).unwrap();
 assert_eq!(p.cpu.eflags&1,0);
 let child=p.cpu.regs[0]as u16;
 let ram_before=p.ram.clone();
 // AH=55h must reject: block is smaller than 0x10 paragraphs
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(child+1);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // RAM and MCB integrity must be completely unchanged
 assert_eq!(p.ram,ram_before);
 assert_eq!(p.arena.block_owner(child),Some(PSP));
}

#[test]
fn child_psp_rejects_mcb_overlap_too_near_end_and_other_program_memory(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x20;
 service(&mut p,0x21,&mut vec![]).unwrap();
 let child=p.cpu.regs[0]as u16;
 let ram_before=p.ram.clone();
 // Overlapping MCB header (child - 1): rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child-1);p.cpu.regs[6]=u32::from(child+0x20);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // Too near end of allocated block (child + 0x11 with size 0x20 leaves only 0x0f paras): rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child+0x11);p.cpu.regs[6]=u32::from(child+0x20);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // Block owned by another program: simulate caller as child, trying to create PSP in parent memory
 p.psp=child;
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(PSP);p.cpu.regs[6]=u32::from(PSP+0x40);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 p.psp=PSP;
 // Untracked conventional memory (below arena, e.g. 0x0600): rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=0x0600;p.cpu.regs[6]=0x0700;
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 assert_eq!(p.ram,ram_before);
}

#[test]
fn child_psp_succeeds_in_parent_owned_interior_and_preserves_mcb_owner(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 // Allocate 0x40 paragraphs for child region
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 let block_base=p.cpu.regs[0]as u16;
 // Interior segment DX = block_base + 0x10, with 0x30 paragraphs remaining (>= 0x10)
 let interior=block_base+0x10;
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(interior);p.cpu.regs[6]=u32::from(block_base+0x40);
 assert!(service(&mut p,0x21,&mut vec![]).is_ok());
 // Active PSP switched to interior
 assert_eq!(p.psp,interior);
 // Verify child PSP fields in RAM
 let base=u32::from(interior)*16;
 assert_eq!(&p.ram.bytes()[base as usize..base as usize+2],&[0xcd,0x20]);
 assert_eq!(p.ram.load(base+2,MemWidth::Word).unwrap()as u16,block_base+0x40);
 assert_eq!(p.ram.load(base+0x16,MemWidth::Word).unwrap()as u16,PSP); // Parent link
 // MCB owner of parent block is UNCHANGED (still PSP; no new MCB at interior-1)
 assert_eq!(p.arena.block_owner(block_base),Some(PSP));
 assert_eq!(p.arena.block_owner(interior),None);
}

#[test]
fn child_psp_validates_si_against_allocation_boundary(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 // Allocate 0x20 paragraphs for child
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x20;
 service(&mut p,0x21,&mut vec![]).unwrap();
 let child=p.cpu.regs[0]as u16;
 let ram_before=p.ram.clone();
 // SI exceeds allocation boundary (child + 0x21 > child + 0x20): rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(child+0x21);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // SI is smaller than PSP size (child + 0x08 < child + 0x10): rejected
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(child+0x08);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 assert_eq!(p.ram,ram_before);
 // SI at exact boundary: succeeds
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(child+0x20);
 assert!(service(&mut p,0x21,&mut vec![]).is_ok());
 assert_eq!(p.ram.load(u32::from(child)*16+2,MemWidth::Word).unwrap()as u16,child+0x20);
}

#[test]
fn child_psp_rejects_protected_mcb_owner_and_preserves_memory_and_arena(){
 let mut p=load(&[0xcd,0x21],"test.com").unwrap();
 p.image.selectors[0]=PSP;p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x40;
 service(&mut p,0x21,&mut vec![]).unwrap();
 // Allocate 0x20 paragraphs for child
 p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x20;
 service(&mut p,0x21,&mut vec![]).unwrap();
 let child=p.cpu.regs[0]as u16;
 // Protect ONLY the 2-byte MCB header owner word at (child-1)*16 + 1
 let mcb_owner_addr=u32::from(child-1)*16+1;
 p.ram.protect(mcb_owner_addr,2).unwrap();
 let ram_before=p.ram.clone();
 // Call AH=55h
 p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(child+0x20);
 assert!(service(&mut p,0x21,&mut vec![]).is_err());
 // Preflight check must fail before any write to child PSP body or arena state change
 assert_eq!(p.ram,ram_before);
 assert_eq!(p.arena.block_owner(child),Some(PSP));
 assert_eq!(p.psp,PSP);
}

#[test]
fn cga_mode_control_port_3d8_routes_video_enable_and_blink_changes(){
 let mut p=load(&[0xee],"out.com").unwrap(); // OUT DX, AL
 p.cpu.regs[2]=0x3d8; // DX = 03D8h
 assert_eq!(p.vga.cga_mode(),0x29);
 assert_eq!(p.vga.attr[16]&8,8);

 // Disable video (bit 3 = 0, value = 0x21)
 p.cpu.regs[0]=0x21;
 assert_eq!(ports::io(&mut p).unwrap(),Some(1));
 assert_eq!(p.vga.cga_mode(),0x21);
 assert_eq!(p.vga.attr[16]&8,8);

 // Disable blink (bit 5 = 0, value = 0x09)
 p.cpu.regs[0]=0x09;
 assert_eq!(ports::io(&mut p).unwrap(),Some(1));
 assert_eq!(p.vga.cga_mode(),0x09);
 assert_eq!(p.vga.attr[16]&8,0);

 // Read from port 3D8h returns 0xFF (write-only port on bus)
 p.ram.write_bytes(0x10100,&[0xec]).unwrap(); // IN AL, DX
 assert_eq!(ports::io(&mut p).unwrap(),Some(1));
 assert_eq!(p.cpu.regs[0]&0xff,0xff);

 // Layout/width change is rejected
 p.ram.write_bytes(0x10100,&[0xee]).unwrap();
 p.cpu.regs[0]=0x28;
 assert!(ports::io(&mut p).is_err());
}


/// Drive an actual in-memory COM child through AH4Ch and a PSP INT22 callback.
/// AH55 itself is still running in the parent; no AH4B CPU frame is pushed.
fn stage_ah55_child(mut p:Loaded,borrowed:bool)->(Loaded,u16,u16,[(u32,u32);SYSTEM_RANGES]){
    let parent=p.psp;
    let system=p.image.system;
    p.dta=(parent,0x1c0);
    let handles=p.open_handles;
    p.cpu.regs[0]=0x4800;p.cpu.regs[3]=0x40;
    service(&mut p,0x21,&mut vec![]).unwrap();
    assert_eq!(p.cpu.eflags&1,0);
    let block=p.cpu.regs[0]as u16;
    let child=if borrowed{block+0x10}else{block};
    let base=u32::from(child)*16;
    p.ram.write_bytes(base+0x100,&[0xb8,0x37,0x4c,0xcd,0x21]).unwrap(); // MOV AX,4C37; INT 21
    p.ram.write_bytes(u32::from(parent)*16+0x180,&[0xb4,0x4d,0xcd,0x21,0xfa,0xf4]).unwrap(); // AH4D; CLI; HLT
    p.ram.store(0x22*4,MemWidth::Dword,(u32::from(parent)<<16)|0x100).unwrap();
    p.cpu.regs[0]=0x5500;p.cpu.regs[2]=u32::from(child);p.cpu.regs[6]=u32::from(block+0x40);
    service(&mut p,0x21,&mut vec![]).unwrap();
    assert_eq!(p.psp,child);
    assert_eq!(p.ram.load(base+0x16,MemWidth::Word).unwrap(),u32::from(parent));
    assert_eq!(p.dta,(child,0x80));
    assert_eq!(p.open_handles,handles);
    assert_eq!(p.in_memory.len(),1);
    // The AH4B parent may already grant a system fetch window covering the
    // entire DOS conventional-memory arena. AH55 then needs no extra slot.
    // Assert the child can execute, rather than demanding an unnecessary
    // mutation to the system-window table.
    let child_end=u32::from(block+0x40)*16;
    let covered=(p.image.fetch_base+p.image.start<=base && p.image.fetch_base+p.image.end>=child_end)
        ||p.image.system.iter().any(|&(a,b)|a<=base&&b>=child_end&&a<b);
    assert!(covered,"AH55 child code region is not executable");
    // Turbo sets the actual return vector once the PSP is created. Read it
    // at termination rather than using an AH55-time CPU/vector snapshot.
    p.ram.store(base+0x0a,MemWidth::Dword,(u32::from(parent)<<16)|0x180).unwrap();
    let cs=SegmentRegister::Cs.index();
    p.selectors[cs]=child;p.image.selectors[cs]=child;p.image.segments[cs]=SegmentCache::real_mode(child);
    p.cpu.eip=0x100;
    (p,child,block,system)
}

#[test]
fn ah55_child_returns_to_ide_without_an_ah4b_frame(){
    let mut p=load(&[0xfa,0xf4],"TURBO.COM").unwrap();
    p.image.selectors[SegmentRegister::Es.index()]=PSP;
    p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x60;
    service(&mut p,0x21,&mut vec![]).unwrap();
    assert_eq!(p.cpu.eflags&1,0);
    p.image.end=0x600;
    let (p,child,block,system)=stage_ah55_child(p,false);
    assert_eq!(p.arena.block_owner(block),Some(child));
    let r=run(p,40).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let p=r.loaded;
    assert_eq!(p.psp,PSP);
    assert!(p.exec.is_empty()&&p.in_memory.is_empty());
    assert_eq!(p.cpu.regs[0]&0xffff,0x0037,"AH4D must read the in-memory child's exit code");
    assert_eq!(p.return_code,0);
    assert_eq!(p.dta,(PSP,0x1c0));
    assert_eq!(p.open_handles,0x1f);
    assert_eq!(p.arena.block_owner(block),Some(0),"child MCB must be released");
    assert_eq!(p.image.system,system,"temporary child fetch range must be dropped");
    assert_eq!(p.selectors[SegmentRegister::Cs.index()],PSP);
}

#[test]
fn ah55_interior_child_returns_to_turbo_not_freecom_and_can_run_twice(){
    use std::time::{SystemTime,UNIX_EPOCH};
    let dir=std::env::temp_dir().join(format!("pil-ah55-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("TURBO.COM"),[0xfa,0xf4]).unwrap();
    // FreeCOM launches a Turbo stand-in through AH4B; the stack has a
    // waiting shell frame even while Turbo's in-memory child runs.
    let mut p=load_with_drives(&[0xfa,0xf4],"COMMAND.COM",vec![Source::Host(dir.clone())],None).unwrap();
    p.image.selectors[SegmentRegister::Es.index()]=PSP;
    p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x60;
    service(&mut p,0x21,&mut vec![]).unwrap();
    assert_eq!(p.cpu.eflags&1,0);
    let base=u32::from(PSP)*16;
    let ptr=|off:u16|(u32::from(PSP)<<16)|u32::from(off);
    p.ram.store(base+0x200,MemWidth::Word,0).unwrap();
    p.ram.store(base+0x202,MemWidth::Dword,ptr(0x80)).unwrap();
    p.ram.store(base+0x206,MemWidth::Dword,ptr(0x5c)).unwrap();
    p.ram.store(base+0x20a,MemWidth::Dword,ptr(0x6c)).unwrap();
    p.ram.write_bytes(base+0x300,b"TURBO.COM\0").unwrap();
    p.cpu.regs[0]=0x4b00;p.cpu.regs[2]=0x300;p.cpu.regs[3]=0x200;
    service(&mut p,0x21,&mut vec![]).unwrap();
    assert_eq!(p.exec.len(),1);
    let turbo=p.psp;
    assert_ne!(turbo,PSP);
    p.image.selectors[SegmentRegister::Es.index()]=turbo;
    p.cpu.regs[0]=0x4a00;p.cpu.regs[3]=0x60;
    service(&mut p,0x21,&mut vec![]).unwrap();
    assert_eq!(p.cpu.eflags&1,0);
    p.image.end=0x600;
    for _ in 0..2 {
        let (child_program,_child,block,system)=stage_ah55_child(p,true);
        assert_eq!(child_program.arena.block_owner(block),Some(turbo),"borrowed MCB ownership changed");
        assert_eq!(child_program.exec.len(),1);
        let r=run(child_program,40).unwrap();
        assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
        p=r.loaded;
        assert_eq!(p.psp,turbo,"must return to Turbo, not FreeCOM");
        assert_eq!(p.exec.len(),1,"shell's AH4B frame was popped");
        assert!(p.in_memory.is_empty());
        assert_eq!(p.cpu.regs[0]&0xffff,0x0037);
        assert_eq!(p.dta,(turbo,0x1c0));
        assert_eq!(p.open_handles,0x1f);
        assert_eq!(p.arena.block_owner(block),Some(turbo));
        assert_eq!(p.image.system,system);
        assert_eq!(p.selectors[SegmentRegister::Cs.index()],turbo);
    }
    // Only Turbo's own termination may return to the waiting shell.
    exec::terminate(&mut p,0,0).unwrap();
    assert_eq!(p.psp,PSP);
    assert!(p.exec.is_empty()&&p.in_memory.is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}
