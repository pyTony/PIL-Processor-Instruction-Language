//! Bounded DOS MCB arena with blocks owned by PSPs. No UMBs or device ownership.
use super::{Loaded,Ram,MemWidth,PSP,LIMIT,ENV,ENV_PARAS};
const END:u16=(LIMIT/16)as u16;
const MAX_BLOCKS:usize=4096;
#[derive(Clone,Debug,PartialEq,Eq)]
struct Block{header:u16,size:u16,owner:u16}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Arena{blocks:Vec<Block>}
impl Arena{
    pub(super) fn new(program_end:u16)->Self{
        let mut blocks=vec![Block{header:ENV-1,size:ENV_PARAS,owner:PSP},Block{header:PSP-1,size:program_end-PSP,owner:PSP}];
        if program_end<END{blocks.push(Block{header:program_end,size:END-program_end-1,owner:0});}
        Self{blocks}
    }
    fn headers(&self)->Vec<(u32,[u8;5])>{
        self.blocks.iter().enumerate().map(|(i,b)|{let mut bytes=[0;5];bytes[0]=if i+1==self.blocks.len(){b'Z'}else{b'M'};bytes[1..3].copy_from_slice(&b.owner.to_le_bytes());bytes[3..5].copy_from_slice(&b.size.to_le_bytes());(u32::from(b.header)*16,bytes)}).collect()
    }
    pub(super) fn write_initial(&self,ram:&mut Ram)->Result<(),String>{
        for(at,bytes)in self.headers(){ram.write_bytes(at,&bytes).map_err(|e|format!("DOS initial MCB: {e:?}"))?;}Ok(())
    }
    fn valid(&self,ram:&Ram)->bool{
        self.headers().iter().all(|(at,bytes)|bytes.iter().enumerate().all(|(n,&b)|ram.load(*at+n as u32,MemWidth::Byte)==Ok(u32::from(b))))
    }
    fn coalesce(&mut self){
        let mut i=0;while i+1<self.blocks.len(){if self.blocks[i].owner==0&&self.blocks[i+1].owner==0{let b=self.blocks.remove(i+1);self.blocks[i].size+=b.size+1;}else{i+=1;}}
    }
    fn largest(&self)->u16{self.blocks.iter().filter(|b|b.owner==0).map(|b|b.size).max().unwrap_or(0)}
    /// An allocated block by its segment, whichever PSP owns it (FreeDOS checks only the MCB).
    fn owned(&self,segment:u16)->Option<usize>{self.blocks.iter().position(|b|b.header+1==segment&&b.owner!=0)}
    /// Hands the block at `segment` to `owner` (EXEC gives the child its blocks).
    pub(super) fn set_owner(&mut self,ram:&mut Ram,segment:u16,owner:u16)->Result<(),String>{
        let i=self.owned(segment).ok_or("EXEC block is not allocated")?;self.blocks[i].owner=owner;
        ram.write_bytes(u32::from(self.blocks[i].header)*16+1,&owner.to_le_bytes()).map_err(|e|format!("DOS MCB owner write: {e:?}"))
    }
    /// Frees every block `owner` holds except `keep` (a TSR's resident block).
    pub(super) fn release(&mut self,ram:&mut Ram,owner:u16,keep:Option<u16>)->Result<(),String>{
        for b in self.blocks.iter_mut().filter(|b|b.owner==owner&&Some(b.header+1)!=keep){
            b.owner=0;ram.write_bytes(u32::from(b.header)*16+1,&[0,0]).map_err(|e|format!("DOS MCB release: {e:?}"))?;
        }
        Ok(())
    }
    /// The largest free block in paragraphs.
    pub(super) fn free(&self)->u16{let mut a=self.clone();a.coalesce();a.largest()}
}
fn low(register:&mut u32,value:u16){*register=(*register&0xffff0000)|u32::from(value);}
fn result(p:&mut Loaded,error:Option<u16>,bx:Option<u16>,allocated:Option<u16>){
    p.cpu.eflags=(p.cpu.eflags&!1)|u32::from(error.is_some());p.cpu.undefined_flags&=!1;
    if let Some(ax)=error.or(allocated){low(&mut p.cpu.regs[0],ax);}
    if let Some(bx)=bx{low(&mut p.cpu.regs[3],bx);}
}
pub(super) fn memory_service(p:&mut Loaded,function:u8)->Result<(),String>{
    // MCB fields are mirrored in RAM. Unsupported direct chain edits return DOS7;
    // do not silently overwrite a corrupted header from trusted host metadata.
    if !p.arena.valid(&p.ram){result(p,Some(7),None,None);return Ok(());}
    let mut next=p.arena.clone();let requested=p.cpu.regs[3]as u16;
    let segment=p.image.selectors[0];let(mut error,mut bx,mut allocated)=(None,None,None);
    match function{
        0x48=>{
            next.coalesce();
            // AH58 strategy, low two bits: 0 first fit, 1 best fit, 2 last fit.
            let fits=|b:&&Block|b.owner==0&&b.size>=requested;
            let chosen=match p.alloc_strategy&3{
                1=>next.blocks.iter().enumerate().filter(|(_,b)|fits(b)).min_by_key(|(_,b)|b.size).map(|(i,_)|i),
                2=>next.blocks.iter().rposition(|b|fits(&b)),
                _=>next.blocks.iter().position(|b|fits(&b)),
            };
            if let Some(i)=chosen{
                let available=next.blocks[i].size;let header=next.blocks[i].header;
                if p.alloc_strategy&3==2&&requested<available{
                    // Last fit takes the top of the block and leaves the bottom free.
                    let low=available-requested-1;next.blocks[i].size=low;
                    next.blocks.insert(i+1,Block{header:header+1+low,size:requested,owner:p.psp});allocated=Some(header+2+low);
                }else{
                    next.blocks[i].owner=p.psp;next.blocks[i].size=requested;allocated=Some(header+1);
                    if requested<available{next.blocks.insert(i+1,Block{header:header+1+requested,size:available-requested-1,owner:0});}
                }
            }else{error=Some(8);bx=Some(next.largest());}
        }
        0x49=>{
            if let Some(i)=next.owned(segment){next.blocks[i].owner=0;}else{error=Some(9);}
            // DOS2..6 defers coalescing until allocation/resize.
        }
        0x4a=>{
            next.coalesce();
            if let Some(i)=next.owned(segment){
                let mut available=next.blocks[i].size;
                if i+1<next.blocks.len()&&next.blocks[i+1].owner==0{let free=next.blocks.remove(i+1);available+=free.size+1;}
                allocated=Some(segment);
                let actual=requested.min(available);let header=next.blocks[i].header;next.blocks[i].size=actual;
                if actual<available{next.blocks.insert(i+1,Block{header:header+1+actual,size:available-actual-1,owner:0});}
                // DOS2..6 expands to the maximum even on insufficient-memory error.
                if requested>available{error=Some(8);bx=Some(available);}
            }else{error=Some(9);}
        }
        _=>return Err("invalid DOS memory service dispatch".into()),
    }
    if next.blocks.len()>MAX_BLOCKS{return Err("DOS probe MCB block-count limit reached".into());}
    let headers:Vec<_>=next.headers().into_iter().filter(|(at,bytes)|bytes.iter().enumerate().any(|(n,&b)|p.ram.load(*at+n as u32,MemWidth::Byte)!=Ok(u32::from(b)))).collect();
    // PSP:0002 retains its load-time value, as in the FreeDOS handler.
    // Report current ownership separately from that original PSP field.
    let program_end=next.owned(p.psp).map(|i|(u32::from(p.psp)+u32::from(next.blocks[i].size))*16).unwrap_or(u32::from(p.psp)*16);
    for(at,bytes)in &headers{for n in 0..bytes.len(){p.ram.check_write(*at+n as u32,MemWidth::Byte).map_err(|e|format!("DOS MCB write: {e:?}"))?;}}
    for(at,bytes)in headers{p.ram.write_bytes(at,&bytes).map_err(|e|format!("DOS MCB commit: {e:?}"))?;}
    p.arena=next;p.allocated_end=program_end;result(p,error,bx,allocated);Ok(())
}
#[cfg(test)]
#[path="../tests/unit/dos_memory.rs"]
mod tests;
