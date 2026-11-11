//! Minimal standalone DOS boundary, independent of the decoder and interpreter.
//! This headless subset has no files, screen, keyboard, installed handlers or EXEC.
use crate::{address::{MemoryContext,SegmentRegister,SegmentState},host_io::Output,MemWidth,Ram};

// The compiler embeds this context and its methods into generated applications.
#[allow(dead_code)]
pub struct Context {
    pub regs:[u32;8], pub flags:u32, pub undefined:u32, pub eip:u32,
    pub ram:Ram, pub machine:SegmentState,
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
        let mut c=Context{regs:p.cpu.regs,flags:0xad7,undefined:0x800,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),calls:Vec::new()};
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
    fn rejected_string_output_preserves_registers_and_output(){
        let p=crate::dos::load(&crate::dos::hello("com").unwrap(),"HELLO.COM").unwrap();
        let mut c=Context{regs:p.cpu.regs,flags:p.cpu.eflags,undefined:p.cpu.undefined_flags,eip:p.cpu.eip,ram:p.ram,machine:p.image.segment_state(),calls:Vec::new()};
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
