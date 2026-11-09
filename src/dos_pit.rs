//! Bounded binary PIT channel 0. Supplied input clocks make device tests exact.
//! GATE is tied high. The same live divisor may be rewritten; other reloads stop.
/// Channel 2 starts with the conventional BIOS square-wave configuration and
/// its gate low. Binary modes 0, 2 and 3 are timed; other speaker modes can still be written
/// for audio but cannot be read as though they were modelled.
#[derive(Clone,Debug)]
pub(super) struct Counter2{mode:u8,access:u8,reload:u32,epoch:u64,frozen:u32,gate:bool,low:Option<u8>,read_high:bool,latch:Option<(u16,bool)>}
impl Default for Counter2{fn default()->Self{Self{mode:3,access:3,reload:65536,epoch:0,frozen:0,gate:false,low:None,read_high:false,latch:None}}}
impl Counter2{
    fn count_full(&self,now:u64)->u32{
        if !self.gate{return self.frozen;}
        if self.mode==0{return u64::from(self.frozen).wrapping_sub(now.saturating_sub(self.epoch))as u16 as u32;}
        let phase=now.saturating_sub(self.epoch)%u64::from(self.reload);
        if self.mode==2{(u64::from(self.reload)-phase)as u32}else{
            let half=u64::from(self.reload).div_ceil(2);let at=if phase<half{phase}else{phase-half};
            u64::from(self.reload&!1).saturating_sub(2*at)as u32
        }
    }
    fn count(&self,now:u64)->u16{self.count_full(now)as u16}
    pub fn gate(&mut self,gate:bool,now:u64){
        if self.gate&&!gate{self.frozen=self.count_full(now);}
        if !self.gate&&gate{self.epoch=if self.mode==0{now}else{now.saturating_add(1)};}
        self.gate=gate;
    }
    pub fn command(&mut self,value:u8,now:u64)->Result<(),String>{
        if value&0x30==0{
            if !matches!(self.mode,0|2|3){return Err(format!("PIT channel 2 latch: mode {} is not modelled",self.mode));}
            if self.latch.is_none(){self.latch=Some((self.count(now),self.read_high));}
        }else{
            self.frozen=self.count_full(now);self.mode=if value&1!=0{255}else{match value>>1&7{6=>2,7=>3,n=>n}};
            self.access=value>>4&3;self.low=None;self.read_high=false;self.latch=None;
        }
        Ok(())
    }
    pub fn write(&mut self,value:u8,now:u64){
        let raw=match self.access{1=>u16::from(value),2=>u16::from(value)<<8,3=>match self.low.take(){None=>{self.low=Some(value);return},Some(lo)=>u16::from_le_bytes([lo,value])},_=>return};
        self.reload=if raw==0{65536}else{u32::from(raw)};self.frozen=self.reload;self.epoch=now.saturating_add(1);
    }
    pub fn read(&mut self,now:u64)->Result<u8,String>{
        if !matches!(self.mode,0|2|3){return Err("PIT channel 2 read requires a modelled binary mode 0, 2 or 3".into());}
        let(count,high)=self.latch.unwrap_or((self.count(now),self.read_high));
        let value=if self.access==2||self.access==3&&high{(count>>8)as u8}else{count as u8};
        if self.latch.is_some(){if self.access!=3||high{self.latch=None}else{self.latch=Some((count,true));}}
        self.read_high=self.access==3&&!high;Ok(value)
    }
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Counter0{mode:u8,access:u8,reload:u32,epoch:u64,irq_base:u64,running:bool,frozen:u16,frozen_irqs:u64,armed:bool,low:Option<u8>,read_high:bool,latch:Option<(u16,bool)>}
impl Default for Counter0{fn default()->Self{Self{mode:3,access:3,reload:65536,epoch:0,irq_base:0,running:true,frozen:0,frozen_irqs:0,armed:false,low:None,read_high:false,latch:None}}}
impl Counter0{
    pub fn count(&self,now:u64)->u16{
        if !self.running{return self.frozen;}
        let phase=now.saturating_sub(self.epoch)%u64::from(self.reload);
        let count=if self.mode==2{u64::from(self.reload)-phase}else{
            let high=u64::from(self.reload).div_ceil(2);let at=if phase<high{phase}else{phase-high};
            u64::from(self.reload&!1).saturating_sub(2*at)
        };count as u16
    }
    pub fn interrupts(&self,now:u64)->u64{if self.running{self.irq_base+now.saturating_sub(self.epoch)/u64::from(self.reload)}else{self.frozen_irqs}}
    pub fn command(&mut self,value:u8,now:u64)->Result<(),String>{
        if value>>6!=0{return Err("PIT channel 0 command selects another channel".into());}
        if value&0x30==0{if self.latch.is_none(){self.latch=Some((self.count(now),self.read_high));}return Ok(());}
        let mode=(value>>1&7)&3;
        if value&1!=0||!matches!(value>>1&7,2|3|6|7){return Err(format!("PIT channel 0 command {value:02X}h: binary modes 2/3 only"));}
        self.frozen=self.count(now);self.frozen_irqs=self.interrupts(now);self.running=false;
        self.mode=mode;self.access=value>>4&3;self.armed=true;self.low=None;self.read_high=false;self.latch=None;Ok(())
    }
    pub fn write(&mut self,value:u8,now:u64)->Result<(),String>{
        // Rewriting the currently running divisor leaves the mode-2/3 phase
        // unchanged: the same value is loaded at the next period boundary.
        // Other live changes remain an explicit unsupported operation.
        if !self.armed && self.access != 3{return Err("PIT channel 0 live reload requires a new control word in this profile".into());}
        let raw=match self.access{1=>u16::from(value),2=>u16::from(value)<<8,_=>match self.low{None=>{self.low=Some(value);return Ok(())},Some(lo)=>u16::from_le_bytes([lo,value])}};
        let reload=if raw==0{65536}else{u32::from(raw)};
        if reload<2{return Err("PIT modes 2/3 require a count of at least two (zero means 65536)".into());}
        if !self.armed{
            if reload!=self.reload{return Err("PIT channel 0 live reload requires a new control word in this profile".into());}
            self.low=None;return Ok(());
        }
        self.reload=reload;self.epoch=now.saturating_add(1);self.irq_base=self.frozen_irqs;self.running=true;self.armed=false;self.low=None;Ok(())
    }
    pub fn read(&mut self,now:u64)->u8{
        let (count,high)=self.latch.unwrap_or((self.count(now),self.read_high));
        let value=if self.access==2||self.access==3&&high{(count>>8)as u8}else{count as u8};
        let finished=self.access!=3||high;
        if self.latch.is_some(){if finished{self.latch=None;}else{self.latch=Some((count,true));}}
        self.read_high=if self.access==3{!high}else{false};value
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn channel_two_latches_count_and_gate_restarts_square_wave(){
        let mut p=Counter2::default();p.command(0xb6,0).unwrap();p.write(0x34,0);p.write(0x12,0);p.gate(true,10);
        p.command(0x80,20).unwrap();let expected=p.count(20);p.command(0x80,30).unwrap();
        assert_eq!(p.read(50).unwrap(),expected as u8);assert_eq!(p.read(100).unwrap(),(expected>>8)as u8);
        p.gate(false,101);let frozen=p.count(101);assert_eq!(p.count(200),frozen);
        p.gate(true,200);assert_eq!(p.count(201),0x1234);assert_ne!(p.count(205),frozen);
        p.command(0xb2,210).unwrap();assert!(p.read(211).is_err());
    }
    #[test]fn channel_two_mode_zero_latches_and_gate_pauses_count(){
        let mut p=Counter2::default();p.command(0xb0,0).unwrap();p.write(0,0);p.write(0,0);p.gate(true,10);
        assert_eq!(p.count(10),0);assert_eq!(p.count(11),0xffff);
        p.command(0x80,15).unwrap();assert_eq!(p.read(19).unwrap(),0xfb);assert_eq!(p.read(100).unwrap(),0xff);
        p.gate(false,101);let frozen=p.count(101);assert_eq!(p.count(500),frozen);p.gate(true,500);
        assert_eq!(p.count(501),frozen.wrapping_sub(1));assert_eq!(p.count(100000),frozen.wrapping_sub(99500u32 as u16));
    }
    fn start(word:u8,count:u16,now:u64)->Counter0{let mut p=Counter0::default();p.command(word,now).unwrap();p.write(count as u8,now).unwrap();p.write((count>>8)as u8,now).unwrap();p}
    #[test]fn mode_two_uses_programmed_divisor_next_clock_load_and_periodic_edges(){
        for n in [2u16,3,0xe90b,0]{let p=start(0x34,n,100);let period=if n==0{65536}else{u64::from(n)};
            assert_eq!(p.count(100),n);assert_eq!(p.count(101),n);assert_eq!(p.count(102),(period-1)as u16);
            assert_eq!(p.count(100+period),1);assert_eq!(p.count(101+period),n);
            assert_eq!(p.interrupts(100+period),0);assert_eq!(p.interrupts(101+3*period),3);
        }
    }
    #[test]fn mode_three_reads_count_by_two_with_even_and_odd_periods(){
        let p=start(0x36,6,0);assert_eq!((1..=13).map(|t|p.count(t)).collect::<Vec<_>>(),vec![6,4,2,6,4,2,6,4,2,6,4,2,6]);
        let p=start(0x36,5,0);assert_eq!((1..=11).map(|t|p.count(t)).collect::<Vec<_>>(),vec![4,2,0,4,2,4,2,0,4,2,4]);
        assert_eq!(p.interrupts(5),0);assert_eq!(p.interrupts(6),1);
    }
    #[test]fn latch_is_stable_ignored_when_unread_and_released_after_complete_read(){
        let mut p=start(0x34,0x1234,0);p.command(0,17).unwrap();assert_eq!(p.read(100),0x24);
        p.command(0,200).unwrap();assert_eq!(p.read(90000),0x12);
        assert_eq!(p.read(90001),p.count(90001)as u8);
        // A latch during a live high-byte read retains the byte sequence.
        p.command(0,90002).unwrap();assert_eq!(p.read(99999),(p.count(90002)>>8)as u8);
    }
    #[test]fn control_word_freezes_counter_until_complete_write_and_preserves_edge_total(){
        let mut p=start(0x34,10,0);p.command(0x34,36).unwrap();assert_eq!(p.count(100),5);assert_eq!(p.interrupts(100),3);
        p.write(20,100).unwrap();assert_eq!(p.count(200),5);assert_eq!(p.interrupts(200),3);
        p.write(0,200).unwrap();assert_eq!(p.count(201),20);assert_eq!(p.interrupts(220),3);assert_eq!(p.interrupts(221),4);
    }
    #[test]fn single_byte_modes_zero_count_and_mode_aliases(){
        let mut p=Counter0::default();p.command(0x14,0).unwrap();p.write(7,0).unwrap();assert_eq!(p.read(1),7);
        p.command(0x24,1).unwrap();p.write(0x12,1).unwrap();assert_eq!(p.count(2),0x1200);assert_eq!(p.read(2),0x12);
        for (a,b) in [(0x34,0x3c),(0x36,0x3e)]{let x=start(a,8,0);let y=start(b,8,0);for t in 0..100{assert_eq!(x.count(t),y.count(t));}}
    }
    #[test]fn identical_live_reload_preserves_running_phase_and_interrupts(){
        for mode in [0x34,0x36]{
            for value in [20u16,0]{
                let mut p=start(mode,value,0);let baseline=p.clone();
                p.write(value as u8,10).unwrap();p.write((value>>8)as u8,11).unwrap();
                for t in [11,19,21,100,65538]{assert_eq!(p.count(t),baseline.count(t));assert_eq!(p.interrupts(t),baseline.interrupts(t));}
            }
        }
    }
    #[test]fn invalid_commands_counts_and_live_reload_stop_without_mutation(){
        let mut p=start(0x34,20,0);
        for cmd in [0x35,0x30,0x32,0x76,0xc2]{let old=p.clone();assert!(p.command(cmd,100).is_err());assert_eq!(p,old);}
        p.write(0,100).unwrap();let old=p.clone();assert!(p.write(0,100).is_err());assert_eq!(p,old);
        p.command(0x34,100).unwrap();p.write(1,100).unwrap();let old=p.clone();assert!(p.write(0,100).is_err());assert_eq!(p,old);p.write(1,100).unwrap();assert_eq!(p.count(101),257);
    }
}
