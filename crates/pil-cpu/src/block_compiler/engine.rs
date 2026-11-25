//! Run-time blocks over the precompiled instruction forms.
//!
//! Dine plans a block the first time execution reaches an address: it
//! decodes straight-line PIL instructions from there (through the shared
//! decode cache) and stops after any instruction that transfers control,
//! stores, changes the machine context, or writes a flag the DOS loop
//! watches between instructions (IF, TF, IOPL, NT, VM). A block runs only
//! when its entry guard holds: the segment state and fetch window are the
//! ones it was planned for, and its code bytes are unchanged. Each
//! instruction runs its precompiled native form (src/precompiled), or the
//! interpreter when the form table has none; either way the semantics are
//! the instruction's PIL description.
//!
//! What the DOS loop does between instructions stays outside blocks:
//! instructions that the loop answers before decoding (port I/O, INT n, HLT,
//! Dine's interrupt entries and BIOS area) never join a block, a block starts
//! only with no interrupt shadow and TF clear, and the caller bounds each
//! call by the next timer poll, so IRQ, keyboard and mouse delivery happen at
//! the same instruction boundaries as before. 80387 escapes and WAIT run
//! inside blocks and return to the loop immediately when ERROR# is signalled
//! or when an escape traps or waits on BUSY#.
//!
//! The planner knows nothing about which program runs: blocks are keyed by
//! linear address and validated by state and bytes alone.
use crate::{address::SegmentState,guest::{self,Decoded,Image,Operation},cache::DecodeCache,Cpu,ExecError,Ram,Token,Flag};
use std::collections::{BTreeMap,HashMap};
use std::rc::Rc;
use super::sites::MAX_BLOCK_INSTRUCTIONS;

/// Blocks kept before the table is emptied and rebuilt.
pub const CAPACITY:usize=16_384;

enum Step {
    Pil(Rc<Decoded>),
    Fpu(crate::fpu::Instruction),
}

pub struct BlockRun {
    pub steps:usize,
    pub pil_steps:usize,
    pub error_signalled:bool,
    pub result:Result<(),ExecError>,
}

struct Block {
    eip:u32,
    machine:SegmentState,
    window:(u32,u32,u32,[(u32,u32);guest::SYSTEM_RANGES]),
    fpu_present:bool,
    linear:u32,
    bytes:Vec<u8>,
    stamp:Option<crate::memory::CodeStamp>,
    steps:Vec<Step>,
    /// runs[n]: entries that retired exactly n instructions.
    runs:Vec<u64>,
    /// The last instruction leaves segments, control state and the flags
    /// the DOS loop watches as they were, so another block may follow.
    chains:bool,
    // A register-only block may reuse its code guard on a back edge.
    register_only:bool,
    compare_branch:Option<super::compare_branch::CompareBranch>,
    /// Blocks that followed this one in a chain: (EIP, slot, slot version).
    /// A link only saves the table lookup and the segment and window
    /// comparison, which a chain cannot change; the code guard of the next
    /// block is still checked every time.
    links:[Option<(u32,usize,u64)>;2],
}

/// A multiplicative hash for linear addresses (the table is host-only).
#[derive(Default,Clone,Copy)]
struct LinearHash(u64);
impl std::hash::Hasher for LinearHash {
    fn finish(&self)->u64{self.0}
    fn write(&mut self,bytes:&[u8]){for &b in bytes{self.0=(self.0.rotate_left(8)^u64::from(b)).wrapping_mul(0x9e37_79b9_7f4a_7c15);}}
    fn write_u32(&mut self,n:u32){self.0=(u64::from(n)^self.0).wrapping_mul(0x9e37_79b9_7f4a_7c15);}
}
/// Blocks kept for one linear address: the same code entered with
/// different segment states (a routine called with several DS or ES values)
/// keeps a block for each, instead of planning anew on every alternation.
const VARIANTS:usize=4;
/// Slots for one linear address, most recently planned first; NONE is free.
type Variants=[usize;VARIANTS];
const NONE:usize=usize::MAX;
type Map=HashMap<u32,Variants,std::hash::BuildHasherDefault<LinearHash>>;

