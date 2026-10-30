//! The PC speaker ports, answered on the host as INT services are: PIT
//! channel 2 (count at 42h, mode at 43h) sets the tone and port 61h bits 0
//! and 1 gate it to the speaker. Byte IN and OUT only; any other port, a word
//! access or a PIT read stops the run.
use super::*;
/// PIT input clock in Hz.
const PIT_HZ:u32=1_193_182;
#[derive(Clone,Debug,Default)]
pub struct Speaker{
    /// Port 61h bits 0..3 as last written; bit 4 toggles on each read as
    /// the refresh bit does.
    control:u8,refresh:bool,
    /// Channel 2 access mode (1 low, 2 high, 3 low then high), the count and
    /// whether the high byte comes next.
    access:u8,count:u16,high_next:bool,
    /// Times the speaker was switched on, for a frontend to sound.
    pub beeps:u64,
}
impl Speaker{
    /// The tone the speaker plays now in Hz, or 0 when it is off.
    pub fn tone(&self)->u32{
        if self.control&3!=3{return 0;}
        // A count of 0 is 65536.
        PIT_HZ/(if self.count==0{65536}else{u32::from(self.count)})
    }
}
/// Runs the IN or OUT at CS:IP when it is one, returning its length;
/// `Ok(None)` for any other instruction.
pub(super) fn io(p:&mut Loaded)->Result<Option<u32>,String>{
    let Ok(first)=p.image.fetch_signature(&p.ram,p.cpu.eip,1).map(|b|b[0])else{return Ok(None)};
    let(port,length)=match first{
        0xe4..=0xe7=>{
            let Ok(bytes)=p.image.fetch_signature(&p.ram,p.cpu.eip,2)else{return Ok(None)};
            (u16::from(bytes[1]),2)
        },
        0xec..=0xef=>((p.cpu.regs[2]&0xffff)as u16,1),
        _=>return Ok(None),
    };
    let input=first&2==0;
    if first&1!=0{return Err(format!("word {} port {port:02x}h is not modelled",if input{"IN from"}else{"OUT to"}));}
    let s=&mut p.speaker;
    if input{
        let value=match port{
            0x61=>{s.refresh=!s.refresh;s.control|if s.refresh{0x10}else{0}},
            _=>return Err(format!("IN from port {port:02x}h is not modelled")),
        };
        p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);
    }else{
        let value=p.cpu.regs[0]as u8;
        match port{
            0x61=>{
                let was=s.tone()!=0;s.control=value&0x0f;
                if !was&&s.tone()!=0{s.beeps+=1;}
            },
            // Channel 2 mode: access in bits 5..4; the counting mode is not timed.
            0x43 if value>>6==2&&value&0x30!=0=>{s.access=value>>4&3;s.high_next=s.access==2;},
            0x43=>return Err(format!("PIT command {value:02x}h (other than a channel 2 mode) is not modelled")),
            0x42 if s.access==0=>return Err("PIT channel 2 count written before its mode".into()),
            0x42=>{
                if s.high_next{s.count=(s.count&0xff)|u16::from(value)<<8;}else{s.count=(s.count&0xff00)|u16::from(value);}
                if s.access==3{s.high_next=!s.high_next;}
            },
            _=>return Err(format!("OUT to port {port:02x}h is not modelled")),
        }
    }
    Ok(Some(length))
}
