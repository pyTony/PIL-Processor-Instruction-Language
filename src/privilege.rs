//! Far transfers with privilege checks: far JMP and CALL through call
//! gates, a CALL through a gate to an inner level (the stack switch through
//! the TSS, with parameters copied), and RETF or IRET to the same or an
//! outer level (SS:ESP popped, data segments the new level may not use
//! nulled). Shared by the interpreter, the generated Rust (this file is
//! embedded there) and interrupt delivery, which uses `inner_stack`.
//!
//! Every function only reads and checks: it returns a `Transfer` that the
//! caller applies once the whole instruction has passed its checks, so a
//! fault leaves the machine as it was. Descriptor tables, the TSS and an
//! inner-level stack are accessed at supervisor level, as the 386 does.
//! Task gates and TSS descriptors (task switches) are not modelled yet.
use super::{ExecError,MemWidth};
use super::memory::Ram;
use super::address::{Descriptor,Gate,MemoryContext,SegmentAccess,SegmentCache,SegmentRegister,SegmentState};

/// The effects of a far transfer, to apply after every check has passed.
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct Transfer{
    pub eip:u32,
    /// Segment registers to load (register, visible selector, cache): CS,
    /// then SS and any data segment register an outer return nulls.
    pub segments:Vec<(usize,u16,SegmentCache)>,
    /// The new ESP, whole, when the transfer changes it.
    pub esp:Option<u32>,
    /// Stack pushes and accessed-bit stores: linear address, width, value,
    /// and whether the store is at user level.
    pub stores:Vec<(u32,MemWidth,u32,bool)>,
}
impl Transfer{
    /// Applies the transfer. The stores were checked when it was built.
    pub fn commit(self,regs:&mut [u32;8],eip:&mut u32,machine:&mut SegmentState,ram:&mut Ram)->Result<(),ExecError>{
        let at=*eip;
        for (address,width,value,user) in self.stores{ram.store_as(user,address,width,value).map_err(|f|f.error(at))?;}
        if let Some(esp)=self.esp{regs[4]=esp;}
        for (reg,selector,cache) in self.segments{machine.selectors[reg]=selector;machine.segments[reg]=cache;}
        *eip=self.eip;
        Ok(())
    }
    fn accessed(&mut self,ram:&Ram,mark:Option<(u32,u8)>,eip:u32)->Result<(),ExecError>{
        if let Some((at,value))=mark{
            ram.as_level(false,|r|r.check_write(at,MemWidth::Byte)).map_err(|f|f.error(eip))?;
            self.stores.push((at,MemWidth::Byte,u32::from(value),false));
        }
        Ok(())
    }
}

/// Reads the 8-byte descriptor at linear `at`, at supervisor level.
pub fn read_descriptor(ram:&Ram,at:u32,eip:u32)->Result<u64,ExecError>{
    ram.as_level(false,|r|{
        let lo=r.load(at,MemWidth::Dword).map_err(|f|f.error(eip))?;
        let hi=r.load(at.wrapping_add(4),MemWidth::Dword).map_err(|f|f.error(eip))?;
        Ok(u64::from(lo)|(u64::from(hi)<<32))
    })
}
fn stack_mask(cache:&SegmentCache)->u32{if matches!(cache.access,SegmentAccess::Data{big:true,..}){u32::MAX}else{0xffff}}
fn gp(eip:u32,error_code:u16)->ExecError{ExecError::GeneralProtection{eip,error_code}}

/// Pushes onto a stack, slot by slot as the 386 does (a 16-bit SP wraps at
/// each slot), checking each one; nothing is written until `Transfer::commit`.
pub struct Frame{context:MemoryContext,mask:u32,pub sp:u32,user:bool,code:u16,eip:u32}
impl Frame{
    /// A stack at `cache`, starting at `sp`; `code` is the #SS error code
    /// for a push that leaves the segment, `user` the level of the writes.
    pub fn new(cache:SegmentCache,sp:u32,user:bool,code:u16,eip:u32)->Self{
        Self{context:MemoryContext{segment:SegmentRegister::Ss,cache},mask:stack_mask(&cache),sp,user,code,eip}
    }
    pub fn push(&mut self,ram:&Ram,width:MemWidth,value:u32,stores:&mut Vec<(u32,MemWidth,u32,bool)>)->Result<(),ExecError>{
        let sp=(self.sp&!self.mask)|(self.sp.wrapping_sub(u32::from(width.bytes()))&self.mask);
        let at=self.context.translate(sp&self.mask,width,true,self.eip).map_err(|e|match e{
            ExecError::StackSegment{eip,..}=>ExecError::StackSegment{eip,error_code:self.code},e=>e})?;
        ram.as_level(self.user,|r|r.check_write(at,width)).map_err(|f|f.error(self.eip))?;
        stores.push((at,width,value,self.user));
        self.sp=sp;
        Ok(())
    }
}
/// Reads a stack slot at SS offset `offset` (already wrapped).
fn read_stack(ram:&Ram,cache:SegmentCache,offset:u32,width:MemWidth,user:bool,eip:u32)->Result<u32,ExecError>{
    let at=MemoryContext{segment:SegmentRegister::Ss,cache}.translate(offset,width,false,eip)?;
    ram.as_level(user,|r|r.load(at,width)).map_err(|f|f.error(eip))
}

