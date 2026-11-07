//! Flat RAM only: no MMIO, segmentation, page tables or guest #PF delivery.
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
}
impl Ram {
    pub fn new(bytes:usize)->Self {Self{data:vec![0;bytes],readonly:Vec::new(),a20_enabled:true}}
    pub fn bytes(&self)->&[u8] {&self.data}
    pub fn a20_enabled(&self)->bool{self.a20_enabled}
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
        for shift in 0..bytes {value|=u32::from(self.data[self.physical_address(address+shift as u32)as usize])<<(shift*8);}
        Ok(value)
    }
    pub fn store(&mut self,address:u32,width:MemWidth,value:u32)->Result<(),AccessFault> {
        let bytes=width.bytes()as usize;self.check_bus(address,bytes,true)?;
        for shift in 0..bytes {let index=self.physical_address(address+shift as u32)as usize;self.data[index]=(value>>(shift*8)) as u8;}
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
