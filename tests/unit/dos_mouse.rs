use super::*;
use crate::dos::{load,session};

fn call(p:&mut Loaded,ax:u16,bx:u16,cx:u16,dx:u16)->Result<(),String>{
    low(&mut p.cpu.regs[0],ax);low(&mut p.cpu.regs[3],bx);
    low(&mut p.cpu.regs[1],cx);low(&mut p.cpu.regs[2],dx);
    service(p)
}
fn word(r:u32)->u16{r as u16}
#[test]
fn mouse_reset_show_hide_text_position_and_limits(){
    let mut p=load(&[0xcd,0x33],"mouse.com").unwrap();p.mouse.enable();
    call(&mut p,0,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[0]),word(p.cpu.regs[3])),(0xffff,3));
    assert_eq!(p.mouse.view(),(true,true,false,320,96,639,199));
    call(&mut p,1,0,0,0).unwrap();assert!(p.mouse.view().2);
    call(&mut p,2,0,0,0).unwrap();assert!(!p.mouse.view().2);
    call(&mut p,4,0,81,19).unwrap();call(&mut p,3,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[1]),word(p.cpu.regs[2])),(80,16));
    call(&mut p,7,0,160,240).unwrap();call(&mut p,8,0,32,80).unwrap();
    call(&mut p,4,0,640,190).unwrap();call(&mut p,3,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[1]),word(p.cpu.regs[2])),(240,80));
}
#[test]
fn mouse_button_edges_and_mickey_deltas(){
    let mut p=load(&[0xcd,0x33],"mouse.com").unwrap();p.mouse.enable();
    p.ram.store(0x449,MemWidth::Byte,0x12).unwrap();
    call(&mut p,0,0,0,0).unwrap();assert_eq!(p.mouse.view().6,479);
    p.mouse.host(500,400,1,0x12);
    call(&mut p,3,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[3]),word(p.cpu.regs[1]),word(p.cpu.regs[2])),(1,500,400));
    call(&mut p,5,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[3]),word(p.cpu.regs[1]),word(p.cpu.regs[2])),(1,500,400));
    call(&mut p,5,0,0,0).unwrap();assert_eq!(word(p.cpu.regs[3]),0);
    p.mouse.host(498,398,0,0x12);
    call(&mut p,6,0,0,0).unwrap();assert_eq!(word(p.cpu.regs[3]),1);
    call(&mut p,0x0b,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[1])as i16,word(p.cpu.regs[2])as i16),(178,316));
    call(&mut p,0x0b,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[1]),word(p.cpu.regs[2])),(0,0));
    p.ram.store(0x449,MemWidth::Byte,0x13).unwrap();
    call(&mut p,3,0,0,0).unwrap();
    assert_eq!(p.mouse.view().6,199);
    assert_eq!(p.mouse.view().5,639);
}
#[test]
fn mouse_callback_registration_masks_clear_and_reset(){
    let mut p=load(&[0xcd,0x33],"mouse.com").unwrap();p.mouse.enable();
    call(&mut p,0,0,0,0).unwrap();
    call(&mut p,0x0c,0,0,0).unwrap();assert!(p.mouse.handler.is_none());
    assert!(call(&mut p,0x0c,0,0x80,0).unwrap_err().contains("reserved"));
    call(&mut p,0x0c,0,7,0x120).unwrap();
    assert_eq!(p.mouse.handler.unwrap().offset,0x120);
    p.mouse.host(160,40,1,3);
    assert_eq!(p.mouse.events.len(),1);
    call(&mut p,0x0c,0,0,0).unwrap();
    assert!(p.mouse.handler.is_none());assert!(p.mouse.events.is_empty());
    call(&mut p,0x0c,0,7,0x120).unwrap();
    call(&mut p,0,0,0,0).unwrap();
    assert!(p.mouse.handler.is_none());assert!(p.mouse.events.is_empty());
}
#[test]
fn session_installs_mouse_vector_but_standalone_has_none(){
    let p=load(&[0xcd,0x33,0xcd,0x20],"mouse.com").unwrap();
    assert!(!p.mouse.view().0);
    assert_eq!(p.ram.load(0x33*4,MemWidth::Dword).unwrap(),0);
    let mut s=session::Session::new(p);
    assert!(!s.loaded().unwrap().mouse.view().0);
    let (reply,_)=s.command(1,"mouse on").unwrap();
    assert!(reply.contains("\"mouse_present\":true"));
    assert!(s.loaded().unwrap().mouse.view().0);
    assert_eq!(s.loaded().unwrap().ram.load(0x33*4,MemWidth::Dword).unwrap(),crate::dos::vectors::entry(0x33));
    let (reply,_)=s.command(2,"run 8").unwrap();
    assert!(reply.contains("\"mouse_active\":true"));
    let (reply,_)=s.command(3,"mouse 160 40 1").unwrap();
    assert!(reply.contains("\"mouse_x\":160"));
    assert!(reply.contains("\"mouse_y\":40"));
    assert!(reply.contains("\"mouse_visible\":false"));
}