#[derive(Default)]
pub struct Blocks {
    /// Linear address to slot.
    map:Map,
    slots:Vec<(u64,Block)>,
    /// Retired counts by mnemonic from blocks that were replaced or evicted
    /// before `report`.
    pending:BTreeMap<String,usize>,
    /// Overrides `PIL_PRECOMPILED` for this machine (tests and tools).
    pub mode:Option<crate::precompiled::Mode>,
    /// Raw ROM execution (`--run-rom`): no DOS low-memory hooks or BIOS
    /// interrupt stubs exist, so only video MMIO is excluded.
    pub rom:bool,
    /// Block entries and instructions retired in blocks, for diagnostics.
    pub blocks:u64,
    pub instructions:u64,
    /// Successfully retired native forms, excluding interpreter fallback.
    pub native_instructions:u64,
    /// Successfully retired PIL instructions executed by the interpreter.
    pub interpreted_instructions:u64,
    /// Native forms retired outside blocks.
    pub single_instructions:u64,
    pub compare_branches:u64,
    /// Returns to the DOS loop with at least one instruction retired (a
    /// chain of blocks counts once).
    pub runs:u64,
    next_version:u64,
}
impl Clone for Blocks { fn clone(&self)->Self{Self{rom:self.rom,..Self::default()}} }

fn window(image:&Image)->(u32,u32,u32,[(u32,u32);guest::SYSTEM_RANGES]){(image.fetch_base,image.start,image.end,image.system)}

/// The first opcode byte after prefixes.
fn opcode(bytes:&[u8])->Option<u8>{
    bytes.iter().copied().find(|b|!matches!(b,0x66|0x67|0x26|0x2e|0x36|0x3e|0x64|0x65|0xf0|0xf2|0xf3))
}
/// Instructions the DOS loop handles before the PIL decoder sees them.
pub fn answered_by_loop(bytes:&[u8])->bool{
    matches!(opcode(bytes),None|Some(0xe4..=0xe7|0xec..=0xef|0x6c..=0x6f|0xd8..=0xdf|0x9b|0xcc..=0xce|0xf4))
}
/// Instructions that never enter a run-time block (port I/O, INT n, HLT).
fn non_block_opcode(bytes:&[u8])->bool{
    matches!(opcode(bytes),None|Some(0xe4..=0xe7|0xec..=0xef|0x6c..=0x6f|0xcc..=0xce|0xf4))
}
fn fpu_writes_memory(ins:&crate::fpu::Instruction)->bool{
    use crate::fpu::Operation as Op;
    ins.modrm.is_some_and(|m|m>>6!=3&&matches!(crate::fpu::classify(ins.opcode,m),Ok(
        Op::Fnstcw|Op::FnstswM16|Op::Fnstenv|Op::Fnsave|Op::FstpM32|Op::FstpM64|
        Op::FstpM80|Op::FistpM16|Op::FistpM32|Op::FistpM64|Op::FbstpM80)))
}
/// Code addresses blocks never cover. In real and virtual-8086 mode,
/// linear: Dine's own low memory (vector table, BIOS data and the DOS data
/// area with its break-handler stubs, whose return points the DOS loop
/// watches), video memory, the ROM/BIOS area with Dine's interrupt entries,
/// and anything above 1 MiB. Otherwise, bus addresses (after paging): video
/// memory and the ROM/BIOS area; Dine watches no return points in 16- or
/// 32-bit protected-mode code. In raw ROM mode (`rom`), Dine installs no
/// stubs or low-memory hooks, so only video MMIO (`A0000h..BFFFFh`) is
/// excluded.
fn excluded(image:&Image,ram:&Ram,linear:u32,rom:bool)->bool{
    if rom{return ram.bus_address(linear,false).map_or(true,|bus|(0xa0000..0xc0000).contains(&bus));}
    if image.real_mode||image.segment_state().v86(){return !(0x1000..0xa0000).contains(&linear);}
    ram.bus_address(linear,false).map_or(true,|bus|(0xa0000..0x100000).contains(&bus))
}

