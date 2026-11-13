//! 386 task switches: far JMP or CALL to a TSS descriptor or through a task
//! gate, an interrupt or exception through an IDT task gate, and IRET with
//! NT set (a return to the task in the back link). Shared by the
//! interpreter, the generated Rust (this file is embedded there next to
//! privilege.rs) and interrupt delivery.
//!
//! As in privilege.rs, `switch` only reads and checks: it returns a
//! `Transfer` with every store and the new task's state, which the caller
//! commits once the instruction has passed its checks, so a fault leaves
//! the machine as it was. TSS and descriptor accesses are supervisor-level.
//!
//! What it does, in the 386's order:
//! - the new TSS: a GDT descriptor of an available TSS (busy for a return),
//!   present, with a limit of at least 67h (386) or 2Bh (286);
//! - the old task's EIP, EFLAGS, general registers and selectors are stored
//!   in the current TSS (386 or 286 layout); IRET stores EFLAGS with NT
//!   clear; JMP and IRET clear the old TSS's busy bit;
//! - CALL and interrupts write the old TR selector to the new TSS's back
//!   link and set NT in the new EFLAGS; JMP, CALL and interrupts mark the
//!   new TSS busy; JMP leaves NT as each TSS has it;
//! - the new task's registers, EFLAGS, EIP, LDTR, segment registers and,
//!   from a 386 TSS while paging is on, CR3 are loaded; TR takes the new
//!   TSS and CR0.TS is set. A 286 TSS gives the general registers FFFFh in
//!   their upper halves, zero EFLAGS and EIP upper halves and null FS and
//!   GS, as the 386 does. A 386 TSS with VM in EFLAGS enters
//!   virtual-8086 mode;
//! - an exception with an error code delivered through a task gate pushes
//!   it on the new task's stack, as a dword for a 386 TSS.
//!
//! Not modelled: the 386 checks the new task's LDT and segment registers
//! after the switch has committed, so a fault there is raised in the new
//! task; here every check comes first and such a fault leaves the old task
//! as it was. The new task's descriptors and the error-code push are
//! accessed under the old CR3. The TSS debug trap bit (T) is ignored.
use super::{ExecError,MemWidth};
use super::memory::Ram;
use super::address::{Descriptor,SegmentAccess,SegmentCache,SegmentState,SystemSegment};
use super::privilege::{read_descriptor,Frame,TaskLoad,Transfer};

/// What starts a task switch.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Entry{Jump,Call,Interrupt,Return}

const NT:u32=0x4000;

fn ts(eip:u32,code:u16)->ExecError{ExecError::InvalidTss{eip,error_code:code}}
fn gp(eip:u32,code:u16)->ExecError{ExecError::GeneralProtection{eip,error_code:code}}

/// The TSS descriptor `selector` names, checked for `entry`: in the GDT, a
/// TSS (available for JMP, CALL and interrupts, else #GP; busy for a
/// return, else #TS), present (#NP) and long enough (#TS). Returns the
/// descriptor's address and the descriptor.
fn new_tss(machine:&SegmentState,ram:&Ram,selector:u16,entry:Entry,ext:u16,eip:u32)->Result<(u32,Descriptor),ExecError>{
    let code=(selector&0xfffc)|ext;
    let wrong=|c:u16|if entry==Entry::Return{ts(eip,c)}else{gp(eip,c)};
    if selector&4!=0{return Err(wrong(code));}
    let at=machine.descriptor_address(selector).ok_or(wrong(code))?;
    let d=Descriptor::decode(read_descriptor(ram,at,eip)?);
    let kind=d.access&0x1f;
    let fits=if entry==Entry::Return{kind==0x03||kind==0x0b}else{kind==0x01||kind==0x09};
    if !fits{return Err(wrong(code));}
    if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
    let minimum=if kind&8!=0{0x67}else{0x2b};
    if d.limit<minimum{return Err(ts(eip,code));}
    Ok((at,d))
}

