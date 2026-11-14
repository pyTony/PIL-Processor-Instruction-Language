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
        let chain=self.seq[4]&8!=0;let plane=if chain{offset&3}else{usize::from(self.gc[4]&3)};let offset=if chain{offset>>2}else{offset};
        let latch=[0,1,2,3].map(|n|self.planes[n*PLANE+offset]);self.latch.set(latch);
        if self.gc[5]&8==0{return latch[plane];}
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
        let chain=self.seq[4]&8!=0;let selected=offset&3;let offset=if chain{offset>>2}else{offset};
        let latch=self.latch.get();let mode=self.gc[5]&3;
        let rotated=value.rotate_right(u32::from(self.gc[3]&7));
        let mut mask=self.gc[8];
        for n in 0..4{
            if chain&&n!=selected{continue;}
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
    /// Sequencer (3C4h/3C5h) and graphics controller (3CEh/3CFh) access in
    /// a 16-colour planar mode. A word OUT to an index port writes the index,
    /// then the data. Every check precedes any change; returns the value read.
    /// Only the planar addressing these modes use is accepted: no chain-4,
    /// odd/even, shift modes or map changes, and no clocking/reset changes.
    pub fn controller_io(&mut self,port:u16,input:bool,value:u16,word:bool)->Result<u16,String>{
        if !matches!(port,0x3c4|0x3c5|0x3ce|0x3cf){return Err(format!("port {port:04X}h is not a VGA sequencer or graphics controller port"));}
        if word&&!matches!(port,0x3c4|0x3ce){return Err("VGA word access must start at index port 3C4h or 3CEh".into());}
        let sequencer=port<0x3ce;
        let (mut seq,mut gc)=(self.seq,self.gc);
        let mut index=if sequencer{self.seq_index}else{self.gc_index};
        let mut access=|data:bool,value:u8|->Result<u8,String>{
            if !data{if input{return Ok(index);}index=value;return Ok(0);}
            let i=usize::from(index);
            let regs:&mut [u8]=if sequencer{&mut seq}else{&mut gc};
            // Unimplemented extension indices are absent on this plain VGA: reads
            // stay FFh and writes have no effect, so vendor probes can fail normally.
            let Some(old)=regs.get(i).copied() else{return Ok(if input{0xff}else{0});};
            if input{return Ok(old);}
            let allowed=match(sequencer,i){
                (true,2)=>true,(true,4)=>value&0x0c==0x04,
                (false,0..=4|7|8)=>true,(false,5)=>value&0x74==0,(false,6)=>value&0x0f==0x05,
                _=>value==old,
            };
            if !allowed{return Err(format!("VGA {} register {i:02X}h value {value:02X}h is outside the planar profile",if sequencer{"sequencer"}else{"graphics controller"}));}
            regs[i]=value;Ok(0)
        };
        let lo=access(port&1!=0,value as u8)?;
        let hi=if word{access(true,(value>>8)as u8)?}else{0};
        self.seq=seq;self.gc=gc;
        if sequencer{self.seq_index=index;}else{self.gc_index=index;}
        Ok(u16::from(lo)|u16::from(hi)<<8)
    }
    /// The 4-bit colour of pixel `bit` (0 = leftmost) of byte `offset`.
    pub fn pixel(&self,offset:usize,bit:u32)->u8{
        (0..4).fold(0,|c,n|c|((self.planes[n*PLANE+offset]>>(7-bit)&1)<<n))
    }
    pub fn set_pixel(&mut self,offset:usize,bit:u32,color:u8){
        for n in 0..4{let b=&mut self.planes[n*PLANE+offset];*b=(*b&!(0x80>>bit))|(((color>>n)&1)<<(7-bit));}
    }
}
