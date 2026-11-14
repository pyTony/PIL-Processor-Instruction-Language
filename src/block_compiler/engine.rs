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
//! instructions that the loop answers before decoding (port I/O, 80387
//! escapes, INT n, HLT, Dine's interrupt entries and BIOS area) never join a
//! block, a block starts only with no interrupt shadow and TF clear, and the
//! caller bounds each call by the next timer poll, so IRQ, keyboard and
//! mouse delivery happen at the same instruction boundaries as before.
//!
//! The planner knows nothing about which program runs: blocks are keyed by
//! linear address and validated by state and bytes alone.
use crate::{address::SegmentState,guest::{self,Decoded,Image,Operation},cache::DecodeCache,Cpu,ExecError,Ram,Token,Flag};
use std::collections::{BTreeMap,HashMap};
use std::rc::Rc;
use super::sites::MAX_BLOCK_INSTRUCTIONS;

/// Blocks kept before the table is emptied and rebuilt.
pub(crate) const CAPACITY:usize=16_384;

struct Block {
    eip:u32,
    machine:SegmentState,
    window:(u32,u32,u32,[(u32,u32);guest::SYSTEM_RANGES]),
    linear:u32,
    bytes:Vec<u8>,
    stamp:Option<crate::memory::CodeStamp>,
    steps:Vec<Rc<Decoded>>,
    /// runs[n]: entries that retired exactly n instructions.
    runs:Vec<u64>,
}

#[derive(Default)]
pub(crate) struct Blocks {
    map:HashMap<u32,Block>,
    /// Retired counts by mnemonic from blocks that were replaced or evicted
    /// before `report`.
    pending:BTreeMap<String,usize>,
    /// Overrides `PIL_PRECOMPILED` for this machine (tests and tools).
    pub(crate) mode:Option<crate::precompiled::Mode>,
    /// Block entries and instructions retired in blocks, for diagnostics.
    pub(crate) blocks:u64,
    pub(crate) instructions:u64,
}
impl Clone for Blocks { fn clone(&self)->Self{Self::default()} }

fn window(image:&Image)->(u32,u32,u32,[(u32,u32);guest::SYSTEM_RANGES]){(image.fetch_base,image.start,image.end,image.system)}

/// The first opcode byte after prefixes.
fn opcode(bytes:&[u8])->Option<u8>{
    bytes.iter().copied().find(|b|!matches!(b,0x66|0x67|0x26|0x2e|0x36|0x3e|0x64|0x65|0xf0|0xf2|0xf3))
}
/// Instructions the DOS loop handles before the PIL decoder sees them.
fn answered_by_loop(bytes:&[u8])->bool{
    matches!(opcode(bytes),None|Some(0xe4..=0xe7|0xec..=0xef|0x6c..=0x6f|0xd8..=0xdf|0x9b|0xcc..=0xce|0xf4))
}
/// Linear code addresses blocks never cover: Dine's own low memory (vector
/// table, BIOS data and the DOS data area with its break-handler stubs,
/// whose return points the DOS loop watches), video memory, the ROM/BIOS
/// area with Dine's interrupt entries, and anything above 1 MiB.
fn excluded(linear:u32)->bool{ !(0x1000..0xa0000).contains(&linear) }

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

impl Blocks {
    pub(crate) fn clear(&mut self){self.map.clear();}

