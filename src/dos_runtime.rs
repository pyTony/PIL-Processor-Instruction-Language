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

/// Existing DOS PSP tail format: count, leading space, CP437 text, CR.
pub(crate) fn command_tail(ram:&mut Ram,psp_base:u32,text:&str)->Result<(),String>{
    let encoded=crate::dos_encoding::command_line(text)?;
    let mut tail=vec![encoded.len() as u8+1,b' '];tail.extend(encoded);tail.push(0x0d);
    ram.write_bytes(psp_base+0x80,&tail).map_err(|e|format!("{e:?}"))
}

#[allow(dead_code)]
impl Context {
    /// Native boundary for the precisely proven VGA OUT DX,AX at port 3CEh.
    /// Write the graphics-controller index followed by its data atomically,
    /// without using the decoder's generic open-bus PIL instruction.
    pub fn out_vga_word(&mut self)->Result<(),String>{
        let port=self.regs[2] as u16;
        if self.video.mode!=0x12 || !self.machine.real_mode || port!=0x3ce {
            return Err(format!("standalone VGA OUT DX,AX requires mode 12h and DX=03CEh, got port {port:04x}h"));
        }
        let index=(self.regs[0]&255)as usize;
        let value=(self.regs[0]>>8)as u8;
        let planar=self.ram.planar_mut().ok_or("standalone VGA planes are not mapped")?;
        // Exactly the GC write-mode and bit-mask registers used by MANDVGA.
        // Extend only with matching Dine controller validation and tests.
        let allowed=match index{5=>value&0x74==0,8=>true,_=>false};
        if !allowed{return Err(format!("standalone VGA GC register {index:02x}h value {value:02x}h is not supported"));}
        let mut next=planar.clone();
        next.gc_index=index as u8;
        next.gc[index]=value;
        *planar=next;
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
                out.write_atomic(&text).map_err(|e|format!("DOS string output provider: {e:?}"))?;
                self.regs[0]=(self.regs[0]&!255)|0x24;
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
        let mut c=Context{regs:p.cpu.regs,flags:0xad7,undefined:0x800,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),calls:Vec::new()};
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
        let mut c=Context{regs:p.cpu.regs,flags:0x246,undefined:0,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),calls:Vec::new()};
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
    fn vga_out_word_uses_shared_plane_latches_mask_and_rejects_unsupported_ports(){
        let p=crate::dos::load(&[0xcd,0x20],"V.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:0x246,undefined:0,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),calls:Vec::new()};
        let mut output=crate::host_io::CaptureOutput::new(64);
        c.regs[0]=0x12;c.interrupt(0x10,&mut output).unwrap();
        assert_eq!(c.ram.load(0x449,MemWidth::Byte).unwrap(),0x12);
        assert!(c.ram.planar().is_some());
        let at=crate::vga_planar::WINDOW+7;
        c.ram.store(at,MemWidth::Byte,0xff).unwrap();
        c.regs[2]=0x3ce;
        c.regs[0]=0x0205;c.out_vga_word().unwrap();
        c.regs[0]=0xc008;c.out_vga_word().unwrap();
        assert_eq!(c.ram.load(at,MemWidth::Byte).unwrap(),0xff); // load latches
        c.ram.store(at,MemWidth::Byte,6).unwrap();
        let p=c.ram.planar().unwrap();
        assert_eq!([p.peek(0,7),p.peek(1,7),p.peek(2,7),p.peek(3,7)],[0x3f,0xff,0xff,0x3f]);
        let before=c.ram.planar().unwrap().clone();
        c.regs[2]=0x3cf;assert!(c.out_vga_word().is_err());
        c.regs[2]=0x3ce;c.regs[0]=0x1106;assert!(c.out_vga_word().is_err());
        assert_eq!(c.ram.planar().unwrap(),&before);
        c.regs[0]=3;c.interrupt(0x10,&mut output).unwrap();
        assert!(c.ram.planar().is_none());
        assert_eq!(c.ram.load(0x449,MemWidth::Byte).unwrap(),3);
    }
    #[test]
    fn rejected_string_output_preserves_registers_and_output(){
        let p=crate::dos::load(&crate::dos::hello("com").unwrap(),"HELLO.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:p.cpu.eflags,undefined:p.cpu.undefined_flags,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),video:VideoState::default(),calls:Vec::new()};
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
