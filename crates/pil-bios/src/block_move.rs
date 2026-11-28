//! INT 15h extended-memory services beyond AH=88h: the AT BIOS block move
//! (AH=87h) and the two-range memory size (AX=E801h).
//!
//! The block move is a host-side copy. An AT BIOS enters protected mode with
//! the caller's table at ES:SI as its GDT, loads the source descriptor (table
//! offset 10h) into DS and the destination descriptor (18h) into ES, and runs
//! REP MOVSW from offset 0 for CX words. The checks here are the ones that
//! copy would fault on (the BIOS then returns CF=1, AH=02h): a descriptor
//! that is not present, not a data segment (or readable code as the source),
//! not writable as the destination, or whose limit is below 2*CX-1.
//! Choices where BIOSes differ or the service is not defined:
//! - The A20 gate is left as the caller had it (SeaBIOS, DOSBox); the IBM AT
//!   BIOS leaves it disabled.
//! - More than 8000h words, a call outside real mode and an address
//!   outside installed RAM stop the run. Words written to ROM (C0000h-
//!   FFFFFh) are dropped, as a CPU store there is.
use super::*;

/// One caller descriptor: (base, limit, access byte). The 386 fields are
/// honoured: limit bits 19-16 and G in byte 6, base bits 31-24 in byte 7.
fn descriptor<F>(p:&Machine<F>,at:u32)->Result<(u32,u32,u8),String>{
    let mut b=[0u8;8];
    for (i,byte) in b.iter_mut().enumerate(){
        *byte=p.ram.load(at.wrapping_add(i as u32),MemWidth::Byte).map_err(|e|format!("INT 15h AH=87h descriptor read: {e:?}"))?as u8;
    }
    let raw=u32::from(b[0])|u32::from(b[1])<<8|u32::from(b[6]&0x0f)<<16;
    let limit=if b[6]&0x80!=0{(raw<<12)|0xfff}else{raw};
    Ok((u32::from(b[2])|u32::from(b[3])<<8|u32::from(b[4])<<16|u32::from(b[7])<<24,limit,b[5]))
}
/// Whether REP MOVSW over `bytes` bytes from offset 0 runs without a fault.
fn usable(limit:u32,access:u8,bytes:u32,write:bool)->bool{
    if access&0x90!=0x90{return false;} // not present, or a system descriptor
    let code=access&0x08!=0;
    if write&&(code||access&0x02==0){return false;}
    if !write&&code&&access&0x02==0{return false;} // execute-only code
    if bytes==0{return true;}
    // Expand-down data is valid above its limit only, so offset 0 faults.
    if !code&&access&0x04!=0{return false;}
    bytes-1<=limit
}
fn fail<F>(p:&mut Machine<F>){
    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff00ff)|0x0200;
    p.cpu.eflags=(p.cpu.eflags|1)&!0x40;p.cpu.undefined_flags&=!0x41;
}
pub fn block_move<F>(p:&mut Machine<F>)->Result<(),String>{
    if !p.image.real_mode{return Err("INT 15h AH=87h block move outside real mode is not modelled".into());}
    let words=p.cpu.regs[1]&0xffff;
    if words>0x8000{return Err(format!("INT 15h AH=87h block move of {words:04X}h words: more than 8000h is not modelled"));}
    let bytes=words*2;
    let table=p.image.segments[SegmentRegister::Es.index()].base.wrapping_add(p.cpu.regs[6]&0xffff);
    let (source,source_limit,source_access)=descriptor(p,table.wrapping_add(0x10))?;
    let (target,target_limit,target_access)=descriptor(p,table.wrapping_add(0x18))?;
    if !usable(source_limit,source_access,bytes,false)||!usable(target_limit,target_access,bytes,true){fail(p);return Ok(());}
    let installed=p.ram.bytes().len()as u64;
    for (name,base) in [("source",source),("destination",target)]{
        if u64::from(base)+u64::from(bytes)>installed{
            return Err(format!("INT 15h AH=87h block move {name} {base:08X}h+{bytes:X}h is outside the {} MiB of RAM",installed>>20));
        }
    }
    // The copy sees physical memory with A20 enabled, whatever the gate says.
    let gate=p.ram.a20_enabled();p.ram.set_a20_enabled(true);
    let result=copy(p,source,target,words);
    p.ram.set_a20_enabled(gate);result?;
    p.cpu.regs[0]&=0xffff00ff;
    p.cpu.eflags=(p.cpu.eflags&!1)|0x40;p.cpu.undefined_flags&=!0x41;
    Ok(())
}
/// Forward word by word, as REP MOVSW with DF=0 does, so overlapping ranges
/// give what the BIOS gives. Every destination word is checked first:
/// nothing is written when any of it is read-only.
fn copy<F>(p:&mut Machine<F>,source:u32,target:u32,words:u32)->Result<(),String>{
    for i in 0..words{
        p.ram.check_write(target+i*2,MemWidth::Word).map_err(|_|format!("INT 15h AH=87h block move destination {:08X}h is read-only memory",target+i*2))?;
    }
    for i in 0..words{
        let word=p.ram.load(source+i*2,MemWidth::Word).map_err(|e|format!("INT 15h AH=87h source read: {e:?}"))?;
        p.ram.store(target+i*2,MemWidth::Word,word).map_err(|e|format!("INT 15h AH=87h destination write: {e:?}"))?;
    }
    Ok(())
}
/// AX=E801h: AX=CX=KiB between 1 MiB and 16 MiB (at most 3C00h), BX=DX=64 KiB
/// blocks above 16 MiB. The same RAM as AH=88h and CMOS 17h/18h/30h/31h.
pub fn memory_size<F>(p:&mut Machine<F>){
    let extended=p.ram.bytes().len().saturating_sub(0x100000);
    let low=(extended/1024).min(0x3c00)as u32;
    let high=(extended.saturating_sub(15<<20)/65536).min(0xffff)as u32;
    for (register,value) in [(0,low),(1,low),(2,high),(3,high)]{p.cpu.regs[register]=(p.cpu.regs[register]&0xffff0000)|value;}
    p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
}
