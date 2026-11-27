//! Bounded DOS MCB arena with blocks owned by PSPs, and upper memory as
//! DOS=UMB gives it (MS-DOS 5+, FreeDOS): while the XMS driver is installed,
//! C800-EFFF is RAM, conventional memory ends one paragraph early, and a
//! system MCB at 9FFFh (owner 0008h, name "SC") spans A000-C7FF to the first
//! upper MCB at C800h. That MCB is 'Z' while the UMBs are unlinked (the chain
//! ends in conventional memory) and 'M' once AX=5803h links them; list of
//! lists byte 63h holds the link state and word 66h the 9FFFh. AH48 takes
//! upper blocks only while linked: strategy 40h-42h upper only, 80h-82h
//! upper first, 00h-02h the whole chain in address order (FreeDOS DosMemAlloc).
use super::{Loaded,Ram,MemWidth,PSP,LIMIT,ENV,ENV_PARAS,DOSDATA,SYSVARS};
const END:u16=(LIMIT/16)as u16;
/// The system MCB that bridges conventional and upper memory.
pub(super) const SC:u16=END-1;
pub(super) const UMB_FIRST:u16=0xc800;
pub(super) const UMB_END:u16=0xf000;
const SC_OWNER:u16=8;
const LINK_FLAG:u32=DOSDATA as u32*16+SYSVARS as u32+0x63;
const UPPER_ROOT:u32=DOSDATA as u32*16+SYSVARS as u32+0x66;
const MAX_BLOCKS:usize=4096;
#[derive(Clone,Debug,PartialEq,Eq)]
struct Block{header:u16,size:u16,owner:u16}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Arena{blocks:Vec<Block>,umb:bool,linked:bool}
impl Arena{
    pub(super) fn new(program_end:u16)->Self{
        let mut blocks=vec![Block{header:ENV-1,size:ENV_PARAS,owner:PSP},Block{header:PSP-1,size:program_end-PSP,owner:PSP}];
        if program_end<END{blocks.push(Block{header:program_end,size:END-program_end-1,owner:0});}
        Self{blocks,umb:false,linked:false}
    }
    pub(super) fn linked(&self)->bool{self.umb&&self.linked}
    fn headers(&self)->Vec<(u32,[u8;5])>{
        self.blocks.iter().enumerate().map(|(i,b)|{let mut bytes=[0;5];bytes[0]=if i+1==self.blocks.len()||(b.header==SC&&self.umb&&!self.linked){b'Z'}else{b'M'};bytes[1..3].copy_from_slice(&b.owner.to_le_bytes());bytes[3..5].copy_from_slice(&b.size.to_le_bytes());(u32::from(b.header)*16,bytes)}).collect()
    }
    pub(super) fn write_initial(&self,ram:&mut Ram)->Result<(),String>{
        for(at,bytes)in self.headers(){ram.write_bytes(at,&bytes).map_err(|e|format!("DOS initial MCB: {e:?}"))?;}Ok(())
    }
    fn valid(&self,ram:&Ram)->bool{
        self.headers().iter().all(|(at,bytes)|bytes.iter().enumerate().all(|(n,&b)|ram.load(*at+n as u32,MemWidth::Byte)==Ok(u32::from(b))))
    }
    /// DOS programs may split/relink their own MCBs (for example while swapping
    /// resident memory). Follow the visible chain rather than overwriting it
    /// from stale host metadata. Malformed or out-of-arena chains stay errors.
    fn refreshed(&self,ram:&Ram)->Result<Self,String>{
        if self.valid(ram){return Ok(self.clone());}
        let mut header=self.blocks.first().ok_or("missing DOS MCB arena")?.header;
        let mut blocks=Vec::new();let mut linked=self.linked;
        let limit=if self.umb{UMB_END}else{END};
        for _ in 0..MAX_BLOCKS{
            let at=u32::from(header)*16;
            let kind=ram.load(at,MemWidth::Byte).map_err(|_|"unreadable DOS MCB")? as u8;
            if !matches!(kind,b'M'|b'Z'){return Err("invalid DOS MCB type".into());}
            let owner=ram.load(at+1,MemWidth::Word).map_err(|_|"unreadable DOS MCB owner")? as u16;
            let size=ram.load(at+3,MemWidth::Word).map_err(|_|"unreadable DOS MCB size")? as u16;
            let next=u32::from(header)+1+u32::from(size);
            if next>u32::from(limit){return Err("DOS MCB exceeds conventional arena".into());}
            blocks.push(Block{header,size,owner});
            // DOS keeps the upper chain while it is unlinked: the 'Z' at the
            // system MCB ends only what programs see.
            if self.umb&&header==SC{linked=kind==b'M';}
            else if kind==b'Z'{return Ok(Self{blocks,umb:self.umb,linked});}
            if next>=u32::from(limit){return Err("DOS MCB chain has no terminator".into());}
            header=next as u16;
        }
        Err("DOS MCB block-count limit reached".into())
    }
    fn coalesce(&mut self){
        let mut i=0;while i+1<self.blocks.len(){if self.blocks[i].owner==0&&self.blocks[i+1].owner==0{let b=self.blocks.remove(i+1);self.blocks[i].size+=b.size+1;}else{i+=1;}}
    }
    #[cfg(test)]
    fn largest(&self)->u16{self.blocks.iter().filter(|b|b.owner==0).map(|b|b.size).max().unwrap_or(0)}
    /// Where AH48 looks, in order: Some(true) upper blocks, Some(false)
    /// conventional ones, None the whole chain.
    fn plan(&self,strategy:u8)->Vec<Option<bool>>{
        if !self.umb{return vec![None];}
        if !self.linked{return vec![Some(false)];}
        match strategy&0xc0{0x40=>vec![Some(true)],0x80=>vec![Some(true),Some(false)],_=>vec![None]}
    }
    fn region(b:&Block,high:Option<bool>)->bool{high.is_none_or(|h|(b.header>SC)==h)}
    fn largest_in(&self,high:Option<bool>)->u16{self.blocks.iter().filter(|b|b.owner==0&&Self::region(b,high)).map(|b|b.size).max().unwrap_or(0)}
    /// The block EXEC gives a program needing `need` paragraphs: the largest
    /// free block of the first region (by strategy and link) where it fits,
    /// else the largest anywhere it may look (too small; EXEC then fails).
    pub(super) fn exec_block(&self,strategy:u8,need:u32)->u16{
        let mut a=self.clone();a.coalesce();
        let mut best=0;
        for high in a.plan(strategy){let l=a.largest_in(high);if u32::from(l)>=need{return l;}best=best.max(l);}
        best
    }
    /// DOS=UMB: shrink the last conventional block by the paragraph the
    /// system MCB takes, then add that MCB and one free upper block. Returns
    /// the owner of the shrunk block.
    fn add_umbs(&mut self)->Result<u16,String>{
        let last=self.blocks.last_mut().ok_or("missing DOS MCB arena")?;
        if self.umb||last.header+1+last.size!=END||last.size==0{return Err("conventional memory does not end at A000h".into());}
        last.size-=1;let owner=last.owner;
        self.blocks.push(Block{header:SC,size:UMB_FIRST-SC-1,owner:SC_OWNER});
        self.blocks.push(Block{header:UMB_FIRST,size:UMB_END-UMB_FIRST-1,owner:0});
        self.umb=true;self.linked=false;Ok(owner)
    }
    /// The reverse, while every upper block is free. Returns the owner of the
    /// grown block.
    fn remove_umbs(&mut self)->Result<u16,String>{
        let mut a=self.clone();a.coalesce();
        let n=a.blocks.len();
        if !a.umb||n<3||a.blocks[n-2].header!=SC||a.blocks[n-1].owner!=0||a.blocks[n-1].header!=UMB_FIRST{return Err("upper memory is in use".into());}
        a.blocks.truncate(n-2);let last=a.blocks.last_mut().unwrap();last.size+=1;let owner=last.owner;
        a.umb=false;a.linked=false;*self=a;Ok(owner)
    }
    /// An allocated block by its segment, whichever PSP owns it (FreeDOS checks only the MCB).
    pub(super) fn owned(&self,segment:u16)->Option<usize>{self.blocks.iter().position(|b|b.header+1==segment&&b.owner!=0)}
    /// Checks if `segment` falls inside any block (including its MCB header or interior).
    pub(super) fn find_segment(&self,segment:u16)->Option<(u16,u16,u16)>{
        self.blocks.iter().find(|b|segment>=b.header&&segment<=b.header+b.size).map(|b|(b.header,b.owner,b.size))
    }
    /// Returns the owner of the block starting at `segment`.
    #[cfg(test)]
    pub(super) fn block_owner(&self,segment:u16)->Option<u16>{
        self.blocks.iter().find(|b|b.header+1==segment).map(|b|b.owner)
    }
    /// Returns the first segment of the first free block in the arena.
    #[cfg(test)]
    pub(super) fn first_free_segment(&self)->Option<u16>{
        self.blocks.iter().find(|b|b.owner==0).map(|b|b.header+1)
    }
    /// Hands the block at `segment` to `owner` (EXEC gives the child its blocks).
    pub(super) fn set_owner(&mut self,ram:&mut Ram,segment:u16,owner:u16)->Result<(),String>{
        let i=self.owned(segment).ok_or("EXEC block is not allocated")?;
        ram.write_bytes(u32::from(self.blocks[i].header)*16+1,&owner.to_le_bytes()).map_err(|e|format!("DOS MCB owner write: {e:?}"))?;
        self.blocks[i].owner=owner;
        Ok(())
    }
    /// Frees every block `owner` holds except `keep` (a TSR's resident block).
    pub(super) fn release(&mut self,ram:&mut Ram,owner:u16,keep:Option<u16>)->Result<(),String>{
        let mut next=self.refreshed(ram)?;
        for b in next.blocks.iter().filter(|b|b.owner==owner&&Some(b.header+1)!=keep){
            ram.check_write(u32::from(b.header)*16+1,MemWidth::Word).map_err(|e|format!("DOS MCB release: {e:?}"))?;
        }
        for b in next.blocks.iter_mut().filter(|b|b.owner==owner&&Some(b.header+1)!=keep){
            b.owner=0;ram.write_bytes(u32::from(b.header)*16+1,&[0,0]).map_err(|e|format!("DOS MCB release: {e:?}"))?;
        }
        *self=next;
        Ok(())
    }
    /// The largest free block in paragraphs.
    #[cfg(test)]
    pub(super) fn free(&self)->u16{let mut a=self.clone();a.coalesce();a.largest()}
}
fn low(register:&mut u32,value:u16){*register=(*register&0xffff0000)|u32::from(value);}
fn result(p:&mut Loaded,error:Option<u16>,bx:Option<u16>,allocated:Option<u16>){
    p.cpu.eflags=(p.cpu.eflags&!1)|u32::from(error.is_some());p.cpu.undefined_flags&=!1;
    if let Some(ax)=error.or(allocated){low(&mut p.cpu.regs[0],ax);}
    if let Some(bx)=bx{low(&mut p.cpu.regs[3],bx);}
}
pub(super) fn memory_service(p:&mut Loaded,function:u8)->Result<(),String>{
    let Ok(mut next)=p.dos.arena.refreshed(&p.ram) else{result(p,Some(7),None,None);return Ok(());};
    let requested=p.cpu.regs[3]as u16;
    let segment=p.image.selectors[0];let(mut error,mut bx,mut allocated)=(None,None,None);
    match function{
        0x48=>{
            next.coalesce();
            // AH58 strategy, low two bits: 0 first fit, 1 best fit, 2 last
            // fit; the regions to search in order come from bits 6-7.
            let plan=next.plan(p.dos.alloc_strategy);
            let mut chosen=None;
            for &high in &plan{
                let fits=|b:&Block|b.owner==0&&b.size>=requested&&Arena::region(b,high);
                chosen=match p.dos.alloc_strategy&3{
                    1=>next.blocks.iter().enumerate().filter(|(_,b)|fits(b)).min_by_key(|(_,b)|b.size).map(|(i,_)|i),
                    2=>next.blocks.iter().rposition(fits),
                    _=>next.blocks.iter().position(fits),
                };
                if chosen.is_some(){break;}
            }
            if let Some(i)=chosen{
                let available=next.blocks[i].size;let header=next.blocks[i].header;
                if p.dos.alloc_strategy&3==2&&requested<available{
                    // Last fit takes the top of the block and leaves the bottom free.
                    let low=available-requested-1;next.blocks[i].size=low;
                    next.blocks.insert(i+1,Block{header:header+1+low,size:requested,owner:p.dos.psp});allocated=Some(header+2+low);
                }else{
                    next.blocks[i].owner=p.dos.psp;next.blocks[i].size=requested;allocated=Some(header+1);
                    if requested<available{next.blocks.insert(i+1,Block{header:header+1+requested,size:available-requested-1,owner:0});}
                }
            }else{error=Some(8);bx=Some(plan.iter().map(|&h|next.largest_in(h)).max().unwrap_or(0));}
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
    let program_end=next.owned(p.dos.psp).map(|i|(u32::from(p.dos.psp)+u32::from(next.blocks[i].size))*16).unwrap_or(u32::from(p.dos.psp)*16);
    for(at,bytes)in &headers{for n in 0..bytes.len(){p.ram.check_write(*at+n as u32,MemWidth::Byte).map_err(|e|format!("DOS MCB write: {e:?}"))?;}}
    for(at,bytes)in headers{p.ram.write_bytes(at,&bytes).map_err(|e|format!("DOS MCB commit: {e:?}"))?;}
    p.dos.arena=next;p.dos.allocated_end=program_end;result(p,error,bx,allocated);Ok(())
}
/// AX=5803h: link (true) or unlink the upper blocks. `Ok(false)` when there
/// are none (the caller answers invalid function, as FreeDOS does).
pub(super) fn link(p:&mut Loaded,on:bool)->Result<bool,String>{
    let Ok(mut next)=p.dos.arena.refreshed(&p.ram) else{return Ok(false);};
    if !next.umb{return Ok(false);}
    next.linked=on;
    p.ram.write_bytes(u32::from(SC)*16,&[if on{b'M'}else{b'Z'}]).map_err(|e|format!("DOS UMB link: {e:?}"))?;
    p.ram.write_bytes(LINK_FLAG,&[u8::from(on)]).map_err(|e|format!("DOS UMB link flag: {e:?}"))?;
    p.dos.arena=next;Ok(true)
}
/// Adds (DOS=UMB) or removes upper memory before the first instruction:
/// C800-EFFF becomes RAM or the empty FFh bus again, the MCBs and list of
/// lists follow, and a program block that ended at A000h (or 9FFFh) keeps its
/// PSP:0002 and end in step.
pub(super) fn set_umbs(p:&mut Loaded,on:bool)->Result<(),String>{
    if p.dos.arena.umb==on{return Ok(());}
    let mut next=p.dos.arena.refreshed(&p.ram)?;
    let owner=if on{next.add_umbs()?}else{next.remove_umbs()?};
    let(base,bytes)=(u32::from(UMB_FIRST)*16,usize::from(UMB_END-UMB_FIRST)*16);
    let e=|e|format!("DOS upper memory: {e:?}");
    if on{p.ram.unrom(base,bytes).map_err(e)?;p.ram.write_bytes(base,&vec![0;bytes]).map_err(e)?;}
    else{p.ram.write_bytes(base,&vec![0xff;bytes]).map_err(e)?;p.ram.rom(base,bytes).map_err(e)?;}
    next.write_initial(&mut p.ram)?;
    if on{p.ram.write_bytes(u32::from(SC)*16+8,b"SC\0\0\0\0\0\0").map_err(e)?;}
    p.ram.write_bytes(LINK_FLAG,&[0]).map_err(e)?;
    p.ram.write_bytes(UPPER_ROOT,&if on{SC}else{0xffff}.to_le_bytes()).map_err(e)?;
    let(old,new)=if on{(END,SC)}else{(SC,END)};
    if owner==p.dos.psp&&owner!=0{
        let at=u32::from(p.dos.psp)*16+2;
        if p.ram.load(at,MemWidth::Word).map_err(e)?==u32::from(old){p.ram.store(at,MemWidth::Word,u32::from(new)).map_err(e)?;}
        if p.dos.allocated_end==u32::from(old)*16{p.dos.allocated_end=u32::from(new)*16;}
        let image=&mut p.image;
        if image.fetch_base+image.end==u32::from(old)*16{image.end=u32::from(new)*16-image.fetch_base;}
    }
    // Programs run code from AH48 blocks: in conventional memory that is
    // inside the program's own window, upper blocks need a range of their own.
    p.image.system[UMB_CODE_SLOT]=if on{(base,base+bytes as u32)}else{(0,0)};
    p.dos.arena=next;Ok(())
}
/// The fixed slot of image.system that holds C800-EFFF while UMBs exist.
pub(super) const UMB_CODE_SLOT:usize=4;
