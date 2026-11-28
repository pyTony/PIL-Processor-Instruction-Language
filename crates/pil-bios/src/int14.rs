//! INT 14h, the BIOS serial calls, on COM1 (`pil_pc::serial`).
use super::*;
use pil_pc::serial;
/// INT 14h for DX=0 (COM1): AH=00h initializes (AL's speed and format are
/// kept, nothing is timed), AH=01h sends AL, AH=02h takes a received byte or
/// reports a timeout at once, AH=03h gives line and modem status.
pub fn bios<F>(p:&mut Machine<F>,ah:u8)->Result<(),String>{
    if p.cpu.regs[2]&0xffff!=0{return Err(format!("INT 14h port {} is not modelled (only COM1)",p.cpu.regs[2]&0xffff));}
    let al=p.cpu.regs[0]as u8;
    let s=serial::port(p)?;
    let ax=match ah{
        0=>{s.lcr=al&0x1f;u32::from(s.lsr())<<8|u32::from(serial::MSR)},
        1=>{s.tx.push(al);u32::from(s.lsr())<<8|u32::from(al)},
        2=>match s.rx.pop_front(){
            Some(b)=>u32::from(b),
            // No byte: the BIOS answer after its wait runs out, without the wait.
            None=>0x8000|u32::from(s.lsr())<<8&0x1e00,
        },
        3=>u32::from(s.lsr())<<8|u32::from(serial::MSR),
        _=>return Err(format!("INT 14h AH={ah:02x} is not modelled")),
    };
    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|ax;Ok(())
}
