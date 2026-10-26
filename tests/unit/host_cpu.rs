//! Independent, development-only x86-64 hardware oracle for shared ALU forms.
use super::*;
// Architectural comparison mask, including defined and preserved arithmetic bits.
// Never derive comparison coverage from the DUT's undefined mask.
const ARITH:u32=0x8d5;
#[derive(Clone,Copy,Debug)]
struct FlagPolicy{defined:u32,preserved:u32,undefined:u32}
fn flag_policy(op:usize)->FlagPolicy{
    let(defined,preserved,undefined)=match op{
        5|6=>(ARITH&!1,!ARITH|1,0),8=>(0,u32::MAX,0),
        9..=12=>(ARITH&!0x10,!ARITH,0x10),
        16=>(0x801,!ARITH,0xd4),
        0..=4|7=>(ARITH,!ARITH,0),_=>unreachable!(),
    };FlagPolicy{defined,preserved,undefined}
}
fn case_policy(op:usize,w:u8,count:u32)->FlagPolicy{
    if matches!(op,13..=15){
        let n=count&31;
        if n==0{return FlagPolicy{defined:0,preserved:u32::MAX,undefined:0};}
        let undefined=0x10|if n!=1{0x800}else{0}|if op!=15&&n>=u32::from(w){1}else{0};
        FlagPolicy{defined:ARITH&!undefined,preserved:!ARITH,undefined}
    }else{flag_policy(op)}
}
fn divergence(actual:&Cpu,expected:&Cpu,policy:FlagPolicy)->Option<String>{
    for (i,name) in ["EAX","ECX","EDX","EBX","ESP","EBP","ESI","EDI"].iter().enumerate(){
        if actual.regs[i]!=expected.regs[i]{return Some(format!("{name}: PIL={:08x},hardware={:08x}",actual.regs[i],expected.regs[i]));}
    }
    let diff=actual.eflags^expected.eflags;
    if diff&policy.defined!=0{return Some(format!("EFLAGS.defined: PIL={:08x},hardware={:08x},mask={:08x}",actual.eflags,expected.eflags,policy.defined));}
    if diff&policy.preserved!=0{return Some("EFLAGS.preserved mismatch".into());}
    if actual.eflags&policy.undefined!=0{return Some("EFLAGS.undefined concrete zero policy mismatch (not a hardware requirement)".into());}
    if actual.undefined_flags!=expected.undefined_flags{return Some("undefined-flag classification mismatch".into());}
    if actual.eip!=expected.eip{return Some("EIP mismatch".into());}None
}
#[test]
fn host_cpu_oracle_rejects_corrupted_results_even_when_marked_undefined(){
    let expected=Cpu{eflags:0x202,eip:0x100,..Cpu::default()};
    for i in 0..8{let mut changed=expected.clone();changed.regs[i]=1;assert!(divergence(&changed,&expected,flag_policy(0)).is_some());}
    for (bits,label) in [(1,"EFLAGS.defined"),(0x200,"EFLAGS.preserved")]{let mut changed=expected.clone();changed.eflags^=bits;changed.undefined_flags=ARITH;assert!(divergence(&changed,&expected,flag_policy(0)).unwrap().contains(label));}
    let mut changed=expected.clone();changed.undefined_flags=1;assert!(divergence(&changed,&expected,flag_policy(0)).is_some());
    let mut changed=expected.clone();changed.eip+=1;assert!(divergence(&changed,&expected,flag_policy(0)).is_some());
    assert!(divergence(&expected,&expected,flag_policy(0)).is_none());
}
#[test]
fn host_cpu_logical_masks_ignore_only_hardware_af_and_enforce_pil_policy(){
    for op in 0..13{let p=flag_policy(op);
        assert_eq!(p.defined|p.preserved|p.undefined,u32::MAX);
        assert_eq!((p.defined&p.preserved)|(p.defined&p.undefined)|(p.preserved&p.undefined),0);
    }
    for op in 9..13{let p=flag_policy(op);
        // Real hardware AF may be one; PIL's deterministic zero is a separate policy.
        let expected=Cpu{eflags:0x212,undefined_flags:0x10,eip:0x100,..Cpu::default()};
        let mut actual=expected.clone();actual.eflags&=!0x10;
        assert!(divergence(&actual,&expected,p).is_none());
        let mut wrong=actual.clone();wrong.eflags|=0x10;
        assert!(divergence(&wrong,&expected,p).unwrap().contains("concrete zero policy"));
        for bit in [1,4,0x40,0x80,0x800]{let mut wrong=actual.clone();wrong.eflags^=bit;wrong.undefined_flags=ARITH;
            assert!(divergence(&wrong,&expected,p).unwrap().contains("EFLAGS.defined"));}
        let mut wrong=actual.clone();wrong.eflags^=0x200;
        assert!(divergence(&wrong,&expected,p).unwrap().contains("EFLAGS.preserved"));
        let mut wrong=actual;wrong.undefined_flags=0;
        assert!(divergence(&wrong,&expected,p).unwrap().contains("classification"));
    }
}
#[test]
fn host_cpu_shift_count_policies_and_imul_masks_are_independent(){
    for op in 13..=15{for w in [8,16,32]{for count in 0..=255{
        let p=case_policy(op,w,count);let n=count&31;
        assert_eq!(p.defined|p.preserved|p.undefined,u32::MAX);
        assert_eq!((p.defined&p.preserved)|(p.defined&p.undefined)|(p.preserved&p.undefined),0);
        if n==0{assert_eq!((p.defined,p.preserved,p.undefined),(0,u32::MAX,0));}
        else{assert_eq!(p.undefined&0x10,0x10);assert_eq!(p.undefined&0x800,if n==1{0}else{0x800});
            assert_eq!(p.undefined&1,u32::from(op!=15&&n>=u32::from(w)));}
        let same=case_policy(op,w,n);assert_eq!((p.defined,p.preserved,p.undefined),(same.defined,same.preserved,same.undefined));
    }}}
    let p=case_policy(16,32,0);assert_eq!((p.defined,p.preserved,p.undefined),(0x801,!ARITH,0xd4));
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
// CMP AL,CL varies the input arithmetic flags without changing any register.
// BT then seeds CF; snapshot actual flags before the target. DF is never modified.
#[inline(never)]
fn hardware(op:usize,w:u8,mut a:u64,mut b:u64,mut d:u64)->Hardware{
    let(mut g0,mut g1)=(GUARD0,GUARD1);let input_flags:u64;let output_flags:u64;
    macro_rules! run{($bytes:literal)=>{unsafe{std::arch::asm!(
        "cmp al, cl","bt rdx, 0","pushfq","pop r8",$bytes,"pushfq","pop r9",
        inout("rax") a,inout("rcx") b,inout("rdx") d,
        inout("r10") g0,inout("r11") g1,lateout("r8") input_flags,lateout("r9") output_flags,
    );}};}
    match(op,w){
        (0,8)=>run!(".byte 0x00,0xc8"),(0,16)=>run!(".byte 0x66,0x01,0xc8"),(0,32)=>run!(".byte 0x01,0xc8"),
        (1,8)=>run!(".byte 0x28,0xc8"),(1,16)=>run!(".byte 0x66,0x29,0xc8"),(1,32)=>run!(".byte 0x29,0xc8"),
        (2,8)=>run!(".byte 0x10,0xc8"),(2,16)=>run!(".byte 0x66,0x11,0xc8"),(2,32)=>run!(".byte 0x11,0xc8"),
        (3,8)=>run!(".byte 0x18,0xc8"),(3,16)=>run!(".byte 0x66,0x19,0xc8"),(3,32)=>run!(".byte 0x19,0xc8"),
        (4,8)=>run!(".byte 0x38,0xc8"),(4,16)=>run!(".byte 0x66,0x39,0xc8"),(4,32)=>run!(".byte 0x39,0xc8"),
        (5,8)=>run!(".byte 0xfe,0xc0"),(5,16)=>run!(".byte 0x66,0xff,0xc0"),(5,32)=>run!(".byte 0xff,0xc0"),
        (6,8)=>run!(".byte 0xfe,0xc8"),(6,16)=>run!(".byte 0x66,0xff,0xc8"),(6,32)=>run!(".byte 0xff,0xc8"),
        (7,8)=>run!(".byte 0xf6,0xd8"),(7,16)=>run!(".byte 0x66,0xf7,0xd8"),(7,32)=>run!(".byte 0xf7,0xd8"),
        (8,8)=>run!(".byte 0xf6,0xd0"),(8,16)=>run!(".byte 0x66,0xf7,0xd0"),(8,32)=>run!(".byte 0xf7,0xd0"),
        (9,8)=>run!(".byte 0x20,0xc8"),(9,16)=>run!(".byte 0x66,0x21,0xc8"),(9,32)=>run!(".byte 0x21,0xc8"),
        (10,8)=>run!(".byte 0x08,0xc8"),(10,16)=>run!(".byte 0x66,0x09,0xc8"),(10,32)=>run!(".byte 0x09,0xc8"),
        (11,8)=>run!(".byte 0x30,0xc8"),(11,16)=>run!(".byte 0x66,0x31,0xc8"),(11,32)=>run!(".byte 0x31,0xc8"),
        (12,8)=>run!(".byte 0x84,0xc8"),(12,16)=>run!(".byte 0x66,0x85,0xc8"),(12,32)=>run!(".byte 0x85,0xc8"),
        (13,8)=>run!(".byte 0xd2,0xe0"),(13,16)=>run!(".byte 0x66,0xd3,0xe0"),(13,32)=>run!(".byte 0xd3,0xe0"),
        (14,8)=>run!(".byte 0xd2,0xe8"),(14,16)=>run!(".byte 0x66,0xd3,0xe8"),(14,32)=>run!(".byte 0xd3,0xe8"),
        (15,8)=>run!(".byte 0xd2,0xf8"),(15,16)=>run!(".byte 0x66,0xd3,0xf8"),(15,32)=>run!(".byte 0xd3,0xf8"),
        (16,16)=>run!(".byte 0x66,0x0f,0xaf,0xc1"),(16,32)=>run!(".byte 0x0f,0xaf,0xc1"),
        _=>unreachable!(),
    }Hardware{a,b,d,g0,g1,input_flags,output_flags}
}
fn changed_flags(op:usize,w:u8,b:u64)->u32{let p=case_policy(op,w,b as u32);p.defined|p.undefined}
fn check_hardware(h:&Hardware,op:usize,w:u8,a:u64,b:u64,d:u64)->Result<(),String>{
    if h.b!=b||h.d!=d||h.g0!=GUARD0||h.g1!=GUARD1{return Err(format!("native source/sentinel corruption: {h:?}"));}
    if h.input_flags&1!=d&1{return Err("native CF seeding failed".into());}
    if (h.input_flags^h.output_flags)&!u64::from(changed_flags(op,w,b))!=0{return Err("native preserved flags changed".into());}
    if matches!(op,4|12){if h.a!=a{return Err("native non-writing instruction changed its operand".into());}}
    else if w==32{if h.a>>32!=0{return Err("native32-bit destination did not zero its upper half".into());}}
    else if (h.a^a)&!mask(w)!=0{return Err("native byte/word changed untouched bits".into());}Ok(())
}
fn harness_selfcheck()->Result<(),String>{
    // Independent known answers validate stub opcode/operand/CF/flag plumbing.
    for(op,w,a,b,carry,result,flags)in [
        (0,8,255,1,0,0,0x55),(1,8,0,1,0,255,0x95),
        (2,32,0xffffffff,0,1,0,0x55),(3,32,0,0,1,0xffffffff,0x95),
        (4,16,0x8000,1,0,0x8000,0x814),(0,16,0x7fff,1,0,0x8000,0x894),
        (5,8,0x7f,0,0,0x80,0x890),(5,8,0x7f,0,1,0x80,0x890),
        (6,8,0x80,0,0,0x7f,0x810),(6,8,0x80,0,1,0x7f,0x810),
        (7,8,1,0,0,0xff,0x95),(8,8,0xf0,0,1,0x0f,0),
        (9,8,0xf0,0x0f,1,0,0x44),(10,8,0x80,1,1,0x81,0x84),
        (11,8,0xff,0xff,1,0,0x44),(12,8,0xf0,0x0f,1,0xf0,0x44),
        (13,8,0x81,1,0,2,0x801),(13,8,0x81,0,1,0x81,0),
        (14,16,0x8000,1,1,0x4000,0x804),(15,8,0x80,7,0,0xff,0x84),
        (15,8,0x80,8,1,0xff,0x85),(16,16,0x7fff,2,0,0xfffe,0x801),
        (16,32,0xffffffff,2,1,0xfffffffe,0),
    ]{
        let h=hardware(op,w,a,b,carry);check_hardware(&h,op,w,a,b,carry)?;
        if h.a&mask(w)!=result||h.output_flags&u64::from(case_policy(op,w,b as u32).defined)!=flags{return Err(format!("native harness known-answer failure: {h:?}"));}
    }Ok(())
}
fn encoded(op:usize,w:u8,native:bool)->Vec<u8>{
    let mut bytes=Vec::new();
    if (native&&w==16)||(!native&&w==32){bytes.push(0x66);}
    if op==16{bytes.extend([0x0f,0xaf,0xc1]);return bytes;}
    let pair=match op{
        0..=4=>[[0x00,0xc8],[0x28,0xc8],[0x10,0xc8],[0x18,0xc8],[0x38,0xc8]][op],
        5=>[0xfe,0xc0],6=>[0xfe,0xc8],7=>[0xf6,0xd8],8=>[0xf6,0xd0],9=>[0x20,0xc8],10=>[0x08,0xc8],11=>[0x30,0xc8],12=>[0x84,0xc8],13=>[0xd2,0xe0],14=>[0xd2,0xe8],15=>[0xd2,0xf8],_=>unreachable!(),
    };
    bytes.extend([pair[0]+u8::from(w!=8),pair[1]]);bytes
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
    for(op,name)in ["add","sub","adc","sbb","cmp","inc","dec","neg","not","and","or","xor","test","shl","shr","sar","imul"].iter().enumerate(){for(wi,w)in [8,16,32].into_iter().enumerate(){
        if op==16&&w==8{continue;}
        let variant=op*3+wi;let bytes=encoded(op,w,false);let native_bytes=encoded(op,w,true);
        let(image,ram)=super::super::guest::Image::load(&bytes)?;
        let decoded=image.decode(&ram,super::super::guest::ENTRY).map_err(|e|format!("{e:?}"))?;
        let super::super::guest::Operation::Pil{program,dst,src,flow,address:None}=decoded.operation else{return Err("expected register PIL instruction".into());};
        source.push_str(&format!("mod p{variant} {{ {} }}\n",emit_rust(&program)));
        dispatch.push_str(&format!("{variant}=>p{variant}::execute_with_flow(&mut regs,&mut flags,&mut undefined,&mut eip,({dr},{dl}),({sr},{sl}),p{variant}::{flow:?}).unwrap(),\n",dr=dst.register,dl=dst.low,sr=src.register,sl=src.low));
        let m=mask(w)as u32;let sign=1u32<<(w-1);
        let mut boundary:Vec<_>=[0,1,2,15,16,127,128,255,256,sign-1,sign,sign+1,m-1,m].into_iter().map(|n|n&m).collect();boundary.sort();boundary.dedup();
        let mut pairs=Vec::new();for a in &boundary{
            if matches!(op,13..=15){for b in 0..=255{pairs.push((*a,b));}}
            else{for b in &boundary{pairs.push((*a,*b));}}
        }
        let mut seed=0x6d2b79f5u32^(variant as u32);for _ in 0..512{
            seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;let a=seed&m;
            seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;pairs.push((a,seed&if matches!(op,13..=15){255}else{m}));
        }
        let metadata:&[u32]=if op>=9{&[0,ARITH]}else if op>=5{&[ARITH&!1,ARITH]}else{&[ARITH&!1]};
        for &undefined_in in metadata{for carry in [0,1]{for &(a,b)in &pairs{
            let policy=case_policy(op,w,b);
            let a=(0x13579bdfu64<<32)|u64::from(initial_low(a,w,false));
            let b=(0xfedcba98u64<<32)|u64::from(initial_low(b,w,true));
            let d=(0x0badf00du64<<32)|0x2468ac00|carry;
            let h=hardware(op,w,std::hint::black_box(a),std::hint::black_box(b),d);check_hardware(&h,op,w,a,b,d)?;
            let initial=Cpu{regs:[a as u32,b as u32,d as u32,0x22446688,0x11223344,0x55aa55aa,0x12345678,0x87654321],eflags:h.input_flags as u32,undefined_flags:undefined_in,eip:super::super::guest::ENTRY};
            let mut expected=initial.clone();expected.regs[0]=h.a as u32;expected.eflags=h.output_flags as u32;expected.undefined_flags=(initial.undefined_flags&policy.preserved)|policy.undefined;expected.eip=flow.next_eip;
            writeln!(json,"{{\"id\":{count},\"op\":\"{name}\",\"width\":{w},\"carry\":{carry},\"guest_bytes\":\"{}\",\"native_bytes\":\"{}\",\"regs_in\":{:?},\"flags_in\":{},\"regs_expected\":{:?},\"flags_expected\":{},\"record_schema\":2,\"defined_mask\":{},\"preserved_mask\":{},\"undefined_mask\":{},\"changed_flags_mask\":{},\"undefined_in\":{},\"undefined_expected\":{},\"native_rax_in\":\"{a:016x}\",\"native_rax_out\":\"{:016x}\"}}",hex(&bytes),hex(&native_bytes),initial.regs,initial.eflags,expected.regs,expected.eflags,policy.defined,policy.preserved,policy.undefined,changed_flags(op,w,b),initial.undefined_flags,expected.undefined_flags,h.a).map_err(|e|e.to_string())?;
            let mut actual=initial.clone();super::super::execute_flow(&program,&mut actual,dst,src,flow).map_err(|e|format!("{e:?}"))?;
            if let Some(error)=divergence(&actual,&expected,policy){json.flush().map_err(|e|e.to_string())?;return Err(format!("host CPU case{count} {name}{w} carry{carry}: {error}; initial={initial:?}"));}
            for n in [count,variant as u32,u32::from(w),initial.eflags,initial.undefined_flags,expected.eflags].into_iter().chain(initial.regs).chain(expected.regs).chain([initial.eip,expected.eip,expected.undefined_flags,policy.defined,policy.preserved,policy.undefined]){data.extend_from_slice(&n.to_le_bytes());}
            count+=1;
        }}}
    }}
    json.flush().map_err(|e|e.to_string())?;fs::write(dir.join("hardware.bin"),&data).map_err(|e|e.to_string())?;
    source.push_str("fn main(){let data=include_bytes!(\"hardware.bin\");assert_eq!(data.len()%112,0);let mut count=0;for c in data.chunks_exact(112){let n=|i:usize|u32::from_le_bytes(c[i*4..i*4+4].try_into().unwrap());let id=n(0);let mut regs=[0;8];for i in 0..8{regs[i]=n(6+i);}let mut flags=n(3);let mut undefined=n(4);let mut eip=n(22);match n(1){\n");
    source.push_str(&dispatch);
    source.push_str("_=>panic!(\"bad variant\"),}for i in 0..8{assert_eq!(regs[i],n(14+i),\"case{id} register{i}\");}assert_eq!((flags^n(5))&n(25),0,\"case{id} EFLAGS.defined\");assert_eq!((flags^n(3))&n(26),0,\"case{id} EFLAGS.preserved\");assert_eq!(flags&n(27),0,\"case{id} PIL undefined concrete zero policy (not hardware)\");assert_eq!(undefined,n(24),\"case{id} undefined flags\");assert_eq!(eip,n(23),\"case{id} EIP\");count+=1;}println!(\"PASS: HOST_GENERATED {count} hardware cases\");}\n");
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
    let report=format!("PASS: HOST_CPU {count} native CPU cases match interpreter and generated Rust; ADD/SUB/ADC/SBB/CMP/INC/DEC/NEG/NOT/AND/OR/XOR/TEST/SHL/SHR/SAR8/16/32 and IMUL16/32,CF0/1.\nTwenty-three known-answer harness checks passed;50 encoding variants; independent defined/preserved/undefined masks (undefined values excluded from hardware comparison); preserved flags/registers/EIP and operation-specific undefined metadata checked.\nNative RAX upper-half policy checked separately; guest16-bit-default and native64-bit-mode encodings are recorded separately. This validates the admitted overlap,not a64-bit guest decoder or386 system/undefined/timing behavior.\n{}\nhost={}/{} version={} source={}\ntoolchain={}\ncase_data_fnv64={fingerprint:016x} (non-cryptographic fingerprint); no timing benchmark.\nArtifacts: {}",host_identity(),std::env::consts::OS,std::env::consts::ARCH,env!("CARGO_PKG_VERSION"),source_revision(),toolchain,dir.display());
    fs::write(dir.join("report.txt"),&report).map_err(|e|e.to_string())?;println!("{report}");Ok(())
}
}
