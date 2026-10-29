//! Bounded mode-3 BIOS console. Video RAM and the BIOS keyboard ring are authoritative.
//! No VGA ports, IRQ1, ANSI driver or host UI is modeled here.
use super::*;
const VIDEO:u32=0xb8000;
const PAGE:u32=4096;
const CELLS:u32=80*25;
fn read(p:&Loaded,at:u32)->Result<u32,String>{
    p.ram.load(at,MemWidth::Word).map_err(|e|format!("BIOS console read: {e:?}"))
}
fn commit(p:&mut Loaded,writes:&[(u32,MemWidth,u32)])->Result<(),String>{
    for &(at,width,_) in writes{p.ram.check_write(at,width).map_err(|e|format!("BIOS console write: {e:?}"))?;}
    for &(at,width,value) in writes{p.ram.store(at,width,value).map_err(|e|format!("BIOS console write: {e:?}"))?;}
    Ok(())
}
pub(super) fn initialize(ram:&mut Ram)->Result<(),String>{
    let mut bytes=vec![0;8*PAGE as usize];
    for page in 0..8{for cell in 0..CELLS as usize{
        bytes[page*PAGE as usize+cell*2]=32;bytes[page*PAGE as usize+cell*2+1]=7;
    }}
    ram.write_bytes(VIDEO,&bytes).map_err(|e|format!("video initialization: {e:?}"))?;
    for (at,width,value) in [(0x449,MemWidth::Byte,3),(0x44a,MemWidth::Word,80),
        (0x44c,MemWidth::Word,PAGE),(0x484,MemWidth::Byte,24),
        (0x41a,MemWidth::Word,0x1e),(0x41c,MemWidth::Word,0x1e)]{
        ram.store(at,width,value).map_err(|e|format!("console initialization: {e:?}"))?;
    }Ok(())
}
fn ring(p:&Loaded)->Result<(u32,u32),String>{
    let head=read(p,0x41a)?;let tail=read(p,0x41c)?;
    if [head,tail].iter().any(|n|!(0x1e..0x3e).contains(n)||n&1!=0){
        return Err("invalid BIOS keyboard ring pointers".into());
    }Ok((head,tail))
}
fn next(n:u32)->u32{if n==0x3c{0x1e}else{n+2}}
pub(super) fn waiting(p:&Loaded,vector:u8)->Result<bool,String>{
    if vector!=0x16 || (p.cpu.regs[0]>>8)as u8!=0{return Ok(false);}
    let (head,tail)=ring(p)?;Ok(head==tail)
}
/// Host adapter input: legacy BIOS scan code in AH, character in AL; full is not EOF.
pub(super) fn enqueue(p:&mut Loaded,key:u16)->Result<bool,String>{
    let(head,tail)=ring(p)?;let after=next(tail);
    if after==head{return Ok(false);}
    commit(p,&[(0x400+tail,MemWidth::Word,u32::from(key)),(0x41c,MemWidth::Word,after)])?;
    Ok(true)
}
pub(super) fn keyboard(p:&mut Loaded,ah:u8)->Result<(),String>{
    match ah{
        0|1=>{
            let(head,tail)=ring(p)?;
            if head==tail{
                if ah==0{return Err("BIOS keyboard requires input".into());}
                p.cpu.eflags|=0x40;p.cpu.undefined_flags&=!0x40;return Ok(());
            }
            let key=read(p,0x400+head)?;
            if ah==0{commit(p,&[(0x41a,MemWidth::Word,next(head))])?;}
            else{p.cpu.eflags&=!0x40;p.cpu.undefined_flags&=!0x40;}
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|key;Ok(())
        },
        2=>{
            let flags=p.ram.load(0x417,MemWidth::Byte).map_err(|e|format!("keyboard flags: {e:?}"))?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|flags;Ok(())
        },
        5=>{
            let inserted=enqueue(p,p.cpu.regs[1]as u16)?;
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(!inserted);Ok(())
        },
        _=>Err(format!("unimplemented BIOS keyboard AH={ah:02x}")),
    }
}
pub(super) fn video(p:&mut Loaded,ah:u8)->Result<(),String>{
    let page=(p.cpu.regs[3]>>8)&255;
    if page>=8{return Err("BIOS video page must be 0..7".into());}
    let position=read(p,0x450+page*2)?;
    let(row,col)=(position>>8,position&255);
    if row>=25||col>=80{return Err("BIOS video cursor outside 80x25 profile".into());}
    let first=row*80+col;let base=VIDEO+page*PAGE;
    if ah==8{
        let value=read(p,base+first*2)?;
        p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|value;return Ok(());
    }
    let count=p.cpu.regs[1]&0xffff;
    // Do not invent a BIOS-dependent overflow policy or spill into another page.
    if count>CELLS-first{return Err("BIOS repeated write exceeds text page".into());}
    let ch=p.cpu.regs[0]&255;let attr=p.cpu.regs[3]&255;
    let writes:Vec<_>=(first..first+count).map(|cell|if ah==9{
        (base+cell*2,MemWidth::Word,ch|(attr<<8))
    }else{(base+cell*2,MemWidth::Byte,ch)}).collect();
    commit(p,&writes)
}
