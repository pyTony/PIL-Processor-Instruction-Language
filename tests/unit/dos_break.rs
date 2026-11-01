use super::*;
const CTRL_C:u16=0x2e03;
fn vector23(p:&Loaded)->u32{p.ram.load(0x23*4,MemWidth::Dword).unwrap()}
fn ring_empty(p:&Loaded)->bool{p.ram.load(0x41a,MemWidth::Word).unwrap()==p.ram.load(0x41c,MemWidth::Word).unwrap()}
/// MOV DX,0111; MOV AX,2523; INT 21 (INT 23h -> handler); STC; MOV AH,1; INT 21;
/// MOV AH,4C; INT 21. Handler: MOV [200],AX; PUSHF; POP [204]; INC BYTE [202]; then `ret`.
fn own_handler(ret:&[u8])->Vec<u8>{
    let mut code=vec![0xba,0x11,0x01,0xb8,0x23,0x25,0xcd,0x21,0xf9,0xb4,0x01,0xcd,0x21,0xb4,0x4c,0xcd,0x21,
        0xa3,0x00,0x02,0x9c,0x8f,0x06,0x04,0x02,0xfe,0x06,0x02,0x02];
    code.extend(ret);code
}
fn word(p:&Loaded,offset:u32)->u32{p.ram.load(u32::from(PSP)*16+offset,MemWidth::Word).unwrap()}
#[test]
fn default_handler_echoes_and_ends_the_program_with_ctrl_c(){
    // MOV AH,1; INT 21; MOV AX,4C07; INT 21
    let mut p=load(&[0xb4,0x01,0xcd,0x21,0xb8,0x07,0x4c,0xcd,0x21],"ask.com").unwrap();
    assert_eq!(vector23(&p),0x0070_0074);assert_eq!(word(&p,0x0e),0x0074);assert_eq!(word(&p,0x10),0x0070);
    p.enqueue_key(CTRL_C).unwrap();p.enqueue_key(0x1e61).unwrap();
    let r=run(p,20).unwrap();
    assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"^C\r\n");
    // The rest of the type-ahead was flushed with the ^C; the screen shows it.
    assert!(ring_empty(&r.loaded));assert!(r.loaded.breaks.is_empty());
    let row:Vec<u8>=(0..2).map(|c|r.loaded.ram.load(0xb8000+c*2,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(row,b"^C");
    assert_eq!(r.loaded.ram.load(0x450,MemWidth::Word).unwrap(),0x0100);
    assert_eq!(r.instructions.get("DOS ^C -> INT23"),Some(&1));assert_eq!(r.instructions.get("DOS INT23 return (abort)"),Some(&1));
    // STC; RETF ran in Dine's data segment and left the flags word on the stack.
    assert_eq!(r.loaded.cpu.regs[4]&0xffff,0xfffc);
}
#[test]
fn iret_and_retf2_handlers_resume_the_call_and_plain_retf_aborts(){
    for(ret,resumes)in [(&[0xcf][..],true),(&[0xca,0x02,0x00][..],true),(&[0xcb][..],false)]{
        let mut p=load(&own_handler(ret),"own.com").unwrap();
        p.enqueue_key(CTRL_C).unwrap();
        // The type-ahead after ^C is flushed; this key comes after the break.
        let mut r=run(p,40).unwrap();assert_eq!(r.stop,if resumes{Stop::WaitingForInput}else{Stop::Exit(0)},"{ret:?}");
        assert_eq!(r.output,b"^C\r\n");assert_eq!(word(&r.loaded,0x202)&255,1);
        // The handler saw the INT 21h registers with CF clear, and IF/TF clear.
        assert_eq!(word(&r.loaded,0x200),0x0123);assert_eq!(word(&r.loaded,0x204)&0x301,0);
        if !resumes{assert_eq!(r.loaded.cpu.regs[4]&0xffff,0xfffc);continue;}
        // Back at the INT 21h with SP as it was; the call runs again and reads.
        assert_eq!((r.loaded.selectors[1],r.loaded.cpu.eip,r.loaded.cpu.regs[4]&0xffff),(PSP,0x10b,0xfffe));
        assert_eq!(r.loaded.cpu.regs[0]&0xffff,0x0123);assert!(r.loaded.breaks.is_empty());
        r.loaded.enqueue_key(0x2d78).unwrap();
        let r=run(r.loaded,10).unwrap();assert_eq!(r.stop,Stop::Exit(b'x'));assert_eq!(r.output,b"x");
    }
}
#[test]
fn line_input_starts_again_after_a_resumed_break(){
    // MOV DX,0114; MOV AX,2523; INT 21; MOV DX,0200; MOV AH,0A; INT 21; MOV AX,4C00; INT 21; IRET
    let code=[0xba,0x14,0x01,0xb8,0x23,0x25,0xcd,0x21,0xba,0x00,0x02,0xb4,0x0a,0xcd,0x21,0xb8,0x00,0x4c,0xcd,0x21,0xcf];
    let mut p=load(&code,"line.com").unwrap();let base=u32::from(PSP)*16+0x200;p.ram.store(base,MemWidth::Byte,10).unwrap();
    p.enqueue_key(0x1e61).unwrap();p.enqueue_key(CTRL_C).unwrap();
    let mut r=run(p,40).unwrap();assert_eq!(r.stop,Stop::WaitingForInput);assert_eq!(r.output,b"a^C\r\n");assert_eq!(r.loaded.line,Some((vec![],0,true)));
    r.loaded.enqueue_key(0x3062).unwrap();r.loaded.enqueue_key(0x1c0d).unwrap();
    let r=run(r.loaded,20).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"b\r");
    let bytes:Vec<u8>=(0..4).map(|i|r.loaded.ram.load(base+i,MemWidth::Byte).unwrap()as u8).collect();assert_eq!(bytes,[10,1,b'b',13]);
}
#[test]
fn break_on_checks_every_call_and_break_off_only_character_calls(){
    // MOV AX,3301; MOV DL,on; INT 21; MOV AH,30; INT 21; MOV AX,4C07; INT 21
    for on in [0u8,1]{
        let mut p=load(&[0xb8,0x01,0x33,0xb2,on,0xcd,0x21,0xb4,0x30,0xcd,0x21,0xb8,0x07,0x4c,0xcd,0x21],"brk.com").unwrap();
        p.enqueue_key(CTRL_C).unwrap();let r=run(p,20).unwrap();
        if on==1{assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"^C\r\n");assert!(ring_empty(&r.loaded));}
        else{assert_eq!(r.stop,Stop::Exit(7));assert!(r.output.is_empty());assert!(!ring_empty(&r.loaded));}
    }
}
#[test]
fn ctrl_break_flushes_sets_the_bios_flag_and_breaks_at_the_next_check(){
    let mut p=load(&[0xb4,0x01,0xcd,0x21,0xb8,0x07,0x4c,0xcd,0x21],"ask.com").unwrap();
    p.enqueue_key(0x1e61).unwrap();p.ctrl_break().unwrap();
    assert_eq!(p.ram.load(0x471,MemWidth::Byte).unwrap()&0x80,0x80);
    let head=p.ram.load(0x41a,MemWidth::Word).unwrap();assert_eq!(p.ram.load(0x400+head,MemWidth::Word).unwrap(),0);
    assert_eq!(p.ram.load(0x41c,MemWidth::Word).unwrap(),head+2);
    let r=run(p,20).unwrap();assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.output,b"^C\r\n");
    assert_eq!(r.loaded.ram.load(0x471,MemWidth::Byte).unwrap()&0x80,0);assert!(ring_empty(&r.loaded));
    // A program's own INT 1Bh handler is not called, so Ctrl+Break is refused.
    let mut p=load(&[0xf4],"x.com").unwrap();p.ram.store(0x1b*4,MemWidth::Dword,0x1000_0200).unwrap();
    let old=p.ram.clone();assert!(p.ctrl_break().is_err());assert_eq!(p.ram,old);
}
#[test]
fn a_break_that_cannot_echo_or_push_changes_nothing(){
    let code=[0xb4,0x01,0xcd,0x21,0xb8,0x07,0x4c,0xcd,0x21];
    // Output full: ^C needs four bytes.
    let mut p=load(&code,"ask.com").unwrap();p.enqueue_key(CTRL_C).unwrap();
    let mut out=crate::host_io::CaptureOutput::new(3);
    let r=run_with_output(p,20,&mut out).unwrap();assert!(matches!(&r.stop,Stop::Service(m) if m.contains("^C")),"{:?}",r.stop);
    assert!(r.output.is_empty());assert!(!ring_empty(&r.loaded));assert_eq!((r.loaded.cpu.eip,r.loaded.cpu.regs[4]&0xffff),(0x102,0xfffe));
    // A read-only stack word: no echo, no flush, no push.
    let mut p=load(&code,"ask.com").unwrap();p.enqueue_key(CTRL_C).unwrap();
    p.ram.protect(u32::from(PSP)*16+0xfffa,2).unwrap();let screen=p.ram.load(0xb8000,MemWidth::Word).unwrap();
    let r=run(p,20).unwrap();assert!(matches!(r.stop,Stop::Service(_)),"{:?}",r.stop);
    assert!(r.output.is_empty());assert!(!ring_empty(&r.loaded));assert_eq!(r.loaded.ram.load(0xb8000,MemWidth::Word).unwrap(),screen);
    assert_eq!((r.loaded.selectors[1],r.loaded.cpu.eip,r.loaded.cpu.regs[4]&0xffff),(PSP,0x102,0xfffe));assert!(r.loaded.breaks.is_empty());
}
#[test]
fn ctrl_c_in_a_child_ends_it_with_type_1_and_vectors_come_back(){
    use std::time::{SystemTime,UNIX_EPOCH};
    let dir=std::env::temp_dir().join(format!("pil-break-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    // CHILD.COM reads a key (the ^C) and would exit with 5.
    std::fs::write(dir.join("child.com"),[0xb4,0x01,0xcd,0x21,0xb8,0x05,0x4c,0xcd,0x21]).unwrap();
    // HOOK.COM points INT 23h at itself and exits normally.
    std::fs::write(dir.join("hook.com"),[0xba,0x00,0x02,0xb8,0x23,0x25,0xcd,0x21,0xb8,0x00,0x4c,0xcd,0x21]).unwrap();
    for(name,code)in [(&b"CHILD.COM\0"[..],0x0100u32),(b"HOOK.COM\0",0)]{
        // Shrink to 100h paragraphs, EXEC the child, AH4D into SI, CLI; HLT.
        let mut parent=vec![0xbb,0x00,0x01,0xb4,0x4a,0xcd,0x21,0x8c,0x0e,0x44,0x01,0x8c,0x0e,0x48,0x01,0x8c,0x0e,0x4c,0x01,
            0xba,0x60,0x01,0xbb,0x40,0x01,0xb8,0x00,0x4b,0xcd,0x21,0xb4,0x4d,0xcd,0x21,0x89,0xc6,0xfa,0xf4];
        parent.resize(0x40,0);parent.extend([0,0,0x50,0x01,0,0,0x50,0x01,0,0,0x50,0x01,0,0]);
        parent.resize(0x50,0);parent.extend([0,0x0d]);parent.resize(0x60,0);parent.extend(name);
        let mut p=load_with_drives(&parent,"PARENT.COM",vec![Source::Host(dir.clone())],None).unwrap();
        p.enqueue_key(CTRL_C).unwrap();
        let r=run(p,200).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
        assert_eq!(r.loaded.cpu.regs[6]&0xffff,code);assert_eq!((r.loaded.psp,r.loaded.exec.len()),(PSP,0));
        assert!(r.loaded.breaks.is_empty());assert_eq!(vector23(&r.loaded),0x0070_0074);
        assert_eq!(r.output,if code==0x100{b"^C\r\n".to_vec()}else{vec![]});
    }
    std::fs::remove_dir_all(dir).unwrap();
}
