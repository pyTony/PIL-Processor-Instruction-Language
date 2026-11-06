//! Text CRTC device, initially 80x25 color text. No planar framebuffer yet.
// Conventional VGA mode 13h palette: 16 EGA colours, 16 greys and nine
// 24-colour hue rings. Components are 6-bit DAC values.
fn default_palette()->[[u8;3];256]{
    let mut p=[[0u8;3];256];
    for i in 0..16usize {
        let bright=if i&8!=0{21}else{0};
        let c=|bit|if i&bit!=0{42+bright}else{bright};
        p[i]=[c(4),if i==6{21}else{c(2)},c(1)];
    }
    let grey=[0,5,8,11,14,17,20,24,28,32,36,40,45,50,56,63];
    for (i,&v) in grey.iter().enumerate(){p[16+i]=[v;3];}
    let rings=[[0,16,31,47,63],[31,39,47,55,63],[45,49,54,58,63],
        [0,7,14,21,28],[14,17,21,24,28],[20,22,24,26,28],
        [0,4,8,12,16],[8,10,12,14,16],[11,12,13,15,16]];
    let mut n=32;
    for a in rings {
        let [lo,_,_,_,hi]=a;
        for k in 0..24 {
            let rgb=match k/4 {
                0=>[a[k%4],lo,hi],1=>[hi,lo,a[4-k%4]],
                2=>[hi,a[k%4],lo],3=>[a[4-k%4],hi,lo],
                4=>[lo,hi,a[k%4]],_=>[lo,a[4-k%4],hi],
            };p[n]=rgb;n+=1;
        }
    }
    p
}
#[derive(Clone,Debug)]
pub(super) struct Vga { cga_color:u8, dac:[[u8;3];256], dac_mask:u8, dac_read:u8, dac_write:u8, read_component:u8, write_component:u8, dac_read_mode:bool, index:u8, misc_output:u8, regs:[u8;25], pub start_override:bool, pub cursor_override:bool }
impl Default for Vga {fn default()->Self{Self{cga_color:0x30,dac:default_palette(),dac_mask:255,dac_read:0,dac_write:0,read_component:0,write_component:0,dac_read_mode:false,index:0,misc_output:0x67,regs:[0x5f,0x4f,0x50,0x82,0x55,0x81,0xbf,0x1f,0,0x4f,6,7,0,0,0,0,0x9c,0x8e,0x8f,0x28,0x1f,0x96,0xb9,0xa3,0xff],start_override:false,cursor_override:false}}}
impl Vga {
    pub fn reset_palette(&mut self){self.dac=default_palette();self.dac_mask=255;}
    pub fn palette_hex(&self)->String{
        (0..256).flat_map(|i|self.dac[i & usize::from(self.dac_mask)])
            .map(|c|format!("{:02x}",(u16::from(c)*255/63)as u8)).collect()
    }
    pub fn cga_color(&self)->u8{self.cga_color}
    pub fn set_cga_color(&mut self,value:u8){self.cga_color=value&0x3f;}
    pub fn color_io(&self)->bool{self.misc_output&1!=0}
    pub fn status_active(&self,port:u16)->bool{port==if self.color_io(){0x3da}else{0x3ba}}
    pub fn select_color(&mut self){self.misc_output|=1;}
    pub fn start(&self)->u16{u16::from_be_bytes([self.regs[12],self.regs[13]])}
    pub fn cursor(&self)->u16{u16::from_be_bytes([self.regs[14],self.regs[15]])}
    pub fn io(&mut self,port:u16,input:bool,value:u8)->Result<Option<u8>,String>{
        match (port,input) {
            (0x3d9,false)=>{self.set_cga_color(value);return Ok(Some(0));},
            (0x3d9,true)=>return Ok(Some(0xff)),
            (0x3c6,true)=>return Ok(Some(self.dac_mask)),
            (0x3c6,false)=>{self.dac_mask=value;return Ok(Some(0));},
            (0x3c7,true)=>return Ok(Some(if self.dac_read_mode{3}else{0})),
            (0x3c7,false)=>{self.dac_read=value;self.read_component=0;self.dac_read_mode=true;return Ok(Some(0));},
            (0x3c8,true)=>return Ok(Some(self.dac_write)),
            (0x3c8,false)=>{self.dac_write=value;self.write_component=0;self.dac_read_mode=false;return Ok(Some(0));},
            (0x3c9,true)=>{let x=self.dac[self.dac_read as usize][self.read_component as usize];self.read_component+=1;if self.read_component==3{self.read_component=0;self.dac_read=self.dac_read.wrapping_add(1);}return Ok(Some(x));},
            (0x3c9,false)=>{self.dac[self.dac_write as usize][self.write_component as usize]=value&63;self.write_component+=1;if self.write_component==3{self.write_component=0;self.dac_write=self.dac_write.wrapping_add(1);}return Ok(Some(0));},
            _=>{},
        }
        if port==0x3cc&&input{return Ok(Some(self.misc_output));}
        if port==0x3c2&&!input{
            if value&!1!=self.misc_output&!1{return Err("VGA miscellaneous clock/memory/polarity changes are not implemented".into());}
            self.misc_output=value;return Ok(Some(0));
        }
        if !matches!(port,0x3b4|0x3b5|0x3d4|0x3d5){return Ok(None);}
        // One VGA CRTC is decoded at the selected bank. Inactive writes have no
        // effect; inactive reads return the profile's unconnected-bus value.
        if (port>=0x3d0)!=self.color_io(){return Ok(Some(if input{0xff}else{0}));}
        if port&1==0 {if input{return Ok(Some(self.index));}self.index=value&0x1f;return Ok(Some(0));}
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
    #[test]fn dac_components_mask_and_wrap(){
        let mut v=Vga::default();
        v.io(0x3c8,false,255).unwrap();
        for x in [63,32,1,7,8,9]{v.io(0x3c9,false,x).unwrap();}
        assert_eq!(v.io(0x3c8,true,0).unwrap(),Some(1));
        v.io(0x3c7,false,255).unwrap();
        for x in [63,32,1,7,8,9]{assert_eq!(v.io(0x3c9,true,0).unwrap(),Some(x));}
        v.io(0x3c6,false,0).unwrap();
        assert_eq!(&v.palette_hex()[..6],"1c2024");
        assert_eq!(v.palette_hex().len(),1536);
    }
    #[test]fn default_mode13_palette_has_standard_colours(){
        let v=Vga::default();assert_eq!(v.dac[1],[0,0,42]);assert_eq!(v.dac[6],[42,21,0]);
        assert_eq!(v.dac[31],[63,63,63]);assert_eq!(v.dac[32],[0,0,63]);assert_eq!(v.dac[248],[0,0,0]);
    }
    #[test]fn crtc_roundtrip_cursor_start_and_bios_sync(){let mut v=Vga::default();write(&mut v,12,0x12);write(&mut v,13,0x34);write(&mut v,14,0x56);write(&mut v,15,0x78);assert_eq!((v.start(),v.cursor()),(0x1234,0x5678));assert_eq!(v.io(0x3d5,true,0).unwrap(),Some(0x78));v.bios_sync(Some(0),Some(81),Some(0x607));assert_eq!(v.cursor(),81);assert!(!v.start_override&&!v.cursor_override);}
    #[test]fn inactive_mono_probe_does_not_change_color_controller(){
        let mut v=Vga::default();write(&mut v,14,0x12);let before=v.clone();
        v.io(0x3b4,false,15).unwrap();v.io(0x3b5,false,0x55).unwrap();
        assert_eq!(v.io(0x3b4,true,0).unwrap(),Some(0xff));assert_eq!(v.io(0x3b5,true,0).unwrap(),Some(0xff));
        assert_eq!(v.index,before.index);assert_eq!(v.regs,before.regs);assert!(v.status_active(0x3da));assert!(!v.status_active(0x3ba));
    }
    #[test]fn io_bank_selection_retains_the_same_crtc(){
        let mut v=Vga::default();write(&mut v,15,0x42);v.io(0x3c2,false,0x66).unwrap();
        assert_eq!(v.io(0x3cc,true,0).unwrap(),Some(0x66));assert_eq!(v.io(0x3b5,true,0).unwrap(),Some(0x42));
        v.io(0x3d4,false,14).unwrap();assert_eq!(v.io(0x3d4,true,0).unwrap(),Some(0xff));
        v.io(0x3b5,false,0x43).unwrap();v.io(0x3c2,false,0x67).unwrap();assert_eq!(v.io(0x3d5,true,0).unwrap(),Some(0x43));
        let before=v.misc_output;assert!(v.io(0x3c2,false,0).is_err());assert_eq!(v.misc_output,before);
    }
    #[test]fn locked_timing_and_unimplemented_mode_changes(){let mut v=Vga::default();write(&mut v,0,0);assert_eq!(v.regs[0],0x5f);write(&mut v,17,0x0e);v.io(0x3d4,false,0).unwrap();assert!(v.io(0x3d5,false,0).is_err());assert_eq!(v.regs[0],0x5f);}
}

