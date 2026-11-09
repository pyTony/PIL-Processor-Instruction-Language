//! Collect instruction sites without exposing host drives to the guest.
//! Generated instructions are a partial backend; all other work stays in Dine.
use crate::{address,dos,guest,Cpu,ExecError,Program,Ram,View};
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Clone)]
struct Site { signature:Vec<u8>, program:Program, dst:View, src:View, next:u32, flow:crate::FlowContext, address:Option<address::AddressContext>, machine:address::SegmentState, window:(u32,u32,u32,[(u32,u32);guest::SYSTEM_RANGES]) }
struct Recorder<'a> { original:&'a [u8], sites:BTreeMap<u32,Site> }
impl dos::InstructionBackend for Recorder<'_> {
    fn execute(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,decoded:&std::rc::Rc<guest::Decoded>,cpu:&mut Cpu,ram:&mut Ram,_:&mut address::SegmentState)->Option<Result<(),ExecError>> {
        // EXEC children share logical instruction offsets with their parent.
        // Only collect code belonging to the executable this backend targets.
        if active.bytes.as_slice()!=self.original{return None;}
        if self.sites.len()>=65_536||self.sites.contains_key(&cpu.eip){return None;}
        if let guest::Operation::Pil{program,dst,src,flow,address}=&decoded.operation {
            if let Ok(signature)=image.fetch_signature(ram,cpu.eip,decoded.length){
                self.sites.insert(cpu.eip,Site{signature:signature[..decoded.length as usize].to_vec(),program:program.clone(),dst:*dst,src:*src,next:flow.next_eip,flow:*flow,address:*address,machine:image.segment_state(),window:(image.fetch_base,image.start,image.end,image.system)});
            }
        }
        None
    }
}

// Bounded compiled blocks; code and machine identity are checked once at
// entry. A store or control transfer ends the block, so changed code and
// machine contexts are revalidated before the next block executes.
fn block_sequence<'a>(sites:&'a BTreeMap<u32,Site>,start:u32)->Vec<(u32,&'a Site)>{
    let mut seq=Vec::new();let mut ip=start;
    while seq.len()<16 {
        let Some(site)=sites.get(&ip) else{break};
        if site.program.partial{break;}
        if seq.first().is_some_and(|(_,first):&(u32,&Site)|first.machine!=site.machine){break;}
        seq.push((ip,site));
        let store=site.program.tokens.iter().any(|t|matches!(t,crate::Token::Store(..)|crate::Token::StoreIf(..)|crate::Token::StoreSegment(..)));
        if site.program.uses_flow||store||site.next<=ip{break;}
        ip=site.next;
    }
    seq
}
fn emit_blocks(sites:&BTreeMap<u32,Site>)->String {
    let mut out=String::new();
    for &start in sites.keys(){
        let sequence=block_sequence(sites,start);
        if sequence.len()<2{continue;}
        writeln!(out,"#[inline(always)] fn block_{start:x}(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){{").unwrap();
        for (n,(ip,site)) in sequence.iter().enumerate(){
            writeln!(out,"if let Err(e)=i{ip:x}::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,({},{}) ,({},{}) ,{:?},ram,{:?},machine){{return ({n},Err(e));}}",site.dst.register,site.dst.low,site.src.register,site.src.low,site.flow,site.address).unwrap();
        }
        writeln!(out,"({},Ok(()))\n}}",sequence.len()).unwrap();
    }
    out
}
fn emit_block_cases(sites:&BTreeMap<u32,Site>)->String {
    let mut out=String::new();
    for &start in sites.keys(){
        let seq=block_sequence(sites,start);if seq.len()<2{continue;}
        let mut bytes=Vec::new();for (_,site) in &seq{bytes.extend_from_slice(&site.signature);}
        let state=seq[0].1.machine;
        writeln!(out,"{start} if budget>={} && *machine=={:?} && (image.fetch_base,image.start,image.end,image.system)=={:?} && !machine.paging && image.segments[1].base.checked_add({start}).and_then(|at|ram.contiguous_slice(at,{})).is_some_and(|s|s=={:?}) => Some(block_{start:x}(cpu,ram,machine)),",seq.len(),state,seq[0].1.window,bytes.len(),bytes).unwrap();
    }
    out
}

