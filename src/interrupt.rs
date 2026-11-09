//! Architectural interrupt and exception delivery, shared by every runner
//! (the ROM runner, Dine's DOS runner and the SingleStepTests adapter).
//!
//! An instruction that faults commits nothing (PIL is transactional), so a
//! fault is delivered from the state before the instruction, with the
//! instruction's own address as the return address. Software interrupts
//! (INT n, INT3, INTO) return to the next instruction.
//!
//! Real mode: the vector comes from the interrupt table at IDTR (reset value
//! base 0, limit 03FFh); FLAGS, CS and IP are pushed as 16-bit words through
//! SS:SP, then IF and TF are cleared. Error codes are never pushed in real
//! mode. A frame that cannot be pushed, or a vector beyond the IDTR limit, is
//! reported as a shutdown.
//!
//! Protected mode (`deliver`): the IDT entry must be a 286 or 386 interrupt
//! or trap gate to a code segment with DPL <= CPL, entered at its own
//! level unless conforming; EFLAGS, CS, EIP and the error code are pushed
//! as words or dwords by the gate size. A fault while delivering is itself
//! delivered, as a double fault (#DF) where the 386 makes one, and a fault
//! while delivering #DF is a shutdown (triple fault). A handler of a more
//! privileged level runs on its stack from the TSS (privilege.rs). Task
//! gates and virtual-8086 mode stop the run.
use super::address::{MemoryContext,SegmentAccess,SegmentCache,SegmentRegister};
use super::privilege::{self,read_descriptor,Frame};
use super::flow::ExecError;
use super::guest::Image;
use super::memory::{MemWidth,Ram};
use super::Cpu;

/// IDTR after reset: base 0, limit 03FFh (256 four-byte real-mode vectors).
pub const RESET_IDTR:(u16,u32)=(0x03ff,0);

/// The vector of an architectural fault. Model stops (RAM bus faults,
/// unsupported contexts, invalid bindings or control values) have none.
pub fn vector(error:&ExecError)->Option<u8>{
    match error{
        ExecError::Exception{vector,..}=>Some(*vector),
        ExecError::StackSegment{..}=>Some(12),
        ExecError::GeneralProtection{..}=>Some(13),
        ExecError::PageFault{..}=>Some(14),
        ExecError::SegmentNotPresent{..}=>Some(11),
        ExecError::InvalidTss{..}=>Some(10),
        _=>None,
    }
}

/// After a divide error has been delivered: Intel lists CF, PF, AF, ZF, SF
/// and OF as undefined for DIV, IDIV and AAM, and the 386 leaves values of
/// its own there. PIL keeps (and pushes) the entry values and marks them
/// undefined, as it does for other undefined flags.
pub fn after_fault(cpu:&mut Cpu,error:&ExecError){
    if matches!(error,ExecError::Exception{vector:0,..}){cpu.undefined_flags|=0x8d5;}
}

/// An interrupt or exception to deliver in protected mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event{
    pub vector:u8,
    /// Pushed after EIP for #DF, #TS, #NP, #SS, #GP and #PF.
    pub error_code:Option<u16>,
    /// The EIP the frame returns to: the faulting instruction for a fault,
    /// the next one for INT n, INT3 and INTO.
    pub return_eip:u32,
    /// INT n, INT3 or INTO: the gate's DPL is checked, and faults during
    /// delivery have EXT clear.
    pub software:bool,
    /// #PF's linear address, which goes to CR2.
    pub cr2:Option<u32>,
}
impl Event{
    /// INT n, INT3 or INTO returning to `next`.
    pub fn software(vector:u8,next:u32)->Self{Self{vector,error_code:None,return_eip:next,software:true,cr2:None}}
    /// The exception an architectural fault raises, returning to the
    /// faulting instruction at `eip`. Model stops have none.
    pub fn fault(error:&ExecError,eip:u32)->Option<Self>{
        let vector=vector(error)?;
        let (error_code,cr2)=match *error{
            ExecError::GeneralProtection{error_code,..}|ExecError::StackSegment{error_code,..}|ExecError::SegmentNotPresent{error_code,..}|ExecError::InvalidTss{error_code,..}=>(Some(error_code),None),
            ExecError::PageFault{address,error_code,..}=>(Some(error_code),Some(address)),
            _=>(None,None),
        };
        Some(Self{vector,error_code,return_eip:eip,software:false,cr2})
    }
}
/// How the 386 combines a fault raised while delivering another (Intel's
/// double-fault table): benign, contributory (#DE, #TS, #NP, #SS, #GP) or
/// a page fault.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class{Benign,Contributory,Page}
fn class(vector:u8)->Class{match vector{0|10..=13=>Class::Contributory,14=>Class::Page,_=>Class::Benign}}

