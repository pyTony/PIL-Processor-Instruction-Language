//! Precompiled instruction forms: native Rust for every PIL instruction form
//! the decoder can produce, built once into the `pil` binary and the `dine`
//! library and used the same way for every guest program.
//!
//! A *form* is a decoded PIL program with its immediate values taken out:
//! two `ADD AX,imm16` instructions with different immediates are one form.
//! `pil --emit-precompiled-forms PATH` enumerates the decoder (prefixes,
//! one- and two-byte opcodes, every ModRM byte, several immediate patterns,
//! 16- and 32-bit real- and protected-mode code and stacks) and
//! writes one Rust function per form to `forms.rs`, rendered from the PIL
//! tokens by the same emitter as `--emit-rust`. Constants that changed
//! between the enumerated decodes are read from a slot at run time; the
//! others stay literals and are checked when an instruction is resolved.
//!
//! At run time a decoded instruction resolves to its form once (the result
//! is kept with the decode-cache entry). A program the table does not have,
//! or whose fixed constants differ, runs in the interpreter as before. The
//! generated function is `execute_with_machine` of the generated-Rust
//! backend, which `--verify-backends` compares with the interpreter; the
//! lockstep mode (`PIL_PRECOMPILED=verify`) compares both after every step.
//!
//! Nothing here knows which program runs. See
//! docs/development/BLOCK_COMPILER.md.
use crate::{address,guest,ExecError,FlowContext,Program,Ram,Token};
use std::collections::{BTreeMap,HashMap};
use std::sync::OnceLock;

// The generated table dominates Android code size. Omit it entirely for
// memory-constrained hosts while retaining PIL decoding and interpretation, or
// select `common-forms` to include only the most frequently used forms.
// This is a compile-time choice: PIL_PRECOMPILED=off only disables execution.
#[cfg(all(not(feature = "compact-interpreter"), not(feature = "common-forms")))]
#[path="forms.rs"]
pub mod forms;
#[cfg(all(not(feature = "compact-interpreter"), feature = "common-forms"))]
#[path="common_forms.rs"]
pub mod forms;
#[cfg(feature = "compact-interpreter")]
pub mod forms {
    pub static FORMS: [(u128, &[Option<u64>], super::FormFn); 0] = [];
}

pub type FormFn=fn(&mut [u32;8],&mut u32,&mut u32,&mut u32,(usize,u8),(usize,u8),FlowContext,&mut Ram,Option<address::AddressContext>,&mut address::SegmentState,&[u64])->Result<(),ExecError>;

/// A resolved instruction: the native function and its constant slots.
#[derive(Clone)]
pub struct Native { pub form:FormFn, pub consts:Box<[u64]> }
impl std::fmt::Debug for Native {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{write!(f,"Native({} constants)",self.consts.len())}
}

/// The structure of a program with every constant value set to zero, and
/// the constant values in token order.
pub fn shape(p:&Program)->(String,Vec<u64>){
    let mut consts=Vec::new();
    let tokens:Vec<Token>=p.tokens.iter().map(|t|match t{
        Token::Const(o,v)=>{consts.push(v.bits);let mut v=*v;v.bits=0;Token::Const(*o,v)},
        t=>t.clone(),
    }).collect();
    let text=format!("{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}",p.operand_width,p.source_width,p.uses_flow,p.uses_displacement,
        p.uses_memory,p.partial,p.checkpoint_flags,p.has_read_address,p.register_effects,p.widths,tokens);
    (text,consts)
}
/// Two independent 64-bit FNV-1a hashes of the shape text.
pub fn key(shape:&str)->u128{
    let fnv=|basis:u64|shape.bytes().fold(basis,|h,b|(h^u64::from(b)).wrapping_mul(0x100_0000_01b3));
    (u128::from(fnv(0xcbf2_9ce4_8422_2325))<<64)|u128::from(fnv(0x8422_2325_cbf2_9ce4))
}

