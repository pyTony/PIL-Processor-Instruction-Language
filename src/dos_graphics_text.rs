//! Text in the graphics modes: CGA 04h/05h (320x200, 2 bits a pixel), CGA 06h
//! (640x200, 1 bit), VGA 13h (320x200, 1 byte) and the EGA/VGA 16-colour
//! planar modes 0Dh/0Eh/10h/12h (8-, 14- or 16-line cells). Like the IBM PC BIOS,
//! characters in 8-line cells are 8x8 glyphs drawn into the framebuffer:
//! 00h-7Fh from the ROM font at F000:FA6E, 80h-FFh from the table the INT 1Fh
//! vector points to (Dine's own table at C000:1000 until a program changes
//! the vector). In the 14- and 16-line cells of modes 10h and 12h all 256
//! glyphs come from the table INT 43h points to, as on an EGA/VGA BIOS; the
//! mode set points it at the 8x14 or 8x16 font in the video BIOS area.
//! Guest RAM stays authoritative. The font and the framebuffer are read from
//! it, and every change is a planned write that the caller commits all or none.
use super::*;
/// Linear address of the ROM font, F000:FA6E as on the IBM PC/XT/AT BIOS.
pub(in super::super) const FONT:u32=0xffa6e;
/// Far pointer to the 80h-FFh half in the video BIOS area (C000:1000). A
/// VGA BIOS points INT 1Fh at its own upper-half table, so Dine's colour
/// VGA identity starts INT 1Fh here; GRAFTABL or a program may replace it.
pub(in super::super) const UPPER_FONT:u32=0xc000_1000;
const FONT_TEXT:&str=include_str!("dos_font8x8.txt");
/// The 2048 font bytes for characters 00h-FFh, parsed from the readable
/// glyph file (a header line, then eight rows of '#' and '.').
pub(in super::super) fn font()->Result<Vec<u8>,String>{
    let mut lines=FONT_TEXT.lines().filter(|l|!l.starts_with(';'));
    let mut bytes=Vec::with_capacity(2048);
    for code in 0..256u32{
        let header=lines.next().ok_or("font file ends early")?;
        if u32::from_str_radix(header.get(..2).unwrap_or(""),16)!=Ok(code){return Err(format!("font glyph {code:02X}h header is {header:?}"));}
        for _ in 0..8{
            let row=lines.next().ok_or("font file ends inside a glyph")?;
            if row.len()!=8||!row.bytes().all(|b|b==b'#'||b==b'.'){return Err(format!("font glyph {code:02X}h row {row:?}"));}
            bytes.push(row.bytes().fold(0u8,|n,b|n<<1|u8::from(b==b'#')));
        }
    }
    if lines.next().is_some(){return Err("font file has data after glyph FFh".into());}
    Ok(bytes)
}
pub(super) fn is_graphics(mode:u32)->bool{matches!(mode,4|5|6|0x13)||super::super::planar::geometry(mode).is_some()}
/// A copy of the framebuffer of a graphics mode; the plan diffs it at the end.
/// Planar modes copy each plane's used bytes through the host plane alias.
struct Frame{mode:u32,base:u32,old:Vec<u8>,bytes:Vec<u8>,columns:u32,rows:u32,cell:u32,plane_bytes:usize,page_offset:usize}
impl Frame{
    /// The frame of `page`; only planar modes have more than page 0.
    fn read(p:&Loaded,page:u32)->Result<Self,String>{
        let mode=byte(p,0x449)?;
        if page>=8{return Err("graphics page outside 0..7".into());}
        if let Some(g)=super::super::planar::geometry(mode){
            let planar=p.ram.planar().ok_or("planar memory is not mapped")?;let n=g.bytes();
            let offset=super::super::planar::page_offset(mode,page)?;
            let old:Vec<u8>=(0..4).flat_map(|k|planar.plane(k)[offset..offset+n].iter().copied()).collect();
            return Ok(Self{mode,base:crate::memory::PLANE_ALIAS,bytes:old.clone(),old,columns:g.columns(),rows:g.rows,cell:g.cell,plane_bytes:n,page_offset:offset});
        }
        if page!=0{return Err("graphics modes have only display page 0".into());}
        let(base,len)=match mode{4|5|6=>(VIDEO,0x4000),0x13=>(0xa0000,64000),_=>return Err(format!("mode {mode:02x}h is not a graphics mode"))};
        let old=p.ram.bytes().get(base as usize..(base+len)as usize).ok_or("framebuffer outside RAM")?.to_vec();
        Ok(Self{mode,base,bytes:old.clone(),old,columns:if mode==6{80}else{40},rows:25,cell:8,plane_bytes:0,page_offset:0})
    }
    fn columns(&self)->u32{self.columns}
    fn planar(&self)->bool{self.plane_bytes!=0}
    fn mask(&self)->u8{match self.mode{6=>1,0x13=>255,_ if self.planar()=>15,_=>3}}
    /// Byte offset, bit shift and value mask of pixel (x, y).
    fn at(&self,x:u32,y:u32)->(usize,u32,u8){
        let bank=(y&1)*0x2000+(y/2)*80;
        match self.mode{
            _ if self.planar()=>((y*self.columns+x/8)as usize,7-x%8,1),
            6=>((bank+x/8)as usize,7-x%8,1),
            0x13=>((y*320+x)as usize,0,255),
            _=>((bank+x/4)as usize,6-(x%4)*2,3),
        }
    }
    fn get(&self,x:u32,y:u32)->u8{
        let(i,s,m)=self.at(x,y);
        if self.planar(){return (0..4).fold(0,|c,n|c|((self.bytes[n*self.plane_bytes+i]>>s)&1)<<n);}
        (self.bytes[i]>>s)&m
    }
    fn set(&mut self,x:u32,y:u32,value:u8){
        let(i,s,m)=self.at(x,y);
        if self.planar(){for n in 0..4{let b=&mut self.bytes[n*self.plane_bytes+i];*b=(*b&!(1<<s))|(((value>>n)&1)<<s);}return;}
        self.bytes[i]=(self.bytes[i]&!(m<<s))|((value&m)<<s);
    }
    /// One character cell. Bit 7 of the colour XORs the glyph into the cell in
    /// the CGA and planar modes; otherwise set pixels take the colour and the
    /// rest 0.
    fn draw(&mut self,row:u32,col:u32,glyph:&[u8],color:u8){
        let xor=self.mode!=0x13&&color&0x80!=0;let color=color&self.mask();
        for r in 0..self.cell{let bits=glyph.get(r as usize).copied().unwrap_or(0);for b in 0..8{
            let(x,y)=(col*8+b,row*self.cell+r);let on=bits>>(7-b)&1!=0;
            if xor{if on{let old=self.get(x,y);self.set(x,y,old^color);}}
            else{self.set(x,y,if on{color}else{0});}
        }}
    }
    /// Moves character rows of an inclusive cell window; vacated rows take
    /// `fill`. Zero lines, or more than the window height, clear the window.
    fn scroll(&mut self,(top,left,bottom,right):(u32,u32,u32,u32),lines:u32,fill:u8,up:bool){
        let height=bottom-top+1;let lines=if lines==0||lines>height{height}else{lines};
        let rows:Vec<u32>=if up{(top..=bottom).collect()}else{(top..=bottom).rev().collect()};
        for row in rows{
            let source=if up{row+lines}else{row.wrapping_sub(lines)};
            let inside=if up{source<=bottom}else{row>=top+lines};
            let h=self.cell;
            for y in row*h..row*h+h{for x in left*8..(right+1)*8{
                let value=if inside{self.get(x,source*h+y%h)}else{fill};
                self.set(x,y,value);
            }}
        }
    }
    fn writes(&self)->Writes{
        self.bytes.iter().zip(&self.old).enumerate().filter(|(_,(n,o))|n!=o)
            .map(|(i,(n,_))|{
                let at=if self.planar(){(i/self.plane_bytes*crate::vga_planar::PLANE+self.page_offset+i%self.plane_bytes)as u32}else{i as u32};
                (self.base+at,MemWidth::Byte,u32::from(*n))
            }).collect()
    }
}
fn byte(p:&Loaded,at:u32)->Result<u32,String>{
    p.ram.load(at,MemWidth::Byte).map_err(|e|format!("BIOS graphics read: {e:?}"))
}
/// The glyph of `ch` for cells `cell` lines high, one byte a line. 14- and
/// 16-line cells read the INT 43h table. In 8-line cells 80h-FFh come through
/// INT 1Fh, which starts at Dine's own upper-half table. A program that sets
/// either vector to 0 leaves no table; the call then stops instead of drawing
/// whatever the interrupt table holds.
fn glyph(p:&Loaded,cell:u32,ch:u32)->Result<Vec<u8>,String>{
    if cell!=8{
        let vector=p.ram.load(0x43*4,MemWidth::Dword).map_err(|e|format!("INT 43h vector read: {e:?}"))?;
        if vector==0{return Err(format!("graphics-mode character {ch:02X}h needs an INT 43h font table; none is installed"));}
        let at=((vector>>16)<<4).wrapping_add(vector&0xffff).wrapping_add(ch*cell);
        return (0..cell).map(|n|byte(p,at+n).map(|b|b as u8)).collect();
    }
    let at=if ch<0x80{FONT+ch*8}else{
        let vector=p.ram.load(0x1f*4,MemWidth::Dword).map_err(|e|format!("INT 1Fh vector read: {e:?}"))?;
        if vector==0{return Err(format!("graphics-mode character {ch:02X}h needs an INT 1Fh font table (as GRAFTABL installs); none is installed"));}
        ((vector>>16)<<4).wrapping_add(vector&0xffff).wrapping_add((ch-0x80)*8)
    };
    (0..8).map(|n|byte(p,at+n).map(|b|b as u8)).collect()
}
/// Cursor of `page`, the page `frame` was read from.
fn cursor(p:&Loaded,frame:&Frame,page:u32)->Result<(u32,u32),String>{
    let position=p.ram.load(0x450+page*2,MemWidth::Word).map_err(|e|format!("BIOS cursor read: {e:?}"))?;
    let(row,col)=(position>>8,position&255);
    if row>=frame.rows||col>=frame.columns(){return Err(format!("BIOS cursor outside the {}x{} graphics text grid",frame.rows,frame.columns()));}
    Ok((row,col))
}
/// The page AH08h-AH0Ah use: BH in the EGA/VGA planar modes, which have
/// several pages, as the IBM EGA BIOS and SeaBIOS take it; the CGA modes and
/// mode 13h have page 0 only, and their BIOSes ignore BH.
fn bh_page(p:&Loaded)->Result<u32,String>{
    if super::super::planar::geometry(byte(p,0x449)?).is_some(){Ok((p.cpu.regs[3]>>8)&255)}else{Ok(0)}
}
/// Teletype (AH0Eh, and DOS console output, which FreeDOS sends through
/// INT 29h as AH0Eh with BX=0007h) in a graphics mode: BEL does nothing here,
/// BS/CR/LF move the cursor, other bytes draw in `color`. A line feed on the
/// last row scrolls the whole screen up 8 scanlines, filling with colour 0.
pub(super) fn teletype(p:&Loaded,bytes:&[u8],color:u32,dos_tabs:bool)->Result<Writes,String>{
    if !dos_tabs&&bytes.contains(&9){return Err("BIOS teletype TAB policy is not part of this profile".into());}
    let mut frame=Frame::read(p,byte(p,0x462)?)?;let cols=frame.columns();
    let(mut row,mut col)=cursor(p,&frame,byte(p,0x462)?)?;
    for &b in bytes{
        let n=if b==9{8-col%8}else{1};
        for _ in 0..n{
            match if b==9{32}else{b}{
                7=>{},
                8=>col=col.saturating_sub(1),
                10=>row+=1,
                13=>col=0,
                ch=>{let shape=glyph(p,frame.cell,u32::from(ch))?;frame.draw(row,col,&shape,color as u8);col+=1;if col==cols{col=0;row+=1;}},
            }
            if row==frame.rows{let last=frame.rows-1;frame.scroll((0,0,last,cols-1),1,0,true);row=last;}
        }
    }
    let mut writes=frame.writes();writes.push((0x450+byte(p,0x462)?*2,MemWidth::Word,(row<<8)|col));
    Ok(writes)
}
/// AH09h/AH0Ah: `count` copies of AL from the cursor in colour BL, without
/// moving the cursor. The run may continue onto later rows but not past the
/// screen, which the BIOS implementations treat differently.
pub(super) fn write_chars(p:&Loaded)->Result<Writes,String>{
    let page=bh_page(p)?;let mut frame=Frame::read(p,page)?;let cols=frame.columns();
    let(row,col)=cursor(p,&frame,page)?;
    let count=p.cpu.regs[1]&0xffff;let first=row*cols+col;
    if count>frame.rows*cols-first{return Err("BIOS repeated write exceeds the graphics screen".into());}
    let shape=glyph(p,frame.cell,p.cpu.regs[0]&255)?;let color=p.cpu.regs[3]as u8;
    for cell in first..first+count{frame.draw(cell/cols,cell%cols,&shape,color);}
    Ok(frame.writes())
}
/// AH08h: the character whose glyph matches the cell's lit pixels (any colour
/// but 0). 14- and 16-line cells search the INT 43h table; 8-line cells
/// search 00h-7Fh, then the INT 1Fh table when one is installed.
/// AL=0 when nothing matches; AH=0, as graphics cells have no attribute.
pub(super) fn read_char(p:&Loaded)->Result<u32,String>{
    let page=bh_page(p)?;let frame=Frame::read(p,page)?;let(row,col)=cursor(p,&frame,page)?;
    let h=frame.cell;
    let cell:Vec<u8>=(0..h).map(|r|(0..8).fold(0u8,|bits,b|if frame.get(col*8+b,row*h+r)!=0{bits|0x80>>b}else{bits})).collect();
    let upper=h!=8||p.ram.load(0x1f*4,MemWidth::Dword).map_err(|e|format!("INT 1Fh vector read: {e:?}"))?!=0;
    for ch in 0..if upper{256}else{128}{
        if glyph(p,h,ch)?==cell{return Ok(ch);}
    }
    Ok(0)
}
/// AH06h/AH07h in a graphics mode: the window is in character cells; vacated
/// rows take the pixel value in BH.
pub(super) fn scroll(p:&Loaded,up:bool)->Result<Writes,String>{
    let mut frame=Frame::read(p,byte(p,0x462)?)?;let cols=frame.columns();
    let(cx,dx)=(p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff);
    let(top,left)=(cx>>8,cx&255);let(bottom,right)=((dx>>8).min(frame.rows-1),(dx&255).min(cols-1));
    if top>bottom||left>right{return Ok(Vec::new());}
    frame.scroll((top,left,bottom,right),p.cpu.regs[0]&255,(p.cpu.regs[3]>>8)as u8,up);
    Ok(frame.writes())
}
/// The characters on the displayed page of a graphics mode, for copying
/// text off the screen: (columns, rows, one CP437 byte a cell). Each cell is
/// matched against the glyphs the BIOS would draw it with, taking lit pixels
/// first as those not the cell's most common colour (the background, also
/// when it is not colour 0), then as those not colour 0. A cell that matches no glyph,
/// such as part of a picture, reads as a space, as does a cell of one
/// colour (a full block cannot be told from a filled area).
pub(in super::super) fn screen_text(p:&Loaded)->Result<(u32,u32,Vec<u8>),String>{
    let frame=Frame::read(p,byte(p,0x462)?)?;let h=frame.cell;
    let upper=h!=8||p.ram.load(0x1f*4,MemWidth::Dword).map_err(|e|format!("INT 1Fh vector read: {e:?}"))?!=0;
    let mut known=std::collections::HashMap::new();
    known.insert(vec![0u8;h as usize],0x20u8);
    for ch in 0..if upper{256}else{128}{known.entry(glyph(p,h,ch)?).or_insert(ch as u8);}
    let mut text=Vec::with_capacity((frame.rows*frame.columns)as usize);
    for row in 0..frame.rows{for col in 0..frame.columns{
        let pixels:Vec<u8>=(0..h*8).map(|i|frame.get(col*8+i%8,row*h+i/8)).collect();
        let mut counts=[0u32;256];for &c in &pixels{counts[usize::from(c)]+=1;}
        let common=(0..=255u8).max_by_key(|&c|(counts[usize::from(c)],std::cmp::Reverse(c))).unwrap_or(0);
        let shape=|background:u8|->Vec<u8>{(0..h as usize).map(|r|(0..8).fold(0u8,|bits,b|if pixels[r*8+b]!=background{bits|0x80>>b}else{bits})).collect()};
        let ch=if counts[usize::from(common)] as usize==pixels.len(){0x20}else{known.get(&shape(common)).or_else(||known.get(&shape(0))).copied().unwrap_or(0x20)};
        text.push(ch);
    }}
    Ok((frame.columns,frame.rows,text))
}