/// Does `p` end a block after it runs?
fn boundary(p:&crate::Program)->bool{
    p.uses_flow||p.tokens.iter().any(|t|match t{
        Token::Store(..)|Token::StoreIf(..)|Token::StoreSegment(..)|Token::WriteSegment(..)|Token::WriteControl(..)|
        Token::WriteTable(..)|Token::LoadSystem(..)|Token::FarBranch(..)|Token::FarJump(..)|Token::FarCall(..)|
        Token::FarReturn(..)|Token::TaskReturnIf(..)|Token::InterruptShadowIf(..)|Token::CheckIo(..)|Token::CheckpointFlags=>true,
        Token::Flag(f,_)|Token::FlagIf(f,..)=>matches!(f,Flag::TF|Flag::IF|Flag::IOPL0|Flag::IOPL1|Flag::NT|Flag::VM),
        _=>false,
    })
}

/// Does `p` change what the DOS loop or a block's entry guard depends on:
/// segments, descriptor tables, control registers, the TSS, the interrupt
/// shadow, a flag checkpoint, I/O permission, or IF, TF, IOPL, NT, VM?
/// A near jump, CALL, RET or store does not.
fn context(p:&crate::Program)->bool{
    p.partial||p.tokens.iter().any(|t|match t{
        Token::WriteSegment(..)|Token::WriteControl(..)|Token::WriteTable(..)|Token::LoadSystem(..)|Token::FarBranch(..)|
        Token::FarJump(..)|Token::FarCall(..)|Token::FarReturn(..)|Token::TaskReturnIf(..)|Token::InterruptShadowIf(..)|
        Token::CheckIo(..)|Token::CheckpointFlags|Token::ResumeIf(..)=>true,
        Token::Flag(f,_)|Token::FlagIf(f,..)=>matches!(f,Flag::TF|Flag::IF|Flag::IOPL0|Flag::IOPL1|Flag::NT|Flag::VM),
        _=>false,
    })
}

impl Blocks {
    pub fn clear(&mut self){
        for (_,b) in self.slots.drain(..){count(&b,&mut self.pending);}
        self.map.clear();
    }

    fn plan(cache:&mut DecodeCache,image:&Image,ram:&Ram,eip:u32,rom:bool,fpu_present:bool,required:bool)->Option<Block>{
        let base=image.segments[1].base;
        let linear=base.checked_add(eip)?;
        // An IP past FFFFh wraps in a 16-bit code segment: a block ends there.
        let default_32=image.segments[1].default_32();
        let wrap=if default_32{u32::MAX}else{0xffff};
        let mut steps=Vec::new();let mut bytes=Vec::new();let mut ip=eip;
        while steps.len()<MAX_BLOCK_INSTRUCTIONS {
            let at=base.checked_add(ip)?;
            if excluded(image,ram,at,rom){break;}
            let numerics=crate::fpu::decode_with_default(|offset|{
                let sig=image.fetch_signature(ram,ip,offset+1)?;
                Ok::<u8,guest::GuestError>(sig[offset as usize])
            },default_32);
            if let Ok(Some(ins))=numerics{
                if image.paging&&(at>>12!=linear>>12||at.checked_add(ins.length-1).is_none_or(|end|end>>12!=linear>>12)){break;}
                let last=fpu_writes_memory(&ins);
                let Some(next)=ip.checked_add(ins.length)else{break};
                bytes.extend_from_slice(&ins.bytes[..ins.length as usize]);
                steps.push(Step::Fpu(ins));
                if last||next<=ip||next>wrap{break;}
                ip=next;
                continue;
            }
            let Ok(decoded)=cache.fetch(image,ram,ip)else{break};
            // With paging a block stays in one page, so one translation
            // covers its code (see Ram::watch_code).
            if image.paging&&(at>>12!=linear>>12||at.checked_add(decoded.length-1).is_none_or(|end|end>>12!=linear>>12)){break;}
            let Ok(signature)=image.fetch_signature(ram,ip,decoded.length)else{break};
            let code=&signature[..decoded.length as usize];
            if non_block_opcode(code){break;}
            let Operation::Pil{program,flow,..}=&decoded.operation else{break};
            if required && decoded.native().is_none(){break;}
            if program.partial{break;}
            let last=boundary(program);let next=flow.next_eip;
            bytes.extend_from_slice(code);steps.push(Step::Pil(decoded.clone()));
            if last||next<=ip||next>wrap{break;}
            ip=next;
        }
        if steps.is_empty(){return None;}
        let stamp=ram.watch_code(linear,bytes.len());
        let chains=steps.last().is_some_and(|s|match s{
            Step::Pil(d)=>matches!(&d.operation,Operation::Pil{program,..} if !context(program)),
            Step::Fpu(_)=>true,
        });
        let register_only = chains && steps.iter().all(|step| matches!(
            step, Step::Pil(d) if matches!(&d.operation, Operation::Pil { program, address: None, .. } if !program.uses_memory)
        ));
        let compare_branch = match steps.as_slice() {
            [.., Step::Pil(a), Step::Pil(b)] => super::compare_branch::CompareBranch::suffix(&[a.clone(), b.clone()]),
            _ => None,
        };
        Some(Block{compare_branch,register_only,eip,machine:image.segment_state(),window:window(image),fpu_present,linear,bytes,stamp,runs:vec![0;steps.len()+1],steps,chains,links:[None;2]})
    }

