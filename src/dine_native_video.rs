//! Native block executor's optional display, reusing Dine's Win32/GDI
//! window thread and the shared four-plane VGA memory model. This module
//! contains no x86 decoder, PIL interpreter or DOS-service implementation.
//! It is embedded verbatim in emitted standalone Rust executables.
use crate::dos_runtime::Context;

/// Standard mode-12h 16-colour EGA palette in host RGB, matching Dine's
/// mode-set defaults. Native standalone currently admits no DAC writes.
/// A future wider service ABI must supply the live palette from Dine.
const EGA_RGB:[u32;16]=[
    0x000000,0x0000aa,0x00aa00,0x00aaaa,
    0xaa0000,0xaa00aa,0xaa5500,0xaaaaaa,
    0x555555,0x5555ff,0x55ff55,0x55ffff,
    0xff5555,0xff55ff,0xffff55,0xffffff,
];

/// Read guest planar pixels without accessing or changing the VGA latches.
fn mode12_rgb(planes:&crate::vga_planar::Planar)->Vec<u32>{
    let mut pixels=vec![0u32;640*480];
    for row in 0..480usize {
        for byte_col in 0..80usize{
            let offset=row*80+byte_col;
            let bytes=[planes.peek(0,offset),planes.peek(1,offset),
                       planes.peek(2,offset),planes.peek(3,offset)];
            for bit in 0..8usize{
                let shift=7-bit;
                let idx=(bytes[0]>>shift&1)|((bytes[1]>>shift&1)<<1)|
                        ((bytes[2]>>shift&1)<<2)|((bytes[3]>>shift&1)<<3);
                pixels[row*640+byte_col*8+bit]=EGA_RGB[usize::from(idx)];
            }
        }
    }
    pixels
}

/// Native video is strictly opt-in. Default execution stays headless, which
/// preserves repeatable verifier runs and Linux/Termux native binaries.
pub(crate) struct Display {
    interactive:bool,
    #[cfg(windows)]
    window:Option<crate::dine_window::Window>,
    #[cfg(windows)]
    last:std::time::Instant,
}
impl Display {
    pub(crate) fn new()->Result<Self,String>{
        let mode=std::env::var("DINE_VIDEO").unwrap_or_default().to_ascii_lowercase();
        let interactive=matches!(mode.as_str(),"window"|"desktop");
        if !mode.is_empty()&&!interactive&&!matches!(mode.as_str(),"off"|"none"|"headless"){
            return Err(format!("unsupported standalone DINE_VIDEO={mode:?}: use window or off"));
        }
        if interactive&&!cfg!(windows){
            return Err("standalone DINE_VIDEO=window currently requires Windows".into());
        }
        Ok(Self{
            interactive,
            #[cfg(windows)]
            window:if interactive{Some(crate::dine_window::Window::new())}else{None},
            #[cfg(windows)]
            last:std::time::Instant::now()-std::time::Duration::from_millis(100),
        })
    }

    /// Preview every 50 ms during compiled block execution, plus the entire
    /// final frame *before* the guest starts its BIOS keyboard wait.
    /// The window is non-activating, and all keyboard input stays in stdin.
    pub(crate) fn tick(&mut self,context:&Context,force:bool){
        #[cfg(windows)]
        if let Some(window)=self.window.as_mut(){
            if context.video.mode==0x12 &&
               (force||self.last.elapsed()>=std::time::Duration::from_millis(50)){
                if let Some(planes)=context.ram.planar(){
                    let pixels=mode12_rgb(planes);
                    window.present(crate::dine_window::Frame{
                        width:640,height:480,pixels,
                    });
                    self.last=std::time::Instant::now();
                }
            }
        }
        #[cfg(not(windows))]
        let _=(context,force);
    }

    /// Only a visible window opts into terminal key input. The legacy
    /// PIL_DOS_BIOS_KEY injection remains deterministic for headless tests.
    /// This intentionally accepts Enter only until a proper BIOS keyboard
    /// compatibility service is exposed to compiled guest programs.
    pub(crate) fn read_key(&self)->Result<Option<u16>,String>{
        if !self.interactive{return Ok(None);}
        use std::io::Write as _;
        eprint!("Press Enter to send a DOS key: ");
        std::io::stderr().flush().map_err(|e|e.to_string())?;
        let mut line=String::new();
        if std::io::stdin().read_line(&mut line).map_err(|e|e.to_string())?==0{
            return Err("DOS program waits for a key but stdin reached EOF".into());
        }
        Ok(Some(0x1c0d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rgb_scanout_reads_all_four_planes_without_changing_guest_state(){
        let mut planes=crate::vga_planar::Planar::new();
        planes.set_pixel(0,0,1); // blue
        planes.set_pixel(0,1,6); // brown
        planes.set_pixel(0,2,15); // white
        let before=planes.clone();
        let pixels=mode12_rgb(&planes);
        assert_eq!(pixels.len(),640*480);
        assert_eq!(&pixels[..4],&[0x0000aa,0xaa5500,0xffffff,0]);
        assert_eq!(planes,before);
    }
}
