//! What the PC speaker cone does, as a signal in PIT clocks, and its
//! synthesis to PCM samples.
//!
//! The speaker is driven by port 61h bit 1 AND the output of PIT channel 2,
//! whose gate is port 61h bit 0. The session records each change of that
//! product as a [`Wave`] at the PIT clock it happened (dos_ports.rs); the
//! host console turns the record into sound with [`Synth`]. The synthesis
//! averages the signal over each sample (a box filter, so tones above half
//! the sample rate fade instead of aliasing loudly) and removes the steady
//! level with a one-pole high-pass filter, as a speaker cone cannot hold a
//! position: a lone switch-on is heard as a click, a square wave as a tone.
//! BIOS beeps (BEL on the teletype) are a second voice: 896 Hz for 31/64 s,
//! the count and length the IBM AT BIOS BEEP routine uses.
use std::collections::VecDeque;

/// PIT input clock in Hz (the same value as dos_ports.rs PIT_HZ).
pub const PIT_HZ:u64=1_193_182;
/// The IBM AT BIOS beep: PIT count 1331 (896 Hz), 31/64 s.
pub const BELL_COUNT:u32=1331;
pub const BELL_CLOCKS:u64=PIT_HZ*31/64;
/// A sound that is on for less than this, after silence, is held this
/// long: the guest switches the speaker on and off in a microsecond or two
/// of host time, where a 386 took several, and a shorter pulse would vanish
/// inside one sample. About 1 ms.
pub const MIN_PULSE:u64=PIT_HZ/1000;
/// The silence before a pulse that counts as "after silence": 10 ms.
const QUIET_BEFORE:u64=PIT_HZ/100;
/// At most this much bell sound waits ahead (2 s).
const BELL_BACKLOG:u64=2*PIT_HZ;

/// The speaker signal from one moment on: 1 is the cone pushed out.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Wave{
    /// A constant level.
    Level(bool),
    /// High for `high` of every `period` clocks, starting high: mode 3
    /// (square wave, high half rounded up) and mode 2 (one clock low).
    Periodic{period:u32,high:u32},
    /// Low for `low` clocks, then high: mode 0 counting to its terminal count.
    OneShot{low:u32},
}
impl Wave{
    /// Clocks spent high in [0, x) after the wave started.
    fn high_until(self,x:f64)->f64{
        match self{
            Wave::Level(level)=>if level{x}else{0.0},
            Wave::Periodic{period,high}=>{
                let (p,h)=(f64::from(period),f64::from(high));
                let whole=(x/p).floor();whole*h+(x-whole*p).min(h)
            },
            Wave::OneShot{low}=>(x-f64::from(low)).max(0.0),
        }
    }
    /// The share of [a, b) spent high, with the wave starting at `start`.
    fn average(self,start:f64,a:f64,b:f64)->f64{
        if b<=a{return 0.0;}
        (self.high_until(b-start)-self.high_until(a-start))/(b-a)
    }
    /// Silent: no movement of the cone from now on.
    pub fn steady(self)->bool{matches!(self,Wave::Level(_))}
}

/// Speaker signal to 16-bit PCM.
pub struct Synth{
    rate:u32,
    /// Peak sample value for a full step, from the volume.
    amplitude:f64,
    /// The next sample starts at this PIT clock.
    at:f64,
    /// The wave in force and when it started.
    wave:(f64,Wave),
    pending:VecDeque<(u64,Wave)>,
    /// Bell sounds, [start, end) in PIT clocks, in order.
    bells:VecDeque<(u64,u64)>,
    /// When the last non-steady or high wave began after silence, for MIN_PULSE.
    sound_from:Option<u64>,
    /// The last time the guest signal was silent (Level(false)).
    quiet_since:u64,
    last_x:f64,y:f64,
}
impl Synth{
    pub fn new(rate:u32,volume:u8,start:u64)->Self{
        Self{rate,amplitude:32767.0*f64::from(volume.min(100))/100.0,at:start as f64,wave:(start as f64,Wave::Level(false)),
            pending:VecDeque::new(),bells:VecDeque::new(),sound_from:None,quiet_since:start,last_x:0.0,y:0.0}
    }
    /// PIT clocks per sample.
    fn step(&self)->f64{PIT_HZ as f64/f64::from(self.rate)}
    /// The time of the next sample, in PIT clocks.
    pub fn now(&self)->u64{self.at as u64}
    /// A change of the speaker signal at PIT clock `at`. A change from before
    /// the samples already made counts from the next sample.
    pub fn wave(&mut self,at:u64,wave:Wave){
        let last=self.pending.back().map_or(self.wave.0 as u64,|e|e.0);
        let mut at=at.max(last).max(self.now());
        let previous=self.pending.back().map_or(self.wave.1,|e|e.1);
        if previous==wave{return;}
        let silent=wave==Wave::Level(false);
        if silent{
            // A short sound after a quiet stretch is held to MIN_PULSE.
            if let Some(from)=self.sound_from.take(){at=at.max(from+MIN_PULSE);}
            self.quiet_since=at;
        }else if previous==Wave::Level(false)&&at.saturating_sub(self.quiet_since)>=QUIET_BEFORE{
            self.sound_from=Some(at);
        }else if self.sound_from.is_some_and(|from|at>=from+MIN_PULSE){self.sound_from=None;}
        self.pending.push_back((at,wave));
    }
    /// A BIOS beep asked for at PIT clock `at`; it follows any beep still due.
    pub fn bell(&mut self,at:u64){
        let start=at.max(self.now()).max(self.bells.back().map_or(0,|b|b.1));
        if start>=self.now()+BELL_BACKLOG{return;}
        self.bells.push_back((start,start+BELL_CLOCKS));
    }
    /// Nothing more to hear until the next change.
    pub fn quiet(&self)->bool{
        self.pending.is_empty()&&self.bells.is_empty()&&self.wave.1.steady()&&self.y.abs()*self.amplitude<0.5
    }
    /// Samples for the signal up to PIT clock `until`, appended to `out`.
    pub fn render(&mut self,until:u64,out:&mut Vec<i16>){
        let step=self.step();
        // A one-pole high-pass filter at about 20 Hz.
        let r=1.0-2.0*std::f64::consts::PI*20.0/f64::from(self.rate);
        while self.at+step<=until as f64{
            let (a,b)=(self.at,self.at+step);
            // The guest signal, split at the changes inside this sample.
            let mut x=0.0;let mut from=a;
            while let Some(&(t,next))=self.pending.front(){
                let t=t as f64;
                if t>=b{break;}
                let t=t.max(from);
                x+=self.wave.1.average(self.wave.0,from,t)*(t-from);
                self.wave=(t,next);from=t;self.pending.pop_front();
            }
            x+=self.wave.1.average(self.wave.0,from,b)*(b-from);
            x/=step;
            // The bell voice.
            while self.bells.front().is_some_and(|bell|bell.1 as f64<=a){self.bells.pop_front();}
            if let Some(&(s,e))=self.bells.front(){
                let (s,e)=(s as f64,e as f64);
                let (lo,hi)=(a.max(s),b.min(e));
                if hi>lo{x+=Wave::Periodic{period:BELL_COUNT,high:BELL_COUNT.div_ceil(2)}.average(s,lo,hi)*(hi-lo)/step;}
            }
            self.y=r*(self.y+x-self.last_x);self.last_x=x;
            out.push((self.y*self.amplitude).clamp(-32767.0,32767.0) as i16);
            self.at=b;
        }
    }
}
