//! PIL proof: text -> validated typed tokens -> register/flag interpreter.
//! Token interpreter, small 386 decoder and Rust-source emitter; no external crates.
use std::collections::HashMap;
mod flow;
mod memory;
mod address;
use memory::{Ram, MemWidth};
mod serial;
mod debugger;
mod guest;
mod cache;
mod native;
mod trace;
mod mandelbrot;
use flow::{ExecError, FlowContext};

type R<T> = Result<T, String>;
const ARITH_FLAGS: u32 = 0x8d5;

#[derive(Clone, Copy, Debug)]
struct Value { bits: u64, width: u8 }
fn mask(w: u8) -> u64 { if w == 64 { u64::MAX } else { (1u64 << w) - 1 } }
fn value(bits: u64, width: u8) -> Value { Value { bits: bits & mask(width), width } }

#[derive(Clone, Copy, Debug)]
enum Binding { Dst, Src }
#[derive(Clone, Copy, Debug)]
enum Shift { Left, Right, Arithmetic }
#[derive(Clone, Copy, Debug)]
enum Binary { Add, Sub, Mul, Xor, And, Or }
#[derive(Clone, Copy, Debug)]
enum Flag { CF, PF, AF, ZF, SF, OF }
impl Flag {
    fn bit(self) -> u8 { match self {
        Self::CF => 0, Self::PF => 2, Self::AF => 4,
        Self::ZF => 6, Self::SF => 7, Self::OF => 11,
    }}
}
fn parse_flag(name: &str) -> R<Flag> {
    match name {
        "CF" => Ok(Flag::CF), "PF" => Ok(Flag::PF), "AF" => Ok(Flag::AF),
        "ZF" => Ok(Flag::ZF), "SF" => Ok(Flag::SF), "OF" => Ok(Flag::OF),
        _ => Err(format!("unknown flag: {name}")),
    }
}
#[derive(Clone, Debug)]
enum Token {
    Read(usize, Binding, u8),
    ReadFlag(usize, Flag),
    ReadAddress(usize),
    Load(usize, usize, MemWidth),
    Store(usize, usize, MemWidth),
    ReadIp(usize, bool, u8),
    ReadDisp(usize, u8),
    BranchIf(usize, usize),
    PreserveFlags(u32),
    UndefFlag(Flag),
    Select(usize, usize, usize, usize),
    WriteIf(usize, usize),
    Const(usize, Value),
    Extend(usize, usize, u8),
    SignExtend(usize, usize, u8),
    Truncate(usize, usize, u8),
    Binary(usize, Binary, usize, usize),
    Shift(usize, Shift, usize, usize),
    Less(usize, usize, usize),
    FlagIf(Flag, usize, usize, usize),
    Not(usize, usize),
    Bit(usize, usize, u8),
    Zero(usize, usize),
    Parity(usize, usize),
    Write(usize),
    Flag(Flag, usize),
    Retire,
}
#[derive(Clone, Debug)]
struct Program { tokens: Vec<Token>, widths: Vec<u8>, operand_width: u8, uses_flow: bool, uses_displacement: bool, uses_memory: bool }

