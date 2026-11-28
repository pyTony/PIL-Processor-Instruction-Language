//! PCM playback for the DOS Sound Blaster DSP through the existing host
//! audio backend. The guest never waits for an audio program or device.
use std::sync::mpsc::{self,SyncSender,TrySendError};
use std::time::Instant;
use super::super::soundblaster::SoundBlaster;
use super::audio;

type Block=(u32,Vec<i16>);

pub(crate) struct Sound{tx:Option<SyncSender<Block>>,worker:Option<std::thread::JoinHandle<()>>}
impl Sound{
    /// speaker_volume=0 mutes both sound devices. Since the PC speaker's
    /// default 5% is intentionally quiet, scale the digital output higher.
    pub(crate) fn host(speaker_volume:u8)->Self{
        if speaker_volume==0{return Self{tx:None,worker:None};}
        let volume=(u16::from(speaker_volume)*10).min(100)as u8;
        let (tx,rx)=mpsc::sync_channel(4);
        let worker=std::thread::Builder::new().name("sound-blaster-pcm".into())
            .spawn(move||play(rx,volume)).ok();
        Self{tx:worker.as_ref().map(|_|tx),worker}
    }
    /// Drain PCM independently of the host's sound support. A frontend
    /// falling behind drops sound, never guest DMA completion or IRQs.
    pub(crate) fn take(&mut self,sb:&mut SoundBlaster){
        for block in sb.take_pcm(){
            if let Some(tx)=&self.tx{
                match tx.try_send(block){
                    Ok(())|Err(TrySendError::Full(_))=>{},
                    Err(TrySendError::Disconnected(_))=>{self.tx=None;break;},
                }
            }
        }
    }
}
impl Drop for Sound{
    fn drop(&mut self){
        self.tx=None;
        if let Some(worker)=self.worker.take(){let _=worker.join();}
    }
}

fn play(rx:mpsc::Receiver<Block>,volume:u8){
    let mut output:Option<Box<dyn audio::Output>>=None;
    let mut rate=0u32;
    let mut since=Instant::now();
    let mut submitted=0u64;
    while let Ok((next_rate,mut samples))=rx.recv(){
        if next_rate==0||samples.is_empty(){continue;}
        if next_rate!=rate{
            output=None;
            rate=next_rate;
            since=Instant::now();
            submitted=0;
        }
        // The guest may advance with synthetic time in headless mode. Bound
        // the host's queued duration so waveOut never grows indefinitely.
        let played=(since.elapsed().as_secs_f64()*f64::from(rate)) as u64;
        if submitted>played+u64::from(rate)/2{continue;}
        if output.is_none(){output=audio::open(rate);}
        let Some(out)=output.as_mut()else{continue};
        for sample in &mut samples{
            *sample=(i32::from(*sample)*i32::from(volume)/100).clamp(-32768,32767) as i16;
        }
        if !out.write(&samples){output=None;continue;}
        submitted=submitted.saturating_add(samples.len() as u64);
    }
}
