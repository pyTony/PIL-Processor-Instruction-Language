//! An XMS 3.0 driver, as DEVICE=HIMEM.SYS (or FreeDOS's HIMEMX) would load
//! it: INT 2Fh AX=4300h finds it (AL=80h), AX=4310h returns its entry in
//! ES:BX, and programs call that entry far with the function in AH.
//!
//! The entry lives in Dine's DOS data segment (RAM, so a program may hook it
//! as XMS 3.0 allows): `JMP SHORT +3; NOP; NOP; NOP`, then a private HLT that
//! the runner catches before decoding, then RETF. A hook that patches the
//! first five bytes with a far JMP chains to the HLT at entry+5.
//!
//! Extended memory blocks (EMBs) come from 1 MiB + 64 KiB (after the HMA) to
//! the end of RAM, first fit in address order. Functions 00h-0Fh and the
//! 32-bit 88h, 89h, 8Eh and 8Fh follow the XMS 3.0 specification; where it
//! leaves room, FreeDOS HIMEMX 3.34 decides (its source was read for
//! behaviour, nothing was copied):
//! - 48 handles. A zero-kilobyte block takes a handle and no memory.
//! - INT 15h AH=88h returns 0 KiB while the driver is installed, so a program
//!   that asks the BIOS does not overwrite EMBs. AX=E801h is not changed.
//! - A20: global enable/disable and a local enable count; global disable
//!   while local enables are counted is AX=0, BL=94h.
//! - Move (0Bh): an odd length is A7h, as is a range past the block's end or
//!   past 10FFF0h for handle 0 (segment:offset). A forward overlap (source
//!   below destination and reaching into it) is A8h. The copy sees physical
//!   memory with A20 on, whatever the gate says.
//! - Reallocate (0Fh/8Fh) grows in place when the memory after the block is
//!   free, otherwise moves the contents to the first block that fits.
//! - Upper memory belongs to DOS (DOS=UMB, see dos_memory): 10h finds none
//!   left (AX=0, BL=B1h, DX=0), 11h and 12h name no block of the driver's
//!   (BL=B2h), as under HIMEM+EMM386 once DOS has taken every UMB. Unknown
//!   functions are AX=0, BL=80h.
//! A call from V86 or protected mode stops the run (not modelled).
use super::*;

/// Offset in DOSDATA of the entry; the HLT Dine catches is 5 bytes on.
const ENTRY:u16=0x03d0;
const CODE:[u8;7]=[0xeb,0x03,0x90,0x90,0x90,0xf4,0xcb];
pub(super) const TRAP:u32=DOSDATA as u32*16+ENTRY as u32+5;
pub(super) const CODE_END:u32=DOSDATA as u32*16+ENTRY as u32+CODE.len()as u32;
const HANDLES:usize=48;
/// The first kilobyte EMBs may use: after the 64 KiB HMA at 1 MiB.
const POOL_START_K:u32=1024+64;
const HMA_END:u32=0x10fff0;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct Block{base_k:u32,size_k:u32,locks:u8}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Xms{installed:bool,hma_used:bool,a20_locks:u32,handles:Vec<Option<Block>>}
impl Default for Xms{
    fn default()->Self{
        // PIL_XMS=off starts sessions without the driver (for comparing runs).
        let installed=!matches!(std::env::var("PIL_XMS").as_deref(),Ok("off"|"0"|"no"|"false"));
        Self{installed,hma_used:false,a20_locks:0,handles:vec![None;HANDLES]}
    }
}
impl Xms{
    pub(super) fn installed(&self)->bool{self.installed}
    pub(super) fn set_installed(&mut self,installed:bool){self.installed=installed;}
}

/// Writes the entry code (always; it is only reachable through AX=4310h).
pub(super) fn install(ram:&mut Ram)->Result<(),String>{
    ram.write_bytes(u32::from(DOSDATA)*16+u32::from(ENTRY),&CODE).map_err(|e|format!("XMS driver entry: {e:?}"))
}

fn set16(p:&mut Loaded,reg:usize,value:u32){p.cpu.regs[reg]=(p.cpu.regs[reg]&0xffff0000)|(value&0xffff);}
fn set_bl(p:&mut Loaded,value:u8){p.cpu.regs[3]=(p.cpu.regs[3]&!0xff)|u32::from(value);}
fn set_bh(p:&mut Loaded,value:u8){p.cpu.regs[3]=(p.cpu.regs[3]&!0xff00)|u32::from(value)<<8;}
fn fail(p:&mut Loaded,code:u8){set16(p,0,0);set_bl(p,code);}
fn ok(p:&mut Loaded){set16(p,0,1);set_bl(p,0);}

