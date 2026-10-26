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
}
impl Ram {
    pub fn new(bytes:usize)->Self {Self{data:vec![0;bytes],readonly:Vec::new()}}
    pub fn bytes(&self)->&[u8] {&self.data}
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
        self.range(address,width.bytes() as usize,true).map(|_|())
    }
    pub fn load(&self,address:u32,width:MemWidth)->Result<u32,AccessFault> {
        let range=self.range(address,width.bytes() as usize,false)?;
        let mut value=0u32;
        for (shift,byte) in self.data[range].iter().enumerate() {value|=u32::from(*byte)<<(shift*8);}
        Ok(value)
    }
    pub fn store(&mut self,address:u32,width:MemWidth,value:u32)->Result<(),AccessFault> {
        let range=self.range(address,width.bytes() as usize,true)?;
        for (shift,index) in range.enumerate() {self.data[index]=(value>>(shift*8)) as u8;}
        Ok(())
    }
    // Debugger bulk writes have the same full-range permission check.
    pub fn write_bytes(&mut self,address:u32,data:&[u8])->Result<(),AccessFault> {
        let range=self.range(address,data.len(),true)?;
        self.data[range].copy_from_slice(data);Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/memory.rs"]
mod tests;
