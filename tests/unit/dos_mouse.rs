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
    assert_eq!((word(p.cpu.regs[1])as i16,word(p.cpu.regs[2])as i16),(179,318));
    call(&mut p,0x0b,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[1]),word(p.cpu.regs[2])),(0,0));
    p.ram.store(0x449,MemWidth::Byte,0x13).unwrap();
    call(&mut p,3,0,0,0).unwrap();
    assert_eq!(p.mouse.view().6,199);
    assert_eq!(p.mouse.view().5,639);
}
#[test]
fn mouse_callback_registration_fails_clearly_not_silently(){
    let mut p=load(&[0xcd,0x33],"mouse.com").unwrap();p.mouse.enable();
    call(&mut p,0,0,0,0).unwrap();
    call(&mut p,0x0c,0,0,0).unwrap();
    assert!(call(&mut p,0x0c,0,1,0).unwrap_err().contains("callbacks"));
}
#[test]
fn session_installs_mouse_vector_but_standalone_has_none(){
    let p=load(&[0xcd,0x33,0xcd,0x20],"mouse.com").unwrap();
    assert!(!p.mouse.view().0);
    assert_eq!(p.ram.load(0x33*4,MemWidth::Dword).unwrap(),0);
    let mut s=session::Session::new(p);
    assert!(s.loaded().unwrap().mouse.view().0);
    assert_eq!(s.loaded().unwrap().ram.load(0x33*4,MemWidth::Dword).unwrap(),crate::dos::vectors::entry(0x33));
    let (reply,_)=s.command(1,"run 8").unwrap();
    assert!(reply.contains("\"mouse_active\":true"));
    let (reply,_)=s.command(2,"mouse 160 40 1").unwrap();
    assert!(reply.contains("\"mouse_x\":160"));
    assert!(reply.contains("\"mouse_y\":40"));
    assert!(reply.contains("\"mouse_visible\":false"));
}
