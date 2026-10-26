//! Offline Rust-source backend proof for fixed guest images, not a debugger JIT.
use std::{fs,path::{Path,PathBuf},process::Command,time::{Instant,SystemTime,UNIX_EPOCH}};
use super::guest::{self,Image,Operation};
use super::address::SegmentRegister;

pub struct Case {
    pub name:&'static str,pub bytes:Vec<u8>,pub image:Image,pub regs:[u32;8],pub flags:u32,
    pub rx:Vec<u8>,pub tx:Vec<u8>,pub protect:Option<(u32,usize)>,pub patch:Option<(u32,Vec<u8>)>,
}
fn case(name:&'static str,bytes:Vec<u8>)->Result<Case,String> {
    let (image,_)=Image::load(&bytes)?;
    Ok(Case{name,bytes,image,regs:[0;8],flags:0x202,rx:Vec::new(),tx:Vec::new(),protect:None,patch:None})
}
pub fn cases()->Result<Vec<Case>,String> {
    let mut tests=vec![case("selftest",guest::selftest_bytes()?)?,case("hello",guest::hello_bytes()?)?,case("factorials",guest::factorial_bytes()?)?,case("loop17",guest::loop_bytes(17)?)?];
    tests[0].rx.push(65);
    let mut memory=case("readonly_store",vec![0x66,0x89,0x07,0xf4])?;memory.regs[0]=0xdeadbeef;memory.regs[3]=0x400;memory.protect=Some((0x500,4));tests.push(memory);
    let mut branch=case("branch_fault",vec![0x74,0x7f,0xf4])?;branch.flags=0x246;branch.image.segments[SegmentRegister::Cs.index()].limit=0x102;tests.push(branch);
    let mut changed=case("code_guard",guest::hello_bytes()?)?;changed.patch=Some((0x100,vec![0x90]));tests.push(changed);
    tests.push(case("rx_wait",vec![0xe4,0xe9,0xf4])?);
    let mut blocked=case("tx_blocked",vec![0xe6,0xe9,0xf4])?;blocked.regs[0]=65;blocked.tx=vec![42;256];tests.push(blocked);
    Ok(tests)
}
fn validate(iterations:u32,samples:u32)->Result<(),String> {
    guest::loop_bytes(iterations)?;
    if !(1..=9).contains(&samples){return Err("comparison samples must be 1..9".into());}Ok(())
}
fn emit_case(case:&Case,name:&str)->Result<String,String> {
    let (_,ram)=Image::load(&case.bytes)?;let mut ip=guest::ENTRY;
    let mut code=String::new();let mut dispatch=String::new();
    while ip<case.image.end {
        let decoded=case.image.decode(&ram,ip).map_err(|e|format!("{} decode: {e:?}",case.name))?;
        let signature=case.image.fetch_signature(&ram,ip,decoded.length).map_err(|e|format!("{e:?}"))?;
        let bytes=&signature[..decoded.length as usize];
        dispatch.push_str(&format!("{ip}=>{{if let Err(status)=guard(s,{ip},&{bytes:?},CS,{}){{return status;}}\n",case.image.end));
        match decoded.operation {
            Operation::Pil{program,dst,src,flow,address}=>{
                code.push_str(&format!("mod i{ip:x} {{use super::{{FlowContext,ExecError,Ram,MemWidth,address}}; {} }}\n",super::emit_rust_functions(&program)));
                let args=format!("&mut s.regs,&mut s.flags,&mut s.undefined,&mut s.eip,({},{}),({},{}),{flow:?}",dst.register,dst.low,src.register,src.low);
                let call=if let Some(a)=address {format!("i{ip:x}::execute_with_segments({args},&mut s.ram,{a:?})")}else{format!("i{ip:x}::execute_with_flow({args})")};
                dispatch.push_str(&format!("match {call} {{Ok(())=>[0,0,0,0,0],Err(e)=>trace::fault(s.eip,e)}}"));
            }
            Operation::Input{next}=>dispatch.push_str(&format!("if let Some(b)=s.rx.pop_front(){{s.regs[0]=(s.regs[0]&!255)|u32::from(b);s.eip={next};[0,0,0,0,0]}}else{{[2,s.eip,0,0,0]}}")),
            Operation::Output{next}=>dispatch.push_str(&format!("if s.tx.len()==256{{[3,s.eip,0,0,0]}}else{{s.tx.push_back(s.regs[0] as u8);s.eip={next};[0,0,0,0,0]}}")),
            Operation::Halt=>dispatch.push_str("[1,s.eip,0,0,0]"),
        }
        dispatch.push_str("},\n");ip+=decoded.length;
    }
    let mut init=format!("let mut ram=Ram::new({});ram.write_bytes(0x100,&{:?}).unwrap();\n",guest::RAM_BYTES,case.bytes);
    if let Some((at,len))=case.protect {init.push_str(&format!("ram.protect({at},{len}).unwrap();\n"));}
    if let Some((at,bytes))=&case.patch {init.push_str(&format!("ram.write_bytes({at},&{bytes:?}).unwrap();\n"));}
    init.push_str(&format!("State{{regs:{:?},flags:{},undefined:0,eip:0x100,ram,rx:VecDeque::from({:?}),tx:VecDeque::from({:?})}}",case.regs,case.flags,case.rx,case.tx));
    Ok(format!("mod {name} {{use super::*; const CS:SegmentCache={:?};\n{code}\npub fn initial()->State{{{init}}}\n#[inline(never)] pub fn step(s:&mut State)->[u32;5]{{match s.eip{{{dispatch}_=>[7,s.eip,0,0,0]}}}}\n}}\n",case.image.segments[SegmentRegister::Cs.index()]))
}
pub fn emit_suite(iterations:u32,samples:u32)->Result<String,String> {
    validate(iterations,samples)?;let tests=cases()?;
    let mut source=String::from("// Generated from validated PIL; fixed-image native Rust proof, not a JIT.\n#![allow(dead_code,unused_variables,unused_mut,unused_imports)]\nuse std::collections::VecDeque;\n");
    source.push_str("mod flow {\n");source.push_str(include_str!("flow.rs"));source.push_str("\n}\nuse flow::{ExecError,FlowContext};\n");
    source.push_str("mod memory {\n");source.push_str(include_str!("memory.rs").split("#[cfg(test)]").next().unwrap());source.push_str("\n}\nuse memory::{Ram,MemWidth};\n");
    source.push_str("mod address {\n");source.push_str(include_str!("address.rs").split("#[cfg(test)]").next().unwrap());source.push_str("\n}\nuse address::*;\nuse address::{EffectiveAddress::*,Address16::*,Gpr::*,SegmentRegister::*,SegmentAccess::*};\n");
    source.push_str("mod trace {\n");source.push_str(include_str!("trace.rs"));source.push_str("\n}\n");
    source.push_str(RUNNER);
    for (id,test) in tests.iter().enumerate(){source.push_str(&emit_case(test,&format!("c{id}"))?);}
    source.push_str(&emit_case(&case("timed_loop",guest::loop_bytes(iterations)?)?,"timed_loop")?);
    source.push_str("fn main(){let mut args=std::env::args().skip(1);let path=args.next().expect(\"trace output path required\");let mode=args.next().unwrap_or_else(||\"all\".into());if mode!=\"bench\"{let mut trace_bytes=Vec::new();\n");
    for (id,test) in tests.iter().enumerate(){source.push_str(&format!("let (state,done,status)=run(c{id}::initial(),c{id}::step,1000,Some((&mut trace_bytes,{id})));assert_ne!(status[0],10);println!(\"GENERATED verify {}: steps={{done}} stop={{}}\",status[0]);\n",test.name));}
    source.push_str("std::fs::write(path,trace_bytes).unwrap();}if mode==\"verify\"{return;}\nlet mut samples=Vec::new();\n");
    source.push_str(&format!("for sample in 0..={samples} {{let state=timed_loop::initial();let initial_ram=state.ram.clone();let start=std::time::Instant::now();let (state,done,status)=run(state,timed_loop::step,{},None);let ns=start.elapsed().as_nanos();assert_eq!(done,{});assert_eq!(status,[1,0x11a,0,0,0]);assert_eq!(state.regs,[{iterations},0,0,1,0,0,0,0]);assert_eq!(state.flags,0x246);assert_eq!(state.undefined,0);assert_eq!(state.ram,initial_ram);if sample!=0{{samples.push(ns);}}}}\n",3*iterations+4,3*iterations+3));
    source.push_str(&format!("println!(\"GENERATED samples_ms={{:?}}\",samples.iter().map(|n|*n as f64/1e6).collect::<Vec<_>>());samples.sort();let mid=samples.len()/2;let median=if samples.len()%2==0{{(samples[mid-1]+samples[mid])/2}}else{{samples[mid]}};println!(\"NATIVE_RESULT median_ns={{median}} instructions={} samples={samples}\");}}\n",3*iterations+3));
    Ok(source)
}
const RUNNER:&str=r#"
struct State{regs:[u32;8],flags:u32,undefined:u32,eip:u32,ram:Ram,rx:VecDeque<u8>,tx:VecDeque<u8>}
fn guard(s:&State,start:u32,bytes:&[u8],cache:SegmentCache,end:u32)->Result<(),[u32;5]>{
 for (index,expected) in bytes.iter().enumerate(){let offset=start+index as u32;
  if offset<0x100 || offset>=end{return Err([7,start,0,0,0]);}
  let context=MemoryContext{segment:SegmentRegister::Cs,cache};
  let at=context.translate(offset,MemWidth::Byte,false,start).map_err(|e|trace::fault(start,e))?;
  let actual=s.ram.load(at,MemWidth::Byte).map_err(|e|trace::fault(start,ExecError::MemoryAccess{eip:start,address:e.address,bytes:1,write:false}))?;
  if actual!=u32::from(*expected){return Err([7,start,0,0,0]);}
 }Ok(())
}
fn run(mut state:State,step:fn(&mut State)->[u32;5],budget:usize,mut trace:Option<(&mut Vec<u8>,u32)>)->(State,usize,[u32;5]){
 for done in 0..budget {
  // Keep the work observable; do not let rustc constant-fold a known guest loop.
  let status=step(std::hint::black_box(&mut state));
  if let Some((out,id))=trace.as_mut(){let rx:Vec<_>=state.rx.iter().copied().collect();let tx:Vec<_>=state.tx.iter().copied().collect();trace::record(out,*id,status,&state.regs,state.flags,state.undefined,state.eip,state.ram.bytes(),&rx,&tx);}
  if status[0]!=0{return (state,done,status);}
 }let ip=state.eip;(state,budget,[10,ip,0,0,0])
}
"#;
// Parse borrowed slices: damaged length fields cannot cause allocations or panics.
struct TraceRecord<'a> { header:[u32;17], sections:[&'a [u8];3] }
fn trace_record<'a>(bytes:&'a [u8],at:&mut usize)->Result<Option<TraceRecord<'a>>,String> {
    if *at==bytes.len(){return Ok(None);}
    fn take<'a>(bytes:&'a [u8],at:&mut usize,len:usize)->Result<&'a [u8],String> {
        let end=at.checked_add(len).ok_or("trace length overflow")?;
        let part=bytes.get(*at..end).ok_or_else(||format!("truncated trace at byte {}: need {len}, {} remain",*at,bytes.len().saturating_sub(*at)))?;
        *at=end;Ok(part)
    }
    let mut header=[0;17];
    for value in &mut header {*value=u32::from_le_bytes(take(bytes,at,4)?.try_into().unwrap());}
    let mut sections:[&[u8];3]=[&[];3];
    for section in &mut sections {let len=u32::from_le_bytes(take(bytes,at,4)?.try_into().unwrap()) as usize;*section=take(bytes,at,len)?;}
    Ok(Some(TraceRecord{header,sections}))
}
pub fn verify_traces(expected:&[u8],actual:&[u8])->Result<(),String> {
    const FIELDS:[&str;17]=["case","stop.kind","stop.eip","stop.detail0","stop.detail1","stop.detail2","EAX","ECX","EDX","EBX","ESP","EBP","ESI","EDI","EFLAGS","undefined_flags","EIP"];
    const CASES:[&str;9]=["selftest","hello","factorials","loop17","readonly_store","branch_fault","code_guard","rx_wait","tx_blocked"];
    let (mut left,mut right,mut record,mut step)=(0,0,0,0);
    let mut previous=None;let mut start_eip=guest::ENTRY;
    loop {
        record+=1;
        let a=trace_record(expected,&mut left).map_err(|e|format!("interpreter record {record}: {e}"))?;
        let b=trace_record(actual,&mut right).map_err(|e|format!("generated record {record}: {e}"))?;
        let (a,b)=match (a,b) {
            (None,None)=>return if record==1{Err("empty backend traces".into())}else{Ok(())},
            (Some(a),Some(b))=>(a,b),
            (a,_)=>return Err(format!("backend trace record {record}: {} record missing",if a.is_none(){"interpreter"}else{"generated"})),
        };
        let id=a.header[0];
        if previous!=Some(id){step=0;start_eip=guest::ENTRY;}
        step+=1;
        let name=CASES.get(id as usize).copied().unwrap_or("unknown");
        let context=format!("backend trace case {id} ({name}), step {step}, record {record}, instruction EIP={start_eip:08x}");
        for (index,field) in FIELDS.iter().enumerate(){
            if a.header[index]!=b.header[index]{return Err(format!("{context}: {field}: interpreter={:08x}, generated={:08x}",a.header[index],b.header[index]));}
        }
        for (index,field) in ["RAM","RX","TX"].iter().enumerate(){
            let (a,b)=(a.sections[index],b.sections[index]);
            if a.len()!=b.len(){return Err(format!("{context}: {field}.length: interpreter={}, generated={}",a.len(),b.len()));}
            if let Some(offset)=a.iter().zip(b).position(|(a,b)|a!=b){return Err(format!("{context}: {field}[0x{offset:x}]: interpreter={:02x}, generated={:02x}",a[offset],b[offset]));}
        }
        previous=Some(id);start_eip=a.header[16];
    }
}
struct Proof {stdout:String,bytes:usize,compile_secs:f64,executable:PathBuf,trace_path:PathBuf}
fn proof_at(directory:&Path,iterations:u32,samples:u32)->Result<Proof,String> {
    validate(iterations,samples)?;let tests=cases()?;let source=emit_suite(iterations,samples)?;
    fs::create_dir_all(directory).map_err(|e|e.to_string())?;
    let source_path=directory.join("generated.rs");let executable=directory.join(if cfg!(windows){"generated.exe"}else{"generated"});let trace_path=directory.join("generated.trace");
    fs::write(&source_path,source).map_err(|e|e.to_string())?;
    let start=Instant::now();let rustc=std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into());
    let built=Command::new(rustc).arg("--edition=2021").arg("-O").arg(&source_path).arg("-o").arg(&executable).output().map_err(|e|format!("cannot run rustc: {e}"))?;
    let compile_time=start.elapsed();if !built.status.success(){return Err(format!("generated Rust build failed: {}",String::from_utf8_lossy(&built.stderr)));}
    // Verification is completed before either engine's timed comparison runs.
    let output=Command::new(&executable).arg(&trace_path).arg("verify").output().map_err(|e|e.to_string())?;
    if !output.status.success(){return Err(format!("generated verifier failed: {}",String::from_utf8_lossy(&output.stderr)));}
    let stdout=String::from_utf8(output.stdout).map_err(|e|e.to_string())?;
    let expected=super::debugger::backend_trace(&tests)?;let actual=fs::read(&trace_path).map_err(|e|e.to_string())?;
    // Preserve both sides even on mismatch so the failure is reproducible.
    fs::write(directory.join("interpreter.trace"),&expected).map_err(|e|e.to_string())?;
    verify_traces(&expected,&actual).map_err(|e|format!("{e}\nArtifacts: {}",directory.display()))?;
    Ok(Proof{stdout,bytes:expected.len(),compile_secs:compile_time.as_secs_f64(),executable,trace_path})
}
pub fn verify_at(directory:&Path)->Result<String,String> {
    let proof=proof_at(directory,16,2)?;
    Ok(format!("PASS: verifier matched CPU/RAM/serial and stop metadata byte-for-byte after every instruction across 9 cases ({} trace bytes).\n{}No benchmark was run. Generated build={:.3} s. Artifacts: {}",proof.bytes,proof.stdout,proof.compile_secs,directory.display()))
}
pub fn compare_at(directory:&Path,iterations:u32,samples:u32)->Result<String,String> {
    let proof=proof_at(directory,iterations,samples)?;
    let output=Command::new(&proof.executable).arg(&proof.trace_path).arg("bench").output().map_err(|e|e.to_string())?;
    if !output.status.success(){return Err(format!("generated benchmark failed: {}",String::from_utf8_lossy(&output.stderr)));}
    let timed=String::from_utf8(output.stdout).map_err(|e|e.to_string())?;
    let median_ns=timed.lines().find_map(|line|line.strip_prefix("NATIVE_RESULT median_ns=")).and_then(|line|line.split_whitespace().next()).ok_or("missing native timing")?.parse::<u128>().map_err(|e|e.to_string())?;
    let (interpreter,median)=super::debugger::benchmark_details(iterations,samples,true)?;
    let native_ms=median_ns as f64/1e6;let native_ips=if median_ns==0{0.0}else{(3*iterations+3) as f64*1e9/median_ns as f64};
    let ratio=if median_ns==0{0.0}else{median/(median_ns as f64/1e9)};
    let cpu=cpu_benchmark(iterations,samples)?;
    Ok(format!("PASS: generated Rust matches interpreter byte-for-byte after every instruction across 9 cases ({} trace bytes).\n{}\n{timed}\n{interpreter}\nGenerated Rust median={native_ms:.3} ms; {native_ips:.0} guest instructions/s; observed ratio={ratio:.2}x vs cached interpreter.\nGenerated build={:.3} s (excluded from samples); rustc -O, fixed-image guarded dispatch, black_box per step. No JIT; debugger remains token-interpreted.\n{cpu}\nArtifacts: {}",proof.bytes,proof.stdout,proof.compile_secs,directory.display()))
}
fn artifact_directory()->Result<PathBuf,String> {
    let root=PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(||"target".into()));
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    Ok(std::env::current_dir().map_err(|e|e.to_string())?.join(root).join("backend-comparison").join(format!("run-{}-{stamp}",std::process::id())))
}
pub fn verify()->Result<String,String> {verify_at(&artifact_directory()?)}
pub fn compare(iterations:u32,samples:u32)->Result<String,String> {
    validate(iterations,samples)?;compare_at(&artifact_directory()?,iterations,samples)
}
#[cfg(test)]
#[path="../tests/unit/native.rs"]
mod tests;

