//! Host output provider boundary. A rejected write must have no visible effects.
//! This is DOS compatibility output, not CPU port I/O or a BIOS screen device.
pub const DOS_OUTPUT_LIMIT:usize=16384;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum OutputError { Full, Disconnected }
pub trait Output {
    fn bytes(&self)->&[u8];
    fn capacity(&self)->usize;
    fn write_atomic(&mut self,bytes:&[u8])->Result<(),OutputError>;
    fn remaining(&self)->usize {self.capacity().saturating_sub(self.bytes().len())}
    fn column(&self)->usize {
        let mut col=0usize;
        for &b in self.bytes(){match b{b'\r'=>col=0,b'\n'|7=>{},8=>col=col.saturating_sub(1),9=>col+=8-col%8,_=>col+=1}}
        col
    }
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct CaptureOutput {bytes:Vec<u8>,capacity:usize,connected:bool}
impl CaptureOutput {
    pub fn new(capacity:usize)->Self {Self{bytes:Vec::new(),capacity,connected:true}}
    pub fn into_bytes(self)->Vec<u8>{self.bytes}
    // Explicit host lifecycle, not a synthetic guest EOF/input result.
    #[allow(dead_code)]
    pub fn disconnect(&mut self){self.connected=false;}
}
impl Output for CaptureOutput {
    fn bytes(&self)->&[u8]{&self.bytes}
    fn capacity(&self)->usize{self.capacity}
    fn write_atomic(&mut self,bytes:&[u8])->Result<(),OutputError>{
        if !self.connected{return Err(OutputError::Disconnected);}
        if bytes.len()>self.remaining(){return Err(OutputError::Full);}
        self.bytes.extend_from_slice(bytes);Ok(())
    }
}
// Keep the existing literal service fixtures against a simple bounded sink.
#[cfg(test)]
impl Output for Vec<u8> {
    fn bytes(&self)->&[u8]{self}
    fn capacity(&self)->usize{DOS_OUTPUT_LIMIT}
    fn write_atomic(&mut self,bytes:&[u8])->Result<(),OutputError>{
        if bytes.len()>self.remaining(){return Err(OutputError::Full);}
        self.extend_from_slice(bytes);Ok(())
    }
}
#[cfg(test)]
#[path = "../tests/unit/host_io.rs"]
mod tests;
