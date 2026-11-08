// EGA/VGA planar display memory: four 64 KiB bit planes behind the A0000h-
// AFFFFh window, with the sequencer map mask and the graphics controller's
// latches, set/reset, rotate, logical function, bit mask and read/write modes.
// Ram routes guest bytes in the window here only while a planar mode is set;
// otherwise the window is ordinary RAM (mode 13h stays linear). Chain-4 and
// odd/even addressing are not modelled. A read loads the four latches, as on
// the hardware, even if the instruction later faults; nothing else is staged.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Planar {
    planes:Vec<u8>,
    latch:std::cell::Cell<[u8;4]>,
    /// Sequencer registers 0-4 (2 map mask, 4 memory mode).
    pub seq:[u8;5],
    /// Graphics controller registers 0-8.
    pub gc:[u8;9],
    /// Selected sequencer and graphics controller index registers.
    pub seq_index:u8,
    pub gc_index:u8,
}
pub const WINDOW:u32=0xa0000;
pub const PLANE:usize=0x10000;
impl Planar {
    /// Register state of a BIOS 16-colour graphics mode: all planes enabled,
    /// write mode 0, read mode 0, no set/reset, bit mask FFh, A000h 64 KiB map.
    pub fn new()->Self{
        Self{planes:vec![0;4*PLANE],latch:std::cell::Cell::new([0;4]),seq:[3,1,0x0f,0,6],gc:[0,0,0,0,0,0,5,0x0f,0xff],seq_index:0,gc_index:0}
    }
    pub fn plane(&self,n:usize)->&[u8]{&self.planes[n*PLANE..(n+1)*PLANE]}
    /// Host access to one plane byte (BIOS pixel and text services), without
    /// latches or registers.
    pub fn peek(&self,plane:usize,offset:usize)->u8{self.planes[plane*PLANE+offset]}
    pub fn poke(&mut self,plane:usize,offset:usize,value:u8){self.planes[plane*PLANE+offset]=value;}
    /// Guest read: load the latches, then read mode 0 (the plane in Read Map
    /// Select) or read mode 1 (bits whose colour matches Colour Compare in
    /// the planes Colour Don't Care selects).
    pub fn read(&self,offset:usize)->u8{
        let latch=[0,1,2,3].map(|n|self.planes[n*PLANE+offset]);self.latch.set(latch);
        if self.gc[5]&8==0{return latch[usize::from(self.gc[4]&3)];}
        let mut differ=0u8;
        for n in 0..4{
            if self.gc[7]>>n&1==0{continue;}
            let want=if self.gc[2]>>n&1!=0{0xff}else{0};differ|=latch[n]^want;
        }
        !differ
    }
    /// Guest write through write modes 0-3, the logical function and bit
    /// mask, into the planes the map mask enables.
    pub fn write(&mut self,offset:usize,value:u8){
        let latch=self.latch.get();let mode=self.gc[5]&3;
        let rotated=value.rotate_right(u32::from(self.gc[3]&7));
        let mut mask=self.gc[8];
        for n in 0..4{
            let set_reset=if self.gc[0]>>n&1!=0{0xff}else{0};
            let data=match mode{
                0=>if self.gc[1]>>n&1!=0{set_reset}else{rotated},
                1=>{if self.seq[2]>>n&1!=0{self.planes[n*PLANE+offset]=latch[n];}continue;},
                2=>if value>>n&1!=0{0xff}else{0},
                _=>set_reset,
            };
            if mode==3{mask=self.gc[8]&rotated;}
            let data=match self.gc[3]>>3&3{0=>data,1=>data&latch[n],2=>data|latch[n],_=>data^latch[n]};
            if self.seq[2]>>n&1!=0{self.planes[n*PLANE+offset]=(data&mask)|(latch[n]&!mask);}
        }
    }
    /// The 4-bit colour of pixel `bit` (0 = leftmost) of byte `offset`.
    pub fn pixel(&self,offset:usize,bit:u32)->u8{
        (0..4).fold(0,|c,n|c|((self.planes[n*PLANE+offset]>>(7-bit)&1)<<n))
    }
    pub fn set_pixel(&mut self,offset:usize,bit:u32,color:u8){
        for n in 0..4{let b=&mut self.planes[n*PLANE+offset];*b=(*b&!(0x80>>bit))|(((color>>n)&1)<<(7-bit));}
    }
}
#[cfg(test)]
#[path="../tests/unit/vga_planar.rs"]
mod tests;
