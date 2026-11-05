//! The PC speaker ports, answered on the host as INT services are: PIT
//! channel 2 (count at 42h, mode at 43h) sets the tone and port 61h bits 0
//! and 1 gate it to the speaker. The CMOS clock and setup bytes read through
//! ports 70h (index) and 71h (data). Byte IN and OUT only; any other port, a
//! word access, a CMOS write or a PIT use other than these stops the run.
//! PIT channel 0 (40h) can be latched and read, and set to the BIOS mode with
//! its 18.2 Hz count. Port 3DAh (and 3BAh) reports the video status register.
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
    /// Channel 0 (the 18.2 Hz system timer): bytes of a new mode's count
    /// still to come (2, 1 or 0), the low byte written, a latched count and
    /// whether its high byte is read next.
    timer_load:u8,timer_low:u8,latch:Option<u16>,latch_high:bool,
    /// Counter for video status register 1 (port 3DAh/3BAh): toggles horizontal
    /// blanking and cycles vertical retrace so snow-check and sync loops advance.
    retrace:u32,
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
    if let Some(value)=serial::io(p,port,input,p.cpu.regs[0]as u8)?{
        if input{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);}
        return Ok(Some(length));
    }
    if port==0x71{
        if !input{return Err(format!("CMOS write to register {:02x}h is not modelled",p.cmos_index));}
        let value=cmos(p,p.cmos_index)?;
        p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);
        return Ok(Some(length));
    }
    if port==0x70{
        if input{return Err("IN from port 70h is not modelled (the index register is write-only)".into());}
        // Bit 7 masks NMI, which nothing raises here; the index is 6 bits.
        p.cmos_index=p.cpu.regs[0]as u8&0x3f;
        return Ok(Some(length));
    }
    let s=&mut p.speaker;
    if input{
        let value=match port{
            0x00..=0x0f=>0xff,
            0x21|0xa1=>0x00,
            0x81..=0x8f=>0xff,
            0x61=>{s.refresh=!s.refresh;s.control|if s.refresh{0x10}else{0}},
            0x3da|0x3ba=>{
                s.retrace=s.retrace.wrapping_add(1);
                let vblank=(s.retrace&0x3f)>=56;
                let hblank=vblank||(s.retrace&1)!=0;
                (if hblank{1}else{0})|(if vblank{8}else{0})
            },
            0x40 if s.timer_load!=0=>return Err("IN from PIT channel 0 while its count is being written is not modelled".into()),
            0x40=>{
                // Low then high byte, of the latched count when one is held
                // (the latch is released after its high byte), else of the
                // live count.
                let count=match s.latch{Some(c)=>c,None=>timer_count(p.clock_offset)};
                let s=&mut p.speaker;
                let byte=if s.latch_high{(count>>8)as u8}else{count as u8};
                if s.latch_high&&s.latch.is_some(){s.latch=None;}
                s.latch_high=!s.latch_high;byte
            },
                        0x378..=0x37a | 0x278..=0x27a => 0x80,
            _=>return Err(format!("IN from port {port:02x}h is not modelled")),
        };
        p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);
    }else{
        let value=p.cpu.regs[0]as u8;
        match port{
            0x00..=0x0f=>{},
            0x20|0x21|0xa0|0xa1=>{},
            0x81..=0x8f=>{},
            0x61=>{
                let was=s.tone()!=0;s.control=value&0x0f;
                if !was&&s.tone()!=0{s.beeps+=1;}
            },
            // Channel 2 mode: access in bits 5..4; the counting mode is not timed.
            0x43 if value>>6==2&&value&0x30!=0=>{s.access=value>>4&3;s.high_next=s.access==2;},
            // Channel 0 counter latch: holds the count until both bytes are read.
            0x43 if value&0xf0==0=>{if s.latch.is_none(){s.latch=Some(timer_count(p.clock_offset));p.speaker.latch_high=false;}},
            // Channel 0 mode 2 or 3 (rate generator or square wave), binary,
            // low then high byte: as the BIOS programs it. Only its count 0
            // (65536, 18.2 Hz) is taken, so the timer rate never changes.
            0x43 if value>>6==0&&value&0x30==0x30&&matches!(value>>1&7,2|3|6|7)&&value&1==0=>{s.timer_load=2;s.latch=None;s.latch_high=false;},
            0x43=>return Err(format!("PIT command {value:02x}h (other than a channel 2 mode, a channel 0 latch or the BIOS channel 0 mode) is not modelled")),
            0x40 if s.timer_load==2=>{s.timer_low=value;s.timer_load=1;},
            0x40 if s.timer_load==1=>{
                s.timer_load=0;
                if s.timer_low!=0||value!=0{return Err(format!("PIT channel 0 count {:04x}h is not modelled (only 0, the BIOS 18.2 Hz rate)",u16::from(value)<<8|u16::from(s.timer_low)));}
            },
            0x40=>return Err("PIT channel 0 count written before its mode".into()),
            0x42 if s.access==0=>return Err("PIT channel 2 count written before its mode".into()),
            0x42=>{
                if s.high_next{s.count=(s.count&0xff)|u16::from(value)<<8;}else{s.count=(s.count&0xff00)|u16::from(value);}
                if s.access==3{s.high_next=!s.high_next;}
            },
            0x80 | 0x190 => {
                eprintln!("  [POST] Stage 0x{:02X} (CS:IP={:04X}:{:04X})", value, p.selectors[1], p.cpu.eip);
            },
            0xe9 | 0x378..=0x37a | 0x278..=0x27a => {},
            _=>return Err(format!("OUT to port {port:02x}h is not modelled")),
        }
    }
    Ok(Some(length))
}
/// The channel 0 count now: it counts down from 65536 once a timer tick.
/// With the host clock it is the phase of the host's local time within the
/// current tick (to the nanosecond, at 1193182 Hz); the tick count INT 1Ah
/// gives uses the same clock. Reprogramming the BIOS mode keeps that phase
/// rather than restarting it. Without the host clock, time moves only a whole
/// tick at a time, so the count is always at the start of a tick (0, which
/// is 65536).
fn timer_count(clock:Option<i64>)->u16{
    let Some(offset)=clock else{return 0};
    let ns=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0,|d|d.as_nanos()as i128)+i128::from(offset)*1_000_000_000;
    let clocks=ns.rem_euclid(86_400_000_000_000)*1_193_180/1_000_000_000;
    (65536-clocks%65536)as u16
}
/// One CMOS byte as an AT with no diskette or hard disk, 640 KiB base memory
/// and no extended memory would hold it. The clock registers are BCD in 24-hour
/// form: the host's local time with the session clock, else midnight on
/// Tuesday 1 January 1980, as INT 21h AH2A/AH2C give. Bit 7 of register 0Ah
/// (update in progress) stays clear, so a read never has to wait.
fn cmos(p:&Loaded,index:u8)->Result<u8,String>{
    let bcd=|n:u32|((n/10)<<4|n%10)as u8;
    let(year,month,day,weekday,seconds)=match p.clock_offset{
        Some(offset)=>{
            let(days,ms)=services::local_now(offset);
            let(_,date)=files::stamp(Some(std::time::UNIX_EPOCH+std::time::Duration::from_secs(days*86400)));
            (1980+u32::from(date>>9),u32::from(date>>5&15),u32::from(date&31),((days+4)%7)as u32+1,(ms/1000)as u32)
        },
        None=>(1980,1,1,3,0),
    };
    Ok(match index{
        0x00=>bcd(seconds%60),0x02=>bcd(seconds/60%60),0x04=>bcd(seconds/3600),
        // Alarm registers: no alarm set.
        0x01|0x03|0x05=>0,
        0x06=>bcd(weekday),0x07=>bcd(day),0x08=>bcd(month),0x09=>bcd(year%100),0x32=>bcd(year/100),
        // Divider 32.768 kHz, 1024 Hz rate; 24-hour BCD; no flags; battery good.
        0x0a=>0x26,0x0b=>0x02,0x0c=>0x00,0x0d=>0x80,
        // Diagnostics passed, normal shutdown, no diskette, no hard disk.
        0x0e|0x0f|0x10|0x11|0x12|0x13=>0,
        // Equipment: CGA 80 columns, no diskette, no coprocessor.
        0x14=>p.fpu.equipment(0x20) as u8,
        0x15=>0x80,0x16=>0x02,
        0x17|0x18|0x30|0x31=>0,
        _=>return Err(format!("CMOS register {index:02x}h is not modelled")),
    })
}