#[cfg(target_arch="x86_64")]
#[inline(never)]
fn cpu_loop(iterations:u32)->(u32,u32) {
    let answer:u32;let remaining:u32;
    // Register-only bounded arithmetic. ESI is input-only; EAX/ECX/EDX and
    // flags are outputs/clobbers. No memory access or stack manipulation.
    unsafe {std::arch::asm!(
        "mov eax, 0", "mov ecx, esi", "mov edx, 1",
        "2:", "add eax, edx", "sub ecx, edx", "jnz 2b",
        in("esi") iterations,lateout("eax") answer,lateout("ecx") remaining,lateout("edx") _,
        options(nomem,nostack)
    );}
    (answer,remaining)
}
pub fn cpu_benchmark(iterations:u32,samples:u32)->Result<String,String> {
    validate(iterations,samples)?;
    #[cfg(target_arch="x86_64")]
    {
        let mut times=Vec::new();
        for sample in 0..=samples {
            let start=Instant::now();let (answer,remaining)=cpu_loop(std::hint::black_box(iterations));let ns=start.elapsed().as_nanos();
            if answer!=iterations || remaining!=0{return Err("native CPU loop result mismatch".into());}
            if sample!=0{times.push(ns);}
        }
        let values=times.iter().map(|n|*n as f64/1e6).collect::<Vec<_>>();times.sort();let mid=times.len()/2;
        let median=if times.len()%2==0{(times[mid-1]+times[mid])/2}else{times[mid]};
        Ok(format!("PASS: PC_NATIVE x86_64 ADD/SUB/JNZ; {iterations} iterations; EAX={iterations}, ECX=0 verified\nsamples_ms={values:?}; median={:.6} ms; {:.3} ns/iteration; {samples} measured samples, 1 excluded warmup.\nEquivalent 32-bit-register arithmetic loop in 64-bit mode; includes call/timer overhead, excludes decoder/PIL/guards. Not execution of the 16-bit image and not 386 MHz.",median as f64/1e6,median as f64/iterations as f64))
    }
    #[cfg(not(target_arch="x86_64"))]
    {Ok(format!("PC_NATIVE unavailable on {}; x86_64 host required. The generated Rust runner still compiles for this host.",std::env::consts::ARCH))}
}
