//! The PC: CPU, memory and the AT devices, apart from any firmware or
//! operating system. A booted OS drives these devices itself; Dine's DOS
//! (crate::dos) and the BIOS (crate::bios) run on top of them.
use crate::Cpu;
use crate::guest::Image;
use crate::memory::Ram;
#[path="pc/ports.rs"]
pub(crate) mod ports;
#[path="pc/soundblaster.rs"]
pub(crate) mod soundblaster;
#[path="pc/rtc.rs"]
pub(crate) mod rtc;
#[path="pc/fdc.rs"]
pub(crate) mod fdc;
#[path="pc/ata.rs"]
pub(crate) mod ata;
#[path="pc/disks.rs"]
pub(crate) mod disks;
#[path="pc/vga.rs"]
pub(crate) mod vga;
#[path="pc/vga_font.rs"]
pub(crate) mod vga_font;
#[path="pc/vga_regs.rs"]
pub(crate) mod vga_regs;
#[path="pc/serial.rs"]
pub(crate) mod serial;
#[path="pc/keyboard.rs"]
pub(crate) mod keyboard;
#[path="pc/history.rs"]
pub(crate) mod history;

/// The machine, with `dos` holding the state of what runs on it: Dine's DOS
/// (`crate::dos::Loaded` is `Machine<Dos>`). Code in this module never looks
/// inside `dos`.
pub struct Machine<F>{
    /// The DOS run by Dine on this machine: its memory chain, processes,
    /// files, console state and drivers. Everything else here is the PC.
    pub dos:F,
    pub cpu:Cpu,
    pub ram:Ram,
    pub image:Image,
    pub selectors:[u16;6],
    pub timer_ticks:u32,
    pub(crate) timer_tick_adjust:i64,
    /// The PC speaker behind ports 42h, 43h and 61h.
    pub speaker:ports::Speaker,
    /// ISA Sound Blaster 2.0 DSP, DMA1 and PCM queue.
    pub(crate) sb:soundblaster::SoundBlaster,
    /// The host's local time, as seconds east of UTC, for the clock (INT 1Ah
    /// AH00, AH2A, AH2C); None keeps the deterministic clock from midnight
    /// on 1 January 1980.
    pub clock_offset:Option<i64>,
    /// The CMOS register that port 71h reads, as port 70h last chose it.
    pub cmos_index:u8,
    pub cmos_shutdown:u8,
    pub(crate) rtc:rtc::Rtc,
    /// COM1 when a session attached it.
    pub com1:Option<serial::Serial>,
    /// Persistent, signature-validated cache across session slices.
    pub cache:crate::cache::DecodeCache,
    /// Raw ROM execution must not use DOS interrupt-entry shortcuts.
    pub(crate) rom_mode:bool,
    /// Replaceable coprocessor state, preserved between session slices.
    pub fpu:crate::fpu::Fpu,
    pub(crate) vga:vga::Vga,
    /// Run-time blocks over the precompiled instruction forms.
    pub(crate) blocks:crate::block_compiler::engine::Blocks,
    /// Opt-in bounded instruction-dispatch history, shared by DOS frontends.
    pub(crate) history:history::History,
    /// Opt-in BIOS and DOS service call trace, independent of instruction history.
    pub(crate) services_trace:history::ServiceTrace,
    /// Disk images the BIOS reads as sector devices (INT 13h), for a boot.
    pub(crate) disks:disks::Disks,
    /// A disk was booted: no DOS runs on this machine, only the BIOS answers
    /// INT n, and a zero vector is a real 0000:0000.
    pub(crate) booted:bool,
    /// The diskette controller and its DMA channel, for a booted OS.
    pub(crate) fdc:fdc::Fdc,
    pub(crate) ata:ata::Ata,
}
impl<F:Clone> Clone for Machine<F>{
    /// A copy starts with an empty decode cache and no blocks.
    fn clone(&self)->Self{Self{dos:self.dos.clone(),
        cpu:self.cpu.clone(),ram:self.ram.clone(),image:self.image.clone(),
        selectors:self.selectors,
        timer_ticks:self.timer_ticks,timer_tick_adjust:self.timer_tick_adjust,
        speaker:self.speaker.clone(),sb:self.sb.clone(),clock_offset:self.clock_offset,
        cmos_index:self.cmos_index,cmos_shutdown:self.cmos_shutdown,rtc:self.rtc.clone(),com1:self.com1.clone(),
        cache:crate::cache::DecodeCache::default(),rom_mode:self.rom_mode,fpu:self.fpu.clone(),vga:self.vga.clone(),blocks:Default::default(),history:self.history.clone(),services_trace:self.services_trace.clone(),disks:self.disks.clone(),booted:self.booted,fdc:self.fdc.clone(),ata:self.ata.clone()}}
}
impl<F> std::fmt::Debug for Machine<F>{
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str("Loaded{..}")}
}
pub(crate) use crate::memory::MemWidth;
pub(crate) use crate::address::SegmentRegister;
pub(crate) use crate::address::MemoryContext;

/// What runs on the machine answers for the few device events whose effect
/// depends on it: Dine's DOS and BIOS (`crate::dos::Dos`) model some of them
/// through BIOS state, a booted OS sees the bare hardware.
pub(crate) trait Firmware:Sized{
    /// Whether keyboard codes go through IRQ1 to a guest INT 09h handler
    /// (else the firmware takes keys straight into its own buffer).
    fn keyboard_irq_owned(p:&Machine<Self>)->Result<bool,String>;
    /// Deliver hardware interrupt `vector` between instructions.
    fn deliver_irq(p:&mut Machine<Self>,vector:u8)->Result<(),String>;
    /// The VGA sequencer, graphics controller and attribute ports (3C0h,
    /// 3C1h, 3C4h, 3C5h, 3CEh, 3CFh).
    fn vga_controller_io(p:&mut Machine<Self>,port:u16,input:bool,value:u16,word:bool)->Result<u16,String>;
}

/// BIOS timer ticks in a day (the PIT's IRQ0 at 1193182/65536 Hz).
pub(crate) const TICKS_PER_DAY:u32=0x0018_00b0;
/// The tick count the PIT has reached on the host clock, with the
/// adjustment a clock set made.
pub(crate) fn sampled_ticks<F>(p:&mut Machine<F>)->u32{
    let raw=p.speaker.pit_interrupts(p.clock_offset)%u64::from(TICKS_PER_DAY);
    (raw as i64+p.timer_tick_adjust).rem_euclid(i64::from(TICKS_PER_DAY))as u32
}
/// A sampled PIT IRQ edge total: a new one sets IRR bit 0. The PIC holds a
/// single request, so edges that pass while one is pending are not counted.
pub(crate) fn timer_edge<F>(p:&mut Machine<F>,now:u32){
    if p.speaker.irq_edges!=Some(now){p.speaker.pic.raise_timer();p.speaker.irq_edges=Some(now);}
}
/// A string as a JSON string literal.
pub(crate) fn json_quote(text:&str)->String{
    let mut out=String::from("\"");
    for c in text.chars(){match c{'"'=>out.push_str("\\\""),'\\'=>out.push_str("\\\\"),'\n'=>out.push_str("\\n"),'\r'=>out.push_str("\\r"),'\t'=>out.push_str("\\t"),c if (c as u32)<0x20=>out.push_str(&format!("\\u{:04x}",c as u32)),c=>out.push(c)}}
    out.push('"');out
}
