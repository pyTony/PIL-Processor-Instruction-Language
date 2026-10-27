use super::*;
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
    let b=[0xb4,0x30,0xcd,0x21];let r=run(load(&b,"service.com").unwrap(),10).unwrap();assert!(matches!(r.stop,Stop::Service(_)));assert_eq!(r.loaded.cpu.eip,0x102);assert_eq!(r.completed,1);assert_eq!(r.dos_calls,0);
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
    let text=report(&paths,100).unwrap();assert!(text.contains("SUMMARY imported=4/4 exited=1"));assert!(text.contains("2 opcode 62"));assert!(text.contains("Hello, DOS!\\r\\n"));assert!(text.contains("Service("));assert!(report(&[],100).is_err());assert!(report(&paths,0).is_err());std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mz_negative_relative_segments_wrap_before_entry_and_stack_validation(){
    let mut bytes=hello("exe").unwrap();put_word(&mut bytes,22,0xfff0);put_word(&mut bytes,20,0x100);put_word(&mut bytes,14,0xfff0);
    // With CS=DS=PSP, hello's data offset10C still names the original string.
    let p=load(&bytes,"wrapped.exe").unwrap();assert_eq!(p.selectors[1],PSP);assert_eq!(p.selectors[2],PSP);assert_eq!(p.image.start,0x100);assert_eq!(p.cpu.eip,0x100);
    let r=run(p,100).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"Hello, DOS!\r\n");
    put_word(&mut bytes,20,0xff);assert!(load(&bytes,"outside.exe").is_err());
}