/// The new task's segment register `reg` (SegmentRegister order), checked
/// against the new CPL as the 386 does on a task switch: #TS(selector) for
/// a wrong type or level, #NP (#SS for SS) when not present. `next` holds
/// the new LDTR, so TI=1 selectors read the new task's LDT.
fn task_segment(next:&SegmentState,ram:&Ram,reg:usize,selector:u16,cpl:u8,ext:u16,eip:u32)->Result<(SegmentCache,Option<(u32,u8)>),ExecError>{
    let code=(selector&0xfffc)|ext;
    let rpl=(selector&3) as u8;
    if selector&0xfffc==0&&selector&4==0{
        if matches!(reg,1|2){return Err(ts(eip,ext));}
        return Ok((SegmentCache{base:0,limit:0,access:SegmentAccess::Unusable},None));
    }
    let at=next.descriptor_address(selector).ok_or(ts(eip,code))?;
    let d=Descriptor::decode(read_descriptor(ram,at,eip)?);
    let conforming=d.access&4!=0;
    let readable_or_writable=d.access&2!=0;
    if !d.segment(){return Err(ts(eip,code));}
    match reg{
        1=>{
            if !d.code()||(conforming&&d.dpl()>rpl)||(!conforming&&d.dpl()!=rpl){return Err(ts(eip,code));}
            if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
        }
        2=>{
            if d.code()||!readable_or_writable||rpl!=cpl||d.dpl()!=cpl{return Err(ts(eip,code));}
            if !d.present(){return Err(ExecError::StackSegment{eip,error_code:code});}
        }
        _=>{
            if d.code()&&!readable_or_writable{return Err(ts(eip,code));}
            if (!d.code()||!conforming)&&(d.dpl()<cpl||d.dpl()<rpl){return Err(ts(eip,code));}
            if !d.present(){return Err(ExecError::SegmentNotPresent{eip,error_code:code});}
        }
    }
    let mut cache=d.cache();
    if reg==1{if let SegmentAccess::Code{ref mut dpl,..}=cache.access{*dpl=cpl;}}
    Ok((cache,(d.access&1==0).then_some((at.wrapping_add(5),d.access|1))))
}

