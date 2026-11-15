//! PC speaker clicks for `pil --dos-console` and `pil --pildos`, as the Tk
//! terminal (tools/dos_terminal.py) plays them.
//!
//! The session counts the times port 61h switched the speaker on
//! (`Speaker::beeps`); it keeps no PCM and no pulse lengths. Each new count
//! plays one short, quiet click (10 ms, 1200 Hz, with an envelope), at most
//! one every 0.2 s, so benchmark ticks are heard without a flood of sound.
//! Windows plays it with winmm PlaySound from memory, asynchronously; other
//! hosts pipe it to `aplay` when that is on PATH, else stay silent.
use std::time::{Duration,Instant};

/// Volume when the configuration names none (`speaker_volume`).
pub(crate) const DEFAULT_VOLUME:u8=5;
/// The shortest time between two clicks.
const INTERVAL:Duration=Duration::from_millis(200);

/// A 10 ms 16-bit mono click at 22050 Hz, as a WAV file in memory; the
/// same samples as the Tk terminal's `speaker_click`.
pub(crate) fn click(level:u8)->Vec<u8>{
    const RATE:u32=22050;const COUNT:usize=220;
    let amplitude=32767.0*f64::from(level.min(100))/100.0;
    let mut wav=Vec::with_capacity(44+COUNT*2);
    let data=(COUNT*2) as u32;
    wav.extend_from_slice(b"RIFF");wav.extend_from_slice(&(36+data).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&RATE.to_le_bytes());wav.extend_from_slice(&(RATE*2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");wav.extend_from_slice(&data.to_le_bytes());
    for i in 0..COUNT{
        let envelope=(i as f64/22.0).min((COUNT-1-i) as f64/22.0).min(1.0);
        let sample=amplitude*envelope*(2.0*std::f64::consts::PI*1200.0*i as f64/f64::from(RATE)).sin();
        wav.extend_from_slice(&(sample as i16).to_le_bytes());
    }
    wav
}

/// Where clicks go: the host's sound output, or a test's recorder.
pub(crate) trait Player{fn play(&mut self,wav:&std::sync::Arc<Vec<u8>>);}

/// Decides when a click sounds, from the session's speaker count.
pub(crate) struct Clicks<P:Player>{
    level:u8,
    last_count:u64,
    last_play:Option<Instant>,
    wav:std::sync::Arc<Vec<u8>>,
    player:P,
}
impl<P:Player> Clicks<P>{
    pub(crate) fn new(level:u8,initial:u64,player:P)->Self{
        Self{level,last_count:initial,last_play:None,wav:std::sync::Arc::new(if level>0{click(level)}else{Vec::new()}),player}
    }
    /// The speaker count after a slice of guest time.
    pub(crate) fn observe(&mut self,count:u64,now:Instant){
        // A new program starts its own count.
        let changed=count>self.last_count;
        self.last_count=count;
        if !changed||self.level==0{return;}
        if self.last_play.is_some_and(|t|now.duration_since(t)<INTERVAL){return;}
        self.last_play=Some(now);
        self.player.play(&self.wav);
    }
    #[cfg(test)]
    pub(crate) fn player(&self)->&P{&self.player}
}

/// The host's sound output.
#[derive(Default)]
pub(crate) struct HostPlayer{
    #[cfg(not(windows))]
    worker:Option<std::thread::JoinHandle<()>>,
    /// The sound PlaySound plays asynchronously; it must outlive playback.
    #[cfg(windows)]
    playing:Option<std::sync::Arc<Vec<u8>>>,
}
#[cfg(windows)]
mod win{
    pub(super) const SND_ASYNC:u32=0x0001;
    pub(super) const SND_NODEFAULT:u32=0x0002;
    pub(super) const SND_MEMORY:u32=0x0004;
    #[link(name="winmm")]
    extern "system"{pub(super) fn PlaySoundA(sound:*const u8,module:isize,flags:u32)->i32;}
}
impl Player for HostPlayer{
    #[cfg(windows)]
    fn play(&mut self,wav:&std::sync::Arc<Vec<u8>>){
        // A new PlaySound call ends the one before, so keep the new buffer
        // before it starts and drop the old one only after.
        let keep=std::sync::Arc::clone(wav);
        unsafe{win::PlaySoundA(keep.as_ptr(),0,win::SND_MEMORY|win::SND_ASYNC|win::SND_NODEFAULT);}
        self.playing=Some(keep);
    }
    #[cfg(not(windows))]
    fn play(&mut self,wav:&std::sync::Arc<Vec<u8>>){
        use std::io::Write;
        if self.worker.as_ref().is_some_and(|w|!w.is_finished()){return;}
        let wav=std::sync::Arc::clone(wav);
        // Never block the DOS loop, and never fail the session for sound.
        self.worker=Some(std::thread::spawn(move||{
            let Ok(mut child)=std::process::Command::new("aplay").arg("-q").stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() else{return};
            if let Some(mut input)=child.stdin.take(){let _=input.write_all(&wav);}
            let _=child.wait();
        }));
    }
}
#[cfg(windows)]
impl Drop for HostPlayer{
    fn drop(&mut self){
        // Stop before the buffer goes.
        if self.playing.is_some(){unsafe{win::PlaySoundA(std::ptr::null(),0,0);}}
    }
}