/// The stack for privilege level `dpl` from the current TSS (TR): SS and
/// ESP at 4+8*dpl in a 386 TSS, SS and SP at 2+4*dpl in a 286 one. Faults
/// are #TS with the TSS selector (an entry past the TSS limit) or the stack
/// selector, plus `ext`; a stack segment that is not present is #SS. The
/// stack must be a writable data segment with RPL and DPL equal to `dpl`.
/// Returns the SS selector, its cache, its accessed-bit store and ESP.
pub fn inner_stack(machine:&SegmentState,ram:&Ram,dpl:u8,ext:u16,eip:u32)->Result<(u16,SegmentCache,Option<(u32,u8)>,u32),ExecError>{
    let tss=machine.ldt_tr[1];
    let ts=|code:u16|ExecError::InvalidTss{eip,error_code:code};
    let big=match tss.kind{0x0b=>true,0x03=>false,_=>return Err(ts((tss.selector&0xfffc)|ext))};
    let (at,last)=if big{(4+8*u32::from(dpl),7)}else{(2+4*u32::from(dpl),3)};
    if at+last>tss.limit{return Err(ts((tss.selector&0xfffc)|ext));}
    let read=|offset:u32,width:MemWidth|ram.as_level(false,|r|r.load(tss.base.wrapping_add(offset),width)).map_err(|f|f.error(eip));
    let (esp,selector)=if big{(read(at,MemWidth::Dword)?,read(at+4,MemWidth::Word)? as u16)}else{(read(at,MemWidth::Word)?,read(at+2,MemWidth::Word)? as u16)};
    let code=(selector&0xfffc)|ext;
    if selector&0xfffc==0{return Err(ts(ext));}
    let at=machine.descriptor_address(selector).ok_or(ts(code))?;
    let d=Descriptor::decode(read_descriptor(ram,at,eip)?);
    if (selector&3) as u8!=dpl||!d.segment()||d.code()||d.access&2==0||d.dpl()!=dpl{return Err(ts(code));}
    if !d.present(){return Err(ExecError::StackSegment{eip,error_code:code});}
    Ok((selector,d.cache(),(d.access&1==0).then_some((at.wrapping_add(5),d.access|1)),esp))
}

/// What a far JMP or CALL selector names.
enum Target{
    /// A code segment, already checked for a transfer at CPL.
    Code((u16,SegmentCache,Option<(u32,u8)>)),
    /// A call gate and the code segment it names (descriptor and its
    /// address), checked as far as JMP and CALL share the rules.
    Gate{gate:Gate,code:Descriptor,code_at:u32},
}
fn far_target(machine:&SegmentState,ram:&Ram,selector:u16,eip:u32)->Result<Target,ExecError>{
    let mut read=|at:u32|read_descriptor(ram,at,eip);
    if machine.real_mode{return Ok(Target::Code(machine.load_segment(1,selector,eip,&mut read)?));}
    let code=selector&0xfffc;
    if code==0&&selector&4==0{return Err(gp(eip,0));}
    let at=machine.descriptor_address(selector).ok_or(gp(eip,code))?;
    let raw=read(at)?;
    let d=Descriptor::decode(raw);
    if d.segment(){return Ok(Target::Code(machine.load_segment(1,selector,eip,&mut read)?));}
    let big=match d.access&0x1f{
        0x0c=>true,0x04=>false,
        // Task gates and TSS descriptors switch tasks (increment 7).
        0x05|0x01|0x03|0x09|0x0b=>return Err(ExecError::InvalidContext),
        _=>return Err(gp(eip,code)),
    };
    let cpl=machine.cpl();
    if d.dpl()<cpl||d.dpl()<(selector&3) as u8{return Err(gp(eip,code));}
    if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
    let gate=Gate::decode(raw,big);
    let target=gate.selector&0xfffc;
    if target==0&&gate.selector&4==0{return Err(gp(eip,0));}
    let code_at=machine.descriptor_address(gate.selector).ok_or(gp(eip,target))?;
    let c=Descriptor::decode(read(code_at)?);
    if !c.segment()||!c.code()||c.dpl()>cpl{return Err(gp(eip,target));}
    if !c.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:target});}
    Ok(Target::Gate{gate,code:c,code_at})
}
/// The CS a gate enters at privilege level `level`.
fn gate_cs(gate:&Gate,code:&Descriptor,level:u8)->(u16,SegmentCache){
    let mut cache=code.cache();
    if let SegmentAccess::Code{ref mut dpl,..}=cache.access{*dpl=level;}
    ((gate.selector&!3)|u16::from(level),cache)
}
fn checked_eip(cache:&SegmentCache,offset:u32,eip:u32)->Result<u32,ExecError>{
    if offset>cache.limit{return Err(gp(eip,0));}
    Ok(offset)
}