/// A task switch to the TSS `selector` (from a TSS descriptor, a task gate
/// or the back link). `regs`, `eflags` and `return_eip` are the state the
/// old task keeps: the next instruction for JMP, CALL, IRET and INT n, the
/// faulting one for an exception. `error_code` is pushed on the new stack
/// (exceptions through a task gate); `ext` is the EXT bit of fault error
/// codes (1 for an exception or external event).
#[allow(clippy::too_many_arguments)]
pub fn switch(machine:&SegmentState,ram:&Ram,regs:&[u32;8],eflags:u32,return_eip:u32,selector:u16,entry:Entry,error_code:Option<u16>,ext:u16,eip:u32)->Result<Transfer,ExecError>{
    let (at,d)=new_tss(machine,ram,selector,entry,ext,eip)?;
    let big=d.access&8!=0;
    let old=machine.ldt_tr[1];
    let old_big=old.kind==0x0b;
    let old_code=(old.selector&0xfffc)|ext;
    if !matches!(old.kind,0x03|0x0b)||old.limit<if old_big{0x5f}else{0x29}{return Err(ts(eip,old_code));}
    let read=|offset:u32,width:MemWidth|ram.as_level(false,|r|r.load(d.base.wrapping_add(offset),width)).map_err(|f|f.error(eip));
    let mut transfer=Transfer::default();
    let mut stores:Vec<(u32,MemWidth,u32,bool)>=Vec::new();
    let stage=|address:u32,width:MemWidth,value:u32,stores:&mut Vec<(u32,MemWidth,u32,bool)>|->Result<(),ExecError>{
        ram.as_level(false,|r|r.check_write(address,width)).map_err(|f|f.error(eip))?;
        stores.push((address,width,value,false));Ok(())
    };
    // The old task's state.
    let saved_flags=if entry==Entry::Return{eflags&!NT}else{eflags};
    if old_big{
        stage(old.base.wrapping_add(0x20),MemWidth::Dword,return_eip,&mut stores)?;
        stage(old.base.wrapping_add(0x24),MemWidth::Dword,saved_flags,&mut stores)?;
        for (i,&value) in regs.iter().enumerate(){stage(old.base.wrapping_add(0x28+4*i as u32),MemWidth::Dword,value,&mut stores)?;}
        for (i,&value) in machine.selectors.iter().enumerate(){stage(old.base.wrapping_add(0x48+4*i as u32),MemWidth::Word,u32::from(value),&mut stores)?;}
    }else{
        stage(old.base.wrapping_add(0x0e),MemWidth::Word,return_eip&0xffff,&mut stores)?;
        stage(old.base.wrapping_add(0x10),MemWidth::Word,saved_flags&0xffff,&mut stores)?;
        for (i,&value) in regs.iter().enumerate(){stage(old.base.wrapping_add(0x12+2*i as u32),MemWidth::Word,value&0xffff,&mut stores)?;}
        for (i,&value) in machine.selectors[..4].iter().enumerate(){stage(old.base.wrapping_add(0x22+2*i as u32),MemWidth::Word,u32::from(value),&mut stores)?;}
    }
    // Busy bits and the back link.
    if matches!(entry,Entry::Jump|Entry::Return){
        let old_at=machine.descriptor_address(old.selector).ok_or(ts(eip,old_code))?;
        let access=(read_descriptor(ram,old_at,eip)?>>40) as u8;
        stage(old_at.wrapping_add(5),MemWidth::Byte,u32::from(access&!2),&mut stores)?;
    }
    if matches!(entry,Entry::Call|Entry::Interrupt){stage(d.base,MemWidth::Word,u32::from(old.selector),&mut stores)?;}
    if entry!=Entry::Return{stage(at.wrapping_add(5),MemWidth::Byte,u32::from(d.access|2),&mut stores)?;}
    // The new task's state.
    let mut new_regs=[0u32;8];
    let mut selectors=[0u16;6];
    let (new_eip,mut new_flags,ldt_selector,cr3);
    if big{
        new_eip=read(0x20,MemWidth::Dword)?;
        new_flags=(read(0x24,MemWidth::Dword)?&0x0002_7fd5)|2;
        for (i,r) in new_regs.iter_mut().enumerate(){*r=read(0x28+4*i as u32,MemWidth::Dword)?;}
        for (i,s) in selectors.iter_mut().enumerate(){*s=read(0x48+4*i as u32,MemWidth::Word)? as u16;}
        ldt_selector=read(0x60,MemWidth::Word)? as u16;
        cr3=machine.paging.then_some(read(0x1c,MemWidth::Dword)?&!0xfff);
    }else{
        new_eip=read(0x0e,MemWidth::Word)?;
        new_flags=(read(0x10,MemWidth::Word)?&0x7fd5)|2;
        for (i,r) in new_regs.iter_mut().enumerate(){*r=0xffff_0000|read(0x12+2*i as u32,MemWidth::Word)?;}
        for (i,s) in selectors[..4].iter_mut().enumerate(){*s=read(0x22+2*i as u32,MemWidth::Word)? as u16;}
        ldt_selector=read(0x2a,MemWidth::Word)? as u16;
        cr3=None;
    }
    if matches!(entry,Entry::Call|Entry::Interrupt){new_flags|=NT;}
    // LDTR: null, or an LDT descriptor in the GDT (#TS otherwise).
    let ldt_code=(ldt_selector&0xfffc)|ext;
    let ldt=if ldt_selector&0xfffc==0{SystemSegment{selector:ldt_selector,..SystemSegment::default()}}else{
        if ldt_selector&4!=0{return Err(ts(eip,ldt_code));}
        let ldt_at=machine.descriptor_address(ldt_selector).ok_or(ts(eip,ldt_code))?;
        let l=Descriptor::decode(read_descriptor(ram,ldt_at,eip)?);
        if l.access&0x1f!=0x02||!l.present(){return Err(ts(eip,ldt_code));}
        SystemSegment{selector:ldt_selector,base:l.base,limit:l.limit,kind:2}
    };
    let tr=SystemSegment{selector,base:d.base,limit:d.limit,kind:(d.access&0x1f)|2};
    let mut next=*machine;
    next.ldt_tr=[ldt,tr];
    let v86=new_flags&0x20000!=0;
    let cpl=if v86{3}else{(selectors[1]&3) as u8};
    let mut caches=[SegmentCache{base:0,limit:0,access:SegmentAccess::Unusable};6];
    if v86{
        for (cache,&s) in caches.iter_mut().zip(&selectors){*cache=SegmentCache::v86(s);}
    }else{
        // CS first: it sets the level the other registers are checked at.
        for reg in [1,2,0,3,4,5]{
            let (cache,mark)=task_segment(&next,ram,reg,selectors[reg],cpl,ext,eip)?;
            caches[reg]=cache;
            if let Some((mark_at,value))=mark{stage(mark_at,MemWidth::Byte,u32::from(value),&mut stores)?;}
        }
    }
    if new_eip>caches[1].limit{return Err(gp(eip,0));}
    if let Some(code)=error_code{
        let width=if big{MemWidth::Dword}else{MemWidth::Word};
        let mut frame=Frame::new(caches[2],new_regs[4],cpl==3,ext,eip);
        frame.push(ram,width,u32::from(code),&mut stores)?;
        new_regs[4]=frame.sp;
    }
    transfer.eip=new_eip;
    transfer.stores=stores;
    for reg in 0..6{transfer.segments.push((reg,selectors[reg],caches[reg]));}
    transfer.task=Some(TaskLoad{regs:new_regs,eflags:new_flags,system:[ldt,tr],cr3});
    Ok(transfer)
}
