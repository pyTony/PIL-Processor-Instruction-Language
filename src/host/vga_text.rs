//! The VGA text screen as a host picture: 9x16 character cells, 720x400 for
//! 80 columns, drawn with the BIOS 8x16 font (`crates/pil-bios/src/font8x16.txt`).
//!
//! As on a VGA in its standard text modes, the ninth column of a cell is the
//! background, except for the line-drawing characters C0h-DFh, which repeat
//! their eighth column there so that horizontal lines and blocks join. A
//! 40-column mode is drawn 360 pixels wide and doubled, as the VGA's halved
//! dot clock shows it. Nothing here reads guest memory: callers pass the
//! character/attribute pairs and the cursor.

#![cfg_attr(not(any(windows,all(unix,feature="x11"),test)),allow(dead_code,unused_imports))]
/// Cell width and height in pixels.
pub(crate) const CELL_WIDTH:usize=9;
pub(crate) use crate::bios::rom_font::CELL_HEIGHT;
pub(crate) use crate::bios::rom_font::font;

/// The 16 colours of the text attribute, as 0x00RRGGBB (the VGA's default
/// palette and DAC).
pub(crate) const COLORS:[u32;16]=[0x000000,0x0000aa,0x00aa00,0x00aaaa,0xaa0000,0xaa00aa,0xaa5500,0xaaaaaa,
    0x555555,0x5555ff,0x55ff55,0x55ffff,0xff5555,0xff55ff,0xffff55,0xffffff];

/// The cursor lines on a 16-line cell for the BIOS shape at 0040:0060
/// (start, end), or None when the shape hides it. A shape within 8 lines is
/// a CGA one, which the VGA BIOS scales: 6-7, the BIOS default, becomes the
/// VGA's 13-14 underline.
pub(crate) fn cursor_lines(start:u8,end:u8)->Option<(u32,u32)>{
    if start&0x20!=0||start&0x1f>end&0x1f{return None;}
    let (start,end)=(u32::from(start&0x1f),u32::from(end&0x1f));
    if end>=8{return Some((start.min(15),end.min(15)));}
    let scale=|line:u32|if line==0{0}else{line*2+1};
    Some((scale(start),if end==7{14}else{scale(end)}))
}

/// The text page as RGB pixels: (width, height, pixels). `cells` holds the
/// character/attribute pairs of `columns` columns; `cursor` is (row, column,
/// first line, last line) when it is to be drawn. Attribute bit 7 (blink,
/// the VGA default) is not drawn: the background takes bits 4-6 only.
pub(crate) fn text_pixels(cells:&[u8],columns:u32,font:&[u8],cursor:Option<(u32,u32,u32,u32)>)->(usize,usize,Vec<u32>){
    let columns=columns.max(1) as usize;let rows=cells.len()/(columns*2);
    let (width,height)=(columns*CELL_WIDTH,rows*CELL_HEIGHT);
    let mut pixels=vec![0u32;width*height];
    for row in 0..rows{for column in 0..columns{
        let at=(row*columns+column)*2;
        let (ch,attr)=(usize::from(cells[at]),cells[at+1]);
        let (fg,bg)=(COLORS[usize::from(attr&15)],COLORS[usize::from((attr>>4)&7)]);
        let under=cursor.filter(|c|(c.0 as usize,c.1 as usize)==(row,column));
        let line_graphics=(0xc0..=0xdf).contains(&ch);
        for y in 0..CELL_HEIGHT{
            let bits=font.get(ch*CELL_HEIGHT+y).copied().unwrap_or(0);
            let solid=under.is_some_and(|c|(c.2 as usize..=c.3 as usize).contains(&y));
            let line=&mut pixels[(row*CELL_HEIGHT+y)*width+column*CELL_WIDTH..][..CELL_WIDTH];
            for (x,p) in line.iter_mut().enumerate(){
                let lit=if x<8{bits&(0x80>>x)!=0}else{line_graphics&&bits&1!=0};
                *p=if solid||lit{fg}else{bg};
            }
        }
    }}
    if columns<=40{
        let mut wide=Vec::with_capacity(pixels.len()*2);
        for p in pixels{wide.push(p);wide.push(p);}
        return (width*2,height,wide);
    }
    (width,height,pixels)
}
