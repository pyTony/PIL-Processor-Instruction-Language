//! PIL proof: text -> validated typed tokens -> register/flag interpreter.
//! Token interpreter, small 386 decoder and Rust-source emitter; no external crates.
use std::collections::HashMap;
mod profile;
mod flow;
mod memory;
mod vga_planar;
mod address;
mod privilege;
use memory::{Ram, MemWidth};
mod serial;
mod host_io;
mod debugger;
mod guest;
mod dos;
mod dos_compile;
mod dos_encoding;
#[cfg(pil_shadow_backend)]
#[allow(dead_code, unused_mut, unused_variables, unused_imports, unused_assignments)]
mod shadow_backend { include!(concat!(env!("OUT_DIR"), "/pil_shadow.rs")); }
mod dos_config;
mod fpu;
mod containers;
mod inflate;
mod cache;
mod native;
mod trace;
mod interrupt;
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
enum Binary { Add, Sub, Mul, Div, Rem, Xor, And, Or }
#[derive(Clone, Copy, Debug)]
enum Flag { CF, PF, AF, ZF, SF, TF, IF, DF, OF, IOPL0, IOPL1, NT, VM }
impl Flag {
    fn bit(self) -> u8 { match self {
        Self::CF => 0, Self::PF => 2, Self::AF => 4,
        Self::ZF => 6, Self::SF => 7, Self::TF => 8, Self::IF => 9, Self::DF => 10, Self::OF => 11, Self::IOPL0 => 12, Self::IOPL1 => 13, Self::NT => 14, Self::VM => 17,
    }}
}
fn parse_flag(name: &str) -> R<Flag> {
    match name {
        "CF" => Ok(Flag::CF), "PF" => Ok(Flag::PF), "AF" => Ok(Flag::AF),
        "TF" => Ok(Flag::TF), "IF" => Ok(Flag::IF), "IOPL0" => Ok(Flag::IOPL0), "IOPL1" => Ok(Flag::IOPL1), "NT" => Ok(Flag::NT), "VM" => Ok(Flag::VM),
        "ZF" => Ok(Flag::ZF), "SF" => Ok(Flag::SF), "DF" => Ok(Flag::DF), "OF" => Ok(Flag::OF),
        _ => Err(format!("unknown flag: {name}")),
    }
}
#[derive(Clone, Debug)]
enum Token {
    Read(usize, Binding, u8),
    ReadFlag(usize, Flag),
    ReadFlags(usize),
    CheckpointFlags,
    RequireRealMode,
    RequireRing0,
    InterruptShadowIf(usize),
    ReadAddress(usize),
    ReadAddressAfter(usize,usize,usize),
    ReadMsw(usize),
    ReadMsw32(usize),
    ReadControl(usize,usize),
    WriteControl(usize,usize),
    WriteTable(usize,usize,usize),
    // SGDT/SIDT: limit and base of GDTR (0) or IDTR (1).
    ReadTable(usize,usize,usize),
    // SLDT/STR: the LDTR (0) or TR (1) selector.
    ReadSystem(usize,usize),
    ReadCpl(usize),
    // The raw 8-byte descriptor a selector names, and whether it was found.
    ReadDescriptor(usize,usize,usize),
    // LLDT (0) or LTR (1), checked by address.rs.
    LoadSystem(usize,usize),
    RequireProtectedMode,
    Require(usize),
    // A false predicate raises an architectural exception (only #DE, vector 0).
    RaiseUnless(usize,u8),
    LoadSegment(usize,usize,MemWidth,usize),
    StoreSegment(usize,usize,MemWidth,usize),
    RetireIf(usize),
    ResumeIf(usize),
    ReadSegment(usize,usize),
    WriteSegment(usize,usize),
    ReadGpr(usize, usize, u8),
    WriteGpr(usize, usize, u8),
    Load(usize, usize, MemWidth),
    Store(usize, usize, MemWidth),
    StoreIf(usize, usize, usize, MemWidth),
    ReadIp(usize, bool, u8),
    ReadDisp(usize, u8),
    BranchIf(usize, usize),
    FarBranch(usize,usize),
    // Far JMP, the start of a far CALL, and RETF/IRET (selector, offset,
    // outer stack offset, slot width, imm16), checked by privilege.rs.
    FarJump(usize,usize),
    FarCall(usize,usize),
    FarReturn(usize,usize,usize,MemWidth,u32,Option<usize>),
    // CLI/STI: #GP(0) in protected mode when CPL > IOPL.
    RequireIopl,
    // IN/OUT/INS/OUTS: the I/O permission check for a port and byte count.
    CheckIo(usize,u8),
    CheckSegment(usize,usize,u32),
    // Segment and RAM write checks without staging or performing a store.
    CheckWriteSegment(usize,usize,MemWidth),
    PreserveFlags(u32),
    UndefFlag(Flag),
    Select(usize, usize, usize, usize),
    WriteIf(usize, usize),
    WriteSource(usize),
    WriteSourceIf(usize,usize),
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
enum PackedOp {
    Token(Token),
    ReadDstSrc { a: usize, b: usize, w: u8 },
    ReadDstConst { a: usize, b: usize, w: u8, c: Value },
    Conditions16,
    Jcc16 { cond: u8, w: u8 },
    RelJmp { w: u8 },
    AddCore { a: usize, b: usize, r: usize, cf: usize, of: usize, af: usize, sf: usize, zf: usize, pf: usize, w: u8 },
    SubCore { a: usize, b: usize, r: usize, cf: usize, of: usize, af: usize, sf: usize, zf: usize, pf: usize, w: u8 },
    LogicCore { clear: usize, sf: usize, zf: usize, pf: usize, r: usize, sign: u8 },
    Flags6 { cf: usize, pf: usize, af: usize, zf: usize, sf: usize, of: usize },
    Flags5NoCf { pf: usize, af: usize, zf: usize, sf: usize, of: usize },
    LogicFlags { clear: usize, sf: usize, zf: usize, pf: usize },
    AddLazy { a: usize, b: usize, r: usize, w: u8, write_cf: bool },
    SubLazy { a: usize, b: usize, r: usize, w: u8, write_cf: bool },
    LogicLazy { r: usize, w: u8 },
    ShiftSetup { b: usize, count: usize, width_slot: usize, yes: usize, no: usize, active: usize, single: usize, last: usize, w: u8 },
    ShiftFlags { cf: usize, cf_defined: usize, of: usize, single: usize, sf: usize, zf: usize, pf: usize, active: usize },
    ImulCore { a: usize, b: usize, r: usize, overflow: usize, w: u8, dbl: u8 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum LazyFlags {
    #[default]
    None,
    Add { a: u64, b: u64, r: u64, w: u8, write_cf: bool },
    Sub { a: u64, b: u64, r: u64, w: u8, write_cf: bool },
    Logic { r: u64, w: u8 },
}
impl LazyFlags {
    #[inline]
    pub fn flush(&mut self, eflags: &mut u32, undefined_flags: &mut u32) {
        match *self {
            Self::None => {}
            Self::Add { a, b, r, w, write_cf } => {
                let sign = w - 1;
                let ab = a ^ b;
                let of = (((!ab & (a ^ r)) >> sign) & 1) as u32;
                let af = (((ab ^ r) >> 4) & 1) as u32;
                let sf = ((r >> sign) & 1) as u32;
                let zf = u32::from(r == 0);
                let pf = u32::from((r as u8).count_ones() & 1 == 0);
                let bits = (pf << 2) | (af << 4) | (zf << 6) | (sf << 7) | (of << 11);
                if write_cf {
                    let cf = (((a + b) >> w) & 1) as u32;
                    *undefined_flags &= !ARITH_FLAGS;
                    *eflags = (*eflags & !ARITH_FLAGS) | bits | cf;
                } else {
                    const NO_CF: u32 = ARITH_FLAGS & !1;
                    *undefined_flags &= !NO_CF;
                    *eflags = (*eflags & !NO_CF) | bits;
                }
                *self = Self::None;
            }
            Self::Sub { a, b, r, w, write_cf } => {
                let sign = w - 1;
                let ab = a ^ b;
                let of = (((ab & (a ^ r)) >> sign) & 1) as u32;
                let af = (((ab ^ r) >> 4) & 1) as u32;
                let sf = ((r >> sign) & 1) as u32;
                let zf = u32::from(r == 0);
                let pf = u32::from((r as u8).count_ones() & 1 == 0);
                let bits = (pf << 2) | (af << 4) | (zf << 6) | (sf << 7) | (of << 11);
                if write_cf {
                    let cf = u32::from(a < b);
                    *undefined_flags &= !ARITH_FLAGS;
                    *eflags = (*eflags & !ARITH_FLAGS) | bits | cf;
                } else {
                    const NO_CF: u32 = ARITH_FLAGS & !1;
                    *undefined_flags &= !NO_CF;
                    *eflags = (*eflags & !NO_CF) | bits;
                }
                *self = Self::None;
            }
            Self::Logic { r, w } => {
                const DEF_MASK: u32 = (1 << 0) | (1 << 2) | (1 << 6) | (1 << 7) | (1 << 11);
                const AF_MASK: u32 = 1 << 4;
                let sf = ((r >> (w - 1)) & 1) as u32;
                let zf = u32::from(r == 0);
                let pf = u32::from((r as u8).count_ones() & 1 == 0);
                *undefined_flags = (*undefined_flags & !DEF_MASK) | AF_MASK;
                *eflags = (*eflags & !(DEF_MASK | AF_MASK)) | (pf << 2) | (zf << 6) | (sf << 7);
                *self = Self::None;
            }
        }
    }
    #[inline]
    pub fn flush_cf_only(&mut self, eflags: &mut u32, undefined_flags: &mut u32) {
        match *self {
            Self::Add { a, b, w, ref mut write_cf, .. } if *write_cf => {
                let cf = (((a + b) >> w) & 1) as u32;
                *undefined_flags &= !1;
                *eflags = (*eflags & !1) | cf;
                *write_cf = false;
            }
            Self::Sub { a, b, ref mut write_cf, .. } if *write_cf => {
                let cf = u32::from(a < b);
                *undefined_flags &= !1;
                *eflags = (*eflags & !1) | cf;
                *write_cf = false;
            }
            Self::Logic { .. } => {
                *undefined_flags &= !1;
                *eflags &= !1;
            }
            _ => {}
        }
    }
    #[inline]
    pub fn eval_jcc(&mut self, cond: u8, eflags: &mut u32, undefined_flags: &mut u32) -> bool {
        match (*self, cond) {
            (Self::Add { r, .. } | Self::Sub { r, .. } | Self::Logic { r, .. }, 4) => return r == 0,
            (Self::Add { r, .. } | Self::Sub { r, .. } | Self::Logic { r, .. }, 5) => return r != 0,
            (Self::Add { r, w, .. } | Self::Sub { r, w, .. } | Self::Logic { r, w }, 8) => return ((r >> (w - 1)) & 1) != 0,
            (Self::Add { r, w, .. } | Self::Sub { r, w, .. } | Self::Logic { r, w }, 9) => return ((r >> (w - 1)) & 1) == 0,
            (Self::Sub { a, b, write_cf: true, .. }, 2) => return a < b,
            (Self::Sub { a, b, write_cf: true, .. }, 3) => return a >= b,
            (Self::Sub { a, b, write_cf: true, .. }, 6) => return a <= b,
            (Self::Sub { a, b, write_cf: true, .. }, 7) => return a > b,
            (Self::Sub { a, b, r, w, .. }, 12) => {
                let sign = w - 1;
                return ((r >> sign) & 1) != ((((a ^ b) & (a ^ r)) >> sign) & 1);
            }
            (Self::Sub { a, b, r, w, .. }, 13) => {
                let sign = w - 1;
                return ((r >> sign) & 1) == ((((a ^ b) & (a ^ r)) >> sign) & 1);
            }
            (Self::Sub { a, b, r, w, .. }, 14) => {
                if r == 0 { return true; }
                let sign = w - 1;
                return ((r >> sign) & 1) != ((((a ^ b) & (a ^ r)) >> sign) & 1);
            }
            (Self::Sub { a, b, r, w, .. }, 15) => {
                if r == 0 { return false; }
                let sign = w - 1;
                return ((r >> sign) & 1) == ((((a ^ b) & (a ^ r)) >> sign) & 1);
            }
            _ => {}
        }
        self.flush(eflags, undefined_flags);
        let f = *eflags;
        let cf = (f & 1) != 0;
        let pf = ((f >> 2) & 1) != 0;
        let zf = ((f >> 6) & 1) != 0;
        let sf = ((f >> 7) & 1) != 0;
        let of = ((f >> 11) & 1) != 0;
        match cond {
            0 => of, 1 => !of, 2 => cf, 3 => !cf, 4 => zf, 5 => !zf,
            6 => cf || zf, 7 => !(cf || zf), 8 => sf, 9 => !sf, 10 => pf, 11 => !pf,
            12 => sf ^ of, 13 => !(sf ^ of), 14 => (sf ^ of) || zf, _ => !((sf ^ of) || zf),
        }
    }
}
fn is_conditions16_prefix(s: &[Token]) -> bool {
    matches!(
        s,
        [
            Token::ReadFlag(0, Flag::OF),
            Token::Not(1, 0),
            Token::ReadFlag(2, Flag::CF),
            Token::Not(3, 2),
            Token::ReadFlag(4, Flag::ZF),
            Token::Not(5, 4),
            Token::Binary(6, Binary::Or, 2, 4),
            Token::Not(7, 6),
            Token::ReadFlag(8, Flag::SF),
            Token::Not(9, 8),
            Token::ReadFlag(10, Flag::PF),
            Token::Not(11, 10),
            Token::Binary(12, Binary::Xor, 8, 0),
            Token::Not(13, 12),
            Token::Binary(14, Binary::Or, 12, 4),
            Token::Not(15, 14),
            ..
        ]
    )
}
fn intermediates_are_dead(rest:&[Token], slots:&[usize])->bool {
    let used=|i:&usize|slots.contains(i);
    !rest.iter().any(|t|match t {
        Token::Extend(_,a,_)|Token::SignExtend(_,a,_)|Token::Truncate(_,a,_)
        |Token::Not(_,a)|Token::Bit(_,a,_)|Token::Zero(_,a)|Token::Parity(_,a)
        |Token::Write(a)|Token::WriteSource(a)|Token::Flag(_,a)|Token::WriteGpr(_,a,_)
        |Token::WriteSegment(_,a)|Token::InterruptShadowIf(a)|Token::RetireIf(a)
        |Token::ResumeIf(a)|Token::Require(a)|Token::RaiseUnless(a,_)|Token::CheckSegment(_,a,_)|Token::CheckWriteSegment(_,a,_)
        |Token::ReadAddressAfter(_,_,a)|Token::Load(_,a,_)|Token::LoadSegment(_,a,_,_)=>used(a),
        Token::WriteControl(_,a)|Token::ReadDescriptor(_,_,a)|Token::LoadSystem(_,a)|Token::CheckIo(a,_)=>used(a),
        Token::FarReturn(a,b,c,_,_,v)=>used(a)||used(b)||used(c)||v.as_ref().is_some_and(used),
        Token::WriteTable(_,a,b)=>used(a)||used(b),
        Token::Binary(_,_,a,b)|Token::Shift(_,_,a,b)|Token::Less(_,a,b)
        |Token::BranchIf(a,b)|Token::FarBranch(a,b)|Token::FarJump(a,b)|Token::FarCall(a,b)|Token::Store(a,b,_)
        |Token::StoreSegment(a,b,_,_)|Token::WriteIf(a,b)|Token::WriteSourceIf(a,b)=>used(a)||used(b),
        Token::Select(_,a,b,c)|Token::FlagIf(_,a,b,c)|Token::StoreIf(a,b,c,_)=>used(a)||used(b)||used(c),
        _=>false,
    })
}
fn pack_tokens(tokens: &[Token]) -> Vec<PackedOp> {
    let filtered: Vec<Token> = tokens.iter().filter(|t| !matches!(t, Token::PreserveFlags(_))).cloned().collect();
    let mut out = Vec::with_capacity(filtered.len());
    let mut i = 0;
    while i < filtered.len() {
        let rest = &filtered[i..];
        if i == 0 && is_conditions16_prefix(rest) {
            if let [
                Token::ReadIp(ft, true, 32),
                Token::Truncate(base, ft2, w),
                Token::ReadDisp(delta, w2),
                Token::Binary(wrapped, Binary::Add, base2, delta2),
                Token::Extend(target, wrapped2, 32),
                Token::BranchIf(target2, cond),
                Token::Retire,
            ] = &rest[16..]
            {
                if ft == ft2 && w == w2 && base == base2 && delta == delta2 && wrapped == wrapped2 && target == target2 && *cond < 16 {
                    out.push(PackedOp::Jcc16 { cond: *cond as u8, w: *w });
                    out.push(PackedOp::Token(Token::Retire));
                    break;
                }
            }
            out.push(PackedOp::Conditions16);
            i += 16;
            continue;
        }
        if let [
            Token::ReadIp(ft, true, 32),
            Token::Truncate(base, ft2, w),
            Token::ReadDisp(delta, w2),
            Token::Binary(wrapped, Binary::Add, base2, delta2),
            Token::Extend(target, wrapped2, 32),
            Token::Const(take, Value { bits: 1, width: 1 }),
            Token::BranchIf(target2, take2),
            ..
        ] = rest
        {
            if ft == ft2 && w == w2 && base == base2 && delta == delta2 && wrapped == wrapped2 && target == target2 && take == take2
                && intermediates_are_dead(&rest[7..],&[*ft,*base,*delta,*wrapped,*target,*take]) {
                out.push(PackedOp::RelJmp { w: *w });
                i += 7;
                continue;
            }
        }
        if let [
            Token::Extend(aw, a, ext),
            Token::Extend(bw, b, ext2),
            Token::Binary(wide, Binary::Add, aw2, bw2),
            Token::Truncate(r, wide2, w),
            Token::Bit(cf, wide3, w2),
            Token::Binary(ab, Binary::Xor, a2, b2),
            Token::Not(nab, ab2),
            Token::Binary(ar, Binary::Xor, a3, r2),
            Token::Binary(overflow, Binary::And, nab2, ar2),
            Token::Bit(of, overflow2, sign),
            Token::Binary(abr, Binary::Xor, ab3, r3),
            Token::Bit(af, abr2, 4),
            Token::Bit(sf, r4, sign2),
            Token::Zero(zf, r5),
            Token::Parity(pf, r6),
            ..
        ] = rest
        {
            if *ext == *w + 1 && ext2 == ext && aw2 == aw && bw2 == bw && wide2 == wide && wide3 == wide
                && w2 == w && a2 == a && b2 == b && ab2 == ab && a3 == a && r2 == r && nab2 == nab
                && ar2 == ar && overflow2 == overflow && *sign == *w - 1 && ab3 == ab && r3 == r
                && abr2 == abr && r4 == r && sign2 == sign && r5 == r && r6 == r
                && intermediates_are_dead(&rest[15..],&[*aw,*bw,*wide,*ab,*nab,*ar,*overflow,*abr])
            {
                out.push(PackedOp::AddCore { a: *a, b: *b, r: *r, cf: *cf, of: *of, af: *af, sf: *sf, zf: *zf, pf: *pf, w: *w });
                i += 15;
                continue;
            }
        }
        if let [
            Token::Extend(aw, a, ext),
            Token::Extend(bw, b, ext2),
            Token::Binary(wide, Binary::Sub, aw2, bw2),
            Token::Truncate(r, wide2, w),
            Token::Bit(cf, wide3, w2),
            Token::Binary(ab, Binary::Xor, a2, b2),
            Token::Binary(ar, Binary::Xor, a3, r2),
            Token::Binary(overflow, Binary::And, ab2, ar2),
            Token::Bit(of, overflow2, sign),
            Token::Binary(abr, Binary::Xor, ab3, r3),
            Token::Bit(af, abr2, 4),
            Token::Bit(sf, r4, sign2),
            Token::Zero(zf, r5),
            Token::Parity(pf, r6),
            ..
        ] = rest
        {
            if *ext == *w + 1 && ext2 == ext && aw2 == aw && bw2 == bw && wide2 == wide && wide3 == wide
                && w2 == w && a2 == a && b2 == b && a3 == a && r2 == r && ab2 == ab
                && ar2 == ar && overflow2 == overflow && *sign == *w - 1 && ab3 == ab && r3 == r
                && abr2 == abr && r4 == r && sign2 == sign && r5 == r && r6 == r
                && intermediates_are_dead(&rest[14..],&[*aw,*bw,*wide,*ab,*ar,*overflow,*abr])
            {
                out.push(PackedOp::SubCore { a: *a, b: *b, r: *r, cf: *cf, of: *of, af: *af, sf: *sf, zf: *zf, pf: *pf, w: *w });
                i += 14;
                continue;
            }
        }
        if let [
            Token::SignExtend(aw, a, dbl),
            Token::SignExtend(bw, b, dbl2),
            Token::Binary(wide, Binary::Mul, aw2, bw2),
            Token::Truncate(r, wide2, w),
            Token::SignExtend(restored, r2, dbl3),
            Token::Binary(lost, Binary::Xor, wide3, restored2),
            Token::Zero(fits, lost2),
            Token::Not(overflow, fits2),
            Token::Write(r3),
            Token::Flag(Flag::CF, overflow2),
            Token::Flag(Flag::OF, overflow3),
            Token::UndefFlag(Flag::PF),
            Token::UndefFlag(Flag::AF),
            Token::UndefFlag(Flag::ZF),
            Token::UndefFlag(Flag::SF),
            ..
        ] = rest
        {
            if dbl2 == dbl && aw2 == aw && bw2 == bw && wide2 == wide && r2 == r && dbl3 == dbl
                && wide3 == wide && restored2 == restored && lost2 == lost && fits2 == fits
                && r3 == r && overflow2 == overflow && overflow3 == overflow
                && intermediates_are_dead(&rest[15..],&[*aw,*bw,*wide,*restored,*lost,*fits])
            {
                out.push(PackedOp::ImulCore { a: *a, b: *b, r: *r, overflow: *overflow, w: *w, dbl: *dbl });
                i += 15;
                continue;
            }
        }
        if let [
            Token::Const(cm, Value { bits: 31, width: w }),
            Token::Binary(count, Binary::And, b, cm2),
            Token::Const(one, Value { bits: 1, width: w2 }),
            Token::Const(width_slot, Value { bits: wb, width: w3 }),
            Token::Const(yes, Value { bits: 1, width: 1 }),
            Token::Const(no, Value { bits: 0, width: 1 }),
            Token::Zero(is_zero, count2),
            Token::Not(active, is_zero2),
            Token::Binary(cmo, Binary::Xor, count3, one2),
            Token::Zero(single, cmo2),
            Token::Binary(last, Binary::Sub, count4, one3),
            ..
        ] = rest
        {
            if cm2 == cm && w2 == w && w3 == w && *wb == u64::from(*w) && count2 == count
                && is_zero2 == is_zero && count3 == count && one2 == one && cmo2 == cmo
                && count4 == count && one3 == one
                && intermediates_are_dead(&rest[11..],&[*cm,*one,*is_zero,*cmo])
            {
                out.push(PackedOp::ShiftSetup { b: *b, count: *count, width_slot: *width_slot, yes: *yes, no: *no, active: *active, single: *single, last: *last, w: *w });
                i += 11;
                continue;
            }
        }
        if let [
            Token::FlagIf(Flag::CF, cf, cf_defined, active),
            Token::FlagIf(Flag::OF, of, single, active2),
            Token::FlagIf(Flag::AF, no, no2, active3),
            Token::FlagIf(Flag::SF, sf, yes, active4),
            Token::FlagIf(Flag::ZF, zf, yes2, active5),
            Token::FlagIf(Flag::PF, pf, yes3, active6),
            ..
        ] = rest
        {
            if active2 == active && active3 == active && active4 == active && active5 == active && active6 == active
                && no2 == no && yes2 == yes && yes3 == yes
                && filtered.iter().any(|t|matches!(t,Token::Const(o,Value{bits:0,width:1}) if o==no))
                && filtered.iter().any(|t|matches!(t,Token::Const(o,Value{bits:1,width:1}) if o==yes))
            {
                out.push(PackedOp::ShiftFlags { cf: *cf, cf_defined: *cf_defined, of: *of, single: *single, sf: *sf, zf: *zf, pf: *pf, active: *active });
                i += 6;
                continue;
            }
        }
        if let [
            Token::Const(clear, Value { bits: 0, width: 1 }),
            Token::Bit(sf, r, sign),
            Token::Zero(zf, r2),
            Token::Parity(pf, r3),
            ..
        ] = rest
        {
            if r2 == r && r3 == r {
                out.push(PackedOp::LogicCore { clear: *clear, sf: *sf, zf: *zf, pf: *pf, r: *r, sign: *sign });
                i += 4;
                continue;
            }
        }
        if let [
            Token::Flag(Flag::CF, cf),
            Token::Flag(Flag::PF, pf),
            Token::Flag(Flag::AF, af),
            Token::Flag(Flag::ZF, zf),
            Token::Flag(Flag::SF, sf),
            Token::Flag(Flag::OF, of),
            ..
        ] = rest
        {
            out.push(PackedOp::Flags6 { cf: *cf, pf: *pf, af: *af, zf: *zf, sf: *sf, of: *of });
            i += 6;
            continue;
        }
        if let [
            Token::Flag(Flag::PF, pf),
            Token::Flag(Flag::AF, af),
            Token::Flag(Flag::ZF, zf),
            Token::Flag(Flag::SF, sf),
            Token::Flag(Flag::OF, of),
            ..
        ] = rest
        {
            out.push(PackedOp::Flags5NoCf { pf: *pf, af: *af, zf: *zf, sf: *sf, of: *of });
            i += 5;
            continue;
        }
        if let [
            Token::Flag(Flag::CF, clear),
            Token::Flag(Flag::OF, clear2),
            Token::UndefFlag(Flag::AF),
            Token::Flag(Flag::SF, sf),
            Token::Flag(Flag::ZF, zf),
            Token::Flag(Flag::PF, pf),
            ..
        ] = rest
        {
            if clear2 == clear {
                out.push(PackedOp::LogicFlags { clear: *clear, sf: *sf, zf: *zf, pf: *pf });
                i += 6;
                continue;
            }
        }
        if let [Token::Read(a, Binding::Dst, w), Token::Read(b, Binding::Src, w2), ..] = rest {
            if w2 == w {
                out.push(PackedOp::ReadDstSrc { a: *a, b: *b, w: *w });
                i += 2;
                continue;
            }
        }
        if let [Token::Read(a, Binding::Dst, w), Token::Const(b, c), ..] = rest {
            out.push(PackedOp::ReadDstConst { a: *a, b: *b, w: *w, c: *c });
            i += 2;
            continue;
        }
        out.push(PackedOp::Token(rest[0].clone()));
        i += 1;
    }
    let n = out.len();
    if n >= 3 && matches!(out[n - 1], PackedOp::Token(Token::Retire)) {
        match (&out[n - 3], &out[n - 2]) {
            (&PackedOp::SubCore { a, b, r, w, cf, pf, af, zf, sf, of }, &PackedOp::Flags6 { cf:fc, pf:fp, af:fa, zf:fz, sf:fs, of:fo })
                if [cf,pf,af,zf,sf,of]==[fc,fp,fa,fz,fs,fo] => {
                out[n - 3] = PackedOp::SubLazy { a, b, r, w, write_cf: true };
                out.remove(n - 2);
            }
            (&PackedOp::LogicCore { r, sign, clear, sf, zf, pf }, &PackedOp::LogicFlags { clear:fc, sf:fs, zf:fz, pf:fp })
                if [clear,sf,zf,pf]==[fc,fs,fz,fp] => {
                out[n - 3] = PackedOp::LogicLazy { r, w: sign + 1 };
                out.remove(n - 2);
            }
            _ if n >= 4 && matches!(out[n - 3], PackedOp::Token(Token::Write(_)) | PackedOp::Token(Token::Store(..))) => {
                match (&out[n - 4], &out[n - 2]) {
                    (&PackedOp::AddCore { a, b, r, w, cf, pf, af, zf, sf, of }, &PackedOp::Flags6 { cf:fc, pf:fp, af:fa, zf:fz, sf:fs, of:fo })
                        if [cf,pf,af,zf,sf,of]==[fc,fp,fa,fz,fs,fo] => {
                        out[n - 4] = PackedOp::AddLazy { a, b, r, w, write_cf: true };
                        out.remove(n - 2);
                    }
                    (&PackedOp::AddCore { a, b, r, w, pf, af, zf, sf, of, .. }, &PackedOp::Flags5NoCf { pf:fp, af:fa, zf:fz, sf:fs, of:fo })
                        if [pf,af,zf,sf,of]==[fp,fa,fz,fs,fo] => {
                        out[n - 4] = PackedOp::AddLazy { a, b, r, w, write_cf: false };
                        out.remove(n - 2);
                    }
                    (&PackedOp::SubCore { a, b, r, w, cf, pf, af, zf, sf, of }, &PackedOp::Flags6 { cf:fc, pf:fp, af:fa, zf:fz, sf:fs, of:fo })
                        if [cf,pf,af,zf,sf,of]==[fc,fp,fa,fz,fs,fo] => {
                        out[n - 4] = PackedOp::SubLazy { a, b, r, w, write_cf: true };
                        out.remove(n - 2);
                    }
                    (&PackedOp::SubCore { a, b, r, w, pf, af, zf, sf, of, .. }, &PackedOp::Flags5NoCf { pf:fp, af:fa, zf:fz, sf:fs, of:fo })
                        if [pf,af,zf,sf,of]==[fp,fa,fz,fs,fo] => {
                        out[n - 4] = PackedOp::SubLazy { a, b, r, w, write_cf: false };
                        out.remove(n - 2);
                    }
                    (&PackedOp::LogicCore { r, sign, clear, sf, zf, pf }, &PackedOp::LogicFlags { clear:fc, sf:fs, zf:fz, pf:fp })
                        if [clear,sf,zf,pf]==[fc,fs,fz,fp] => {
                        out[n - 4] = PackedOp::LogicLazy { r, w: sign + 1 };
                        out.remove(n - 2);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}
#[derive(Clone, Debug)]
struct Program { tokens: Vec<Token>, packed: Vec<PackedOp>, widths: Vec<u8>, register_effects: usize, operand_width: u8, source_width: u8, uses_flow: bool, uses_displacement: bool, uses_memory: bool, has_read_address: bool, flushes_lazy: bool, partial: bool, checkpoint_flags:bool }

impl Program {
    pub fn token_line(t: &Token) -> String {
        match t {
            Token::Read(v, b, w) => format!("v{v} = read {b:?} {w}"),
            Token::ReadFlag(v, f) => format!("v{v} = read_flag {f:?}"),
            Token::ReadFlags(v) => format!("v{v} = read_flags"),
            Token::CheckpointFlags => "checkpoint_flags".into(),
            Token::RequireRealMode => "require_real_mode".into(),
            Token::RequireRing0 => "require_ring0".into(),
            Token::InterruptShadowIf(c) => format!("interrupt_shadow_if v{c}"),
            Token::ReadAddress(v) => format!("v{v} = read_address"),
            Token::ReadAddressAfter(v, r, a) => format!("v{v} = read_address_after r{r} v{a}"),
            Token::WriteTable(t,l,b) => format!("write_table {t} v{l} v{b}"),
            Token::ReadTable(l,b,t) => format!("v{l}, v{b} = read_table {t}"),
            Token::ReadSystem(v,t) => format!("v{v} = read_system {t}"),
            Token::ReadCpl(v) => format!("v{v} = read_cpl"),
            Token::ReadDescriptor(r,f,a) => format!("v{r}, v{f} = read_descriptor v{a}"),
            Token::LoadSystem(t,a) => format!("load_system {t} v{a}"),
            Token::RequireProtectedMode => "require_protected_mode".into(),
            Token::ReadControl(v,r)=>format!("v{v} = read_control {r}"),
            Token::WriteControl(r,v)=>format!("write_control {r} v{v}"),
            Token::ReadMsw(v) => format!("v{v} = read_msw"),
            Token::ReadMsw32(v) => format!("v{v} = read_msw32"),
            Token::Require(c) => format!("require v{c}"),
            Token::RaiseUnless(c,v) => format!("raise_unless v{c} {v}"),
            Token::LoadSegment(v, a, m, s) => format!("v{v} = load_segment [v{a}] {} (seg {s})", m.bytes() * 8),
            Token::StoreSegment(a, v, m, s) => format!("store_segment [v{a}] v{v} {} (seg {s})", m.bytes() * 8),
            Token::RetireIf(c) => format!("retire_if v{c}"),
            Token::ResumeIf(c) => format!("resume_if v{c}"),
            Token::ReadSegment(v, s) => format!("v{v} = read_segment {s}"),
            Token::WriteSegment(s, v) => format!("write_segment {s} v{v}"),
            Token::ReadGpr(v, r, w) => format!("v{v} = read_gpr r{r} {w}"),
            Token::WriteGpr(r, v, w) => format!("write_gpr r{r} v{v} {w}"),
            Token::Load(v, a, m) => format!("v{v} = load [v{a}] {}", m.bytes() * 8),
            Token::Store(a, v, m) => format!("store [v{a}] v{v} {}", m.bytes() * 8),
            Token::StoreIf(a, v, c, m) => format!("store_if [v{a}] v{v} {} (when v{c})", m.bytes() * 8),
            Token::ReadIp(v, _, w) => format!("v{v} = read_ip {w}"),
            Token::ReadDisp(v, w) => format!("v{v} = read_disp {w}"),
            Token::BranchIf(t, c) => format!("branch_if v{t} (when v{c})"),
            Token::FarBranch(t, s) => format!("far_branch v{t}:v{s}"),
            Token::FarJump(t, s) => format!("far_jump v{t}:v{s}"),
            Token::FarCall(t, s) => format!("far_call v{t}:v{s}"),
            Token::FarReturn(t, s, o, w, c, v) => format!("far_return v{t}:v{s} outer v{o} {} +{c}{}", w.bytes() * 8, v.map(|v|format!(" v86 v{v}")).unwrap_or_default()),
            Token::RequireIopl => "require_iopl".into(),
            Token::CheckIo(a, n) => format!("check_io v{a} {n}"),
            Token::CheckSegment(s, a, n) => format!("check_segment {s} v{a} {n}"),
            Token::CheckWriteSegment(s, a, w) => format!("check_write_segment {s} v{a} {}",w.bits()),
            Token::PreserveFlags(m) => format!("preserve_flags 0x{m:03x}"),
            Token::UndefFlag(f) => format!("undef_flag {f:?}"),
            Token::Select(v, c, a, b) => format!("v{v} = select v{c} ? v{a} : v{b}"),
            Token::WriteIf(v, c) => format!("write_if dst v{v} (when v{c})"),
            Token::WriteSource(v) => format!("write_source v{v}"),
            Token::WriteSourceIf(v, c) => format!("write_source_if src v{v} (when v{c})"),
            Token::Const(v, val) => format!("v{v} = const {} 0x{:x}", val.width, val.bits),
            Token::Extend(v, a, w) => format!("v{v} = zext v{a} {w}"),
            Token::SignExtend(v, a, w) => format!("v{v} = sext v{a} {w}"),
            Token::Truncate(v, a, w) => format!("v{v} = trunc v{a} {w}"),
            Token::Binary(v, op, a, b) => format!("v{v} = {op:?} v{a}, v{b}"),
            Token::Shift(v, op, a, b) => format!("v{v} = {op:?} v{a}, v{b}"),
            Token::Less(v, a, b) => format!("v{v} = ult v{a}, v{b}"),
            Token::FlagIf(f, a, d, c) => format!("flag_if {f:?} v{a} (def=v{d}, act=v{c})"),
            Token::Not(v, a) => format!("v{v} = not v{a}"),
            Token::Bit(v, a, b) => format!("v{v} = bit v{a} {b}"),
            Token::Zero(v, a) => format!("v{v} = zero v{a}"),
            Token::Parity(v, a) => format!("v{v} = parity v{a}"),
            Token::Write(v) => format!("write dst v{v}"),
            Token::Flag(f, v) => format!("flag {f:?} v{v}"),
            Token::Retire => "retire".into(),
        }
    }
    pub fn disassemble(&self) -> String {
        self.tokens.iter().map(Self::token_line).collect::<Vec<_>>().join("\n")
    }
    pub fn trace_disassemble(
        &self,
        stepped: usize,
        cpu: &Cpu,
        dst: View,
        src: View,
        context: FlowContext,
        ram: &Ram,
        address: u32,
        memory_context: Option<address::MemoryContext>,
        machine: Option<&address::SegmentState>,
        effective: Option<address::EffectiveAddress>,
    ) -> (String, Option<usize>) {
        let mut values: Vec<Value> = self.widths.iter().map(|&w| value(0, w)).collect();
        let mut lines = Vec::new();
        let mut early_stop = None;
        for (idx, t) in self.tokens.iter().enumerate() {
            let s = Self::token_line(t);
            let mut note = String::new();
            if early_stop.is_none() {
                match *t {
                    _ if machine.is_none() && matches!(t,
                        Token::CheckpointFlags | Token::RequireRealMode | Token::RequireRing0 | Token::InterruptShadowIf(..)
                        | Token::LoadSegment(..) | Token::StoreSegment(..) | Token::ReadSegment(..)
                        | Token::ReadControl(..)|Token::WriteControl(..)|Token::WriteSegment(..) | Token::ReadMsw(..)|Token::ReadMsw32(..)|Token::WriteTable(..) | Token::CheckSegment(..) | Token::CheckWriteSegment(..) | Token::FarBranch(..) | Token::FarJump(..) | Token::FarCall(..) | Token::FarReturn(..) | Token::RequireIopl | Token::CheckIo(..)
                        | Token::ReadTable(..) | Token::ReadSystem(..) | Token::ReadCpl(..) | Token::ReadDescriptor(..) | Token::LoadSystem(..) | Token::RequireProtectedMode) => {
                        early_stop = Some(idx); note = "; FAULT (missing machine context)".into();
                    }
                    Token::RequireRing0 => {
                        if machine.unwrap().cpl() != 0 {
                            early_stop = Some(idx); note = "; FAULT (CPL 0 required)".into();
                        } else { note = "; CPL 0 ok".into(); }
                    }
                    Token::RequireIopl => {
                        let m = machine.unwrap();
                        if !m.real_mode && u32::from(m.cpl()) > (cpu.eflags >> 12) & 3 {
                            early_stop = Some(idx); note = "; FAULT (CPL above IOPL)".into();
                        } else { note = "; IOPL ok".into(); }
                    }
                    Token::CheckIo(..) => {
                        let m = machine.unwrap();
                        if cpu.eflags & 0x20000 != 0 || (!m.real_mode && u32::from(m.cpl()) > (cpu.eflags >> 12) & 3) {
                            early_stop = Some(idx); note = "; I/O bitmap check not shown".into();
                        } else { note = "; I/O permitted".into(); }
                    }
                    Token::FarJump(a, b) | Token::FarCall(a, b) | Token::FarReturn(a, b, ..) => {
                        if !machine.unwrap().real_mode || cpu.eflags & 0x20000 != 0 {
                            early_stop = Some(idx); note = "; protected-mode far transfer not shown".into();
                        } else if values[b].bits > u64::from(address::SegmentCache::real_mode(values[a].bits as u16).limit) {
                            early_stop = Some(idx); note = "; FAULT (far branch exceeds code limit)".into();
                        } else { note = format!("; stage CS:EIP <- {:04x}:{:08x}", values[a].bits, values[b].bits); }
                    }
                    Token::RequireProtectedMode => {
                        if machine.unwrap().real_mode || cpu.eflags & 0x20000 != 0 {
                            early_stop = Some(idx); note = "; FAULT (protected mode required)".into();
                        } else { note = "; protected mode ok".into(); }
                    }
                    Token::ReadTable(l,b,t) => {
                        let (limit,base)=machine.unwrap().tables[t];values[l]=value(limit as u64,16);values[b]=value(base as u64,32);
                        note = format!("; limit 0x{limit:04x}, base 0x{base:08x}");
                    }
                    Token::ReadSystem(o,t) => {
                        values[o]=value(machine.unwrap().ldt_tr[t].selector as u64,16);note = format!("; v{o} = 0x{:04x}", values[o].bits);
                    }
                    Token::ReadCpl(o) => {
                        values[o]=value(machine.unwrap().cpl() as u64,8);note = format!("; v{o} = {}", values[o].bits);
                    }
                    Token::ReadDescriptor(..) | Token::LoadSystem(..) => { note = "; descriptor read not shown".into(); early_stop = Some(idx); }
                    Token::RequireRealMode => {
                        if !machine.unwrap().real_mode || cpu.eflags & 0x20000 != 0 {
                            early_stop = Some(idx); note = "; FAULT (real mode required)".into();
                        } else { note = "; real mode ok".into(); }
                    }
                    Token::CheckWriteSegment(reg,a,w) => {
                        let m=machine.unwrap();
                        let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][reg],cache:m.segments[reg]};
                        let result=c.translate(values[a].bits as u32,w,true,cpu.eip).and_then(|at|{
                            // Preview must not set paging A/D bits in live RAM.
                            ram.clone().check_write(at,w).map_err(|f|f.error(cpu.eip))
                        });
                        match result{Ok(())=>note="; write permission ok (no store)".into(),Err(e)=>{early_stop=Some(idx);note=format!("; FAULT ({e:?})");}}
                    }
                    Token::CheckSegment(reg, a, n) => {
                        let m = machine.unwrap();
                        let c = address::MemoryContext {
                            segment: [address::SegmentRegister::Es, address::SegmentRegister::Cs, address::SegmentRegister::Ss, address::SegmentRegister::Ds, address::SegmentRegister::Fs, address::SegmentRegister::Gs][reg],
                            cache: m.segments[reg],
                        };
                        match c.translate_span(values[a].bits, n, false, cpu.eip) {
                            Ok(_) => note = "; segment span ok".into(),
                            Err(e) => { early_stop = Some(idx); note = format!("; FAULT ({e:?})"); }
                        }
                    }
                    Token::ReadFlags(o) => {
                        values[o] = value(cpu.eflags as u64, 32);
                        note = format!("; v{o} = 0x{:08x}", values[o].bits);
                    }
                    Token::Read(o, b, w) => {
                        let v = match b { Binding::Dst => dst, Binding::Src => src };
                        values[o] = value(cpu.read(v).bits, w);
                        note = format!("; v{o} = 0x{:x} ({})", values[o].bits, values[o].bits);
                    }
                    Token::LoadSegment(o, a, w, segment) => {
                        if let Some(m) = machine {
                            let c = address::MemoryContext {
                                segment: [address::SegmentRegister::Es, address::SegmentRegister::Cs, address::SegmentRegister::Ss, address::SegmentRegister::Ds, address::SegmentRegister::Fs, address::SegmentRegister::Gs][segment],
                                cache: m.segments[segment],
                            };
                            if let Ok(at) = c.translate(values[a].bits as u32, w, false, cpu.eip) {
                                if let Ok(v) = ram.load(at, w) {
                                    values[o] = value(v as u64, w.bits());
                                    note = format!("; [0x{at:08x}] -> v{o} = 0x{:x} ({v})", values[o].bits);
                                } else { early_stop = Some(idx); note = format!("; RAM read fault at 0x{at:08x}"); }
                            } else { early_stop = Some(idx); note = "; segment fault".into(); }
                        }
                    }
                    Token::StoreSegment(a, v, w, segment) => {
                        if let Some(m) = machine {
                            let c = address::MemoryContext {
                                segment: [address::SegmentRegister::Es, address::SegmentRegister::Cs, address::SegmentRegister::Ss, address::SegmentRegister::Ds, address::SegmentRegister::Fs, address::SegmentRegister::Gs][segment],
                                cache: m.segments[segment],
                            };
                            if let Ok(at) = c.translate(values[a].bits as u32, w, true, cpu.eip) {
                                if ram.check_write(at, w).is_ok() {
                                    note = format!("; stage [0x{at:08x}] <- 0x{:x} ({}B)", values[v].bits, w.bytes());
                                } else { early_stop = Some(idx); note = format!("; RAM write fault at 0x{at:08x}"); }
                            } else { early_stop = Some(idx); note = "; segment fault".into(); }
                        }
                    }
                    Token::RetireIf(c) => {
                        if values[c].bits != 0 { early_stop = Some(idx); note = "; retire early (cond=1)".into(); }
                        else { note = "; continue (cond=0)".into(); }
                    }
                    Token::ResumeIf(c) => {
                        note = if values[c].bits != 0 { "; repeat next iteration".into() } else { "; finish repeat".into() };
                    }
                    Token::ReadSegment(o, reg) => {
                        if let Some(m) = machine {
                            values[o] = value(m.selectors[reg] as u64, 16);
                            note = format!("; v{o} = 0x{:04x}", values[o].bits);
                        }
                    }
                    Token::WriteSegment(reg, a) => {
                        if !machine.unwrap().real_mode {
                            early_stop = Some(idx); note = "; FAULT (real mode required)".into();
                        } else { note = format!("; stage seg {reg} <- 0x{:04x}", values[a].bits); }
                    }
                    Token::ReadGpr(o, reg, w) => {
                        values[o] = value(cpu.regs[reg] as u64, w);
                        note = format!("; v{o} = 0x{:x} ({})", values[o].bits, values[o].bits);
                    }
                    Token::WriteGpr(reg, a, _) => {
                        note = format!("; stage r{reg} <- 0x{:x} ({})", values[a].bits, values[a].bits);
                    }
                    Token::ReadControl(o,r)=>{match machine.unwrap().read_control(r,cpu.eflags,cpu.eip){Ok(v)=>values[o]=value(v as u64,32),Err(e)=>{early_stop=Some(idx);note=format!("; FAULT {e:?}");}}}
                    Token::WriteControl(r,v)=>{if let Err(e)=machine.unwrap().validate_control(r,values[v].bits as u32,cpu.eflags,cpu.eip){early_stop=Some(idx);note=format!("; FAULT {e:?}");}else{note="; stage control register".into();}}
                    Token::WriteTable(..) => { note = "; stage descriptor table".into(); }
                    Token::ReadMsw(o) => {
                        if let Some(m) = machine {
                            values[o] = value(m.machine_status() as u64, 16);
                            note = format!("; v{o} = 0x{:04x}", values[o].bits);
                        }
                    }
                    Token::ReadMsw32(o) => {
                        if let Some(m) = machine { values[o]=value(u64::from(m.machine_status())|(u64::from(m.paging)<<31),32); }
                    }
                    Token::Require(c) => {
                        if values[c].bits == 0 { early_stop = Some(idx); note = "; FAULT (require v=0)".into(); }
                        else { note = "; require ok".into(); }
                    }
                    Token::RaiseUnless(c, v) => {
                        if values[c].bits == 0 { early_stop = Some(idx); note = format!("; EXCEPTION vector {v:02x}h"); }
                        else { note = "; no exception".into(); }
                    }
                    Token::ReadAddressAfter(o, reg, a) => {
                        let mut regs = cpu.regs;
                        regs[reg] = values[a].bits as u32;
                        if let Some(ea) = effective {
                            match ea.resolve(&regs, None) {
                                Ok(res) => {
                                    values[o] = value(res.offset as u64, 32);
                                    note = format!("; v{o} = 0x{:08x}", values[o].bits);
                                }
                                Err(e) => { early_stop = Some(idx); note = format!("; FAULT ({e:?})"); }
                            }
                        } else { early_stop = Some(idx); note = "; FAULT (missing effective address)".into(); }
                    }
                    Token::ReadAddress(o) => {
                        values[o] = value(address as u64, 32);
                        note = format!("; v{o} = 0x{:08x}", values[o].bits);
                    }
                    Token::Load(o, a, w) => {
                        let offset = values[a].bits as u32;
                        let addr_res = match memory_context {
                            Some(c) => c.translate(offset, w, false, cpu.eip),
                            None => Ok(offset),
                        };
                        if let Ok(addr) = addr_res {
                            if let Ok(v) = ram.load(addr, w) {
                                values[o] = value(v as u64, w.bits());
                                note = format!("; [0x{addr:08x}] -> v{o} = 0x{:x} ({v})", values[o].bits);
                            } else { early_stop = Some(idx); note = format!("; RAM read fault at 0x{addr:08x}"); }
                        } else { early_stop = Some(idx); note = "; segment fault".into(); }
                    }
                    Token::StoreIf(_, _, c, _) if values[c].bits == 0 => { note = "; store skipped".into(); }
                    Token::Store(a, v, w) | Token::StoreIf(a, v, _, w) => {
                        let offset = values[a].bits as u32;
                        let addr_res = match memory_context {
                            Some(c) => c.translate(offset, w, true, cpu.eip),
                            None => Ok(offset),
                        };
                        if let Ok(addr) = addr_res {
                            if ram.check_write(addr, w).is_ok() {
                                note = format!("; stage [0x{addr:08x}] <- 0x{:x} ({}B)", values[v].bits, w.bytes());
                            } else { early_stop = Some(idx); note = format!("; RAM write fault at 0x{addr:08x}"); }
                        } else { early_stop = Some(idx); note = "; segment fault".into(); }
                    }
                    Token::ReadIp(o, next, w) => {
                        values[o] = value((if next { context.next_eip } else { cpu.eip }) as u64, w);
                        note = format!("; v{o} = 0x{:08x}", values[o].bits);
                    }
                    Token::ReadDisp(o, w) => {
                        values[o] = value(context.displacement as u32 as u64, w);
                        note = format!("; v{o} = 0x{:x} ({:+})", values[o].bits, context.displacement);
                    }
                    Token::BranchIf(a, c) => {
                        if values[c].bits != 0 && values[a].bits > u64::from(context.code_limit) {
                            early_stop = Some(idx); note = "; FAULT (branch exceeds code limit)".into();
                        } else {
                            note = if values[c].bits != 0 { format!("; stage EIP <- 0x{:08x} (taken)", values[a].bits) } else { "; branch not taken".into() };
                        }
                    }
                    Token::FarBranch(a, b) => {
                        if !machine.unwrap().real_mode || cpu.eflags & 0x20000 != 0 {
                            early_stop = Some(idx); note = "; FAULT (real mode required)".into();
                        } else if values[b].bits > u64::from(address::SegmentCache::real_mode(values[a].bits as u16).limit) {
                            early_stop = Some(idx); note = "; FAULT (far branch exceeds code limit)".into();
                        } else { note = format!("; stage CS:EIP <- {:04x}:{:08x}", values[a].bits, values[b].bits); }
                    }
                    Token::Select(o, c, a, b) => {
                        values[o] = if values[c].bits != 0 { values[a] } else { values[b] };
                        note = format!("; v{o} = 0x{:x} ({})", values[o].bits, values[o].bits);
                    }
                    Token::WriteIf(a, c) => {
                        note = if values[c].bits != 0 { format!("; stage dst <- 0x{:x} ({})", values[a].bits, values[a].bits) } else { "; dst write skipped".into() };
                    }
                    Token::ReadFlag(o, f) => {
                        values[o] = value(((cpu.eflags >> f.bit()) & 1) as u64, 1);
                        note = format!("; v{o} = {}", values[o].bits);
                    }
                    Token::Const(o, x) => {
                        values[o] = x;
                        note = format!("; v{o} = 0x{:x} ({})", x.bits, x.bits);
                    }
                    Token::Extend(o, a, w) | Token::Truncate(o, a, w) => {
                        values[o] = value(values[a].bits, w);
                        note = format!("; v{o} = 0x{:x} ({})", values[o].bits, values[o].bits);
                    }
                    Token::SignExtend(o, a, w) => {
                        let x = values[a];
                        let bits = if x.bits & (1u64 << (x.width - 1)) != 0 { x.bits | !mask(x.width) } else { x.bits };
                        values[o] = value(bits, w);
                        note = format!("; v{o} = 0x{:x} ({})", values[o].bits, values[o].bits);
                    }
                    Token::Binary(o, op, a, b) => {
                        let x = values[a]; let y = values[b];
                        let n = match op {
                            Binary::Add => x.bits.wrapping_add(y.bits),
                            Binary::Sub => x.bits.wrapping_sub(y.bits),
                            Binary::Div => if y.bits == 0 { 0 } else { x.bits / y.bits },
                            Binary::Rem => if y.bits == 0 { 0 } else { x.bits % y.bits },
                            Binary::Mul => x.bits.wrapping_mul(y.bits),
                            Binary::Xor => x.bits ^ y.bits,
                            Binary::And => x.bits & y.bits,
                            Binary::Or => x.bits | y.bits,
                        };
                        values[o] = value(n, x.width);
                        note = format!("; v{o} = 0x{:x} ({})", values[o].bits, values[o].bits);
                    }
                    Token::Shift(o, kind, a, b) => {
                        let x = values[a]; let c = values[b].bits;
                        let n = match kind {
                            Shift::Left => if c >= u64::from(x.width) { 0 } else { x.bits << c },
                            Shift::Right => if c >= u64::from(x.width) { 0 } else { x.bits >> c },
                            Shift::Arithmetic => {
                                let signed = ((x.bits << (64 - x.width)) as i64) >> (64 - x.width);
                                (signed >> c.min(u64::from(x.width - 1))) as u64
                            }
                        };
                        values[o] = value(n, x.width);
                        note = format!("; v{o} = 0x{:x} ({})", values[o].bits, values[o].bits);
                    }
                    Token::Less(o, a, b) => {
                        values[o] = value(u64::from(values[a].bits < values[b].bits), 1);
                        note = format!("; v{o} = {}", values[o].bits);
                    }
                    Token::FlagIf(f, a, d, c) => {
                        note = if values[c].bits == 0 { format!("; preserve {f:?}") }
                            else if values[d].bits == 0 { format!("; stage {f:?} <- ?") }
                            else { format!("; stage {f:?} <- {}", values[a].bits) };
                    }
                    Token::Not(o, a) => {
                        values[o] = value(!values[a].bits, values[a].width);
                        note = format!("; v{o} = 0x{:x} ({})", values[o].bits, values[o].bits);
                    }
                    Token::Bit(o, a, i) => {
                        values[o] = value((values[a].bits >> i) & 1, 1);
                        note = format!("; v{o} = {}", values[o].bits);
                    }
                    Token::Zero(o, a) => {
                        values[o] = value(u64::from(values[a].bits == 0), 1);
                        note = format!("; v{o} = {}", values[o].bits);
                    }
                    Token::Parity(o, a) => {
                        values[o] = value(u64::from((values[a].bits as u8).count_ones() % 2 == 0), 1);
                        note = format!("; v{o} = {}", values[o].bits);
                    }
                    Token::Write(a) => {
                        note = format!("; stage dst <- 0x{:x} ({})", values[a].bits, values[a].bits);
                    }
                    Token::WriteSource(a) => {
                        note = format!("; stage src <- 0x{:x} ({})", values[a].bits, values[a].bits);
                    }
                    Token::WriteSourceIf(a, c) => {
                        note = if values[c].bits != 0 { format!("; stage src <- 0x{:x} ({})", values[a].bits, values[a].bits) } else { "; src write skipped".into() };
                    }
                    Token::Flag(f, a) => {
                        note = format!("; stage {f:?} <- {}", values[a].bits);
                    }
                    Token::UndefFlag(f) => {
                        note = format!("; stage {f:?} <- ?");
                    }
                    Token::PreserveFlags(m) => {
                        note = format!("; preserve mask 0x{m:03x}");
                    }
                    Token::Retire => {
                        note = "; commit staged effects".into();
                    }
                    _ => {}
                }
            }
            if idx < stepped {
                lines.push(format!("✓ {idx:02}: {s:<32} {note}"));
            } else if idx == stepped {
                lines.push(format!("> {idx:02}: {s:<32} {note}"));
            } else {
                lines.push(format!("  {idx:02}: {s}"));
            }
        }
        (lines.join("\n"), early_stop)
    }
}

fn number(s: &str) -> R<u64> {
    if let Some(hex) = s.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| format!("bad number: {s}"))
    } else { s.parse().map_err(|_| format!("bad number: {s}")) }
}
/// Staged RAM stores one PIL program may commit together (PUSHA needs eight).
// Up to eight staged stores live on the stack; ENTER with a nesting level
// needs up to 33 (BP, 31 copied frame pointers and the new frame pointer).
const MAX_STORES:usize=8;
const MAX_PROGRAM_STORES:usize=33;
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
    let mut wrote_src = false;
    let mut branched = false;
    let mut stored = false;
    let mut store_count=0;
    let mut segment_written=false;
    let mut shadow_written=false;
    let mut flags_seen = 0u32;
    let mut source_width=None;
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
                ["write", "src", input] | ["write_if", "src", input, _] => {
                    let a=slot(&names,input)?;let w=widths[a];
                    if ![8,16,32].contains(&w){return Err("source write width must be 8,16,32".into());}
                    if source_width.is_some_and(|old|old!=w){return Err("inconsistent source binding widths".into());}
                    source_width=Some(w);
                    if wrote_src{return Err("multiple source writes".into());}wrote_src=true;
                    if fields[0]=="write_if"{let c=slot(&names,fields[3])?;
                        if widths[c]!=1{return Err("write_if predicate needs bv<1>".into());}
                        Token::WriteSourceIf(a,c)
                    }else{Token::WriteSource(a)}
                }
                ["write_if", "dst", input, predicate] => {
                    let a=slot(&names,input)?; let c=slot(&names,predicate)?;
                    if widths[a]!=operand_width {return Err("write width mismatch".into());}
                    if widths[c]!=1 {return Err("write_if predicate needs bv<1>".into());}
                    if wrote {return Err("multiple destination writes".into());}
                    wrote=true; Token::WriteIf(a,c)
                }
                ["checkpoint_flags"]=>{
                    if tokens.iter().any(|t|!matches!(t,Token::PreserveFlags(0))){return Err("checkpoint_flags must precede all operations".into());}
                    Token::CheckpointFlags
                }
                ["read_flags", out] => Token::ReadFlags(define(&mut names,&mut widths,out,32)?),
                ["require_real_mode"] => Token::RequireRealMode,
                ["require_ring0"] => Token::RequireRing0,
                ["interrupt_shadow_if", input] => {
                    let c=slot(&names,input)?;
                    if widths[c]!=1||shadow_written{return Err("one bv1 interrupt shadow effect required".into());}
                    shadow_written=true;Token::InterruptShadowIf(c)
                }
                ["read_segment", out, register] => {
                    let reg=number(register)?;if reg>=6{return Err("segment index must be 0..5".into());}
                    Token::ReadSegment(define(&mut names,&mut widths,out,16)?,reg as usize)
                }
                ["write_segment", register, input] => {
                    let reg=number(register)?;let a=slot(&names,input)?;
                    if reg>=6||reg==1||widths[a]!=16||segment_written{return Err("one non-CS segment write of bv16 is required".into());}
                    segment_written=true;Token::WriteSegment(reg as usize,a)
                }
                ["read_gpr", out, register, bits] => {
                    let reg=number(register)? as usize;let w=width(bits)?;
                    if reg>=8 || ![8,16,32].contains(&w){return Err("read_gpr needs register 0..7 and width8/16/32".into());}
                    Token::ReadGpr(define(&mut names,&mut widths,out,w)?,reg,w)
                }
                ["write_gpr", register, input] => {
                    let reg=number(register)? as usize;let a=slot(&names,input)?;let w=widths[a];
                    if reg>=8 || ![8,16,32].contains(&w){return Err("write_gpr needs register 0..7 and bv8/16/32".into());}
                    Token::WriteGpr(reg,a,w)
                }
                ["load_segment", out, register, address, bits]=>{
                    // A load_segment after a staged store sees that store's bytes.
                    let segment=number(register)?;let a=slot(&names,address)?;let w=width(bits)?;
                    if segment>=6||widths[a]!=32{return Err("load_segment needs segment0..5 and bv32 offset".into());}
                    let mem=MemWidth::from_bits(w).ok_or("memory width must be8/16/32")?;
                    Token::LoadSegment(define(&mut names,&mut widths,out,w)?,a,mem,segment as usize)
                }
                ["store_segment", register, address, input]=>{
                    let segment=number(register)?;let a=slot(&names,address)?;let v=slot(&names,input)?;
                    if segment>=6||widths[a]!=32||store_count>=MAX_PROGRAM_STORES{return Err("at most 33 stores with segment0..5 and bv32 offset required".into());}
                    let mem=MemWidth::from_bits(widths[v]).ok_or("memory width must be8/16/32")?;stored=true;store_count+=1;Token::StoreSegment(a,v,mem,segment as usize)
                }
                ["retire_if", predicate]=>{
                    let c=slot(&names,predicate)?;
                    if widths[c]!=1||tokens.iter().any(|t|matches!(t,Token::Write(..)|Token::WriteIf(..)|Token::WriteSource(..)|Token::WriteSourceIf(..)|Token::WriteGpr(..)|Token::WriteSegment(..)|Token::WriteControl(..)|Token::WriteTable(..)|Token::InterruptShadowIf(..)|Token::Store(..)|Token::StoreIf(..)|Token::StoreSegment(..)|Token::Flag(..)|Token::FlagIf(..)|Token::UndefFlag(..)|Token::BranchIf(..)|Token::FarBranch(..)|Token::ResumeIf(..))){return Err("retire_if requires bv1 and no earlier effects".into());}
                    Token::RetireIf(c)
                }
                ["resume_if", predicate]=>{
                    let c=slot(&names,predicate)?;if widths[c]!=1||branched{return Err("one resume/branch effect with bv1 required".into());}branched=true;Token::ResumeIf(c)
                }
                ["write_table", table, limit, base]=>{
                    let t=number(table)? as usize;let l=slot(&names,limit)?;let b=slot(&names,base)?;
                    if t>1||widths[l]!=16||widths[b]!=32||tokens.iter().any(|x|matches!(x,Token::WriteTable(..))){return Err("write_table needs table 0..1, bv16 limit and bv32 base, once".into());}
                    Token::WriteTable(t,l,b)
                }
                ["read_control",out,register]=>{let r=number(register)? as usize;if !matches!(r,0|2|3){return Err("read_control needs CR0, CR2 or CR3".into());}Token::ReadControl(define(&mut names,&mut widths,out,32)?,r)}
                ["write_control",register,input]=>{let r=number(register)? as usize;let v=slot(&names,input)?;if !matches!(r,0|2|3)||widths[v]!=32||tokens.iter().any(|t|matches!(t,Token::WriteControl(..))){return Err("write_control needs CR0, CR2 or CR3 and bv32, once".into());}Token::WriteControl(r,v)}
                ["read_msw", out]=>Token::ReadMsw(define(&mut names,&mut widths,out,16)?),
                ["read_msw32", out]=>Token::ReadMsw32(define(&mut names,&mut widths,out,32)?),
                ["require_protected_mode"]=>Token::RequireProtectedMode,
                ["read_table", limit, base, table]=>{
                    let t=number(table)? as usize;if t>1{return Err("read_table needs table 0..1".into());}
                    let l=define(&mut names,&mut widths,limit,16)?;Token::ReadTable(l,define(&mut names,&mut widths,base,32)?,t)
                }
                ["read_system", out, which]=>{let t=number(which)? as usize;if t>1{return Err("read_system needs 0 (LDTR) or 1 (TR)".into());}Token::ReadSystem(define(&mut names,&mut widths,out,16)?,t)}
                ["read_cpl", out]=>Token::ReadCpl(define(&mut names,&mut widths,out,8)?),
                ["read_descriptor", raw, found, selector]=>{
                    let a=slot(&names,selector)?;if widths[a]!=16{return Err("read_descriptor needs a bv16 selector".into());}
                    let r=define(&mut names,&mut widths,raw,64)?;Token::ReadDescriptor(r,define(&mut names,&mut widths,found,1)?,a)
                }
                ["load_system", which, selector]=>{
                    let t=number(which)? as usize;let a=slot(&names,selector)?;
                    if t>1||widths[a]!=16||tokens.iter().any(|x|matches!(x,Token::LoadSystem(..))){return Err("load_system needs 0 (LLDT) or 1 (LTR) and a bv16 selector, once".into());}
                    Token::LoadSystem(t,a)
                }
                ["raise_unless", predicate, vector]=>{
                    let c=slot(&names,predicate)?;let v=number(vector)?;
                    if widths[c]!=1||!matches!(v,0|5|13){return Err("raise_unless needs a bv1 predicate and vector 0 (#DE), 5 (#BR) or 13 (#GP(0))".into());}Token::RaiseUnless(c,v as u8)
                }
                ["require", predicate]=>{
                    let c=slot(&names,predicate)?;if widths[c]!=1{return Err("require needs bv1 predicate".into());}Token::Require(c)
                }
                ["read_address_after", out, register, input]=>{
                    let reg=number(register)?;let a=slot(&names,input)?;
                    if reg>=8||widths[a]!=32{return Err("read_address_after requires register0..7 and bv32 replacement".into());}
                    Token::ReadAddressAfter(define(&mut names,&mut widths,out,32)?,reg as usize,a)
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
                    if store_count>=MAX_PROGRAM_STORES {return Err("at most 33 staged RAM stores are supported".into());}
                    stored=true;store_count+=1;Token::Store(a,v,mem)
                }
                ["store_if", address, input, predicate] => {
                    let a=slot(&names,address)?;let v=slot(&names,input)?;let c=slot(&names,predicate)?;
                    if widths[a]!=32 {return Err("memory address must be bv<32>".into());}
                    if widths[c]!=1 {return Err("store_if predicate must be bv<1>".into());}
                    let mem=MemWidth::from_bits(widths[v]).ok_or("memory width must be 8,16,32")?;
                    if store_count>=MAX_PROGRAM_STORES {return Err("at most 33 staged RAM stores are supported".into());}
                    stored=true;store_count+=1;Token::StoreIf(a,v,c,mem)
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
                ["check_write_segment", register, offset, bits]=>{
                    let reg=number(register)?;let a=slot(&names,offset)?;
                    if reg>=6||widths[a]!=32{return Err("check_write_segment requires segment0..5 and bv32 offset".into());}
                    let mem=MemWidth::from_bits(width(bits)?).ok_or("memory width must be8/16/32")?;
                    Token::CheckWriteSegment(reg as usize,a,mem)
                }
                ["check_segment", register, offset, bytes]=>{
                    let reg=number(register)?;let a=slot(&names,offset)?;let n=number(bytes)?;
                    if reg>=6||widths[a]!=64||n==0||n>u32::MAX as u64{return Err("check_segment requires segment0..5,bv64 offset,positive u32 byte count".into());}
                    Token::CheckSegment(reg as usize,a,n as u32)
                }
                ["far_branch", selector, offset]=>{
                    let a=slot(&names,selector)?;let b=slot(&names,offset)?;
                    if widths[a]!=16||widths[b]!=32||branched||segment_written||![16,32].contains(&operand_width){return Err("one far_branch requires bv16 selector,bv32 offset and no other segment/branch effect".into());}
                    branched=true;segment_written=true;Token::FarBranch(a,b)
                }
                ["far_jump", selector, offset]|["far_call", selector, offset]=>{
                    let a=slot(&names,selector)?;let b=slot(&names,offset)?;
                    if widths[a]!=16||widths[b]!=32||branched||segment_written||![16,32].contains(&operand_width){return Err("one far transfer requires bv16 selector,bv32 offset and no other segment/branch effect".into());}
                    let call=fields[0]=="far_call";
                    // A gate CALL ends the instruction here, so nothing may be staged before it.
                    if call&&tokens.iter().any(|t|matches!(t,Token::Write(..)|Token::WriteIf(..)|Token::WriteSource(..)|Token::WriteSourceIf(..)|Token::WriteGpr(..)|Token::WriteSegment(..)|Token::WriteControl(..)|Token::WriteTable(..)|Token::InterruptShadowIf(..)|Token::Store(..)|Token::StoreIf(..)|Token::StoreSegment(..)|Token::Flag(..)|Token::FlagIf(..)|Token::UndefFlag(..)|Token::ResumeIf(..))){return Err("far_call must precede every effect".into());}
                    branched=true;segment_written=true;if call{Token::FarCall(a,b)}else{Token::FarJump(a,b)}
                }
                ["far_return", selector, offset, outer, bytes, cleanup, rest @ ..] if rest.len()<=1=>{
                    let a=slot(&names,selector)?;let b=slot(&names,offset)?;let c=slot(&names,outer)?;
                    let vm=match rest{[v]=>{let v=slot(&names,v)?;if widths[v]!=1{return Err("far_return's V86 operand must be bv1".into());}Some(v)},_=>None};
                    let w=match number(bytes)?{2=>MemWidth::Word,4=>MemWidth::Dword,_=>return Err("far_return slot width must be 2 or 4 bytes".into())};
                    let n=number(cleanup)?;
                    if widths[a]!=16||widths[b]!=32||widths[c]!=32||n>0xffff||branched||segment_written||![16,32].contains(&operand_width){return Err("one far_return requires bv16 selector,bv32 offset and outer offset, imm16 and no other segment/branch effect".into());}
                    if tokens.iter().any(|t|matches!(t,Token::WriteGpr(4,..))){return Err("far_return writes ESP itself".into());}
                    branched=true;segment_written=true;Token::FarReturn(a,b,c,w,n as u32,vm)
                }
                ["require_iopl"]=>Token::RequireIopl,
                ["check_io", port, bytes]=>{
                    let a=slot(&names,port)?;let n=number(bytes)?;
                    if widths[a]!=16||![1,2,4].contains(&n){return Err("check_io requires a bv16 port and 1, 2 or 4 bytes".into());}
                    Token::CheckIo(a,n as u8)
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
                    let b = match *binding { "dst" => Binding::Dst, "src" => Binding::Src,
                        _ => return Err("expected dst or src".into()) };
                    match b {
                        Binding::Dst if w!=operand_width=>return Err("destination binding width mismatch".into()),
                        Binding::Src=>{
                            if ![8,16,32].contains(&w){return Err("source binding width must be 8,16,32".into());}
                            if source_width.is_some_and(|old|old!=w){return Err("inconsistent source binding widths".into());}
                            source_width=Some(w);
                        },_=>{},
                    }
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
                [op @ ("add" | "sub" | "mul" | "div" | "rem" | "xor" | "and" | "or"), out, left, right] => {
                    let a = slot(&names,left)?; let b = slot(&names,right)?;
                    if widths[a] != widths[b] { return Err("binary width mismatch".into()); }
                    let w = widths[a];
                    let o = define(&mut names, &mut widths, out, w)?;
                    let op = match *op { "add" => Binary::Add, "sub" => Binary::Sub,
                        "div" => Binary::Div, "rem" => Binary::Rem, "mul" => Binary::Mul, "xor" => Binary::Xor, "or" => Binary::Or, _ => Binary::And };
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
    if tokens.iter().any(|t|matches!(t,Token::WriteGpr(..))) && (wrote||wrote_src) {
        return Err("named register effects cannot mix with bound writes".into());
    }
    if !retired { return Err("missing retire".into()); }
    // DF is optional in descriptions; unmentioned non-arithmetic flags stay intact.
    if flags_seen & ARITH_FLAGS != ARITH_FLAGS { return Err("classify all six arithmetic flags: flag, preserve_flag, preserve_flags or undef_flag".into()); }
    let uses_flow=tokens.iter().any(|t| matches!(t,Token::ReadControl(..)|Token::WriteControl(..)|Token::ReadMsw(..)|Token::ReadMsw32(..)|Token::WriteTable(..)|Token::ReadTable(..)|Token::ReadSystem(..)|Token::ReadCpl(..)|Token::ReadDescriptor(..)|Token::LoadSystem(..)|Token::RequireProtectedMode|Token::CheckSegment(..)|Token::CheckWriteSegment(..)|Token::CheckpointFlags|Token::RequireRealMode|Token::RequireRing0|Token::RequireIopl|Token::CheckIo(..)|Token::InterruptShadowIf(..)|Token::ReadIp(..)|Token::ReadDisp(..)|Token::BranchIf(..)|Token::FarBranch(..)|Token::FarJump(..)|Token::FarCall(..)|Token::FarReturn(..)|Token::ResumeIf(..)));
    let uses_displacement=tokens.iter().any(|t| matches!(t,Token::ReadDisp(..)));
    let uses_memory=tokens.iter().any(|t|matches!(t,Token::ReadAddressAfter(..)|Token::ReadAddress(..)|Token::Load(..)|Token::Store(..)|Token::StoreIf(..)|Token::LoadSegment(..)|Token::StoreSegment(..)|Token::CheckWriteSegment(..)));
    let partial=tokens.iter().any(|t|matches!(t,Token::ResumeIf(..)));
    let checkpoint_flags=tokens.iter().any(|t|matches!(t,Token::CheckpointFlags));
    let has_read_address=tokens.iter().any(|t|matches!(t,Token::ReadAddress(_)));
    let packed=pack_tokens(&tokens);
    let register_effects=tokens.iter().filter(|t|matches!(t,Token::WriteGpr(..))).count();
    let flushes_lazy=packed.iter().any(|op|matches!(op,
        PackedOp::Conditions16|PackedOp::Flags6{..}|PackedOp::Flags5NoCf{..}|PackedOp::LogicFlags{..}|PackedOp::ShiftFlags{..}|PackedOp::ImulCore{..}
        |PackedOp::Token(Token::ReadFlags(_)|Token::ReadFlag(..)|Token::Flag(..)|Token::FlagIf(..)|Token::UndefFlag(_))
    ));
    Ok(Program { checkpoint_flags,partial,tokens,packed, widths, register_effects, operand_width, source_width:source_width.unwrap_or(operand_width), uses_flow, uses_displacement, uses_memory, has_read_address, flushes_lazy })
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
    let mut lazy=LazyFlags::None;
    let res=execute_state(p,cpu,dst,src,context,None,0,None,None,None,&mut lazy).map(|_|());
    lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);res
}
#[cfg(test)]
fn execute_memory(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError> {
    let mut lazy=LazyFlags::None;
    let res=execute_state(p,cpu,dst,src,context,Some(ram),address,None,None,None,&mut lazy).map(|_|());
    lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);res
}
fn execute_memory_probed(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,ram:&mut Ram,address:u32)->Result<Option<ProbedMemoryAccess>,ExecError> {
    let mut lazy=LazyFlags::None;
    let res=execute_state(p,cpu,dst,src,context,Some(ram),address,None,None,None,&mut lazy);
    lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);res
}
#[cfg(test)]
fn execute_segmented(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError> {
    let (offset, memory_context)=address.resolve(&cpu.regs)?;
    let mut lazy=LazyFlags::None;
    let res=execute_state(p,cpu,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective),&mut lazy).map(|_|());
    lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);res
}
fn execute_machine(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,ram:&mut Ram,address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
    let mut lazy=LazyFlags::None;
    let res=execute_machine_lazy(p,cpu,dst,src,context,ram,address,machine,&mut lazy).map(|_|());
    lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);res
}
fn execute_machine_lazy(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState,lazy:&mut LazyFlags)->Result<Option<ProbedMemoryAccess>,ExecError>{
    let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let (o,c)=a.resolve(&cpu.regs)?;(o,Some(c))}else{(0,None)};
    if p.checkpoint_flags{lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);machine.checkpoint_flags(cpu.eip,cpu.eflags,cpu.undefined_flags);}
    // RAM walks the page tables with CR0.PG, CR3 and CPL as they are now.
    ram.set_paging(machine.ram_paging());
    let result=execute_state(p,cpu,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective),lazy);
    ram.set_paging(machine.ram_paging());
    if result.is_err()&&p.checkpoint_flags{if let Some((f,u))=machine.recover_flags(){cpu.eflags=f;cpu.undefined_flags=u;*lazy=LazyFlags::None;}}
    result
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbedMemoryAccess {
    pub address: u32,
    pub bytes: u8,
    pub write: bool,
    pub is_stack: bool,
}
fn merge_probed_access(current:&mut Option<ProbedMemoryAccess>,next:ProbedMemoryAccess){
    match *current {
        None => *current = Some(next),
        Some(prev) => {
            if prev.address == next.address && prev.bytes == next.bytes {
                *current = Some(ProbedMemoryAccess { write: prev.write || next.write, is_stack: prev.is_stack && next.is_stack, ..prev });
            } else if prev.is_stack && !next.is_stack {
                *current = Some(next);
            } else if !prev.is_stack && next.is_stack {
                // Keep explicit data operand over implicit stack frame slot.
            } else if prev.write == next.write && prev.is_stack == next.is_stack
                && next.address == prev.address.wrapping_add(u32::from(prev.bytes))
                && prev.bytes.saturating_add(next.bytes) <= 8
            {
                *current = Some(ProbedMemoryAccess { bytes: prev.bytes + next.bytes, ..prev });
            } else if next.write && !prev.write {
                *current = Some(next);
            }
        }
    }
}
fn probe_machine_access(p:&Program,cpu:&Cpu,dst:View,src:View,context:FlowContext,ram:&Ram,mut address:Option<address::AddressContext>,machine:&address::SegmentState)->Option<ProbedMemoryAccess>{
    if !p.uses_memory{return None;}
    let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let (o,c)=a.resolve(&cpu.regs).ok()?;(o,Some(c))}else{(0,None)};
    probe_program_access(p,cpu,dst,src,context,ram,offset,memory,Some(machine),address.map(|a|a.effective))
}
fn probe_program_access(p:&Program,cpu:&Cpu,dst:View,src:View,context:FlowContext,ram:&Ram,address:u32,memory_context:Option<address::MemoryContext>,machine:Option<&address::SegmentState>,effective:Option<address::EffectiveAddress>)->Option<ProbedMemoryAccess>{
    if !p.uses_memory{return None;}
    let has_read_address=p.tokens.iter().any(|t|matches!(t,Token::ReadAddress(_)));
    let mut values:Vec<Value>=p.widths.iter().map(|&w|value(0,w)).collect();
    let mut found:Option<ProbedMemoryAccess>=None;
    for t in &p.tokens {
        match *t {
            Token::ReadFlags(o)=>values[o]=value(cpu.eflags as u64,32),
            Token::Read(o,b,w)=>{let v=match b{Binding::Dst=>dst,Binding::Src=>src};values[o]=value(cpu.read(v).bits,w);}
            Token::LoadSegment(o,a,w,segment)=>{
                let m=machine?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment],cache:m.segments[segment]};
                let at=c.translate(values[a].bits as u32,w,false,cpu.eip).ok()?;
                let v=ram.load(at,w).unwrap_or(0);
                values[o]=value(v as u64,w.bits());
                merge_probed_access(&mut found,ProbedMemoryAccess{address:at,bytes:w.bytes(),write:false,is_stack:segment==2&&!p.checkpoint_flags});
            }
            Token::StoreSegment(a,_,w,segment)=>{
                let m=machine?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment],cache:m.segments[segment]};
                let at=c.translate(values[a].bits as u32,w,true,cpu.eip).ok()?;
                merge_probed_access(&mut found,ProbedMemoryAccess{address:at,bytes:w.bytes(),write:true,is_stack:segment==2&&!p.checkpoint_flags});
            }
            Token::RetireIf(c)=>if values[c].bits!=0{return found;},
            Token::ReadSegment(o,reg)=>values[o]=value(machine?.selectors[reg] as u64,16),
            Token::ReadGpr(o,reg,w)=>values[o]=value(cpu.regs[reg] as u64,w),
            Token::WriteControl(..)=>{},
            Token::ReadControl(o,r)=>values[o]=value(machine?.read_control(r,cpu.eflags,cpu.eip).ok()? as u64,32),
            Token::WriteTable(..)=>{},
            Token::ReadTable(l,b,t)=>{let (limit,base)=machine?.tables[t];values[l]=value(limit as u64,16);values[b]=value(base as u64,32);}
            Token::ReadSystem(o,t)=>values[o]=value(machine?.ldt_tr[t].selector as u64,16),
            Token::ReadCpl(o)=>values[o]=value(machine?.cpl() as u64,8),
            Token::ReadMsw(o)=>values[o]=value(machine?.machine_status() as u64,16),
            Token::ReadMsw32(o)=>{let m=machine?;values[o]=value(u64::from(m.machine_status())|(u64::from(m.paging)<<31),32)},
            Token::Require(c)|Token::RaiseUnless(c,_)=>if values[c].bits==0{return found;},
            Token::ReadAddressAfter(o,reg,a)=>{
                let mut regs=cpu.regs;regs[reg]=values[a].bits as u32;
                values[o]=value(effective?.resolve(&regs,None).ok()?.offset as u64,32);
            }
            Token::ReadAddress(o)=>values[o]=value(address as u64,32),
            Token::Load(o,a,w)=>{
                let offset=values[a].bits as u32;
                let addr=match memory_context{Some(c)=>c.translate(offset,w,false,cpu.eip).ok()?,None=>offset};
                let v=ram.load(addr,w).unwrap_or(0);
                values[o]=value(v as u64,w.bits());
                merge_probed_access(&mut found,ProbedMemoryAccess{address:addr,bytes:w.bytes(),write:false,is_stack:!has_read_address});
            }
            Token::StoreIf(_,_,c,_) if values[c].bits==0=>{}
            Token::Store(a,_,w)|Token::StoreIf(a,_,_,w)=>{
                let offset=values[a].bits as u32;
                let addr=match memory_context{Some(c)=>c.translate(offset,w,true,cpu.eip).ok()?,None=>offset};
                merge_probed_access(&mut found,ProbedMemoryAccess{address:addr,bytes:w.bytes(),write:true,is_stack:!has_read_address});
            }
            Token::ReadIp(o,next,w)=>values[o]=value((if next{context.next_eip}else{cpu.eip})as u64,w),
            Token::ReadDisp(o,w)=>values[o]=value(context.displacement as u32 as u64,w),
            Token::Select(o,c,a,b)=>values[o]=if values[c].bits!=0{values[a]}else{values[b]},
            Token::ReadFlag(o,f)=>values[o]=value(((cpu.eflags>>f.bit())&1)as u64,1),
            Token::Const(o,x)=>values[o]=x,
            Token::Extend(o,a,w)|Token::Truncate(o,a,w)=>values[o]=value(values[a].bits,w),
            Token::SignExtend(o,a,w)=>{
                let x=values[a];let bits=if x.bits&(1u64<<(x.width-1))!=0{x.bits|!mask(x.width)}else{x.bits};
                values[o]=value(bits,w);
            }
            Token::Binary(o,op,a,b)=>{
                let x=values[a];let y=values[b];
                let n=match op{Binary::Add=>x.bits.wrapping_add(y.bits),Binary::Sub=>x.bits.wrapping_sub(y.bits),Binary::Div=>if y.bits==0{0}else{x.bits/y.bits},Binary::Rem=>if y.bits==0{0}else{x.bits%y.bits},Binary::Mul=>x.bits.wrapping_mul(y.bits),Binary::Xor=>x.bits^y.bits,Binary::And=>x.bits&y.bits,Binary::Or=>x.bits|y.bits};
                values[o]=value(n,x.width);
            }
            Token::Shift(o,kind,a,b)=>{
                let x=values[a];let c=values[b].bits;
                let n=match kind{Shift::Left=>if c>=u64::from(x.width){0}else{x.bits<<c},Shift::Right=>if c>=u64::from(x.width){0}else{x.bits>>c},Shift::Arithmetic=>{let signed=((x.bits<<(64-x.width))as i64)>>(64-x.width);(signed>>c.min(u64::from(x.width-1)))as u64}};
                values[o]=value(n,x.width);
            }
            Token::Less(o,a,b)=>values[o]=value(u64::from(values[a].bits<values[b].bits),1),
            Token::Not(o,a)=>values[o]=value(!values[a].bits,values[a].width),
            Token::Bit(o,a,i)=>values[o]=value((values[a].bits>>i)&1,1),
            Token::Zero(o,a)=>values[o]=value(u64::from(values[a].bits==0),1),
            Token::Parity(o,a)=>values[o]=value(u64::from((values[a].bits as u8).count_ones()%2==0),1),
            Token::Retire=>return found,
            _=>{}
        }
    }
    found
}
/// Reads an 8-byte GDT/LDT entry at a linear address (paging is off).
/// Descriptor-table reads are supervisor-level accesses at any CPL.
fn read_descriptor(ram:Option<&Ram>,at:u32,eip:u32)->Result<u64,ExecError>{
    privilege::read_descriptor(ram.ok_or(ExecError::InvalidContext)?,at,eip)
}
/// Stages the descriptor's accessed bit as one more byte store.
fn stage_accessed(ram:Option<&Ram>,at:u32,v:u8,eip:u32,stores:&mut [Option<(u32,MemWidth,u32)>;MAX_STORES],extra:&mut Vec<(u32,MemWidth,u32)>,index:&mut usize)->Result<(),ExecError>{
    ram.ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::Byte).map_err(|f|access_error(eip,f))?;
    if *index<MAX_STORES{stores[*index]=Some((at,MemWidth::Byte,u32::from(v)));}else{extra.push((at,MemWidth::Byte,u32::from(v)));}*index+=1;Ok(())
}
/// Generated code for a segment load (shared helper in address.rs): defines
/// `selector` and `cache` and stages the accessed-bit store.
fn emit_segment_load(reg:&str,a:usize)->String{
    format!("let (selector,cache,accessed)={{let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let r=ram.as_deref();let e=*eip;m.load_segment({reg},v{a} as u16,e,&mut |at|privilege::read_descriptor(r.ok_or(ExecError::InvalidContext)?,at,e))?}};if let Some((at,v))=accessed{{ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::Byte).map_err(|f|f.error(*eip))?;pending_stores[store_index]=Some((at,MemWidth::Byte,u32::from(v)));store_index+=1;}}")
}
/// Generated code: a closure-like expression reading the 8-byte descriptor at
/// `at`, for the emitted read_descriptor and load_system tokens.
const EMIT_DESCRIPTOR_READ:&str="(|at:u32|->Result<u64,ExecError>{privilege::read_descriptor(ram.as_deref().ok_or(ExecError::InvalidContext)?,at,*eip)})";
fn access_error(eip:u32,f:memory::AccessFault)->ExecError {
    f.error(eip)
}
fn execute_state(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,mut ram:Option<&mut Ram>,address:u32,memory_context:Option<address::MemoryContext>,mut machine:Option<&mut address::SegmentState>,effective:Option<address::EffectiveAddress>,lazy:&mut LazyFlags)->Result<Option<ProbedMemoryAccess>,ExecError> {
    if p.uses_memory && ram.is_none() {return Err(ExecError::InvalidContext);}
    dst.validate(p.operand_width).map_err(|_|ExecError::InvalidBinding)?;
    src.validate(p.source_width).map_err(|_|ExecError::InvalidBinding)?;
    context.validate(p.operand_width,p.uses_displacement)?;
    let mut pending_eip=context.next_eip;
    let mut pending_resume=false;
    let mut pending_interrupt_shadow=false;
    let mut pending_control:Option<(usize,u32)>=None;
    let mut pending_table:Option<(usize,u16,u32)>=None;
    let mut pending_system:Option<(usize,address::SystemSegment)>=None;
    let mut pending_segment:Option<(usize,u16,address::SegmentCache)>=None;
    let mut pending_transfer:Option<privilege::Transfer>=None;
    let mut pending_stores:[Option<(u32,MemWidth,u32)>;MAX_STORES]=[None;MAX_STORES];let mut store_index=0;let mut extra_stores:Vec<(u32,MemWidth,u32)>=Vec::new();
    let mut probed:Option<ProbedMemoryAccess>=None;
    // Keep ordinary instructions on the stack without restricting valid PIL programs.
    let mut value_stack = [Value { bits: 0, width: 0 }; 64];
    let mut value_heap;
    let values:&mut [Value]=if p.widths.len()<=value_stack.len(){&mut value_stack}else{
        value_heap=vec![Value{bits:0,width:0};p.widths.len()];&mut value_heap
    };
    let mut pending_dst = None;
    let mut pending_src = None;
    let empty_effect=(View { register: 0, low: 0, width: 0 }, Value { bits: 0, width: 0 });
    let mut register_stack = [empty_effect; 4];
    let mut register_heap;
    let pending_registers:&mut [(View,Value)]=if p.register_effects<=register_stack.len(){&mut register_stack}else{
        register_heap=vec![empty_effect;p.register_effects];&mut register_heap
    };
    let mut reg_count = 0usize;
    if p.flushes_lazy {
        lazy.flush(&mut cpu.eflags, &mut cpu.undefined_flags);
    }
    let mut pending_flags = cpu.eflags;
    let mut pending_undefined = cpu.undefined_flags;
    let mut pending_lazy = *lazy;
    for op in &p.packed {
        match *op {
            PackedOp::ReadDstSrc { a, b, w } => {
                values[a] = value(cpu.read(dst).bits, w);
                values[b] = value(cpu.read(src).bits, w);
            }
            PackedOp::ReadDstConst { a, b, w, c } => {
                values[a] = value(cpu.read(dst).bits, w);
                values[b] = c;
            }
            PackedOp::Conditions16 => {
                let f = cpu.eflags;
                let of = u64::from((f >> 11) & 1);
                let cf = u64::from(f & 1);
                let zf = u64::from((f >> 6) & 1);
                let sf = u64::from((f >> 7) & 1);
                let pf = u64::from((f >> 2) & 1);
                let be = cf | zf;
                let l = sf ^ of;
                let le = l | zf;
                values[0] = Value { bits: of, width: 1 };
                values[1] = Value { bits: of ^ 1, width: 1 };
                values[2] = Value { bits: cf, width: 1 };
                values[3] = Value { bits: cf ^ 1, width: 1 };
                values[4] = Value { bits: zf, width: 1 };
                values[5] = Value { bits: zf ^ 1, width: 1 };
                values[6] = Value { bits: be, width: 1 };
                values[7] = Value { bits: be ^ 1, width: 1 };
                values[8] = Value { bits: sf, width: 1 };
                values[9] = Value { bits: sf ^ 1, width: 1 };
                values[10] = Value { bits: pf, width: 1 };
                values[11] = Value { bits: pf ^ 1, width: 1 };
                values[12] = Value { bits: l, width: 1 };
                values[13] = Value { bits: l ^ 1, width: 1 };
                values[14] = Value { bits: le, width: 1 };
                values[15] = Value { bits: le ^ 1, width: 1 };
            }
            PackedOp::Jcc16 { cond, w } => {
                let take = pending_lazy.eval_jcc(cond, &mut pending_flags, &mut pending_undefined);
                if take {
                    let target = context.next_eip.wrapping_add(context.displacement as u32) & (mask(w) as u32);
                    if target > context.code_limit { return Err(ExecError::GeneralProtection { eip: cpu.eip, error_code: 0 }); }
                    pending_eip = target;
                }
            }
            PackedOp::RelJmp { w } => {
                let target = context.next_eip.wrapping_add(context.displacement as u32) & (mask(w) as u32);
                if target > context.code_limit { return Err(ExecError::GeneralProtection { eip: cpu.eip, error_code: 0 }); }
                pending_eip = target;
            }
            PackedOp::AddLazy { a, b, r, w, write_cf } => {
                let va = values[a].bits;
                let vb = values[b].bits;
                let vr = (va + vb) & mask(w);
                values[r] = Value { bits: vr, width: w };
                if !write_cf {
                    pending_lazy.flush_cf_only(&mut pending_flags, &mut pending_undefined);
                }
                pending_lazy = LazyFlags::Add { a: va, b: vb, r: vr, w, write_cf };
            }
            PackedOp::SubLazy { a, b, r, w, write_cf } => {
                let va = values[a].bits;
                let vb = values[b].bits;
                let vr = va.wrapping_sub(vb) & mask(w);
                values[r] = Value { bits: vr, width: w };
                if !write_cf {
                    pending_lazy.flush_cf_only(&mut pending_flags, &mut pending_undefined);
                }
                pending_lazy = LazyFlags::Sub { a: va, b: vb, r: vr, w, write_cf };
            }
            PackedOp::LogicLazy { r, w } => {
                let vr = values[r].bits;
                pending_lazy = LazyFlags::Logic { r: vr, w };
            }
            PackedOp::AddCore { a, b, r, cf, of, af, sf, zf, pf, w } => {
                let va = values[a].bits;
                let vb = values[b].bits;
                let wide = va + vb;
                let vr = wide & mask(w);
                let sign = w - 1;
                let ab = va ^ vb;
                values[r] = Value { bits: vr, width: w };
                values[cf] = Value { bits: (wide >> w) & 1, width: 1 };
                values[of] = Value { bits: ((!ab & (va ^ vr)) >> sign) & 1, width: 1 };
                values[af] = Value { bits: ((ab ^ vr) >> 4) & 1, width: 1 };
                values[sf] = Value { bits: (vr >> sign) & 1, width: 1 };
                values[zf] = Value { bits: u64::from(vr == 0), width: 1 };
                values[pf] = Value { bits: u64::from((vr as u8).count_ones() & 1 == 0), width: 1 };
            }
            PackedOp::SubCore { a, b, r, cf, of, af, sf, zf, pf, w } => {
                let va = values[a].bits;
                let vb = values[b].bits;
                let wide = va.wrapping_sub(vb) & mask(w + 1);
                let vr = wide & mask(w);
                let sign = w - 1;
                let ab = va ^ vb;
                values[r] = Value { bits: vr, width: w };
                values[cf] = Value { bits: (wide >> w) & 1, width: 1 };
                values[of] = Value { bits: ((ab & (va ^ vr)) >> sign) & 1, width: 1 };
                values[af] = Value { bits: ((ab ^ vr) >> 4) & 1, width: 1 };
                values[sf] = Value { bits: (vr >> sign) & 1, width: 1 };
                values[zf] = Value { bits: u64::from(vr == 0), width: 1 };
                values[pf] = Value { bits: u64::from((vr as u8).count_ones() & 1 == 0), width: 1 };
            }
            PackedOp::LogicCore { clear, sf, zf, pf, r, sign } => {
                let vr = values[r].bits;
                values[clear] = Value { bits: 0, width: 1 };
                values[sf] = Value { bits: (vr >> sign) & 1, width: 1 };
                values[zf] = Value { bits: u64::from(vr == 0), width: 1 };
                values[pf] = Value { bits: u64::from((vr as u8).count_ones() & 1 == 0), width: 1 };
            }
            PackedOp::Flags6 { cf, pf, af, zf, sf, of } => {
                pending_lazy = LazyFlags::None;
                pending_undefined &= !ARITH_FLAGS;
                pending_flags = (pending_flags & !ARITH_FLAGS)
                    | (values[cf].bits as u32)
                    | ((values[pf].bits as u32) << 2)
                    | ((values[af].bits as u32) << 4)
                    | ((values[zf].bits as u32) << 6)
                    | ((values[sf].bits as u32) << 7)
                    | ((values[of].bits as u32) << 11);
            }
            PackedOp::Flags5NoCf { pf, af, zf, sf, of } => {
                pending_lazy.flush_cf_only(&mut pending_flags, &mut pending_undefined);
                pending_lazy = LazyFlags::None;
                const NO_CF: u32 = ARITH_FLAGS & !1;
                pending_undefined &= !NO_CF;
                pending_flags = (pending_flags & !NO_CF)
                    | ((values[pf].bits as u32) << 2)
                    | ((values[af].bits as u32) << 4)
                    | ((values[zf].bits as u32) << 6)
                    | ((values[sf].bits as u32) << 7)
                    | ((values[of].bits as u32) << 11);
            }
            PackedOp::LogicFlags { clear, sf, zf, pf } => {
                pending_lazy = LazyFlags::None;
                let c = values[clear].bits as u32;
                const DEF_MASK: u32 = (1 << 0) | (1 << 2) | (1 << 6) | (1 << 7) | (1 << 11);
                const AF_MASK: u32 = 1 << 4;
                pending_undefined = (pending_undefined & !DEF_MASK) | AF_MASK;
                pending_flags = (pending_flags & !(DEF_MASK | AF_MASK))
                    | c
                    | ((values[pf].bits as u32) << 2)
                    | ((values[zf].bits as u32) << 6)
                    | ((values[sf].bits as u32) << 7)
                    | (c << 11);
            }
            PackedOp::ShiftSetup { b, count, width_slot, yes, no, active, single, last, w } => {
                let c = values[b].bits & 31;
                values[count] = Value { bits: c, width: w };
                values[width_slot] = Value { bits: u64::from(w), width: w };
                values[yes] = Value { bits: 1, width: 1 };
                values[no] = Value { bits: 0, width: 1 };
                values[active] = Value { bits: u64::from(c != 0), width: 1 };
                values[single] = Value { bits: u64::from(c == 1), width: 1 };
                values[last] = value(c.wrapping_sub(1), w);
            }
            PackedOp::ShiftFlags { cf, cf_defined, of, single, sf, zf, pf, active } => {
                if values[active].bits != 0 {
                    pending_lazy = LazyFlags::None;
                    const SHIFT_CLEAR: u32 = ARITH_FLAGS;
                    pending_flags = (pending_flags & !SHIFT_CLEAR)
                        | ((values[pf].bits as u32) << 2)
                        | ((values[zf].bits as u32) << 6)
                        | ((values[sf].bits as u32) << 7);
                    pending_undefined = (pending_undefined & !((1 << 2) | (1 << 6) | (1 << 7))) | (1 << 4);
                    if values[cf_defined].bits != 0 {
                        pending_flags |= values[cf].bits as u32;
                        pending_undefined &= !1;
                    } else {
                        pending_undefined |= 1;
                    }
                    if values[single].bits != 0 {
                        pending_flags |= (values[of].bits as u32) << 11;
                        pending_undefined &= !(1 << 11);
                    } else {
                        pending_undefined |= 1 << 11;
                    }
                }
            }
            PackedOp::ImulCore { a, b, r, overflow, w, dbl } => {
                pending_lazy = LazyFlags::None;
                let xa = values[a];
                let xb = values[b];
                let sa = if xa.bits & (1u64 << (xa.width - 1)) != 0 { xa.bits | !mask(xa.width) } else { xa.bits };
                let sb = if xb.bits & (1u64 << (xb.width - 1)) != 0 { xb.bits | !mask(xb.width) } else { xb.bits };
                let aw = sa & mask(dbl);
                let bw = sb & mask(dbl);
                let wide = aw.wrapping_mul(bw) & mask(dbl);
                let vr = value(wide, w);
                let restored = (if vr.bits & (1u64 << (w - 1)) != 0 { vr.bits | !mask(w) } else { vr.bits }) & mask(dbl);
                let ovf = u64::from(wide != restored);
                values[r] = vr;
                values[overflow] = Value { bits: ovf, width: 1 };
                pending_dst = Some(vr);
                const CF_OF: u32 = (1 << 0) | (1 << 11);
                const UNDEF_REST: u32 = (1 << 2) | (1 << 4) | (1 << 6) | (1 << 7);
                pending_undefined = (pending_undefined & !CF_OF) | UNDEF_REST;
                pending_flags = (pending_flags & !(CF_OF | UNDEF_REST)) | (ovf as u32) | ((ovf as u32) << 11);
            }
            PackedOp::Token(ref t) => match *t {
            Token::CheckpointFlags=>{machine.as_deref().ok_or(ExecError::InvalidContext)?;},
            Token::ReadFlags(o)=>values[o]=value(cpu.eflags as u64,32),
            Token::RequireRealMode=>if !machine.as_deref().ok_or(ExecError::InvalidContext)?.real_mode||cpu.eflags&0x20000!=0{return Err(ExecError::InvalidContext);},
            Token::RequireRing0=>{let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;if m.cpl()!=0{return Err(ExecError::GeneralProtection{eip:cpu.eip,error_code:0});}},
            Token::RequireIopl=>{let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;if !m.real_mode&&u32::from(m.cpl())>(cpu.eflags>>12)&3{return Err(ExecError::GeneralProtection{eip:cpu.eip,error_code:0});}},
            Token::CheckIo(a,n)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let r=ram.as_deref().ok_or(ExecError::InvalidContext)?;let eip=cpu.eip;
                m.io_permitted(cpu.eflags,values[a].bits as u16,n,eip,&mut |at|r.as_level(false,|r|r.load(at,MemWidth::Byte)).map(|v|v as u8).map_err(|f|f.error(eip)))?;
            }
            Token::InterruptShadowIf(c)=>{machine.as_deref().ok_or(ExecError::InvalidContext)?;pending_interrupt_shadow=values[c].bits!=0;},
            Token::Read(o,b,w) => { let v = match b { Binding::Dst => dst, Binding::Src => src };
                values[o] = value(cpu.read(v).bits,w); }
            Token::LoadSegment(o,a,w,segment)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment],cache:m.segments[segment]};
                let at=c.translate(values[a].bits as u32,w,false,cpu.eip)?;
                values[o]=value(ram.as_deref().ok_or(ExecError::InvalidContext)?.load_staged(at,w,&pending_stores[..store_index.min(MAX_STORES)],&extra_stores).map_err(|f|access_error(cpu.eip,f))? as u64,w.bits());
                merge_probed_access(&mut probed,ProbedMemoryAccess{address:at,bytes:w.bytes(),write:false,is_stack:segment==2&&!p.checkpoint_flags});
            }
            Token::StoreSegment(a,v,w,segment)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment],cache:m.segments[segment]};
                let at=c.translate(values[a].bits as u32,w,true,cpu.eip)?;
                ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,w).map_err(|f|access_error(cpu.eip,f))?;if store_index<MAX_STORES{pending_stores[store_index]=Some((at,w,values[v].bits as u32));}else{extra_stores.push((at,w,values[v].bits as u32));}store_index+=1;
                merge_probed_access(&mut probed,ProbedMemoryAccess{address:at,bytes:w.bytes(),write:true,is_stack:segment==2&&!p.checkpoint_flags});
            }
            Token::RetireIf(c)=>if values[c].bits!=0{cpu.eip=context.next_eip;if let Some(m)=machine.as_deref_mut(){m.retired();}return Ok(probed);},
            Token::ResumeIf(c)=>if values[c].bits!=0{pending_eip=cpu.eip;pending_resume=true;},
            Token::ReadSegment(o,reg)=>values[o]=value(machine.as_deref().ok_or(ExecError::InvalidContext)?.selectors[reg] as u64,16),
            Token::WriteSegment(reg,a)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let r=ram.as_deref();let eip=cpu.eip;
                let (selector,cache,accessed)=m.load_segment(reg,values[a].bits as u16,eip,&mut |at|read_descriptor(r,at,eip))?;
                if let Some((at,v))=accessed{stage_accessed(ram.as_deref(),at,v,eip,&mut pending_stores,&mut extra_stores,&mut store_index)?;}
                pending_segment=Some((reg,selector,cache));
            }
            Token::ReadGpr(o,reg,w)=>values[o]=value(cpu.regs[reg] as u64,w),
            Token::WriteGpr(reg,a,w)=>{pending_registers[reg_count]=(View{register:reg,low:0,width:w},values[a]);reg_count+=1;},
            Token::ReadControl(o,r)=>values[o]=value(machine.as_deref().ok_or(ExecError::InvalidContext)?.read_control(r,cpu.eflags,cpu.eip)? as u64,32),
            Token::WriteControl(r,v)=>{let x=values[v].bits as u32;machine.as_deref().ok_or(ExecError::InvalidContext)?.validate_control(r,x,cpu.eflags,cpu.eip)?;pending_control=Some((r,x));},
            Token::WriteTable(t,l,b)=>{machine.as_deref().ok_or(ExecError::InvalidContext)?;pending_table=Some((t,values[l].bits as u16,values[b].bits as u32));},
            Token::ReadMsw(o)=>values[o]=value(machine.as_deref().ok_or(ExecError::InvalidContext)?.machine_status() as u64,16),
            Token::ReadMsw32(o)=>{let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;values[o]=value(u64::from(m.machine_status())|(u64::from(m.paging)<<31),32)},
            Token::RequireProtectedMode=>if machine.as_deref().ok_or(ExecError::InvalidContext)?.real_mode||cpu.eflags&0x20000!=0{return Err(ExecError::InvalidContext);},
            Token::ReadTable(l,b,t)=>{let (limit,base)=machine.as_deref().ok_or(ExecError::InvalidContext)?.tables[t];values[l]=value(limit as u64,16);values[b]=value(base as u64,32);}
            Token::ReadSystem(o,t)=>values[o]=value(machine.as_deref().ok_or(ExecError::InvalidContext)?.ldt_tr[t].selector as u64,16),
            Token::ReadCpl(o)=>values[o]=value(machine.as_deref().ok_or(ExecError::InvalidContext)?.cpl() as u64,8),
            Token::ReadDescriptor(r,f,a)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let ram=ram.as_deref();let eip=cpu.eip;
                let raw=m.find_descriptor(values[a].bits as u16,&mut |at|read_descriptor(ram,at,eip))?;
                values[r]=value(raw.unwrap_or(0),64);values[f]=value(u64::from(raw.is_some()),1);
            }
            Token::LoadSystem(t,a)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let r=ram.as_deref();let eip=cpu.eip;
                let (loaded,busy)=m.load_system(t,values[a].bits as u16,eip,&mut |at|read_descriptor(r,at,eip))?;
                if let Some((at,v))=busy{stage_accessed(ram.as_deref(),at,v,eip,&mut pending_stores,&mut extra_stores,&mut store_index)?;}
                pending_system=Some((t,loaded));
            }
            Token::Require(c)=>if values[c].bits==0{return Err(ExecError::InvalidContext);},
            Token::RaiseUnless(c,v)=>if values[c].bits==0{return Err(if v==13{ExecError::GeneralProtection{eip:cpu.eip,error_code:0}}else{ExecError::Exception{eip:cpu.eip,vector:v}});},
            Token::ReadAddressAfter(o,reg,a)=>{
                let mut regs=cpu.regs;regs[reg]=values[a].bits as u32;
                let ea=effective.ok_or(ExecError::InvalidContext)?;
                values[o]=value(ea.resolve(&regs,None)?.offset as u64,32);
            }
            Token::ReadAddress(o)=>values[o]=value(address as u64,32),
            Token::Load(o,a,w)=>{
                let m=ram.as_deref_mut().ok_or(ExecError::InvalidContext)?;
                let offset=values[a].bits as u32;
                let addr=match memory_context {Some(c)=>c.translate(offset,w,false,cpu.eip)?,None=>offset};
                let v=m.load(addr,w).map_err(|f|access_error(cpu.eip,f))?;
                values[o]=value(v as u64,w.bits());
                merge_probed_access(&mut probed,ProbedMemoryAccess{address:addr,bytes:w.bytes(),write:false,is_stack:!p.has_read_address});
            }
            Token::StoreIf(_,_,c,_) if values[c].bits==0=>{}
            Token::Store(a,v,w)|Token::StoreIf(a,v,_,w)=>{
                let m=ram.as_deref_mut().ok_or(ExecError::InvalidContext)?;
                let offset=values[a].bits as u32;
                let addr=match memory_context {Some(c)=>c.translate(offset,w,true,cpu.eip)?,None=>offset};
                m.check_write(addr,w).map_err(|f|access_error(cpu.eip,f))?;
                if store_index<MAX_STORES{pending_stores[store_index]=Some((addr,w,values[v].bits as u32));}else{extra_stores.push((addr,w,values[v].bits as u32));}store_index+=1;
                merge_probed_access(&mut probed,ProbedMemoryAccess{address:addr,bytes:w.bytes(),write:true,is_stack:!p.has_read_address});
            }
            Token::ReadIp(o,next,w) => values[o]=value((if next {context.next_eip} else {cpu.eip}) as u64,w),
            Token::ReadDisp(o,w) => values[o]=value(context.displacement as u32 as u64,w),
            Token::CheckWriteSegment(reg,a,w)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][reg],cache:m.segments[reg]};
                let at=c.translate(values[a].bits as u32,w,true,cpu.eip)?;
                ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,w).map_err(|f|access_error(cpu.eip,f))?;
                merge_probed_access(&mut probed,ProbedMemoryAccess{address:at,bytes:w.bytes(),write:true,is_stack:reg==2});
            }
            Token::CheckSegment(reg,a,n)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][reg],cache:m.segments[reg]};
                c.translate_span(values[a].bits,n,false,cpu.eip)?;
            }
            Token::FarBranch(a,b)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let r=ram.as_deref();let eip=cpu.eip;
                let (selector,cache,accessed)=m.load_segment(1,values[a].bits as u16,eip,&mut |at|read_descriptor(r,at,eip))?;let target=values[b].bits as u32;
                if target>cache.limit{return Err(ExecError::GeneralProtection{eip:cpu.eip,error_code:0});}
                if let Some((at,v))=accessed{stage_accessed(ram.as_deref(),at,v,eip,&mut pending_stores,&mut extra_stores,&mut store_index)?;}
                pending_segment=Some((1,selector,cache));pending_eip=target;
            }
            Token::FarJump(a,b)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                pending_transfer=Some(privilege::far_jump(m,ram.as_deref().ok_or(ExecError::InvalidContext)?,values[a].bits as u16,values[b].bits as u32,cpu.eip)?);
            }
            Token::FarReturn(a,b,c,w,n,v)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                pending_transfer=Some(privilege::far_return(m,ram.as_deref().ok_or(ExecError::InvalidContext)?,&cpu.regs,values[a].bits as u16,values[b].bits as u32,values[c].bits as u32,w,n,v.is_some_and(|v|values[v].bits!=0),cpu.eip)?);
            }
            Token::FarCall(a,b)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let mut direct=None;let eip=cpu.eip;
                if let Some(t)=privilege::far_call(m,ram.as_deref().ok_or(ExecError::InvalidContext)?,&cpu.regs,values[a].bits as u16,context.next_eip,eip,&mut direct)?{
                    // A call gate: the whole transfer, nothing else of the description.
                    let m=machine.as_deref_mut().ok_or(ExecError::InvalidContext)?;
                    t.commit(&mut cpu.regs,&mut cpu.eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;
                    m.retired();return Ok(probed);
                }
                let (selector,cache,accessed)=direct.ok_or(ExecError::InvalidContext)?;let target=values[b].bits as u32;
                if target>cache.limit{return Err(ExecError::GeneralProtection{eip,error_code:0});}
                if let Some((at,v))=accessed{stage_accessed(ram.as_deref(),at,v,eip,&mut pending_stores,&mut extra_stores,&mut store_index)?;}
                pending_segment=Some((1,selector,cache));pending_eip=target;
            }
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
                    Binary::Div => if y.bits == 0 { 0 } else { x.bits / y.bits },
                    Binary::Rem => if y.bits == 0 { 0 } else { x.bits % y.bits },
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
                pending_lazy.flush(&mut pending_flags, &mut pending_undefined);
                let m=1u32<<f.bit();pending_flags&=!m;
                if values[d].bits!=0{pending_flags|=(values[a].bits as u32)<<f.bit();pending_undefined&=!m;}else{pending_undefined|=m;}
            },
            Token::Not(o,a) => values[o] = value(!values[a].bits,values[a].width),
            Token::Bit(o,a,i) => values[o] = value((values[a].bits >> i) & 1,1),
            Token::Zero(o,a) => values[o] = value(u64::from(values[a].bits == 0),1),
            Token::Parity(o,a) => values[o] = value(u64::from((values[a].bits as u8).count_ones() % 2 == 0),1),
            Token::Write(a) => pending_dst = Some(values[a]),
            Token::WriteSource(a)=>pending_src=Some(values[a]),
            Token::WriteSourceIf(a,c)=>if values[c].bits!=0{pending_src=Some(values[a]);},
            Token::Flag(f,a) => {
                pending_lazy.flush(&mut pending_flags, &mut pending_undefined);
                let m = 1u32 << f.bit();
                pending_undefined &= !m;
                pending_flags = (pending_flags & !m) | ((values[a].bits as u32) << f.bit());
            }
            Token::Retire => {
                if let(Some(a),Some(b))=(pending_dst,pending_src){
                    let overlap=(mask(dst.width)<<dst.low)&(mask(src.width)<<src.low);
                    if dst.register==src.register&&((a.bits<<dst.low)^(b.bits<<src.low))&overlap!=0{
                        return Err(ExecError::InvalidBinding);
                    }
                }
                for (addr,w,v) in pending_stores.into_iter().flatten().chain(extra_stores) {
                    ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|access_error(cpu.eip,f))?;
                }
                if let Some(x) = pending_dst { cpu.write(dst,x); }
                if let Some(x) = pending_src { cpu.write(src,x); }
                for &(v,x) in &pending_registers[..reg_count] {cpu.write(v,x);}
                cpu.eflags = pending_flags;
                cpu.undefined_flags = pending_undefined;
                *lazy = pending_lazy;
                cpu.eip = pending_eip;
                if let Some(m)=machine.as_deref_mut(){
                    if let Some((r,v))=pending_control{m.commit_control(r,v);}
                    if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}
                    if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}
                    if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}
                    if let Some(t)=pending_transfer{t.commit(&mut cpu.regs,&mut cpu.eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}
                }
                return Ok(probed);
            }
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
fn repeat_description(body:&str,a:u8)->String{
    let body=body.replace("$A",&a.to_string());
    format!("read_gpr count 1 {a}\nzero empty count\nretire_if empty\n{}\nconst one {a} 1\nsub remaining count one\nwrite_gpr 1 remaining\nzero done remaining\nnot again done\nresume_if again\npreserve_flags all\nretire",body.strip_suffix("preserve_flags all\nretire\n").unwrap())
}
fn compare_repeat_description(body:&str,a:u8,equal:bool)->String{
    let body=body.replace("$A",&a.to_string());let tail=if equal{"and again more zf"}else{"not different zf\nand again more different"};
    format!("checkpoint_flags\nread_gpr count 1 {a}\nzero empty count\nretire_if empty\n{}\nconst one {a} 1\nsub remaining count one\nwrite_gpr 1 remaining\nzero done remaining\nnot more done\n{tail}\nresume_if again\nretire",body.strip_suffix("retire\n").unwrap())
}
fn description(name: &str) -> R<String> {
    for (prefix,equal) in [("repe_",true),("repne_",false)]{if let Some(base)=name.strip_prefix(prefix){if matches!(base,"cmps"|"scas"){return Ok(compare_repeat_description(&description(base)?,16,equal));}}}
    if let Some(base)=name.strip_prefix("rep_"){if matches!(base,"movs"|"stos"|"lods"){return Ok(repeat_description(&description(base)?,16));}}

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
        "movzx"|"movzx8" => Some(include_str!("../isa/i386/movzx8.pil")),
        "movzx16" => Some(include_str!("../isa/i386/movzx16.pil")),
        "movsx"|"movsx8" => Some(include_str!("../isa/i386/movsx8.pil")),
        "movsx16" => Some(include_str!("../isa/i386/movsx16.pil")),
        "lea" => Some(include_str!("../isa/i386/lea.pil")),
        "cwd"|"cdq" => Some(include_str!("../isa/i386/cwd.pil")),
        "imul" => Some(include_str!("../isa/i386/imul.pil")),
        "jmp" => Some(include_str!("../isa/i386/jmp.pil")),
        "shl"|"sal" => Some(include_str!("../isa/i386/shl.pil")),
        "shr" => Some(include_str!("../isa/i386/shr.pil")),
        "sar" => Some(include_str!("../isa/i386/sar.pil")),
        "rol" => Some(include_str!("../isa/i386/rol.pil")),
        "ror" => Some(include_str!("../isa/i386/ror.pil")),
        "rcl" => Some(include_str!("../isa/i386/rcl.pil")),
        "daa" => Some(include_str!("../isa/i386/daa.pil")),
        "aam" => Some(include_str!("../isa/i386/aam.pil")),
        "aaa" => Some(include_str!("../isa/i386/aaa.pil")),
        "aas" => Some(include_str!("../isa/i386/aas.pil")),
        "aad" => Some(include_str!("../isa/i386/aad.pil")),
        "das" => Some(include_str!("../isa/i386/das.pil")),
        "rcr" => Some(include_str!("../isa/i386/rcr.pil")),
        "clc" => Some(include_str!("../isa/i386/clc.pil")),
        "call" => Some(include_str!("../isa/i386/call.pil")),
        "ret" => Some(include_str!("../isa/i386/ret.pil")),
        "retf" => Some(include_str!("../isa/i386/retf.pil")),
        "iret" => Some(include_str!("../isa/i386/iret.pil")),
        "iret_protected" => Some(include_str!("../isa/i386/iret_protected.pil")),
        "call_far" => Some(include_str!("../isa/i386/call_far.pil")),
        "movs" => Some(include_str!("../isa/i386/movs.pil")),
        "stos" => Some(include_str!("../isa/i386/stos.pil")),
        "lods" => Some(include_str!("../isa/i386/lods.pil")),
        "push" => Some(include_str!("../isa/i386/push.pil")),
        "push_memory" => Some(include_str!("../isa/i386/push_memory.pil")),
        "pop" => Some(include_str!("../isa/i386/pop.pil")),
        "pop_memory" => Some(include_str!("../isa/i386/pop_memory.pil")),
        "smsw" => Some(include_str!("../isa/i386/smsw.pil")),
        "pushf" => Some(include_str!("../isa/i386/pushf.pil")),
        "popf" => Some(include_str!("../isa/i386/popf.pil")),
        "cli" => Some(include_str!("../isa/i386/cli.pil")),
        "sti" => Some(include_str!("../isa/i386/sti.pil")),
        "jcxz" => Some(include_str!("../isa/i386/jcxz.pil")),
        "leave" => Some(include_str!("../isa/i386/leave.pil")),
        "cmps" => Some(include_str!("../isa/i386/cmps.pil")),
        "scas" => Some(include_str!("../isa/i386/scas.pil")),
        "cld" => Some(include_str!("../isa/i386/cld.pil")),
        "std" => Some(include_str!("../isa/i386/std.pil")),
        "stc" => Some(include_str!("../isa/i386/stc.pil")),
        "cmc" => Some(include_str!("../isa/i386/cmc.pil")),
        "lahf" => Some(include_str!("../isa/i386/lahf.pil")),
        "sahf" => Some(include_str!("../isa/i386/sahf.pil")),
        "xchg" => Some(include_str!("../isa/i386/xchg.pil")),
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
    Err("unknown description: use add/sub/cmp/adc/sbb/and/or/xor/test/not/neg/inc/dec/nop/imul/movzx8/movzx16/movsx8/movsx16/lea/cwd/shl/sal/shr/sar/rol/ror/clc/stc/cmc/cld/std/lahf/sahf/xchg, set<condition>, j<condition>, or write_if_z".into())
}
fn instantiate(text: impl AsRef<str>, w: u8) -> String {
    text.as_ref().replace("$WRAP_GUARD","").replace("$HIGH_MASK","4294901760").replace("$FAR_TARGET","const target_value $W 0\nconst selector 16 0").replace("$TARGET","read_disp delta $W\nadd target_value return_offset delta").replace("$CLEANUP","0").replace("$FRAME",&(w/4).to_string()).replace("$SEG","3").replace("$A","16").replace("$S\n","16\n").replace("$S ","16 ").replace("$BYTES",&(w/8).to_string()).replace("$REG","0").replace("$DOUBLE",&(2*w as u16).to_string()).replace("$EXT",&(w+1).to_string()).replace("$SIGN",&(w-1).to_string()).replace("$W",&w.to_string())
}
fn emit_rust(p: &Program) -> String {
    let mut out=String::from("// Generated from validated PIL tokens.\n");
    out.push_str(include_str!("flow.rs"));
    // The included module has an inner documentation comment; in the emitted
    // standalone body it must be an ordinary comment.
    out.push_str("\npub mod memory {\n");out.push_str(include_str!("memory.rs").split("#[cfg(test)]").next().unwrap());out.push_str("\n}\npub use memory::*;\n");
    out.push_str("\npub mod vga_planar {\n");out.push_str(include_str!("vga_planar.rs").split("#[cfg(test)]").next().unwrap());out.push_str("\n}\n");
    out.push_str("\npub mod address {\n");
    out.push_str(include_str!("address.rs").split("#[cfg(test)]").next().unwrap());
    out.push_str("\n}\n");
    out.push_str("\npub mod privilege {\n");
    out.push_str(include_str!("privilege.rs").split("#[cfg(test)]").next().unwrap());
    out.push_str("\n}\n");
    out=out.lines().map(|line|line.strip_prefix("//!").map(|s|format!("//{s}")).unwrap_or_else(||line.to_owned())).collect::<Vec<_>>().join("\n");
    out.push_str(&emit_rust_functions(p));out
}
// Shared support types can be supplied by a compiled guest runner instead of
// duplicated in each generated instruction module.
fn emit_rust_functions(p:&Program)->String {
    let mut out=String::new();
    // A segment load may stage one more byte store: the descriptor's accessed bit.
    let stores=p.tokens.iter().filter(|t|matches!(t,Token::Store(..)|Token::StoreIf(..)|Token::StoreSegment(..)|Token::WriteSegment(..)|Token::FarBranch(..)|Token::FarCall(..)|Token::LoadSystem(..))).count().max(1);
    out.push_str(&format!(
        "\nfn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {{\n\
         let w = {}u8;\n\
         if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + {}u16 > 32 {{ return Err(ExecError::InvalidBinding); }}\n\
         context.validate(w,{})?;\n\
         if {} && ram.is_none() {{ return Err(ExecError::InvalidContext); }}\n\
         let mut pending_control: Option<(usize,u32)> = None;\n\
         let mut pending_table: Option<(usize,u16,u32)> = None;\n\
         let mut pending_system: Option<(usize,address::SystemSegment)> = None;\n\
         let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;\n\
         let mut pending_transfer: Option<privilege::Transfer> = None;\n\
         let mut pending_stores: [Option<(u32,MemWidth,u32)>;{}] = [None;{}];let mut store_index=0;\n\
         let mut pending_eip = context.next_eip;\n\
         let mut pending_resume = false;\nlet mut pending_interrupt_shadow = false;\n\
         let mut pending_flags = *flags;\n\
         let mut pending_undefined = *undefined;\n\
         let mut pending_dst: Option<u64> = None;\n\
         let mut pending_src: Option<u64> = None;\n\
         let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();\n",
        p.operand_width,p.source_width,p.uses_displacement,p.uses_memory,stores,stores));
    for t in &p.tokens {
        let line = match *t {
            Token::CheckpointFlags=>"machine.as_deref().ok_or(ExecError::InvalidContext)?;".into(),
            Token::ReadFlags(o)=>format!("let v{o}:u64 = *flags as u64;"),
            Token::RequireRealMode=>"if !machine.as_deref().ok_or(ExecError::InvalidContext)?.real_mode||*flags&0x20000!=0 {return Err(ExecError::InvalidContext);}".into(),
            Token::RequireRing0=>"if machine.as_deref().ok_or(ExecError::InvalidContext)?.cpl()!=0 {return Err(ExecError::GeneralProtection{eip:*eip,error_code:0});}".into(),
            Token::RequireIopl=>"{let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;if !m.real_mode&&u32::from(m.cpl())>(*flags>>12)&3 {return Err(ExecError::GeneralProtection{eip:*eip,error_code:0});}}".into(),
            Token::CheckIo(a,n)=>format!("{{let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let r=ram.as_deref().ok_or(ExecError::InvalidContext)?;let e=*eip;m.io_permitted(*flags,v{a} as u16,{n},e,&mut |at|r.as_level(false,|r|r.load(at,MemWidth::Byte)).map(|v|v as u8).map_err(|f|f.error(e)))?;}}"),
            Token::FarJump(a,b)=>format!("pending_transfer=Some(privilege::far_jump(machine.as_deref().ok_or(ExecError::InvalidContext)?,ram.as_deref().ok_or(ExecError::InvalidContext)?,v{a} as u16,v{b} as u32,*eip)?);"),
            Token::FarReturn(a,b,c,w,n,v)=>format!("pending_transfer=Some(privilege::far_return(machine.as_deref().ok_or(ExecError::InvalidContext)?,ram.as_deref().ok_or(ExecError::InvalidContext)?,regs,v{a} as u16,v{b} as u32,v{c} as u32,MemWidth::{w:?},{n},{},*eip)?);",v.map(|v|format!("v{v}!=0")).unwrap_or("false".into())),
            Token::FarCall(a,b)=>format!("let mut direct=None;if let Some(t)=privilege::far_call(machine.as_deref().ok_or(ExecError::InvalidContext)?,ram.as_deref().ok_or(ExecError::InvalidContext)?,regs,v{a} as u16,context.next_eip,*eip,&mut direct)?{{let m=machine.as_deref_mut().ok_or(ExecError::InvalidContext)?;t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;m.retired();return Ok(());}}let (selector,cache,accessed)=direct.ok_or(ExecError::InvalidContext)?;if v{b}>u64::from(cache.limit){{return Err(ExecError::GeneralProtection{{eip:*eip,error_code:0}});}}if let Some((at,v))=accessed{{ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::Byte).map_err(|f|f.error(*eip))?;pending_stores[store_index]=Some((at,MemWidth::Byte,u32::from(v)));store_index+=1;}}pending_segment=Some((1,selector,cache));pending_eip=v{b} as u32;"),
            Token::InterruptShadowIf(c)=>format!("machine.as_deref().ok_or(ExecError::InvalidContext)?;pending_interrupt_shadow=v{c}!=0;"),
            Token::Read(o,b,w) => {
                let binding = match b { Binding::Dst => "dst", Binding::Src => "src" };
                format!("let v{o}: u64 = ((regs[{binding}.0] as u64) >> {binding}.1) & {}u64;",mask(w))
            }
            Token::LoadSegment(o,a,w,segment)=>format!("let cache=machine.as_deref().ok_or(ExecError::InvalidContext)?.segments[{segment}];let at=address::MemoryContext{{segment:address::SegmentRegister::{:?},cache}}.translate(v{a} as u32,MemWidth::{w:?},false,*eip)?;let v{o}:u64=ram.as_deref().ok_or(ExecError::InvalidContext)?.load_staged(at,MemWidth::{w:?},&pending_stores[..store_index],&[]).map_err(|f|f.error(*eip))? as u64;",[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment]),
            Token::StoreSegment(a,v,w,segment)=>format!("let cache=machine.as_deref().ok_or(ExecError::InvalidContext)?.segments[{segment}];let at=address::MemoryContext{{segment:address::SegmentRegister::{:?},cache}}.translate(v{a} as u32,MemWidth::{w:?},true,*eip)?;ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::{w:?}).map_err(|f|f.error(*eip))?;pending_stores[store_index]=Some((at,MemWidth::{w:?},v{v} as u32));store_index+=1;",[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment]),
            Token::RetireIf(c)=>format!("if v{c}!=0 {{*eip=context.next_eip;if let Some(m)=machine.as_deref_mut(){{m.retired();}}return Ok(());}}"),
            Token::ResumeIf(c)=>format!("if v{c}!=0 {{pending_eip=*eip;pending_resume=true;}}"),
            Token::ReadSegment(o,reg)=>format!("let v{o}: u64 = machine.as_deref().ok_or(ExecError::InvalidContext)?.selectors[{reg}] as u64;"),
            Token::WriteSegment(reg,a)=>format!("{}pending_segment=Some(({reg},selector,cache));",emit_segment_load(&reg.to_string(),a)),
            Token::ReadGpr(o,reg,w)=>format!("let v{o}: u64 = (regs[{reg}] as u64) & {}u64;",mask(w)),
            Token::WriteGpr(reg,a,w)=>format!("pending_registers.push(({reg},{w},v{a}));"),
            Token::ReadControl(o,r)=>format!("let v{o}:u64=machine.as_deref().ok_or(ExecError::InvalidContext)?.read_control({r},*flags,*eip)? as u64;"),
            Token::WriteControl(r,v)=>format!("machine.as_deref().ok_or(ExecError::InvalidContext)?.validate_control({r},v{v} as u32,*flags,*eip)?;pending_control=Some(({r},v{v} as u32));"),
            Token::WriteTable(t,l,b)=>format!("machine.as_deref().ok_or(ExecError::InvalidContext)?;pending_table=Some(({t},v{l} as u16,v{b} as u32));"),
            Token::ReadMsw(o)=>format!("let v{o}:u64=machine.as_deref().ok_or(ExecError::InvalidContext)?.machine_status() as u64;"),
            Token::ReadMsw32(o)=>format!("let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let v{o}:u64=u64::from(m.machine_status())|(u64::from(m.paging)<<31);"),
            Token::RequireProtectedMode=>"if machine.as_deref().ok_or(ExecError::InvalidContext)?.real_mode||*flags&0x20000!=0 {return Err(ExecError::InvalidContext);}".into(),
            Token::ReadTable(l,b,t)=>format!("let (v{l},v{b}):(u64,u64)={{let (l,b)=machine.as_deref().ok_or(ExecError::InvalidContext)?.tables[{t}];(l as u64,b as u64)}};"),
            Token::ReadSystem(o,t)=>format!("let v{o}:u64=machine.as_deref().ok_or(ExecError::InvalidContext)?.ldt_tr[{t}].selector as u64;"),
            Token::ReadCpl(o)=>format!("let v{o}:u64=machine.as_deref().ok_or(ExecError::InvalidContext)?.cpl() as u64;"),
            Token::ReadDescriptor(r,f,a)=>format!("let (v{r},v{f}):(u64,u64)={{let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let raw=m.find_descriptor(v{a} as u16,&mut |at|{}(at))?;(raw.unwrap_or(0),u64::from(raw.is_some()))}};",EMIT_DESCRIPTOR_READ),
            Token::LoadSystem(t,a)=>format!("{{let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let (loaded,busy)=m.load_system({t},v{a} as u16,*eip,&mut |at|{}(at))?;if let Some((at,v))=busy{{ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::Byte).map_err(|f|f.error(*eip))?;pending_stores[store_index]=Some((at,MemWidth::Byte,u32::from(v)));store_index+=1;}}pending_system=Some(({t},loaded));}}",EMIT_DESCRIPTOR_READ),
            Token::Require(c)=>format!("if v{c}==0{{return Err(ExecError::InvalidContext);}}"),
            Token::RaiseUnless(c,v)=>if v==13{format!("if v{c}==0{{return Err(ExecError::GeneralProtection{{eip:*eip,error_code:0}});}}")}else{format!("if v{c}==0{{return Err(ExecError::Exception{{eip:*eip,vector:{v}}});}}")},
            Token::ReadAddressAfter(o,reg,a)=>format!("let mut updated=*regs;updated[{reg}]=v{a} as u32;let v{o}:u64=effective.ok_or(ExecError::InvalidContext)?.resolve(&updated,None)?.offset as u64;"),
            Token::ReadAddress(o)=>format!("let v{o}: u64 = address as u64;"),
            Token::Load(o,a,w)=>format!("let addr{o}=match memory_context {{Some(c)=>c.translate(v{a} as u32,MemWidth::{w:?},false,*eip)?,None=>v{a} as u32}}; let v{o}: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr{o},MemWidth::{w:?}).map_err(|f|f.error(*eip))? as u64;"),
            Token::Store(a,v,w)=>format!("let store_addr=match memory_context {{Some(c)=>c.translate(v{a} as u32,MemWidth::{w:?},true,*eip)?,None=>v{a} as u32}}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::{w:?}).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::{w:?},v{v} as u32));store_index+=1;"),
            Token::StoreIf(a,v,c,w)=>format!("if v{c}!=0{{let store_addr=match memory_context {{Some(c)=>c.translate(v{a} as u32,MemWidth::{w:?},true,*eip)?,None=>v{a} as u32}}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::{w:?}).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::{w:?},v{v} as u32));store_index+=1;}}"),
            Token::ReadIp(o,next,w) => format!("let v{o}: u64 = ({} as u64) & {}u64;",if next {"context.next_eip"} else {"*eip"},mask(w)),
            Token::ReadDisp(o,w) => format!("let v{o}: u64 = (context.displacement as u32 as u64) & {}u64;",mask(w)),
            Token::CheckWriteSegment(reg,a,w)=>format!("let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let at=address::MemoryContext{{segment:address::SegmentRegister::{:?},cache:m.segments[{reg}]}}.translate(v{a} as u32,MemWidth::{w:?},true,*eip)?;ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::{w:?}).map_err(|f|f.error(*eip))?;",[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][reg]),
            Token::CheckSegment(reg,a,n)=>format!("let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;address::MemoryContext{{segment:address::SegmentRegister::{:?},cache:m.segments[{reg}]}}.translate_span(v{a},{n},false,*eip)?;",[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][reg]),
            Token::FarBranch(a,b)=>format!("{}if v{b}>u64::from(cache.limit){{return Err(ExecError::GeneralProtection{{eip:*eip,error_code:0}});}}pending_segment=Some((1,selector,cache));pending_eip=v{b} as u32;",emit_segment_load("1",a)),
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
                    Binary::Div => format!("if v{b} == 0 {{ 0 }} else {{ v{a} / v{b} }}"),
                    Binary::Rem => format!("if v{b} == 0 {{ 0 }} else {{ v{a} % v{b} }}"),
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
            Token::WriteSource(a)=>format!("pending_src = Some(v{a});"),
            Token::WriteSourceIf(a,c)=>format!("if v{c} != 0 {{pending_src=Some(v{a});}}"),
            Token::Flag(f,a) => format!("pending_undefined &= !{}u32; pending_flags = (pending_flags & !{}u32) | ((v{a} as u32) << {});",1u32 << f.bit(),1u32 << f.bit(),f.bit()),
            Token::Retire => format!(
                "if let(Some(a),Some(b))=(pending_dst,pending_src) {{ let overlap=({}u64<<dst.1)&({}u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {{return Err(ExecError::InvalidBinding);}} }}\nfor (addr,w,v) in pending_stores.into_iter().flatten() {{ ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }}\nif let Some(value) = pending_dst {{ let m = {}u64 << dst.1;\nregs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }}\nif let Some(value)=pending_src {{let m={}u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}}\nfor (reg,width,value) in pending_registers {{let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}}\n*flags = pending_flags;\n*undefined = pending_undefined;\n*eip = pending_eip;\nif let Some(m)=machine.as_deref_mut(){{if let Some((r,v))=pending_control{{m.commit_control(r,v);}}if let Some((t,l,b))=pending_table{{m.tables[t]=(l,b);}}if let Some((t,loaded))=pending_system{{m.ldt_tr[t]=loaded;}}if !pending_resume{{m.retired();}}if pending_interrupt_shadow{{m.sti_shadow=1;}}if let Some((reg,selector,cache))=pending_segment{{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{{m.ss_shadow=1;}}}}if let Some(t)=pending_transfer{{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}}}\nOk(())",mask(p.operand_width),mask(p.source_width),mask(p.operand_width),mask(p.source_width)),
        };
        out.push_str(&line); out.push('\n');
    }
    out.push_str("}\n");
    out.push_str("pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{\n");
    if p.uses_memory {out.push_str("Err(ExecError::InvalidContext)\n}\n");}
    else {out.push_str("execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)\n}\n");}
    out.push_str("pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{\nexecute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)\n}\n");
    out.push_str("pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{\nlet (offset,memory_context)=address.resolve(regs)?;\nexecute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))\n}\n");
    out.push_str(&r#"pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if CHECKPOINT{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&CHECKPOINT{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
"#.replace("CHECKPOINT",&p.checkpoint_flags.to_string()));
    out.push_str("pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {\n");
    if p.uses_flow || p.uses_memory {out.push_str("Err(\"flow context required: use execute_with_flow\")\n");}
    else {out.push_str("let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};\nexecute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|\"invalid execution context\")\n");}
    out.push_str("}\n// Compatibility helper: discards diagnostic undefined-flag metadata.\npub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {\nlet mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)\n}\n"); out
}
const CLI_HELP:&str = "PIL - Processor Instruction Language
Usage: pil [OPTIONS] COMMAND [ARGUMENTS]

DOS and ROM:
  --pildos-startup [CONFIG]          Read configured program and arguments as JSON
  --dos-session [--config FILE] [--mount DIR] [--args TEXT] PROGRAM
      Interactive JSON service; PROGRAM can be IMAGE.img::INNER_PATH.
  --dos-probe [--steps N] PROGRAM...   Bounded DOS compatibility probe
  --run-rom PATH [--steps N]          Run a 64 or 128 KiB ROM
  --dos-list [--all] IMAGE|ZIP        List container entries
  --dos-read IMAGE|ZIP INNER [DEST]   Read/extract one entry
  --dos-extract IMAGE|ZIP NEW_DIR     Extract a container
  --dos-unpack-drive IMAGE NEW_DIR    Copy a FAT image into a new writable folder
  --dos-drive-name IMAGE              Suggest a labelled host-folder name

Debugger and checks:
  --repl [--load PATH]               Interactive debugger
  --debug-service [--load PATH]      JSON debugger service
  --selftest                        Run built-in execution checks
  --numeric-test                    Check numeric instructions
  --verify-backends                 Compare execution backends
  --compare-backends [N] [SAMPLES]   Compare and benchmark backends
  --bench [N] [SAMPLES] [--uncached] Interpreter benchmark
  --cpu-bench [N] [SAMPLES]          Native CPU benchmark
  --stress [PASSES]                 Factorial stress check
  --write-dos-mandel PATH            Write VGA Mandelbrot DOS program
  --mandelbrot [SIZE] [ITERATIONS]   Render Mandelbrot
  --compare-mandelbrot [SIZE] [ITERATIONS]

Code generation and examples:
  --emit-rust 8|16|32 [DESCRIPTION]  Emit a Rust instruction implementation
  --emit-suite-rust [N] [SAMPLES]    Emit backend verification suite
  --write-selftest PATH             Write a raw example
  --write-hello PATH                Write a raw hello example
  --write-factorials PATH           Write a factorial example
  --write-loop PATH                 Write a loop example
  --write-dos-hello com|exe PATH     Write a DOS hello (new file only)
  --emit-dos-backend INPUT OUTPUT    Profile a DOS COM/EXE and emit a partial Rust backend
  --demo-branch                     Show a conditional branch example

Options (before COMMAND):
  --stop-log PATH                   Append DOS-session/ROM diagnostics (default: PILdos-stops.jsonl)
  --profile JSON_PATH               Write DOS host timings on normal exit
  --ignore-unassigned-out           Ignore/log unassigned OUT ports
  --help, -h                        Show this help

For a DOS window use: python tools/dos_terminal.py --help
";
fn main() -> R<()> {
    let mut dos_args:Vec<_>=std::env::args().collect();
    if dos_args.get(1).map(String::as_str)==Some("--emit-dos-backend"){
        if dos_args.len()!=4{return Err("usage: pil --emit-dos-backend INPUT.COM|INPUT.EXE OUTPUT.rs".into());}
        let path=std::path::Path::new(&dos_args[2]);
        let name=path.file_name().and_then(|n|n.to_str()).ok_or("DOS program name must be Unicode")?;
        let bytes=dos::read_file(path)?;
        let (source,status)=dos_compile::emit(&bytes,name)?;
        use std::io::Write;
        let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&dos_args[3]).map_err(|e|e.to_string())?;
        file.write_all(source.as_bytes()).map_err(|e|e.to_string())?;
        println!("{status}; wrote {}",dos_args[3]);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--pildos-startup"){
        if dos_args.len()>3{return Err("usage: pil --pildos-startup [CONFIG]".into());}
        let(program,args)=dos_config::startup(dos_args.get(2).map(String::as_str))?;
        println!("{{\"program\":{},\"args\":{}}}",dos::session::quote(&program),args.as_deref().map(dos::session::quote).unwrap_or_else(||"null".into()));return Ok(());
    }
    if dos_args.len()==1 || matches!(dos_args.get(1).map(String::as_str),Some("--help"|"-h")){
        if dos_args.len()>2{return Err("--help accepts no arguments".into());}
        print!("{CLI_HELP}");return Ok(());
    }
    let mut _profile=None;
    let mut stop_log=String::from("PILdos-stops.jsonl");
    let mut log_given=false;
    loop {match dos_args.get(1).map(String::as_str){
        Some("--stop-log")=>{if log_given{return Err("duplicate --stop-log".into());}stop_log=dos_args.get(2).ok_or("--stop-log requires a path (empty disables logging)")?.clone();log_given=true;dos_args.drain(1..3);},
        Some("--profile")=>{if _profile.is_some(){return Err("duplicate --profile".into());}let path=dos_args.get(2).ok_or("--profile requires an output JSON path")?;_profile=Some(profile::start(path)?);dos_args.drain(1..3);},
        Some("--ignore-unassigned-out")=>{dos::ports::IGNORE_UNASSIGNED_OUT.store(true,std::sync::atomic::Ordering::Relaxed);dos_args.remove(1);},
        _=>break,
    }}
    if dos_args.get(1).map(String::as_str)==Some("--dos-list"){
        if dos_args.len()<3||dos_args.len()>4{return Err("usage: pil --dos-list [--all] IMAGE.img|ARCHIVE.zip".into());}
        let(all,path)=if dos_args[2]=="--all"{(true,dos_args.get(3).ok_or("missing image path")?)}else{(false,&dos_args[2])};
        println!("{}",if all{containers::listing_all(std::path::Path::new(path))?}else{containers::listing(std::path::Path::new(path))?});return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-read"){
        if dos_args.len()<4||dos_args.len()>5{return Err("usage: pil --dos-read IMAGE.img|ARCHIVE.zip INNER_PATH [DEST]".into());}
        let bytes=containers::read_entry(std::path::Path::new(&dos_args[2]),&dos_args[3])?;
        if let Some(dest)=dos_args.get(4){
            use std::io::Write;
            let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(dest).map_err(|e|e.to_string())?;
            f.write_all(&bytes).map_err(|e|e.to_string())?;println!("extracted {} ({} bytes) to {}",dos_args[3],bytes.len(),dest);
        }else{
            use std::io::Write;std::io::stdout().write_all(&bytes).map_err(|e|e.to_string())?;
        }return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-extract"){
        if dos_args.len()!=4{return Err("usage: pil --dos-extract IMAGE.img|ARCHIVE.zip NEW_DIRECTORY".into());}
        println!("{}",containers::extract(std::path::Path::new(&dos_args[2]),std::path::Path::new(&dos_args[3]))?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-unpack-drive"){
        if dos_args.len()!=4{return Err("usage: pil --dos-unpack-drive IMAGE.img NEW_DIRECTORY".into());}
        println!("{}",containers::extract_drive(std::path::Path::new(&dos_args[2]),std::path::Path::new(&dos_args[3]))?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-drive-name"){
        if dos_args.len()!=3{return Err("usage: pil --dos-drive-name IMAGE.img".into());}
        println!("{}",containers::drive_image_name(std::path::Path::new(&dos_args[2]))?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--write-dos-mandel"){
        if dos_args.len()!=3{return Err("usage: pil --write-dos-mandel PATH".into());}
        use std::io::Write;
        let bytes=mandelbrot::guest_vga_bytes(128,32)?;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&dos_args[2]).map_err(|e|e.to_string())?;
        f.write_all(&bytes).map_err(|e|e.to_string())?;println!("wrote {}",dos_args[2]);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--write-dos-hello"){
        if dos_args.len()!=4{return Err("usage: pil --write-dos-hello com|exe PATH (new file only)".into());}
        let bytes=dos::hello(&dos_args[2])?;
        use std::io::Write;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&dos_args[3]).map_err(|e|e.to_string())?;
        f.write_all(&bytes).map_err(|e|e.to_string())?;println!("wrote original DOS hello {} ({} bytes)",dos_args[3],bytes.len());return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-session"){
        let usage="usage: pil --dos-session [--config FILE] [--mount HOST_FOLDER] [--args COMMAND_LINE] PROGRAM.COM|PROGRAM.EXE|IMAGE.img::INNER_PATH";
        let(mut mount,mut tail,mut config,mut rest)=(None,None,None,&dos_args[2..]);
        while let[flag,value,more@..]=rest{
            match flag.as_str(){"--mount" if mount.is_none()=>mount=Some(value.as_str()),"--args" if tail.is_none()=>tail=Some(value.as_str()),"--config" if config.is_none()=>config=Some(value.as_str()),_=>break}
            rest=more;
        }
        let[program]=rest else{return Err(usage.into())};
        return dos::session::serve(program,mount,tail,&stop_log,config).map_err(Into::into);
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-probe"){
        let mut at=2;let mut budget=10000;
        if dos_args.get(at).map(String::as_str)==Some("--steps"){
            let n=dos_args.get(at+1).ok_or("--steps requires a budget")?;
            budget=usize::try_from(number(n)?).map_err(|_|"budget out of range")?;at+=2;
        }
        println!("{}",dos::report(&dos_args[at..],budget)?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--run-rom"){
        if dos_args.len()<3||dos_args.len()>5{return Err("usage: pil --run-rom PATH [--steps N]".into());}
        let rom_path=std::path::Path::new(&dos_args[2]);
        let mut budget=dos::MAX_STEPS;
        if dos_args.len()==5&&dos_args[3]=="--steps"{
            budget=usize::try_from(number(&dos_args[4])?).map_err(|_|"budget out of range")?;
        }else if dos_args.len()!=3{
            return Err("usage: pil --run-rom PATH [--steps N]".into());
        }
        let probe=dos::run_rom(rom_path,budget)?;
        dos::stoplog::Logger::new(&stop_log,&dos_args[2]).record(&probe.loaded,match probe.stop{dos::Stop::Budget=>"running",dos::Stop::WaitingForInput=>"waiting_input",dos::Stop::Exit(_)=>"exit",_=>"stopped"},&format!("{:?}",probe.stop),probe.completed,&probe.exceptions,probe.exception_count);
        print!("{}",dos::format_rom_report(&probe));
        return Ok(());
    }

    let args = dos_args;
    if matches!(args.get(1).map(String::as_str),Some("--mandelbrot"|"--compare-mandelbrot")) {
        if args.len()>4{return Err("usage: pil --mandelbrot|--compare-mandelbrot [size 2..256 power of two] [iterations 1..64]".into());}
        let parse=|i:usize,default:u32|->R<u32>{if let Some(x)=args.get(i){u32::try_from(number(x.as_str())?).map_err(|_|"parameter out of range".into())}else{Ok(default)}};
        let (size,iterations)=(parse(2,256)?,parse(3,32)?);
        println!("{}",if args[1]=="--compare-mandelbrot"{native::compare_mandelbrot(size,iterations)?}else{mandelbrot::render_files(size,iterations)?});return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--verify-backends") {
        if args.len()!=2{return Err("usage: pil --verify-backends".into());}
        println!("{}",native::verify()?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--cpu-bench") {
        if args.len()>4{return Err("usage: pil --cpu-bench [iterations] [samples]".into());}
        let n=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{10000};
        let samples=if let Some(x)=args.get(3){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{5};
        println!("{}",native::cpu_benchmark(n,samples)?);return Ok(());
    }
    if matches!(args.get(1).map(String::as_str),Some("--compare-backends"|"--emit-suite-rust")) {
        if args.len()>4{return Err("usage: pil --compare-backends|--emit-suite-rust [iterations] [samples]".into());}
        let n=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{10000};
        let samples=if let Some(x)=args.get(3){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{5};
        if args[1]=="--emit-suite-rust"{print!("{}",native::emit_suite(n,samples)?);}else{println!("{}",native::compare(n,samples)?);}
        return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-factorials") || args.get(1).map(String::as_str)==Some("--write-loop") {
        let path=args.get(2).ok_or("usage: pil --write-factorials|--write-loop PATH")?;
        let bytes=if args[1]=="--write-factorials"{guest::factorial_bytes()?}else{guest::loop_bytes(guest::LOOP_ITERATIONS)?};
        std::fs::write(path,bytes).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--numeric-test") {return debugger::numeric_test();}
    if args.get(1).map(String::as_str)==Some("--bench") {
        let mut counts=Vec::new();let mut cached=true;
        for arg in &args[2..] {
            if arg=="--uncached" && cached {cached=false;}else{counts.push(arg);}
        }
        if counts.len()>2{return Err("usage: pil --bench [iterations] [samples] [--uncached]".into());}
        let n=if let Some(x)=counts.first(){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{1000};
        let samples=if let Some(x)=counts.get(1){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{3};
        println!("{}",debugger::benchmark_mode(n,samples,cached)?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--stress") {
        if args.len()>3{return Err("usage: pil --stress [passes]".into());}
        let passes=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"passes out of range")?}else{25};
        println!("{}",debugger::factorial_stress(passes)?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-hello") {
        let path=args.get(2).ok_or("usage: pil --write-hello PATH")?;
        std::fs::write(path,guest::hello_bytes()?).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-selftest") {
        let path=args.get(2).ok_or("usage: pil --write-selftest PATH")?;
        std::fs::write(path,guest::selftest_bytes()?).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--selftest") {
        return debugger::selftest();
    }
    if args.get(1).map(String::as_str)==Some("--debug-service") {
        let path=match args.len(){2=>None,4 if args[2]=="--load"=>Some(args[3].as_str()),_=>return Err("usage: pil --debug-service [--load RAW_BINARY]".into())};
        return debugger::serve(std::io::stdin().lock(),std::io::stdout().lock(),path).map_err(|e|e.to_string());
    }
    if args.get(1).map(String::as_str)==Some("--repl") {
        if args.len()>2 {
            if args.len()!=4 || args[2]!="--load" {return Err("usage: pil --repl [--load PATH]".into());}
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
        let w = args.get(2).ok_or("usage: pil --emit-rust 8|16|32 [description]")?;
        let w = width(w)?;
        let name = args.get(3).map(String::as_str).unwrap_or("add");
        let p = compile(&instantiate(description(name)?,w),w)?;
        print!("{}",emit_rust(&p)); return Ok(());
    }
    Err(format!("unknown command or option: {}. Use pil --help for usage.",args.get(1).map(String::as_str).unwrap_or("(missing command)")))
}

#[cfg(test)]
#[path = "../tests/unit/cpu.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/memory_cpu.rs"]
mod memory_tests;

#[cfg(test)]
#[path = "../tests/unit/segment_cpu.rs"]
mod segment_tests;

#[cfg(test)]
#[path = "../tests/unit/host_cpu.rs"]
mod host_cpu_tests;

#[cfg(test)]
#[path = "../tests/unit/effects.rs"]
mod effect_tests;

#[cfg(test)]
#[path = "../tests/unit/flag_transfer.rs"]
mod flag_transfer_tests;
#[cfg(test)]
#[path = "../tests/unit/direction_flag.rs"]
mod direction_flag_tests;


#[cfg(test)]
#[path = "../tests/unit/stack.rs"]
mod stack_tests;

#[cfg(test)]
#[path = "../tests/unit/segments.rs"]
mod segment_transfer_tests;

#[cfg(test)]
#[path = "../tests/unit/mov_forms.rs"]
mod mov_form_tests;

#[cfg(test)]
#[path = "../tests/unit/calls_strings.rs"]
mod calls_strings_tests;

#[cfg(test)]
#[path = "../tests/unit/control_flags.rs"]
mod control_flag_tests;

#[cfg(test)]
#[path = "../tests/unit/leave.rs"]
mod leave_tests;

#[cfg(test)]
#[path = "../tests/unit/compare_strings.rs"]
mod compare_string_tests;

#[cfg(test)]
#[path = "../tests/unit/retf.rs"]
mod retf_tests;

#[cfg(test)]
#[path = "../tests/unit/real_mode_forms.rs"]
mod real_mode_form_tests;
#[cfg(test)]
#[path = "../tests/unit/protected_mode.rs"]
mod protected_mode_tests;
#[cfg(test)]
#[path = "../tests/unit/pm_interrupts.rs"]
mod pm_interrupt_tests;
#[cfg(test)]
#[path = "../tests/unit/privilege.rs"]
mod privilege_tests;
#[cfg(test)]
#[path = "../tests/unit/v86.rs"]
mod v86_tests;
#[cfg(test)]
#[path = "../tests/unit/paging.rs"]
mod paging_tests;
#[cfg(test)]
#[path = "../tests/unit/system_registers.rs"]
mod system_registers_tests;

#[cfg(test)]
#[path = "../tests/unit/pop_msw.rs"]
mod pop_msw_tests;

#[cfg(test)]
#[path = "../tests/unit/push_memory.rs"]
mod push_memory_tests;

#[cfg(test)]
#[path = "../tests/unit/call_far.rs"]
mod call_far_tests;


#[cfg(test)]
#[path = "../tests/unit/exchange_memory.rs"]
mod exchange_memory_tests;

#[cfg(test)]
#[path = "../tests/unit/les.rs"]
mod les_tests;

#[cfg(test)]
#[path = "../tests/unit/sst80386.rs"]
mod sst80386_tests;


#[cfg(test)]
#[path="../tests/unit/control_registers.rs"]
mod control_register_tests;
