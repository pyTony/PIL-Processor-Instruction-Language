//! COM1, answered on the host as INT services are: the BIOS serial calls
//! (INT 14h), the 8250 registers at 3F8h-3FFh polled without interrupts, and
//! DOS's AUX device (handle 3, or COM1/AUX opened by name). Bytes the program
//! sends wait in `tx` for the session frontend; bytes the frontend sends wait
//! in `rx` for the program. Only a session has the port.
use super::*;
use std::collections::VecDeque;
/// The port's I/O base, as the BIOS data area lists it.
pub const BASE:u16=0x3f8;
/// Line status: transmitter empty and holding register empty; bit 0 is added
/// while a received byte waits.
const LSR_EMPTY:u8=0x60;
/// Modem status: the frontend is always there, so CTS, DSR and DCD are on.
const MSR:u8=0xb0;
#[derive(Clone,Debug,Default)]
pub struct Serial{
    pub tx:Vec<u8>,pub rx:VecDeque<u8>,
    /// Line control (bit 7 selects the divisor latch), modem control,
    /// scratch and the divisor latch.
    lcr:u8,mcr:u8,scratch:u8,divisor:u16,
    /// The AUX device's information word low byte for INT 21h AX=4400h/4401h:
    /// a device, not at end of input; CTTY sets bits 0-1 (console input and
    /// output) and bit 5 (raw).
    pub info:u8,
}
impl Serial{
    fn lsr(&self)->u8{LSR_EMPTY|u8::from(!self.rx.is_empty())}
}
/// Lists COM1 in the BIOS data area (0040:0000 and one serial port in the
/// equipment word) and gives the session the port.
pub fn attach(p:&mut Loaded)->Result<(),String>{
    let equipment=p.ram.load(0x410,MemWidth::Word).map_err(|e|format!("BIOS equipment read: {e:?}"))?;
    p.ram.store(0x400,MemWidth::Word,u32::from(BASE)).map_err(|e|format!("BIOS COM1 address: {e:?}"))?;
    p.ram.store(0x410,MemWidth::Word,equipment&!0x0e00|0x0200).map_err(|e|format!("BIOS equipment write: {e:?}"))?;
    p.com1=Some(Serial{info:0xc0,..Serial::default()});Ok(())
}
fn port(p:&mut Loaded)->Result<&mut Serial,String>{p.com1.as_mut().ok_or_else(||"COM1 is not attached (only a DOS session has it)".into())}
/// INT 14h for DX=0 (COM1): AH=00h initializes (AL's speed and format are
/// kept, nothing is timed), AH=01h sends AL, AH=02h takes a received byte or
/// reports a timeout at once, AH=03h gives line and modem status.
pub(super) fn bios(p:&mut Loaded,ah:u8)->Result<(),String>{
    if p.cpu.regs[2]&0xffff!=0{return Err(format!("INT 14h port {} is not modelled (only COM1)",p.cpu.regs[2]&0xffff));}
    let al=p.cpu.regs[0]as u8;
    let s=port(p)?;
    let ax=match ah{
        0=>{s.lcr=al&0x1f;u32::from(s.lsr())<<8|u32::from(MSR)},
        1=>{s.tx.push(al);u32::from(s.lsr())<<8|u32::from(al)},
        2=>match s.rx.pop_front(){
            Some(b)=>u32::from(b),
            // No byte: the BIOS answer after its wait runs out, without the wait.
            None=>0x8000|u32::from(s.lsr())<<8&0x1e00,
        },
        3=>u32::from(s.lsr())<<8|u32::from(MSR),
        _=>return Err(format!("INT 14h AH={ah:02x} is not modelled")),
    };
    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|ax;Ok(())
}
/// One byte IN or OUT at 3F8h-3FFh; `None` when the port is not COM1's.
pub(super) fn io(p:&mut Loaded,port_number:u16,input:bool,value:u8)->Result<Option<u8>,String>{
    if !(BASE..BASE+8).contains(&port_number){return Ok(None);}
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
/// DOS write to the AUX device: all bytes go out.
pub(super) fn write(p:&mut Loaded,bytes:&[u8])->Result<u32,String>{port(p)?.tx.extend_from_slice(bytes);Ok(bytes.len()as u32)}
/// DOS read from the AUX device: the bytes received so far, up to `wanted`;
/// with none yet the read waits for the frontend, as a device read waits.
pub(super) fn read(p:&mut Loaded,wanted:usize)->Result<Option<Vec<u8>>,String>{
    let s=port(p)?;
    if s.rx.is_empty(){return Ok(None);}
    let n=wanted.min(s.rx.len());Ok(Some(s.rx.drain(..n).collect()))
}
/// Whether a DOS name is the COM1 device (COM1 or AUX, with any directory
/// or extension), when the session has the port.
pub(super) fn is_device(p:&Loaded,raw:&[u8])->bool{
    p.com1.is_some()&&(device_name(raw,b"COM1")||device_name(raw,b"AUX"))
}
/// Whether a DOS name is the device `name`, with any directory or extension.
pub(super) fn device_name(raw:&[u8],name:&[u8])->bool{
    let base=raw.rsplit(|&c|c==b'\\'||c==b'/'||c==b':').next().unwrap_or(raw);
    base.split(|&c|c==b'.').next().unwrap_or(base).eq_ignore_ascii_case(name)
}