/// INT 2Fh AH=43h: AL=00h installation check, AL=10h driver entry.
/// `Ok(false)` when the driver is not installed or AL is another function.
pub(super) fn multiplex(p:&mut Loaded)->Result<bool,String>{
    if !p.dos.xms.installed{return Ok(false);}
    match p.cpu.regs[0]&0xffff{
        0x4300=>{p.cpu.regs[0]=(p.cpu.regs[0]&!0xff)|0x80;Ok(true)},
        0x4310=>{
            let es=SegmentRegister::Es.index();
            p.selectors[es]=DOSDATA;p.image.selectors[es]=DOSDATA;
            p.image.segments[es]=p.image.segments[es].real_mode_reload(DOSDATA);
            set16(p,3,u32::from(ENTRY));Ok(true)
        },
        _=>Ok(false),
    }
}

/// At the entry's HLT: run the function in AH and continue at the RETF.
/// `Ok(None)` anywhere else; `Ok(Some(ah))` names the function served.
pub(super) fn trapped(p:&mut Loaded)->Result<Option<u8>,String>{
    let at=p.image.segments[SegmentRegister::Cs.index()].base.wrapping_add(p.cpu.eip);
    if at!=TRAP{return Ok(None);}
    if !p.image.real_mode{
        if p.cpu.eflags&0x20000!=0{return Err("XMS driver call from V86 mode is not modelled".into());}
        return Ok(None);
    }
    if !p.dos.xms.installed{return Err("XMS driver entry reached while no XMS driver is installed".into());}
    let ah=(p.cpu.regs[0]>>8)as u8;
    call(p,ah)?;
    p.cpu.eip=(p.cpu.eip+1)&0xffff;p.image.retired();
    Ok(Some(ah))
}

fn call(p:&mut Loaded,ah:u8)->Result<(),String>{
    match ah{
        0x00=>{set16(p,0,0x0300);set16(p,3,0x0300);set16(p,2,1);},
        0x01=>if p.dos.xms.hma_used{fail(p,0x91)}else{p.dos.xms.hma_used=true;ok(p)},
        0x02=>if p.dos.xms.hma_used{p.dos.xms.hma_used=false;ok(p)}else{fail(p,0x93)},
        0x03=>{p.speaker.bios_a20(&mut p.ram,true);ok(p)},
        0x04=>global_disable(p),
        0x05=>{p.dos.xms.a20_locks+=1;p.speaker.bios_a20(&mut p.ram,true);ok(p)},
        0x06=>{
            if p.dos.xms.a20_locks==0{fail(p,0x82);}
            else{p.dos.xms.a20_locks-=1;if p.dos.xms.a20_locks>0{fail(p,0x94)}else{global_disable(p)}}
        },
        0x07=>{set16(p,0,u32::from(p.ram.a20_enabled()));set_bl(p,0);},
        0x08|0x88=>{
            let(largest,total)=free(p);
            let none=total==0;
            if ah==0x08{set16(p,0,largest.min(0xffff));set16(p,2,total.min(0xffff));}
            else{
                p.cpu.regs[0]=largest;p.cpu.regs[2]=total;
                p.cpu.regs[1]=(pool_end_k(p)*1024).wrapping_sub(1);
            }
            set_bl(p,if none{0xa0}else{0});
        },
        0x09|0x89=>{
            let size=if ah==0x09{p.cpu.regs[2]&0xffff}else{p.cpu.regs[2]};
            match allocate(p,size){
                Ok(handle)=>{ok(p);set16(p,2,handle);},
                Err(code)=>fail(p,code),
            }
        },
        0x0a=>match handle(p){
            Some((i,b)) if b.locks==0=>{p.dos.xms.handles[i]=None;ok(p)},
            Some(_)=>fail(p,0xab),
            None=>fail(p,0xa2),
        },
        0x0b=>move_block(p)?,
        0x0c=>match handle(p){
            Some((_,b)) if b.locks==u8::MAX=>fail(p,0xac),
            Some((i,b))=>{
                p.dos.xms.handles[i]=Some(Block{locks:b.locks+1,..b});
                let address=b.base_k*1024;
                set16(p,0,1);set16(p,3,address);set16(p,2,address>>16);
            },
            None=>fail(p,0xa2),
        },
        0x0d=>match handle(p){
            Some((_,b)) if b.locks==0=>fail(p,0xaa),
            Some((i,b))=>{p.dos.xms.handles[i]=Some(Block{locks:b.locks-1,..b});ok(p)},
            None=>fail(p,0xa2),
        },
        0x0e|0x8e=>match handle(p){
            Some((_,b))=>{
                let free_handles=p.dos.xms.handles.iter().filter(|h|h.is_none()).count()as u32;
                set16(p,0,1);set_bh(p,b.locks);
                if ah==0x0e{set_bl(p,free_handles.min(255)as u8);set16(p,2,b.size_k.min(0xffff));}
                else{set16(p,1,free_handles);p.cpu.regs[2]=b.size_k;}
            },
            None=>fail(p,0xa2),
        },
        0x0f|0x8f=>{
            let size=if ah==0x0f{p.cpu.regs[3]&0xffff}else{p.cpu.regs[3]};
            match handle(p){
                Some((_,b)) if b.locks!=0=>fail(p,0xab),
                Some((i,_))=>match reallocate(p,i,size)?{Ok(())=>ok(p),Err(code)=>fail(p,code)},
                None=>fail(p,0xa2),
            }
        },
        0x10=>{fail(p,0xb1);set16(p,2,0);},
        0x11|0x12=>fail(p,0xb2),
        _=>fail(p,0x80),
    }
    Ok(())
}

