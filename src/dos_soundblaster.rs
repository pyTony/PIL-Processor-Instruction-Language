//! Sound Blaster 2.0-compatible DSP and 8237 channel-1 DMA.
//!
//! All guest-device state stays here. The host's PCM player consumes bounded
//! blocks separately; lack of an audio device never changes guest IRQ results.
//! Timing uses PIT clocks supplied by the DOS executor, or a deterministic
//! synthetic step when the host clock is disabled.
use std::collections::VecDeque;
use super::{MemWidth,Ram};
use super::ports::{pic::Pair,PIT_HZ};

const BASE:u16=0x220;
const MAX_QUEUED_BLOCKS:usize=8;
const MAX_DSP_REPLY:usize=16;

#[derive(Clone,Debug)]
struct Dma1{
    base:u16,count:u16,addr:u16,left:u16,page:u8,
    flip:bool,mode:u8,masked:bool,terminal:bool,
}
impl Default for Dma1{
    fn default()->Self{Self{base:0,count:0,addr:0,left:0,page:0,flip:false,mode:0,masked:true,terminal:false}}
}
impl Dma1{
    /// Only channel 1 is consumed by the DSP. Other DMA channels still use
    /// the pre-existing open-bus/no-op policy in dos_ports.rs.
    fn io(&mut self,port:u16,input:bool,value:u8)->Option<u8>{
        let result=match (port,input){
            (0x02,true)=>{
                let byte=if self.flip{(self.addr>>8)as u8}else{self.addr as u8};
                self.flip=!self.flip;byte
            },
            (0x03,true)=>{
                let byte=if self.flip{(self.left>>8)as u8}else{self.left as u8};
                self.flip=!self.flip;byte
            },
            (0x02,false)=>{
                self.base=if self.flip{self.base&0x00ff|u16::from(value)<<8}else{self.base&0xff00|u16::from(value)};
                self.addr=self.base;self.flip=!self.flip;0
            },
            (0x03,false)=>{
                self.count=if self.flip{self.count&0x00ff|u16::from(value)<<8}else{self.count&0xff00|u16::from(value)};
                self.left=self.count;self.terminal=false;self.flip=!self.flip;0
            },
            (0x83,true)=>self.page,
            (0x83,false)=>{self.page=value;0},
            (0x08,true)=>{let result=if self.terminal{2}else{0};self.terminal=false;result},
            (0x0a,false)=>{if value&3==1{self.masked=value&4!=0;}0},
            (0x0b,false)=>{if value&3==1{self.mode=value;}0},
            (0x0c,false)=>{self.flip=false;0},
            (0x0d,false)=>{*self=Self::default();0},
            (0x0e,false)=>{self.masked=false;0},
            (0x0f,false)=>{self.masked=value&2!=0;0},
            (0x0f,true)=>if self.masked{0x0f}else{0x0d},
            _=>return None,
        };
        Some(result)
    }
    fn take(&mut self,ram:&Ram,wanted:usize)->Result<Vec<u8>,String>{
        if self.masked{return Err("Sound Blaster DMA channel 1 is masked".into());}
        if self.mode&3!=1||self.mode>>2&3!=2{
            return Err(format!("Sound Blaster requires DMA channel 1 memory-to-device mode (got {:02X}h)",self.mode));
        }
        let size=wanted.min(usize::from(self.left)+1);
        let mut bytes=Vec::with_capacity(size);
        for offset in 0..size{
            // The 8237 increments the 16-bit address *inside* the page; a
            // transfer wrapping at FFFFh cannot carry into the next page.
            let at=(u32::from(self.page)<<16)|u32::from(self.addr.wrapping_add(offset as u16));
            bytes.push(ram.load(at,MemWidth::Byte)
                .map_err(|e|format!("Sound Blaster DMA read {at:06X}h: {e:?}"))? as u8);
        }
        self.addr=self.addr.wrapping_add(size as u16);
        if size==usize::from(self.left)+1{
            self.terminal=true;
            if self.mode&0x10!=0{self.addr=self.base;self.left=self.count;}
            else{self.left=0xffff;self.masked=true;}
        }else{self.left-=size as u16;}
        Ok(bytes)
    }
}

