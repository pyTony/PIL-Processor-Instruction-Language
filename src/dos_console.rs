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
const COLS:u32=80;
const ROWS:u32=25;
fn byte(p:&Loaded,at:u32)->Result<u32,String>{
    p.ram.load(at,MemWidth::Byte).map_err(|e|format!("BIOS console read: {e:?}"))
}
fn active_page(p:&Loaded)->Result<u32,String>{
    let page=byte(p,0x462)?;
    if page>=8{return Err("BIOS active page outside 0..7".into());}Ok(page)
}
fn cursor(p:&Loaded,page:u32)->Result<(u32,u32),String>{
    let position=read(p,0x450+page*2)?;
    let(row,col)=(position>>8,position&255);
    if row>=ROWS||col>=COLS{return Err("BIOS video cursor outside 80x25 profile".into());}
    Ok((row,col))
}
fn page_cells(p:&Loaded,page:u32)->Result<Vec<u32>,String>{
    (0..CELLS).map(|cell|read(p,VIDEO+page*PAGE+cell*2)).collect()
}
/// Move rows inside an inclusive window; vacated rows become blanks in `attr`.
/// Zero or more lines than the window height clears the whole window.
fn scroll(cells:&mut [u32],window:(u32,u32,u32,u32),lines:u32,attr:u32,up:bool){
    let(top,left,bottom,right)=window;let height=bottom-top+1;
    let lines=if lines==0||lines>height{height}else{lines};
    let rows:Vec<u32>=if up{(top..=bottom).collect()}else{(top..=bottom).rev().collect()};
    for row in rows{
        let source=if up{row+lines}else{row.wrapping_sub(lines)};
        let keep=if up{source<=bottom}else{row>=top+lines};
        for col in left..=right{
            cells[(row*COLS+col)as usize]=if keep{cells[(source*COLS+col)as usize]}else{0x20|(attr<<8)};
        }
    }
}
fn window_writes(page:u32,cells:&[u32],window:(u32,u32,u32,u32))->Vec<(u32,MemWidth,u32)>{
    let(top,left,bottom,right)=window;
    (top..=bottom).flat_map(|row|(left..=right).map(move|col|row*COLS+col))
        .map(|cell|(VIDEO+page*PAGE+cell*2,MemWidth::Word,cells[cell as usize])).collect()
}
/// INT10/AH0E for page BH. Text mode keeps the cell attribute; the blank line
/// after a bottom-row scroll uses the attribute under the incoming cursor.
fn teletype(p:&mut Loaded)->Result<(),String>{
    let ch=p.cpu.regs[0]&255;let page=(p.cpu.regs[3]>>8)&255;
    if page>=8{return Err("BIOS video page must be 0..7".into());}
    // IBM and DOSBox-derived BIOSes differ for TAB; do not choose silently.
    if ch==9{return Err("BIOS teletype TAB policy is not part of this profile".into());}
    let(mut row,mut col)=cursor(p,page)?;let base=VIDEO+page*PAGE;
    let at=base+(row*COLS+col)*2;let mut writes=Vec::new();
    match ch{
        7=>{},
        8=>col=col.saturating_sub(1),
        10=>row+=1,
        13=>col=0,
        _=>{writes.push((at,MemWidth::Byte,ch));col+=1;if col==COLS{col=0;row+=1;}},
    }
    if row==ROWS{
        let attr=read(p,at)?>>8;let mut cells=page_cells(p,page)?;
        if let Some(&(_,_,value))=writes.first(){let cell=((at-base)/2)as usize;cells[cell]=(cells[cell]&0xff00)|value;}
        scroll(&mut cells,(0,0,ROWS-1,COLS-1),1,attr,true);
        writes=window_writes(page,&cells,(0,0,ROWS-1,COLS-1));row=ROWS-1;
    }
    writes.push((0x450+page*2,MemWidth::Word,(row<<8)|col));
    commit(p,&writes)
}
/// INT10/AH06 (up) and AH07 (down) on the active page. Lower-right corners
/// beyond the text page are clamped; an inverted window changes nothing.
fn scroll_service(p:&mut Loaded,up:bool)->Result<(),String>{
    let page=active_page(p)?;
    let(cx,dx)=(p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff);
    let(top,left)=(cx>>8,cx&255);let(bottom,right)=((dx>>8).min(ROWS-1),(dx&255).min(COLS-1));
    if top>bottom||left>right{return Ok(());}
    let mut cells=page_cells(p,page)?;
    scroll(&mut cells,(top,left,bottom,right),p.cpu.regs[0]&255,(p.cpu.regs[3]>>8)&255,up);
    commit(p,&window_writes(page,&cells,(top,left,bottom,right)))
}
/// Fixed mode-3 profile: AH00 accepts mode 3 only. Bit 7 keeps video RAM.
fn set_mode(p:&mut Loaded)->Result<(),String>{
    let al=p.cpu.regs[0]&255;
    if al&0x7f!=3{return Err(format!("BIOS video mode {:02x} is outside the 80x25 colour text profile",al&0x7f));}
    let mut writes=vec![(0x449,MemWidth::Byte,3),(0x44a,MemWidth::Word,COLS),(0x44c,MemWidth::Word,PAGE),
        (0x44e,MemWidth::Word,0),(0x460,MemWidth::Word,0x0607),(0x462,MemWidth::Byte,0),(0x484,MemWidth::Byte,ROWS-1)];
    writes.extend((0..8).map(|page|(0x450+page*2,MemWidth::Word,0)));
    if al&0x80==0{
        writes.extend((0..8).flat_map(|page|(0..CELLS).map(move|cell|(VIDEO+page*PAGE+cell*2,MemWidth::Word,0x0720))));
    }
    commit(p,&writes)
}
fn select_page(p:&mut Loaded)->Result<(),String>{
    let page=p.cpu.regs[0]&255;
    if page>=8{return Err("BIOS video page must be 0..7".into());}
    commit(p,&[(0x462,MemWidth::Byte,page),(0x44e,MemWidth::Word,page*PAGE)])
}
fn video_mode(p:&mut Loaded)->Result<(),String>{
    let(mode,cols,page)=(byte(p,0x449)?,byte(p,0x44a)?,active_page(p)?);
    p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|(cols<<8)|mode;
    p.cpu.regs[3]=(p.cpu.regs[3]&0xffff00ff)|(page<<8);Ok(())
}
pub(super) fn display(p:&mut Loaded,ah:u8)->Result<(),String>{
    match ah{
        0x00=>set_mode(p),0x05=>select_page(p),
        0x06|0x07=>scroll_service(p,ah==6),
        0x0e=>teletype(p),0x0f=>video_mode(p),
        _=>Err(format!("unimplemented BIOS video AH={ah:02x}")),
    }
}
