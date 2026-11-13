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
    let mut p=load(&[0xcd,0x33],"mouse.com").unwrap();p.mouse.enable(0);
    call(&mut p,0,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[0]),word(p.cpu.regs[3])),(0xffff,3));
    let v=p.mouse.view();assert_eq!((v.present,v.active,v.visible,v.x,v.y,v.max_x,v.max_y),(true,true,false,320,96,639,199));
    call(&mut p,1,0,0,0).unwrap();assert!(p.mouse.view().visible);
    call(&mut p,2,0,0,0).unwrap();assert!(!p.mouse.view().visible);
    call(&mut p,4,0,81,19).unwrap();call(&mut p,3,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[1]),word(p.cpu.regs[2])),(80,16));
    call(&mut p,7,0,160,240).unwrap();call(&mut p,8,0,32,80).unwrap();
    call(&mut p,4,0,640,190).unwrap();call(&mut p,3,0,0,0).unwrap();
    assert_eq!((word(p.cpu.regs[1]),word(p.cpu.regs[2])),(240,80));
}
#[test]
fn mouse_button_edges_and_mickey_deltas(){
    let mut p=load(&[0xcd,0x33],"mouse.com").unwrap();p.mouse.enable(0);
    p.ram.store(0x449,MemWidth::Byte,0x12).unwrap();
    call(&mut p,0,0,0,0).unwrap();assert_eq!(p.mouse.view().max_y,479);
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
    assert_eq!(p.mouse.view().max_y,199);
    assert_eq!(p.mouse.view().max_x,639);
}
#[test]
fn mouse_callback_registration_masks_clear_and_reset(){
    let mut p=load(&[0xcd,0x33],"mouse.com").unwrap();p.mouse.enable(0);
    call(&mut p,0,0,0,0).unwrap();
    call(&mut p,0x0c,0,0,0).unwrap();assert!(!p.mouse.handler.active());
    // Bits 7-15 are unused (RBIL); CuteMouse keeps the low byte, so a
    // program passing CX=FFFFh registers for every event.
    call(&mut p,0x0c,0,0xffff,0x120).unwrap();
    assert_eq!((p.mouse.handler.mask,p.mouse.handler.offset),(0xff,0x120));
    call(&mut p,0x0c,0,0x80,0x120).unwrap();assert!(!p.mouse.handler.active());
    call(&mut p,0x0c,0,7,0x120).unwrap();
    assert_eq!(p.mouse.handler.offset,0x120);
    p.mouse.host(160,40,1,3);
    assert_eq!(p.mouse.events.len(),1);
    call(&mut p,0x0c,0,0,0).unwrap();
    assert!(!p.mouse.handler.active());assert!(p.mouse.events.is_empty());
    call(&mut p,0x0c,0,7,0x120).unwrap();
    call(&mut p,0,0,0,0).unwrap();
    assert!(!p.mouse.handler.active());assert!(p.mouse.events.is_empty());
}
#[test]
fn session_installs_mouse_vector_but_standalone_has_none(){
    let p=load(&[0xcd,0x33,0xcd,0x20],"mouse.com").unwrap();
    assert!(!p.mouse.view().present);
    assert_eq!(p.ram.load(0x33*4,MemWidth::Dword).unwrap(),0);
    let mut s=session::Session::new(p);
    assert!(!s.loaded().unwrap().mouse.view().present);
    let (reply,_)=s.command(1,"mouse on").unwrap();
    assert!(reply.contains("\"mouse_present\":true"));
    assert!(s.loaded().unwrap().mouse.view().present);
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
    p.mouse.enable(0);
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
    p.mouse.handler=Handler{mask:2,segment:0xb800,offset:0x0100};
    p.mouse.host(320,240,1,0x12);
    let sp=p.cpu.regs[4];let original=p.ram.load(PSP as u32*16+sp-4,MemWidth::Dword).unwrap();
    let result=crate::dos::run(p,20).unwrap();
    assert!(matches!(&result.stop,crate::dos::Stop::Service(s) if s.contains("outside executable DOS memory")));
    assert_eq!(result.loaded.cpu.regs[4],sp);
    assert_eq!(result.loaded.ram.load(PSP as u32*16+sp-4,MemWidth::Dword).unwrap(),original);
}

