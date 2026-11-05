//! Text CRTC device, initially 80x25 color text. No planar framebuffer yet.
#[derive(Clone,Debug)]
pub(super) struct Vga { index:u8, regs:[u8;25], pub start_override:bool, pub cursor_override:bool }
impl Default for Vga {fn default()->Self{Self{index:0,regs:[0x5f,0x4f,0x50,0x82,0x55,0x81,0xbf,0x1f,0,0x4f,6,7,0,0,0,0,0x9c,0x8e,0x8f,0x28,0x1f,0x96,0xb9,0xa3,0xff],start_override:false,cursor_override:false}}}
impl Vga {
    pub fn start(&self)->u16{u16::from_be_bytes([self.regs[12],self.regs[13]])}
    pub fn cursor(&self)->u16{u16::from_be_bytes([self.regs[14],self.regs[15]])}
    pub fn io(&mut self,port:u16,input:bool,value:u8)->Result<Option<u8>,String>{
        if !matches!(port,0x3d4|0x3d5){return Ok(None);}
        if port==0x3d4 {if input{return Ok(Some(self.index));}self.index=value&0x1f;return Ok(Some(0));}
        let i=self.index as usize;
        if i>=self.regs.len(){return if input{Ok(Some(0xff))}else{Err(format!("VGA CRTC register {:02X} is outside the text profile",self.index))};}
        if input{return Ok(Some(self.regs[i]));}
        if self.regs[17]&0x80!=0&&i<=7{
            if i==7{self.regs[7]=(self.regs[7]&!0x10)|(value&0x10);}
            return Ok(Some(0));
        }
        // Cursor and start-address changes have visible text-mode effects.
        // Accept the protection bit without pretending timing/mode changes work.
        if i==17 {if value&0x7f!=self.regs[i]&0x7f{return Err("VGA retrace timing changes are not implemented".into());}}
        else if !(10..=15).contains(&i)&&value!=self.regs[i]{return Err(format!("VGA CRTC timing/mode register {i:02X} change is not implemented"));}
        self.regs[i]=value;
        if matches!(i,12|13){self.start_override=true;}
        if matches!(i,14|15){self.cursor_override=true;}
        Ok(Some(0))
    }
    pub fn bios_sync(&mut self,start:Option<u16>,cursor:Option<u16>,shape:Option<u16>){
        if let Some(v)=start{let [hi,lo]=v.to_be_bytes();self.regs[12]=hi;self.regs[13]=lo;self.start_override=false;}
        if let Some(v)=cursor{let [hi,lo]=v.to_be_bytes();self.regs[14]=hi;self.regs[15]=lo;self.cursor_override=false;}
        if let Some(v)=shape{let [hi,lo]=v.to_be_bytes();self.regs[10]=hi;self.regs[11]=lo;}
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    fn write(v:&mut Vga,i:u8,x:u8){v.io(0x3d4,false,i).unwrap();v.io(0x3d5,false,x).unwrap();}
    #[test]fn crtc_roundtrip_cursor_start_and_bios_sync(){let mut v=Vga::default();write(&mut v,12,0x12);write(&mut v,13,0x34);write(&mut v,14,0x56);write(&mut v,15,0x78);assert_eq!((v.start(),v.cursor()),(0x1234,0x5678));assert_eq!(v.io(0x3d5,true,0).unwrap(),Some(0x78));v.bios_sync(Some(0),Some(81),Some(0x607));assert_eq!(v.cursor(),81);assert!(!v.start_override&&!v.cursor_override);}
    #[test]fn locked_timing_and_unimplemented_mode_changes(){let mut v=Vga::default();write(&mut v,0,0);assert_eq!(v.regs[0],0x5f);write(&mut v,17,0x0e);v.io(0x3d4,false,0).unwrap();assert!(v.io(0x3d5,false,0).is_err());assert_eq!(v.regs[0],0x5f);}
}
