//! Independent, development-only x86-64 hardware oracle for shared ALU forms.
use super::*;
// Architectural mask for the admitted operations, not the DUT's undefined mask.
const DEFINED:u32=0x8d5;
fn divergence(actual:&Cpu,expected:&Cpu)->Option<String>{
    for (i,name) in ["EAX","ECX","EDX","EBX","ESP","EBP","ESI","EDI"].iter().enumerate(){
        if actual.regs[i]!=expected.regs[i]{return Some(format!("{name}: PIL={:08x},hardware={:08x}",actual.regs[i],expected.regs[i]));}
    }
    let diff=actual.eflags^expected.eflags;
    if diff&DEFINED!=0{return Some(format!("EFLAGS.defined: PIL={:08x},hardware={:08x},mask={DEFINED:08x}",actual.eflags,expected.eflags));}
    if diff&!DEFINED!=0{return Some("EFLAGS.preserved mismatch".into());}
    if actual.undefined_flags!=expected.undefined_flags{return Some("undefined-flag classification mismatch".into());}
    if actual.eip!=expected.eip{return Some("EIP mismatch".into());}None
}
#[test]
fn host_cpu_oracle_rejects_corrupted_results_even_when_marked_undefined(){
    let expected=Cpu{eflags:0x202,eip:0x100,..Cpu::default()};
    for i in 0..8{let mut changed=expected.clone();changed.regs[i]=1;assert!(divergence(&changed,&expected).is_some());}
    for (bits,label) in [(1,"EFLAGS.defined"),(0x200,"EFLAGS.preserved")]{let mut changed=expected.clone();changed.eflags^=bits;changed.undefined_flags=DEFINED;assert!(divergence(&changed,&expected).unwrap().contains(label));}
    let mut changed=expected.clone();changed.undefined_flags=1;assert!(divergence(&changed,&expected).is_some());
    let mut changed=expected.clone();changed.eip+=1;assert!(divergence(&changed,&expected).is_some());
    assert!(divergence(&expected,&expected).is_none());
}
#[test]
#[cfg_attr(not(target_arch="x86_64"),ignore="requires an actual x86-64 host; no hardware verification on this target")]
fn host_cpu_alu_matches_interpreter_and_generated_rust(){
    #[cfg(target_arch="x86_64")]
    x86::verify().unwrap();
    #[cfg(not(target_arch="x86_64"))]
    panic!("UNAVAILABLE: actual x86-64 CPU oracle cannot run on this target");
}
#[cfg(target_arch="x86_64")]
mod x86{
use super::*;
use std::{fs,io::{BufWriter,Write},path::PathBuf,process::Command,time::{SystemTime,UNIX_EPOCH}};
const GUARD0:u64=0x55aa33cc77889900;
const GUARD1:u64=0xaabbccdd01234567;
#[derive(Debug)]
struct Hardware{a:u64,b:u64,d:u64,g0:u64,g1:u64,input_flags:u64,output_flags:u64}
// The Rust compiler manages the function ABI. Only volatile registers are used
// explicitly on both Windows x64 and SysV. PUSHFQ/POP pairs balance the stack;
// default asm stack/memory/flag clobbers are intentional (no nostack/nomem).
// BT seeds only the consumed CF; snapshot its other actual flag values. Every
// admitted ALU operation defines all six arithmetic flags. DF is never modified.
#[inline(never)]
fn hardware(op:usize,w:u8,mut a:u64,mut b:u64,mut d:u64)->Hardware{
    let(mut g0,mut g1)=(GUARD0,GUARD1);let input_flags:u64;let output_flags:u64;
    macro_rules! run{($bytes:literal)=>{unsafe{std::arch::asm!(
        "bt rdx, 0","pushfq","pop r8",$bytes,"pushfq","pop r9",
        inout("rax") a,inout("rcx") b,inout("rdx") d,
        inout("r10") g0,inout("r11") g1,lateout("r8") input_flags,lateout("r9") output_flags,
    );}};}
    match(op,w){
        (0,8)=>run!(".byte 0x00,0xc8"),(0,16)=>run!(".byte 0x66,0x01,0xc8"),(0,32)=>run!(".byte 0x01,0xc8"),
        (1,8)=>run!(".byte 0x28,0xc8"),(1,16)=>run!(".byte 0x66,0x29,0xc8"),(1,32)=>run!(".byte 0x29,0xc8"),
        (2,8)=>run!(".byte 0x10,0xc8"),(2,16)=>run!(".byte 0x66,0x11,0xc8"),(2,32)=>run!(".byte 0x11,0xc8"),
        (3,8)=>run!(".byte 0x18,0xc8"),(3,16)=>run!(".byte 0x66,0x19,0xc8"),(3,32)=>run!(".byte 0x19,0xc8"),
        (4,8)=>run!(".byte 0x38,0xc8"),(4,16)=>run!(".byte 0x66,0x39,0xc8"),(4,32)=>run!(".byte 0x39,0xc8"),
        _=>unreachable!(),
    }Hardware{a,b,d,g0,g1,input_flags,output_flags}
}
fn check_hardware(h:&Hardware,op:usize,w:u8,a:u64,b:u64,d:u64)->Result<(),String>{
    if h.b!=b||h.d!=d||h.g0!=GUARD0||h.g1!=GUARD1{return Err(format!("native source/sentinel corruption: {h:?}"));}
    if h.input_flags&1!=d&1{return Err("native CF seeding failed".into());}
    if (h.input_flags^h.output_flags)&!u64::from(DEFINED)!=0{return Err("native non-arithmetic flags changed".into());}
    if op==4{if h.a!=a{return Err("native CMP changed its operand".into());}}
    else if w==32{if h.a>>32!=0{return Err("native32-bit destination did not zero its upper half".into());}}
    else if (h.a^a)&!mask(w)!=0{return Err("native byte/word changed untouched bits".into());}Ok(())
}
fn harness_selfcheck()->Result<(),String>{
    // Independent known answers validate stub opcode/operand/CF/flag plumbing.
    for(op,w,a,b,carry,result,flags)in [
        (0,8,255,1,0,0,0x55),(1,8,0,1,0,255,0x95),
        (2,32,0xffffffff,0,1,0,0x55),(3,32,0,0,1,0xffffffff,0x95),
        (4,16,0x8000,1,0,0x8000,0x814),(0,16,0x7fff,1,0,0x8000,0x894),
    ]{
        let h=hardware(op,w,a,b,carry);check_hardware(&h,op,w,a,b,carry)?;
        if h.a&mask(w)!=result||h.output_flags&u64::from(DEFINED)!=flags{return Err(format!("native harness known-answer failure: {h:?}"));}
    }Ok(())
}
fn encoded(op:usize,w:u8,native:bool)->Vec<u8>{
    let base=[0x00,0x28,0x10,0x18,0x38][op];let mut bytes=Vec::new();
    if (native&&w==16)||(!native&&w==32){bytes.push(0x66);}
    bytes.extend([base+u8::from(w!=8),0xc8]);bytes
}
fn hex(bytes:&[u8])->String{bytes.iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join(" ")}
fn initial_low(n:u32,w:u8,source:bool)->u32{
    let tag=if source{0x3c5a8100}else{0xa5c37e00};(tag&!(mask(w)as u32))|(n&mask(w)as u32)
}
#[allow(unused_unsafe)] // CPUID was unsafe on older Rust toolchains.
fn host_identity()->String{
    let z=unsafe{std::arch::x86_64::__cpuid(0)};let one=unsafe{std::arch::x86_64::__cpuid(1)};
    let vendor=[z.ebx.to_le_bytes(),z.edx.to_le_bytes(),z.ecx.to_le_bytes()].concat();
    format!("vendor={} cpuid1_eax={:08x} cpuid1_ecx={:08x} cpuid1_edx={:08x}",String::from_utf8_lossy(&vendor),one.eax,one.ecx,one.edx)
}
fn source_revision()->String{
    Command::new("git").args(["rev-parse","HEAD"]).current_dir(env!("CARGO_MANIFEST_DIR")).output().ok()
        .filter(|x|x.status.success()).map(|x|String::from_utf8_lossy(&x.stdout).trim().to_owned()).unwrap_or_else(||"unavailable".into())
}
fn directory()->Result<PathBuf,String>{
    let root=PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(||"target".into()));
    let root=if root.is_absolute(){root}else{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(root)};
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let p=root.join("host-cpu-oracle").join(format!("run-{}-{stamp}",std::process::id()));fs::create_dir_all(&p).map_err(|e|e.to_string())?;Ok(p)
}
pub fn verify()->Result<(),String>{
    harness_selfcheck()?;let dir=directory()?;
    let result=verify_at(&dir);result.map_err(|e|format!("{e}\nArtifacts: {}",dir.display()))
}
fn verify_at(dir:&std::path::Path)->Result<(),String>{
    let mut json=BufWriter::new(fs::File::create(dir.join("cases.jsonl")).map_err(|e|e.to_string())?);
    let(mut source,mut dispatch,mut data)=(String::new(),String::new(),Vec::new());let mut count=0u32;
    for(op,name)in ["add","sub","adc","sbb","cmp"].iter().enumerate(){for(wi,w)in [8,16,32].into_iter().enumerate(){
        let variant=op*3+wi;let bytes=encoded(op,w,false);let native_bytes=encoded(op,w,true);
        let(image,ram)=super::super::guest::Image::load(&bytes)?;
        let decoded=image.decode(&ram,super::super::guest::ENTRY).map_err(|e|format!("{e:?}"))?;
        let super::super::guest::Operation::Pil{program,dst,src,flow,address:None}=decoded.operation else{return Err("expected register PIL instruction".into());};
        source.push_str(&format!("mod p{variant} {{ {} }}\n",emit_rust(&program)));
        dispatch.push_str(&format!("{variant}=>p{variant}::execute_with_flow(&mut regs,&mut flags,&mut undefined,&mut eip,(0,0),(1,0),p{variant}::{flow:?}).unwrap(),\n"));
        let m=mask(w)as u32;let sign=1u32<<(w-1);
        let mut boundary:Vec<_>=[0,1,2,15,16,127,128,255,256,sign-1,sign,sign+1,m-1,m].into_iter().map(|n|n&m).collect();boundary.sort();boundary.dedup();
        let mut pairs=Vec::new();for a in &boundary{for b in &boundary{pairs.push((*a,*b));}}
        let mut seed=0x6d2b79f5u32^(variant as u32);for _ in 0..512{
            seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;let a=seed&m;
            seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;pairs.push((a,seed&m));
        }
        for carry in [0,1]{for &(a,b)in &pairs{
            let a=(0x13579bdfu64<<32)|u64::from(initial_low(a,w,false));
            let b=(0xfedcba98u64<<32)|u64::from(initial_low(b,w,true));
            let d=(0x0badf00du64<<32)|0x2468ac00|carry;
            let h=hardware(op,w,std::hint::black_box(a),std::hint::black_box(b),d);check_hardware(&h,op,w,a,b,d)?;
            let initial=Cpu{regs:[a as u32,b as u32,d as u32,0x22446688,0x11223344,0x55aa55aa,0x12345678,0x87654321],eflags:h.input_flags as u32,undefined_flags:DEFINED&!1,eip:super::super::guest::ENTRY};
            let mut expected=initial.clone();expected.regs[0]=h.a as u32;expected.eflags=h.output_flags as u32;expected.undefined_flags=0;expected.eip=flow.next_eip;
            writeln!(json,"{{\"id\":{count},\"op\":\"{name}\",\"width\":{w},\"carry\":{carry},\"guest_bytes\":\"{}\",\"native_bytes\":\"{}\",\"regs_in\":{:?},\"flags_in\":{},\"regs_expected\":{:?},\"flags_expected\":{},\"defined_mask\":{DEFINED},\"native_rax_in\":\"{a:016x}\",\"native_rax_out\":\"{:016x}\"}}",hex(&bytes),hex(&native_bytes),initial.regs,initial.eflags,expected.regs,expected.eflags,h.a).map_err(|e|e.to_string())?;
            let mut actual=initial.clone();super::super::execute_flow(&program,&mut actual,dst,src,flow).map_err(|e|format!("{e:?}"))?;
            if let Some(error)=divergence(&actual,&expected){json.flush().map_err(|e|e.to_string())?;return Err(format!("host CPU case{count} {name}{w} carry{carry}: {error}; initial={initial:?}"));}
            for n in [count,variant as u32,u32::from(w),initial.eflags,initial.undefined_flags,expected.eflags].into_iter().chain(initial.regs).chain(expected.regs).chain([initial.eip,expected.eip,expected.undefined_flags,DEFINED]){data.extend_from_slice(&n.to_le_bytes());}
            count+=1;
        }}
    }}
    json.flush().map_err(|e|e.to_string())?;fs::write(dir.join("hardware.bin"),&data).map_err(|e|e.to_string())?;
    source.push_str("fn main(){let data=include_bytes!(\"hardware.bin\");assert_eq!(data.len()%104,0);let mut count=0;for c in data.chunks_exact(104){let n=|i:usize|u32::from_le_bytes(c[i*4..i*4+4].try_into().unwrap());let id=n(0);let mut regs=[0;8];for i in 0..8{regs[i]=n(6+i);}let mut flags=n(3);let mut undefined=n(4);let mut eip=n(22);match n(1){\n");
    source.push_str(&dispatch);
    source.push_str("_=>panic!(\"bad variant\"),}for i in 0..8{assert_eq!(regs[i],n(14+i),\"case{id} register{i}\");}assert_eq!((flags^n(5))&0x8d5,0,\"case{id} EFLAGS.defined\");assert_eq!((flags^n(3))&!0x8d5u32,0,\"case{id} EFLAGS.preserved\");assert_eq!(undefined,n(24),\"case{id} undefined flags\");assert_eq!(eip,n(23),\"case{id} EIP\");count+=1;}println!(\"PASS: HOST_GENERATED {count} hardware cases\");}\n");
    let path=dir.join("generated.rs");fs::write(&path,source).map_err(|e|e.to_string())?;
    let bin=dir.join(if cfg!(windows){"generated.exe"}else{"generated"});let rustc=std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into());
    let built=Command::new(&rustc).arg("--edition=2021").arg("-O").arg(&path).arg("-o").arg(&bin).output().map_err(|e|e.to_string())?;
    fs::write(dir.join("build.stderr"),&built.stderr).map_err(|e|e.to_string())?;
    if !built.status.success(){return Err(format!("generated oracle runner build: {}",String::from_utf8_lossy(&built.stderr)));}
    let output=Command::new(&bin).output().map_err(|e|e.to_string())?;
    fs::write(dir.join("generated.stdout"),&output.stdout).map_err(|e|e.to_string())?;fs::write(dir.join("generated.stderr"),&output.stderr).map_err(|e|e.to_string())?;
    if !output.status.success(){return Err(format!("generated result differs from hardware: {}",String::from_utf8_lossy(&output.stderr)));}
    let fingerprint=data.iter().fold(0xcbf29ce484222325u64,|h,b|(h^u64::from(*b)).wrapping_mul(0x100000001b3));
    let toolchain=Command::new(&rustc).arg("--version").output().ok().map(|o|String::from_utf8_lossy(&o.stdout).trim().to_owned()).unwrap_or_else(||"unavailable".into());
    let report=format!("PASS: HOST_CPU {count} native CPU cases match interpreter and generated Rust; ADD/SUB/ADC/SBB/CMP8/16/32,CF0/1.\nSix known-answer harness checks passed;15 encoding variants; fixed independent defined mask=0x8d5; preserved other flags/registers/EIP and cleared arithmetic undefined markers.\nNative RAX upper-half policy checked separately; guest16-bit-default and native64-bit-mode encodings are recorded separately. This validates the admitted overlap,not a64-bit guest decoder or386 system/undefined/timing behavior.\n{}\nhost={}/{} version={} source={}\ntoolchain={}\ncase_data_fnv64={fingerprint:016x} (non-cryptographic fingerprint); no timing benchmark.\nArtifacts: {}",host_identity(),std::env::consts::OS,std::env::consts::ARCH,env!("CARGO_PKG_VERSION"),source_revision(),toolchain,dir.display());
    fs::write(dir.join("report.txt"),&report).map_err(|e|e.to_string())?;println!("{report}");Ok(())
}
}