fn global_disable(p:&mut Loaded){
    if p.dos.xms.a20_locks>0{fail(p,0x94);return;}
    p.speaker.bios_a20(&mut p.ram,false);ok(p)
}

fn pool_end_k(p:&Loaded)->u32{(p.ram.bytes().len()/1024)as u32}
/// The handle in DX, when it names an allocated block.
fn handle(p:&Loaded)->Option<(usize,Block)>{handle_value(p,p.cpu.regs[2]&0xffff)}
fn handle_value(p:&Loaded,value:u32)->Option<(usize,Block)>{
    let i=usize::try_from(value).ok()?.checked_sub(1)?;
    p.dos.xms.handles.get(i).copied().flatten().map(|b|(i,b))
}
/// Free gaps (base, size) in kilobytes, in address order, ignoring `skip`.
fn gaps(p:&Loaded,skip:Option<usize>)->Vec<(u32,u32)>{
    let mut used:Vec<(u32,u32)>=p.dos.xms.handles.iter().enumerate()
        .filter(|&(i,_)|Some(i)!=skip)
        .filter_map(|(_,b)|*b).filter(|b|b.size_k>0).map(|b|(b.base_k,b.size_k)).collect();
    used.sort_unstable();
    let mut at=POOL_START_K;let mut out=Vec::new();
    for(base,size)in used{if base>at{out.push((at,base-at));}at=at.max(base+size);}
    let end=pool_end_k(p);if end>at{out.push((at,end-at));}
    out
}
fn free(p:&Loaded)->(u32,u32){
    let g=gaps(p,None);
    (g.iter().map(|g|g.1).max().unwrap_or(0),g.iter().map(|g|g.1).sum())
}
fn allocate(p:&mut Loaded,size_k:u32)->Result<u32,u8>{
    let slot=p.dos.xms.handles.iter().position(Option::is_none).ok_or(0xa1)?;
    let base_k=if size_k==0{0}else{gaps(p,None).into_iter().find(|g|g.1>=size_k).ok_or(0xa0)?.0};
    p.dos.xms.handles[slot]=Some(Block{base_k,size_k,locks:0});
    Ok(slot as u32+1)
}
fn reallocate(p:&mut Loaded,i:usize,size_k:u32)->Result<Result<(),u8>,String>{
    let b=p.dos.xms.handles[i].expect("checked handle");
    if size_k<=b.size_k||b.size_k==0&&size_k==0{
        p.dos.xms.handles[i]=Some(Block{size_k,base_k:if size_k==0{0}else{b.base_k},..b});
        return Ok(Ok(()));
    }
    let g=gaps(p,Some(i));
    // In place when the block (or what follows it) can grow far enough.
    if b.size_k>0&&g.iter().any(|&(base,size)|base<=b.base_k&&b.base_k+size_k<=base+size){
        p.dos.xms.handles[i]=Some(Block{size_k,..b});return Ok(Ok(()));
    }
    // Elsewhere: the copy goes through a buffer, so the new place may
    // overlap the old one.
    let Some(&(base,_))=g.iter().find(|&&(_,size)|size>=size_k)else{return Ok(Err(0xa0));};
    if b.size_k>0{copy(p,b.base_k*1024,base*1024,b.size_k*1024)?;}
    p.dos.xms.handles[i]=Some(Block{base_k:base,size_k,..b});
    Ok(Ok(()))
}

