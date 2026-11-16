//! A stream of 16-bit mono PCM to the host's sound output, for the PC
//! speaker (dos_host_speaker.rs).
//!
//! Windows: winmm waveOut, buffers queued as they are made. Other hosts:
//! `aplay` (ALSA), else `pacat` (PulseAudio, as Termux has it), reading raw
//! samples on standard input. Without either the session stays silent.

/// Where samples go: the host's sound output, or a test's recorder.
pub(crate) trait Output:Send{
    /// Queue samples; false when the output is gone (then nothing more is sent).
    fn write(&mut self,samples:&[i16])->bool;
}

/// Open the host's sound output at `rate` samples a second, or None.
pub(crate) fn open(rate:u32)->Option<Box<dyn Output>>{
    #[cfg(windows)]{wave_out::WaveOut::open(rate).map(|w|Box::new(w) as Box<dyn Output>)}
    #[cfg(not(windows))]{pipe::Pipe::open(rate).map(|w|Box::new(w) as Box<dyn Output>)}
}

#[cfg(not(windows))]
mod pipe{
    use std::io::Write;
    use std::process::{Child,Command,Stdio};
    /// A player program reading raw samples on standard input.
    pub(super) struct Pipe{child:Child}
    impl Pipe{
        pub(super) fn open(rate:u32)->Option<Self>{
            let rate=rate.to_string();
            // aplay: 100 ms of device buffer, so the sound follows the program.
            let players:[(&str,Vec<String>);2]=[
                ("aplay",["-q","-t","raw","-f","S16_LE","-c","1","-r",&rate,"-B","100000","-"].map(String::from).to_vec()),
                ("pacat",["--raw","--format=s16le","--channels=1",&format!("--rate={rate}"),"--latency-msec=60"].map(String::from).to_vec()),
            ];
            players.iter().find_map(|(program,args)|Command::new(program).args(args).stdin(Stdio::piped())
                .stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()).map(|child|Pipe{child})
        }
    }
    impl super::Output for Pipe{
        fn write(&mut self,samples:&[i16])->bool{
            let Some(input)=self.child.stdin.as_mut() else{return false};
            let bytes:Vec<u8>=samples.iter().flat_map(|s|s.to_le_bytes()).collect();
            input.write_all(&bytes).is_ok()
        }
    }
    impl Drop for Pipe{
        fn drop(&mut self){
            // End of input lets the player finish what it holds and exit.
            drop(self.child.stdin.take());
            let _=self.child.wait();
        }
    }
}

#[cfg(windows)]
mod wave_out{
    use std::collections::VecDeque;
    #[repr(C,packed(1))]
    struct WaveFormatEx{format_tag:u16,channels:u16,samples_per_sec:u32,avg_bytes_per_sec:u32,block_align:u16,bits_per_sample:u16,size:u16}
    #[repr(C)]
    struct WaveHdr{data:*mut u8,buffer_length:u32,bytes_recorded:u32,user:usize,flags:u32,loops:u32,next:*mut WaveHdr,reserved:usize}
    const WAVE_MAPPER:u32=0xffff_ffff;
    const WAVE_FORMAT_PCM:u16=1;
    const WHDR_DONE:u32=1;
    const HDR_SIZE:u32=std::mem::size_of::<WaveHdr>() as u32;
    #[link(name="winmm")]
    extern "system"{
        fn waveOutOpen(handle:*mut isize,device:u32,format:*const WaveFormatEx,callback:usize,instance:usize,flags:u32)->u32;
        fn waveOutPrepareHeader(handle:isize,header:*mut WaveHdr,size:u32)->u32;
        fn waveOutUnprepareHeader(handle:isize,header:*mut WaveHdr,size:u32)->u32;
        fn waveOutWrite(handle:isize,header:*mut WaveHdr,size:u32)->u32;
        fn waveOutReset(handle:isize)->u32;
        fn waveOutClose(handle:isize)->u32;
    }
    /// One queued buffer: the header winmm holds a pointer to, and its samples.
    struct Block{header:Box<WaveHdr>,_samples:Vec<i16>}
    pub(super) struct WaveOut{handle:isize,queued:VecDeque<Block>}
    // The handle and the buffers are used from the audio thread only.
    unsafe impl Send for WaveOut{}
    impl WaveOut{
        pub(super) fn open(rate:u32)->Option<Self>{
            let format=WaveFormatEx{format_tag:WAVE_FORMAT_PCM,channels:1,samples_per_sec:rate,avg_bytes_per_sec:rate*2,block_align:2,bits_per_sample:16,size:0};
            let mut handle=0isize;
            // CALLBACK_NULL: done buffers are found by their WHDR_DONE flag.
            if unsafe{waveOutOpen(&mut handle,WAVE_MAPPER,&format,0,0,0)}!=0{return None;}
            Some(Self{handle,queued:VecDeque::new()})
        }
        /// Release buffers the device has played.
        fn reclaim(&mut self){
            while self.queued.front().is_some_and(|b|unsafe{std::ptr::read_volatile(&b.header.flags)}&WHDR_DONE!=0){
                let mut block=self.queued.pop_front().unwrap();
                unsafe{waveOutUnprepareHeader(self.handle,&mut *block.header,HDR_SIZE);}
            }
        }
    }
    impl super::Output for WaveOut{
        fn write(&mut self,samples:&[i16])->bool{
            self.reclaim();
            if samples.is_empty(){return true;}
            let mut data=samples.to_vec();
            let mut header=Box::new(WaveHdr{data:data.as_mut_ptr().cast(),buffer_length:(data.len()*2) as u32,bytes_recorded:0,user:0,flags:0,loops:0,next:std::ptr::null_mut(),reserved:0});
            unsafe{
                if waveOutPrepareHeader(self.handle,&mut *header,HDR_SIZE)!=0{return false;}
                if waveOutWrite(self.handle,&mut *header,HDR_SIZE)!=0{waveOutUnprepareHeader(self.handle,&mut *header,HDR_SIZE);return false;}
            }
            self.queued.push_back(Block{header,_samples:data});
            true
        }
    }
    impl Drop for WaveOut{
        fn drop(&mut self){
            unsafe{
                // Reset marks every buffer done; then they can be released.
                waveOutReset(self.handle);
                for block in &mut self.queued{waveOutUnprepareHeader(self.handle,&mut *block.header,HDR_SIZE);}
                waveOutClose(self.handle);
            }
        }
    }
}
