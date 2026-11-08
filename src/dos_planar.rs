//! The VGA BIOS side of the 16-colour planar modes 0Dh, 0Eh, 10h and 12h:
//! mode sets, the sequencer/graphics controller/attribute ports, BIOS pixels,
//! and the INT 10h palette (AH=10h), font information (AX=1130h), EGA
//! information (AH=12h BL=10h) and display combination (AH=1Ah) calls that a
//! colour VGA BIOS answers. The planes themselves live in Ram (vga_planar).
use super::*;
use crate::vga_planar::Planar;
/// Geometry of a planar mode: pixels, character cell height and text rows.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(super) struct Geometry{pub width:u32,pub height:u32,pub cell:u32,pub rows:u32}
impl Geometry{
    pub fn columns(&self)->u32{self.width/8}
    pub fn line_bytes(&self)->u32{self.width/8}
    pub fn bytes(&self)->usize{(self.line_bytes()*self.height)as usize}
}
pub(super) fn geometry(mode:u32)->Option<Geometry>{
    Some(match mode{
        0x0d=>Geometry{width:320,height:200,cell:8,rows:25},
        0x0e=>Geometry{width:640,height:200,cell:8,rows:25},
        0x10=>Geometry{width:640,height:350,cell:14,rows:25},
        0x12=>Geometry{width:640,height:480,cell:16,rows:30},
        _=>return None,
    })
}
/// Page storage in each 64 KiB plane, including BIOS alignment padding.
pub(super) fn page_offset(mode:u32,page:u32)->Result<usize,String>{
    let stride=match mode{0x0d=>0x2000,0x0e=>0x4000,0x10=>0x8000,0x12=>0x10000,_=>return Err("not a planar mode".into())};
    if page>=0x10000/stride{return Err(format!("graphics page {page} outside mode {mode:02x}h"));}
    Ok((page*stride)as usize)
}
fn err(e:crate::memory::AccessFault)->String{format!("BIOS video write: {e:?}")}
fn commit(p:&mut Loaded,writes:&[(u32,MemWidth,u32)])->Result<(),String>{
    for &(at,w,_) in writes{p.ram.check_write(at,w).map_err(err)?;}
    for &(at,w,v) in writes{p.ram.store(at,w,v).map_err(err)?;}Ok(())
}
/// INT 10h AH=00h for a planar mode: BIOS data area, a fresh register set,
/// the mode's palette, and cleared planes unless AL bit 7 asks to keep them.
pub(super) fn set_mode(p:&mut Loaded,mode:u32,keep:bool)->Result<(),String>{
    let g=geometry(mode).ok_or("not a planar mode")?;
    // Regen buffer length as the VGA BIOS parameter table gives it.
    let page=match mode{0x0d=>0x2000,0x0e=>0x4000,0x10=>0x8000,_=>0xa000};
    let mut writes=vec![(0x449,MemWidth::Byte,mode),(0x44a,MemWidth::Word,g.columns()),(0x44c,MemWidth::Word,page),
        (0x44e,MemWidth::Word,0),(0x462,MemWidth::Byte,0),(0x463,MemWidth::Word,0x3d4),
        (0x484,MemWidth::Byte,g.rows-1),(0x485,MemWidth::Word,g.cell)];
    writes.extend((0..8).map(|page|(0x450+page*2,MemWidth::Word,0)));
    commit(p,&writes)?;
    let mut planar=Planar::new();
    if keep{if let Some(old)=p.ram.planar(){for n in 0..4{for (i,b) in old.plane(n).iter().enumerate(){planar.poke(n,i,*b);}}}}
    p.ram.set_planar(Some(planar));
    p.vga.planar_palette(g.height==200);p.vga.select_color();
    Ok(())
}
/// Sequencer (3C4h/3C5h), graphics controller (3CEh/3CFh) and attribute
/// controller (3C0h/3C1h) accesses; a word OUT to an index port writes the
/// index then the data. Returns the value read, or None for another port.
pub(super) fn io(p:&mut Loaded,port:u16,input:bool,value:u16,word:bool)->Result<Option<u16>,String>{
    match port{
        0x3c0|0x3c1=>{
            if word{return Err("word access to the VGA attribute controller is not modelled".into());}
            return p.vga.attribute_io(port,input,value as u8).map(|v|Some(u16::from(v)));
        },
        0x3c4|0x3c5|0x3ce|0x3cf=>{},
        _=>return Ok(None),
    }
    if word&&!matches!(port,0x3c4|0x3ce){return Err("VGA word access must start at index port 3C4h or 3CEh".into());}
    let sequencer=port<0x3ce;
    let Some(planar)=p.ram.planar() else{return idle_io(p,sequencer,port&1==0,input,value,word).map(Some);};
    let mut next=planar.clone();
    let mut index=if sequencer{next.seq_index}else{next.gc_index};
    let mut access=|data:bool,value:u8|->Result<u8,String>{
        if !data{if input{return Ok(index);}index=value;return Ok(0);}
        let i=usize::from(index&0x1f);
        let regs:&mut [u8]=if sequencer{&mut next.seq}else{&mut next.gc};
        let Some(old)=regs.get(i).copied() else{return if input{Ok(0xff)}else{Err(format!("VGA {} register {i:02X}h is not modelled",if sequencer{"sequencer"}else{"graphics controller"}))}};
        if input{return Ok(old);}
        // Only the planar addressing these modes use: no chain-4, odd/even,
        // shift modes or map changes, and no clocking/reset changes.
        let allowed=match(sequencer,i){
            (true,2)=>true,(true,4)=>value&0x0c==0x04,
            (false,0..=4|7|8)=>true,(false,5)=>value&0x74==0,(false,6)=>value&0x0f==0x05,
            _=>value==old,
        };
        if !allowed{return Err(format!("VGA {} register {i:02X}h value {value:02X}h is outside the planar profile",if sequencer{"sequencer"}else{"graphics controller"}));}
        regs[i]=value;Ok(0)
    };
    let even=port&1==0;
    let lo=access(!even,value as u8)?;
    let hi=if word{access(true,(value>>8)as u8)?}else{0};
    if sequencer{next.seq_index=index;}else{next.gc_index=index;}
    *p.ram.planar_mut().unwrap()=next;
    Ok(Some(u16::from(lo)|u16::from(hi)<<8))
}
/// The same ports outside the planar modes (see dos_vga::Controllers):
/// all checks precede any change.
fn idle_io(p:&mut Loaded,sequencer:bool,even:bool,input:bool,value:u16,word:bool)->Result<u16,String>{
    let mut next=p.vga.idle.clone();
    let mut index=if sequencer{next.seq_index}else{next.gc_index};
    let mut access=|data:bool,value:u8|->Result<u8,String>{
        if !data{if input{return Ok(index);}index=value;return Ok(0);}
        let i=usize::from(index&0x1f);
        let regs:&[u8]=if sequencer{&next.seq}else{&next.gc};
        let Some(old)=regs.get(i).copied() else{return if input{Ok(0xff)}else{Err(format!("VGA {} register {i:02X}h is not modelled",if sequencer{"sequencer"}else{"graphics controller"}))}};
        if input{return Ok(old);}
        if !next.allowed(sequencer,i,value){return Err(format!("VGA {} register {i:02X}h value {value:02X}h outside the 16-colour planar modes is not modelled",if sequencer{"sequencer"}else{"graphics controller"}));}
        if sequencer{next.seq[i]=value;}else{next.gc[i]=value;}Ok(0)
    };
    let lo=access(!even,value as u8)?;
    let hi=if word{access(true,(value>>8)as u8)?}else{0};
    if sequencer{next.seq_index=index;}else{next.gc_index=index;}
    p.vga.idle=next;
    Ok(u16::from(lo)|u16::from(hi)<<8)
}
/// INT 10h AH=0Ch/0Dh in a planar mode, page 0. Bit 7 of AL XORs.
pub(super) fn pixel(p:&mut Loaded,mode:u32,write:bool)->Result<(),String>{
    let g=geometry(mode).ok_or("not a planar mode")?;
    let(x,y)=(p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff);
    let page=(p.cpu.regs[3]>>8)&255;let page_offset=page_offset(mode,page)?;
    if x>=g.width||y>=g.height{return Err("graphics pixel outside mode bounds".into());}
    let(offset,bit)=(page_offset+(y*g.line_bytes()+x/8)as usize,x%8);
    let planar=p.ram.planar_mut().ok_or("planar memory is not mapped")?;
    let old=planar.pixel(offset,bit);
    if write{
        let al=(p.cpu.regs[0]&255)as u8;
        planar.set_pixel(offset,bit,if al&0x80!=0{old^(al&15)}else{al&15});
    }else{p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(old);}
    Ok(())
}
fn es_dx(p:&Loaded)->u32{(u32::from(p.selectors[SegmentRegister::Es.index()])<<4).wrapping_add(p.cpu.regs[2]&0xffff)}
fn get8(p:&Loaded,at:u32)->Result<u8,String>{p.ram.load(at,MemWidth::Byte).map(|v|v as u8).map_err(|e|format!("BIOS palette table read: {e:?}"))}
/// INT 10h AH=10h: attribute palette and DAC services of a VGA BIOS. Colour
/// paging (13h/1Ah) and grey-scale summing (1Bh) are not modelled.
pub(super) fn palette(p:&mut Loaded)->Result<(),String>{
    let al=p.cpu.regs[0]&255;let(bl,bh)=((p.cpu.regs[3]&255)as usize,((p.cpu.regs[3]>>8)&255)as u8);
    let set_bh=|p:&mut Loaded,v:u8|p.cpu.regs[3]=(p.cpu.regs[3]&0xffff00ff)|u32::from(v)<<8;
    match al{
        0x00=>{if bl>15{return Err("AH=10h AL=00h palette register above 0Fh".into());}p.vga.attr[bl]=bh&0x3f;},
        0x01=>p.vga.attr[17]=bh,
        0x02=>{let at=es_dx(p);let mut regs=[0u8;17];for(n,r)in regs.iter_mut().enumerate(){*r=get8(p,at+n as u32)?;}
            for n in 0..16{p.vga.attr[n]=regs[n]&0x3f;}p.vga.attr[17]=regs[16];},
        0x03=>{if bl>1{return Err("AH=10h AL=03h BL must be 0 or 1".into());}p.vga.attr[16]=(p.vga.attr[16]&!8)|((bl as u8)<<3);},
        0x07=>{if bl>15{return Err("AH=10h AL=07h palette register above 0Fh".into());}let v=p.vga.attr[bl];set_bh(p,v);},
        0x08=>{let v=p.vga.attr[17];set_bh(p,v);},
        0x09=>{let at=es_dx(p);let mut writes:Vec<_>=(0..16).map(|n|(at+n,MemWidth::Byte,u32::from(p.vga.attr[n as usize]))).collect();
            writes.push((at+16,MemWidth::Byte,u32::from(p.vga.attr[17])));commit(p,&writes)?;},
        0x10=>{let n=(p.cpu.regs[3]&0xffff)as usize;if n>255{return Err("AH=10h AL=10h DAC register above FFh".into());}
            let(r,g,b)=(((p.cpu.regs[2]>>8)&255)as u8,((p.cpu.regs[1]>>8)&255)as u8,(p.cpu.regs[1]&255)as u8);p.vga.set_dac(n,[r,g,b]);},
        0x12=>{let(first,count)=((p.cpu.regs[3]&0xffff)as usize,(p.cpu.regs[1]&0xffff)as usize);
            if first+count>256{return Err("AH=10h AL=12h DAC block runs past entry FFh".into());}
            let at=es_dx(p);let mut rgb=Vec::with_capacity(count);
            for n in 0..count{rgb.push([get8(p,at+3*n as u32)?,get8(p,at+3*n as u32+1)?,get8(p,at+3*n as u32+2)?]);}
            for(n,c)in rgb.into_iter().enumerate(){p.vga.set_dac(first+n,c);}},
        0x15=>{let n=(p.cpu.regs[3]&0xffff)as usize;if n>255{return Err("AH=10h AL=15h DAC register above FFh".into());}
            let[r,g,b]=p.vga.dac(n);p.cpu.regs[2]=(p.cpu.regs[2]&0xffff00ff)|u32::from(r)<<8;
            p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|u32::from(g)<<8|u32::from(b);},
        0x17=>{let(first,count)=((p.cpu.regs[3]&0xffff)as usize,(p.cpu.regs[1]&0xffff)as usize);
            if first+count>256{return Err("AH=10h AL=17h DAC block runs past entry FFh".into());}
            let at=es_dx(p);let writes:Vec<_>=(0..count).flat_map(|n|{let c=p.vga.dac(first+n);(0..3).map(move|k|(at+3*n as u32+k as u32,MemWidth::Byte,u32::from(c[k])))}).collect();
            commit(p,&writes)?;},
        0x18=>p.vga.set_dac_mask(bl as u8),
        0x19=>{let m=p.vga.dac_mask();p.cpu.regs[3]=(p.cpu.regs[3]&0xffffff00)|u32::from(m);},
        _=>return Err(format!("INT 10h AX=10{al:02X}h is not modelled")),
    }
    Ok(())
}
/// AX=1130h: font information. BH=0/1 return the INT 1Fh/43h vectors, BH=3
/// the 8x8 ROM font (00h-7Fh) and BH=4 its upper half (80h-FFh); CX is the
/// current character height and DL the last text row. Font loading (other
/// AL) is not modelled.
pub(super) fn font(p:&mut Loaded)->Result<(),String>{
    let al=p.cpu.regs[0]&255;
    if al!=0x30{return Err(format!("INT 10h AX=11{al:02X}h (font load) is not modelled"));}
    let bh=(p.cpu.regs[3]>>8)&255;
    let pointer=match bh{
        0|1=>p.ram.load(if bh==0{0x1f*4}else{0x43*4},MemWidth::Dword).map_err(|e|format!("{e:?}"))?,
        3=>0xf000_fa6e,
        4=>super::console::graphics_text::UPPER_FONT,
        _=>return Err(format!("INT 10h AX=1130h BH={bh:02X}h: only the 8x8 ROM font is present")),
    };
    let height=p.ram.load(0x485,MemWidth::Word).map_err(|e|format!("{e:?}"))?;
    let rows=p.ram.load(0x484,MemWidth::Byte).map_err(|e|format!("{e:?}"))?;
    let es=SegmentRegister::Es.index();let segment=(pointer>>16)as u16;
    p.selectors[es]=segment;p.image.selectors[es]=segment;p.image.segments[es]=SegmentCache::real_mode(segment);
    p.cpu.regs[5]=(p.cpu.regs[5]&0xffff0000)|(pointer&0xffff);
    p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|height;
    p.cpu.regs[2]=(p.cpu.regs[2]&0xffff_ff00)|rows;
    Ok(())
}
/// AH=12h BL=10h: EGA information of a colour VGA with 256 KiB. Other BL
/// subfunctions return unchanged (AL stays, which callers read as failure).
pub(super) fn alternate(p:&mut Loaded){
    if p.cpu.regs[3]&255==0x10{
        p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|0x0003;
        p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|0x0009;
    }
}
/// AH=1Ah: AL=00h reads the display combination, colour VGA (08h) with no
/// alternate display. Setting it (AL=01h) is not modelled.
pub(super) fn combination(p:&mut Loaded)->Result<(),String>{
    match p.cpu.regs[0]&255{
        0=>{p.cpu.regs[0]=(p.cpu.regs[0]&0xffffff00)|0x1a;p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|0x0008;Ok(())},
        al=>Err(format!("INT 10h AX=1A{al:02X}h is not modelled")),
    }
}
/// Session view: the used part of each plane and the 16 pixel colours.
pub(super) fn snapshot(p:&Loaded)->Option<(u32,Geometry,String,String)>{
    let mode=p.ram.load(0x449,MemWidth::Byte).ok()?;let g=geometry(mode)?;let planar=p.ram.planar()?;
    let mut hex=String::with_capacity(8*g.bytes());
    let start=usize::from(p.vga.start());
    for n in 0..4{for i in 0..g.bytes(){let b=planar.peek(n,(start+i)&0xffff);hex.push_str(&format!("{b:02x}"));}}
    let rgb:String=p.vga.planar_rgb().iter().flatten().map(|c|format!("{c:02x}")).collect();
    Some((mode,g,hex,rgb))
}