/// One side of a move: the linear address of `length` bytes at handle:offset,
/// or the XMS error for it (`bad_handle`, then `bad_handle+1` for the offset).
fn side(p:&Loaded,handle:u32,offset:u32,length:u32,bad_handle:u8)->Result<u32,u8>{
    if handle==0{
        // FFFF:FFFF is the highest a segment:offset reaches; only the
        // length can pass 10FFF0h.
        let start=(offset>>16)*16+(offset&0xffff);
        if u64::from(start)+u64::from(length)>u64::from(HMA_END){return Err(0xa7);}
        return Ok(start);
    }
    let(_,b)=handle_value(p,handle).ok_or(bad_handle)?;
    let size=u64::from(b.size_k)*1024;
    if u64::from(offset)>size{return Err(bad_handle+1);}
    if u64::from(offset)+u64::from(length)>size{return Err(0xa7);}
    Ok(b.base_k*1024+offset)
}
fn move_block(p:&mut Loaded)->Result<(),String>{
    let table=p.image.segments[SegmentRegister::Ds.index()].base.wrapping_add(p.cpu.regs[6]&0xffff);
    let read=|p:&Loaded,at:u32,width|p.ram.load(table.wrapping_add(at),width).map_err(|e|format!("XMS move structure read: {e:?}"));
    let length=read(p,0,MemWidth::Dword)?;
    let(src_handle,src_offset)=(read(p,4,MemWidth::Word)?,read(p,6,MemWidth::Dword)?);
    let(dst_handle,dst_offset)=(read(p,10,MemWidth::Word)?,read(p,12,MemWidth::Dword)?);
    if length&1!=0{fail(p,0xa7);return Ok(());}
    let dst=match side(p,dst_handle,dst_offset,length,0xa5){Ok(a)=>a,Err(code)=>{fail(p,code);return Ok(());}};
    let src=match side(p,src_handle,src_offset,length,0xa3){Ok(a)=>a,Err(code)=>{fail(p,code);return Ok(());}};
    if src<dst&&src+length>dst{fail(p,0xa8);return Ok(());}
    if length>0&&src!=dst{copy(p,src,dst,length)?;}
    ok(p);Ok(())
}
/// A copy of physical memory with A20 on. Video memory goes byte by byte
/// through the planar window; ROM bytes stay; read-only memory stops the run.
fn copy(p:&mut Loaded,src:u32,dst:u32,length:u32)->Result<(),String>{
    let gate=p.ram.a20_enabled();p.ram.set_a20_enabled(true);
    let result=copy_a20(p,src,dst,length);
    p.ram.set_a20_enabled(gate);result
}
fn copy_a20(p:&mut Loaded,src:u32,dst:u32,length:u32)->Result<(),String>{
    let video=|a:u32|a<0xc0000&&a+length>0xa0000;
    let mut bytes=Vec::with_capacity(length as usize);
    if video(src){
        for i in 0..length{bytes.push(p.ram.load(src+i,MemWidth::Byte).map_err(|e|format!("XMS move source read: {e:?}"))?as u8);}
    }else{
        bytes.extend_from_slice(p.ram.contiguous_slice(src,length as usize).ok_or_else(||format!("XMS move source {src:08X}h+{length:X}h is outside RAM"))?);
    }
    // Nothing is written when any destination byte is read-only.
    for i in (0..length).step_by(2){
        p.ram.check_write(dst+i,if length-i>=2{MemWidth::Word}else{MemWidth::Byte}).map_err(|_|format!("XMS move destination {:08X}h is read-only memory",dst+i))?;
    }
    if video(dst){
        for(i,&b)in bytes.iter().enumerate(){p.ram.store(dst+i as u32,MemWidth::Byte,u32::from(b)).map_err(|e|format!("XMS move destination write: {e:?}"))?;}
        Ok(())
    }else{
        p.ram.write_bytes(dst,&bytes).map_err(|e|format!("XMS move destination write: {e:?}"))
    }
}

#[cfg(test)]
#[path="../tests/unit/dos_xms.rs"]
mod tests;