#[test]
fn guest_can_install_callback_through_int33_and_receive_host_session_event(){
    // Set ES=CS; reset mouse; register ES:0120 for left-button press,
    // then spin. No direct Rust invocation of the mouse service is needed.
    let mut code=vec![
        0xb8,0,0,0xcd,0x33,      // mov ax,0 / int 33
        0x0e,0x07,             // push cs / pop es
        0xba,0x20,0x01,        // mov dx,0120
        0xb9,0x02,0x00,        // mov cx,2 (left press only)
        0xb8,0x0c,0x00,        // mov ax,000c
        0xcd,0x33,            // register callback
        0xeb,0xfe,            // spin
    ];
    code.resize(0x20,0x90);
    code.extend_from_slice(&[0x2e,0xff,0x06,0x80,0x01,0xcb]); // inc word [cs:0180]; retf
    code.resize(0x82,0);
    let mut session=session::Session::new(load(&code,"mouseapp.com").unwrap());
    let (reply,_)=session.command(1,"mouse on").unwrap();
    assert!(reply.contains("\"mouse_present\":true"));
    let (reply,_)=session.command(2,"run 8").unwrap();
    assert!(reply.contains("\"status\":\"running\""));
    assert_eq!(session.loaded().unwrap().mouse.handler.offset,0x120);
    session.command(3,"mouse 320 96 1").unwrap();
    session.command(4,"run 48").unwrap();
    let p=session.loaded().unwrap();
    assert_eq!(mouse_word(p,0x180),1);
    assert!(p.mouse.frame.is_none());
}
#[test]
fn callback_iret_is_rejected_as_an_unbalanced_far_return(){
    let mut p=callback_setup();
    p.ram.store(u32::from(PSP)*16+0x110,MemWidth::Byte,0xcf).unwrap(); // iret, not retf
    p.mouse.host(320,240,1,0x12);
    let result=crate::dos::run(p,20).unwrap();
    assert!(matches!(&result.stop,crate::dos::Stop::Service(s) if s.contains("RETF required")),
        "unexpected result: {:?}",result.stop);
}

