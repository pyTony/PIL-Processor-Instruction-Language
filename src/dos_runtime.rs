//! Minimal standalone DOS boundary, independent of the decoder and interpreter.
//! This headless subset has no files, framebuffer, keyboard, installed handlers or EXEC.
use crate::{address::{MemoryContext,SegmentRegister,SegmentState},host_io::Output,MemWidth,Ram};

/// BIOS video mode and cursor state. Planar memory is owned by Ram and uses
/// Dine's shared VGA engine; no duplicate pixel/graphics semantics are emitted.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct VideoState {pub mode:u8,pub cursor_row:u8,pub cursor_col:u8}
impl Default for VideoState {
    fn default()->Self{Self{mode:3,cursor_row:0,cursor_col:0}}
}

// The compiler embeds this context and its methods into generated applications.
#[allow(dead_code)]
pub struct Context {
    pub regs:[u32;8], pub flags:u32, pub undefined:u32, pub eip:u32,
    pub ram:Ram, pub machine:SegmentState, pub video:VideoState,
    /// An explicitly supplied BIOS scan-code/ASCII word (one-shot).
    pub key:Option<u16>,
    /// Proven near-call return sites and operand widths; never guest memory.
    pub calls:Vec<(u32,u8)>,
}

/// Shared AH09 validation: collect the whole string before any visible effects.
pub(crate) fn dollar_string(ram:&Ram,context:MemoryContext,offset:u32,eip:u32,remaining:usize)->Result<Vec<u8>,String>{
    let mut text=Vec::new();
    for n in offset..=0xffff {
        let at=context.translate(n,MemWidth::Byte,false,eip).map_err(|e|format!("DOS string access: {e:?}"))?;
        let b=ram.load(at,MemWidth::Byte).map_err(|e|format!("DOS string RAM access: {e:?}"))? as u8;
        if b==b'$'{return Ok(text);}
        if text.len()>=remaining{return Err("DOS output limit reached before string terminator".into());}
        text.push(b);
    }
    Err("DOS string has no $ terminator before segment end".into())
}

/// Shared AH02 console-byte plan. Validate a complete tab before output or AL.
pub(crate) fn character_output(byte:u8,out:&impl Output)->Result<(Vec<u8>,u8),String>{
    let count=if byte==9{8-out.column()%8}else{1};
    if count>out.remaining(){return Err("DOS output limit reached before character output".into());}
    Ok((vec![if byte==9{b' '}else{byte};count],if byte==9{b' '}else{byte}))
}

/// Explicit, deterministic headless BIOS key: four hex digits, scan then
/// ASCII. The runtime never invents a key for a waiting DOS program.
pub fn parse_key(text:&str)->Result<u16,String>{
    let hex=text.strip_prefix("0x").or_else(||text.strip_prefix("0X")).unwrap_or(text);
    if hex.len()!=4||!hex.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err("PIL_DOS_BIOS_KEY must be four hex digits (e.g. 1C0D for Enter)".into());
    }
    u16::from_str_radix(hex,16).map_err(|e|format!("invalid PIL_DOS_BIOS_KEY: {e}"))
}

// Same readable 8x8 ROM font asset used by Dine graphics_text. The native
// generator embeds this constant verbatim to avoid a runtime source-file
// dependency, while preserving the original glyph bytes.
const ROM_FONT_TEXT:&str=include_str!("dos_font8x8.txt");
fn rom_font()->Result<[u8;2048],String>{
    let mut lines=ROM_FONT_TEXT.lines().filter(|l|!l.starts_with(';'));
    let mut bytes=[0u8;2048];
    for code in 0..256usize{
        let header=lines.next().ok_or("VGA font ends before glyph FFh")?;
        if usize::from_str_radix(header.get(..2).unwrap_or(""),16)!=Ok(code){
            return Err(format!("VGA ROM glyph header mismatch for {code:02X}h"));
        }
        for r in 0..8{
            let row=lines.next().ok_or("VGA ROM font ends inside a glyph")?;
            if row.len()!=8||!row.bytes().all(|b|b==b'#'||b==b'.'){
                return Err(format!("invalid VGA ROM glyph {code:02X}h row"));
            }
            bytes[code*8+r]=row.bytes().fold(0u8,|n,b|(n<<1)|u8::from(b==b'#'));
        }
    }
    if lines.next().is_some(){return Err("extra VGA ROM glyph rows".into());}
    Ok(bytes)
}