/// Delivers `event`. In real mode this is the IVT frame; in protected mode
/// the IDT gate, with a fault during delivery handled as the 386 does.
/// Errors are model stops; a "shutdown" message is the triple fault.
pub fn deliver(cpu:&mut Cpu,image:&mut Image,ram:&mut Ram,event:Event)->Result<(),String>{
    if image.real_mode{return deliver_real(cpu,image,ram,event.vector,event.return_eip);}
    if let Some(address)=event.cr2{image.controls[0]=address;}
    let mut current=event;
    loop{
        let error=match protected_frame(cpu,image,ram,&current){Ok(())=>return Ok(()),Err(e)=>e};
        let Some(next)=Event::fault(&error,cpu.eip) else{
            return Err(format!("{error:?} while delivering vector {:02x}h is not modelled",current.vector));
        };
        // CR2 is written when the page fault is recognised, even if it then
        // turns into a double fault.
        if let Some(address)=next.cr2{image.controls[0]=address;}
        if current.vector==8{return Err(format!("shutdown: {error:?} while delivering a double fault (triple fault)"));}
        let double=matches!((class(current.vector),class(next.vector)),(Class::Contributory,Class::Contributory)|(Class::Page,Class::Contributory|Class::Page));
        current=if double{Event{vector:8,error_code:Some(0),cr2:None,..next}}else{Event{cr2:None,..next}};
    }
}

/// One protected-mode delivery attempt. Every read, check and stack write
/// is tried before anything changes, so on error nothing has. A
/// nonconforming handler of a more privileged level runs on the stack for
/// its level from the TSS, with the old SS and ESP pushed first.
///
/// From virtual-8086 mode (CPL 3) INT n, INT3 and INTO need IOPL 3, else
/// #GP(0), and the handler must be a nonconforming ring 0 segment, else
/// #GP(selector). GS, FS, DS and ES are pushed before the old SS, and all
/// four are loaded with null selectors as VM clears.
fn protected_frame(cpu:&mut Cpu,image:&mut Image,ram:&mut Ram,event:&Event)->Result<(),ExecError>{
    let eip=cpu.eip;
    let machine=image.segment_state();
    let v86=machine.v86();
    if v86&&event.software&&(cpu.eflags>>12)&3!=3{return Err(ExecError::GeneralProtection{eip,error_code:0});}
    ram.set_paging(machine.ram_paging());
    let result=(||{
        let r:&Ram=ram;
        let mut read=|at:u32|read_descriptor(r,at,eip);
        let gate=machine.interrupt_gate(event.vector,event.software,eip,&mut read)?;
        let (selector,cache,accessed)=machine.interrupt_target(gate.selector,event.software,eip,&mut read)?;
        let offset=if gate.big{gate.offset}else{gate.offset&0xffff};
        if offset>cache.limit{return Err(ExecError::GeneralProtection{eip,error_code:0});}
        let level=match cache.access{SegmentAccess::Code{dpl,..}=>dpl,_=>return Err(ExecError::InvalidContext)};
        let cpl=machine.cpl();
        let width=if gate.big{MemWidth::Dword}else{MemWidth::Word};
        let mut stores=Vec::new();
        if let Some((at,value))=accessed{
            r.as_level(false,|r|r.check_write(at,MemWidth::Byte)).map_err(|f|f.error(eip))?;
            stores.push((at,MemWidth::Byte,u32::from(value),false));
        }
        let ss_index=SegmentRegister::Ss.index();
        let (mut frame,stack)=if level<cpl{
            let ext=u16::from(!event.software);
            let (ss_selector,ss_cache,ss_mark,esp)=privilege::inner_stack(&machine,r,level,ext,eip)?;
            if let Some((at,value))=ss_mark{
                r.as_level(false,|r|r.check_write(at,MemWidth::Byte)).map_err(|f|f.error(eip))?;
                stores.push((at,MemWidth::Byte,u32::from(value),false));
            }
            let mut frame=Frame::new(ss_cache,esp,level==3,(ss_selector&0xfffc)|ext,eip);
            if v86{
                for reg in [SegmentRegister::Gs,SegmentRegister::Fs,SegmentRegister::Ds,SegmentRegister::Es]{
                    frame.push(r,width,u32::from(machine.selectors[reg.index()]),&mut stores)?;
                }
            }
            frame.push(r,width,u32::from(machine.selectors[ss_index]),&mut stores)?;
            frame.push(r,width,cpu.regs[4],&mut stores)?;
            (frame,Some((ss_selector,ss_cache)))
        }else{(Frame::new(machine.segments[ss_index],cpu.regs[4],cpl==3,0,eip),None)};
        let values=[Some(cpu.eflags),Some(u32::from(machine.selectors[SegmentRegister::Cs.index()])),Some(event.return_eip),event.error_code.map(u32::from)];
        for value in values.into_iter().flatten(){frame.push(r,width,value,&mut stores)?;}
        Ok((gate,selector,cache,offset,stack,frame.sp,stores))
    })();
    let (gate,selector,cache,offset,stack,sp,stores)=match result{Ok(v)=>v,Err(e)=>{ram.set_paging(machine.ram_paging());return Err(e);}};
    for (at,width,value,user) in stores{ram.store_as(user,at,width,value).map_err(|f|f.error(eip))?;}
    cpu.regs[4]=sp;
    // TF, NT, RF and VM clear; an interrupt gate clears IF too.
    let clear=0x0003_4100|if gate.trap{0}else{0x200};
    cpu.eflags&=!clear;cpu.undefined_flags&=!clear;
    if let Some((ss_selector,ss_cache))=stack{
        let index=SegmentRegister::Ss.index();
        image.selectors[index]=ss_selector;image.segments[index]=ss_cache;
    }
    if v86{
        for reg in [SegmentRegister::Es,SegmentRegister::Ds,SegmentRegister::Fs,SegmentRegister::Gs]{
            image.selectors[reg.index()]=0;image.segments[reg.index()]=SegmentCache{base:0,limit:0,access:SegmentAccess::Unusable};
        }
    }
    let index=SegmentRegister::Cs.index();
    image.selectors[index]=selector;image.segments[index]=cache;
    cpu.eip=offset;
    image.retired();
    ram.set_paging(image.segment_state().ram_paging());
    Ok(())
}