fn number(s: &str) -> R<u64> {
    if let Some(hex) = s.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| format!("bad number: {s}"))
    } else { s.parse().map_err(|_| format!("bad number: {s}")) }
}
fn width(s: &str) -> R<u8> {
    let w = number(s)?;
    if (1..=64).contains(&w) { Ok(w as u8) } else { Err("width must be 1..64".into()) }
}
fn slot(names: &HashMap<String, usize>, s: &str) -> R<usize> {
    names.get(s).copied().ok_or_else(|| format!("undefined value: {s}"))
}
fn define(names: &mut HashMap<String, usize>, widths: &mut Vec<u8>, s: &str, w: u8) -> R<usize> {
    if names.contains_key(s) { return Err(format!("duplicate value: {s}")); }
    let id = widths.len();
    widths.push(w); names.insert(s.to_owned(), id); Ok(id)
}
fn compile(source: &str, operand_width: u8) -> R<Program> {
    if ![8,16,32].contains(&operand_width) { return Err("operand width must be 8,16,32".into()); }
    let mut names = HashMap::new();
    let mut widths = Vec::new();
    let mut tokens = Vec::new();
    let mut retired = false;
    let mut wrote = false;
    let mut branched = false;
    let mut stored = false;
    let mut flags_seen = 0u32;
    for (line_no, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }
        let fields: Vec<_> = line.split_whitespace().collect();
        let parsed: R<Token> = (|| {
            if retired { return Err("tokens after retire".into()); }
            let t = match fields.as_slice() {
                ["require_width", required] => {
                    let available=if *required=="16_or_32" { [16,32].contains(&operand_width) }
                        else {width(required)?==operand_width};
                    if !available { return Err("instruction width not available".into()); }
                    Token::PreserveFlags(0)
                }
                ["preserve_flag", name] => {
                    let f=parse_flag(name)?; let m=1u32<<f.bit();
                    if flags_seen&m!=0 {return Err("duplicate flag classification".into());}
                    flags_seen|=m; Token::PreserveFlags(m)
                }
                ["preserve_flags", what] => {
                    let m=match *what {
                        "all" => ARITH_FLAGS,
                        "remaining" => ARITH_FLAGS & !flags_seen,
                        _ => return Err("expected all or remaining".into()),
                    };
                    if flags_seen&m!=0 {return Err("duplicate flag classification".into());}
                    flags_seen|=m; Token::PreserveFlags(m)
                }
                ["undef_flag", name] => {
                    let f=parse_flag(name)?; let m=1u32<<f.bit();
                    if flags_seen&m!=0 {return Err("duplicate flag classification".into());}
                    flags_seen|=m; Token::UndefFlag(f)
                }
                ["flag_if", name, input, defined, active] => {
                    let f=parse_flag(name)?;let m=1u32<<f.bit();
                    if flags_seen&m!=0{return Err("duplicate flag classification".into());}
                    let a=slot(&names,input)?;let d=slot(&names,defined)?;let c=slot(&names,active)?;
                    if [a,d,c].iter().any(|&n|widths[n]!=1){return Err("flag_if requires three bv<1> inputs".into());}
                    flags_seen|=m;Token::FlagIf(f,a,d,c)
                }
                [op @ ("shl"|"lshr"|"ashr"),out,input,count]=>{
                    let a=slot(&names,input)?;let b=slot(&names,count)?;let w=widths[a];
                    let kind=match *op{"shl"=>Shift::Left,"lshr"=>Shift::Right,_=>Shift::Arithmetic};
                    Token::Shift(define(&mut names,&mut widths,out,w)?,kind,a,b)
                }
                ["ult",out,left,right]=>{
                    let a=slot(&names,left)?;let b=slot(&names,right)?;
                    if widths[a]!=widths[b]{return Err("ult width mismatch".into());}
                    Token::Less(define(&mut names,&mut widths,out,1)?,a,b)
                }
                ["select", out, predicate, yes, no] => {
                    let c=slot(&names,predicate)?; let a=slot(&names,yes)?; let b=slot(&names,no)?;
                    if widths[c]!=1 {return Err("select predicate needs bv<1>".into());}
                    if widths[a]!=widths[b] {return Err("select arm width mismatch".into());}
                    let w=widths[a];
                    Token::Select(define(&mut names,&mut widths,out,w)?,c,a,b)
                }
                ["write_if", "dst", input, predicate] => {
                    let a=slot(&names,input)?; let c=slot(&names,predicate)?;
                    if widths[a]!=operand_width {return Err("write width mismatch".into());}
                    if widths[c]!=1 {return Err("write_if predicate needs bv<1>".into());}
                    if wrote {return Err("multiple destination writes".into());}
                    wrote=true; Token::WriteIf(a,c)
                }
                ["read_address", out] => Token::ReadAddress(define(&mut names,&mut widths,out,32)?),
                ["load", out, address, bits] => {
                    if stored {return Err("loads after a staged store are not supported".into());}
                    let a=slot(&names,address)?;
                    if widths[a]!=32 {return Err("memory address must be bv<32>".into());}
                    let w=width(bits)?;let mem=MemWidth::from_bits(w).ok_or("memory width must be 8,16,32")?;
                    Token::Load(define(&mut names,&mut widths,out,w)?,a,mem)
                }
                ["store", address, input] => {
                    let a=slot(&names,address)?;let v=slot(&names,input)?;
                    if widths[a]!=32 {return Err("memory address must be bv<32>".into());}
                    let mem=MemWidth::from_bits(widths[v]).ok_or("memory width must be 8,16,32")?;
                    if stored {return Err("only one staged RAM store is supported".into());}
                    stored=true;Token::Store(a,v,mem)
                }
                ["read_ip", out, which, w] => {
                    let next=match *which {"next"=>true,"current"=>false,_=>return Err("read_ip needs current or next".into())};
                    let w=width(w)?;
                    Token::ReadIp(define(&mut names,&mut widths,out,w)?,next,w)
                }
                ["read_disp", out, w] => {
                    let w=width(w)?;
                    if w!=operand_width || ![16,32].contains(&w) {return Err("read_disp needs the 16/32-bit operand width".into());}
                    Token::ReadDisp(define(&mut names,&mut widths,out,w)?,w)
                }
                ["branch_if", target, predicate] => {
                    let a=slot(&names,target)?;let c=slot(&names,predicate)?;
                    if widths[a]!=32 || widths[c]!=1 {return Err("branch_if requires bv<32> target and bv<1> predicate".into());}
                    if branched {return Err("multiple branch effects".into());}
                    if ![16,32].contains(&operand_width) {return Err("branch operand width must be 16 or 32".into());}
                    branched=true;Token::BranchIf(a,c)
                }
                ["read", out, binding, w] => {
                    let w = width(w)?;
                    if w != operand_width { return Err("binding width mismatch".into()); }
                    let b = match *binding { "dst" => Binding::Dst, "src" => Binding::Src,
                        _ => return Err("expected dst or src".into()) };
                    Token::Read(define(&mut names, &mut widths, out, w)?, b, w)
                }
                ["const", out, w, n] => {
                    let w = width(w)?; let n = number(n)?;
                    if n & !mask(w) != 0 { return Err("constant does not fit".into()); }
                    Token::Const(define(&mut names, &mut widths, out, w)?, value(n,w))
                }
                ["read_flag", out, name] => {
                    let f = parse_flag(name)?;
                    Token::ReadFlag(define(&mut names, &mut widths, out, 1)?,f)
                }
                ["zext", out, input, w] | ["sext", out, input, w] | ["trunc", out, input, w] => {
                    let a = slot(&names, input)?; let w = width(w)?;
                    let extend = fields[0] != "trunc";
                    if (extend && w < widths[a]) || (!extend && w > widths[a]) {
                        return Err("invalid conversion direction".into());
                    }
                    let o = define(&mut names, &mut widths, out, w)?;
                    if fields[0]=="sext" {Token::SignExtend(o,a,w)} else if extend { Token::Extend(o,a,w) } else { Token::Truncate(o,a,w) }
                }
                [op @ ("add" | "sub" | "mul" | "xor" | "and" | "or"), out, left, right] => {
                    let a = slot(&names,left)?; let b = slot(&names,right)?;
                    if widths[a] != widths[b] { return Err("binary width mismatch".into()); }
                    let w = widths[a];
                    let o = define(&mut names, &mut widths, out, w)?;
                    let op = match *op { "add" => Binary::Add, "sub" => Binary::Sub,
                        "mul" => Binary::Mul, "xor" => Binary::Xor, "or" => Binary::Or, _ => Binary::And };
                    Token::Binary(o,op,a,b)
                }
                ["not", out, input] => {
                    let a = slot(&names,input)?; let w = widths[a];
                    Token::Not(define(&mut names, &mut widths, out, w)?,a)
                }
                ["bit", out, input, index] => {
                    let a = slot(&names,input)?; let i = number(index)?;
                    if i >= widths[a] as u64 { return Err("bit index out of range".into()); }
                    Token::Bit(define(&mut names, &mut widths, out, 1)?,a,i as u8)
                }
                ["zero", out, input] | ["parity", out, input] => {
                    let a = slot(&names,input)?;
                    if fields[0] == "parity" && widths[a] < 8 { return Err("parity requires >=8 bits".into()); }
                    let o = define(&mut names, &mut widths, out, 1)?;
                    if fields[0] == "zero" { Token::Zero(o,a) } else { Token::Parity(o,a) }
                }
                ["write", "dst", input] => {
                    let a = slot(&names,input)?;
                    if widths[a] != operand_width { return Err("write width mismatch".into()); }
                    if wrote { return Err("multiple destination writes".into()); }
                    wrote = true; Token::Write(a)
                }
                ["flag", name, input] => {
                    let a = slot(&names,input)?;
                    if widths[a] != 1 { return Err("flag needs bv<1>".into()); }
                    let f = parse_flag(name)?;
                    if flags_seen & (1 << f.bit()) != 0 { return Err("duplicate flag write".into()); }
                    flags_seen |= 1 << f.bit(); Token::Flag(f,a)
                }
                ["retire"] => { retired = true; Token::Retire }
                _ => return Err("unknown token or wrong operand count".into()),
            }; Ok(t)
        })();
        tokens.push(parsed.map_err(|e| format!("line {}: {e}",line_no+1))?);
    }
    if !retired { return Err("missing retire".into()); }
    if flags_seen != ARITH_FLAGS { return Err("classify all six arithmetic flags: flag, preserve_flag, preserve_flags or undef_flag".into()); }
    let uses_flow=tokens.iter().any(|t| matches!(t,Token::ReadIp(..)|Token::ReadDisp(..)|Token::BranchIf(..)));
    let uses_displacement=tokens.iter().any(|t| matches!(t,Token::ReadDisp(..)));
    let uses_memory=tokens.iter().any(|t|matches!(t,Token::ReadAddress(..)|Token::Load(..)|Token::Store(..)));
    Ok(Program { tokens, widths, operand_width, uses_flow, uses_displacement, uses_memory })
}

