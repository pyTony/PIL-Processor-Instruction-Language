//! On-demand, read-only DOS guest snapshots for the native host console.
//! Capture only between execution slices, while Session owns the Loaded state.
//! The first capture may opt into bounded dispatch tracing, which intentionally
//! disables native block execution until the user turns tracing off.
use super::*;
use std::time::{SystemTime,UNIX_EPOCH};

fn bda(p:&Loaded,address:u32,width:MemWidth)->Option<u32>{
    p.ram.load(address,width).ok()
}
fn number(value:Option<u32>)->String{
    value.map_or_else(||"null".into(),|v|v.to_string())
}
/// Standard DOS PSP:2Ch points to the environment. After its double NUL,
/// Dine's EXEC loader writes a count word (1) and the child's full path.
/// Treat this as best-effort, untrusted guest data: bound both scans.
fn current_program(p:&Loaded)->Option<String>{
    let psp=u32::from(p.psp)*16;
    let segment=bda(p,psp+0x2c,MemWidth::Word)?;
    if segment==0{return None;}
    let base=segment.checked_mul(16)?;
    let mut previous_zero=false;
    for i in 0..0x8000u32{
        let value=bda(p,base.checked_add(i)?,MemWidth::Byte)? as u8;
        if value==0&&previous_zero{
            let count=bda(p,base.checked_add(i+1)?,MemWidth::Word)?;
            if count!=1{return None;}
            let start=base.checked_add(i+3)?;
            let mut bytes=Vec::new();
            for n in 0..260u32{
                let value=bda(p,start.checked_add(n)?,MemWidth::Byte)? as u8;
                if value==0{
                    return (!bytes.is_empty()).then(||String::from_utf8_lossy(&bytes).into_owned());
                }
                bytes.push(value);
            }
            return None;
        }
        previous_zero=value==0;
    }
    None
}
/// Raw first five rows of the active text page, character/attribute pairs.
/// Capture before mouse overlays or host palette conversion.
fn text_cells_hex(p:&Loaded,mode:Option<u32>)->Option<String>{
    if !mode.is_some_and(|m|matches!(m,0..=3|7)){return None;}
    let columns=bda(p,0x44a,MemWidth::Word)?.clamp(1,80);
    let page=bda(p,0x462,MemWidth::Byte)?&7;
    let size=bda(p,0x44c,MemWidth::Word)?;
    let origin=if p.vga.start_override{u32::from(p.vga.start())*2}else{page*size};
    let base=if mode==Some(7){0xb0000}else{0xb8000};
    let mut hex=String::with_capacity((columns*5*4)as usize);
    use std::fmt::Write as _;
    for index in 0..columns*5*2{
        let byte=bda(p,base+(origin+index)%32768,MemWidth::Byte)? as u8;
        write!(&mut hex,"{byte:02x}").ok()?;
    }
    Some(hex)
}
/// One JSON object, with no embedded newlines, for the append-only log.
/// The initial executable name belongs to the session; PSP and EXEC depth
/// distinguish a running child from the original COMMAND.COM loader.
pub(super) fn capture(p:&Loaded,steps:usize,host_pending:usize,status:&str)->String{
    let timestamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let mode=bda(p,0x449,MemWidth::Byte).map(|n|n&0x7f);
    let graphics=mode.is_some_and(|n|!matches!(n,0..=3|7));
    let video=format!(
        r#"{{"bios_mode":{},"graphics_mode":{graphics},"bios_columns":{},"bios_page_start":{},"crtc_start":{},"crtc_start_override":{},"cga_mode_control":{},"attribute_mode_control":{},"sequencer":{:?},"graphics_controller":{:?}}}"#,
        number(mode),number(bda(p,0x44a,MemWidth::Word)),
        number(bda(p,0x44e,MemWidth::Word)),p.vga.start(),
        p.vga.start_override,p.vga.cga_mode(),p.vga.attr[16],
        p.vga.idle.seq,p.vga.idle.gc
    );
    let text_cells=text_cells_hex(p,mode).map_or_else(||"null".into(),|v|stoplog::quote(&v));
    let text_write_watch=p.ram.text_watch_json();
    let head=bda(p,0x41a,MemWidth::Word);
    let tail=bda(p,0x41c,MemWidth::Word);
    let valid=|n:u32|(0x1e..0x3e).contains(&n)&&n&1==0;
    let bios_queued=match(head,tail){
        (Some(h),Some(t)) if valid(h)&&valid(t)=>Some((t+32-h)%32/2),
        _=>None,
    };
    let (irq_queued,controller_unread,controller_latch)=p.speaker.kbd.diagnostic();
    let keyboard=format!(
        r#"{{"host_pending":{host_pending},"bios_head":{},"bios_tail":{},"bios_queued":{},"irq_queued":{irq_queued},"controller_unread":{controller_unread},"controller_latch":{}}}"#,
        number(head),number(tail),number(bios_queued),
        number(controller_latch.map(u32::from))
    );
    let pc=history::Dispatch::capture(p).json();
    let recent=p.history.recent_json(16);
    let program=stoplog::quote(&String::from_utf8_lossy(&p.own_name));
    let active=current_program(p).map_or_else(||"null".into(),|s|stoplog::quote(&s));
    format!(
        r#"{{"schema":1,"time_unix_ms":{timestamp},"status":{},"steps":{steps},"initial_executable":{program},"active_executable":{active},"current_psp":"{:04X}","exec_depth":{},"current_instruction":{pc},"history_enabled":{},"history_capacity":{},"recent_dispatches":{recent},"video":{video},"text_cells_first5_hex":{text_cells},"text_write_watch":{text_write_watch},"keyboard":{keyboard}}}"#,
        stoplog::quote(status),p.psp,p.exec.len(),p.history.enabled(),p.history.capacity()
    )
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn live_snapshot_traces_after_first_capture_and_can_be_disabled(){
        // Infinite guest loop. The host is able to inspect it without a stop.
        let mut p=load(&[0xeb,0xfe],"SPIN.COM").unwrap();
        p.history=history::History::default(); // independent of the test runner environment
        let mut session=session::Session::new(p);
        let before=session.machine().unwrap().cpu.eip;
        let (first,started)=session.live_snapshot(3).unwrap();
        assert!(started);
        assert!(first.contains(r#""history_enabled":true"#));
        assert!(first.contains(r#""host_pending":3"#));
        assert!(first.contains(r#""current_psp":"#));
        assert!(first.contains(r#""active_executable":"#));
        assert!(current_program(session.machine().unwrap()).unwrap().to_ascii_uppercase().ends_with("SPIN.COM"));
        assert!(first.contains(r#""bios_mode":3"#));
        assert!(first.contains(r#""text_cells_first5_hex":"2007"#));
        assert!(first.contains(r#""recent_dispatches":[]"#));
        assert!(!first.contains('\n'));
        assert_eq!(session.machine().unwrap().cpu.eip,before);
        session.run(24).unwrap();
        let (second,started)=session.live_snapshot(0).unwrap();
        assert!(!started);
        assert!(second.contains(r#""recent_dispatches":[{"sequence":"#));
        assert!(second.contains(r#""steps":24"#));
        let recent=second.split(r#""recent_dispatches":"#).nth(1).unwrap().split(r#","video":"#).next().unwrap();
        assert_eq!(recent.matches(r#""sequence":"#).count(),16);
        assert!(!second.contains('\n'));
        session.disable_live_trace().unwrap();
        assert!(!session.machine().unwrap().history.enabled());
    }
    #[test]
    fn explicit_dos_history_can_be_enabled_without_running_guest(){
        let p=load(&[0xeb,0xfe],"SPIN.COM").unwrap();
        let mut session=session::Session::new(p);
        let before=session.machine().unwrap().cpu.eip;
        session.enable_live_trace(256).unwrap();
        assert_eq!(session.machine().unwrap().history.capacity(),256);
        assert_eq!(session.machine().unwrap().cpu.eip,before);
        session.run(32).unwrap();
        let (snapshot,started)=session.live_snapshot(0).unwrap();
        assert!(!started);
        assert!(snapshot.contains(r#""history_capacity":256"#));
        assert!(snapshot.contains(r#""recent_dispatches":[{"sequence":"#));
        session.disable_live_trace().unwrap();
        assert_eq!(session.machine().unwrap().history.capacity(),0);
    }
    #[test]
    fn text_diagnostics_capture_exact_guest_character_and_attribute_bytes(){
        let mut p=load(&[0xcd,0x20],"VC.COM").unwrap();
        p.ram.store(0x449,MemWidth::Byte,2).unwrap();
        // VisiCalc A1 near the top of the sheet, with black-on-black '1'.
        p.ram.store(0xb8000+(3*80+1)*2,MemWidth::Word,0x0031).unwrap();
        let hex=text_cells_hex(&p,Some(2)).unwrap();
        let offset=(3*80+1)*4;
        assert_eq!(&hex[offset..offset+4],"3100");
        assert_eq!(hex.len(),80*5*4);
        assert_eq!(text_cells_hex(&p,Some(0x13)),None);
    }
    #[test]
    fn whole_screen_text_write_watch_tracks_changed_bytes_and_runtime_site(){
        let p=load(&[0xeb,0xfe],"GENERIC.COM").unwrap();
        let mut session=session::Session::new(p);
        assert!(!session.machine().unwrap().ram.text_watch_enabled());
        session.start_text_write_watch().unwrap();
        let p=session.machine_mut().unwrap();
        p.ram.text_watch_site(0x1234,0x5678);
        // CGA page 0, later text page and the MDA aperture.
        // Change the attribute too: the initialized screen already uses 07h.
        p.ram.store(0xb8000,MemWidth::Word,0x1f31).unwrap();
        p.ram.store(0xb9000,MemWidth::Byte,0x41).unwrap();
        p.ram.write_bytes(0xb0000,&[0x42,0x1f]).unwrap();
        // An identical write must not generate a spurious event.
        p.ram.store(0xb9000,MemWidth::Byte,0x41).unwrap();
        // Normal DOS memory is outside the watched region.
        p.ram.store(0x2000,MemWidth::Byte,0x44).unwrap();
        let json=p.ram.text_watch_json();
        assert!(json.contains(r#""total":5"#),"{json}");
        assert!(json.contains(r#""address":753664"#),"{json}"); // B8000h
        assert!(json.contains(r#""address":757760"#),"{json}"); // B9000h
        assert!(json.contains(r#""address":720896"#),"{json}"); // B0000h
        assert!(json.contains(r#""cs":"1234","ip":"5678""#),"{json}");
        let (snapshot,_)=session.live_snapshot(0).unwrap();
        assert!(snapshot.contains(r#""text_write_watch":{"enabled":true"#));
        session.stop_text_write_watch().unwrap();
        assert!(!session.machine().unwrap().ram.text_watch_enabled());
        assert!(session.machine().unwrap().ram.text_watch_json().contains(r#""enabled":false"#));
    }
    #[test]
    fn live_snapshot_preserves_preexisting_dispatch_history(){
        let mut p=load(&[0xeb,0xfe],"SPIN.COM").unwrap();
        p.history=history::History::new(64);
        let mut session=session::Session::new(p);
        let (_,started)=session.live_snapshot(0).unwrap();
        assert!(!started);
        assert_eq!(session.machine().unwrap().history.capacity(),64);
    }
}