#[derive(Clone,Debug)]
struct Playback{deadline:u64,remaining:Option<u64>,auto:bool}
/// State of an optional SB2 digital audio card (no OPL/FM synthesizer).
#[derive(Clone,Debug)]
pub(crate) struct SoundBlaster{
    dma:Dma1,reset_high:bool,reply:VecDeque<u8>,
    pending:Option<(u8,Vec<u8>,usize)>,
    speaker_on:bool,rate:u32,block_length:u16,
    mixer_index:u8,irq:u8,irq_pending:bool,test_byte:u8,
    playback:Option<Playback>,pcm:VecDeque<(u32,Vec<i16>)>,now:u64,
}
impl Default for SoundBlaster{
    fn default()->Self{Self{
        dma:Dma1::default(),reset_high:false,reply:VecDeque::new(),pending:None,
        speaker_on:false,rate:11_025,block_length:0,
        mixer_index:0,irq:5,irq_pending:false,test_byte:0,
        playback:None,pcm:VecDeque::new(),now:0,
    }}
}
impl SoundBlaster{
    fn answer(&mut self,value:u8){
        if self.reply.len()==MAX_DSP_REPLY{self.reply.pop_front();}
        self.reply.push_back(value);
    }
    fn raise(&mut self,pic:&mut Pair){
        if !self.irq_pending{self.irq_pending=true;pic.raise_soundblaster(self.irq);}
    }
    fn queue(&mut self,bytes:&[u8]){
        if !self.speaker_on||bytes.is_empty(){return;}
        if self.pcm.len()==MAX_QUEUED_BLOCKS{self.pcm.pop_front();}
        self.pcm.push_back((self.rate,bytes.iter().map(|&b|(i16::from(b)-128)<<8).collect()));
    }
    /// Audio blocks already prepared for the host; removing them does not
    /// change DSP timing, DMA state, or the PIC.
    pub(crate) fn take_pcm(&mut self)->Vec<(u32,Vec<i16>)>{self.pcm.drain(..).collect()}
    fn start(&mut self,ram:&Ram,length:usize,auto:bool,at:u64)->Result<(),String>{
        if self.playback.is_some(){return Err("Sound Blaster DMA playback already active".into());}
        let bytes=self.dma.take(ram,length)?;
        if bytes.is_empty(){return Err("Sound Blaster DMA transfer has no samples".into());}
        self.queue(&bytes);
        let duration=((bytes.len() as u64)*u64::from(PIT_HZ)/u64::from(self.rate)).max(1);
        self.playback=Some(Playback{deadline:at.saturating_add(duration),remaining:None,auto});
        Ok(())
    }
    /// Poll completion and make a *single edge* of IRQ5/IRQ7 for each
    /// completed DMA block. Automatic initialization reuses the DMA base
    /// and count, but never queues unbounded audio when a frontend stalls.
    pub(in crate::dos) fn tick(&mut self,clock:Option<u64>,ram:&Ram,pic:&mut Pair)->Result<(),String>{
        self.now=match clock{Some(t)=>self.now.max(t),None=>self.now.saturating_add(1024)};
        for _ in 0..8{
            let Some(play)=self.playback.as_ref() else{break};
            if play.remaining.is_some()||play.deadline>self.now{break;}
            let at=play.deadline;
            let repeat=play.auto;
            self.playback=None;
            self.raise(pic);
            if repeat{self.start(ram,usize::from(self.block_length)+1,true,at)?;}
        }
        Ok(())
    }
    fn execute(&mut self,command:u8,args:&[u8],ram:&Ram,pic:&mut Pair)->Result<(),String>{
        match command{
            0x10=>self.queue(args),
            0x14=>{
                let len=usize::from(u16::from_le_bytes([args[0],args[1]]))+1;
                self.start(ram,len,false,self.now)?;
            },
            0x1c=>self.start(ram,usize::from(self.block_length)+1,true,self.now)?,
            0x20=>self.answer(0x80), // silent/unconnected ADC
            0x40=>{
                self.rate=(1_000_000/(256-u32::from(args[0]))).clamp(4_000,44_100);
            },
            0x41=>{
                let rate=u16::from_be_bytes([args[0],args[1]]);
                self.rate=u32::from(rate).clamp(4_000,44_100);
            },
            0x48=>self.block_length=u16::from_le_bytes([args[0],args[1]]),
            0x80=>{ // programmed silence, an IRQ after the requested interval
                let len=u64::from(u16::from_le_bytes([args[0],args[1]]))+1;
                self.playback=Some(Playback{deadline:self.now+len*u64::from(PIT_HZ)/u64::from(self.rate),remaining:None,auto:false});
            },
            0xd0=>{if let Some(play)=self.playback.as_mut(){play.remaining=Some(play.deadline.saturating_sub(self.now));}},
            0xd1=>self.speaker_on=true,
            0xd3=>self.speaker_on=false,
            0xd4=>{if let Some(play)=self.playback.as_mut(){if let Some(left)=play.remaining.take(){play.deadline=self.now.saturating_add(left);}}},
            0xd8=>self.answer(if self.speaker_on{0xff}else{0x00}),
            0xda=>{if let Some(play)=self.playback.as_mut(){play.auto=false;}},
            0xe0=>self.answer(!args[0]),
            0xe1=>{self.answer(2);self.answer(1);}, // Sound Blaster 2.01
            0xe4=>self.test_byte=args[0],
            0xe8=>self.answer(self.test_byte),
            0xf2=>self.raise(pic),
            _=>return Err(format!("Sound Blaster DSP command {command:02X}h is not modelled")),
        }
        Ok(())
    }
    fn command(&mut self,value:u8,ram:&Ram,pic:&mut Pair)->Result<(),String>{
        if let Some((_,args,required))=self.pending.as_mut(){
            args.push(value);
            if args.len()==*required{
                let (cmd,args,_)=self.pending.take().unwrap();
                self.execute(cmd,&args,ram,pic)?;
            }
            return Ok(());
        }
        let required=match value{
            0x10|0x40|0xe0|0xe4=>1,
            0x14|0x41|0x48|0x80=>2,
            _=>0,
        };
        if required==0{self.execute(value,&[],ram,pic)}
        else{self.pending=Some((value,Vec::new(),required));Ok(())}
    }
    /// Byte I/O to DMA channel 1 and SB at 220h..22Fh. None means another
    /// port owns the access. Even unsupported DSP commands stop explicitly,
    /// instead of pretending the hardware accepted an operation.
    pub(super) fn io(&mut self,port:u16,input:bool,value:u8,ram:&Ram,pic:&mut Pair)->Result<Option<u8>,String>{
        if let Some(result)=self.dma.io(port,input,value){return Ok(Some(result));}
        if !(BASE..=BASE+15).contains(&port){return Ok(None);}
        let result=match(port-BASE,input){
            // SB 2.0 FM aliases follow the absent-OPL profile of 388h/389h.
            // They neither consume DSP replies nor claim an FM synthesizer.
            (0x08|0x09,true)=>0xff,
            (0x08|0x09,false)=>0,
            (0x04,true)=>self.mixer_index,
            (0x04,false)=>{self.mixer_index=value;0},
            (0x05,true)=>match self.mixer_index{
                0x80=>if self.irq==5{2}else{8},
                0x81=>2, // DMA channel 1
                _=>0,
            },
            (0x05,false)=>{
                if self.mixer_index==0x80{
                    self.irq=match value{2=>5,8=>7,_=>return Err(format!("Sound Blaster IRQ mixer mask {value:02X}h is not supported (IRQ5/7 only)"))};
                }
                0
            },
            (0x06,true)=>0xff,
            (0x06,false)=>{
                if value&1!=0{self.reset_high=true;}
                else if self.reset_high{
                    self.reset_high=false;self.reply.clear();self.pending=None;
                    self.playback=None;self.pcm.clear();self.irq_pending=false;self.speaker_on=false;
                    self.answer(0xaa);
                }
                0
            },
            (0x0a,true)=>self.reply.pop_front().unwrap_or(0xff),
            // 22Ah is the DSP's read-only data latch. An ISA OUT to
            // that address is not a DSP command and must not consume replies.
            (0x0a,false)=>0,
            (0x0c,true)=>0, // DSP write buffer is ready
            (0x0c,false)=>{self.command(value,ram,pic)?;0},
            (0x0e,true)=>{
                // Reading the 8-bit IRQ status acknowledges the DSP latch.
                self.irq_pending=false;
                if self.reply.is_empty(){0}else{0x80}
            },
            (0x0f,true)=>{self.irq_pending=false;0}, // 16-bit acknowledge (no 16-bit DMA)
            (0x0e|0x0f,false)=>0,
            _=>return Err(format!("Sound Blaster port {port:03X}h {} is not modelled",if input{"read"}else{"write"})),
        };
        Ok(Some(result))
    }
    pub(super) fn irq(&self)->u8{self.irq}
    pub(super) fn pending_irq(&self)->bool{self.irq_pending}
}