/// Far JMP to `selector:offset`: a code segment at CPL, or a call gate to
/// a conforming segment or a nonconforming one at CPL (#GP(selector) for a
/// nonconforming one of another level). The offset is checked against the
/// new CS limit (#GP(0)).
pub fn far_jump(machine:&SegmentState,ram:&Ram,selector:u16,offset:u32,eip:u32)->Result<Transfer,ExecError>{
    let mut transfer=Transfer::default();
    let (visible,cache,mark,target)=match far_target(machine,ram,selector,eip)?{
        Target::Code((visible,cache,mark))=>(visible,cache,mark,offset),
        Target::Gate{gate,code,code_at}=>{
            let cpl=machine.cpl();
            if code.access&4==0&&code.dpl()!=cpl{return Err(gp(eip,gate.selector&0xfffc));}
            let (visible,cache)=gate_cs(&gate,&code,cpl);
            (visible,cache,(code.access&1==0).then_some((code_at.wrapping_add(5),code.access|1)),gate.offset)
        }
    };
    transfer.eip=checked_eip(&cache,target,eip)?;
    transfer.accessed(ram,mark,eip)?;
    transfer.segments.push((1,visible,cache));
    Ok(transfer)
}

/// The start of a far CALL. A code segment returns `Ok(None)` with its CS
/// load in `direct`: the PIL description pushes the return address at the
/// operand size and branches. A call gate returns the whole transfer: to a
/// conforming segment or one at CPL, CS and EIP are pushed at the gate's
/// size on the current stack; to a nonconforming segment of a more
/// privileged level, the stack for that level comes from the TSS and gets
/// the old SS and ESP, the gate's parameter count of words or dwords copied
/// from the old stack, then CS and EIP. `next` is the return EIP.
pub fn far_call(machine:&SegmentState,ram:&Ram,regs:&[u32;8],selector:u16,next:u32,eip:u32,direct:&mut Option<(u16,SegmentCache,Option<(u32,u8)>)>)->Result<Option<Transfer>,ExecError>{
    let (gate,code,code_at)=match far_target(machine,ram,selector,eip)?{
        Target::Code(load)=>{*direct=Some(load);return Ok(None);},
        Target::Gate{gate,code,code_at}=>(gate,code,code_at),
    };
    let cpl=machine.cpl();
    let level=if code.access&4!=0{cpl}else{code.dpl()};
    let (visible,cache)=gate_cs(&gate,&code,level);
    let offset=if gate.big{gate.offset}else{gate.offset&0xffff};
    let mut transfer=Transfer{eip:checked_eip(&cache,offset,eip)?,..Transfer::default()};
    let width=if gate.big{MemWidth::Dword}else{MemWidth::Word};
    let old_cs=u32::from(machine.selectors[1]);
    let old_ss=machine.segments[2];
    let old_esp=regs[4];
    let mut stores=Vec::new();
    let frame=if level<cpl{
        let (ss_selector,ss_cache,ss_mark,esp)=inner_stack(machine,ram,level,0,eip)?;
        let mut frame=Frame::new(ss_cache,esp,level==3,ss_selector&0xfffc,eip);
        frame.push(ram,width,u32::from(machine.selectors[2]),&mut stores)?;
        frame.push(ram,width,old_esp,&mut stores)?;
        // Parameters keep their order: the one at the old ESP ends up just
        // above the return address.
        let mask=stack_mask(&old_ss);
        let bytes=u32::from(width.bytes());
        for i in (0..u32::from(gate.count)).rev(){
            let value=read_stack(ram,old_ss,old_esp.wrapping_add(i*bytes)&mask,width,cpl==3,eip)?;
            frame.push(ram,width,value,&mut stores)?;
        }
        transfer.accessed(ram,ss_mark,eip)?;
        transfer.segments.push((2,ss_selector,ss_cache));
        frame
    }else{Frame::new(old_ss,old_esp,cpl==3,0,eip)};
    let mut frame=frame;
    frame.push(ram,width,old_cs,&mut stores)?;
    frame.push(ram,width,next,&mut stores)?;
    transfer.esp=Some(frame.sp);
    transfer.accessed(ram,(code.access&1==0).then_some((code_at.wrapping_add(5),code.access|1)),eip)?;
    transfer.stores.splice(0..0,stores);
    transfer.segments.insert(0,(1,visible,cache));
    Ok(Some(transfer))
}