/// Existing DOS PSP tail format: count, leading space, CP437 text, CR.
pub(crate) fn command_tail(ram:&mut Ram,psp_base:u32,text:&str)->Result<(),String>{
    let encoded=crate::dos_encoding::command_line(text)?;
    let mut tail=vec![encoded.len() as u8+1,b' '];tail.extend(encoded);tail.push(0x0d);
    ram.write_bytes(psp_base+0x80,&tail).map_err(|e|format!("{e:?}"))
}

#[allow(dead_code)]
impl Context {
    /// Four visible mode-12h planes in the same scanline order exported by
    /// Dine. The unused tails of VGA's 64 KiB planes are intentionally omitted.
    pub fn visible_vga_planes(&self)->Result<Vec<u8>,String>{
        if self.video.mode!=0x12{return Err("VGA plane capture requires BIOS mode 12h".into());}
        let planar=self.ram.planar().ok_or("VGA planes are not mapped")?;
        let count=80*480;
        let mut bytes=Vec::with_capacity(4*count);
        for n in 0..4{bytes.extend_from_slice(&planar.plane(n)[..count]);}
        Ok(bytes)
    }
    /// Plan DOS console text in mode 12h using the same 8x8 ROM glyphs as
    /// Dine. Each source row is doubled to fit the VGA 8x16 cell, white
    /// foreground (planes 0..2) on black, with the cursor updated only
    /// after an accepted console write.
    fn plan_graphics_text(&self,text:&[u8])->Result<Option<(crate::vga_planar::Planar,VideoState)>,String>{
        if self.video.mode!=0x12{return Ok(None);}
        let font=rom_font()?;
        let mut planar=self.ram.planar().ok_or("standalone VGA text requires mapped planes")?.clone();
        let mut video=self.video;
        for &ch in text{
            let count=if ch==9{8-usize::from(video.cursor_col)%8}else{1};
            for _ in 0..count{
                match if ch==9{b' '}else{ch}{
                    7=>{},
                    8=>video.cursor_col=video.cursor_col.saturating_sub(1),
                    13=>video.cursor_col=0,
                    10=>video.cursor_row=video.cursor_row.saturating_add(1),
                    glyph=>{
                        let row=usize::from(video.cursor_row);
                        let col=usize::from(video.cursor_col);
                        if row>=30||col>=80{return Err("standalone VGA text cursor outside mode 12h".into());}
                        for y in 0..16usize{
                            let at=(row*16+y)*80+col;
                            let bits=font[usize::from(glyph)*8+y/2];
                            for plane in 0..4{
                                planar.poke(plane,at,if plane<3{bits}else{0});
                            }
                        }
                        video.cursor_col+=1;
                        if video.cursor_col==80{video.cursor_col=0;video.cursor_row+=1;}
                    },
                }
                if video.cursor_row>=30{return Err("standalone VGA text scrolling is not implemented".into());}
            }
        }
        Ok(Some((planar,video)))
    }
    /// IN or OUT through DX to the VGA sequencer or graphics controller
    /// (3C4h/3C5h/3CEh/3CFh) in the planar mode, with the same register
    /// rules as Dine (vga_planar::Planar::controller_io), instead of the
    /// decoder's open-bus PIL. IN writes AL or AX; nothing else changes.
    pub fn vga_port(&mut self,input:bool,word:bool)->Result<(),String>{
        let port=self.regs[2] as u16;
        if !self.machine.real_mode{return Err("standalone VGA port access requires real mode".into());}
        let planar=self.ram.planar_mut().ok_or_else(||format!("standalone VGA port {port:04x}h needs a planar BIOS mode"))?;
        let value=planar.controller_io(port,input,self.regs[0] as u16,word)?;
        if input{let mask=if word{0xffff}else{0xff};self.regs[0]=(self.regs[0]&!mask)|(u32::from(value)&mask);}
        Ok(())
    }
    pub fn interrupt(&mut self,vector:u8,out:&mut impl Output)->Result<Option<u8>,String>{
        match (vector,(self.regs[0]>>8)as u8) {
            (0x20,_)=>Ok(Some(0)),
            (0x21,0x4c)=>Ok(Some(self.regs[0] as u8)),
            (0x21,2)=>{
                let (bytes,value)=character_output(self.regs[2]as u8,out)?;
                out.write_atomic(&bytes).map_err(|e|format!("DOS character output provider: {e:?}"))?;
                self.regs[0]=(self.regs[0]&!255)|u32::from(value);Ok(None)
            },
            (0x21,9)=>{
                let context=MemoryContext{segment:SegmentRegister::Ds,cache:self.machine.segments[SegmentRegister::Ds.index()]};
                let text=dollar_string(&self.ram,context,self.regs[2]&0xffff,self.eip,out.remaining())?;
                let graphic=self.plan_graphics_text(&text)?;
                if graphic.is_some(){
                    self.ram.check_write(0x450,MemWidth::Word).map_err(|e|format!("BIOS VGA cursor write: {e:?}"))?;
                }
                out.write_atomic(&text).map_err(|e|format!("DOS string output provider: {e:?}"))?;
                if let Some((planar,video))=graphic{
                    let cursor=u32::from(video.cursor_row)<<8|u32::from(video.cursor_col);
                    self.ram.store(0x450,MemWidth::Word,cursor).map_err(|e|format!("BIOS VGA cursor write: {e:?}"))?;
                    *self.ram.planar_mut().ok_or("VGA planar mode disappeared during text output")?=planar;
                    self.video=video;
                }
                self.regs[0]=(self.regs[0]&!255)|0x24;
                Ok(None)
            },
            (0x16,0)=>{
                let key=self.key.ok_or("standalone INT 16h key wait needs explicit PIL_DOS_BIOS_KEY=1C0D")?;
                // Explicit verifier-only capture at the key wait, while VGA
                // mode and its planar latches are still active. No file is
                // written by default, and a failed write cannot consume key.
                if let Some(path)=std::env::var_os("PIL_DOS_VGA_CAPTURE"){
                    let bytes=self.visible_vga_planes()?;
                    std::fs::write(path,bytes).map_err(|e|format!("native VGA capture: {e}"))?;
                }
                self.key=None;
                self.regs[0]=(self.regs[0]&0xffff0000)|u32::from(key);
                Ok(None)
            },
            (0x10,0)=>{
                let mode=self.regs[0] as u8;
                if !matches!(mode,3|0x12){return Err(format!("standalone BIOS mode {mode:02x} is not supported"));}
                // Match Dine's public VGA BIOS data for mode 12h; validate
                // before changing either RAM or the mapped video device.
                let writes=[
                    (0x449,MemWidth::Byte,u32::from(mode)),
                    (0x44a,MemWidth::Word,80),
                    (0x44c,MemWidth::Word,if mode==0x12{0xa000}else{0x1000}),
                    (0x450,MemWidth::Word,0),
                    (0x462,MemWidth::Byte,0),
                    (0x463,MemWidth::Word,0x3d4),
                    (0x484,MemWidth::Byte,if mode==0x12{29}else{24}),
                    (0x485,MemWidth::Word,16),
                ];
                for &(at,width,_) in &writes{self.ram.check_write(at,width).map_err(|e|format!("BIOS video mode: {e:?}"))?;}
                for &(at,width,value) in &writes{self.ram.store(at,width,value).map_err(|e|format!("BIOS video mode: {e:?}"))?;}
                self.ram.set_planar(if mode==0x12{Some(crate::vga_planar::Planar::new())}else{None});
                self.video=VideoState{mode,..VideoState::default()};
                Ok(None)
            },
            (0x10,2)=>{
                let page=(self.regs[3]>>8)as u8;
                let row=(self.regs[2]>>8)as u8;
                let col=self.regs[2] as u8;
                let rows=if self.video.mode==0x12{30}else{25};
                if page!=0||row>=rows||col>=80{
                    return Err(format!("standalone BIOS cursor page/position unsupported: page {page}, row {row}, col {col}"));
                }
                let value=u32::from(row)<<8|u32::from(col);
                self.ram.store(0x450,MemWidth::Word,value).map_err(|e|format!("BIOS video cursor: {e:?}"))?;
                self.video.cursor_row=row;
                self.video.cursor_col=col;
                Ok(None)
            },
            _=>Err(format!("standalone runtime does not support INT {vector:02x}/AH{:02x}",(self.regs[0]>>8)as u8)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn character_output_returns_al_and_rejects_partial_tabs(){
        let p=crate::dos::load(&[0xcd,0x20],"T.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:0xad7,undefined:0x800,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),key:None,calls:Vec::new()};
        c.regs[0]=0x123402ab;c.regs[2]=9;
        let before=(c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram.clone());
        let mut out=crate::host_io::CaptureOutput::new(7);
        assert!(c.interrupt(0x21,&mut out).is_err());assert!(out.bytes().is_empty());
        assert_eq!((c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram.clone()),before);
        let mut out=crate::host_io::CaptureOutput::new(8);
        c.interrupt(0x21,&mut out).unwrap();assert_eq!(out.bytes(),b"        ");
        assert_eq!(c.regs[0],0x12340220);assert_eq!(c.flags,0xad7);assert_eq!(c.undefined,0x800);
        let regs=c.regs;let mut disconnected=crate::host_io::CaptureOutput::new(8);disconnected.disconnect();
        assert!(c.interrupt(0x21,&mut disconnected).is_err());assert_eq!(c.regs,regs);
    }
    #[test]
    fn command_tail_bounds_and_validation_are_atomic(){
        let mut ram=Ram::new(0x200);ram.protect(0x100,0x100).unwrap();
        command_tail(&mut ram,0,&"x".repeat(125)).unwrap();
        assert_eq!(ram.load(0x80,MemWidth::Byte).unwrap(),126);
        assert_eq!(ram.load(0xff,MemWidth::Byte).unwrap(),13);
        let before=ram.clone();
        for text in [&"x".repeat(126),"bad\nline","🙂"]{assert!(command_tail(&mut ram,0,text).is_err());assert_eq!(ram,before);}
        ram.protect(0x82,1).unwrap();let before=ram.clone();
        assert!(command_tail(&mut ram,0,"abc").is_err());assert_eq!(ram,before);
    }
    #[test]
    fn video_bios_mode_cursor_and_rejections_preserve_cpu_state(){
        let p=crate::dos::load(&[0xcd,0x20],"T.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:0x246,undefined:0,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),key:None,calls:Vec::new()};
        let mut out=crate::host_io::CaptureOutput::new(32);
        c.regs[0]=0x12;
        let before=(c.regs,c.flags,c.eip);
        assert_eq!(c.interrupt(0x10,&mut out).unwrap(),None);
        assert_eq!((c.regs,c.flags,c.eip),before);
        assert_eq!(c.video.mode,0x12);
        c.regs[0]=0x0200;c.regs[3]=0;c.regs[2]=0x1d00;
        c.interrupt(0x10,&mut out).unwrap();
        assert_eq!((c.video.cursor_row,c.video.cursor_col),(29,0));
        let video=c.video;
        c.regs[3]=0x100;
        assert!(c.interrupt(0x10,&mut out).is_err());assert_eq!(c.video,video);
        c.regs[0]=0x0013;
        assert!(c.interrupt(0x10,&mut out).is_err());assert_eq!(c.video,video);
        c.regs[0]=3;
        c.interrupt(0x10,&mut out).unwrap();
        assert_eq!(c.video,VideoState::default());
        assert!(out.bytes().is_empty());
    }
    #[test]
    fn vga_ports_use_shared_controller_rules_latches_and_mask(){
        let p=crate::dos::load(&[0xcd,0x20],"V.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:0x246,undefined:0,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),key:None,calls:Vec::new()};
        let mut output=crate::host_io::CaptureOutput::new(64);
        c.regs[0]=0x12;c.interrupt(0x10,&mut output).unwrap();
        assert_eq!(c.ram.load(0x449,MemWidth::Byte).unwrap(),0x12);
        assert!(c.ram.planar().is_some());
        let at=crate::vga_planar::WINDOW+7;
        c.ram.store(at,MemWidth::Byte,0xff).unwrap();
        c.regs[2]=0x3ce;
        c.regs[0]=0x0205;c.vga_port(false,true).unwrap();
        c.regs[0]=0xc008;c.vga_port(false,true).unwrap();
        assert_eq!(c.ram.load(at,MemWidth::Byte).unwrap(),0xff); // load latches
        c.ram.store(at,MemWidth::Byte,6).unwrap();
        let p=c.ram.planar().unwrap();
        assert_eq!([p.peek(0,7),p.peek(1,7),p.peek(2,7),p.peek(3,7)],[0x3f,0xff,0xff,0x3f]);
        let image=c.visible_vga_planes().unwrap();
        assert_eq!(image.len(),4*80*480);
        for (n,want) in [0x3f,0xff,0xff,0x3f].into_iter().enumerate(){
            assert_eq!(image[n*80*480+7],want);
        }
        let before=c.ram.planar().unwrap().clone();
        c.regs[2]=0x3cf;assert!(c.vga_port(false,true).is_err());
        c.regs[2]=0x3ce;c.regs[0]=0x1106;assert!(c.vga_port(false,true).is_err());
        c.regs[2]=0x3d4;c.regs[0]=0x0005;assert!(c.vga_port(false,false).is_err());
        assert_eq!(c.ram.planar().unwrap(),&before);
        // Byte accesses: select bit mask (08h) at 3CEh, read it back at 3CFh
        // into AL only; the sequencer map mask round-trips the same way.
        c.regs[2]=0x3ce;c.regs[0]=0x1234_0008;c.vga_port(false,false).unwrap();
        c.regs[2]=0x3cf;c.vga_port(true,false).unwrap();
        assert_eq!(c.regs[0],0x1234_00c0);
        c.regs[2]=0x3c4;c.regs[0]=0x0502;c.vga_port(false,true).unwrap();
        c.regs[2]=0x3c5;c.vga_port(true,false).unwrap();
        assert_eq!(c.regs[0]&0xff,5);
        c.regs[0]=3;c.interrupt(0x10,&mut output).unwrap();
        assert!(c.ram.planar().is_none());
        assert_eq!(c.ram.load(0x449,MemWidth::Byte).unwrap(),3);
        c.regs[2]=0x3ce;assert!(c.vga_port(false,true).is_err());
    }
    #[test]
    fn bios_key_wait_needs_one_explicit_key_and_consumes_it_once(){
        let p=crate::dos::load(&[0xcd,0x20],"KEY.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:0x246,undefined:0,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),key:None,calls:Vec::new()};
        let mut out=crate::host_io::CaptureOutput::new(32);
        c.regs[0]=0;
        let before=(c.regs,c.flags,c.eip);
        assert!(c.interrupt(0x16,&mut out).is_err());
        assert_eq!((c.regs,c.flags,c.eip),before);
        assert_eq!(parse_key("1C0D").unwrap(),0x1c0d);
        assert_eq!(parse_key("0x1C0D").unwrap(),0x1c0d);
        assert!(parse_key("garbage").is_err());
        c.key=Some(0x1c0d);
        c.interrupt(0x16,&mut out).unwrap();
        assert_eq!(c.regs[0]&0xffff,0x1c0d);
        assert_eq!(c.flags,0x246);
        assert_eq!(c.key,None);
        assert!(c.interrupt(0x16,&mut out).is_err());
        assert!(out.bytes().is_empty());
    }
    #[test]
    fn mode12_graphics_caption_uses_rom_glyph_doubled_into_four_planes(){
        let p=crate::dos::load(&[0xcd,0x20],"CAP.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:0,undefined:0,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),key:None,calls:Vec::new()};
        let mut out=crate::host_io::CaptureOutput::new(64);
        c.regs[0]=0x12;c.interrupt(0x10,&mut out).unwrap();
        c.regs[0]=0x0200;c.regs[2]=0x1d00;c.interrupt(0x10,&mut out).unwrap();
        let (planar,video)=c.plan_graphics_text(b"A").unwrap().unwrap();
        let font=rom_font().unwrap();
        assert_eq!((video.cursor_row,video.cursor_col),(29,1));
        for y in 0..16usize{
            let at=(29*16+y)*80;
            for n in 0..4{
                assert_eq!(planar.peek(n,at),if n<3{font[65*8+y/2]}else{0});
            }
        }
        assert_eq!((c.video.cursor_row,c.video.cursor_col),(29,0),"planning must not mutate cursor");
        assert!(c.plan_graphics_text(&vec![b'A';81]).is_err(),"text beyond bottom row requires scrolling");
    }
    #[test]
    fn rejected_string_output_preserves_registers_and_output(){
        let p=crate::dos::load(&crate::dos::hello("com").unwrap(),"HELLO.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:p.cpu.eflags,undefined:p.cpu.undefined_flags,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),key:None,calls:Vec::new()};
        c.regs[0]=0x123409ab;c.regs[2]=0x10c;
        let before=c.regs;
        let mut out=crate::host_io::CaptureOutput::new(2);
        assert!(c.interrupt(0x21,&mut out).is_err());
        assert_eq!(c.regs,before);assert!(out.bytes().is_empty());
        let mut out=crate::host_io::CaptureOutput::new(13);
        assert_eq!(c.interrupt(0x21,&mut out).unwrap(),None);
        assert_eq!(out.bytes(),b"Hello, DOS!\r\n");assert_eq!(c.regs[0],0x12340924);
    }
}
