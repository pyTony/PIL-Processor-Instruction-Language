//! COM1: the 8250 registers at 3F8h-3FFh, polled without interrupts. The
//! BIOS serial calls (INT 14h, crate::bios int14) and DOS's AUX device
//! (dos/aux_device.rs) use the same port. Bytes the program sends wait in
//! `tx` for the session frontend; bytes the frontend sends wait in `rx` for
//! the program. Only a session has the port.
use super::*;
use std::collections::VecDeque;
/// The port's I/O base, as the BIOS data area lists it.
pub const BASE:u16=0x3f8;
/// Line status: transmitter empty and holding register empty; bit 0 is added
/// while a received byte waits.
pub(crate) const LSR_EMPTY:u8=0x60;
/// Modem status: the frontend is always there, so CTS, DSR and DCD are on.
pub(crate) const MSR:u8=0xb0;
#[derive(Clone,Debug,Default)]
pub struct Serial{
    pub tx:Vec<u8>,pub rx:VecDeque<u8>,
    /// Line control (bit 7 selects the divisor latch), modem control,
    /// scratch and the divisor latch.
    pub(crate) lcr:u8,pub(crate) mcr:u8,pub(crate) scratch:u8,pub(crate) divisor:u16,
    /// The AUX device's information word low byte for INT 21h AX=4400h/4401h:
    /// a device, not at end of input; CTTY sets bits 0-1 (console input and
    /// output) and bit 5 (raw).
    pub info:u8,
    /// The column DOS output on AUX has reached, as the FreeDOS kernel's
    /// scr_pos counts it: line input there starts a cancelled line below it.
    pub column:usize,
}
impl Serial{
    pub(crate) fn lsr(&self)->u8{LSR_EMPTY|u8::from(!self.rx.is_empty())}
}
/// Lists COM1 in the BIOS data area (0040:0000 and one serial port in the
/// equipment word) and gives the session the port.
pub fn attach<F>(p:&mut Machine<F>)->Result<(),String>{
    let equipment=p.ram.load(0x410,MemWidth::Word).map_err(|e|format!("BIOS equipment read: {e:?}"))?;
    p.ram.store(0x400,MemWidth::Word,u32::from(BASE)).map_err(|e|format!("BIOS COM1 address: {e:?}"))?;
    p.ram.store(0x410,MemWidth::Word,equipment&!0x0e00|0x0200).map_err(|e|format!("BIOS equipment write: {e:?}"))?;
    p.com1=Some(Serial{info:0xc0,..Serial::default()});Ok(())
}
pub(crate) fn port<F>(p:&mut Machine<F>)->Result<&mut Serial,String>{p.com1.as_mut().ok_or_else(||"COM1 is not attached (only a DOS session has it)".into())}
/// One byte IN or OUT at a serial port; `None` when the port is not one.
/// A UART that is not there (COM2-COM4 always, COM1 unless a session
/// attached it) is an open bus: reads give FFh and writes go nowhere, which
/// is how drivers find that the port is absent.
pub(crate) fn io<F>(p:&mut Machine<F>,port_number:u16,input:bool,value:u8)->Result<Option<u8>,String>{
    let com1=(BASE..BASE+8).contains(&port_number);
    let other=matches!(port_number,0x2f8..=0x2ff|0x3e8..=0x3ef|0x2e8..=0x2ef);
    if !com1&&!other{return Ok(None);}
    if other||p.com1.is_none(){return Ok(Some(0xff));}
    let s=port(p)?;let latch=s.lcr&0x80!=0;
    Ok(Some(match(port_number-BASE,input){
        (0,true) if latch=>s.divisor as u8,
        (1,true) if latch=>(s.divisor>>8)as u8,
        (0,false) if latch=>{s.divisor=s.divisor&0xff00|u16::from(value);0},
        (1,false) if latch=>{s.divisor=s.divisor&0xff|u16::from(value)<<8;0},
        (0,true)=>s.rx.pop_front().unwrap_or(0),
        (0,false)=>{s.tx.push(value);0},
        (1,true)=>0,
        (1,false) if value==0=>0,
        (1,false)=>return Err(format!("COM1 interrupts (IER {value:02x}h) are not modelled; poll the line status instead")),
        // No interrupt pending; FIFO control writes are taken and ignored.
        (2,true)=>1,(2,false)=>0,
        (3,true)=>s.lcr,(3,false)=>{s.lcr=value;0},
        (4,true)=>s.mcr,(4,false)=>{s.mcr=value&0x1f;0},
        (5,true)=>s.lsr(),(6,true)=>MSR,
        (5|6,false)=>return Err("COM1 status registers are read-only here".into()),
        (7,true)=>s.scratch,(7,false)=>{s.scratch=value;0},
        _=>unreachable!(),
    }))
}
