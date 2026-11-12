//! Host-only presentation for Dine's existing VGA framebuffer.
//!
//! The DOS/VGA machine does not depend on this module. The native terminal
//! remains the DOS keyboard and text console. In Windows desktop mode a GDI
//! window displays the current guest scanout; the window has no input focus.
//! We deliberately avoid adding any graphics crates to Cargo.lock just to
//! display a software framebuffer. Non-Windows builds retain ANSI preview.
use crate::dos;

/// A top-down 32-bit RGB scanout (0x00RRGGBB), not guest VRAM.

pub(crate) enum Display {
    Off,
    Ansi { shown: bool },
    #[cfg(windows)]
    Window(crate::dine_window::Window),
}

impl Display {
    pub(crate) fn new() -> Self {
        let mode=std::env::var("DINE_VIDEO")
            .unwrap_or_default().to_ascii_lowercase();
        match mode.as_str() {
            "off" | "none" | "headless" => Self::Off,
            "ansi" | "terminal" => Self::Ansi { shown: false },
            #[cfg(windows)]
            "" | "window" | "desktop" => Self::Window(crate::dine_window::Window::new()),
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
                if let Some((width,height,pixels))=dos::desktop_vga_frame(p) {
                    window.present(crate::dine_window::Frame{width:width as i32,height:height as i32,pixels});
                }
            },
        }
    }
}
