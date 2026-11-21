//! RAM addressed by linear address. While paging is on (CR0.PG), every
//! access walks the two-level 386 page tables (see `translate`); otherwise a
//! linear address is the bus address. The one memory-mapped device is
//! optional EGA/VGA planar memory (vga_planar) behind A0000h-AFFFFh, present
//! only while a 16-colour planar mode is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemWidth { Byte, Word, Dword }
impl MemWidth {
    pub fn from_bits(bits:u8)->Option<Self> {
        match bits {8=>Some(Self::Byte),16=>Some(Self::Word),32=>Some(Self::Dword),_=>None}
    }
    pub fn bits(self)->u8 {self.bytes()*8}
    pub fn bytes(self)->u8 {match self {Self::Byte=>1,Self::Word=>2,Self::Dword=>4}}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessFault {pub address:u32,pub bytes:usize,pub write:bool,
    /// A page fault (#PF) and its error code (P, W/R, U/S bits); `address`
    /// is then the faulting linear address that goes to CR2. None is a bus
    /// fault: the access left RAM or hit write-protected memory.
    pub page:Option<u8>}
impl AccessFault {
    fn bus(address:u32,bytes:usize,write:bool)->Self{Self{address,bytes,write,page:None}}
    /// The CPU-level error at instruction `eip`.
    pub fn error(self,eip:u32)->super::ExecError{
        match self.page{
            Some(code)=>super::ExecError::PageFault{eip,address:self.address,error_code:u16::from(code)},
            None=>super::ExecError::MemoryAccess{eip,address:self.address,bytes:self.bytes as u8,write:self.write},
        }
    }
}
/// Paging state while CR0.PG is set: the page directory base (CR3) and
/// whether accesses are user-level (CPL 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paging {pub directory:u32,pub user:bool}

// Host cache metadata, never guest write protection. Every write to RAM
// counts in its 64-byte line; a stamp holds the sum of the counts over the
// lines its code covers, so code stays trusted until a write lands in one of
// those lines, however close other writes come (data next to code, as small
// programs keep it). After such a write the owner compares bytes, and may
// take a new stamp once they match.
pub(crate) const CODE_LINE:usize=64;
#[derive(Clone,Copy,Debug)]
pub(crate) struct CodeStamp {
    owner: u64,
    address: u32,
    bytes: usize,
    a20: bool,
    planar: bool,
    first_line: usize,
    last_line: usize,
    writes: u64,
}
struct CodeWatch {owner:u64,lines:Vec<std::cell::Cell<u64>>}
impl CodeWatch {
    fn new(bytes:usize)->Self{
        static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(1);
        Self{owner:NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed),lines:(0..bytes.div_ceil(CODE_LINE)).map(|_|std::cell::Cell::new(0)).collect()}
    }
    #[inline]
    fn write(&self,start:usize,bytes:usize){
        if bytes==0{return;}
        for line in start/CODE_LINE..=(start+bytes-1)/CODE_LINE {
            if let Some(count)=self.lines.get(line){count.set(count.get()+1);}
        }
    }
    fn writes(&self,lines:std::ops::RangeInclusive<usize>)->u64{lines.map(|i|self.lines[i].get()).sum()}
}
impl Clone for CodeWatch {fn clone(&self)->Self{let mut copy=Self::new(0);copy.lines=self.lines.clone();copy}}
// Cache bookkeeping is not architectural RAM state.
impl PartialEq for CodeWatch {fn eq(&self,_:&Self)->bool{true}}
impl Eq for CodeWatch {}
impl std::fmt::Debug for CodeWatch {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str("CodeWatch")}}

