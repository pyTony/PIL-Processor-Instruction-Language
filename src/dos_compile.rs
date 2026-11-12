//! Collect instruction sites without exposing host drives to the guest.
//! Generated instructions are a partial backend; all other work stays in Dine.
use crate::{address,dos,guest,Cpu,ExecError,Ram};
use crate::dos_codegen::{self,Bindings,BlockAdapter};
use crate::dos_blocks::{Site,InstructionProfile,BlockPlan};
use std::collections::{BTreeMap,BTreeSet};
use std::fmt::Write as _;

struct Recorder<'a> { original:&'a [u8], sites:BTreeMap<u32,Site>, visits:BTreeMap<u32,u64>, entries:BTreeSet<u32>, previous:Option<(u32,bool)> }
impl dos::InstructionBackend for Recorder<'_> {
    fn may_block(&mut self,_:&dos::ProgramIdentity,_:&guest::Image,_:&Ram,_:u32)->bool{false}
    fn execute(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,decoded:&std::rc::Rc<guest::Decoded>,cpu:&mut Cpu,ram:&mut Ram,_:&mut address::SegmentState)->Option<Result<(),ExecError>> {
        // EXEC children share logical instruction offsets with their parent.
        // Only collect code belonging to the executable this backend targets.
        if active.bytes.as_slice()!=self.original{return None;}
        *self.visits.entry(cpu.eip).or_default()+=1;
        if let guest::Operation::Pil{program,dst,src,flow,address}=&decoded.operation {
            if self.previous.is_none_or(|(next,transfer)|transfer||next!=cpu.eip){self.entries.insert(cpu.eip);}
            self.previous=Some((flow.next_eip,program.uses_flow));
            if self.sites.len()>=65_536||self.sites.contains_key(&cpu.eip){return None;}
            if let Ok(signature)=image.fetch_signature(ram,cpu.eip,decoded.length){
                self.sites.insert(cpu.eip,Site{signature:signature[..decoded.length as usize].to_vec(),program:program.clone(),dst:*dst,src:*src,next:flow.next_eip,flow:*flow,address:*address,machine:image.segment_state(),profile:InstructionProfile::of(program,*address)});
            }
        }
        None
    }
}

struct DineBlocks;
impl BlockAdapter for DineBlocks {
    fn header(&self,start:u32)->String{format!("#[inline(never)] fn block_{start:x}(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){{\n")}
    fn instruction(&self,ip:u32,site:&Site,n:usize)->String{
        let call=dos_codegen::instruction_call(ip,site,&Bindings{regs:"&mut cpu.regs",flags:"&mut cpu.eflags",undefined:"&mut cpu.undefined_flags",eip:"&mut cpu.eip",ram:"ram",machine:"machine"},true);
        format!("if let Err(e)={call}{{return ({n},Err(e));}}\n")
    }
    fn footer(&self,length:usize)->String{format!("({length},Ok(()))\n}}\n")}
}
fn emit_block_cases(sites:&BTreeMap<u32,Site>,plan:&BlockPlan)->String {
    let mut out=String::new();
    for &start in sites.keys(){
        let seq=plan.sequence(sites,start);if seq.is_empty(){continue;}
        let mut bytes=Vec::new();for (_,site) in &seq{bytes.extend_from_slice(&site.signature);}
        let state=seq[0].1.machine;
        writeln!(out,"{start} if remaining>={} && compatible(machine,&{:?}) && code_matches(image,ram,{start},&{:?}) => Some(block_{start:x}(cpu,ram,machine)),",seq.len(),state,bytes).unwrap();
    }
    out
}