/// Far RETF or IRET in protected mode, after the frame's EIP (`offset`)
/// and CS (`selector`) have been read. `outer` is the stack offset just
/// past the popped frame and any RETF imm16 (`cleanup`), wrapped at the
/// stack size; `width` is the operand size of a slot.
///
/// The return CS must be a present code segment with RPL >= CPL; a
/// conforming one needs DPL <= RPL, a nonconforming one DPL = RPL. RPL =
/// CPL returns at the same level (ESP = `outer`). RPL > CPL returns to an
/// outer level: ESP and SS are popped from `outer`, SS must be a present
/// writable data segment with RPL and DPL equal to the new CPL, ESP gets
/// `cleanup` added, and ES, DS, FS and GS are nulled when they hold a data
/// or nonconforming code segment more privileged than the new CPL. In real
/// mode CS gets its real-mode cache and ESP = `outer`.
pub fn far_return(machine:&SegmentState,ram:&Ram,regs:&[u32;8],selector:u16,offset:u32,outer:u32,width:MemWidth,cleanup:u32,eip:u32)->Result<Transfer,ExecError>{
    let ss=machine.segments[2];
    let mask=stack_mask(&ss);
    let old_esp=regs[4];
    let mut transfer=Transfer::default();
    if machine.real_mode{
        let (visible,cache,_)=machine.load_segment(1,selector,eip,&mut |at|read_descriptor(ram,at,eip))?;
        transfer.eip=checked_eip(&cache,offset,eip)?;
        transfer.segments.push((1,visible,cache));
        transfer.esp=Some((old_esp&!mask)|(outer&mask));
        return Ok(transfer);
    }
    let cpl=machine.cpl();
    let rpl=(selector&3) as u8;
    let code=selector&0xfffc;
    if code==0&&selector&4==0{return Err(gp(eip,0));}
    let at=machine.descriptor_address(selector).ok_or(gp(eip,code))?;
    let d=Descriptor::decode(read_descriptor(ram,at,eip)?);
    let conforming=d.access&4!=0;
    if !d.segment()||!d.code()||rpl<cpl||(conforming&&d.dpl()>rpl)||(!conforming&&d.dpl()!=rpl){return Err(gp(eip,code));}
    if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
    let mut cache=d.cache();
    if let SegmentAccess::Code{ref mut dpl,..}=cache.access{*dpl=rpl;}
    if rpl==cpl{
        transfer.eip=checked_eip(&cache,offset,eip)?;
        transfer.accessed(ram,(d.access&1==0).then_some((at.wrapping_add(5),d.access|1)),eip)?;
        transfer.segments.push((1,selector,cache));
        transfer.esp=Some((old_esp&!mask)|(outer&mask));
        return Ok(transfer);
    }
    // Return to an outer level: ESP, then SS, above the frame.
    let user=cpl==3;
    let bytes=u32::from(width.bytes());
    let popped_esp=read_stack(ram,ss,outer&mask,width,user,eip)?;
    let ss_selector=read_stack(ram,ss,outer.wrapping_add(bytes)&mask,width,user,eip)? as u16;
    let ss_code=ss_selector&0xfffc;
    if ss_code==0&&ss_selector&4==0{return Err(gp(eip,0));}
    let ss_at=machine.descriptor_address(ss_selector).ok_or(gp(eip,ss_code))?;
    let s=Descriptor::decode(read_descriptor(ram,ss_at,eip)?);
    if (ss_selector&3) as u8!=rpl||!s.segment()||s.code()||s.access&2==0||s.dpl()!=rpl{return Err(gp(eip,ss_code));}
    if !s.present(){return Err(ExecError::StackSegment{eip,error_code:ss_code});}
    transfer.eip=checked_eip(&cache,offset,eip)?;
    transfer.accessed(ram,(d.access&1==0).then_some((at.wrapping_add(5),d.access|1)),eip)?;
    transfer.accessed(ram,(s.access&1==0).then_some((ss_at.wrapping_add(5),s.access|1)),eip)?;
    let ss_cache=s.cache();
    let new_esp=popped_esp.wrapping_add(cleanup);
    transfer.esp=Some(if stack_mask(&ss_cache)==u32::MAX{new_esp}else{(old_esp&!0xffff)|(new_esp&0xffff)});
    transfer.segments.push((1,selector,cache));
    transfer.segments.push((2,ss_selector,ss_cache));
    for reg in [0,3,4,5]{
        let stale=match machine.segments[reg].access{
            SegmentAccess::Data{dpl,..}|SegmentAccess::Code{conforming:false,dpl,..}=>dpl<rpl,
            _=>false,
        };
        if stale{transfer.segments.push((reg,0,SegmentCache{base:0,limit:0,access:SegmentAccess::Unusable}));}
    }
    Ok(transfer)
}