#[cfg(test)]
mod tests{
    use super::*;
    fn write(sb:&mut SoundBlaster,pic:&mut Pair,ram:&Ram,port:u16,value:u8){
        sb.io(port,false,value,ram,pic).unwrap();
    }
    fn read(sb:&mut SoundBlaster,pic:&mut Pair,ram:&Ram,port:u16)->u8{
        sb.io(port,true,0,ram,pic).unwrap().unwrap()
    }
    #[test]
    fn fm_aliases_float_high_without_changing_dsp_or_dma_state(){
        let ram=Ram::new(1024);let mut sb=SoundBlaster::default();let mut pic=Pair::default();
        write(&mut sb,&mut pic,&ram,0x226,1);write(&mut sb,&mut pic,&ram,0x226,0);
        let before=format!("{sb:?}");
        for port in [0x228,0x229]{
            for value in [0,4,0x60,0xff]{write(&mut sb,&mut pic,&ram,port,value);}
            assert_eq!(read(&mut sb,&mut pic,&ram,port),0xff);
        }
        assert_eq!(format!("{sb:?}"),before);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),0xaa);
    }
    fn setup(sb:&mut SoundBlaster,pic:&mut Pair,ram:&Ram,count:u16,address:u16,page:u8,mode:u8){
        write(sb,pic,ram,0x0c,0);
        write(sb,pic,ram,0x02,address as u8);
        write(sb,pic,ram,0x02,(address>>8)as u8);
        write(sb,pic,ram,0x03,count as u8);
        write(sb,pic,ram,0x03,(count>>8)as u8);
        write(sb,pic,ram,0x83,page);
        write(sb,pic,ram,0x0b,mode);
        write(sb,pic,ram,0x0a,1);
    }
    #[test]fn reset_version_and_dsp_reply_status(){
        let(mut sb,mut pic,ram)=(SoundBlaster::default(),Pair::default(),Ram::new(1<<20));
        write(&mut sb,&mut pic,&ram,0x226,1);write(&mut sb,&mut pic,&ram,0x226,0);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22e),0x80);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),0xaa);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22e),0);
        write(&mut sb,&mut pic,&ram,0x22c,0xe1);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),2);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),1);
        write(&mut sb,&mut pic,&ram,0x22c,0xe0);write(&mut sb,&mut pic,&ram,0x22c,0x55);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),0xaa);
        write(&mut sb,&mut pic,&ram,0x22c,0xd1);write(&mut sb,&mut pic,&ram,0x22c,0xd8);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),0xff);
    }
    #[test]fn prince_of_persia_write_to_dsp_read_port_is_ignored(){
        let(mut sb,mut pic,ram)=(SoundBlaster::default(),Pair::default(),Ram::new(1<<20));
        write(&mut sb,&mut pic,&ram,0x226,1);
        write(&mut sb,&mut pic,&ram,0x226,0);
        // Prince writes 22Ah during startup; it is read-only, not a command
        // register. A queued DSP reset acknowledgement must survive.
        write(&mut sb,&mut pic,&ram,0x22a,0x55);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22e),0x80);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),0xaa);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22e),0);
        // Subsequent genuine DSP commands at 22Ch still work.
        write(&mut sb,&mut pic,&ram,0x22c,0xe1);
        write(&mut sb,&mut pic,&ram,0x22a,0xff);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),2);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22a),1);
    }
    #[test]fn dma_wraps_within_page_and_irq_waits_for_completion(){
        let(mut sb,mut pic,mut ram)=(SoundBlaster::default(),Pair::default(),Ram::new(1<<20));
        ram.store(0x1ffff,MemWidth::Byte,0).unwrap();
        ram.store(0x10000,MemWidth::Byte,255).unwrap();
        setup(&mut sb,&mut pic,&ram,1,0xffff,1,0x49);
        write(&mut sb,&mut pic,&ram,0x22c,0xd1);
        write(&mut sb,&mut pic,&ram,0x22c,0x14);
        write(&mut sb,&mut pic,&ram,0x22c,1);
        write(&mut sb,&mut pic,&ram,0x22c,0);
        assert_eq!(sb.take_pcm(),vec![(11_025,vec![-32768,32512])]);
        assert!(!sb.pending_irq());
        sb.tick(Some(100),&ram,&mut pic).unwrap();
        assert!(!sb.pending_irq());
        sb.tick(Some(1000),&ram,&mut pic).unwrap();
        assert!(sb.pending_irq());
        assert!(pic.soundblaster_ready(5));
        pic.acknowledge_soundblaster(5);
        assert_eq!(read(&mut sb,&mut pic,&ram,0x22e),0);
        assert!(!sb.pending_irq());
    }
    #[test]fn automatic_dma_reloads_and_pause_defers_irq(){
        let(mut sb,mut pic,mut ram)=(SoundBlaster::default(),Pair::default(),Ram::new(1<<20));
        ram.store(0x2000,MemWidth::Byte,0x80).unwrap();
        setup(&mut sb,&mut pic,&ram,0,0x2000,0,0x59);
        write(&mut sb,&mut pic,&ram,0x22c,0x48);
        write(&mut sb,&mut pic,&ram,0x22c,0);
        write(&mut sb,&mut pic,&ram,0x22c,0);
        write(&mut sb,&mut pic,&ram,0x22c,0xd1);
        write(&mut sb,&mut pic,&ram,0x22c,0x1c);
        write(&mut sb,&mut pic,&ram,0x22c,0xd0);
        sb.tick(Some(2000),&ram,&mut pic).unwrap();assert!(!sb.pending_irq());
        write(&mut sb,&mut pic,&ram,0x22c,0xd4);
        sb.tick(Some(3000),&ram,&mut pic).unwrap();assert!(sb.pending_irq());
        assert!(!sb.take_pcm().is_empty());
        write(&mut sb,&mut pic,&ram,0x22c,0xda);
        sb.tick(Some(4000),&ram,&mut pic).unwrap();
        assert!(sb.playback.is_none());
    }
    #[test]fn invalid_dma_mode_and_unknown_dsp_command_are_explicit(){
        let(mut sb,mut pic,ram)=(SoundBlaster::default(),Pair::default(),Ram::new(1<<20));
        setup(&mut sb,&mut pic,&ram,0,0x1000,0,0x45);
        write(&mut sb,&mut pic,&ram,0x22c,0x14);
        write(&mut sb,&mut pic,&ram,0x22c,0);
        assert!(sb.io(0x22c,false,0,&ram,&mut pic).is_err());
        assert!(sb.io(0x22c,false,0xff,&ram,&mut pic).is_err());
    }
}
