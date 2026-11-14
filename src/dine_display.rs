//! Host-only presentation for Dine's existing VGA framebuffer.
//!
//! The DOS/VGA machine does not depend on this module. The native terminal
//! remains the DOS keyboard and text console. In Windows desktop mode a GDI
//! window displays the current guest scanout; the window has no input focus.
//! We deliberately avoid adding any graphics crates to Cargo.lock just to
//! display a software framebuffer. Non-Windows builds retain ANSI preview.
use crate::dos;

/// Opt-in native-video diagnostics, independent of the session JSON protocol.
/// The host launcher inherits DINE_VIDEO_LOG from the user's environment.
/// Each event is one append so the Win32 worker and DOS execution thread can
/// record transitions without holding a shared lock or altering guest state.
pub(crate) fn debug_log(message:&str) {
    let Some(path)=std::env::var_os("DINE_VIDEO_LOG") else {return;};
    if path.is_empty(){return;}
    let milliseconds=std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0,|elapsed|elapsed.as_millis());
    let line=format!("{milliseconds} pid={} {message}\n",std::process::id());
    let _=append_log(std::path::Path::new(&path),line.as_bytes());
}
fn append_log(path:&std::path::Path,line:&[u8])->std::io::Result<()> {
    use std::io::Write as _;
    std::fs::OpenOptions::new().create(true).append(true).open(path)?.write_all(line)
}

// The independent RGB frame type lives in dine_window.rs.
pub(crate) enum Display {
    Off,
    Ansi { shown: bool },
    #[cfg(windows)]
    Window(crate::dine_window::Window),
}

impl Display {
    #[cfg(windows)]
    pub(crate) fn window_closed(&self)->bool{
        match self{Self::Window(window)=>window.closed(),_=>false}
    }
    #[cfg(windows)]
    pub(crate) fn drain_mouse_events(&self)->Vec<(u16,u16,u16)>{
        match self{Self::Window(window)=>window.drain_mouse_events(),_=>Vec::new()}
    }
    pub(crate) fn new() -> Self {
        let mode=std::env::var("DINE_VIDEO")
            .unwrap_or_default().to_ascii_lowercase();
        match mode.as_str() {
            "off" | "none" | "headless" => Self::Off,
            "ansi" | "terminal" => Self::Ansi { shown: false },
            #[cfg(windows)]
            "" | "window" | "desktop" => {
                debug_log("display selected: Win32 GDI window (lazy until first graphics frame)");
                Self::Window(crate::dine_window::Window::new())
            },
            #[cfg(not(windows))]
            "" | "window" | "desktop" => Self::Ansi { shown: false },
            _ => Self::Ansi { shown: false },
        }
    }

    /// Called at bounded DOS execution slices and immediately before an
    /// input wait; window updates never block on keyboard input or painting.
    pub(crate) fn update(&mut self, p:&dos::Loaded, waiting:bool) {
        match self {
            Self::Off => {},
            Self::Ansi { shown } => {
                if waiting && !*shown {
                    if let Some(frame)=dos::ansi_graphics_preview(p) {
                        println!("\n{frame}");
                        *shown=true;
                    }
                }
            },
            #[cfg(windows)]
            Self::Window(window) => {
                let (max_x,max_y,visible)=dos::mouse_state(p);
                window.set_mouse_state(max_x,max_y,visible);
                if let Some((width,height,pixels))=dos::desktop_vga_frame(p) {
                    window.present(crate::dine_window::Frame{width:width as i32,height:height as i32,pixels});
                }else{
                    window.no_frame();
                }
            },
        }
    }
}

#[cfg(test)]
mod video_log_tests {
    #[test]
    fn video_log_appends_without_replacing_existing_events() {
        let unique=std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir()
            .join(format!("pil-video-log-{}-{unique}.txt",std::process::id()));
        super::append_log(&path,b"frame absent\n").unwrap();
        super::append_log(&path,b"window created\n").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(),b"frame absent\nwindow created\n");
        std::fs::remove_file(path).unwrap();
    }
}