fn callback_guest()->Vec<u8>{
    // Infinite loop at 0100h, FAR handler at 0110h. The handler writes
    // callback registers through CS (DS points to the mouse driver), trashes
    // AX/BX intentionally, then returns via RETF. The data is at 0180h.
    let mut bytes=vec![0xeb,0xfe];
    bytes.resize(0x10,0x90);
    bytes.extend_from_slice(&[
        0x2e,0x89,0x06,0x80,0x01, // mov [cs:0180h],ax
        0x2e,0x89,0x1e,0x82,0x01, // mov [cs:0182h],bx
        0x2e,0x89,0x0e,0x84,0x01, // mov [cs:0184h],cx
        0x2e,0x89,0x16,0x86,0x01, // mov [cs:0186h],dx
        0x2e,0x89,0x36,0x88,0x01, // mov [cs:0188h],si
        0x2e,0x89,0x3e,0x8a,0x01, // mov [cs:018Ah],di
        0x8c,0xd8,                // mov ax,ds
        0x2e,0xa3,0x8c,0x01,      // mov [cs:018Ch],ax
        0xb8,0xef,0xbe,           // mov ax,beefh
        0x31,0xdb,                // xor bx,bx
        0xcb,                     // retf
    ]);
    bytes.resize(0x90,0);
    bytes
}
fn callback_setup()->Loaded{
    let mut p=load(&callback_guest(),"MOUSECALL.COM").unwrap();
    p.mouse.enable();
    p.ram.store(0x449,MemWidth::Byte,0x12).unwrap();
    call(&mut p,0,0,0,0).unwrap();
    let es=SegmentRegister::Es.index();
    p.selectors[es]=PSP;p.image.selectors[es]=PSP;
    p.image.segments[es]=SegmentCache::real_mode(PSP);
    call(&mut p,0x0c,0,0x7f,0x110).unwrap();
    p
}
fn mouse_word(p:&Loaded,offset:u32)->u16{
    p.ram.load(u32::from(PSP)*16+offset,MemWidth::Word).unwrap()as u16
}
#[test]
fn callback_far_return_preserves_interrupted_cpu_and_delivers_registers(){
    let mut p=callback_setup();
    let original=[0x1122_3344,0xaabb_ccdd,0x1234_5678,0x5566_7788,0xfffe,0x1000_2000,0x2233_4455,0x6677_8899];
    p.cpu.regs=original;
    let saved=p.cpu.clone();
    let selectors=p.selectors;
    p.mouse.host(160,80,1,0x12);
    let result=crate::dos::run(p,120).unwrap();
    let p=result.loaded;
    assert_eq!(p.cpu,saved,"callback must restore caller registers/flags/IP");
    assert_eq!(p.selectors,selectors,"callback must restore DS/CS/ES");
    assert!(p.mouse.frame.is_none());
    assert_eq!((mouse_word(&p,0x180),mouse_word(&p,0x182)),(3,1));
    assert_eq!((mouse_word(&p,0x184),mouse_word(&p,0x186)),(160,80));
    assert_eq!((mouse_word(&p,0x188)as i16,mouse_word(&p,0x18a)as i16),(-160,-320));
    assert_eq!(mouse_word(&p,0x18c),DOSDATA);
}
#[test]
fn callback_button_edges_are_not_lost_and_can_cross_run_slices(){
    let mut p=callback_setup();
    p.mouse.host(320,240,1,0x12);
    p.mouse.host(320,240,0,0x12);
    assert_eq!(p.mouse.events.len(),2);
    let first=crate::dos::run(p,1).unwrap().loaded;
    assert!(first.mouse.frame.is_some(),"callback entry spans the slice");
    let second=crate::dos::run(first,70).unwrap().loaded;
    assert_eq!(mouse_word(&second,0x180),4,"release event must be delivered last");
    assert_eq!(mouse_word(&second,0x182),0);
    assert!(second.mouse.events.is_empty());
    assert!(second.mouse.frame.is_none());
}
#[test]
fn callback_is_deferred_when_interrupts_are_disabled(){
    let mut p=callback_setup();
    p.cpu.eflags&=!0x200;
    p.mouse.host(320,240,1,0x12);
    let mut p=crate::dos::run(p,12).unwrap().loaded;
    assert_eq!(mouse_word(&p,0x180),0);
    assert_eq!(p.mouse.events.len(),1);
    p.cpu.eflags|=0x200;
    let p=crate::dos::run(p,60).unwrap().loaded;
    assert_eq!(mouse_word(&p,0x180),2);
    assert!(p.mouse.events.is_empty());
}
#[test]
fn invalid_mouse_callback_target_stops_without_changing_guest_stack(){
    let mut p=callback_setup();
    p.mouse.handler=Some(Handler{mask:2,segment:0x9000,offset:0x0100});
    p.mouse.host(320,240,1,0x12);
    let sp=p.cpu.regs[4];let original=p.ram.load(PSP as u32*16+sp-4,MemWidth::Dword).unwrap();
    let result=crate::dos::run(p,20).unwrap();
    assert!(matches!(&result.stop,crate::dos::Stop::Service(s) if s.contains("outside executable DOS memory")));
    assert_eq!(result.loaded.cpu.regs[4],sp);
    assert_eq!(result.loaded.ram.load(PSP as u32*16+sp-4,MemWidth::Dword).unwrap(),original);
}
