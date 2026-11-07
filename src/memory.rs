//! Flat RAM, no segmentation, page tables or guest #PF delivery. The one
//! memory-mapped device is optional EGA/VGA planar memory (vga_planar) behind
//! A0000h-AFFFFh, present only while a 16-colour planar mode is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemWidth { Byte, Word, Dword }
impl MemWidth {
    pub fn from_bits(bits:u8)->Option<Self> {
        match bits {8=>Some(Self::Byte),16=>Some(Self::Word),32=>Some(Self::Dword),_=>None}
    }
    pub fn bits(self)->u8 {self.bytes()*8}
    pub fn bytes(self)->u8 {match self {Self::Byte=>1,Self::Word=>2,Self::Dword=>4}}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessFault {pub address:u32,pub bytes:usize,pub write:bool}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ram {
    data:Vec<u8>,
    readonly:Vec<std::ops::Range<usize>>,
    a20_enabled:bool,
    planar:Option<Box<super::vga_planar::Planar>>,
}
/// Host-only alias of the four planes (plane n at PLANE_ALIAS + n*10000h),
/// for BIOS services that write pixels without the graphics controller. Real-
/// mode guest addresses end at 10FFEFh, so guests cannot reach it.
pub const PLANE_ALIAS:u32=0xf000_0000;
impl Ram {
    pub fn new(bytes:usize)->Self {Self{data:vec![0;bytes],readonly:Vec::new(),a20_enabled:true,planar:None}}
    pub fn bytes(&self)->&[u8] {&self.data}
    pub fn a20_enabled(&self)->bool{self.a20_enabled}
    pub fn planar(&self)->Option<&super::vga_planar::Planar>{self.planar.as_deref()}
    pub fn planar_mut(&mut self)->Option<&mut super::vga_planar::Planar>{self.planar.as_deref_mut()}
    pub fn set_planar(&mut self,planar:Option<super::vga_planar::Planar>){self.planar=planar.map(Box::new);}
    /// Plane offset of a physical byte in the planar window, while one is mapped.
    fn window(&self,index:usize)->Option<usize>{
        let start=super::vga_planar::WINDOW as usize;
        if self.planar.is_some()&&(start..start+super::vga_planar::PLANE).contains(&index){Some(index-start)}else{None}
    }
    fn alias(&self,address:u32,bytes:usize)->Option<usize>{
        let offset=address.checked_sub(PLANE_ALIAS)? as usize;
        (self.planar.is_some()&&offset+bytes<=4*super::vga_planar::PLANE).then_some(offset)
    }
    pub fn set_a20_enabled(&mut self,enabled:bool){self.a20_enabled=enabled;}
    pub fn physical_address(&self,address:u32)->u32{if self.a20_enabled{address}else{address&!0x10_0000}}
    /// A contiguous bus span, or None when it crosses an A20 boundary.
    pub fn contiguous_slice(&self,address:u32,bytes:usize)->Option<&[u8]>{
        if !self.a20_enabled&&bytes>0x100000-(address as usize&0xfffff){return None;}
        let start=self.physical_address(address)as usize;let end=start.checked_add(bytes)?;self.data.get(start..end)
    }
    fn range(&self,address:u32,bytes:usize,write:bool)->Result<std::ops::Range<usize>,AccessFault> {
        let fault=AccessFault{address,bytes,write};
        let start=usize::try_from(address).map_err(|_|fault)?;
        let end=start.checked_add(bytes).ok_or(fault)?;
        if end>self.data.len() {return Err(fault);}
        // A 32-bit guest access cannot wrap to the beginning of RAM.
        if (address as u64)+(bytes as u64)>0x1_0000_0000 {return Err(fault);}
        if write && bytes!=0 && self.readonly.iter().any(|r|start<r.end && end>r.start) {return Err(fault);}
        Ok(start..end)
    }
    pub fn protect(&mut self,address:u32,bytes:usize)->Result<(),AccessFault> {
        let range=self.range(address,bytes,false)?;
        if !range.is_empty() {self.readonly.push(range);}
        Ok(())
    }
    pub fn check_write(&self,address:u32,width:MemWidth)->Result<(),AccessFault> {
        self.check_bus(address,width.bytes()as usize,true)
    }
    fn check_bus(&self,address:u32,bytes:usize,write:bool)->Result<(),AccessFault>{
        let fault=AccessFault{address,bytes,write};
        if self.alias(address,bytes).is_some(){return Ok(());}
        if u64::from(address).checked_add(bytes as u64).is_none_or(|end|end>0x1_0000_0000){return Err(fault);}
        let mut done=0usize;
        while done<bytes{
            let guest=address+done as u32;
            let chunk=if self.a20_enabled{bytes-done}else{(bytes-done).min(0x100000-(guest as usize&0xfffff))};
            self.range(self.physical_address(guest),chunk,write).map_err(|_|fault)?;done+=chunk;
        }
        if bytes==0{self.range(self.physical_address(address),0,write).map_err(|_|fault)?;}
        Ok(())
    }
    pub fn load(&self,address:u32,width:MemWidth)->Result<u32,AccessFault> {
        let bytes=width.bytes()as usize;self.check_bus(address,bytes,false)?;
        let mut value=0u32;
        if let Some(offset)=self.alias(address,bytes){
            let p=self.planar.as_deref().unwrap();
            for shift in 0..bytes{let at=offset+shift;value|=u32::from(p.peek(at/super::vga_planar::PLANE,at%super::vga_planar::PLANE))<<(shift*8);}
            return Ok(value);
        }
        for shift in 0..bytes {
            let index=self.physical_address(address+shift as u32)as usize;
            let byte=match self.window(index){Some(at)=>self.planar.as_deref().unwrap().read(at),None=>self.data[index]};
            value|=u32::from(byte)<<(shift*8);
        }
        Ok(value)
    }
    pub fn store(&mut self,address:u32,width:MemWidth,value:u32)->Result<(),AccessFault> {
        let bytes=width.bytes()as usize;self.check_bus(address,bytes,true)?;
        if let Some(offset)=self.alias(address,bytes){
            let p=self.planar.as_deref_mut().unwrap();
            for shift in 0..bytes{let at=offset+shift;p.poke(at/super::vga_planar::PLANE,at%super::vga_planar::PLANE,(value>>(shift*8))as u8);}
            return Ok(());
        }
        for shift in 0..bytes {
            let index=self.physical_address(address+shift as u32)as usize;let byte=(value>>(shift*8)) as u8;
            match self.window(index){Some(at)=>self.planar.as_deref_mut().unwrap().write(at,byte),None=>self.data[index]=byte}
        }
        Ok(())
    }
    // Debugger bulk writes have the same full-range permission check.
    pub fn write_bytes(&mut self,address:u32,data:&[u8])->Result<(),AccessFault> {
        self.check_bus(address,data.len(),true)?;
        let mut done=0;while done<data.len(){let guest=address+done as u32;let chunk=if self.a20_enabled{data.len()-done}else{(data.len()-done).min(0x100000-(guest as usize&0xfffff))};let start=self.physical_address(guest)as usize;self.data[start..start+chunk].copy_from_slice(&data[done..done+chunk]);done+=chunk;}Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/memory.rs"]
mod tests;
