//! PIL proof: text -> validated typed tokens -> register/flag interpreter.
//! Token interpreter, small 386 decoder and Rust-source emitter; no external crates.
use std::collections::HashMap;
mod flow;
mod memory;
mod address;
use memory::{Ram, MemWidth};
mod serial;
mod host_io;
mod debugger;
mod guest;
mod dos;
mod containers;
mod inflate;
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
enum Binary { Add, Sub, Mul, Div, Rem, Xor, And, Or }
#[derive(Clone, Copy, Debug)]
enum Flag { CF, PF, AF, ZF, SF, TF, IF, DF, OF, IOPL0, IOPL1, NT }
impl Flag {
    fn bit(self) -> u8 { match self {
        Self::CF => 0, Self::PF => 2, Self::AF => 4,
        Self::ZF => 6, Self::SF => 7, Self::TF => 8, Self::IF => 9, Self::DF => 10, Self::OF => 11, Self::IOPL0 => 12, Self::IOPL1 => 13, Self::NT => 14,
    }}
}
fn parse_flag(name: &str) -> R<Flag> {
    match name {
        "CF" => Ok(Flag::CF), "PF" => Ok(Flag::PF), "AF" => Ok(Flag::AF),
        "TF" => Ok(Flag::TF), "IF" => Ok(Flag::IF), "IOPL0" => Ok(Flag::IOPL0), "IOPL1" => Ok(Flag::IOPL1), "NT" => Ok(Flag::NT),
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
    InterruptShadowIf(usize),
    ReadAddress(usize),
    ReadAddressAfter(usize,usize,usize),
    ReadMsw(usize),
    Require(usize),
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
    ReadIp(usize, bool, u8),
    ReadDisp(usize, u8),
    BranchIf(usize, usize),
    FarBranch(usize,usize),
    CheckSegment(usize,usize,u32),
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
            Self::Add { a, b, w, write_cf: true, .. } => {
                let cf = (((a + b) >> w) & 1) as u32;
                *undefined_flags &= !1;
                *eflags = (*eflags & !1) | cf;
            }
            Self::Sub { a, b, write_cf: true, .. } => {
                let cf = u32::from(a < b);
                *undefined_flags &= !1;
                *eflags = (*eflags & !1) | cf;
            }
            Self::Logic { .. } => {
                *undefined_flags &= !1;
                *eflags &= !1;
            }
            _ => {}
        }
        *self = Self::None;
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
        |Token::ResumeIf(a)|Token::Require(a)|Token::CheckSegment(_,a,_)
        |Token::ReadAddressAfter(_,_,a)|Token::Load(_,a,_)|Token::LoadSegment(_,a,_,_)=>used(a),
        Token::Binary(_,_,a,b)|Token::Shift(_,_,a,b)|Token::Less(_,a,b)
        |Token::BranchIf(a,b)|Token::FarBranch(a,b)|Token::Store(a,b,_)
        |Token::StoreSegment(a,b,_,_)|Token::WriteIf(a,b)|Token::WriteSourceIf(a,b)=>used(a)||used(b),
        Token::Select(_,a,b,c)|Token::FlagIf(_,a,b,c)=>used(a)||used(b)||used(c),
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
            Token::InterruptShadowIf(c) => format!("interrupt_shadow_if v{c}"),
            Token::ReadAddress(v) => format!("v{v} = read_address"),
            Token::ReadAddressAfter(v, r, a) => format!("v{v} = read_address_after r{r} v{a}"),
            Token::ReadMsw(v) => format!("v{v} = read_msw"),
            Token::Require(c) => format!("require v{c}"),
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
            Token::ReadIp(v, _, w) => format!("v{v} = read_ip {w}"),
            Token::ReadDisp(v, w) => format!("v{v} = read_disp {w}"),
            Token::BranchIf(t, c) => format!("branch_if v{t} (when v{c})"),
            Token::FarBranch(t, s) => format!("far_branch v{t}:v{s}"),
            Token::CheckSegment(s, a, n) => format!("check_segment {s} v{a} {n}"),
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
                        Token::CheckpointFlags | Token::RequireRealMode | Token::InterruptShadowIf(..)
                        | Token::LoadSegment(..) | Token::StoreSegment(..) | Token::ReadSegment(..)
                        | Token::WriteSegment(..) | Token::ReadMsw(..) | Token::CheckSegment(..) | Token::FarBranch(..)) => {
                        early_stop = Some(idx); note = "; FAULT (missing machine context)".into();
                    }
                    Token::RequireRealMode => {
                        if !machine.unwrap().real_mode || cpu.eflags & 0x20000 != 0 {
                            early_stop = Some(idx); note = "; FAULT (real mode required)".into();
                        } else { note = "; real mode ok".into(); }
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
                    Token::ReadMsw(o) => {
                        if let Some(m) = machine {
                            values[o] = value(m.machine_status() as u64, 16);
                            note = format!("; v{o} = 0x{:04x}", values[o].bits);
                        }
                    }
                    Token::Require(c) => {
                        if values[c].bits == 0 { early_stop = Some(idx); note = "; FAULT (require v=0)".into(); }
                        else { note = "; require ok".into(); }
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
                    Token::Store(a, v, w) => {
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
const MAX_STORES:usize=8;
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
                    if stored{return Err("loads after a staged store are not supported".into());}
                    let segment=number(register)?;let a=slot(&names,address)?;let w=width(bits)?;
                    if segment>=6||widths[a]!=32{return Err("load_segment needs segment0..5 and bv32 offset".into());}
                    let mem=MemWidth::from_bits(w).ok_or("memory width must be8/16/32")?;
                    Token::LoadSegment(define(&mut names,&mut widths,out,w)?,a,mem,segment as usize)
                }
                ["store_segment", register, address, input]=>{
                    let segment=number(register)?;let a=slot(&names,address)?;let v=slot(&names,input)?;
                    if segment>=6||widths[a]!=32||store_count>=MAX_STORES{return Err("at most eight stores with segment0..5 and bv32 offset required".into());}
                    let mem=MemWidth::from_bits(widths[v]).ok_or("memory width must be8/16/32")?;stored=true;store_count+=1;Token::StoreSegment(a,v,mem,segment as usize)
                }
                ["retire_if", predicate]=>{
                    let c=slot(&names,predicate)?;
                    if widths[c]!=1||tokens.iter().any(|t|matches!(t,Token::Write(..)|Token::WriteIf(..)|Token::WriteSource(..)|Token::WriteSourceIf(..)|Token::WriteGpr(..)|Token::WriteSegment(..)|Token::InterruptShadowIf(..)|Token::Store(..)|Token::StoreSegment(..)|Token::Flag(..)|Token::FlagIf(..)|Token::UndefFlag(..)|Token::BranchIf(..)|Token::FarBranch(..)|Token::ResumeIf(..))){return Err("retire_if requires bv1 and no earlier effects".into());}
                    Token::RetireIf(c)
                }
                ["resume_if", predicate]=>{
                    let c=slot(&names,predicate)?;if widths[c]!=1||branched{return Err("one resume/branch effect with bv1 required".into());}branched=true;Token::ResumeIf(c)
                }
                ["read_msw", out]=>Token::ReadMsw(define(&mut names,&mut widths,out,16)?),
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
                    if store_count>=MAX_STORES {return Err("at most eight staged RAM stores are supported".into());}
                    stored=true;store_count+=1;Token::Store(a,v,mem)
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
    let uses_flow=tokens.iter().any(|t| matches!(t,Token::ReadMsw(..)|Token::CheckSegment(..)|Token::CheckpointFlags|Token::RequireRealMode|Token::InterruptShadowIf(..)|Token::ReadIp(..)|Token::ReadDisp(..)|Token::BranchIf(..)|Token::FarBranch(..)|Token::ResumeIf(..)));
    let uses_displacement=tokens.iter().any(|t| matches!(t,Token::ReadDisp(..)));
    let uses_memory=tokens.iter().any(|t|matches!(t,Token::ReadAddressAfter(..)|Token::ReadAddress(..)|Token::Load(..)|Token::Store(..)|Token::LoadSegment(..)|Token::StoreSegment(..)));
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
    let result=execute_state(p,cpu,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective),lazy);
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
            Token::ReadMsw(o)=>values[o]=value(machine?.machine_status() as u64,16),
            Token::Require(c)=>if values[c].bits==0{return found;},
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
            Token::Store(a,_,w)=>{
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
fn access_error(eip:u32,f:memory::AccessFault)->ExecError {
    ExecError::MemoryAccess{eip,address:f.address,bytes:f.bytes as u8,write:f.write}
}
fn execute_state(p:&Program,cpu:&mut Cpu,dst:View,src:View,context:FlowContext,mut ram:Option<&mut Ram>,address:u32,memory_context:Option<address::MemoryContext>,mut machine:Option<&mut address::SegmentState>,effective:Option<address::EffectiveAddress>,lazy:&mut LazyFlags)->Result<Option<ProbedMemoryAccess>,ExecError> {
    if p.uses_memory && ram.is_none() {return Err(ExecError::InvalidContext);}
    dst.validate(p.operand_width).map_err(|_|ExecError::InvalidBinding)?;
    src.validate(p.source_width).map_err(|_|ExecError::InvalidBinding)?;
    context.validate(p.operand_width,p.uses_displacement)?;
    let mut pending_eip=context.next_eip;
    let mut pending_resume=false;
    let mut pending_interrupt_shadow=false;
    let mut pending_segment:Option<(usize,u16,address::SegmentCache)>=None;
    let mut pending_stores:[Option<(u32,MemWidth,u32)>;MAX_STORES]=[None;MAX_STORES];let mut store_index=0;
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
            Token::InterruptShadowIf(c)=>{machine.as_deref().ok_or(ExecError::InvalidContext)?;pending_interrupt_shadow=values[c].bits!=0;},
            Token::Read(o,b,w) => { let v = match b { Binding::Dst => dst, Binding::Src => src };
                values[o] = value(cpu.read(v).bits,w); }
            Token::LoadSegment(o,a,w,segment)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment],cache:m.segments[segment]};
                let at=c.translate(values[a].bits as u32,w,false,cpu.eip)?;
                values[o]=value(ram.as_deref().ok_or(ExecError::InvalidContext)?.load(at,w).map_err(|f|access_error(cpu.eip,f))? as u64,w.bits());
                merge_probed_access(&mut probed,ProbedMemoryAccess{address:at,bytes:w.bytes(),write:false,is_stack:segment==2&&!p.checkpoint_flags});
            }
            Token::StoreSegment(a,v,w,segment)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment],cache:m.segments[segment]};
                let at=c.translate(values[a].bits as u32,w,true,cpu.eip)?;
                ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,w).map_err(|f|access_error(cpu.eip,f))?;pending_stores[store_index]=Some((at,w,values[v].bits as u32));store_index+=1;
                merge_probed_access(&mut probed,ProbedMemoryAccess{address:at,bytes:w.bytes(),write:true,is_stack:segment==2&&!p.checkpoint_flags});
            }
            Token::RetireIf(c)=>if values[c].bits!=0{cpu.eip=context.next_eip;if let Some(m)=machine.as_deref_mut(){m.retired();}return Ok(probed);},
            Token::ResumeIf(c)=>if values[c].bits!=0{pending_eip=cpu.eip;pending_resume=true;},
            Token::ReadSegment(o,reg)=>values[o]=value(machine.as_deref().ok_or(ExecError::InvalidContext)?.selectors[reg] as u64,16),
            Token::WriteSegment(reg,a)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                if !m.real_mode{return Err(ExecError::InvalidContext);}
                let selector=values[a].bits as u16;pending_segment=Some((reg,selector,address::SegmentCache::real_mode(selector)));
            }
            Token::ReadGpr(o,reg,w)=>values[o]=value(cpu.regs[reg] as u64,w),
            Token::WriteGpr(reg,a,w)=>{pending_registers[reg_count]=(View{register:reg,low:0,width:w},values[a]);reg_count+=1;},
            Token::ReadMsw(o)=>values[o]=value(machine.as_deref().ok_or(ExecError::InvalidContext)?.machine_status() as u64,16),
            Token::Require(c)=>if values[c].bits==0{return Err(ExecError::InvalidContext);},
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
            Token::Store(a,v,w)=>{
                let m=ram.as_deref_mut().ok_or(ExecError::InvalidContext)?;
                let offset=values[a].bits as u32;
                let addr=match memory_context {Some(c)=>c.translate(offset,w,true,cpu.eip)?,None=>offset};
                m.check_write(addr,w).map_err(|f|access_error(cpu.eip,f))?;
                pending_stores[store_index]=Some((addr,w,values[v].bits as u32));store_index+=1;
                merge_probed_access(&mut probed,ProbedMemoryAccess{address:addr,bytes:w.bytes(),write:true,is_stack:!p.has_read_address});
            }
            Token::ReadIp(o,next,w) => values[o]=value((if next {context.next_eip} else {cpu.eip}) as u64,w),
            Token::ReadDisp(o,w) => values[o]=value(context.displacement as u32 as u64,w),
            Token::CheckSegment(reg,a,n)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                let c=address::MemoryContext{segment:[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][reg],cache:m.segments[reg]};
                c.translate_span(values[a].bits,n,false,cpu.eip)?;
            }
            Token::FarBranch(a,b)=>{
                let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;
                if !m.real_mode||cpu.eflags&0x20000!=0{return Err(ExecError::InvalidContext);}
                let selector=values[a].bits as u16;let cache=address::SegmentCache::real_mode(selector);let target=values[b].bits as u32;
                if target>cache.limit{return Err(ExecError::GeneralProtection{eip:cpu.eip,error_code:0});}
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
                for (addr,w,v) in pending_stores.into_iter().flatten() {
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
                    if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}
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
        "das" => Some(include_str!("../isa/i386/das.pil")),
        "rcr" => Some(include_str!("../isa/i386/rcr.pil")),
        "clc" => Some(include_str!("../isa/i386/clc.pil")),
        "call" => Some(include_str!("../isa/i386/call.pil")),
        "ret" => Some(include_str!("../isa/i386/ret.pil")),
        "retf" => Some(include_str!("../isa/i386/retf.pil")),
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
        "\nfn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {{\n\
         let w = {}u8;\n\
         if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + {}u16 > 32 {{ return Err(ExecError::InvalidBinding); }}\n\
         context.validate(w,{})?;\n\
         if {} && ram.is_none() {{ return Err(ExecError::InvalidContext); }}\n\
         let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;\n\
         let mut pending_stores: [Option<(u32,MemWidth,u32)>;8] = [None;8];let mut store_index=0;\n\
         let mut pending_eip = context.next_eip;\n\
         let mut pending_resume = false;\nlet mut pending_interrupt_shadow = false;\n\
         let mut pending_flags = *flags;\n\
         let mut pending_undefined = *undefined;\n\
         let mut pending_dst: Option<u64> = None;\n\
         let mut pending_src: Option<u64> = None;\n\
         let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();\n",
        p.operand_width,p.source_width,p.uses_displacement,p.uses_memory));
    for t in &p.tokens {
        let line = match *t {
            Token::CheckpointFlags=>"machine.as_deref().ok_or(ExecError::InvalidContext)?;".into(),
            Token::ReadFlags(o)=>format!("let v{o}:u64 = *flags as u64;"),
            Token::RequireRealMode=>"if !machine.as_deref().ok_or(ExecError::InvalidContext)?.real_mode||*flags&0x20000!=0 {return Err(ExecError::InvalidContext);}".into(),
            Token::InterruptShadowIf(c)=>format!("machine.as_deref().ok_or(ExecError::InvalidContext)?;pending_interrupt_shadow=v{c}!=0;"),
            Token::Read(o,b,w) => {
                let binding = match b { Binding::Dst => "dst", Binding::Src => "src" };
                format!("let v{o}: u64 = ((regs[{binding}.0] as u64) >> {binding}.1) & {}u64;",mask(w))
            }
            Token::LoadSegment(o,a,w,segment)=>format!("let cache=machine.as_deref().ok_or(ExecError::InvalidContext)?.segments[{segment}];let at=address::MemoryContext{{segment:address::SegmentRegister::{:?},cache}}.translate(v{a} as u32,MemWidth::{w:?},false,*eip)?;let v{o}:u64=ram.as_deref().ok_or(ExecError::InvalidContext)?.load(at,MemWidth::{w:?}).map_err(|f|ExecError::MemoryAccess{{eip:*eip,address:f.address,bytes:f.bytes as u8,write:f.write}})? as u64;",[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment]),
            Token::StoreSegment(a,v,w,segment)=>format!("let cache=machine.as_deref().ok_or(ExecError::InvalidContext)?.segments[{segment}];let at=address::MemoryContext{{segment:address::SegmentRegister::{:?},cache}}.translate(v{a} as u32,MemWidth::{w:?},true,*eip)?;ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::{w:?}).map_err(|f|ExecError::MemoryAccess{{eip:*eip,address:f.address,bytes:f.bytes as u8,write:f.write}})?;pending_stores[store_index]=Some((at,MemWidth::{w:?},v{v} as u32));store_index+=1;",[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][segment]),
            Token::RetireIf(c)=>format!("if v{c}!=0 {{*eip=context.next_eip;if let Some(m)=machine.as_deref_mut(){{m.retired();}}return Ok(());}}"),
            Token::ResumeIf(c)=>format!("if v{c}!=0 {{pending_eip=*eip;pending_resume=true;}}"),
            Token::ReadSegment(o,reg)=>format!("let v{o}: u64 = machine.as_deref().ok_or(ExecError::InvalidContext)?.selectors[{reg}] as u64;"),
            Token::WriteSegment(reg,a)=>format!("if !machine.as_deref().ok_or(ExecError::InvalidContext)?.real_mode {{return Err(ExecError::InvalidContext);}} pending_segment=Some(({reg},v{a} as u16,address::SegmentCache::real_mode(v{a} as u16)));"),
            Token::ReadGpr(o,reg,w)=>format!("let v{o}: u64 = (regs[{reg}] as u64) & {}u64;",mask(w)),
            Token::WriteGpr(reg,a,w)=>format!("pending_registers.push(({reg},{w},v{a}));"),
            Token::ReadMsw(o)=>format!("let v{o}:u64=machine.as_deref().ok_or(ExecError::InvalidContext)?.machine_status() as u64;"),
            Token::Require(c)=>format!("if v{c}==0{{return Err(ExecError::InvalidContext);}}"),
            Token::ReadAddressAfter(o,reg,a)=>format!("let mut updated=*regs;updated[{reg}]=v{a} as u32;let v{o}:u64=effective.ok_or(ExecError::InvalidContext)?.resolve(&updated,None)?.offset as u64;"),
            Token::ReadAddress(o)=>format!("let v{o}: u64 = address as u64;"),
            Token::Load(o,a,w)=>format!("let addr{o}=match memory_context {{Some(c)=>c.translate(v{a} as u32,MemWidth::{w:?},false,*eip)?,None=>v{a} as u32}}; let v{o}: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr{o},MemWidth::{w:?}).map_err(|f|ExecError::MemoryAccess{{eip:*eip,address:f.address,bytes:f.bytes as u8,write:f.write}})? as u64;"),
            Token::Store(a,v,w)=>format!("let store_addr=match memory_context {{Some(c)=>c.translate(v{a} as u32,MemWidth::{w:?},true,*eip)?,None=>v{a} as u32}}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::{w:?}).map_err(|f|ExecError::MemoryAccess{{eip:*eip,address:f.address,bytes:f.bytes as u8,write:f.write}})?; pending_stores[store_index]=Some((store_addr,MemWidth::{w:?},v{v} as u32));store_index+=1;"),
            Token::ReadIp(o,next,w) => format!("let v{o}: u64 = ({} as u64) & {}u64;",if next {"context.next_eip"} else {"*eip"},mask(w)),
            Token::ReadDisp(o,w) => format!("let v{o}: u64 = (context.displacement as u32 as u64) & {}u64;",mask(w)),
            Token::CheckSegment(reg,a,n)=>format!("let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;address::MemoryContext{{segment:address::SegmentRegister::{:?},cache:m.segments[{reg}]}}.translate_span(v{a},{n},false,*eip)?;",[address::SegmentRegister::Es,address::SegmentRegister::Cs,address::SegmentRegister::Ss,address::SegmentRegister::Ds,address::SegmentRegister::Fs,address::SegmentRegister::Gs][reg]),
            Token::FarBranch(a,b)=>format!("let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;if !m.real_mode||*flags&0x20000!=0{{return Err(ExecError::InvalidContext);}}let selector=v{a} as u16;let cache=address::SegmentCache::real_mode(selector);if v{b}>u64::from(cache.limit){{return Err(ExecError::GeneralProtection{{eip:*eip,error_code:0}});}}pending_segment=Some((1,selector,cache));pending_eip=v{b} as u32;"),
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
                "if let(Some(a),Some(b))=(pending_dst,pending_src) {{ let overlap=({}u64<<dst.1)&({}u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {{return Err(ExecError::InvalidBinding);}} }}\nfor (addr,w,v) in pending_stores.into_iter().flatten() {{ ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|ExecError::MemoryAccess{{eip:*eip,address:f.address,bytes:f.bytes as u8,write:f.write}})?; }}\nif let Some(value) = pending_dst {{ let m = {}u64 << dst.1;\nregs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }}\nif let Some(value)=pending_src {{let m={}u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}}\nfor (reg,width,value) in pending_registers {{let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}}\n*flags = pending_flags;\n*undefined = pending_undefined;\n*eip = pending_eip;\nif let Some(m)=machine.as_deref_mut(){{if !pending_resume{{m.retired();}}if pending_interrupt_shadow{{m.sti_shadow=1;}}if let Some((reg,selector,cache))=pending_segment{{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{{m.ss_shadow=1;}}}}}}\nOk(())",mask(p.operand_width),mask(p.source_width),mask(p.operand_width),mask(p.source_width)),
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
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
if result.is_err()&&CHECKPOINT{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
"#.replace("CHECKPOINT",&p.checkpoint_flags.to_string()));
    out.push_str("pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {\n");
    if p.uses_flow || p.uses_memory {out.push_str("Err(\"flow context required: use execute_with_flow\")\n");}
    else {out.push_str("let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};\nexecute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|\"invalid execution context\")\n");}
    out.push_str("}\n// Compatibility helper: discards diagnostic undefined-flag metadata.\npub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {\nlet mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)\n}\n"); out
}
fn main() -> R<()> {
    let dos_args:Vec<_>=std::env::args().collect();
    if dos_args.get(1).map(String::as_str)==Some("--dos-list"){
        if dos_args.len()<3||dos_args.len()>4{return Err("usage: pil-proof --dos-list [--all] IMAGE.img|ARCHIVE.zip".into());}
        let(all,path)=if dos_args[2]=="--all"{(true,dos_args.get(3).ok_or("missing image path")?)}else{(false,&dos_args[2])};
        println!("{}",if all{containers::listing_all(std::path::Path::new(path))?}else{containers::listing(std::path::Path::new(path))?});return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-read"){
        if dos_args.len()<4||dos_args.len()>5{return Err("usage: pil-proof --dos-read IMAGE.img|ARCHIVE.zip INNER_PATH [DEST]".into());}
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
        if dos_args.len()!=4{return Err("usage: pil-proof --dos-extract IMAGE.img|ARCHIVE.zip NEW_DIRECTORY".into());}
        println!("{}",containers::extract(std::path::Path::new(&dos_args[2]),std::path::Path::new(&dos_args[3]))?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--write-dos-hello"){
        if dos_args.len()!=4{return Err("usage: pil-proof --write-dos-hello com|exe PATH (new file only)".into());}
        let bytes=dos::hello(&dos_args[2])?;
        use std::io::Write;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&dos_args[3]).map_err(|e|e.to_string())?;
        f.write_all(&bytes).map_err(|e|e.to_string())?;println!("wrote original DOS hello {} ({} bytes)",dos_args[3],bytes.len());return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-session"){
        if dos_args.len()!=3{return Err("usage: pil-proof --dos-session PROGRAM.COM|PROGRAM.EXE|IMAGE.img::INNER_PATH".into());}
        return dos::session::serve(&dos_args[2]).map_err(Into::into);
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-probe"){
        let mut at=2;let mut budget=10000;
        if dos_args.get(at).map(String::as_str)==Some("--steps"){
            let n=dos_args.get(at+1).ok_or("--steps requires a budget")?;
            budget=usize::try_from(number(n)?).map_err(|_|"budget out of range")?;at+=2;
        }
        println!("{}",dos::report(&dos_args[at..],budget)?);return Ok(());
    }

    let args: Vec<_> = std::env::args().collect();
    if matches!(args.get(1).map(String::as_str),Some("--mandelbrot"|"--compare-mandelbrot")) {
        if args.len()>4{return Err("usage: pil-proof --mandelbrot|--compare-mandelbrot [size 2..256 power of two] [iterations 1..64]".into());}
        let parse=|i:usize,default:u32|->R<u32>{if let Some(x)=args.get(i){u32::try_from(number(x.as_str())?).map_err(|_|"parameter out of range".into())}else{Ok(default)}};
        let (size,iterations)=(parse(2,256)?,parse(3,32)?);
        println!("{}",if args[1]=="--compare-mandelbrot"{native::compare_mandelbrot(size,iterations)?}else{mandelbrot::render_files(size,iterations)?});return Ok(());
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
    if args.get(1).map(String::as_str)==Some("--debug-service") {
        let path=match args.len(){2=>None,4 if args[2]=="--load"=>Some(args[3].as_str()),_=>return Err("usage: pil-proof --debug-service [--load RAW_BINARY]".into())};
        return debugger::serve(std::io::stdin().lock(),std::io::stdout().lock(),path).map_err(|e|e.to_string());
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
