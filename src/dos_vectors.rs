//! The interrupt vector table as programs see it. Each vector Dine answers
//! starts at a three-byte entry in the BIOS area (F000:F000 + 3*n): INT n,
//! then IRET. While a vector still holds its entry (or 0, as vectors Dine has
//! no entry for start), INT n is answered by Dine on the spot, as before. When
//! a program has put its own handler there (AH25h or a direct write), INT n
//! is a real-mode interrupt: FLAGS, CS and the return IP are pushed, IF and TF
//! are cleared, and the handler runs. A handler that chains to the address it
//! saved reaches the entry, where Dine answers the call; leaving the entry
//! then returns to the handler's caller with the caller's IF, TF, DF and IOPL
//! and the status flags (CF, PF, AF, ZF, SF, OF) the service left, as DOS and
//! BIOS handlers return them (RETF 2 or a patched IRET frame).
use super::*;
/// Linear address of vector 0's entry; vector n's is 3*n bytes on.
pub(super) const ENTRIES:u32=0xff000;
const SEGMENT:u16=0xf000;
const FIRST:u16=0xf000;
/// The vectors that start at an entry: the timer and its user hook, the BIOS
/// video, equipment, memory, disk, serial, system, keyboard, printer and
/// clock services, Ctrl-Break, and the DOS calls (20h, 21h, idle 28h, fast
/// console 29h, network 2Ah, multiplex 2Fh). Data pointers (1Dh-1Fh, 41h,
/// 43h, 46h), the DOS-kept 22h-24h and the free and user vectors stay as
/// they were.
pub(super) const ANSWERED:[u8;18]=[0x08,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x1a,0x1b,0x1c,0x20,0x21,0x28,0x29,0x2a,0x2f];
/// End of the entry area, for the fetch window.
pub(super) const END:u32=ENTRIES+3*256;
/// The far pointer of vector n's entry.
pub(super) fn entry(n:u8)->u32{u32::from(SEGMENT)<<16|(u32::from(FIRST)+3*u32::from(n))}
/// Writes the entries (into the read-only BIOS area, before it is protected)
/// and points the answered vectors at them.
pub(super) fn install(ram:&mut Ram)->Result<(),String>{
    let mut code=vec![0u8;3*256];
    for n in 0..=255u8{code[3*usize::from(n)..][..3].copy_from_slice(&[0xcd,n,0xcf]);}
    ram.write_bytes(ENTRIES,&code).map_err(|e|format!("interrupt entries: {e:?}"))?;
    for n in ANSWERED{ram.store(u32::from(n)*4,MemWidth::Dword,entry(n)).map_err(|e|format!("INT {n:02x}h vector: {e:?}"))?;}
    Ok(())
}
/// Whether vector n still leads to Dine: its entry, or 0 for a vector that
/// has none.
pub(super) fn answered(p:&Loaded,n:u8)->Result<bool,String>{
    let vector=p.ram.load(u32::from(n)*4,MemWidth::Dword).map_err(|e|format!("INT {n:02x}h vector read: {e:?}"))?;
    Ok(vector==0||vector==entry(n))
}
fn linear(p:&Loaded)->u32{p.image.segments[SegmentRegister::Cs.index()].base.wrapping_add(p.cpu.eip)}
/// Whether CS:IP is at an entry's INT n, so Dine answers whatever the vector holds.
pub(super) fn at_entry(p:&Loaded)->bool{let at=linear(p);(ENTRIES..END).contains(&at)&&(at-ENTRIES)%3==0}
fn set_cs(p:&mut Loaded,selector:u16){
    let cs=SegmentRegister::Cs.index();
    p.selectors[cs]=selector;p.image.selectors[cs]=selector;p.image.segments[cs]=SegmentCache::real_mode(selector);
}
fn stack(p:&Loaded,offset:u32,write:bool)->Result<u32,String>{
    let context=MemoryContext{segment:SegmentRegister::Ss,cache:p.image.segments[SegmentRegister::Ss.index()]};
    context.translate(offset&0xffff,MemWidth::Word,write,p.cpu.eip).map_err(|e|format!("interrupt stack access: {e:?}"))
}
/// INT n at CS:IP (length 2) to a program's handler. All three stack words
/// and the target are checked before anything changes.
pub(super) fn deliver(p:&mut Loaded,n:u8)->Result<(),String>{
    if p.cpu.regs[4]&0xffff<6&&p.cpu.regs[4]&0xffff!=0{return Err("interrupt frame would wrap the stack segment".into());}
    let vector=p.ram.load(u32::from(n)*4,MemWidth::Dword).map_err(|e|format!("INT {n:02x}h vector read: {e:?}"))?;
    let(segment,offset)=((vector>>16)as u16,vector&0xffff);
    let target=u32::from(segment)*16+offset;let image=&p.image;
    let fetchable=(target>=image.fetch_base+image.start&&target<image.fetch_base+image.end)
        ||(target>=image.system.0&&target<image.system.1);
    if !fetchable{return Err(format!("INT {n:02x}h handler at {segment:04x}:{offset:04x} is outside the running program"));}
    let sp=p.cpu.regs[4]&0xffff;
    let values=[(2,p.cpu.eflags&0xffff),(4,u32::from(p.selectors[SegmentRegister::Cs.index()])),(6,(p.cpu.eip+2)&0xffff)];
    let mut writes=Vec::new();
    for(k,value)in values{writes.push((stack(p,sp.wrapping_sub(k),true)?,value));}
    for(at,_)in &writes{p.ram.check_write(*at,MemWidth::Word).map_err(|e|format!("interrupt stack write: {e:?}"))?;}
    for(at,value)in writes{p.ram.store(at,MemWidth::Word,value).map_err(|e|format!("interrupt stack write: {e:?}"))?;}
    p.cpu.regs[4]=(p.cpu.regs[4]&0xffff0000)|(sp.wrapping_sub(6)&0xffff);
    p.cpu.eflags&=!0x300;p.cpu.undefined_flags&=!0x300;
    set_cs(p,segment);p.cpu.eip=offset;p.image.retired();
    Ok(())
}
/// At an entry's IRET: return to the caller with its own control flags and
/// the status flags the service left. `Ok(false)` anywhere else.
pub(super) fn leave(p:&mut Loaded)->Result<bool,String>{
    let at=linear(p);
    if !(ENTRIES..END).contains(&at)||(at-ENTRIES)%3!=2{return Ok(false);}
    let sp=p.cpu.regs[4]&0xffff;let mut words=[0u32;3];
    for(k,word)in words.iter_mut().enumerate(){
        let address=stack(p,sp.wrapping_add(2*k as u32),false)?;
        *word=p.ram.load(address,MemWidth::Word).map_err(|e|format!("interrupt stack read: {e:?}"))?;
    }
    let[ip,cs,flags]=words;
    const STATUS:u32=0x08d5;
    p.cpu.eflags=(p.cpu.eflags&0xffff0000)|(flags&0x7fd5&!STATUS)|(p.cpu.eflags&STATUS)|2;
    p.cpu.regs[4]=(p.cpu.regs[4]&0xffff0000)|(sp.wrapping_add(6)&0xffff);
    set_cs(p,cs as u16);p.cpu.eip=ip;p.image.retired();
    Ok(true)
}
#[cfg(test)]
#[path="../tests/unit/dos_vectors.rs"]
mod tests;