/// Emit a Rust backend module; the caller chooses where it will be built.
pub(crate) fn emit(bytes:&[u8],name:&str)->Result<(String,String),String>{
    if !crate::containers::executable(name){return Err("expected a DOS .COM or .EXE file".into());}
    let mut p=dos::load_with_drives(bytes,name,vec![dos::Source::Empty,dos::Source::Memory(dos::MemDir::default())],None)?;
    let mut recorder=Recorder{original:bytes,sites:BTreeMap::new()};
    let mut output=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    // A single DOS slice is limited to one million steps. Keep the recorder
    // across slices until the program exits, waits for input, or reaches the
    // explicit profiling limit, so longer programs aren't truncated early.
    let mut slices=0;
    let stop=loop {
        let result=dos::run_with_backend(p,dos::MAX_STEPS,&mut output,Some(&mut recorder))?;
        slices+=1;
        if !matches!(result.stop,dos::Stop::Budget)||slices==120 {break result.stop;}
        p=result.loaded;
    };
    if recorder.sites.is_empty(){return Err(format!("no PIL instructions recorded; profiling stop: {stop:?}"));}
    let status=format!("profile stop: {stop:?}; {} instruction sites; {slices} slices",recorder.sites.len());
    let blocks=emit_blocks(&recorder.sites);
    let block_cases=emit_block_cases(&recorder.sites);
    let mut modules=String::new();let mut cases=String::new();
    for (ip,site) in &recorder.sites {
        let Site{signature,program,..}=site;
        writeln!(modules,"mod i{ip:x} {{use crate::{{FlowContext,ExecError,Ram,MemWidth,address,privilege}}; {} }}",crate::emit_rust_functions(&program).replace("fn execute_state(","#[inline(always)] fn execute_state(").replace("pub fn execute_with_machine(","#[inline(always)] pub fn execute_with_machine(")).unwrap();
        let fingerprint=format!("{program:?}");
        writeln!(cases,"{ip} if image.fetch_signature(ram,cpu.eip,{len}).ok().is_some_and(|s|s[..{len}]=={signature:?}) && self.matches(cpu.eip,decoded,||format!(\"{{program:?}}\")=={fingerprint:?}) => i{ip:x}::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),",len=signature.len()).unwrap();
    }
    let source=format!(r#"// Generated from observed PIL sites; unknown or changed instructions use the interpreter.
use crate::{{address,dos,guest,Cpu,Ram,FlowContext}};
use address::*;
use address::SegmentAccess::*;
use address::{{EffectiveAddress::*,Address16::*,SegmentRegister::*}};
const ORIGINAL:&[u8]=&{bytes:?};
{modules}
{blocks}
#[derive(Default)]
pub(crate) struct Backend {{ matched:Option<(std::sync::Arc<Vec<u8>>,bool)>, checks:std::collections::BTreeMap<u32,(std::rc::Rc<guest::Decoded>,bool)>,blocks:u64,block_instructions:u64 }}
impl Backend {{
 // The decode cache returns the same immutable Rc only while its image and
 // instruction bytes remain valid. Retain it so address reuse cannot turn a
 // different decode into a false match; a new decode is checked again.
 fn matches(&mut self,ip:u32,decoded:&std::rc::Rc<guest::Decoded>,check:impl FnOnce()->bool)->bool {{
  if let Some((old,result))=self.checks.get(&ip) {{ if std::rc::Rc::ptr_eq(old,decoded) {{return *result;}} }}
  let result=check();self.checks.insert(ip,(decoded.clone(),result));result
 }}
}}
impl dos::InstructionBackend for Backend {{
 fn block_stats(&self)->(u64,u64){{(self.blocks,self.block_instructions)}}
 fn block(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState,budget:usize)->Option<(usize,Result<(),crate::ExecError>)>{{
  if !self.matched.as_ref().is_some_and(|(bytes,_)|std::sync::Arc::ptr_eq(bytes,&active.bytes)){{self.matched=Some((active.bytes.clone(),active.bytes.as_slice()==ORIGINAL));}}
  if !self.matched.as_ref().unwrap().1{{return None;}}
  let result=match cpu.eip{{
{block_cases}
   _=>None,
  }};
  if let Some((n,_))=&result{{self.blocks+=1;self.block_instructions+=*n as u64;}}
  result
 }}
 fn execute(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,decoded:&std::rc::Rc<guest::Decoded>,cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->Option<Result<(),crate::flow::ExecError>> {{
  if !self.matched.as_ref().is_some_and(|(bytes,_)|std::sync::Arc::ptr_eq(bytes,&active.bytes)){{
   self.matched=Some((active.bytes.clone(),active.bytes.as_slice()==ORIGINAL));
  }}
  if !self.matched.as_ref().unwrap().1{{return None;}}
  let guest::Operation::Pil{{program,dst,src,flow,address}}=&decoded.operation else{{return None}};
  let result=match cpu.eip {{
{cases}
   _=>return None,
  }};
  Some(result)
 }}
}}
#[cfg(test)]
mod guard_tests {{
 use super::*;
 use crate::host_io::Output;
 use crate::dos::InstructionBackend;
 #[test]
 fn changed_code_or_fetch_window_prevents_block_entry() {{
  let mut p=dos::load_with_drives(ORIGINAL,"TEST.COM",vec![dos::Source::Empty,dos::Source::Memory(dos::MemDir::default())],None).unwrap();
  let mut backend=Backend::default();let mut c=p.cpu.clone();let mut r=p.ram.clone();let mut m=p.image.segment_state();
  assert!(backend.block(&p.active_program,&p.image,&mut c,&mut r,&mut m,16).is_some(),"fixture must enter a compiled block");
  let mut narrowed=p.image.clone();narrowed.end=narrowed.start;
  let mut m=p.image.segment_state();
  assert!(backend.block(&p.active_program,&narrowed,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
  let at=p.image.segments[1].base+p.cpu.eip;
  let old=p.ram.load(at,crate::MemWidth::Byte).unwrap();p.ram.store(at,crate::MemWidth::Byte,old^1).unwrap();
  assert!(backend.block(&p.active_program,&p.image,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
 }}
 #[test]
 fn generated_blocks_preserve_state_across_small_budgets() {{
  let mut a=dos::load(ORIGINAL,"TEST.COM").unwrap();let mut b=a.clone();let mut backend=Backend::default();
  for budget in [1,2,3,16,33,127,1000]{{
   let mut x=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
   let mut y=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
   let left=dos::run_with_backend(a,budget,&mut x,None).unwrap();
   let right=dos::run_with_backend(b,budget,&mut y,Some(&mut backend)).unwrap();
   assert_eq!(left.loaded.cpu,right.loaded.cpu,"budget {{budget}}");
   assert_eq!(left.loaded.image,right.loaded.image);
   assert_eq!(left.loaded.ram.bytes(),right.loaded.ram.bytes());
   assert_eq!(left.loaded.ram.planar(),right.loaded.ram.planar());
   assert_eq!(left.stop,right.stop);assert_eq!(left.completed,right.completed);
   assert_eq!(x.bytes(),y.bytes());
   if !matches!(left.stop,dos::Stop::Budget){{break;}}
   a=left.loaded;b=right.loaded;
  }}
 }}
 #[test]
 fn cached_decode_reuses_guard_but_a_new_decode_rechecks_it() {{
  let mut backend=Backend::default();
  let decoded=std::rc::Rc::new(guest::Decoded{{label:String::new(),length:1,operation:guest::Operation::Halt}});
  let calls=std::cell::Cell::new(0);
  assert!(backend.matches(7,&decoded,||{{calls.set(calls.get()+1);true}}));
  assert!(backend.matches(7,&decoded.clone(),||{{calls.set(calls.get()+1);false}}));
  assert_eq!(calls.get(),1);
  let changed=std::rc::Rc::new((*decoded).clone());
  assert!(!backend.matches(7,&changed,||{{calls.set(calls.get()+1);false}}));
  assert_eq!(calls.get(),2);
  assert!(!backend.matches(7,&changed,||panic!("cached failure was not reused")));
 }}
}}
"#);
    Ok((source,status))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emitted_block_ends_at_a_branch() {
        // MOV AX,1; INC AX; JMP over two NOPs; exit via DOS.
        let bytes=[0xb8,1,0,0x40,0xeb,2,0x90,0x90,0xb4,0x4c,0xcd,0x21];
        let (source,_)=emit(&bytes,"TEST.COM").unwrap();
        let block=source.split("fn block_100(").nth(1).unwrap().split("\n}\n").next().unwrap();
        assert!(block.contains("i100::execute_with_machine"));
        assert!(block.contains("i103::execute_with_machine"));
        assert!(block.contains("i104::execute_with_machine"),"branch is the last instruction in the block");
    }
    #[test]
    fn profiles_hello_in_isolation_and_emits_guarded_sites(){
        let (source,status)=emit(&dos::hello("com").unwrap(),"HELLO.COM").unwrap();
        assert!(status.contains("Exit(0)"));
        assert!(status.contains("3 instruction sites"));
        assert!(source.contains("fetch_signature"));
        assert!(source.contains("fn block_100("));
        assert!(source.contains("active.bytes.as_slice()==ORIGINAL"));
        assert!(source.contains("_=>return None"));
        assert!(emit(&[],"README.TXT").is_err());
    }
}