pub fn table()->&'static HashMap<u128,usize>{
    static TABLE:OnceLock<HashMap<u128,usize>>=OnceLock::new();
    TABLE.get_or_init(||forms::FORMS.iter().enumerate().map(|(i,(k,_,_))|(*k,i)).collect())
}
/// Number of forms compiled into this build.
#[cfg(any(test,feature="test-support"))]
pub fn forms()->usize{forms::FORMS.len()}

/// The native form of `p`, if this build has one and every fixed constant
/// matches.
pub fn resolve(p:&Program)->Option<Native>{
    if forms::FORMS.is_empty(){return None;}
    let (text,consts)=shape(p);
    let &(_,fixed,form)=forms::FORMS.get(*table().get(&key(&text))?)?;
    if fixed.len()!=consts.len()||fixed.iter().zip(&consts).any(|(f,c)|f.is_some_and(|f|f!=*c)){return None;}
    Some(Native{form,consts:consts.into_boxed_slice()})
}

/// Resolve an already compiled application call. No guest decoding or PIL
/// parsing occurs here. Fixed literals are still checked at the library ABI.
pub fn prepared(form_key:u128,consts:&[u64])->Option<Native>{
    let &(_,fixed,form)=forms::FORMS.get(*table().get(&form_key)?)?;
    if fixed.len()!=consts.len()||fixed.iter().zip(consts).any(|(f,c)|f.is_some_and(|f|f!=*c)){return None;}
    Some(Native{form,consts:consts.into()})
}

/// Execution mode, from `PIL_PRECOMPILED`: `off` uses only the interpreter,
/// `verify` runs both and compares the full state after each instruction,
/// `required` refuses an unavailable native form before it executes;
/// anything else (the default) runs the native form when there is one.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Mode { Off, On, Verify, Required }
pub fn mode()->Mode{
    static MODE:OnceLock<Mode>=OnceLock::new();
    *MODE.get_or_init(||match std::env::var("PIL_PRECOMPILED").unwrap_or_default().to_ascii_lowercase().as_str(){
        "off"|"0"|"no"|"interpreter"=>Mode::Off,
        "verify"|"lockstep"=>Mode::Verify,
        "required"|"strict"=>Mode::Required,
        _=>Mode::On,
    })
}

// ---------------------------------------------------------------------------
// Generation

/// Collected forms: shape text -> (first program, constant slots that never
/// changed: Some(value), changed: None).
type Collected=BTreeMap<String,(Program,Vec<Option<u64>>)>;

fn merge(into:&mut Collected,from:Collected){
    for (text,(program,slots)) in from{
        match into.get_mut(&text){
            None=>{into.insert(text,(program,slots));},
            Some((_,known))=>for (k,s) in known.iter_mut().zip(slots){if *k!=s{*k=None;}},
        }
    }
}
fn record(into:&mut Collected,p:&Program){
    let (text,consts)=shape(p);
    match into.get_mut(&text){
        None=>{into.insert(text,(p.clone(),consts.into_iter().map(Some).collect()));},
        Some((_,known))=>for (k,c) in known.iter_mut().zip(consts){if *k!=Some(c){*k=None;}},
    }
}

const FILLS:[[u8;10];4]=[
    [0x12,0x34,0x56,0x78,0x9a,0xbc,0xde,0xf0,0x01,0x02],
    [0x81,0x00,0x00,0x80,0x03,0x00,0x00,0x00,0x7f,0x40],
    [0x24,0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08],
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00],
];

