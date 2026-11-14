use super::*;
fn session(bytes:&[u8])->Session{Session::new(dos::load(bytes,"FIXTURE.COM").unwrap())}
fn echo()->Session{session(&[0xb4,1,0xcd,0x21,0x3c,13,0x75,0xf8,0xb8,7,0x4c,0xcd,0x21])}
#[test]
fn typed_cp437_and_crlf_reach_dos_and_preserve_exit_status(){
    let mut s=echo();let mut output=Vec::new();
    assert_eq!(stream(&mut s,&mut io::Cursor::new("äbc\r\n"),&mut output).unwrap(),7);
    assert_eq!(output,"äbc\r".as_bytes());
    assert_eq!(&s.screen().unwrap().0[..6],&[0x84,7,b'b',7,b'c',7]);
    assert_eq!(line_keys("\r\n").unwrap().len(),1);
}
#[test]
fn eof_does_not_synthesize_enter(){
    let mut s=echo();
    assert!(stream(&mut s,&mut io::Cursor::new(""),&mut Vec::new()).unwrap_err().contains("EOF"));
    assert_eq!(s.status().0,"waiting_input");
    assert_eq!(s.machine().unwrap().cpu.eip,0x102);
}
#[test]
fn bounded_lines_and_unmappable_input_fail_explicitly(){
    for text in ["a".repeat(4097),"🙂\n".into()]{
        assert!(stream(&mut echo(),&mut io::Cursor::new(text),&mut Vec::new()).is_err());
    }
}
#[test]
fn a_full_bios_queue_retains_the_rest_of_the_host_line(){
    let mut s=echo();let text=format!("{}\n","a".repeat(80));let mut output=Vec::new();
    assert_eq!(stream(&mut s,&mut io::Cursor::new(text),&mut output).unwrap(),7);
    assert_eq!(output,[vec![b'a';80],vec![13]].concat());
}
#[test]
fn split_ansi_is_handled_by_the_existing_guest_driver(){
    let mut bytes=Vec::new();
    for b in b"\x1b[31mX"{bytes.extend_from_slice(&[0xb2,*b,0xb4,2,0xcd,0x21]);}
    bytes.extend_from_slice(&[0xb8,0,0x4c,0xcd,0x21]);
    let mut s=session(&bytes);s.command(0,"ansi on").unwrap();let mut output=Vec::new();
    stream(&mut s,&mut io::Cursor::new(""),&mut output).unwrap();
    assert_eq!(&s.screen().unwrap().0[..2],&[b'X',4]);
    assert_eq!(output,b"\\x1b[31mX");
}
#[test]
fn direct_video_memory_writes_stay_in_the_shared_screen(){
    let mut s=session(&[0xb8,0,0xb8,0x8e,0xc0,0x26,0xc7,6,0,0,0xdb,0x1e,0xb8,0,0x4c,0xcd,0x21]);
    let mut output=Vec::new();stream(&mut s,&mut io::Cursor::new(""),&mut output).unwrap();
    assert!(output.is_empty());assert_eq!(&s.screen().unwrap().0[..2],&[0xdb,0x1e]);
}
#[test]
fn backend_sessions_use_the_same_io_and_screen(){
    use std::{cell::Cell,rc::Rc};
    struct Backend(Rc<Cell<usize>>);
    impl dos::InstructionBackend for Backend{
        fn execute(&mut self,_:&crate::guest::Image,d:&Rc<crate::guest::Decoded>,cpu:&mut crate::Cpu,ram:&mut crate::Ram,m:&mut crate::address::SegmentState)->Option<Result<(),crate::flow::ExecError>>{
            if let crate::guest::Operation::Pil{program,dst,src,flow,address}=&d.operation{
                self.0.set(self.0.get()+1);
                return Some(crate::execute_machine(program,cpu,*dst,*src,*flow,ram,*address,m));
            }None
        }
    }
    let p=dos::load(&dos::hello("com").unwrap(),"FIXTURE.COM").unwrap();let count=Rc::new(Cell::new(0));
    let mut a=Session::new(p.clone());let mut b=Session::with_backend(p,Box::new(Backend(count.clone())));
    let(mut x,mut y)=(Vec::new(),Vec::new());
    assert_eq!(stream(&mut a,&mut io::Cursor::new(""),&mut x).unwrap(),stream(&mut b,&mut io::Cursor::new(""),&mut y).unwrap());
    assert!(count.get()>0);assert_eq!(x,y);assert_eq!(a.screen().unwrap(),b.screen().unwrap());
}
#[test]
fn transcript_never_operates_the_host_terminal(){
    assert_eq!(transcript(b"\x1b]title\x07\r\n\x84"),"\\x1b]title\\x07\r\nä");
}