    /// Run blocks from `cpu.eip`, at most `budget` instructions in all.
    /// Nothing inside a block can make a timer, keyboard, mouse or break
    /// event due (see `boundary`), and the caller bounds `budget` by the
    /// next timer poll, so the DOS loop sees events at the same instruction
    /// boundaries as the interpreter.
    ///
    /// With `chain`, a block that ended on a near transfer of control (a
    /// jump, CALL, RET, LOOP) or a store goes straight on to the block at the
    /// new CS:IP, with that block's full entry guard, instead of returning to
    /// the DOS loop first. The caller sets `chain` only when the loop has no
    /// pending keyboard or mouse event to look at between instructions; a
    /// block that changed segments, control registers, IF, TF, IOPL, NT, VM
    /// or an interrupt shadow, or that faulted, always returns to the loop.
    /// None: no block applies here; the caller interprets.
    pub fn run(&mut self,cache:&mut DecodeCache,image:&Image,cpu:&mut Cpu,ram:&mut Ram,fpu:&mut crate::fpu::Fpu,machine:&mut SegmentState,budget:usize,chain:bool)->Option<BlockRun>{
        let mut total=0usize;let mut pil_total=0usize;let mut from=None;
        let mut flags = crate::LazyFlags::None;
        // Code guards translate with the current CR3 and privilege level.
        ram.set_paging(machine.ram_paging());
        loop {
            let Some((slot,n,pil_n,error_signalled,result,more))=self.enter(cache,image,cpu,ram,fpu,machine,budget-total,from,chain,&mut flags)else{
                if total>0{self.runs+=1;}
                flags.flush(&mut cpu.eflags, &mut cpu.undefined_flags);
                return (total>0).then_some(BlockRun{steps:total,pil_steps:pil_total,error_signalled:false,result:Ok(())});
            };
            total+=n;pil_total+=pil_n;
            if error_signalled||result.is_err()||!more||!chain||total==budget{
                if total>0||result.is_err(){self.runs+=1;}
                flags.flush(&mut cpu.eflags, &mut cpu.undefined_flags);
                if total==0&&!error_signalled&&result.is_ok(){return None;}
                return Some(BlockRun{steps:total,pil_steps:pil_total,error_signalled,result});
            }
            from=Some(slot);
        }
    }