/// Enumerate every decodable form in the given machines (real- and
/// protected-mode 16- and 32-bit code and stack segments).
/// Deterministic; uses all host cores.
pub fn enumerate(machines:&[(guest::Image,Ram)])->Collected{
    let mut prefixes=Vec::new();
    for a in [None,Some(0x66u8)]{for b in [None,Some(0x67u8)]{for s in [None,Some(0x26u8),Some(0x2e),Some(0x36),Some(0x3e),Some(0x64),Some(0x65)]{for r in [None,Some(0xf3u8),Some(0xf2)]{
        prefixes.push([a,b,s,r].into_iter().flatten().collect::<Vec<u8>>());
    }}}}
    let threads=std::thread::available_parallelism().map_or(1,|n|n.get()).clamp(1,16);
    let opcodes:Vec<u32>=(0..512).collect();
    let chunks:Vec<Vec<u32>>=(0..threads).map(|t|opcodes.iter().copied().filter(|o|*o as usize%threads==t).collect()).collect();
    let parts:Vec<Collected>=std::thread::scope(|scope|{
        let handles:Vec<_>=chunks.iter().map(|chunk|{let prefixes=&prefixes;let machines=machines.to_vec();scope.spawn(move||{
            let mut found=Collected::new();
            for (image,mut ram) in machines{
                let at=image.segments[1].base+0x100;
                for prefix in prefixes{for &op in chunk{for modrm in 0..=255u8{for fill in &FILLS{
                    let mut bytes=prefix.clone();if op>=256{bytes.push(0x0f);}bytes.push(op as u8);bytes.push(modrm);bytes.extend_from_slice(fill);
                    if ram.write_bytes(at,&bytes).is_err(){continue;}
                    if let Ok(d)=image.decode(&ram,0x100){if let guest::Operation::Pil{program,..}=&d.operation{record(&mut found,program);}}
                }}}}
            }
            found
        })}).collect();
        handles.into_iter().map(|h|h.join().expect("enumeration thread")).collect()
    });
    let mut all=Collected::new();for part in parts{merge(&mut all,part);}all
}

/// Which precompiled form table to emit or verify.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum FormSet { Full, Common }

fn is_common_opcode(op:u32)->bool{
    matches!(op,
        0x00..=0x05|0x08..=0x0d|0x10..=0x15|0x18..=0x1d|0x20..=0x25|0x28..=0x2d|0x30..=0x35|0x38..=0x3d|0x80|0x81|0x83
        |0x40..=0x4f|0x84|0x85|0xa8|0xa9|0xf6|0xf7|0xfe|0xff|0x69|0x6b|0x1af
        |0x88..=0x8b|0xa0..=0xa3|0xb0..=0xbf|0xc6|0xc7|0x8d|0x98|0x99|0x86|0x87|0x91..=0x97|0x1b6|0x1b7|0x1be|0x1bf
        |0xc0|0xc1|0xd0..=0xd3
        |0x50..=0x5f|0x68|0x6a|0x8f|0x70..=0x7f|0x180..=0x18f|0xe8|0xe9|0xeb|0xc2|0xc3|0xc9|0xe2
        |0xa4..=0xa7|0xaa..=0xaf
        |0x90|0xf8|0xf9|0xfc|0xfd|0x190..=0x19f|0x1a3|0x1ab|0x1b3|0x1bb|0x1ba)
}

fn common_modrm_included(op:u32,modrm:u8)->bool{
    let ext=(modrm>>3)&7;
    match op{
        0xff=>matches!(ext,0|1|2|4|6),
        0xc0|0xc1|0xd0..=0xd3=>matches!(ext,0|1|4|5|6|7),
        0x86|0x87|0x190..=0x19f|0x1a3|0x1ab|0x1b3|0x1bb|0x1ba=>modrm>=0xc0,
        _=>true,
    }
}

