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
    format!(
        r#"{{"schema":1,"time_unix_ms":{timestamp},"status":{},"steps":{steps},"initial_executable":{program},"current_psp":"{:04X}","exec_depth":{},"current_instruction":{pc},"history_enabled":{},"history_capacity":{},"recent_dispatches":{recent},"video":{video},"keyboard":{keyboard}}}"#,
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
        assert!(first.contains(r#""bios_mode":3"#));
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
    fn live_snapshot_preserves_preexisting_dispatch_history(){
        let mut p=load(&[0xeb,0xfe],"SPIN.COM").unwrap();
        p.history=history::History::new(64);
        let mut session=session::Session::new(p);
        let (_,started)=session.live_snapshot(0).unwrap();
        assert!(!started);
        assert_eq!(session.machine().unwrap().history.capacity(),64);
    }
}