    fn plan(cache:&mut DecodeCache,image:&Image,ram:&Ram,eip:u32)->Option<Block>{
        if image.paging||!image.real_mode{return None;}
        let base=image.segments[1].base;
        let linear=base.checked_add(eip)?;
        let mut steps=Vec::new();let mut bytes=Vec::new();let mut ip=eip;
        while steps.len()<MAX_BLOCK_INSTRUCTIONS {
            let at=base.checked_add(ip)?;
            if excluded(at){break;}
            let Ok(decoded)=cache.fetch(image,ram,ip)else{break};
            let Ok(signature)=image.fetch_signature(ram,ip,decoded.length)else{break};
            let code=&signature[..decoded.length as usize];
            if answered_by_loop(code){break;}
            let Operation::Pil{program,flow,..}=&decoded.operation else{break};
            if program.partial{break;}
            let last=boundary(program);let next=flow.next_eip;
            bytes.extend_from_slice(code);steps.push(decoded.clone());
            if last||next<=ip||next>0xffff{break;}
            ip=next;
        }
        if steps.is_empty(){return None;}
        let stamp=ram.watch_code(linear,bytes.len());
        Some(Block{eip,machine:image.segment_state(),window:window(image),linear,bytes,stamp,runs:vec![0;steps.len()+1],steps})
    }

    /// Run the block at `cpu.eip`, at most `budget` instructions of it.
    /// One block per call: the DOS loop checks timer, keyboard, mouse and
    /// break events between blocks, as it does between instructions, and
    /// nothing inside a block can make one of them due (see `boundary`).
    /// None: no block applies here; the caller interprets.
    pub(crate) fn run(&mut self,cache:&mut DecodeCache,image:&Image,cpu:&mut Cpu,ram:&mut Ram,machine:&mut SegmentState,budget:usize)->Option<(usize,Result<(),ExecError>)>{
        if budget==0||cpu.eflags&0x100!=0||machine.ss_shadow!=0||machine.sti_shadow!=0{return None;}
        let linear=machine.segments[1].base.checked_add(cpu.eip)?;
        let valid=self.map.get(&linear).is_some_and(|b|b.eip==cpu.eip&&b.machine==*machine&&b.window==window(image)&&
            (b.stamp.is_some_and(|s|ram.code_unchanged(s,b.linear,b.bytes.len()))||ram.contiguous_slice(b.linear,b.bytes.len()).is_some_and(|s|s==b.bytes)));
        if !valid{
            if self.map.len()>=CAPACITY{for (_,b) in self.map.drain(){count(&b,&mut self.pending);}}
            let planned=Self::plan(cache,image,ram,cpu.eip);
            if let Some(old)=self.map.remove(&linear){count(&old,&mut self.pending);}
            self.map.insert(linear,planned?);
        }
        let block=self.map.get_mut(&linear).expect("planned block");
        let mut n=0usize;let mut failure=None;
        for decoded in block.steps.iter().take(budget){
            let Operation::Pil{program,dst,src,flow,address}=&decoded.operation else{unreachable!("blocks hold PIL operations")};
            let result=match decoded.native(){
                Some(native)=>(native.form)(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine,&native.consts),
                None=>{let mut lazy=crate::LazyFlags::None;let r=crate::execute_machine_lazy(program,cpu,*dst,*src,*flow,ram,*address,machine,&mut lazy).map(|_|());lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);r},
            };
            if let Err(e)=result{failure=Some(e);break;}
            n+=1;
        }
        block.runs[n]+=1;self.blocks+=1;self.instructions+=n as u64;
        Some((n,failure.map_or(Ok(()),Err)))
    }

    /// Move the per-mnemonic counts of retired block instructions into the
    /// probe's instruction mix, keyed as the interpreter keys them.
    pub(crate) fn report(&mut self,mix:&mut BTreeMap<String,usize>){
        for block in self.map.values_mut(){count(block,mix);block.runs.fill(0);}
        for (k,n) in std::mem::take(&mut self.pending){*mix.entry(k).or_insert(0)+=n;}
    }
}
fn count(block:&Block,mix:&mut BTreeMap<String,usize>){
    let mut at_least=0u64;
    for n in (1..block.runs.len()).rev(){
        at_least+=block.runs[n];
        if at_least>0{
            let key=block.steps[n-1].label.split_whitespace().next().unwrap_or("?");
            *mix.entry(key.to_string()).or_insert(0)+=at_least as usize;
        }
    }
}