// Optional host debugger watch for the complete MDA/CGA/VGA text apertures.
// Guest memory comparison deliberately ignores this diagnostic-only state.
#[derive(Clone,Debug)]
struct TextWrite{address:u32,before:u8,after:u8,cs:u16,ip:u32}
#[derive(Clone,Debug)]
struct TextWatch{
    site:(u16,u32),events:std::collections::VecDeque<TextWrite>,total:u64,
}
impl TextWatch{
    const CAPACITY:usize=2048;
    fn new()->Self{Self{site:(0,0),events:std::collections::VecDeque::new(),total:0}}
    fn write(&mut self,at:usize,before:u8,after:u8){
        if before==after||!(0xb0000..0xc0000).contains(&at){return;}
        self.total=self.total.saturating_add(1);
        if self.events.len()==Self::CAPACITY{self.events.pop_front();}
        self.events.push_back(TextWrite{address:at as u32,before,after,cs:self.site.0,ip:self.site.1});
    }
    fn json(&self)->String{
        let entries=self.events.iter().map(|e|format!(
            r#"{{"address":{},"old":{},"new":{},"cs":"{:04X}","ip":"{:04X}"}}"#,
            e.address,e.before,e.after,e.cs,e.ip)).collect::<Vec<_>>().join(",");
        format!(r#"{{"enabled":true,"total":{},"capacity":{},"writes":[{}]}}"#,
            self.total,Self::CAPACITY,entries)
    }
}
impl PartialEq for TextWatch{fn eq(&self,_:&Self)->bool{true}}
impl Eq for TextWatch{}
#[derive(Clone,Debug,Default)]
struct VideoWatch{total:u64,changed_known:u64,graphics:u64,text:u64}
impl PartialEq for VideoWatch{fn eq(&self,_:&Self)->bool{true}}
impl Eq for VideoWatch{}
impl VideoWatch{
    fn note(&mut self,index:usize,changed:Option<bool>){
        if !(0xa0000..0xc0000).contains(&index){return;}
        self.total=self.total.saturating_add(1);
        if index<0xb0000{self.graphics=self.graphics.saturating_add(1);}
        else{self.text=self.text.saturating_add(1);}
        if changed==Some(true){self.changed_known=self.changed_known.saturating_add(1);}
    }
    fn json(&self)->String{
        format!(r#"{{"enabled":true,"total":{},"graphics":{},"text":{},"changed_known":{}}}"#,
            self.total,self.graphics,self.text,self.changed_known)
    }
}
/// Optional origin hints for code read from an immutable executable resource.
/// A hint never authorizes execution: the application backend still checks its
/// compiled bytes. Ordinary CPU writes can leave hints stale safely; changed
/// instructions take the reference path. These are not architectural RAM state.
#[derive(Clone,Debug,Default)]
struct CodeOrigins{identity:Option<usize>,ranges:std::collections::BTreeMap<u32,(u32,u32)>}
impl PartialEq for CodeOrigins{fn eq(&self,_:&Self)->bool{true}}
impl Eq for CodeOrigins{}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ram {
    data:Vec<u8>,
    readonly:Vec<std::ops::Range<usize>>,
    /// ROM: writes pass every check and change nothing, as a write to a
    /// ROM chip on the bus does. Unlike `readonly`, no fault is raised.
    rom:Vec<std::ops::Range<usize>>,
    a20_enabled:bool,
    planar:Option<Box<super::vga_planar::Planar>>,
    /// A Cell so that `as_level` can make one access supervisor-level (a
    /// descriptor or TSS read at CPL 3) through `&self`.
    paging:std::cell::Cell<Option<Paging>>,
    /// Accessed and dirty bits a page walk has set but `&self` methods could
    /// not yet write: (physical byte, bits to OR). Reads see them at once;
    /// `settle` writes them, and every `&mut` method settles first.
    marks:std::cell::RefCell<Vec<(usize,u8)>>,
    code_watch:CodeWatch,
    text_watch:Option<TextWatch>,
    video_watch:Option<VideoWatch>,
    code_origins:CodeOrigins,
}
/// Host-only alias of the four planes (plane n at PLANE_ALIAS + n*10000h),
/// for BIOS services that write pixels without the graphics controller. Real-
/// mode guest addresses end at 10FFEFh, so guests cannot reach it.
pub const PLANE_ALIAS:u32=0xf000_0000;
impl Ram {
    pub fn new(bytes:usize)->Self {Self{data:vec![0;bytes],readonly:Vec::new(),rom:Vec::new(),a20_enabled:true,planar:None,paging:std::cell::Cell::new(None),marks:Default::default(),code_watch:CodeWatch::new(bytes),text_watch:None,video_watch:None,code_origins:Default::default()}}
    pub(crate) fn track_resource(&mut self,identity:usize){self.code_origins=CodeOrigins{identity:Some(identity),ranges:Default::default()};}
    pub(crate) fn resource_origin(&self,address:u32)->Option<(usize,u32)>{
        if self.paging.get().is_some(){return None;}
        let identity=self.code_origins.identity?;let at=self.physical_address(address);
        let(&start,&(end,offset))=self.code_origins.ranges.range(..=at).next_back()?;
        (at<end).then(||offset.checked_add(at-start)).flatten().map(|offset|(identity,offset))
    }
    pub(crate) fn resource_read(&mut self,identity:usize,address:u32,offset:u32,bytes:usize){
        if self.code_origins.identity!=Some(identity)||self.paging.get().is_some(){return;}
        if bytes>u32::MAX as usize||offset.checked_add(bytes as u32).is_none(){return;}
        let mut done=0;
        while done<bytes{
            let guest=address.wrapping_add(done as u32);let start=self.physical_address(guest);
            let count=if self.a20_enabled{bytes-done}else{(bytes-done).min(0x100000-(guest as usize&0xfffff))};
            let Some(end)=start.checked_add(count as u32)else{return};
            let ranges=&mut self.code_origins.ranges;
            let overlaps:Vec<_>=ranges.range(..end).filter(|(_,r)|r.0>start).map(|(&s,&r)|(s,r)).collect();
            for(old,(old_end,old_offset))in overlaps{
                ranges.remove(&old);
                if old<start{ranges.insert(old,(start,old_offset));}
                if old_end>end{ranges.insert(end,(old_end,old_offset+(end-old)));}
            }
            if ranges.len()>=4096{ranges.clear();}
            ranges.insert(start,(end,offset+done as u32));done+=count;
        }
    }
    fn code_pages(&self,address:u32,bytes:usize)->Option<std::ops::RangeInclusive<usize>>{
        if bytes==0{return None;}
        self.contiguous_slice(address,bytes)?;
        let start=self.physical_address(address)as usize;let end=start+bytes;
        if self.planar.is_some()&&start<0xb0000&&end>0xa0000{return None;}
        Some(start/CODE_LINE..=(end-1)/CODE_LINE)
    }
    /// A stamp for code at `address`: valid until a write reaches one of
    /// its lines (or A20 or the RAM object changes). The caller must have
    /// read the bytes it trusts after the last such write.
    pub(crate) fn watch_code(&self,address:u32,bytes:usize)->Option<CodeStamp>{
        let lines=self.code_pages(address,bytes)?;
        Some(CodeStamp {
            owner: self.code_watch.owner,
            address,
            bytes,
            a20: self.a20_enabled,
            planar: self.planar.is_some(),
            first_line: *lines.start(),
            last_line: *lines.end(),
            writes: self.code_watch.writes(lines),
        })
    }
    #[inline]
    pub(crate) fn code_unchanged(&self, stamp: CodeStamp, address: u32, bytes: usize) -> bool {
        if stamp.owner != self.code_watch.owner || stamp.address != address
            || stamp.bytes != bytes || stamp.a20 != self.a20_enabled
            || stamp.planar != self.planar.is_some() || self.paging.get().is_some()
        {
            return false;
        }
        // The owner fixes the backing RAM and the stamp was issued for this
        // exact address, length, and mapping.
        let writes = if stamp.first_line == stamp.last_line {
            self.code_watch.lines[stamp.first_line].get()
        } else {
            self.code_watch.writes(stamp.first_line..=stamp.last_line)
        };
        writes == stamp.writes
    }
    /// Turns paging on or off (CR0.PG, CR3, CPL). The 386 TLB is not
    /// modelled: every access walks the tables as they are in memory.
    #[inline]
    pub fn set_paging(&mut self, paging: Option<Paging>) {
        if !self.marks.get_mut().is_empty() {
            self.settle();
        }
        self.paging.set(paging);
    }
    /// Runs `f` with accesses at user level (`user`) or supervisor level,
    /// whatever CPL selected: the 386 reads descriptor tables and the TSS,
    /// and pushes on an inner-level stack, at supervisor level.
    pub fn as_level<T>(&self,user:bool,f:impl FnOnce(&Self)->T)->T{
        let saved=self.paging.get();
        self.paging.set(saved.map(|p|Paging{user,..p}));
        let result=f(self);
        self.paging.set(saved);
        result
    }
    /// A store at user or supervisor level (see `as_level`).
    pub fn store_as(&mut self,user:bool,address:u32,width:MemWidth,value:u32)->Result<(),AccessFault>{
        let saved=self.paging.get();
        self.paging.set(saved.map(|p|Paging{user,..p}));
        let result=self.store(address,width,value);
        self.paging.set(saved);
        result
    }
    /// Writes the accessed and dirty bits that walks have marked.
    pub fn settle(&mut self){for (at,bits) in self.marks.get_mut().drain(..){if let Some(b)=self.data.get_mut(at){self.code_watch.write(at,1);*b|=bits;}}}
    fn byte(&self,index:usize)->u8{
        let mut b=self.data[index];
        for &(at,bits) in self.marks.borrow().iter(){if at==index{b|=bits;}}
        b
    }
    /// A page-table entry at a physical address, with marks applied.
    fn entry(&self,at:u32,linear:u32,write:bool)->Result<u32,AccessFault>{
        let at=self.physical_address(at);
        let range=self.range(at,4,false).map_err(|_|AccessFault::bus(linear,1,write))?;
        Ok(range.fold((0u32,0u32),|(v,i),index|(v|(u32::from(self.byte(index))<<(8*i)),i+1)).0)
    }
    /// The bus address of linear byte `linear` (before the A20 gate). With
    /// paging on: the PDE at CR3 + bits 31-22 * 4, then the PTE at its frame +
    /// bits 21-12 * 4. A missing entry is #PF with error bit 0 clear; a user
    /// access to a supervisor page, or a user write to a read-only page, is
    /// #PF with bit 0 set (the 386 does not check writes at CPL 0-2). Bit 1 is
    /// set for a write, bit 2 for user level. Only a walk that succeeds sets
    /// the accessed bits of both entries, and the PTE's dirty bit on a write.
    pub fn translate(&self,linear:u32,write:bool)->Result<u32,AccessFault>{
        let Some(paging)=self.paging.get() else{return Ok(linear);};
        let code=(u8::from(write)<<1)|(u8::from(paging.user)<<2);
        let fault=|p:u8|AccessFault{address:linear,bytes:1,write,page:Some(code|p)};
        let pde_at=(paging.directory&!0xfff).wrapping_add((linear>>22)*4);
        let pde=self.entry(pde_at,linear,write)?;
        if pde&1==0{return Err(fault(0));}
        let pte_at=(pde&!0xfff).wrapping_add(((linear>>12)&0x3ff)*4);
        let pte=self.entry(pte_at,linear,write)?;
        if pte&1==0{return Err(fault(0));}
        let both=pde&pte;
        if paging.user&&(both&4==0||(write&&both&2==0)){return Err(fault(1));}
        let mut marks=self.marks.borrow_mut();
        if pde&0x20==0{marks.push((self.physical_address(pde_at)as usize,0x20));}
        let bits:u8=0x20|if write{0x40}else{0};
        if pte&u32::from(bits)!=u32::from(bits){marks.push((self.physical_address(pte_at)as usize,bits));}
        Ok((pte&!0xfff)|(linear&0xfff))
    }
    /// The physical byte (A20 applied) behind a linear byte.
    pub fn bus_address(&self,linear:u32,write:bool)->Result<u32,AccessFault>{self.locate(linear,write)}
    fn locate(&self,linear:u32,write:bool)->Result<u32,AccessFault>{Ok(self.physical_address(self.translate(linear,write)?))}
    /// Watch all text pages at B0000h..BFFFFh, regardless of program or mode.
    /// Disabled by default; only *changed* bytes create history records.
    pub(crate) fn start_text_watch(&mut self){self.text_watch=Some(TextWatch::new());}
    pub(crate) fn stop_text_watch(&mut self){self.text_watch=None;}
    pub(crate) fn text_watch_enabled(&self)->bool{self.text_watch.is_some()}
    pub(crate) fn text_watch_site(&mut self,cs:u16,ip:u32){
        if let Some(w)=self.text_watch.as_mut(){w.site=(cs,ip);}
    }
    pub(crate) fn text_watch_json(&self)->String{
        self.text_watch.as_ref().map_or_else(||r#"{"enabled":false}"#.into(),TextWatch::json)
    }
    pub(crate) fn start_video_watch(&mut self){self.video_watch=Some(VideoWatch::default());}
    pub(crate) fn stop_video_watch(&mut self){self.video_watch=None;}
    pub(crate) fn video_watch_json(&self)->String{
        self.video_watch.as_ref().map_or_else(||r#"{"enabled":false}"#.into(),VideoWatch::json)
    }
    #[inline]
    fn note_video_write(&mut self,index:usize,changed:Option<bool>){
        if let Some(w)=self.video_watch.as_mut(){w.note(index,changed);}
    }
    pub fn bytes(&self)->&[u8] {&self.data}
    /// Physical read-only spans, for reproducing an initialized guest snapshot.
    pub fn a20_enabled(&self)->bool{self.a20_enabled}
    pub fn planar(&self)->Option<&super::vga_planar::Planar>{self.planar.as_deref()}
    pub fn planar_mut(&mut self)->Option<&mut super::vga_planar::Planar>{self.planar.as_deref_mut()}
    pub fn set_planar(&mut self,planar:Option<super::vga_planar::Planar>){self.planar=planar.map(Box::new);}
    /// Plane offset of a physical byte in the planar window, while one is mapped.
    fn window(&self,index:usize)->Option<usize>{
        let start=super::vga_planar::WINDOW as usize;
        if self.planar.is_some()&&(start..start+super::vga_planar::PLANE).contains(&index){Some(index-start)}else{None}
    }
    fn alias(&self,address:u32,bytes:usize)->Option<usize>{
        let offset=address.checked_sub(PLANE_ALIAS)? as usize;
        (self.planar.is_some()&&offset+bytes<=4*super::vga_planar::PLANE).then_some(offset)
    }
    pub fn set_a20_enabled(&mut self,enabled:bool){self.a20_enabled=enabled;}
    pub fn physical_address(&self,address:u32)->u32{if self.a20_enabled{address}else{address&!0x10_0000}}
    /// A contiguous bus span, or None when it crosses an A20 boundary.
    pub fn contiguous_slice(&self,address:u32,bytes:usize)->Option<&[u8]>{
        // Paged fetches go byte by byte, so the walk marks the code page.
        if self.paging.get().is_some(){return None;}
        if !self.a20_enabled&&bytes>0x100000-(address as usize&0xfffff){return None;}
        let start=self.physical_address(address)as usize;let end=start.checked_add(bytes)?;self.data.get(start..end)
    }
    fn range(&self,address:u32,bytes:usize,write:bool)->Result<std::ops::Range<usize>,AccessFault> {
        let fault=AccessFault::bus(address,bytes,write);
        let start=usize::try_from(address).map_err(|_|fault)?;
        let end=start.checked_add(bytes).ok_or(fault)?;
        if end>self.data.len() {return Err(fault);}
        // A 32-bit guest access cannot wrap to the beginning of RAM.
        if (address as u64)+(bytes as u64)>0x1_0000_0000 {return Err(fault);}
        if write && bytes!=0 && self.readonly.iter().any(|r|start<r.end && end>r.start) {return Err(fault);}
        Ok(start..end)
    }
    pub fn protect(&mut self,address:u32,bytes:usize)->Result<(),AccessFault> {
        self.settle();
        let range=self.range(address,bytes,false)?;
        if !range.is_empty() {self.readonly.push(range);}
        Ok(())
    }
    /// Marks physical bytes as ROM: later stores to them are accepted and
    /// dropped. Load the contents first (with `write_bytes`).
    pub fn rom(&mut self,address:u32,bytes:usize)->Result<(),AccessFault> {
        self.settle();
        let range=self.range(address,bytes,false)?;
        if !range.is_empty() {self.rom.push(range);}
        Ok(())
    }
    /// Makes physical bytes ordinary RAM again (the reverse of `rom`).
    pub fn unrom(&mut self,address:u32,bytes:usize)->Result<(),AccessFault> {
        self.settle();
        let cut=self.range(address,bytes,false)?;
        let mut kept=Vec::new();
        for r in self.rom.drain(..){
            if r.end<=cut.start||r.start>=cut.end{kept.push(r);continue;}
            if r.start<cut.start{kept.push(r.start..cut.start);}
            if r.end>cut.end{kept.push(cut.end..r.end);}
        }
        self.rom=kept;Ok(())
    }
    fn in_rom(&self,index:usize)->bool{self.rom.iter().any(|r|r.contains(&index))}
    pub fn check_write(&self,address:u32,width:MemWidth)->Result<(),AccessFault> {
        self.check_bus(address,width.bytes()as usize,true)
    }
    fn check_bus(&self,address:u32,bytes:usize,write:bool)->Result<(),AccessFault>{
        let fault=AccessFault::bus(address,bytes,write);
        if u64::from(address).checked_add(bytes as u64).is_none_or(|end|end>0x1_0000_0000){return Err(fault);}
        if self.paging.get().is_some(){
            // Byte by byte: a page fault names the first byte that faults.
            for i in 0..bytes as u32{
                let at=self.locate(address+i,write)?;
                self.range(at,1,write).map_err(|_|fault)?;
            }
            return Ok(());
        }
        if self.alias(address,bytes).is_some(){return Ok(());}
        let mut done=0usize;
        while done<bytes{
            let guest=address+done as u32;
            let chunk=if self.a20_enabled{bytes-done}else{(bytes-done).min(0x100000-(guest as usize&0xfffff))};
            self.range(self.physical_address(guest),chunk,write).map_err(|_|fault)?;done+=chunk;
        }
        if bytes==0{self.range(self.physical_address(address),0,write).map_err(|_|fault)?;}
        Ok(())
    }
    pub fn load(&self,address:u32,width:MemWidth)->Result<u32,AccessFault> {
        let bytes=width.bytes()as usize;self.check_bus(address,bytes,false)?;
        let mut value=0u32;
        if self.paging.get().is_some(){
            for shift in 0..bytes{
                let index=self.locate(address+shift as u32,false)?as usize;
                let byte=match self.window(index){Some(at)=>self.planar.as_deref().unwrap().read(at),None=>self.byte(index)};
                value|=u32::from(byte)<<(shift*8);
            }
            return Ok(value);
        }
        if let Some(offset)=self.alias(address,bytes){
            let p=self.planar.as_deref().unwrap();
            for shift in 0..bytes{let at=offset+shift;value|=u32::from(p.peek(at/super::vga_planar::PLANE,at%super::vga_planar::PLANE))<<(shift*8);}
            return Ok(value);
        }
        for shift in 0..bytes {
            let index=self.physical_address(address+shift as u32)as usize;
            let byte=match self.window(index){Some(at)=>self.planar.as_deref().unwrap().read(at),None=>self.data[index]};
            value|=u32::from(byte)<<(shift*8);
        }
        Ok(value)
    }
    /// Loads as `load` does, then overlays bytes of stores this instruction
    /// has staged but not committed, in staging order, so a later read in the
    /// same instruction sees its earlier writes (ENTER reads frame pointers
    /// it may just have pushed).
    pub fn load_staged(&self,address:u32,width:MemWidth,staged:&[Option<(u32,MemWidth,u32)>],extra:&[(u32,MemWidth,u32)])->Result<u32,AccessFault> {
        let mut value=self.load(address,width)?;
        let bytes=width.bytes()as u32;
        for &(at,w,v) in staged.iter().flatten().chain(extra.iter()){
            for b in 0..w.bytes()as u32{
                let Ok(phys)=self.locate(at.wrapping_add(b),true) else{continue;};
                for i in 0..bytes{
                    if self.locate(address.wrapping_add(i),false).ok()==Some(phys){value=(value&!(0xff<<(i*8)))|(((v>>(b*8))&0xff)<<(i*8));}
                }
            }
        }
        Ok(value)
    }
    pub fn store(&mut self,address:u32,width:MemWidth,value:u32)->Result<(),AccessFault> {
        self.settle();
        let bytes=width.bytes()as usize;self.check_bus(address,bytes,true)?;
        if self.paging.get().is_some(){
            for shift in 0..bytes{
                let index=self.locate(address+shift as u32,true)?as usize;let byte=(value>>(shift*8)) as u8;
                if self.video_watch.is_some() && !self.in_rom(index){self.note_video_write(index,if self.window(index).is_some(){None}else{Some(self.data[index]!=byte)});}
                match self.window(index){Some(at)=>self.planar.as_deref_mut().unwrap().write(at,byte),None if self.in_rom(index)=>{},None=>{if let Some(w)=self.text_watch.as_mut(){w.write(index,self.data[index],byte);}self.code_watch.write(index,1);self.data[index]=byte;}}
            }
            self.settle();
            return Ok(());
        }
        if let Some(offset)=self.alias(address,bytes){
            if self.video_watch.is_some(){for shift in 0..bytes{self.note_video_write(0xa0000+(offset+shift)%super::vga_planar::PLANE,None);}}
            let p=self.planar.as_deref_mut().unwrap();
            for shift in 0..bytes{let at=offset+shift;p.poke(at/super::vga_planar::PLANE,at%super::vga_planar::PLANE,(value>>(shift*8))as u8);}
            return Ok(());
        }
        for shift in 0..bytes {
            let index=self.physical_address(address+shift as u32)as usize;let byte=(value>>(shift*8)) as u8;
            if self.video_watch.is_some() && !self.in_rom(index){self.note_video_write(index,if self.window(index).is_some(){None}else{Some(self.data[index]!=byte)});}
                match self.window(index){Some(at)=>self.planar.as_deref_mut().unwrap().write(at,byte),None if self.in_rom(index)=>{},None=>{if let Some(w)=self.text_watch.as_mut(){w.write(index,self.data[index],byte);}self.code_watch.write(index,1);self.data[index]=byte;}}
        }
        Ok(())
    }
    // Debugger bulk writes have the same full-range permission check.
    pub fn write_bytes(&mut self,address:u32,data:&[u8])->Result<(),AccessFault> {
        self.settle();
        self.check_bus(address,data.len(),true)?;
        if self.paging.get().is_some(){
            for (i,&b) in data.iter().enumerate(){let index=self.locate(address+i as u32,true)?as usize;if !self.in_rom(index){self.note_video_write(index,Some(self.data[index]!=b));if let Some(w)=self.text_watch.as_mut(){w.write(index,self.data[index],b);}self.code_watch.write(index,1);self.data[index]=b;}}
            self.settle();
            return Ok(());
        }
        let mut done=0;while done<data.len(){let guest=address+done as u32;let chunk=if self.a20_enabled{data.len()-done}else{(data.len()-done).min(0x100000-(guest as usize&0xfffff))};let start=self.physical_address(guest)as usize;self.code_watch.write(start,chunk);
            // ROM bytes stay as they are, as for `store`.
            for i in 0..chunk{if !self.in_rom(start+i){self.note_video_write(start+i,Some(self.data[start+i]!=data[done+i]));if let Some(w)=self.text_watch.as_mut(){w.write(start+i,self.data[start+i],data[done+i]);}self.data[start+i]=data[done+i];}}
            done+=chunk;}Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/memory.rs"]
mod tests;
