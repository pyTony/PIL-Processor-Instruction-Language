//! Decoded 386 addresses and already-loaded segment caches. No byte decoder.
use super::{ExecError, MemWidth};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gpr { Eax, Ecx, Edx, Ebx, Esp, Ebp, Esi, Edi }
impl Gpr {
    fn index(self) -> usize { self as usize }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentRegister { Es, Cs, Ss, Ds, Fs, Gs }
impl SegmentRegister {
    pub fn index(self) -> usize { self as usize }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Address16 { BxSi, BxDi, BpSi, BpDi, Si, Di, Bp, Bx, Direct }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveAddress {
    Bits16 { form: Address16, displacement: u16 },
    Bits32 { base: Option<Gpr>, index: Option<Gpr>, scale: u8, displacement: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedAddress { pub offset: u32, pub segment: SegmentRegister }
impl EffectiveAddress {
    // Displacements have already been sign-extended (disp8) and normalized to
    // address size by the caller. Address size is independent of operand size.
    pub fn resolve(self, regs: &[u32;8], override_segment: Option<SegmentRegister>) -> Result<ResolvedAddress, ExecError> {
        let (offset, stack) = match self {
            Self::Bits16 { form, displacement } => {
                let (base, index) = match form {
                    Address16::BxSi => (Some(Gpr::Ebx), Some(Gpr::Esi)),
                    Address16::BxDi => (Some(Gpr::Ebx), Some(Gpr::Edi)),
                    Address16::BpSi => (Some(Gpr::Ebp), Some(Gpr::Esi)),
                    Address16::BpDi => (Some(Gpr::Ebp), Some(Gpr::Edi)),
                    Address16::Si => (None, Some(Gpr::Esi)),
                    Address16::Di => (None, Some(Gpr::Edi)),
                    Address16::Bp => (Some(Gpr::Ebp), None),
                    Address16::Bx => (Some(Gpr::Ebx), None),
                    Address16::Direct => (None, None),
                };
                let sum = base.map_or(0, |r|regs[r.index()]).wrapping_add(index.map_or(0, |r|regs[r.index()])).wrapping_add(u32::from(displacement));
                (sum & 0xffff, base == Some(Gpr::Ebp))
            }
            Self::Bits32 { base, index, scale, displacement } => {
                if ![1,2,4,8].contains(&scale) || index == Some(Gpr::Esp) || (index.is_none() && scale != 1) {
                    return Err(ExecError::InvalidContext);
                }
                let offset = base.map_or(0, |r|regs[r.index()]).wrapping_add(index.map_or(0, |r|regs[r.index()]).wrapping_mul(u32::from(scale))).wrapping_add(displacement);
                (offset, matches!(base, Some(Gpr::Ebp | Gpr::Esp)))
            }
        };
        Ok(ResolvedAddress { offset, segment: override_segment.unwrap_or(if stack {SegmentRegister::Ss} else {SegmentRegister::Ds}) })
    }
}

// These are normalized hidden-cache attributes, not raw GDT/LDT descriptors.
// Caller has already performed descriptor-load, present and privilege checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentAccess {
    // Supplied by callers/tests; the current monitor creates usable caches only.
    #[allow(dead_code)]
    Unusable,
    Data { writable: bool, expand_down: bool, big: bool, dpl: u8 },
    // A protected-mode code segment: `big` is the D bit (32-bit default
    // operand and address size) and `dpl` is the CPL while it is CS.
    Code { readable: bool, conforming: bool, big: bool, dpl: u8 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentCache { pub base: u32, pub limit: u32, pub access: SegmentAccess }
impl SegmentCache {
    // Ordinary real-mode cache after a segment load; does not model transitions
    // from protected mode with retained hidden caches ("unreal mode").
    // CS uses these ordinary real-mode data-access permissions as well;
    // Code caches describe protected-mode permissions, not the register name.
    pub fn real_mode(selector: u16) -> Self {
        Self { base: u32::from(selector) << 4, limit: 0xffff,
            access: SegmentAccess::Data { writable: true, expand_down: false, big: false, dpl: 0 } }
    }
    /// The cache a segment load makes in virtual-8086 mode: as in real mode,
    /// but at privilege level 3. A CS cache of this kind is how the machine
    /// state marks virtual-8086 mode (`SegmentState::v86`), together with
    /// EFLAGS.VM.
    pub fn v86(selector: u16) -> Self {
        Self { base: u32::from(selector) << 4, limit: 0xffff,
            access: SegmentAccess::Data { writable: true, expand_down: false, big: false, dpl: 3 } }
    }
    /// The 32-bit default operand/address size of a code segment (CS.D).
    pub fn default_32(&self) -> bool { matches!(self.access, SegmentAccess::Code { big: true, .. }) }
}
/// A raw 8-byte GDT/LDT entry decoded into its fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor { pub base: u32, pub limit: u32, pub access: u8, pub big: bool }
impl Descriptor {
    pub fn decode(raw: u64) -> Self {
        let (lo, hi) = (raw as u32, (raw >> 32) as u32);
        let mut limit = (lo & 0xffff) | (hi & 0xf_0000);
        if hi & 0x80_0000 != 0 { limit = (limit << 12) | 0xfff; }
        Self { base: (lo >> 16) | ((hi & 0xff) << 16) | (hi & 0xff00_0000), limit, access: (hi >> 8) as u8, big: hi & 0x40_0000 != 0 }
    }
    pub fn present(&self) -> bool { self.access & 0x80 != 0 }
    pub fn dpl(&self) -> u8 { (self.access >> 5) & 3 }
    /// Code or data (S=1), as opposed to a system descriptor or gate.
    pub fn segment(&self) -> bool { self.access & 0x10 != 0 }
    pub fn code(&self) -> bool { self.access & 0x08 != 0 }
    pub fn cache(&self) -> SegmentCache {
        let t = self.access;
        let access = if self.code() {
            SegmentAccess::Code { readable: t & 2 != 0, conforming: t & 4 != 0, big: self.big, dpl: self.dpl() }
        } else {
            SegmentAccess::Data { writable: t & 2 != 0, expand_down: t & 4 != 0, big: self.big, dpl: self.dpl() }
        };
        SegmentCache { base: self.base, limit: self.limit, access }
    }
}
/// An IDT interrupt or trap gate, or a call gate: target selector and
/// offset, whether it is a trap gate (IF stays as it is), whether it is a
/// 386 gate (32-bit frame and offset) rather than a 286 one, and for a call
/// gate the number of stack parameters it copies to an inner level. An IDT
/// task gate has `task` set and names a TSS selector (task.rs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gate { pub selector: u16, pub offset: u32, pub trap: bool, pub big: bool, pub count: u8, pub task: bool }
impl Gate {
    /// Decodes the gate fields of a raw 8-byte descriptor of a 386 (`big`)
    /// or 286 gate.
    pub fn decode(raw: u64, big: bool) -> Self {
        let low = (raw & 0xffff) as u32;
        let offset = if big { low | (((raw >> 48) as u32) << 16) } else { low };
        Self { selector: (raw >> 16) as u16, offset, trap: (raw >> 40) & 1 != 0, big, count: ((raw >> 32) & 0x1f) as u8, task: false }
    }
}
/// A segment register load, ready to commit: the visible selector, the new
/// hidden cache and, in protected mode, the descriptor's access byte to mark
/// accessed (address, new value) when it is not already.
pub type SegmentLoad = (u16, SegmentCache, Option<(u32, u8)>);
/// LDTR or TR: the visible selector and the hidden base, limit and
/// descriptor type (2 for an LDT, 0Bh for a busy 386 TSS, 3 for a busy 286
/// TSS). A null LDTR has selector 0 and limit 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemSegment { pub selector: u16, pub base: u32, pub limit: u32, pub kind: u8 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryContext { pub segment: SegmentRegister, pub cache: SegmentCache }
impl MemoryContext {
    pub fn translate(self, offset: u32, width: MemWidth, write: bool, eip: u32) -> Result<u32, ExecError> {
        self.translate_span(u64::from(offset),u32::from(width.bytes()),write,eip)
    }
    // Widened offsets let multi-slot stack frames reject wrap before any reads.
    pub fn translate_span(self, offset:u64, bytes:u32, write:bool, eip:u32)->Result<u32,ExecError>{
        // An invalid loaded SS cache is a caller error: loading SS is not yet
        // implemented and would have rejected null/code/read-only descriptors.
        if self.segment == SegmentRegister::Ss && !matches!(self.cache.access, SegmentAccess::Data { writable: true, .. }) {
            return Err(ExecError::InvalidContext);
        }
        let gp = ExecError::GeneralProtection { eip, error_code: 0 };
        let (permitted, expand_down, big) = match self.cache.access {
            SegmentAccess::Unusable => return Err(gp),
            SegmentAccess::Data { writable, expand_down, big, .. } => (!write || writable, expand_down, big),
            SegmentAccess::Code { readable, .. } => (!write && readable, false, false),
        };
        if !permitted { return Err(gp); }
        // Use a widened end: a multi-byte operand cannot wrap its segment offset.
        let end = offset.checked_add(u64::from(bytes).saturating_sub(1)).unwrap_or(u64::MAX);
        let in_bounds = if expand_down {
            offset > u64::from(self.cache.limit) && end <= if big {u64::from(u32::MAX)} else {0xffff}
        } else { end <= u64::from(self.cache.limit) };
        if bytes==0 || offset>u64::from(u32::MAX) || !in_bounds {
            return Err(if self.segment == SegmentRegister::Ss {
                ExecError::StackSegment { eip, error_code: 0 }
            } else { gp });
        }
        Ok(self.cache.base.wrapping_add(offset as u32))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddressContext {
    pub effective: EffectiveAddress,
    pub override_segment: Option<SegmentRegister>,
    // ES, CS, SS, DS, FS, GS, in SegmentRegister order.
    pub segments: [SegmentCache;6],
}
impl AddressContext {
    pub fn resolve(self, regs: &[u32;8]) -> Result<(u32, MemoryContext), ExecError> {
        let address = self.effective.resolve(regs, self.override_segment)?;
        Ok((address.offset, MemoryContext { segment: address.segment, cache: self.segments[address.segment.index()] }))
    }
}

// Mutable architectural segment state. Only ordinary real-mode loads are admitted.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SegmentState {
    pub segments:[SegmentCache;6],pub selectors:[u16;6],
    pub msw:u16,pub real_mode:bool,pub ss_shadow:u8,pub sti_shadow:u8,pub tables:[(u16,u32);2],pub controls:[u32;2],pub flag_checkpoint:Option<(u32,u32,u32)>,
    /// LDTR (0) and TR (1).
    pub ldt_tr:[SystemSegment;2],
    /// CR0.PG.
    pub paging:bool,
    /// Debug registers DR0-DR3, DR6 and DR7 (`DEBUG_RESET` after reset).
    /// DR4 and DR5 are reserved on the 386; moves to and from them reach
    /// DR6 and DR7, as on later processors. Breakpoints are not modelled.
    pub debug:[u32;6],
}
/// DR0-DR3, DR6 and DR7 after reset (Intel SDM vol. 3 table 9-1; the 386
/// manual gives no values).
pub const DEBUG_RESET:[u32;6]=[0,0,0,0,0xffff_0ff0,0x0000_0400];
/// TR after reset (Intel SDM vol. 3 table 9-1): selector 0, base 0, limit
/// FFFFh, present. Its type is a busy 386 TSS, as Bochs has it, so a task
/// switch before any LTR stores the old task at linear address 0, as the 386
/// does, instead of faulting.
pub const TR_RESET:SystemSegment=SystemSegment{selector:0,base:0,limit:0xffff,kind:0x0b};
/// The debug register index (0-5 in `SegmentState::debug`) that MOV DRn
/// reaches: DR4 and DR5 are DR6 and DR7.
pub fn debug_index(n:usize)->usize{match n{0..=3=>n,4|6=>4,_=>5}}
impl SegmentState {
    // The mode bit is coherent with the existing normalized mode state.
    pub fn machine_status(&self)->u16{(self.msw&!1)|u16::from(!self.real_mode)}
    /// CR0, CR2, CR3 (`register` 0, 2, 3) or DR0-DR7 (8-15): CPL 0 only,
    /// #GP(0) in virtual-8086 mode and above ring 0.
    pub fn read_control(&self,register:usize,flags:u32,eip:u32)->Result<u32,ExecError>{
        if flags&0x20000!=0{return Err(ExecError::GeneralProtection{eip,error_code:0});}
        if self.cpl()!=0{return Err(ExecError::GeneralProtection{eip,error_code:0});}
        match register{0=>Ok(u32::from(self.machine_status())|(u32::from(self.paging)<<31)),2=>Ok(self.controls[0]),3=>Ok(self.controls[1]),8..=15=>Ok(self.debug[debug_index(register-8)]),_=>Err(ExecError::InvalidContext)}
    }
    pub fn validate_control(&self,register:usize,value:u32,flags:u32,eip:u32)->Result<(),ExecError>{
        self.read_control(register,flags,eip)?;
        if register==0&&value&0x80000001==0x80000000{return Err(ExecError::GeneralProtection{eip,error_code:0});}
        // PE and PG may be set or cleared (PG needs PE); CR0 bits the 386 does
        // not have stay unsupported, as do CR3's low bits (486 PCD/PWT).
        if (register==0&&value&!0x8000_001f!=0)||(register==3&&value&0xfff!=0){return Err(ExecError::UnsupportedControl{eip,register:register as u8,value});}
        // Registers 8-15 are DR0-DR7. DR7 with a breakpoint enabled (L0-G3)
        // or general detect (GD) is not modelled, so it stops instead of
        // being stored and never firing.
        if register>=8&&debug_index(register-8)==5&&value&0x20ff!=0{return Err(ExecError::UnsupportedControl{eip,register:register as u8,value});}
        Ok(())
    }
    pub fn commit_control(&mut self,register:usize,value:u32){match register{0=>{self.msw=value as u16;self.real_mode=value&1==0;self.paging=value>>31!=0;},2=>self.controls[0]=value,3=>self.controls[1]=value,8..=15=>self.debug[debug_index(register-8)]=value,_=>unreachable!()}}
    /// The RAM paging state these registers select: CR3's directory and
    /// CPL 3 for user-level accesses, while CR0.PG is set.
    pub fn ram_paging(&self)->Option<super::memory::Paging>{
        self.paging.then(||super::memory::Paging{directory:self.controls[1]&!0xfff,user:self.cpl()==3})
    }
    /// Current privilege level: the DPL of the CS cache in protected mode
    /// (3 in virtual-8086 mode).
    pub fn cpl(&self)->u8{
        if self.real_mode{return 0;}
        match self.segments[1].access{SegmentAccess::Code{dpl,..}|SegmentAccess::Data{dpl,..}=>dpl,_=>0}
    }
    /// Virtual-8086 mode: protected mode with a V86 CS cache
    /// (`SegmentCache::v86`). The real-mode cache CS keeps between setting
    /// PE and the first far jump has DPL 0, so it does not count.
    pub fn v86(&self)->bool{!self.real_mode&&matches!(self.segments[1].access,SegmentAccess::Data{dpl:3,..})}
    /// Loads segment register `reg` (0..5, SegmentRegister order) with
    /// `selector`. In real mode only the base changes, so limits and
    /// attributes loaded in protected mode stay ("unreal mode"); CS gets the
    /// ordinary real-mode cache. In virtual-8086 mode every load makes the
    /// full V86 cache (base selector*16, limit FFFFh, level 3). In protected
    /// mode the descriptor is read
    /// from the GDT through `read` (a linear 8-byte read) and checked as the
    /// 386 does for MOV/POP/LDS-style loads (DS/ES/FS/GS, SS) and for a far
    /// JMP/CALL/RETF to a code segment at the same privilege (CS). TI=1
    /// selectors read the LDT. Gates and privilege changes are in
    /// privilege.rs.
    pub fn load_segment(&self,reg:usize,selector:u16,eip:u32,read:&mut dyn FnMut(u32)->Result<u64,ExecError>)->Result<SegmentLoad,ExecError>{
        if self.v86(){return Ok((selector,SegmentCache::v86(selector),None));}
        if self.real_mode{
            let cache=match self.segments[reg]{
                c if reg!=1&&c.access!=SegmentAccess::Unusable=>SegmentCache{base:u32::from(selector)<<4,..c},
                _=>SegmentCache::real_mode(selector),
            };
            return Ok((selector,cache,None));
        }
        let gp=|code:u16|ExecError::GeneralProtection{eip,error_code:code};
        let (index,rpl,cpl)=(selector&0xfff8,(selector&3) as u8,self.cpl());
        if index==0&&selector&4==0{
            // A null selector: allowed in DS, ES, FS and GS (later use faults).
            return if matches!(reg,0|3|4|5){Ok((selector,SegmentCache{base:0,limit:0,access:SegmentAccess::Unusable},None))}else{Err(gp(0))};
        }
        let code=selector&0xfffc;
        let Some(at)=self.descriptor_address(selector) else{return Err(gp(code));};
        let d=Descriptor::decode(read(at)?);
        if !d.segment(){
            // Call gates, task gates and TSS descriptors are read by
            // privilege.rs before a far JMP or CALL gets here.
            return Err(gp(code));
        }
        let (readable_or_writable,conforming)=(d.access&2!=0,d.access&4!=0);
        let mut visible=selector;
        match reg{
            1=>{
                // A far JMP or CALL straight to a code segment stays at CPL:
                // a conforming one needs DPL <= CPL, a nonconforming one
                // RPL <= CPL and DPL = CPL. Returns go through privilege.rs.
                if !d.code(){return Err(gp(code));}
                if conforming{if d.dpl()>cpl{return Err(gp(code));}}
                else if rpl>cpl||d.dpl()!=cpl{return Err(gp(code));}
                if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
                visible=(selector&!3)|u16::from(cpl);
            }
            2=>{
                if rpl!=cpl||d.code()||!readable_or_writable||d.dpl()!=cpl{return Err(gp(code));}
                if !d.present(){return Err(ExecError::StackSegment{eip,error_code:code});}
            }
            _=>{
                if d.code()&&!readable_or_writable{return Err(gp(code));}
                if (!d.code()||!conforming)&&(rpl>d.dpl()||cpl>d.dpl()){return Err(gp(code));}
                if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
            }
        }
        let mut cache=d.cache();
        if reg==1{if let SegmentAccess::Code{ref mut dpl,..}=cache.access{if conforming{*dpl=cpl;}}}
        Ok((visible,cache,(d.access&1==0).then_some((at.wrapping_add(5),d.access|1))))
    }
    /// The linear address of the GDT or LDT entry a selector names, or None
    /// for a null GDT selector, a TI=1 selector with no LDT loaded, or an
    /// index past the table limit.
    pub fn descriptor_address(&self,selector:u16)->Option<u32>{
        let index=u32::from(selector&0xfff8);
        let (base,limit)=if selector&4!=0{
            let ldt=self.ldt_tr[0];
            if ldt.selector&0xfffc==0{return None;}
            (ldt.base,ldt.limit)
        }else{
            if index==0{return None;}
            (self.tables[0].1,u32::from(self.tables[0].0))
        };
        (index+7<=limit).then(||base.wrapping_add(index))
    }
    /// The raw descriptor a selector names, for LAR, LSL, VERR and VERW,
    /// which check it themselves and never fault on a bad selector.
    pub fn find_descriptor(&self,selector:u16,read:&mut dyn FnMut(u32)->Result<u64,ExecError>)->Result<Option<u64>,ExecError>{
        self.descriptor_address(selector).map(&mut *read).transpose()
    }
    /// LLDT (`which`=0) or LTR (`which`=1) at CPL 0 in protected mode: the
    /// new LDTR or TR and, for LTR, the access byte to store with the TSS
    /// marked busy. A null selector empties LDTR; LTR faults on it.
    pub fn load_system(&self,which:usize,selector:u16,eip:u32,read:&mut dyn FnMut(u32)->Result<u64,ExecError>)->Result<(SystemSegment,Option<(u32,u8)>),ExecError>{
        let code=selector&0xfffc;
        let gp=|code:u16|ExecError::GeneralProtection{eip,error_code:code};
        if code==0{return if which==0{Ok((SystemSegment{selector,..SystemSegment::default()},None))}else{Err(gp(0))};}
        // Both tables live in the GDT: a TI=1 selector faults.
        if selector&4!=0{return Err(gp(code));}
        let at=self.descriptor_address(selector).ok_or(gp(code))?;
        let d=Descriptor::decode(read(at)?);
        let kind=d.access&0x1f;
        let valid=if which==0{kind==2}else{kind==1||kind==9};
        if !valid{return Err(gp(code));}
        if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
        let busy=(which==1).then_some((at.wrapping_add(5),d.access|2));
        Ok((SystemSegment{selector,base:d.base,limit:d.limit,kind:kind|if which==1{2}else{0}},busy))
    }
    /// The interrupt or trap gate for `vector` in the IDT, checked as the
    /// 386 does before it reads the target: the vector inside the IDTR limit,
    /// a 286 or 386 interrupt or trap gate, for INT n/INT3/INTO (`software`)
    /// a gate DPL of at least CPL, and the present bit. Faults name the IDT
    /// entry (vector*8+2) with EXT (bit 0) set when an exception or external
    /// event is being delivered. A task gate comes back with `task` set
    /// and its TSS selector, after the same DPL and present checks.
    pub fn interrupt_gate(&self,vector:u8,software:bool,eip:u32,read:&mut dyn FnMut(u32)->Result<u64,ExecError>)->Result<Gate,ExecError>{
        let code=u16::from(vector)*8+2+u16::from(!software);
        let gp=ExecError::GeneralProtection{eip,error_code:code};
        let (limit,base)=self.tables[1];
        let at=u32::from(vector)*8;
        if at+7>u32::from(limit){return Err(gp);}
        let raw=read(base.wrapping_add(at))?;
        let access=(raw>>40) as u8;
        let (big,task)=match access&0x1f{0x06|0x07=>(false,false),0x0e|0x0f=>(true,false),0x05=>(false,true),_=>return Err(gp)};
        if software&&((access>>5)&3)<self.cpl(){return Err(gp);}
        if access&0x80==0{return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
        Ok(Gate{count:0,task,..Gate::decode(raw,big)})
    }
    /// The code segment an interrupt gate names: a present code segment
    /// with DPL at most CPL. A conforming one runs at CPL; a nonconforming
    /// one at its DPL, which below CPL means the stack switch through the TSS
    /// (the caller does it). Selector faults carry EXT as in `interrupt_gate`.
    pub fn interrupt_target(&self,selector:u16,software:bool,eip:u32,read:&mut dyn FnMut(u32)->Result<u64,ExecError>)->Result<SegmentLoad,ExecError>{
        let ext=u16::from(!software);
        let code=(selector&0xfffc)|ext;
        let gp=|code:u16|ExecError::GeneralProtection{eip,error_code:code};
        let cpl=self.cpl();
        let Some(at)=self.descriptor_address(selector) else{return Err(gp(if selector&0xfffc==0{ext}else{code}));};
        let d=Descriptor::decode(read(at)?);
        if !d.segment()||!d.code()||d.dpl()>cpl{return Err(gp(code));}
        if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
        // From virtual-8086 mode only a nonconforming ring 0 handler will do.
        if self.v86()&&(d.access&4!=0||d.dpl()!=0){return Err(gp(code));}
        let level=if d.access&4!=0{cpl}else{d.dpl()};
        let mut cache=d.cache();
        if let SegmentAccess::Code{ref mut dpl,..}=cache.access{*dpl=level;}
        Ok(((selector&!3)|u16::from(level),cache,(d.access&1==0).then_some((at.wrapping_add(5),d.access|1))))
    }
    /// I/O permission for IN, OUT, INS and OUTS on `bytes` ports from
    /// `port`. Real mode, and protected mode with CPL <= IOPL, may use every
    /// port. Otherwise (and always in virtual-8086 mode) each port's bit in
    /// the 386 TSS I/O bitmap must be clear: the bitmap starts at the offset
    /// in the TSS word at 66h, and a bit beyond the TSS limit, a 286 TSS or a
    /// set bit is #GP(0). `read` reads one byte at a linear address.
    pub fn io_permitted(&self,flags:u32,port:u16,bytes:u8,eip:u32,read:&mut dyn FnMut(u32)->Result<u8,ExecError>)->Result<(),ExecError>{
        if self.real_mode{return Ok(());}
        let v86=flags&0x20000!=0;
        if !v86&&self.cpl()<=((flags>>12)&3) as u8{return Ok(());}
        let gp=ExecError::GeneralProtection{eip,error_code:0};
        let tss=self.ldt_tr[1];
        if tss.kind!=0x0b||tss.limit<0x67{return Err(gp);}
        let base=u32::from(read(tss.base.wrapping_add(0x66))?)|(u32::from(read(tss.base.wrapping_add(0x67))?)<<8);
        let first=base+u32::from(port)/8;
        // The 386 reads two bytes, so the byte after the port's must be inside the limit too.
        if first+1>tss.limit{return Err(gp);}
        let map=u32::from(read(tss.base.wrapping_add(first))?)|(u32::from(read(tss.base.wrapping_add(first+1))?)<<8);
        let wanted=((1u32<<bytes)-1)<<(port%8);
        if map&wanted!=0{return Err(gp);}
        Ok(())
    }
    pub fn checkpoint_flags(&mut self,eip:u32,flags:u32,undefined:u32){if self.flag_checkpoint.map(|v|v.0)!=Some(eip){self.flag_checkpoint=Some((eip,flags,undefined));}}
    pub fn recover_flags(&mut self)->Option<(u32,u32)>{self.flag_checkpoint.take().map(|(_,f,u)|(f,u))}
    pub fn retired(&mut self){self.flag_checkpoint=None;self.ss_shadow=self.ss_shadow.saturating_sub(1);self.sti_shadow=self.sti_shadow.saturating_sub(1);}
}

#[cfg(test)]
#[path = "../tests/unit/address.rs"]
mod tests;