#[derive(Clone, Copy, Debug)]
struct View { register: usize, low: u8, width: u8 }
impl View {
    fn validate(self, w: u8) -> R<()> {
        if self.register >= 8 || self.width != w || self.low as u16 + w as u16 > 32 {
            return Err("invalid register binding".into());
        } Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct Cpu { regs: [u32;8], eflags: u32, undefined_flags: u32, eip: u32 }
impl Cpu {
    fn read(&self, v: View) -> Value { value((self.regs[v.register] as u64) >> v.low,v.width) }
    fn write(&mut self, v: View, x: Value) {
        let m = mask(v.width) << v.low;
        self.regs[v.register] = (((self.regs[v.register] as u64) & !m) | ((x.bits << v.low) & m)) as u32;
    }
}
fn execute(p: &Program, cpu: &mut Cpu, dst: View, src: View) -> R<()> {
    if p.uses_flow {return Err("flow context required: use execute_flow".into());}
    if p.uses_memory {return Err("RAM context required: use execute_memory".into());}
    let context=FlowContext{next_eip:cpu.eip,displacement:0,displacement_bits:32,code_limit:u32::MAX};
    execute_flow(p,cpu,dst,src,context).map_err(|e|format!("{e:?}"))
}
fn execute_flow(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext)->Result<(),ExecError> {
    if p.uses_memory {return Err(ExecError::InvalidContext);}
    execute_state(p,cpu,dst,src,context,None,0,None)
}
fn execute_memory(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError> {
    execute_state(p,cpu,dst,src,context,Some(ram),address,None)
}
fn execute_segmented(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError> {
    let (offset, memory_context)=address.resolve(&cpu.regs)?;
    execute_state(p,cpu,dst,src,context,Some(ram),offset,Some(memory_context))
}
fn access_error(eip:u32,f:memory::AccessFault)->ExecError {
    ExecError::MemoryAccess{eip,address:f.address,bytes:f.bytes as u8,write:f.write}
}
fn execute_state(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,mut ram:Option<&mut Ram>,address:u32,memory_context:Option<address::MemoryContext>)->Result<(),ExecError> {
    if p.uses_memory && ram.is_none() {return Err(ExecError::InvalidContext);}
    dst.validate(p.operand_width).map_err(|_|ExecError::InvalidBinding)?;
    src.validate(p.operand_width).map_err(|_|ExecError::InvalidBinding)?;
    context.validate(p.operand_width,p.uses_displacement)?;
    let mut pending_eip=context.next_eip;
    let mut pending_store:Option<(u32,MemWidth,u32)>=None;
    let mut values: Vec<Value> = p.widths.iter().map(|&w| value(0,w)).collect();
    let mut pending_dst = None;
    let mut pending_flags = cpu.eflags;
    let mut pending_undefined = cpu.undefined_flags;
    for t in &p.tokens {
        match *t {
            Token::Read(o,b,w) => { let v = match b { Binding::Dst => dst, Binding::Src => src };
                values[o] = value(cpu.read(v).bits,w); }
            Token::ReadAddress(o)=>values[o]=value(address as u64,32),
            Token::Load(o,a,w)=>{
                let m=ram.as_deref_mut().ok_or(ExecError::InvalidContext)?;
                let offset=values[a].bits as u32;
                let addr=match memory_context {Some(c)=>c.translate(offset,w,false,cpu.eip)?,None=>offset};
                let v=m.load(addr,w).map_err(|f|access_error(cpu.eip,f))?;
                values[o]=value(v as u64,w.bits());
            }
            Token::Store(a,v,w)=>{
                let m=ram.as_deref_mut().ok_or(ExecError::InvalidContext)?;
                let offset=values[a].bits as u32;
                let addr=match memory_context {Some(c)=>c.translate(offset,w,true,cpu.eip)?,None=>offset};
                m.check_write(addr,w).map_err(|f|access_error(cpu.eip,f))?;
                pending_store=Some((addr,w,values[v].bits as u32));
            }
            Token::ReadIp(o,next,w) => values[o]=value((if next {context.next_eip} else {cpu.eip}) as u64,w),
            Token::ReadDisp(o,w) => values[o]=value(context.displacement as u32 as u64,w),
            Token::BranchIf(a,c) => {
                if values[c].bits!=0 {
                    let target=values[a].bits as u32;
                    if target>context.code_limit {return Err(ExecError::GeneralProtection{eip:cpu.eip,error_code:0});}
                    pending_eip=target;
                }
            }
            Token::PreserveFlags(_mask) => {},
            Token::UndefFlag(f) => {
                pending_flags &= !(1u32<<f.bit()); // deterministic zero policy, still undefined
                pending_undefined |= 1u32<<f.bit();
            }
            Token::Select(o,c,a,b) => values[o]=if values[c].bits!=0 {values[a]} else {values[b]},
            Token::WriteIf(a,c) => {if values[c].bits!=0 {pending_dst=Some(values[a]);}},
            Token::ReadFlag(o,f) => values[o] = value(((cpu.eflags >> f.bit()) & 1) as u64,1),
            Token::Const(o,x) => values[o] = x,
            Token::Extend(o,a,w) | Token::Truncate(o,a,w) => values[o] = value(values[a].bits,w),
            Token::SignExtend(o,a,w) => {
                let x=values[a];let bits=if x.bits&(1u64<<(x.width-1))!=0{x.bits|!mask(x.width)}else{x.bits};
                values[o]=value(bits,w);
            }
            Token::Binary(o,op,a,b) => {
                let x = values[a]; let y = values[b];
                let n = match op { Binary::Add => x.bits.wrapping_add(y.bits),
                    Binary::Sub => x.bits.wrapping_sub(y.bits),
                    Binary::Mul => x.bits.wrapping_mul(y.bits),
                    Binary::Xor => x.bits ^ y.bits, Binary::And => x.bits & y.bits, Binary::Or => x.bits | y.bits };
                values[o] = value(n,x.width);
            }
            Token::Shift(o,kind,a,b)=>{
                let x=values[a];let c=values[b].bits;
                let n=match kind {
                    Shift::Left=>if c>=u64::from(x.width){0}else{x.bits<<c},
                    Shift::Right=>if c>=u64::from(x.width){0}else{x.bits>>c},
                    Shift::Arithmetic=>{let signed=((x.bits<<(64-x.width)) as i64)>>(64-x.width);(signed>>c.min(u64::from(x.width-1))) as u64},
                };values[o]=value(n,x.width);
            }
            Token::Less(o,a,b)=>values[o]=value(u64::from(values[a].bits<values[b].bits),1),
            Token::FlagIf(f,a,d,c)=>if values[c].bits!=0{
                let m=1u32<<f.bit();pending_flags&=!m;
                if values[d].bits!=0{pending_flags|=(values[a].bits as u32)<<f.bit();pending_undefined&=!m;}else{pending_undefined|=m;}
            },
            Token::Not(o,a) => values[o] = value(!values[a].bits,values[a].width),
            Token::Bit(o,a,i) => values[o] = value((values[a].bits >> i) & 1,1),
            Token::Zero(o,a) => values[o] = value(u64::from(values[a].bits == 0),1),
            Token::Parity(o,a) => values[o] = value(u64::from((values[a].bits as u8).count_ones() % 2 == 0),1),
            Token::Write(a) => pending_dst = Some(values[a]),
            Token::Flag(f,a) => { let m = 1u32 << f.bit();
                pending_undefined &= !m;
                pending_flags = (pending_flags & !m) | ((values[a].bits as u32) << f.bit()); }
            Token::Retire => {
                if let Some((addr,w,v))=pending_store {
                    ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|access_error(cpu.eip,f))?;
                }
                if let Some(x) = pending_dst { cpu.write(dst,x); }
                cpu.eflags = pending_flags;
                cpu.undefined_flags = pending_undefined;
                cpu.eip = pending_eip;
                return Ok(());
            }
        }
    }
    Err(ExecError::InvalidContext)
}

const ADD: &str = include_str!("../isa/i386/add.pil");
const SUB: &str = include_str!("../isa/i386/sub.pil");
const CMP: &str = include_str!("../isa/i386/cmp.pil");
const ADC: &str = include_str!("../isa/i386/adc.pil");
const SBB: &str = include_str!("../isa/i386/sbb.pil");
const AND_DESC: &str = include_str!("../isa/i386/and.pil");
const CONDITIONS: &str = include_str!("../isa/i386/conditions.pil");
const SETCC: &str = include_str!("../isa/i386/setcc.pil");
const LOAD_RAM: &str = include_str!("../isa/i386/load_ram.pil");
const STORE_RAM: &str = include_str!("../isa/i386/store_ram.pil");
const ADD_RAM: &str = include_str!("../isa/i386/add_ram.pil");
const JCC: &str = include_str!("../isa/i386/jcc.pil");
const WRITE_IF_Z: &str = include_str!("../isa/i386/write_if_z.pil");
const CONDITIONS_NAMES: [&str;16] = ["o","no","b","ae","e","ne","be","a","s","ns","p","np","l","ge","le","g"];
fn description(name: &str) -> R<String> {
    let plain=match name {
        "add" => Some(ADD), "sub" => Some(SUB), "cmp" => Some(CMP),
        "or" => Some(include_str!("../isa/i386/or.pil")),
        "xor" => Some(include_str!("../isa/i386/xor.pil")),
        "test" => Some(include_str!("../isa/i386/test.pil")),
        "not" => Some(include_str!("../isa/i386/not.pil")),
        "neg" => Some(include_str!("../isa/i386/neg.pil")),
        "inc" => Some(include_str!("../isa/i386/inc.pil")),
        "dec" => Some(include_str!("../isa/i386/dec.pil")),
        "nop" => Some(include_str!("../isa/i386/nop.pil")),
        "imul" => Some(include_str!("../isa/i386/imul.pil")),
        "jmp" => Some(include_str!("../isa/i386/jmp.pil")),
        "shl"|"sal" => Some(include_str!("../isa/i386/shl.pil")),
        "shr" => Some(include_str!("../isa/i386/shr.pil")),
        "sar" => Some(include_str!("../isa/i386/sar.pil")),
        "adc" => Some(ADC), "sbb" => Some(SBB), "and" => Some(AND_DESC),
        "write_if_z" => Some(WRITE_IF_Z), "load_ram"=>Some(LOAD_RAM),
        "store_ram"=>Some(STORE_RAM), "add_ram"=>Some(ADD_RAM), _ => None,
    };
    if let Some(text)=plain {return Ok(text.to_owned());}
    if let Some(condition)=name.strip_prefix("set") {
        if CONDITIONS_NAMES.contains(&condition) {
            return Ok(format!("{}\n{}",CONDITIONS,SETCC.replace("$CONDITION", &format!("cond_{condition}"))));
        }
    }
    if let Some(condition)=name.strip_prefix('j') {
        if CONDITIONS_NAMES.contains(&condition) {
            return Ok(format!("{}\n{}",CONDITIONS,JCC.replace("$CONDITION", &format!("cond_{condition}"))));
        }
    }
    Err("unknown description: use add/sub/cmp/adc/sbb/and/or/xor/test/not/neg/inc/dec/nop/imul/shl/sal/shr/sar, set<condition>, j<condition>, or write_if_z".into())
}
fn instantiate(text: impl AsRef<str>, w: u8) -> String {
    text.as_ref().replace("$DOUBLE",&(2*w as u16).to_string()).replace("$EXT",&(w+1).to_string()).replace("$SIGN",&(w-1).to_string()).replace("$W",&w.to_string())
}
fn emit_rust(p: &Program) -> String {
    let mut out=String::from("// Generated from validated PIL tokens.\n");
    out.push_str(include_str!("flow.rs"));
    // The included module has an inner documentation comment; in the emitted
    // standalone body it must be an ordinary comment.
    out.push_str("\n");out.push_str(include_str!("memory.rs").split("#[cfg(test)]").next().unwrap());
    out.push_str("\npub mod address {\n");
    out.push_str(include_str!("address.rs").split("#[cfg(test)]").next().unwrap());
    out.push_str("\n}\n");
    out=out.lines().map(|line|line.strip_prefix("//!").map(|s|format!("//{s}")).unwrap_or_else(||line.to_owned())).collect::<Vec<_>>().join("\n");
    out.push_str(&emit_rust_functions(p));out
}
// Shared support types can be supplied by a compiled guest runner instead of
// duplicated in each generated instruction module.
fn emit_rust_functions(p:&Program)->String {
    let mut out=String::new();
    out.push_str(&format!(
        "\nfn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>) -> Result<(), ExecError> {{\n\
         let w = {}u8;\n\
         if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + w as u16 > 32 {{ return Err(ExecError::InvalidBinding); }}\n\
         context.validate(w,{})?;\n\
         if {} && ram.is_none() {{ return Err(ExecError::InvalidContext); }}\n\
         let mut pending_store: Option<(u32,MemWidth,u32)> = None;\n\
         let mut pending_eip = context.next_eip;\n\
         let mut pending_flags = *flags;\n\
         let mut pending_undefined = *undefined;\n\
         let mut pending_dst: Option<u64> = None;\n",
        p.operand_width,p.uses_displacement,p.uses_memory));
    for t in &p.tokens {
        let line = match *t {
            Token::Read(o,b,w) => {
                let binding = match b { Binding::Dst => "dst", Binding::Src => "src" };
                format!("let v{o}: u64 = ((regs[{binding}.0] as u64) >> {binding}.1) & {}u64;",mask(w))
            }
            Token::ReadAddress(o)=>format!("let v{o}: u64 = address as u64;"),
            Token::Load(o,a,w)=>format!("let addr{o}=match memory_context {{Some(c)=>c.translate(v{a} as u32,MemWidth::{w:?},false,*eip)?,None=>v{a} as u32}}; let v{o}: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr{o},MemWidth::{w:?}).map_err(|f|ExecError::MemoryAccess{{eip:*eip,address:f.address,bytes:f.bytes as u8,write:f.write}})? as u64;"),
            Token::Store(a,v,w)=>format!("let store_addr=match memory_context {{Some(c)=>c.translate(v{a} as u32,MemWidth::{w:?},true,*eip)?,None=>v{a} as u32}}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::{w:?}).map_err(|f|ExecError::MemoryAccess{{eip:*eip,address:f.address,bytes:f.bytes as u8,write:f.write}})?; pending_store=Some((store_addr,MemWidth::{w:?},v{v} as u32));"),
            Token::ReadIp(o,next,w) => format!("let v{o}: u64 = ({} as u64) & {}u64;",if next {"context.next_eip"} else {"*eip"},mask(w)),
            Token::ReadDisp(o,w) => format!("let v{o}: u64 = (context.displacement as u32 as u64) & {}u64;",mask(w)),
            Token::BranchIf(a,c) => format!("if v{c} != 0 {{ if v{a} as u32 > context.code_limit {{ return Err(ExecError::GeneralProtection{{eip:*eip,error_code:0}}); }} pending_eip = v{a} as u32; }}"),
            Token::Const(o,x) => format!("let v{o}: u64 = {}u64;",x.bits),
            Token::PreserveFlags(m) => format!("// Preserve flag mask {m:#x}, including undefined status."),
            Token::UndefFlag(f) => format!("pending_flags &= !{}u32; pending_undefined |= {}u32;",1u32<<f.bit(),1u32<<f.bit()),
            Token::Select(o,c,a,b) => format!("let v{o}: u64 = if v{c} != 0 {{ v{a} }} else {{ v{b} }};"),
            Token::WriteIf(a,c) => format!("if v{c} != 0 {{ pending_dst = Some(v{a}); }}"),
            Token::ReadFlag(o,f) => format!("let v{o}: u64 = ((*flags >> {}) & 1) as u64;",f.bit()),
            Token::Extend(o,a,w) | Token::Truncate(o,a,w) =>
                format!("let v{o}: u64 = v{a} & {}u64;",mask(w)),
            Token::SignExtend(o,a,w) => format!("let v{o}: u64 = (if v{a} & {}u64 != 0 {{v{a} | !{}u64}} else {{v{a}}}) & {}u64;",1u64<<(p.widths[a]-1),mask(p.widths[a]),mask(w)),
            Token::Binary(o,op,a,b) => {
                let expr = match op { Binary::Add => format!("v{a}.wrapping_add(v{b})"),
                    Binary::Sub => format!("v{a}.wrapping_sub(v{b})"),
                    Binary::Mul => format!("v{a}.wrapping_mul(v{b})"),
                    Binary::Xor => format!("v{a} ^ v{b}"), Binary::And => format!("v{a} & v{b}"), Binary::Or => format!("v{a} | v{b}") };
                format!("let v{o}: u64 = ({expr}) & {}u64;",mask(p.widths[o]))
            }
            Token::Shift(o,kind,a,b)=>{
                let w=p.widths[a];let expr=match kind {
                    Shift::Left=>format!("if v{b}>={w}u64 {{0}}else{{v{a}<<v{b}}}"),
                    Shift::Right=>format!("if v{b}>={w}u64 {{0}}else{{v{a}>>v{b}}}"),
                    Shift::Arithmetic=>format!("((((v{a}<<{}) as i64)>>{})>>v{b}.min({}u64)) as u64",64-w,64-w,w-1),
                };format!("let v{o}: u64 = ({expr}) & {}u64;",mask(w))
            }
            Token::Less(o,a,b)=>format!("let v{o}: u64 = u64::from(v{a}<v{b});"),
            Token::FlagIf(f,a,d,c)=>format!("if v{c}!=0 {{pending_flags &= !{m}u32; if v{d}!=0 {{pending_flags |= (v{a} as u32)<<{bit};pending_undefined &= !{m}u32;}}else{{pending_undefined |= {m}u32;}}}}",m=1u32<<f.bit(),bit=f.bit()),
            Token::Not(o,a) => format!("let v{o}: u64 = (!v{a}) & {}u64;",mask(p.widths[o])),
            Token::Bit(o,a,i) => format!("let v{o}: u64 = (v{a} >> {i}) & 1;"),
            Token::Zero(o,a) => format!("let v{o}: u64 = u64::from(v{a} == 0);"),
            Token::Parity(o,a) => format!("let v{o}: u64 = u64::from((v{a} as u8).count_ones() % 2 == 0);"),
            Token::Write(a) => format!("pending_dst = Some(v{a});"),
            Token::Flag(f,a) => format!("pending_undefined &= !{}u32; pending_flags = (pending_flags & !{}u32) | ((v{a} as u32) << {});",1u32 << f.bit(),1u32 << f.bit(),f.bit()),
            Token::Retire => format!(
                "if let Some((addr,w,v))=pending_store {{ ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|ExecError::MemoryAccess{{eip:*eip,address:f.address,bytes:f.bytes as u8,write:f.write}})?; }}\nif let Some(value) = pending_dst {{ let m = {}u64 << dst.1;\nregs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }}\n*flags = pending_flags;\n*undefined = pending_undefined;\n*eip = pending_eip;\nOk(())",mask(p.operand_width)),
        };
        out.push_str(&line); out.push('\n');
    }
    out.push_str("}\n");
    out.push_str("pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{\n");
    if p.uses_memory {out.push_str("Err(ExecError::InvalidContext)\n}\n");}
    else {out.push_str("execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None)\n}\n");}
    out.push_str("pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{\nexecute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None)\n}\n");
    out.push_str("pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{\nlet (offset,memory_context)=address.resolve(regs)?;\nexecute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context))\n}\n");
    out.push_str("pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {\n");
    if p.uses_flow || p.uses_memory {out.push_str("Err(\"flow context required: use execute_with_flow\")\n");}
    else {out.push_str("let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};\nexecute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|\"invalid execution context\")\n");}
    out.push_str("}\n// Compatibility helper: discards diagnostic undefined-flag metadata.\npub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {\nlet mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)\n}\n"); out
}
fn main() -> R<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str)==Some("--mandelbrot") {
        if args.len()>4{return Err("usage: pil-proof --mandelbrot [size 2..256 power of two] [iterations 1..64]".into());}
        let parse=|i:usize,default:u32|->R<u32>{if let Some(x)=args.get(i){u32::try_from(number(x.as_str())?).map_err(|_|"parameter out of range".into())}else{Ok(default)}};
        println!("{}",mandelbrot::render_files(parse(2,256)?,parse(3,32)?)?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--verify-backends") {
        if args.len()!=2{return Err("usage: pil-proof --verify-backends".into());}
        println!("{}",native::verify()?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--cpu-bench") {
        if args.len()>4{return Err("usage: pil-proof --cpu-bench [iterations] [samples]".into());}
        let n=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{10000};
        let samples=if let Some(x)=args.get(3){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{5};
        println!("{}",native::cpu_benchmark(n,samples)?);return Ok(());
    }
    if matches!(args.get(1).map(String::as_str),Some("--compare-backends"|"--emit-suite-rust")) {
        if args.len()>4{return Err("usage: pil-proof --compare-backends|--emit-suite-rust [iterations] [samples]".into());}
        let n=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{10000};
        let samples=if let Some(x)=args.get(3){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{5};
        if args[1]=="--emit-suite-rust"{print!("{}",native::emit_suite(n,samples)?);}else{println!("{}",native::compare(n,samples)?);}
        return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-factorials") || args.get(1).map(String::as_str)==Some("--write-loop") {
        let path=args.get(2).ok_or("usage: pil-proof --write-factorials|--write-loop PATH")?;
        let bytes=if args[1]=="--write-factorials"{guest::factorial_bytes()?}else{guest::loop_bytes(guest::LOOP_ITERATIONS)?};
        std::fs::write(path,bytes).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--numeric-test") {return debugger::numeric_test();}
    if args.get(1).map(String::as_str)==Some("--bench") {
        let mut counts=Vec::new();let mut cached=true;
        for arg in &args[2..] {
            if arg=="--uncached" && cached {cached=false;}else{counts.push(arg);}
        }
        if counts.len()>2{return Err("usage: pil-proof --bench [iterations] [samples] [--uncached]".into());}
        let n=if let Some(x)=counts.first(){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{1000};
        let samples=if let Some(x)=counts.get(1){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{3};
        println!("{}",debugger::benchmark_mode(n,samples,cached)?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--stress") {
        if args.len()>3{return Err("usage: pil-proof --stress [passes]".into());}
        let passes=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"passes out of range")?}else{25};
        println!("{}",debugger::factorial_stress(passes)?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-hello") {
        let path=args.get(2).ok_or("usage: pil-proof --write-hello PATH")?;
        std::fs::write(path,guest::hello_bytes()?).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-selftest") {
        let path=args.get(2).ok_or("usage: pil-proof --write-selftest PATH")?;
        std::fs::write(path,guest::selftest_bytes()?).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--selftest") {
        return debugger::selftest();
    }
    if args.get(1).map(String::as_str)==Some("--repl") {
        if args.len()>2 {
            if args.len()!=4 || args[2]!="--load" {return Err("usage: pil-proof --repl [--load PATH]".into());}
            return debugger::repl_image(std::io::stdin().lock(),std::io::stdout().lock(),&args[3]).map_err(|e|e.to_string());
        }
        return debugger::repl(std::io::stdin().lock(),std::io::stdout().lock()).map_err(|e|e.to_string());
    }
    if args.get(1).map(String::as_str)==Some("--demo-branch") {
        let cmp=compile(&instantiate(CMP,32),32)?;
        let jb=compile(&instantiate(description("jb")?,32),32)?;
        let mut cpu=Cpu::default();cpu.eip=0x100;cpu.regs[0]=10;cpu.regs[1]=20;
        let dst=View{register:0,low:0,width:32};let src=View{register:1,low:0,width:32};
        execute(&cmp,&mut cpu,dst,src)?;
        execute_flow(&jb,&mut cpu,dst,src,FlowContext{next_eip:0x106,displacement:-4,displacement_bits:8,code_limit:u32::MAX}).map_err(|e|format!("{e:?}"))?;
        println!("CMP 10,20; JB -4: EIP=0x{:08x} (expected 0x00000102)",cpu.eip);
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--emit-rust") {
        let w = args.get(2).ok_or("usage: pil-proof --emit-rust 8|16|32 [description]")?;
        let w = width(w)?;
        let name = args.get(3).map(String::as_str).unwrap_or("add");
        let p = compile(&instantiate(description(name)?,w),w)?;
        print!("{}",emit_rust(&p)); return Ok(());
    }
    let p = compile(&instantiate(ADD,16),16)?;
    let mut cpu = Cpu::default(); cpu.regs[0] = 0x1234_7fff; cpu.regs[1] = 1;
    execute(&p,&mut cpu,View{register:0,low:0,width:16},View{register:1,low:0,width:16})?;
    println!("AX=0x{:04x}; EAX=0x{:08x}; flags=0x{:04x}; {} typed tokens",
        cpu.regs[0] & 0xffff,cpu.regs[0],cpu.eflags,p.tokens.len());
    Ok(())
}



