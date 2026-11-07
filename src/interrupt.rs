//! Architectural interrupt and exception delivery, shared by every runner
//! (the ROM runner, Dine's DOS runner and the SingleStepTests adapter).
//!
//! An instruction that faults commits nothing (PIL is transactional), so a
//! fault is delivered from the state before the instruction, with the
//! instruction's own address as the return address. Software interrupts
//! (INT n, INT3, INTO) return to the next instruction.
//!
//! Only real mode is modelled so far. The vector comes from the interrupt
//! table at IDTR (reset value base 0, limit 03FFh); FLAGS, CS and IP are
//! pushed as 16-bit words through SS:SP, then IF and TF are cleared. Error
//! codes are never pushed in real mode. A frame that cannot be pushed, or a
//! vector beyond the IDTR limit, is reported as a shutdown instead of being
//! delivered as #DF: double-fault delivery is not modelled yet.
use super::address::{MemoryContext,SegmentCache,SegmentRegister};
use super::flow::ExecError;
use super::guest::Image;
use super::memory::{MemWidth,Ram};
use super::Cpu;

/// IDTR after reset: base 0, limit 03FFh (256 four-byte real-mode vectors).
pub const RESET_IDTR:(u16,u32)=(0x03ff,0);

/// The vector of an architectural fault. Model stops (RAM bus faults,
/// unsupported contexts, invalid bindings or control values) have none.
pub fn vector(error:&ExecError)->Option<u8>{
    match error{
        ExecError::Exception{vector,..}=>Some(*vector),
        ExecError::StackSegment{..}=>Some(12),
        ExecError::GeneralProtection{..}=>Some(13),
        _=>None,
    }
}

/// After a divide error has been delivered: Intel lists CF, PF, AF, ZF, SF
/// and OF as undefined for DIV, IDIV and AAM, and the 386 leaves values of
/// its own there. PIL keeps (and pushes) the entry values and marks them
/// undefined, as it does for other undefined flags.
pub fn after_fault(cpu:&mut Cpu,error:&ExecError){
    if matches!(error,ExecError::Exception{vector:0,..}){cpu.undefined_flags|=0x8d5;}
}

/// Short mnemonic for a vector, for reports.
pub fn name(vector:u8)->&'static str{
    match vector{0=>"#DE",1=>"#DB",3=>"#BP",4=>"#OF",5=>"#BR",6=>"#UD",7=>"#NM",8=>"#DF",12=>"#SS",13=>"#GP",14=>"#PF",_=>"INT"}
}

/// The real-mode handler address (segment, offset) for vector `n`.
pub fn real_vector(image:&Image,ram:&Ram,n:u8)->Result<(u16,u16),String>{
    let(limit,base)=image.tables[1];
    let offset=u32::from(n)*4;
    if offset+3>u32::from(limit){return Err(format!("shutdown: vector {n:02x}h is beyond the IDTR limit {limit:04x}h"));}
    let pointer=ram.load(base.wrapping_add(offset),MemWidth::Dword).map_err(|e|format!("shutdown: vector {n:02x}h read: {e:?}"))?;
    Ok(((pointer>>16)as u16,pointer as u16))
}

/// Pushes the real-mode frame and enters `segment:offset`. Every stack word
/// is translated and checked before anything changes; a word that would
/// straddle the end of the stack segment (SP odd and below 6) is the
/// processor shutdown the 386 performs there.
pub fn real_frame(cpu:&mut Cpu,image:&mut Image,ram:&mut Ram,segment:u16,offset:u16,return_ip:u32)->Result<(),String>{
    if !image.real_mode||cpu.eflags&0x20000!=0{return Err("interrupt delivery outside real mode is not modelled".into());}
    let ss=MemoryContext{segment:SegmentRegister::Ss,cache:image.segments[SegmentRegister::Ss.index()]};
    let sp=cpu.regs[4]&0xffff;
    let cs=image.selectors[SegmentRegister::Cs.index()];
    let words=[(2u32,cpu.eflags&0xffff),(4,u32::from(cs)),(6,return_ip&0xffff)];
    let mut writes=[(0u32,0u32);3];
    for(slot,(down,value))in writes.iter_mut().zip(words){
        let at=ss.translate(sp.wrapping_sub(down)&0xffff,MemWidth::Word,true,cpu.eip)
            .map_err(|_|format!("shutdown: interrupt frame at SS:{:04x} crosses the stack segment limit",sp.wrapping_sub(down)&0xffff))?;
        ram.check_write(at,MemWidth::Word).map_err(|e|format!("interrupt frame write: {e:?}"))?;
        *slot=(at,value);
    }
    for(at,value)in writes{ram.store(at,MemWidth::Word,value).map_err(|e|format!("interrupt frame write: {e:?}"))?;}
    cpu.regs[4]=(cpu.regs[4]&0xffff0000)|(sp.wrapping_sub(6)&0xffff);
    cpu.eflags&=!0x300;cpu.undefined_flags&=!0x300;
    let index=SegmentRegister::Cs.index();
    image.selectors[index]=segment;image.segments[index]=SegmentCache::real_mode(segment);
    cpu.eip=u32::from(offset);
    image.retired();
    Ok(())
}

/// Delivers vector `n` in real mode through the IDTR table.
#[cfg_attr(not(test),allow(dead_code))]
pub fn deliver_real(cpu:&mut Cpu,image:&mut Image,ram:&mut Ram,n:u8,return_ip:u32)->Result<(),String>{
    let(segment,offset)=real_vector(image,ram,n)?;
    real_frame(cpu,image,ram,segment,offset,return_ip)
}

#[cfg(test)]
#[path="../tests/unit/interrupt.rs"]
mod tests;
