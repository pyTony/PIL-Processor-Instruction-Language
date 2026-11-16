//! ANSI true-colour approximation of the existing DOS VGA/CGA framebuffer.
//! Used by the Termux/Linux terminal frontend without a graphical display.
//! This module does not read guest RAM, decode video modes, or own input.
pub(super) fn half_block_frame(frame:&(usize,usize,Vec<u32>),columns:u16,rows:u16)->String{
    let (width,height,pixels)=frame;
    if *width==0||*height==0||pixels.len()!=width.saturating_mul(*height){return String::new();}
    // Stay away from the last column and row: some terminals wrap/scroll on
    // writing the bottom-right cell. Limit work on unusually large terminals.
    let cols=usize::from(columns).saturating_sub(1).clamp(1,160);
    let lines=usize::from(rows).saturating_sub(1).clamp(1,60);
    let mut out=String::with_capacity(cols*lines*36);
    out.push_str("\x1b[H\x1b[?25l");
    for row in 0..lines{
        use std::fmt::Write as _;
        write!(out,"\x1b[{};1H",row+1).unwrap();
        let mut last=None;
        for col in 0..cols{
            let x=(col*width/cols).min(width-1);
            let upper=((2*row)*height/(2*lines)).min(height-1);
            let lower=((2*row+1)*height/(2*lines)).min(height-1);
            let top=pixels[upper*width+x]&0xffffff;
            let bottom=pixels[lower*width+x]&0xffffff;
            let colors=(top,bottom);
            if last!=Some(colors){
                write!(out,"\x1b[38;2;{};{};{};48;2;{};{};{}m",
                    top>>16&255,top>>8&255,top&255,
                    bottom>>16&255,bottom>>8&255,bottom&255).unwrap();
                last=Some(colors);
            }
            out.push('▀');
        }
        out.push_str("\x1b[0m");
    }
    out
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn two_pixel_rows_use_independent_foreground_and_background(){
        let frame=(2,2,vec![0xff0000,0x00ff00,0x0000ff,0xffffff]);
        let ansi=half_block_frame(&frame,3,2);
        assert!(ansi.contains("\x1b[38;2;255;0;0;48;2;0;0;255m▀"));
        assert!(ansi.contains("\x1b[38;2;0;255;0;48;2;255;255;255m▀"));
        assert!(!ansi.contains("\n"));
    }
    #[test]
    fn invalid_frame_is_not_rendered(){
        assert_eq!(half_block_frame(&(2,2,vec![0]),80,25),"");
        assert_eq!(half_block_frame(&(0,0,vec![]),80,25),"");
    }
}
