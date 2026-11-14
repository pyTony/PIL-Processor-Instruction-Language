//! Precompiled instruction forms: native Rust for every PIL instruction form
//! the decoder can produce, built once into the `pil` binary and the `dine`
//! library and used the same way for every guest program.
//!
//! A *form* is a decoded PIL program with its immediate values taken out:
//! two `ADD AX,imm16` instructions with different immediates are one form.
//! `pil --emit-precompiled-forms PATH` enumerates the decoder (prefixes,
//! one- and two-byte opcodes, every ModRM byte, several immediate patterns,
//! real-mode code with a 16-bit stack) and
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

#[path="forms.rs"]
mod forms;

pub(crate) type FormFn=fn(&mut [u32;8],&mut u32,&mut u32,&mut u32,(usize,u8),(usize,u8),FlowContext,&mut Ram,Option<address::AddressContext>,&mut address::SegmentState,&[u64])->Result<(),ExecError>;

/// A resolved instruction: the native function and its constant slots.
#[derive(Clone)]
pub(crate) struct Native { pub(crate) form:FormFn, pub(crate) consts:Box<[u64]> }
impl std::fmt::Debug for Native {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{write!(f,"Native({} constants)",self.consts.len())}
}

/// The structure of a program with every constant value set to zero, and
/// the constant values in token order.
pub(crate) fn shape(p:&Program)->(String,Vec<u64>){
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
pub(crate) fn key(shape:&str)->u128{
    let fnv=|basis:u64|shape.bytes().fold(basis,|h,b|(h^u64::from(b)).wrapping_mul(0x100_0000_01b3));
    (u128::from(fnv(0xcbf2_9ce4_8422_2325))<<64)|u128::from(fnv(0x8422_2325_cbf2_9ce4))
}

fn table()->&'static HashMap<u128,usize>{
    static TABLE:OnceLock<HashMap<u128,usize>>=OnceLock::new();
    TABLE.get_or_init(||forms::FORMS.iter().enumerate().map(|(i,(k,_,_))|(*k,i)).collect())
}
/// Number of forms compiled into this build.
pub(crate) fn forms()->usize{forms::FORMS.len()}

/// The native form of `p`, if this build has one and every fixed constant
/// matches.
pub(crate) fn resolve(p:&Program)->Option<Native>{
    if forms::FORMS.is_empty(){return None;}
    let (text,consts)=shape(p);
    let &(_,fixed,form)=forms::FORMS.get(*table().get(&key(&text))?)?;
    if fixed.len()!=consts.len()||fixed.iter().zip(&consts).any(|(f,c)|f.is_some_and(|f|f!=*c)){return None;}
    Some(Native{form,consts:consts.into_boxed_slice()})
}

/// Execution mode, from `PIL_PRECOMPILED`: `off` uses only the interpreter,
/// `verify` runs both and compares the full state after each instruction,
/// anything else (the default) runs the native form when there is one.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum Mode { Off, On, Verify }
pub(crate) fn mode()->Mode{
    static MODE:OnceLock<Mode>=OnceLock::new();
    *MODE.get_or_init(||match std::env::var("PIL_PRECOMPILED").unwrap_or_default().to_ascii_lowercase().as_str(){
        "off"|"0"|"no"|"interpreter"=>Mode::Off,
        "verify"|"lockstep"=>Mode::Verify,
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

/// The machine the forms are enumerated in: real mode, 16-bit stack.
/// Protected-mode and 32-bit-stack forms are not in the table yet; such
/// instructions run in the interpreter.
fn machines()->Vec<(guest::Image,Ram)>{
    let p=crate::dos::load(&[0xcd,0x20],"FORMS.COM").expect("enumeration image");
    let mut image=p.image.clone();image.end=0xff00;
    vec![(image,p.ram)]
}

const FILLS:[[u8;10];3]=[
    [0x12,0x34,0x56,0x78,0x9a,0xbc,0xde,0xf0,0x01,0x02],
    [0x81,0x00,0x00,0x80,0x03,0x00,0x00,0x00,0x7f,0x40],
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00],
];

/// Enumerate every decodable form. Deterministic; uses all host cores.
pub(crate) fn enumerate()->Collected{
    let mut prefixes=Vec::new();
    for a in [None,Some(0x66u8)]{for b in [None,Some(0x67u8)]{for s in [None,Some(0x26u8),Some(0x2e),Some(0x36),Some(0x3e),Some(0x64),Some(0x65)]{for r in [None,Some(0xf3u8),Some(0xf2)]{
        prefixes.push([a,b,s,r].into_iter().flatten().collect::<Vec<u8>>());
    }}}}
    let threads=std::thread::available_parallelism().map_or(1,|n|n.get()).clamp(1,16);
    let opcodes:Vec<u32>=(0..512).collect();
    let chunks:Vec<Vec<u32>>=(0..threads).map(|t|opcodes.iter().copied().filter(|o|*o as usize%threads==t).collect()).collect();
    let parts:Vec<Collected>=std::thread::scope(|scope|{
        let handles:Vec<_>=chunks.iter().map(|chunk|{let prefixes=&prefixes;scope.spawn(move||{
            let mut found=Collected::new();
            for (image,mut ram) in machines(){
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

/// Render forms.rs.
pub(crate) fn render(forms:&Collected)->String{
    let mut out=String::from("// @generated by `pil --emit-precompiled-forms src/precompiled/forms.rs`. Do not edit.\n\
// One native function per PIL instruction form; see src/precompiled/mod.rs.\n\
#![allow(unused_variables,unused_mut,unused_assignments,unused_parens,unused_braces,dead_code,clippy::all)]\n\
#[allow(unused_imports)]\nuse crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege,task};\n");
    out.push_str(&format!("pub(super) static FORMS:[(u128,&[Option<u64>],super::FormFn);{}]=[\n",forms.len()));
    for (n,(text,(_,slots))) in forms.iter().enumerate(){
        let slots:Vec<String>=slots.iter().map(|s|s.map_or("None".into(),|v|format!("Some({v})"))).collect();
        out.push_str(&format!("({:#034x},&[{}],f{n}::execute_form),\n",key(text),slots.join(",")));
    }
    out.push_str("];\n");
    for (n,(_,(program,slots))) in forms.iter().enumerate(){
        out.push_str(&format!("mod f{n}{{use super::*;\n{}}}\n",crate::emit_rust_form(program,slots)));
    }
    out
}

/// `pil --emit-precompiled-forms PATH [--check]`.
pub(crate) fn emit(path:&str,check:bool)->Result<String,String>{
    let forms=enumerate();
    let text=render(&forms);
    if check{
        let current=std::fs::read_to_string(path).map_err(|e|format!("{path}: {e}"))?;
        if current.replace("\r\n","\n")!=text{return Err(format!("{path} is out of date: run pil --emit-precompiled-forms {path}"));}
        return Ok(format!("{path} is current: {} forms",forms.len()));
    }
    std::fs::write(path,&text).map_err(|e|format!("{path}: {e}"))?;
    Ok(format!("wrote {} forms ({} bytes) to {path}",forms.len(),text.len()))
}