/// Short mnemonic for a vector, for reports.
pub fn name(vector:u8)->&'static str{
    match vector{0=>"#DE",1=>"#DB",3=>"#BP",4=>"#OF",5=>"#BR",6=>"#UD",7=>"#NM",8=>"#DF",10=>"#TS",11=>"#NP",12=>"#SS",13=>"#GP",14=>"#PF",_=>"INT"}
}

/// The real-mode handler address (segment, offset) for vector `n`.
pub fn real_vector(image:&Image,ram:&Ram,n:u8)->Result<(u16,u16),String>{
    let(limit,base)=image.tables[1];
    let offset=u32::from(n)*4;
    if offset+3>u32::from(limit){return Err(format!("shutdown: vector {n:02x}h is beyond the IDTR limit {limit:04x}h"));}
    let pointer=ram.load(base.wrapping_add(offset),MemWidth::Dword).map_err(|e|format!("shutdown: vector {n:02x}h read: {e:?}"))?;
    Ok(((pointer>>16)as u16,pointer as u16))
}

/// Pushes the real-mode frame and enters `segment:offset`. Every stack word
/// is translated and checked before anything changes; a word that would
/// straddle the end of the stack segment (SP odd and below 6) is the
/// processor shutdown the 386 performs there.
pub fn real_frame(cpu:&mut Cpu,image:&mut Image,ram:&mut Ram,segment:u16,offset:u16,return_ip:u32)->Result<(),String>{
    if !image.real_mode||cpu.eflags&0x20000!=0{return Err("interrupt delivery outside real mode is not modelled".into());}
    let ss=MemoryContext{segment:SegmentRegister::Ss,cache:image.segments[SegmentRegister::Ss.index()]};
    let sp=cpu.regs[4]&0xffff;
    let cs=image.selectors[SegmentRegister::Cs.index()];
    let words=[(2u32,cpu.eflags&0xffff),(4,u32::from(cs)),(6,return_ip&0xffff)];
    let mut writes=[(0u32,0u32);3];
    for(slot,(down,value))in writes.iter_mut().zip(words){
        let at=ss.translate(sp.wrapping_sub(down)&0xffff,MemWidth::Word,true,cpu.eip)
            .map_err(|_|format!("shutdown: interrupt frame at SS:{:04x} crosses the stack segment limit",sp.wrapping_sub(down)&0xffff))?;
        ram.check_write(at,MemWidth::Word).map_err(|e|format!("interrupt frame write: {e:?}"))?;
        *slot=(at,value);
    }
    for(at,value)in writes{ram.store(at,MemWidth::Word,value).map_err(|e|format!("interrupt frame write: {e:?}"))?;}
    cpu.regs[4]=(cpu.regs[4]&0xffff0000)|(sp.wrapping_sub(6)&0xffff);
    cpu.eflags&=!0x300;cpu.undefined_flags&=!0x300;
    let index=SegmentRegister::Cs.index();
    image.selectors[index]=segment;image.segments[index]=SegmentCache::real_mode(segment);
    cpu.eip=u32::from(offset);
    image.retired();
    Ok(())
}

/// Delivers vector `n` in real mode through the IDTR table.
pub fn deliver_real(cpu:&mut Cpu,image:&mut Image,ram:&mut Ram,n:u8,return_ip:u32)->Result<(),String>{
    let(segment,offset)=real_vector(image,ram,n)?;
    real_frame(cpu,image,ram,segment,offset,return_ip)
}

