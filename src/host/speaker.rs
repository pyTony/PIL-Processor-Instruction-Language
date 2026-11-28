//! PC speaker sound for `pil --dos-console` and `pil --pildos`.
//!
//! The session records the speaker signal (port 61h bits 0-1 and PIT
//! channel 2) as timed changes, and BEL on the console as BIOS beeps
//! (dos_ports.rs). After each slice of guest time the console hands them
//! to [`Sound`], which sends them, with their host times, to an audio
//! thread. The thread synthesises them (dos_speaker_wave.rs) about
//! [`LAG`] behind the host clock, so changes from a slice still in progress
//! arrive in time, and streams the samples to the host's sound output
//! (dos_host_audio.rs). After a second of silence the output is closed;
//! the next sound opens it again.
use std::sync::mpsc;
use std::time::{Duration,Instant};
use super::super::ports::wave::{Synth,Wave,PIT_HZ};

/// Volume when the configuration names none (`speaker_volume`).
pub(crate) const DEFAULT_VOLUME:u8=5;
/// Sample rate of the synthesised sound.
pub(crate) const RATE:u32=22050;
/// How far the sound runs behind the guest.
pub(crate) const LAG:Duration=Duration::from_millis(50);
/// Silence sent first when an output opens, so it starts with a cushion.
const CUSHION:Duration=Duration::from_millis(40);
/// The longest the sound goes on after the session ends (a last beep).
const FINISH:Duration=Duration::from_secs(1);
/// Silence after which the output closes.
const CLOSE_AFTER:Duration=Duration::from_secs(1);

/// One speaker event at its host time.
pub(crate) enum Change{Wave(Instant,Wave),Bell(Instant)}

/// Opens an output at a sample rate; the host's, or a test's.
pub(crate) type Opener=Box<dyn FnMut(u32)->Option<Box<dyn super::audio::Output>>+Send>;

/// The console's end of the audio thread. Dropping it lets the sound
/// already made finish, for at most [`FINISH`].
pub(crate) struct Sound{tx:Option<mpsc::Sender<Change>>,worker:Option<std::thread::JoinHandle<()>>}
impl Sound{
    /// Sound at `volume` (0..100) through `open`; volume 0 starts nothing.
    pub(crate) fn new(volume:u8,open:Opener)->Self{
        if volume==0{return Self{tx:None,worker:None};}
        let (tx,rx)=mpsc::channel();
        let worker=std::thread::Builder::new().name("pc-speaker".into()).spawn(move||play(&rx,volume,open)).ok();
        Self{tx:worker.as_ref().map(|_|tx),worker}
    }
    /// Sound through the host's output.
    pub(crate) fn host(volume:u8)->Self{Self::new(volume,Box::new(super::audio::open))}
    /// Take the speaker changes and beeps the session recorded.
    pub(crate) fn take(&mut self,speaker:&mut super::super::ports::Speaker){
        let (changes,bells)=speaker.take_sound();
        let Some(tx)=&self.tx else{return};
        for (clock,wave) in changes{if let Some(at)=speaker.host_time(clock){let _=tx.send(Change::Wave(at,wave));}}
        for clock in bells{if let Some(at)=speaker.host_time(clock){let _=tx.send(Change::Bell(at));}}
    }
}
impl Drop for Sound{
    fn drop(&mut self){
        // The thread ends when the channel closes.
        self.tx=None;
        if let Some(worker)=self.worker.take(){let _=worker.join();}
    }
}

/// The audio thread: synthesise changes and keep the output fed.
fn play(rx:&mpsc::Receiver<Change>,volume:u8,mut open:Opener){
    let base=Instant::now();
    // Host time as PIT clocks since the thread started.
    let clocks=|t:Instant|(t.saturating_duration_since(base).as_nanos()*u128::from(PIT_HZ)/1_000_000_000) as u64;
    let mut synth:Option<Synth>=None;
    let mut output:Option<Box<dyn super::audio::Output>>=None;
    let mut quiet_from:Option<Instant>=None;
    // No output to be had: take changes and drop them.
    let mut broken=false;
    let mut samples=Vec::new();
    loop{
        let first=if synth.is_some(){
            match rx.recv_timeout(Duration::from_millis(10)){
                Ok(change)=>Some(change),Err(mpsc::RecvTimeoutError::Timeout)=>None,
                // The session ended: let what it already played finish.
                Err(mpsc::RecvTimeoutError::Disconnected)=>{
                    let end=Instant::now()+FINISH;
                    while let (Some(s),Some(out))=(synth.as_mut(),output.as_mut()){
                        if s.quiet()||Instant::now()>=end{break;}
                        std::thread::sleep(Duration::from_millis(10));
                        samples.clear();s.render(clocks(Instant::now().checked_sub(LAG).unwrap_or(base)),&mut samples);
                        if !out.write(&samples){break;}
                    }
                    return;
                },
            }
        }else{match rx.recv(){Ok(change)=>Some(change),Err(_)=>return}};
        let now=clocks(Instant::now().checked_sub(LAG).unwrap_or(base));
        for change in first.into_iter().chain(rx.try_iter()){
            if broken{continue;}
            let s=synth.get_or_insert_with(||Synth::new(RATE,volume,now));
            match change{Change::Wave(at,wave)=>s.wave(clocks(at),wave),Change::Bell(at)=>s.bell(clocks(at))}
        }
        let Some(s)=synth.as_mut() else{continue};
        if output.is_none(){
            output=open(RATE);
            let Some(out)=output.as_mut() else{broken=true;synth=None;continue};
            let cushion=vec![0i16;(u128::from(RATE)*CUSHION.as_millis()/1000) as usize];
            if !out.write(&cushion){broken=true;synth=None;output=None;continue;}
        }
        samples.clear();s.render(now,&mut samples);
        if !output.as_mut().is_some_and(|out|out.write(&samples)){broken=true;synth=None;output=None;continue;}
        if s.quiet(){
            if quiet_from.get_or_insert_with(Instant::now).elapsed()>=CLOSE_AFTER{synth=None;output=None;quiet_from=None;}
        }else{quiet_from=None;}
    }
}
