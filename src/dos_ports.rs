//! The PC speaker ports, answered on the host as INT services are: PIT
//! channel 2 (count at 42h, mode at 43h) sets the tone and port 61h bits 0
//! and 1 gate it to the speaker. The CMOS clock and setup bytes read through
//! ports 70h (index) and 71h (data). Byte IN and OUT only; any other port, a
//! word access, unsupported CMOS writes or unsupported PIT use stops the run.
//! Channel 0 supports binary modes 2/3; channel 2 has timed binary modes
//! 0/2/3 and latched reads. Port 3DAh (and 3BAh) reports video status.
use super::*;
#[path="dos_pit.rs"]
mod pit;
#[path="dos_pic.rs"]
pub(super) mod pic;
/// PIT input clock in Hz.
pub(crate) static IGNORE_UNASSIGNED_OUT:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
/// Conventional PC assignments, not an exhaustive list of expansion hardware.
fn device(port:u16)->Option<&'static str>{Some(match port{
    0x00..=0x0f|0x80..=0x8f|0xc0..=0xdf=>"DMA controller/page registers",
    0x20..=0x21|0xa0..=0xa1=>"interrupt controller",
    0x40..=0x43=>"PIT timer",0x60..=0x64=>"keyboard/system control",
    0x70..=0x71=>"CMOS/RTC",0x92=>"system control/A20",
    0xf0..=0xf1=>"coprocessor control",0x170..=0x177|0x1f0..=0x1f7|0x376|0x3f6=>"ATA disk controller",
    0x200..=0x207=>"game/joystick port",0x220..=0x22f=>"Sound Blaster",
    0x278..=0x27f|0x378..=0x37f|0x3bc..=0x3bf=>"parallel port",
    0x2e8..=0x2ef|0x2f8..=0x2ff|0x3e8..=0x3ef|0x3f8..=0x3ff=>"serial port",
    0x3b0..=0x3bb|0x3c0..=0x3df=>"video controller",
    0x3f0..=0x3f5|0x3f7=>"floppy controller",
    _=>return None,
})}
pub(super) const PIT_HZ:u32=1_193_182;
#[derive(Clone,Debug,Default)]
pub struct Speaker{
    /// Port 61h bits 0..3 as last written; bit 4 toggles on each read as
    /// the refresh bit does.
    control:u8,refresh:bool,
    /// Channel 2 access mode (1 low, 2 high, 3 low then high), the count and
    /// whether the high byte comes next.
    access:u8,count:u16,high_next:bool,
    counter2:pit::Counter2,
    pub(super) pic:pic::Pair,
    /// The channel 0 IRQ edge total (BIOS ticks since midnight) last seen;
    /// a different total is a new edge that sets IRR bit 0.
    pub(super) irq_edges:Option<u32>,
    /// Times the speaker was switched on, for a frontend to sound.
    pub beeps:u64,
    pit:pit::Counter0,
    pit_clock:Option<(std::time::Instant,u64)>,
    /// Counter for video status register 1 (port 3DAh/3BAh): toggles horizontal
    /// blanking and cycles vertical retrace so snow-check and sync loops advance.
    retrace:u32,
    ignored_out:std::collections::BTreeSet<u16>,
    fast_a20:u8,kbc_a20:bool,kbc_write_output:bool,kbc_reply:Option<u8>,
    /// Scan codes on their way to port 60h and IRQ1.
    pub(super) kbd:keyboard::Keyboard,
}
impl Speaker{
    fn update_a20(&self,ram:&mut Ram){ram.set_a20_enabled(self.kbc_a20||self.fast_a20&2!=0);}
    pub(super) fn bios_a20(&mut self,ram:&mut Ram,enabled:bool){self.kbc_a20=enabled;if !enabled{self.fast_a20&=!2;}self.update_a20(ram);}
    /// Host PIT time is sampled once, then advances monotonically. Headless
    /// deterministic sessions keep it at zero unless a host clock is selected.
    pub fn pit_now(&mut self,clock:Option<i64>)->u64{
        let Some(offset)=clock else{return 0;};
        let (start,initial)=self.pit_clock.get_or_insert_with(||{
            let ns=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()as i128+i128::from(offset)*1_000_000_000;
            let initial=(ns.rem_euclid(86_400_000_000_000)*i128::from(PIT_HZ)/1_000_000_000)as u64;
            (std::time::Instant::now(),initial)
        });
        *initial+(start.elapsed().as_nanos()*u128::from(PIT_HZ)/1_000_000_000)as u64
    }
    pub fn pit_interrupts(&mut self,clock:Option<i64>)->u64{let now=self.pit_now(clock);self.pit.interrupts(now)}
    /// The tone the speaker plays now in Hz, or 0 when it is off.
    pub fn tone(&self)->u32{
        if self.control&3!=3{return 0;}
        // A count of 0 is 65536.
        PIT_HZ/(if self.count==0{65536}else{u32::from(self.count)})
    }
}
/// Runs the IN or OUT at CS:IP when it is one, returning its length;
/// `Ok(None)` for any other instruction.
/// The port and byte count of the unprefixed IN or OUT at CS:EIP, for the
/// protected-mode I/O permission check before `io` answers it.
fn instruction(p:&Loaded)->Option<(u8,u16,u32,u8)>{
    let mut at=0u32;let mut operand=false;
    let first=loop{
        if at>=15{return None;}
        let byte=p.image.fetch_signature(&p.ram,p.cpu.eip,at+1).ok()?[at as usize];at+=1;
        match byte{0x66=>operand=true,0x67|0x26|0x2e|0x36|0x3e|0x64|0x65|0xf2|0xf3=>{},_=>break byte}
    };
    let port=match first{
        0xe4..=0xe7=>{if at>=15{return None;}let byte=p.image.fetch_signature(&p.ram,p.cpu.eip,at+1).ok()?[at as usize];at+=1;u16::from(byte)},
        0xec..=0xef=>(p.cpu.regs[2]&0xffff)as u16,
        _=>return None,
    };
    let width=if first&1==0{1}else if p.image.segments[1].default_32()^operand{4}else{2};
    Some((first,port,at,width))
}
pub(super) fn access(p:&Loaded)->Option<(u16,u8)>{instruction(p).map(|(_,port,_,width)|(port,width))}
pub(super) fn io(p:&mut Loaded)->Result<Option<u32>,String>{
    let Some((first,port,length,width))=instruction(p)else{return Ok(None)};
    if width==4{return Err(format!("dword I/O at port {port:04x}h is not modelled in Dine"));}
    let input=first&2==0;
    if matches!(port,0x3c0|0x3c1|0x3c4|0x3c5|0x3ce|0x3cf){
        let word=first&1!=0;
        let value=super::planar::io(p,port,input,p.cpu.regs[0]as u16,word)?.unwrap();
        if input{let mask=if word{65535}else{255};p.cpu.regs[0]=(p.cpu.regs[0]&!mask)|(u32::from(value)&mask);}
        return Ok(Some(length));
    }
    if matches!(port,0x3b4|0x3b5|0x3d4|0x3d5|0x3c2|0x3cc|0x3c6..=0x3c9|0x3d9){
        let word=first&1!=0;
        if word&&!matches!(port,0x3b4|0x3d4){return Err("VGA word access must start at CRTC index port 3B4h or 3D4h".into());}
        let mut device=p.vga.clone();
        let lo=device.io(port,input,p.cpu.regs[0] as u8)?.ok_or_else(||format!("unsupported VGA port direction at {port:04X}h"))?;
        let hi=if word{device.io(port+1,input,(p.cpu.regs[0]>>8)as u8)?.unwrap()}else{0};
        p.vga=device;
        if input{let mask=if word{65535}else{255};p.cpu.regs[0]=(p.cpu.regs[0]&!mask)|u32::from(lo)|(u32::from(hi)<<8);}
        return Ok(Some(length));
    }
    let word=first&1!=0;
    let assigned=device(port).or_else(||if word{device(port.wrapping_add(1))}else{None});
    // Only truly unassigned writes qualify; normal device handlers still run.
    if !input&&assigned.is_none()&&!matches!(port,0xe9|0x190)&&IGNORE_UNASSIGNED_OUT.load(std::sync::atomic::Ordering::Relaxed){
        if p.speaker.ignored_out.insert(port){eprintln!("Ignored unassigned OUT port {port:04X}h, width {}, value {:04X}, CS:IP={:04X}:{:04X}",if word{16}else{8},p.cpu.regs[0]&if word{65535}else{255},p.selectors[1],p.cpu.eip);}
        return Ok(Some(length));
    }
    if first&1!=0{return Err(format!("word {} port {port:02x}h is not modelled",if input{"IN from"}else{"OUT to"}));}
    if let Some(value)=serial::io(p,port,input,p.cpu.regs[0]as u8)?{
        if input{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);}
        return Ok(Some(length));
    }
    if port==0x71{
        if !input{
            match p.cmos_index{
                // BIOS shutdown byte: a subsequent CPU reset is separate.
                0x0f=>p.cmos_shutdown=p.cpu.regs[0]as u8,
                // BCD calendar-year writes adjust only this virtual RTC.
                0x09|0x32=>p.rtc.write_year(p.clock_offset,p.cmos_index,p.cpu.regs[0]as u8)?,
                _=>return Err(format!("CMOS write to register {:02x}h is not modelled",p.cmos_index)),
            }
            return Ok(Some(length));
        }
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
    if matches!(port,0x20|0x21|0xa0|0xa1){
        if input{
            // IRR must reflect a pending edge even when IF or the IRQ mask
            // prevents the BIOS timer shortcut from delivering it.
            if port==0x20&&p.clock_offset.is_some(){
                let now=services::sampled_ticks(p);
                services::timer_edge(p,now);
            }
            let value=p.speaker.pic.read(port);p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);
        }else{p.speaker.pic.write(port,p.cpu.regs[0]as u8)?;}
        return Ok(Some(length));
    }
    // Bounded AT keyboard-controller output-port interface and fast A20 gate.
    // CPU reset requests remain explicit stops rather than being ignored.
    if matches!(port,0x60|0x64|0x92){
        let s=&mut p.speaker;
        if input{
            let value=match port{
                // Output full (a controller reply or a scan code); input always ready.
                0x64=>u8::from(s.kbc_reply.is_some()||s.kbd.full()),
                // A controller reply first, else the keyboard's last scan code.
                0x60=>match s.kbc_reply.take(){Some(reply)=>reply,None=>s.kbd.read()},
                _=>s.fast_a20,
            };
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);
        }else{
            let value=p.cpu.regs[0]as u8;
            match port{
                0x92=>{if value&1!=0{return Err("system reset through port 92h is not modelled".into());}s.fast_a20=value;s.update_a20(&mut p.ram);},
                0x64=>match value{
                    0xd0=>{if s.kbc_reply.is_some(){return Err("keyboard-controller output buffer is already full".into());}s.kbc_reply=Some(1|u8::from(s.kbc_a20)<<1);},
                    0xd1=>s.kbc_write_output=true,
                    _=>return Err(format!("keyboard-controller command {value:02x}h is not modelled")),
                },
                0x60 if s.kbc_write_output=>{if value&1==0{return Err("keyboard-controller CPU reset is not modelled".into());}s.kbc_write_output=false;s.kbc_a20=value&2!=0;s.update_a20(&mut p.ram);},
                // AT keyboard diagnostic echo: no ACK precedes the EE reply.
                0x60 if value==0xee=>{if s.kbc_reply.is_some(){return Err("keyboard-controller output buffer is already full".into());}s.kbc_reply=Some(0xee);},
                _=>return Err("OUT 60h without a keyboard-controller output-port command is not modelled".into()),
            }
        }
        return Ok(Some(length));
    }
    // DOS benchmarks turn off diskette motors while measuring. This does
    // not issue a disk command; controller/data accesses remain unsupported.
    if port==0x3f2&&!input&&p.cpu.regs[0]&0xf0==0{return Ok(Some(length));}
    let s=&mut p.speaker;
    if input{
        let value=match port{
            0x00..=0x0f=>0xff,
            0x201=>0xff, // no game adapter: open bus
            0x81..=0x8f=>0xff,
            0x61=>{s.refresh=!s.refresh;s.control|if s.refresh{0x10}else{0}},
            0x3da|0x3ba if !p.vga.status_active(port)=>0xff,
            0x3da|0x3ba=>{
                p.vga.reset_attribute_flip_flop();
                s.retrace=s.retrace.wrapping_add(1);
                let vblank=(s.retrace&0x3f)>=56;
                let hblank=vblank||(s.retrace&1)!=0;
                (if hblank{1}else{0})|(if vblank{8}else{0})
            },
            0x40=>{let now=s.pit_now(p.clock_offset);s.pit.read(now)},
            0x42=>{let now=s.pit_now(p.clock_offset);s.counter2.read(now)?},
                        0x378..=0x37a | 0x278..=0x27a => 0x80,
            _=>return Err(format!("IN from port {port:04x}h ({}) is not modelled",device(port).unwrap_or("unassigned or unknown device"))),
        };
        p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(value);
    }else{
        let value=p.cpu.regs[0]as u8;
        match port{
            0x00..=0x0f=>{},
            0x201=>{}, // no game adapter to receive the write
            0x81..=0x8f=>{},
            0x61=>{
                let was=s.tone()!=0;let now=s.pit_now(p.clock_offset);s.counter2.gate(value&1!=0,now);s.control=value&0x0f;
                if !was&&s.tone()!=0{s.beeps+=1;}
            },
            // Channel 2 mode: access in bits 5..4; the counting mode is not timed.
            0x43 if value>>6==2=>{let now=s.pit_now(p.clock_offset);s.counter2.command(value,now)?;if value&0x30!=0{s.access=value>>4&3;s.high_next=s.access==2;}},
            0x43 if value>>6==0=>{let now=s.pit_now(p.clock_offset);s.pit.command(value,now)?;},
            0x43=>return Err(format!("PIT command {value:02x}h is not modelled")),
            0x40=>{let now=s.pit_now(p.clock_offset);s.pit.write(value,now)?;},
            0x42 if s.access==0=>return Err("PIT channel 2 count written before its mode".into()),
            0x42=>{
                let now=s.pit_now(p.clock_offset);s.counter2.write(value,now);
                if s.high_next{s.count=(s.count&0xff)|u16::from(value)<<8;}else{s.count=(s.count&0xff00)|u16::from(value);}
                if s.access==3{s.high_next=!s.high_next;}
            },
            0x80 | 0x190 => {
                eprintln!("  [POST] Stage 0x{:02X} (CS:IP={:04X}:{:04X})", value, p.selectors[1], p.cpu.eip);
            },
            0xe9 | 0x378..=0x37a | 0x278..=0x27a => {},
            _=>return Err(format!("OUT to port {port:04x}h ({}) is not modelled",device(port).unwrap_or("unassigned or unknown device"))),
        }
    }
    Ok(Some(length))
}
/// One CMOS byte as an AT with no diskette or hard disk, 640 KiB base memory
/// and no extended memory would hold it. The clock registers are BCD in 24-hour
/// form: the host's local time with the session clock, else midnight on
/// Tuesday 1 January 1980. Direct RTC year changes are session-local and do
/// not alter the DOS-maintained clock or the host clock. Bit 7 of register
/// 0Ah (update in progress) stays clear, so a read never has to wait.
fn cmos(p:&Loaded,index:u8)->Result<u8,String>{
    if let Some(value)=p.rtc.read(p.clock_offset,index){return Ok(value);}
    Ok(match index{
        // Alarm registers: no alarm set.
        0x01|0x03|0x05=>0,
        // Divider 32.768 kHz, 1024 Hz rate; 24-hour BCD; no flags; battery good.
        0x0a=>0x26,0x0b=>0x02,0x0c=>0x00,0x0d=>0x80,
        // Diagnostics passed, normal shutdown, no diskette, no hard disk.
        0x0e|0x10|0x11|0x12|0x13=>0,0x0f=>p.cmos_shutdown,
        // Equipment: CGA 80 columns, no diskette, no coprocessor.
        0x14=>p.fpu.equipment(0x20) as u8,
        0x15=>0x80,0x16=>0x02,
        // No expansion card or pending setup notice in the POST information byte.
        0x17|0x30=>(p.ram.bytes().len().saturating_sub(0x100000)/1024).min(0xffff)as u8,
        0x18|0x31=>((p.ram.bytes().len().saturating_sub(0x100000)/1024).min(0xffff)>>8)as u8,
        0x33=>0,
        _=>return Err(format!("CMOS register {index:02x}h is not modelled")),
    })
}


#[cfg(test)]
mod timer_tests{
    use super::*;
    #[test]fn programmed_edges_drive_bios_ticks_and_obey_interrupt_shadows(){
        let mut p=load(&[0xfa,0xf4],"pit.com").unwrap();p.speaker.pit.command(0x34,0).unwrap();p.speaker.pit.write(0x0b,0).unwrap();p.speaker.pit.write(0xe9,0).unwrap();
        let now=p.speaker.pit.interrupts(1+3*0xe90b)as u32;assert_eq!(now,3);
        services::timer_interrupt_at(&mut p,now).unwrap();assert_eq!(p.timer_ticks,3);assert_eq!(p.ram.load(0x46c,MemWidth::Dword).unwrap(),3);
        for shadow in 0..3{p.cpu.eflags=if shadow==0{2}else{0x202};p.image.sti_shadow=u8::from(shadow==1);p.image.ss_shadow=u8::from(shadow==2);services::timer_interrupt_at(&mut p,4).unwrap();assert_eq!(p.timer_ticks,3);}
        p.image.ss_shadow=0;p.image.sti_shadow=0;p.cpu.eflags=0x202;services::timer_interrupt_at(&mut p,4).unwrap();assert_eq!(p.timer_ticks,4);
    }
}

