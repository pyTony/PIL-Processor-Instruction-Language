//! The PC speaker ports, answered on the host as INT services are: PIT
//! channel 2 (count at 42h, mode at 43h) sets the tone and port 61h bits 0
//! and 1 gate it to the speaker. The CMOS clock and setup bytes read through
//! ports 70h (index) and 71h (data). Byte IN and OUT only; any other port, a
//! word access, a PIT read or a CMOS write stops the run.
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
        0x14=>0x20,
        0x15=>0x80,0x16=>0x02,
        0x17|0x18|0x30|0x31=>0,
        _=>return Err(format!("CMOS register {index:02x}h is not modelled")),
    })
}