    /// The slot of a valid block for CS:IP, planning one if needed.
    /// `from`: the block that just ran to its end in the same chain, so the
    /// segment state and window are the ones it was entered with.
    fn find(&mut self,cache:&mut DecodeCache,image:&Image,ram:&Ram,fpu_present:bool,machine:&SegmentState,eip:u32,from:Option<usize>)->Option<usize>{
        let linear=machine.segments[1].base.checked_add(eip)?;
        let linked=from.and_then(|f|self.slots[f].1.links.iter().flatten().find(|l|l.0==eip).copied())
            .filter(|&(_,slot,version)|self.slots.get(slot).is_some_and(|s|s.0==version&&s.1.fpu_present==fpu_present));
        let slot=match linked{
            Some((_,slot,_))=>Some(slot),
            None=>self.map.get(&linear).and_then(|v|v.iter().copied().filter(|&i|i!=NONE).find(|&i|{let b=&self.slots[i].1;b.eip==eip&&b.fpu_present==fpu_present&&b.machine.same_code_context(machine)&&b.window==window(image)})),
        };
        if let Some(i)=slot{
            let b=&mut self.slots[i].1;
            let same=b.stamp.is_some_and(|s|ram.code_unchanged(s,b.linear,b.bytes.len()))||
                // A write reached the block's lines: compare its bytes, and
                // trust them again from here if they are the same.
                ram.contiguous_slice(b.linear,b.bytes.len()).is_some_and(|s|s==b.bytes)&&{b.stamp=ram.watch_code(b.linear,b.bytes.len());true};
            if same{
                if linked.is_none(){self.link(from,eip,i);}
                return Some(i);
            }
        }
        // Plan anew. A replaced block keeps its slot under a new version,
        // so links to the old one stop matching. The block whose code
        // changed is replaced; otherwise a free variant is used, and with
        // none free the least recently planned one gives way.
        let mut from=from;let mut stale=slot;
        if self.slots.len()>=CAPACITY{self.clear();from=None;stale=None;}
        let planned=Self::plan(cache,image,ram,eip,self.rom,fpu_present,self.mode.unwrap_or_else(crate::precompiled::mode)==crate::precompiled::Mode::Required)?;
        let mut variants=self.map.get(&linear).copied().unwrap_or([NONE;VARIANTS]);
        let reuse=stale.or_else(||variants.iter().copied().all(|v|v!=NONE).then(||variants[VARIANTS-1]));
        let i=match reuse{
            Some(i)=>{let (version,old)=&mut self.slots[i];count(old,&mut self.pending);*old=planned;*version+=1;i},
            None=>{self.slots.push((self.next_version,planned));self.next_version+=1;self.slots.len()-1},
        };
        // Most recently planned first.
        let at=variants.iter().position(|&v|v==i).unwrap_or(VARIANTS-1);
        variants.copy_within(0..at,1);variants[0]=i;
        self.map.insert(linear,variants);
        self.link(from,eip,i);
        Some(i)
    }
    fn link(&mut self,from:Option<usize>,eip:u32,to:usize){
        let Some(f)=from else{return};
        let version=self.slots[to].0;
        let links=&mut self.slots[f].1.links;
        if links[0].is_some_and(|l|l.0!=eip){links[1]=links[0];}
        links[0]=Some((eip,to,version));
    }