/// Emit a Rust backend module; the caller chooses where it will be built.
#[cfg(test)]
pub(crate) fn emit(bytes:&[u8],name:&str)->Result<(String,String),String>{
    emit_trained(bytes,name,&[],120_000_000,65_536)
}
/// Training keystrokes stay inside the isolated DOS machine: no host-folder
/// mounts are exposed to the profiled program.
#[cfg(test)]
pub(crate) fn emit_trained(bytes:&[u8],name:&str,keys:&[u8],steps:usize,max_sites:usize)->Result<(String,String),String>{
    emit_trained_with_options(bytes,name,keys,steps,max_sites,None,None)
}
/// Opt-in real-time IRQ0 clock and a caller-provided training directory. The
/// caller must pass a disposable directory: host-mounted DOS files are writable.
pub(crate) fn emit_trained_with_options(bytes:&[u8],name:&str,keys:&[u8],steps:usize,max_sites:usize,clock_offset:Option<i64>,mount:Option<&std::path::Path>)->Result<(String,String),String>{
    if !(1..=120_000_000).contains(&steps)||!(1..=65_536).contains(&max_sites){return Err("invalid profiling budget or site limit".into());}
    if !crate::containers::executable(name){return Err("expected a DOS .COM or .EXE file".into());}
    if clock_offset.is_some_and(|seconds|seconds.abs()>14*3600){return Err("profile clock offset must be -50400..50400 seconds".into());}
    let drives=if let Some(dir)=mount{
        vec![dos::Source::Host(dir.to_path_buf()),dos::Source::Memory(dos::MemDir::default())]
    }else{
        vec![dos::Source::Empty,dos::Source::Memory(dos::MemDir::default())]
    };
    let mut p=dos::load_with_drives(bytes,name,drives,None)?;
    p.clock_offset=clock_offset;
    let entry=p.cpu.eip;
    let mut recorder=Recorder{original:bytes,sites:BTreeMap::new(),visits:BTreeMap::new(),entries:BTreeSet::new(),previous:None};
    let mut output=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    // A single DOS slice is limited to one million steps. Keep the recorder
    // across slices until the program exits, waits for input, or reaches the
    // explicit profiling limit, so longer programs aren't truncated early.
    let mut slices=0;let mut used=0;let mut key_at=0;let mut next_progress=1_000_000;
    let stop=loop {
        // Give interactive startup time to initialize/flush its BIOS input
        // queue before replaying a script (not a workload-completion signal).
        let ready=keys.is_empty()||used>=steps.min(500_000);
        if ready&&key_at<keys.len(){
            let b=keys[key_at];let scan=if b==13{28}else{0};
            if p.enqueue_key(u16::from(b)|(scan<<8))?{key_at+=1;}
        }
        let quantum=if !ready{50_000}else if key_at<keys.len(){10_000}else{dos::MAX_STEPS};
        let budget=quantum.min(steps-used);
        let result=dos::run_with_backend(p,budget,&mut output,Some(&mut recorder))?;
        slices+=1;used+=budget;
        if used>=next_progress||!matches!(result.stop,dos::Stop::Budget|dos::Stop::WaitingForInput){
            eprintln!("DOS profile: budget {used}/{steps}; {} sites; stop {:?}",recorder.sites.len(),result.stop);
            next_progress=used.saturating_add(1_000_000);
        }
        if !matches!(result.stop,dos::Stop::Budget|dos::Stop::WaitingForInput)||used==steps||matches!(result.stop,dos::Stop::WaitingForInput)&&key_at==keys.len(){break result.stop;}
        p=result.loaded;
    };
    if recorder.sites.is_empty(){return Err(format!("no PIL instructions recorded; profiling stop: {stop:?}"));}
    let recorded=recorder.sites.len();
    if recorder.sites.len()>max_sites{
        let plan=BlockPlan::new(&recorder.sites,recorder.entries.iter().copied());
        let mut keep:std::collections::BTreeSet<_>=plan.sequence(&recorder.sites,entry).into_iter().take(max_sites).map(|(ip,_)|ip).collect();
        let mut ranked:Vec<_>=recorder.sites.keys().copied().collect();
        ranked.sort_by_key(|ip|(std::cmp::Reverse(recorder.visits.get(ip).copied().unwrap_or(0)),*ip));
        for ip in ranked{if keep.len()==max_sites{break;}keep.insert(ip);}
        recorder.sites.retain(|ip,_|keep.contains(ip));
    }
    let captured=output.into_bytes();let tail=&captured[captured.len().saturating_sub(512)..];
    let status=format!("profile stop: {stop:?}; {} instruction sites ({recorded} observed); {slices} slices; {key_at}/{} training keys queued; clock={clock_offset:?}; training mount={}; output tail: {:?}",recorder.sites.len(),keys.len(),if mount.is_some(){"yes"}else{"no"},String::from_utf8_lossy(tail));
    let mut counts:BTreeMap<&str,usize>=BTreeMap::new();
    for site in recorder.sites.values(){*counts.entry(site.profile.tag()).or_default()+=1;}
    let status=format!("{status}; instruction profiles: {counts:?}");
    let plan=BlockPlan::new(&recorder.sites,recorder.entries.iter().copied());
    let blocks=dos_codegen::emit_blocks(&recorder.sites,&plan,&DineBlocks);
    let block_cases=emit_block_cases(&recorder.sites,&plan);
    let entries=|block:bool|{
        let cases=recorder.sites.iter().filter(|&(&ip,_)|!block||plan.contains(ip)).map(|(ip,site)|format!("{ip}=>code_matches(image,ram,ip,&{:?}),",site.signature)).collect::<String>();
        format!("match ip {{{cases}_=>false}}")
    };
    let block_entries=entries(true);let instruction_entries=entries(false);
    let mut modules=String::new();let mut cases=String::new();
    for (ip,site) in &recorder.sites {
        let Site{signature,program,..}=site;
        writeln!(modules,"// instruction {ip:x}: {}",site.profile.tag()).unwrap();
        modules.push_str(&dos_codegen::instruction_module(*ip,program));
        let fingerprint=format!("{program:?}");
        writeln!(cases,"{ip} if image.fetch_signature(ram,cpu.eip,{len}).ok().is_some_and(|s|s[..{len}]=={signature:?}) && self.matches(cpu.eip,decoded,||format!(\"{{program:?}}\")=={fingerprint:?}) => ins_{ip:x}::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),",len=signature.len()).unwrap();
    }
    let source=format!(r#"// Generated from observed PIL sites; unknown or changed instructions use the interpreter.
use crate::{{address,dos,guest,Cpu,Ram,FlowContext}};
use address::*;
use address::SegmentAccess::*;
use address::{{EffectiveAddress::*,Address16::*,SegmentRegister::*}};
const ORIGINAL:&[u8]=&{bytes:?};
fn compatible(live:&SegmentState,expected:&SegmentState)->bool{{
 if live.paging{{return false;}}if live==expected{{return true;}}
 if !live.real_mode||!expected.real_mode{{return false;}}
 if !live.segments.iter().zip(live.selectors).all(|(c,s)|*c==SegmentCache::real_mode(s))||!expected.segments.iter().zip(expected.selectors).all(|(c,s)|*c==SegmentCache::real_mode(s)){{return false;}}
 let mut normalized=*live;normalized.selectors=expected.selectors;normalized.segments=expected.segments;normalized==*expected
}}
fn code_matches(image:&guest::Image,ram:&Ram,ip:u32,bytes:&[u8])->bool{{
 let cs=image.segments[1];let Some(at)=cs.base.checked_add(ip)else{{return false;}};
 let last=u64::from(ip)+bytes.len() as u64;
 if last>u64::from(cs.limit)+1{{return false;}}
 let physical=u64::from(ram.physical_address(at));let end=physical+bytes.len() as u64;
 let start=u64::from(image.fetch_base)+u64::from(image.start);let bound=u64::from(image.fetch_base)+u64::from(image.end);
 if !((physical>=start&&end<=bound)||image.system.iter().any(|&(s,e)|physical>=u64::from(s)&&end<=u64::from(e))){{return false;}}
 ram.contiguous_slice(at,bytes.len()).is_some_and(|s|s==bytes)
}}
{modules}
{blocks}
#[derive(Default)]
pub(crate) struct Backend {{ matched:Option<(std::sync::Arc<Vec<u8>>,bool)>, checks:std::collections::BTreeMap<u32,(std::rc::Rc<guest::Decoded>,bool)>,blocks:u64,block_instructions:u64,single_instructions:u64 }}
impl Backend {{
 fn accepts(&mut self,active:&dos::ProgramIdentity)->bool{{
  if !self.matched.as_ref().is_some_and(|(bytes,_)|std::sync::Arc::ptr_eq(bytes,&active.bytes)){{self.matched=Some((active.bytes.clone(),active.bytes.as_slice()==ORIGINAL));}}
  self.matched.as_ref().unwrap().1
 }}
 // The decode cache returns the same immutable Rc only while its image and
 // instruction bytes remain valid. Retain it so address reuse cannot turn a
 // different decode into a false match; a new decode is checked again.
 fn matches(&mut self,ip:u32,decoded:&std::rc::Rc<guest::Decoded>,check:impl FnOnce()->bool)->bool {{
  if let Some((old,result))=self.checks.get(&ip) {{ if std::rc::Rc::ptr_eq(old,decoded) {{return *result;}} }}
  let result=check();self.checks.insert(ip,(decoded.clone(),result));result
 }}
}}
impl dos::InstructionBackend for Backend {{
 fn may_block(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,ram:&Ram,ip:u32)->bool{{self.accepts(active)&&{block_entries}}}
 fn may_execute(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,ram:&Ram,ip:u32)->bool{{self.accepts(active)&&{instruction_entries}}}
 fn block_stats(&self)->(u64,u64){{(self.blocks,self.block_instructions)}}
 fn single_stats(&self)->u64{{self.single_instructions}}
 fn block(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState,budget:usize)->Option<(usize,Result<(),crate::ExecError>)>{{
  if !self.matched.as_ref().is_some_and(|(bytes,_)|std::sync::Arc::ptr_eq(bytes,&active.bytes)){{self.matched=Some((active.bytes.clone(),active.bytes.as_slice()==ORIGINAL));}}
  if !self.matched.as_ref().unwrap().1{{return None;}}
  // Follow observed branches/calls/returns without returning to Dine after
  // each basic block. Every entry retains its full byte and context guard.
  // Cap the region so input and external interrupt checks remain bounded.
  let limit=budget.min(256);let mut completed=0;
  loop{{
  let remaining=limit-completed;
  // Image is immutable during this call. A segment/context change must
  // return to Dine before another byte guard consults Image's cached CS.
  let entry_machine=*machine;
  let result=match cpu.eip{{
{block_cases}
   _=>None,
  }};
  let Some((n,result))=result else{{return if completed==0{{None}}else{{Some((completed,Ok(())))}};}};
  self.blocks+=1;self.block_instructions+=n as u64;completed+=n;
  if result.is_err(){{return Some((completed,result));}}
  if n==0||completed==limit||*machine!=entry_machine||machine.ss_shadow!=0||machine.sti_shadow!=0||machine.paging{{return Some((completed,Ok(())));}}
  }}
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
  self.single_instructions+=1;
  Some(result)
 }}
}}
#[cfg(test)]
mod guard_tests {{
 use super::*;
 use crate::host_io::Output;
 use crate::dos::InstructionBackend;
 #[test]
 fn real_mode_relocation_keeps_guards_but_unreal_and_paging_do_not(){{
  let p=dos::load(ORIGINAL,"TEST.COM").unwrap();let expected=p.image.segment_state();
  let mut moved=expected;moved.selectors=moved.selectors.map(|s|s.wrapping_add(0x1000));moved.segments=moved.selectors.map(SegmentCache::real_mode);
  if expected.real_mode&&expected.segments.iter().zip(expected.selectors).all(|(c,s)|*c==SegmentCache::real_mode(s)){{assert!(compatible(&moved,&expected));}}
  moved.paging=true;assert!(!compatible(&moved,&expected));moved.paging=false;
  moved.segments[1].limit=0x1ffff;assert!(!compatible(&moved,&expected));
 }}
 #[test]
 fn changed_code_or_fetch_window_prevents_block_entry() {{
  let mut p=dos::load_with_drives(ORIGINAL,"TEST.COM",vec![dos::Source::Empty,dos::Source::Memory(dos::MemDir::default())],None).unwrap();
  let mut backend=Backend::default();let mut c=p.cpu.clone();let mut r=p.ram.clone();let mut m=p.image.segment_state();
  assert!(backend.may_block(&p.active_program,&p.image,&p.ram,p.cpu.eip));
  assert!(backend.may_execute(&p.active_program,&p.image,&p.ram,p.cpu.eip));
  assert!(backend.block(&p.active_program,&p.image,&mut c,&mut r,&mut m,16).is_some(),"fixture must enter a compiled block");
  let mut narrowed=p.image.clone();narrowed.end=narrowed.start;
  let mut m=p.image.segment_state();
  assert!(backend.block(&p.active_program,&narrowed,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
  // A stale paged RAM view must not enter the unpaged register fast path.
  p.ram.set_paging(Some(crate::memory::Paging{{directory:0,user:false}}));
  assert!(backend.block(&p.active_program,&p.image,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
  p.ram.set_paging(None);
  let at=p.image.segments[1].base+p.cpu.eip;
  let old=p.ram.load(at,crate::MemWidth::Byte).unwrap();p.ram.store(at,crate::MemWidth::Byte,old^1).unwrap();
  assert!(!backend.may_block(&p.active_program,&p.image,&p.ram,p.cpu.eip));
  assert!(!backend.may_execute(&p.active_program,&p.image,&p.ram,p.cpu.eip));
  assert!(backend.block(&p.active_program,&p.image,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
 }}
 #[test]
 fn generated_blocks_preserve_state_across_small_budgets() {{
  let mut a=dos::load(ORIGINAL,"TEST.COM").unwrap();let mut b=a.clone();let mut backend=Backend::default();
  for budget in [1,2,3,16,33,127,255,256,257,1000].into_iter().cycle().take(160){{
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
    fn observed_indirect_target_splits_a_previously_linear_block(){
        // First pass falls through INC AX to INC DX; later JMP BX enters INC DX.
        // There is no direct relative edge naming that interior target.
        let bytes=[0xb9,2,0,0x40,0x42,0x49,0x74,5,0xbb,4,1,0xff,0xe3,0xb4,0x4c,0xcd,0x21];
        let (source,status)=emit(&bytes,"INDIRECT.COM").unwrap();
        assert!(status.contains("Exit(1)"),"{status}");
        let block=source.split("fn block_100(").nth(1).unwrap().split("\n}\n").next().unwrap();
        assert!(block.contains("ins_100::")&&block.contains("ins_103::"));
        assert!(!block.contains("ins_104::"));
        let target=source.split("fn block_104(").nth(1).unwrap().split("\n}\n").next().unwrap();
        for ip in ["104","105","106"]{assert!(target.contains(&format!("ins_{ip}::")));}
    }
    #[test]
    fn instruction_profiles_select_ram_free_execution_only_when_proven(){
        // Register MOV, memory MOV, segment MOV, then DOS exit.
        let bytes=[0xb8,0,0,0xa1,0,1,0x8e,0xd8,0xb8,0,0x4c,0xcd,0x21];
        let (source,status)=emit(&bytes,"TAGS.COM").unwrap();
        assert!(status.contains("instruction profiles:"));
        assert!(source.contains("instruction 100: register-only"));
        assert!(source.contains("instruction 103: memory"));
        assert!(source.contains("instruction 106: machine-context"));
        let block=source.split("fn block_100(").nth(1).unwrap().split("\n}\n").next().unwrap();
        assert!(block.contains("ins_100::execute_unpaged_register"));
        assert!(block.contains("ins_103::execute_with_machine"));
        assert!(block.contains("ins_106::execute_with_machine"));
        assert!(!block.contains("ins_108::"),"segment writes must terminate the block");
        assert!(source.contains("src,context,None,0,None,Some(machine),None)"));
    }
    #[test]
    fn hexadecimal_f_offset_cannot_form_the_rust_keyword_if(){
        let mut bytes=dos::hello("exe").unwrap();
        bytes[20..22].copy_from_slice(&15u16.to_le_bytes());
        bytes[47..52].copy_from_slice(&[0xb8,0,0x4c,0xcd,0x21]);
        let (source,status)=emit(&bytes,"KEYWORD.EXE").unwrap();
        assert!(status.contains("Exit(0)"));
        assert!(source.contains("mod ins_f {"));
        assert!(source.contains("ins_f::execute_with_machine"));
        assert!(!source.contains("mod if {")&&!source.contains("if::execute"));
    }
    #[test]
    fn training_replays_input_and_limits_generated_sites(){
        let bytes=[0xb4,0,0xcd,0x16,0xb8,0,0x4c,0xcd,0x21];
        let(source,status)=emit_trained(&bytes,"KEY.COM",b"\r",1_000_000,1).unwrap();
        assert!(status.contains("Exit(0)"),"{status}");
        assert!(status.contains("1 instruction sites (2 observed)"),"{status}");
        assert!(status.contains("1/1 training keys queued"));assert!(source.contains("const ORIGINAL"));
    }
    #[test]
    fn emitted_block_ends_at_a_branch() {
        // MOV AX,1; INC AX; JMP over two NOPs; exit via DOS.
        let bytes=[0xb8,1,0,0x40,0xeb,2,0x90,0x90,0xb4,0x4c,0xcd,0x21];
        let (source,_)=emit(&bytes,"TEST.COM").unwrap();
        let block=source.split("fn block_100(").nth(1).unwrap().split("\n}\n").next().unwrap();
        assert!(block.contains("ins_100::execute_unpaged_register"));
        assert!(block.contains("ins_103::execute_unpaged_register"));
        assert!(block.contains("ins_104::execute_unpaged_register"),"branch is the last instruction in the block");
    }
    #[test]
    fn clocked_profile_options_do_not_change_default_or_touch_unmounted_files(){
        let bytes=dos::hello("com").unwrap();
        let (_,normal)=emit_trained(&bytes,"HELLO.COM",&[],1_000,16).unwrap();
        let (_,clocked)=emit_trained_with_options(&bytes,"HELLO.COM",&[],1_000,16,Some(0),None).unwrap();
        assert!(normal.contains("clock=None; training mount=no"));
        assert!(clocked.contains("clock=Some(0); training mount=no"));
        assert!(emit_trained_with_options(&bytes,"HELLO.COM",&[],1_000,16,Some(50_401),None).is_err());
        assert!(emit_trained_with_options(&bytes,"HELLO.COM",&[],1_000,16,None,Some(std::path::Path::new("nonexistent-training-mount"))).is_err());
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