/// Enumerate only the most frequently used 16-bit real-mode and 32-bit
/// protected-mode instruction forms. Deterministic; uses all host cores.
pub fn enumerate_common(machines:&[(guest::Image,Ram)])->Collected{
    let plain_prefixes:[&[u8];2]=[&[],&[0x66]];
    let string_prefixes:[&[u8];6]=[&[],&[0x66],&[0xf3],&[0x66,0xf3],&[0xf2],&[0x66,0xf2]];
    let threads=std::thread::available_parallelism().map_or(1,|n|n.get()).clamp(1,16);
    let opcodes:Vec<u32>=(0..512).filter(|&op|is_common_opcode(op)).collect();
    let chunks:Vec<Vec<u32>>=(0..threads).map(|t|opcodes.iter().copied().filter(|o|*o as usize%threads==t).collect()).collect();
    let common_machines:Vec<_>=machines.iter().take(2).cloned().collect();
    let parts:Vec<Collected>=std::thread::scope(|scope|{
        let handles:Vec<_>=chunks.iter().map(|chunk|{let machines=common_machines.clone();scope.spawn(move||{
            let mut found=Collected::new();
            for (image,mut ram) in machines{
                let at=image.segments[1].base+0x100;
                for &op in chunk{
                    let prefixes:&[&[u8]]=if matches!(op,0xa4..=0xa7|0xaa..=0xaf){&string_prefixes}else{&plain_prefixes};
                    for &prefix in prefixes{for modrm in 0..=255u8{
                        if !common_modrm_included(op,modrm){continue;}
                        for fill in &FILLS{
                            let mut bytes=prefix.to_vec();if op>=256{bytes.push(0x0f);}bytes.push(op as u8);bytes.push(modrm);bytes.extend_from_slice(fill);
                            if ram.write_bytes(at,&bytes).is_err(){continue;}
                            if let Ok(d)=image.decode(&ram,0x100){if let guest::Operation::Pil{program,..}=&d.operation{record(&mut found,program);}}
                        }
                    }}
                }
            }
            found
        })}).collect();
        handles.into_iter().map(|h|h.join().expect("enumeration thread")).collect()
    });
    let mut all=Collected::new();for part in parts{merge(&mut all,part);}all
}

/// Render a precompiled form table with an explicit generator command header.
pub fn render_with_header(forms:&Collected,generator:&str)->String{
    let mut out=format!("// @generated by `{generator}`. Do not edit.\n\
// One native function per PIL instruction form; see crates/pil-cpu/src/precompiled/mod.rs.\n\
#![allow(unused_variables,unused_mut,unused_assignments,unused_parens,unused_braces,dead_code,clippy::all)]\n\
#[allow(unused_imports)]\nuse crate::{{FlowContext,ExecError,Ram,MemWidth,address,privilege,task}};\n");
    out.push_str(&format!("pub static FORMS:[(u128,&[Option<u64>],super::FormFn);{}]=[\n",forms.len()));
    for (n,(text,(_,slots))) in forms.iter().enumerate(){
        let slots:Vec<String>=slots.iter().map(|s|s.map_or("None".into(),|v|format!("Some({v})"))).collect();
        out.push_str(&format!("({:#034x},&[{}],f{n}),\n",key(text),slots.join(",")));
    }
    out.push_str("];\n");
    for (n,(_,(program,slots))) in forms.iter().enumerate(){
        out.push_str(&crate::emit_rust_form(program,slots).replacen("pub fn execute_form(",&format!("fn f{n}("),1));
    }
    out
}

/// Render forms.rs.
pub fn render(forms:&Collected)->String{
    render_with_header(forms,"pil --emit-precompiled-forms crates/pil-cpu/src/precompiled/forms.rs")
}

/// `pil --emit-precompiled-forms PATH [--common] [--check]`.
pub fn emit_set(path:&str,check:bool,set:FormSet,machines:&[(guest::Image,Ram)])->Result<String,String>{
    let forms=match set{FormSet::Full=>enumerate(machines),FormSet::Common=>enumerate_common(machines)};
    let text=match set{
        FormSet::Full=>render(&forms),
        FormSet::Common=>render_with_header(&forms,"pil --emit-precompiled-forms crates/pil-cpu/src/precompiled/common_forms.rs --common"),
    };
    if check{
        let current=std::fs::read_to_string(path).map_err(|e|format!("{path}: {e}"))?;
        if current.replace("\r\n","\n")!=text{
            let suffix=if set==FormSet::Common{" --common"}else{""};
            return Err(format!("{path} is out of date: run pil --emit-precompiled-forms {path}{suffix}"));
        }
        return Ok(format!("{path} is current: {} forms",forms.len()));
    }
    std::fs::write(path,&text).map_err(|e|format!("{path}: {e}"))?;
    Ok(format!("wrote {} forms ({} bytes) to {path}",forms.len(),text.len()))
}

/// `pil --emit-precompiled-forms PATH [--check]`.
pub fn emit(path:&str,check:bool,machines:&[(guest::Image,Ram)])->Result<String,String>{
    emit_set(path,check,FormSet::Full,machines)
}