    /// One block: its slot, retired instructions, retired PIL instructions,
    /// whether ERROR# was signalled, the PIL result, and whether the block
    /// ran to its end on an instruction after which another block may follow.
    fn enter(&mut self,cache:&mut DecodeCache,image:&Image,cpu:&mut Cpu,ram:&mut Ram,fpu:&mut crate::fpu::Fpu,machine:&mut SegmentState,budget:usize,from:Option<usize>,chain:bool,flags:&mut crate::LazyFlags)->Option<(usize,usize,usize,bool,Result<(),ExecError>,bool)>{
        if budget==0||cpu.eflags&0x100!=0||machine.ss_shadow!=0||machine.sti_shadow!=0{return None;}
        let slot=self.find(cache,image,ram,fpu.available(),machine,cpu.eip,from)?;
        let block=&mut self.slots[slot].1;
        let mut total = 0;
        let mut pil_total = 0;
        let wrap = if machine.segments[1].default_32() { u32::MAX } else { 0xffff };
        loop {
            let mut n=0usize;let mut pil_n=0usize;let mut failure=None;let mut error_signalled=false;let mut fpu_stop=false;
            let available = (budget - total).min(block.steps.len());
            let pair = block.compare_branch.as_ref().filter(|_| available == block.steps.len());
            let prefix = available - if pair.is_some() { 2 } else { 0 };
            if prefix != 0 {
                flags.flush(&mut cpu.eflags, &mut cpu.undefined_flags);
            }
            for step in &block.steps[..prefix]{
                match step {
                    Step::Pil(decoded)=>{
                        let Operation::Pil{program,dst,src,flow,address}=&decoded.operation else{unreachable!("blocks hold PIL operations")};
                        let native = decoded.native();
                        let result=match native{
                            Some(native)=>(native.form)(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine,&native.consts),
                            None=>{let mut lazy=crate::LazyFlags::None;let r=crate::execute_machine_lazy(program,cpu,*dst,*src,*flow,ram,*address,machine,&mut lazy).map(|_|());lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);r},
                        };
                        if let Err(e)=result{failure=Some(e);break;}
                        if native.is_some(){self.native_instructions+=1;}else{self.interpreted_instructions+=1;}
                        n+=1;pil_n+=1;
                    }
                    Step::Fpu(ins)=>{
                        ram.set_paging(machine.ram_paging());
                        match fpu.execute(ins,cpu,ram,image){
                            crate::fpu::Outcome::Completed=>{
                                cpu.eip=cpu.eip.wrapping_add(ins.length)&wrap;
                                machine.retired();
                                n+=1;
                            }
                            crate::fpu::Outcome::ErrorSignalled=>{
                                cpu.eip=cpu.eip.wrapping_add(ins.length)&wrap;
                                machine.retired();
                                n+=1;
                                error_signalled=true;
                                break;
                            }
                            // Atomic pre-execution conditions (#NM, BUSY#,
                            // memory fault, unsupported): return to the outer
                            // loop so its existing handler runs at cpu.eip.
                            _=>{fpu_stop=true;break;}
                        }
                    }
                }
            }
            if failure.is_none() && !error_signalled && !fpu_stop {
                if let Some(pair) = pair {
                    ram.set_paging(machine.ram_paging());
                    // A pair overwrites all arithmetic flags before its branch.
                    *flags = pair.execute(cpu);
                    self.compare_branches += 1;
                    self.native_instructions += 2;
                    machine.retired();
                    n += 2;
                    pil_n += 2;
                }
            }
            if n > 0 || failure.is_some() {
                block.runs[n] += 1;
                self.blocks += 1;
                self.instructions += n as u64;
            }
            total += n;
            pil_total += pil_n;
            let more = !error_signalled && !fpu_stop && n == block.steps.len() && block.chains;
            if failure.is_some() || error_signalled || fpu_stop || !more || !chain || !block.register_only
                || cpu.eip != block.eip || total == budget {
                return Some((slot, total, pil_total, error_signalled, failure.map_or(Ok(()), Err), more));
            }
        }
    }

    /// Move the per-mnemonic counts of retired block instructions into the
    /// probe's instruction mix, keyed as the interpreter keys them.
    pub fn report(&mut self,mix:&mut BTreeMap<String,usize>){
        for (_,block) in self.slots.iter_mut(){count(block,mix);block.runs.fill(0);}
        for (k,n) in std::mem::take(&mut self.pending){*mix.entry(k).or_insert(0)+=n;}
    }
}
fn count(block:&Block,mix:&mut BTreeMap<String,usize>){
    let fpu_key=if block.fpu_present{"80387 finite arithmetic backend (partial)"}else{"80387 unavailable (no coprocessor installed)"};
    let mut at_least=0u64;
    for n in (1..block.runs.len()).rev(){
        at_least+=block.runs[n];
        if at_least>0{
            let key=match &block.steps[n-1]{
                Step::Pil(d)=>d.label.split_whitespace().next().unwrap_or("?"),
                Step::Fpu(_)=>fpu_key,
            };
            *mix.entry(key.to_string()).or_insert(0)+=at_least as usize;
        }
    }
}
