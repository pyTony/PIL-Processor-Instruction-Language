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
/// Blocking reads stop without retiring. AH0C flushes once before its first
/// wait, so resuming after a key arrives does not discard that key.
pub(super) fn waiting(p:&mut Loaded,vector:u8)->Result<bool,String>{
    let(ah,al)=((p.cpu.regs[0]>>8)as u8,p.cpu.regs[0]as u8);
    match(vector,ah){
        (0x16,0)=>{let(head,tail)=ring(p)?;Ok(head==tail)},
        (0x21,1|7|8)=>Ok(!dos_available(p)?),
        (0x21,0x0c) if matches!(al,1|7|8)=>{
            if !p.flushed{let writes=dos_flush_writes(p)?;commit(p,&writes)?;p.pending_scan=None;p.flushed=true;}
            Ok(!dos_available(p)?)
        },
        _=>Ok(false),
    }
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
type Writes=Vec<(u32,MemWidth,u32)>;
/// Teletype bytes onto a copy of `page`. Text mode keeps cell attributes; a
/// bottom-row scroll fills with the attribute under the cursor before that byte.
/// DOS callers expand TAB from the screen column; BIOS AH0E rejects it because
/// IBM-derived and DOSBox-derived BIOSes differ.
fn teletype_plan(p:&Loaded,page:u32,bytes:&[u8],dos_tabs:bool)->Result<Writes,String>{
    if page>=8{return Err("BIOS video page must be 0..7".into());}
    if !dos_tabs&&bytes.contains(&9){return Err("BIOS teletype TAB policy is not part of this profile".into());}
    let(mut row,mut col)=cursor(p,page)?;let mut cells=page_cells(p,page)?;
    let(mut dirty,mut scrolled)=(Vec::new(),false);
    for &b in bytes{
        let n=if b==9{8-col%8}else{1};
        for _ in 0..n{
            let ch=if b==9{32}else{u32::from(b)};let incoming=(row*COLS+col)as usize;
            match ch{
                7=>{},
                8=>col=col.saturating_sub(1),
                10=>row+=1,
                13=>col=0,
                _=>{cells[incoming]=(cells[incoming]&0xff00)|ch;dirty.push(incoming);col+=1;if col==COLS{col=0;row+=1;}},
            }
            if row==ROWS{
                let attr=cells[incoming]>>8;
                scroll(&mut cells,(0,0,ROWS-1,COLS-1),1,attr,true);row=ROWS-1;scrolled=true;
            }
        }
    }
    let base=VIDEO+page*PAGE;
    let mut writes=if scrolled{window_writes(page,&cells,(0,0,ROWS-1,COLS-1))}
        else{dirty.into_iter().map(|cell|(base+cell as u32*2,MemWidth::Byte,cells[cell]&255)).collect()};
    writes.push((0x450+page*2,MemWidth::Word,(row<<8)|col));
    Ok(writes)
}
fn teletype(p:&mut Loaded)->Result<(),String>{
    let writes=teletype_plan(p,(p.cpu.regs[3]>>8)&255,&[p.cpu.regs[0]as u8],false)?;
    commit(p,&writes)
}
/// DOS console output also reaches the active page, as through INT29/AH0E.
pub(super) fn dos_screen(p:&Loaded,bytes:&[u8])->Result<Writes,String>{
    let writes=teletype_plan(p,active_page(p)?,bytes,true)?;
    check(p,&writes)?;Ok(writes)
}
pub(super) fn check(p:&Loaded,writes:&[(u32,MemWidth,u32)])->Result<(),String>{
    for &(at,width,_) in writes{p.ram.check_write(at,width).map_err(|e|format!("BIOS console write: {e:?}"))?;}
    Ok(())
}
/// Store a write set that `check` already accepted.
pub(super) fn store(p:&mut Loaded,writes:&[(u32,MemWidth,u32)])->Result<(),String>{commit(p,writes)}
/// DOS stdin is CON on the BIOS ring. An ASCII-0 key returns 00h, then its scan
/// code on the next read, as single-byte DOS reads do.
pub(super) fn dos_available(p:&Loaded)->Result<bool,String>{
    if p.pending_scan.is_some(){return Ok(true);}
    let(head,tail)=ring(p)?;Ok(head!=tail)
}
/// Plan one DOS character read: the byte, ring writes and the next pending scan.
pub(super) fn dos_take(p:&Loaded)->Result<(u8,Writes,Option<u8>),String>{
    if let Some(scan)=p.pending_scan{return Ok((scan,Vec::new(),None));}
    let(head,tail)=ring(p)?;
    if head==tail{return Err("DOS console read requires input".into());}
    let key=read(p,0x400+head)?;let writes=vec![(0x41a,MemWidth::Word,next(head))];
    if key&255==0{Ok((0,writes,Some((key>>8)as u8)))}else{Ok((key as u8,writes,None))}
}
/// Ctrl-C at the head of input would run DOS break handling (INT23), which
/// this shim does not model; stop before any effect instead of ignoring it.
pub(super) fn dos_break_check(p:&Loaded)->Result<(),String>{
    if p.pending_scan.is_some(){return Ok(());}
    let(head,tail)=ring(p)?;
    if head!=tail&&read(p,0x400+head)?&255==3{return Err("DOS Ctrl-C break handling (INT23) is not modeled".into());}
    Ok(())
}
pub(super) fn dos_flush_writes(p:&Loaded)->Result<Writes,String>{
    let(_,tail)=ring(p)?;let writes=vec![(0x41a,MemWidth::Word,tail)];check(p,&writes)?;Ok(writes)
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