fn set_es(p:&mut Loaded,es:u16){
    let i=SegmentRegister::Es.index();
    p.selectors[i]=es;p.image.selectors[i]=es;p.image.segments[i]=SegmentCache::real_mode(es);
}
fn regs(p:&Loaded)->(u16,u16,u16,u16){(word(p.cpu.regs[0]),word(p.cpu.regs[3]),word(p.cpu.regs[1]),word(p.cpu.regs[2]))}
fn ready_mouse(mode:u8)->Loaded{
    let mut p=load(&[0xcd,0x33],"mouse.com").unwrap();p.mouse.enable(0x1234_5678);
    p.ram.store(0x449,MemWidth::Byte,u32::from(mode)).unwrap();
    call(&mut p,0,0,0,0).unwrap();p
}
#[test]
fn callback_ax_holds_only_masked_conditions_and_si_di_the_motion_counters(){
    let mut p=callback_setup();
    // Left press only: the move that precedes it raises no event, but its
    // mickeys are in the counters the press reports (as AX=000Bh reads them).
    call(&mut p,0x0c,0,0x02,0x110).unwrap();
    p.mouse.host(160,80,0,0x12);assert!(p.mouse.events.is_empty());
    p.mouse.host(168,80,1,0x12);
    let mid=crate::dos::run(p,1).unwrap().loaded;
    assert!(mid.mouse.frame.is_some());
    assert_eq!(mid.cpu.eflags&0x300,0x200,"IF stays set (CuteMouse STI), TF clear");
    let p=crate::dos::run(mid,60).unwrap().loaded;
    assert_eq!(mouse_word(&p,0x180),2,"move bit is not in the call mask");
    assert_eq!((mouse_word(&p,0x184),mouse_word(&p,0x186)),(168,80));
    assert_eq!((mouse_word(&p,0x188)as i16,mouse_word(&p,0x18a)as i16),(-152,-320));
}
#[test]
fn turbo_vision_style_full_mask_through_int33_receives_events(){
    let mut code=vec![
        0xb8,0,0,0xcd,0x33,0x0e,0x07,0xba,0x20,0x01,
        0xb9,0xff,0xff,             // mov cx,ffff
        0xb8,0x0c,0x00,0xcd,0x33,0xeb,0xfe,
    ];
    code.resize(0x20,0x90);
    code.extend_from_slice(&[0x2e,0xa3,0x80,0x01,0xcb]); // mov [cs:0180],ax; retf
    code.resize(0x82,0);
    let mut s=session::Session::new(load(&code,"tv.com").unwrap());
    s.command(1,"mouse on").unwrap();
    let (reply,_)=s.command(2,"run 8").unwrap();
    assert!(reply.contains("\"status\":\"running\""),"{reply}");
    s.command(3,"mouse 320 96 2").unwrap();
    s.command(4,"run 40").unwrap();
    assert_eq!(mouse_word(s.loaded().unwrap(),0x180),0x08,"right press; position unchanged");
}
#[test]
fn software_text_cursor_masks_are_drawn_on_the_snapshot_not_in_b800(){
    let mut p=ready_mouse(3);
    p.ram.store(0xb8000+(12*80+40)*2,MemWidth::Word,0x0741).unwrap();
    call(&mut p,1,0,0,0).unwrap();
    let s=session::Session::new(p);
    let (cells,..)=s.screen().unwrap();
    assert_eq!(&cells[(12*80+40)*2..][..2],&[0x41,0x70],"default 77FFh/7700h swaps the colours");
    let mut p=s.loaded().unwrap().clone();
    assert_eq!(p.ram.load(0xb8000+(12*80+40)*2,MemWidth::Word).unwrap(),0x0741,"guest memory unchanged");
    call(&mut p,0x0a,0,0x00ff,0x0f18).unwrap();
    let s=session::Session::new(p);
    assert_eq!(&s.screen().unwrap().0[(12*80+40)*2..][..2],&[0x59,0x0f]);
    let (reply,_)=session::Session::new(s.loaded().unwrap().clone()).command(1,"snapshot").unwrap();
    assert!(reply.contains("\"mouse_drawn\":true"),"{reply}");
    let mut p=s.loaded().unwrap().clone();
    call(&mut p,2,0,0,0).unwrap();
    assert_eq!(&session::Session::new(p.clone()).screen().unwrap().0[(12*80+40)*2..][..2],&[0x41,0x07],"hidden");
    let before=p.mouse.clone();
    assert!(call(&mut p,0x0a,1,6,7).unwrap_err().contains("hardware text cursor"));
    assert_eq!(p.mouse,before);
}
#[test]
fn graphics_cursor_shape_is_read_from_es_dx_and_reported(){
    let mut p=ready_mouse(0x12);
    let base=u32::from(PSP)*16+0x200;
    for i in 0..32u32{p.ram.store(base+2*i,MemWidth::Word,0x1000+i).unwrap();}
    set_es(&mut p,PSP);
    call(&mut p,9,0xfffe,3,0x200).unwrap();
    let (hx,hy,rows)=p.mouse.shape().unwrap();
    assert_eq!((hx,hy,rows[0],rows[31]),(-2,3,0x1000,0x101f));
    let (reply,_)=session::Session::new(p).command(1,"snapshot").unwrap();
    assert!(reply.contains("\"mouse_hotspot\":[-2,3],\"mouse_shape\":\"00100110"),"{reply}");
}
#[test]
fn save_and_restore_driver_state_round_trip_and_reject_foreign_buffers(){
    let mut p=ready_mouse(0x12);
    set_es(&mut p,PSP);
    call(&mut p,0x15,0,0,0).unwrap();assert_eq!(regs(&p).1,158);
    call(&mut p,0x0f,0,4,4).unwrap();call(&mut p,7,0,100,300).unwrap();
    call(&mut p,0x0a,0,0x1234,0x5678).unwrap();call(&mut p,1,0,0,0).unwrap();
    call(&mut p,0x0c,0,0x06,0x300).unwrap();call(&mut p,4,0,200,100).unwrap();
    p.mouse.host(250,100,1,0x12);
    let saved=p.mouse.clone();
    // Turbo Pascal's IDE calls 16h/17h with garbage in BX: it is ignored.
    call(&mut p,0x16,0xdead,0,0x400).unwrap();
    call(&mut p,0,0,0,0).unwrap();
    assert_ne!(p.mouse.handler,saved.handler);
    call(&mut p,0x17,0,0,0x400).unwrap();
    let mut expect=saved.clone();expect.events.clear();
    assert_eq!(p.mouse,expect);
    // A buffer this driver did not write is refused, with no change.
    p.ram.store(u32::from(PSP)*16+0x500,MemWidth::Word,0).unwrap();
    let before=p.mouse.clone();
    assert!(call(&mut p,0x17,0,0,0x500).unwrap_err().contains("no saved mouse state"));
    assert_eq!(p.mouse,before);
}
#[test]
fn save_state_into_a_buffer_that_runs_into_rom_writes_nothing(){
    let mut p=ready_mouse(3);
    set_es(&mut p,0xbff0);
    let first=0xbff0*16+0xa0;
    let before=p.ram.load(first,MemWidth::Dword).unwrap();
    assert!(call(&mut p,0x16,0,0,0xa0).unwrap_err().contains("buffer"));
    assert_eq!(p.ram.load(first,MemWidth::Dword).unwrap(),before);
}
#[test]
fn update_region_hides_the_cursor_inside_until_show(){
    let mut p=ready_mouse(0x12);
    call(&mut p,1,0,0,0).unwrap();assert!(p.mouse.view().visible);
    p.mouse.host(100,100,0,0x12);
    // CX,DX upper left and SI,DI lower right, given in either order.
    low(&mut p.cpu.regs[6],50);low(&mut p.cpu.regs[7],50);
    call(&mut p,0x10,0,200,200).unwrap();
    assert!(!p.mouse.view().visible);
    p.mouse.host(300,300,0,0x12);assert!(p.mouse.view().visible);
    p.mouse.host(60,60,0,0x12);assert!(!p.mouse.view().visible);
    call(&mut p,1,0,0,0).unwrap();assert!(p.mouse.view().visible,"show cancels the region");
}
#[test]
fn sensitivity_threshold_page_language_version_and_no_effect_calls(){
    let mut p=ready_mouse(3);
    call(&mut p,0x1b,0,0,0).unwrap();assert_eq!(regs(&p),(0x1b,50,50,64));
    call(&mut p,0x1a,30,40,100).unwrap();call(&mut p,0x1b,0,0,0).unwrap();assert_eq!(regs(&p),(0x1b,30,40,100));
    call(&mut p,0x13,0,0,0).unwrap();call(&mut p,0x1b,0,0,0).unwrap();assert_eq!(regs(&p).3,64,"0 = default 64");
    p.ram.store(0x462,MemWidth::Byte,2).unwrap();
    call(&mut p,0x1e,9,9,9).unwrap();assert_eq!(regs(&p).1,2);
    call(&mut p,0x23,9,9,9).unwrap();assert_eq!(regs(&p).1,0);
    call(&mut p,0x24,0,0,0).unwrap();assert_eq!(regs(&p),(0x24,0x0705,0x0400,0));
    let before=p.mouse.clone();
    for f in [0x0d,0x0e,0x12,0x1c,0x1d,0x22]{
        call(&mut p,f,0x1111,0x2222,0x3333).unwrap();
        assert_eq!(regs(&p),(f,0x1111,0x2222,0x3333),"AX={f:04x}h changes no register");
    }
    assert_eq!(p.mouse,before);
    assert!(call(&mut p,0x25,0,0,0).unwrap_err().contains("not implemented"));
}
#[test]
fn exchange_handlers_returns_the_previous_one_in_cx_es_dx(){
    let mut p=ready_mouse(3);
    set_es(&mut p,0x2000);call(&mut p,0x0c,0,0x1f,0x0123).unwrap();
    set_es(&mut p,0x3000);call(&mut p,0x14,0,0x02,0x0456).unwrap();
    assert_eq!((regs(&p).2,regs(&p).3,p.selectors[SegmentRegister::Es.index()]),(0x1f,0x0123,0x2000));
    assert_eq!(p.image.segments[SegmentRegister::Es.index()].base,0x20000);
    assert_eq!(p.mouse.handler,Handler{mask:2,segment:0x3000,offset:0x0456});
}
#[test]
fn disable_enable_and_software_reset(){
    let mut p=ready_mouse(3);
    call(&mut p,0x0f,0,4,4).unwrap();
    call(&mut p,0x1f,0,0,0).unwrap();
    assert_eq!((regs(&p).0,regs(&p).1,p.selectors[SegmentRegister::Es.index()]),(0x1f,0x5678,0x1234));
    assert!(!p.mouse.view().active);
    let (x,y)=(p.mouse.x,p.mouse.y);
    p.mouse.host(0,0,1,3);assert_eq!((p.mouse.x,p.mouse.y,p.mouse.buttons),(x,y,0),"no input while disabled");
    call(&mut p,0x26,0,0,0).unwrap();assert_eq!(regs(&p).1,1);
    call(&mut p,0x20,0,0,0).unwrap();assert!(p.mouse.view().active);
    call(&mut p,0x26,0,0,0).unwrap();assert_eq!(regs(&p).1,0);
    // 21h resets the driver state (not just reports it), like 00h.
    call(&mut p,0x21,0,0,0).unwrap();assert_eq!(regs(&p).0,0xffff);
    assert_eq!((p.mouse.ratio_x,p.mouse.ratio_y),(8,16));
    call(&mut p,0x1f,0,0,0).unwrap();call(&mut p,0x21,0,0,0).unwrap();assert!(!p.mouse.view().active,"21h keeps it disabled");
    call(&mut p,0,0,0,0).unwrap();assert!(p.mouse.view().active,"00h enables it again");
}
