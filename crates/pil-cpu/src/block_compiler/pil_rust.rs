//! Whole-block PIL-to-Rust compiler for ahead-of-time native applications.
//!
//! Discovers reachable 16-bit real-mode basic blocks from a relocated guest
//! image and lowers each block's PIL tokens inline using local scalar CPU state
//! (`r0..r7`, `flags`, `undef`, `ip`). `rustc`/LLVM sees consecutive
//! instructions together so it can eliminate dead flags, fold branches, and
//! allocate guest registers to host registers, calling `dine` only through a
//! non-aliasing memory/service C ABI.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;

use crate::address::{Address16, AddressContext, EffectiveAddress, Gpr, SegmentRegister};
use crate::guest::{Image, Operation};
use crate::{mask, Binary, Binding, Flag, FlowContext, Program, Ram, Shift, Token, View};

pub const MAX_BLOCK_INSTRUCTIONS: usize = 32;
pub const REGION_MAX_BLOCKS: usize = 64;
pub const REGION_MAX_INSNS: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeKey {
    pub cs_rel: u16,
    pub ip: u16,
}

impl CodeKey {
    pub fn packed(self) -> u32 {
        (u32::from(self.cs_rel) << 16) | u32::from(self.ip)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terminator {
    Fallthrough,
    ConditionalBranch(u32),
    UnconditionalJump(u32),
    Call(Option<u32>),
    FarCall(Option<CodeKey>),
    FarJump(Option<CodeKey>),
    FarReturn,
    DynamicJump,
    Return,
}

#[derive(Clone)]
pub struct CompiledSite {
    pub cs_rel: u16,
    pub ip: u32,
    pub length: u32,
    pub program: Program,
    pub dst: View,
    pub src: View,
    pub flow: FlowContext,
    pub address: Option<AddressContext>,
    pub terminator: Terminator,
}

pub struct DiscoveredCfg {
    pub load_segment: u16,
    pub entry: CodeKey,
    pub segments: BTreeSet<u16>,
    pub sites: BTreeMap<CodeKey, CompiledSite>,
    pub blocks: BTreeMap<CodeKey, Vec<CodeKey>>,
    pub code_mask: Vec<u8>,
}

fn flag_allowed(flag: Flag) -> bool {
    matches!(
        flag,
        Flag::CF | Flag::PF | Flag::AF | Flag::ZF | Flag::SF | Flag::DF | Flag::OF
    )
}

/// Eligibility: straight-line/flow/memory/far-transfer PIL subset without
/// partial repeats, checkpointed flags, control writes, or load-after-store.
pub fn is_compilable(p: &Program) -> bool {
    if p.partial || p.checkpoint_flags {
        return false;
    }
    let mut saw_store = false;
    for t in &p.tokens {
        let ok = match *t {
            Token::Read(..)
            | Token::ReadGpr(..)
            | Token::ReadFlag(..)
            | Token::ReadFlags(..)
            | Token::ReadSegment(..)
            | Token::ReadIp(..)
            | Token::ReadDisp(..)
            | Token::ReadAddress(..)
            | Token::Const(..)
            | Token::Extend(..)
            | Token::SignExtend(..)
            | Token::Truncate(..)
            | Token::Binary(..)
            | Token::Shift(..)
            | Token::Less(..)
            | Token::Not(..)
            | Token::Bit(..)
            | Token::Zero(..)
            | Token::Parity(..)
            | Token::Select(..)
            | Token::Write(..)
            | Token::WriteIf(..)
            | Token::WriteSource(..)
            | Token::WriteSourceIf(..)
            | Token::WriteGpr(..)
            | Token::PreserveFlags(..)
            | Token::BranchIf(..)
            | Token::RaiseUnless(..)
            | Token::Require(..)
            | Token::RequireRealMode
            | Token::CheckWriteSegment(..)
            | Token::CheckSegment(..)
            | Token::Retire => true,
            Token::FarJump(..) | Token::FarCall(..) => p.operand_width == 16,
            Token::FarReturn(_, _, _, crate::MemWidth::Word, _, None) => p.operand_width == 16,
            Token::Flag(f, _) | Token::FlagIf(f, ..) | Token::UndefFlag(f) => flag_allowed(f),
            Token::Load(..) | Token::LoadSegment(..) => !saw_store,
            Token::Store(..) | Token::StoreIf(..) | Token::StoreSegment(..) => {
                saw_store = true;
                true
            }
            _ => false,
        };
        if !ok {
            return false;
        }
    }
    true
}

fn static_far_target(p: &Program) -> Option<(u16, u16)> {
    let mut consts: Vec<Option<u64>> = vec![None; p.widths.len()];
    for t in &p.tokens {
        match *t {
            Token::Const(o, val) => consts[o] = Some(val.bits),
            Token::Extend(o, a, _) | Token::Truncate(o, a, _) => consts[o] = consts[a],
            Token::FarJump(sel, off) | Token::FarCall(sel, off) => {
                let s = consts[sel]?;
                let o = consts[off]?;
                if s <= 0xffff && o <= 0xffff {
                    return Some((s as u16, o as u16));
                }
                return None;
            }
            _ => {}
        }
    }
    None
}

fn classify_terminator(
    label: &str,
    program: &Program,
    flow: &FlowContext,
    load_segment: u16,
    module_base: u32,
    module_end: usize,
) -> Terminator {
    if !program.uses_flow {
        return Terminator::Fallthrough;
    }
    let far_key = |sel: u16, off: u16| -> Option<CodeKey> {
        let lin = (u32::from(sel) << 4).checked_add(u32::from(off))? as usize;
        if lin < module_base as usize || lin >= module_end {
            return None;
        }
        Some(CodeKey {
            cs_rel: sel.wrapping_sub(load_segment),
            ip: off,
        })
    };
    if program.tokens.iter().any(|t| matches!(t, Token::FarCall(..))) {
        return Terminator::FarCall(
            static_far_target(program).and_then(|(s, o)| far_key(s, o)),
        );
    }
    if program.tokens.iter().any(|t| matches!(t, Token::FarJump(..))) {
        return Terminator::FarJump(
            static_far_target(program).and_then(|(s, o)| far_key(s, o)),
        );
    }
    if program.tokens.iter().any(|t| matches!(t, Token::FarReturn(..))) {
        return Terminator::FarReturn;
    }
    let target = program.uses_displacement.then(|| {
        flow.next_eip.wrapping_add_signed(flow.displacement) & mask(program.operand_width) as u32
    });
    if label.starts_with("CALL ") {
        Terminator::Call(target)
    } else if label == "RET" || label.starts_with("RET ") {
        Terminator::Return
    } else if label.starts_with("JMP ") {
        match target {
            Some(t) => Terminator::UnconditionalJump(t),
            None => Terminator::DynamicJump,
        }
    } else if let Some(t) = target {
        Terminator::ConditionalBranch(t)
    } else {
        Terminator::DynamicJump
    }
}

/// Discover reachable 16-bit real-mode sites and basic blocks inside `module_base..module_base+module_len`,
/// following intra-module static `CALL FAR` / `JMP FAR` targets across segments.
pub fn discover_real16(
    image: &Image,
    ram: &Ram,
    module_base: u32,
    module_len: usize,
    entry_ip: u32,
) -> DiscoveredCfg {
    let load_segment = (module_base >> 4) as u16;
    let entry = CodeKey {
        cs_rel: image.selectors[SegmentRegister::Cs.index()].wrapping_sub(load_segment),
        ip: entry_ip as u16,
    };
    let module_end = module_base as usize + module_len;
    let mut decode_image = image.clone();
    let mut sites: BTreeMap<CodeKey, CompiledSite> = BTreeMap::new();
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();
    if entry_ip <= 0xffff {
        queue.push_back(entry);
    }

    while let Some(key) = queue.pop_front() {
        if !visited.insert(key) {
            continue;
        }
        let cs = load_segment.wrapping_add(key.cs_rel);
        let cs_base = u32::from(cs) << 4;
        let ip = u32::from(key.ip);
        let Some(linear) = cs_base.checked_add(ip) else {
            continue;
        };
        let linear_usize = linear as usize;
        if linear_usize < module_base as usize || linear_usize >= module_end {
            continue;
        }
        decode_image.selectors[SegmentRegister::Cs.index()] = cs;
        decode_image.segments[SegmentRegister::Cs.index()] =
            crate::address::SegmentCache::real_mode(cs);
        let Ok(decoded) = decode_image.decode(ram, ip) else {
            continue;
        };
        let Some(end_linear) = linear_usize.checked_add(decoded.length as usize) else {
            continue;
        };
        if end_linear > module_end || ip + decoded.length > 0x10000 {
            continue;
        }
        let Some(bytes) = ram.contiguous_slice(linear, decoded.length as usize) else {
            continue;
        };
        let next_ip = ip + decoded.length;
        let next_key = (next_ip <= 0xffff).then_some(CodeKey {
            cs_rel: key.cs_rel,
            ip: next_ip as u16,
        });
        let falls_through = match &decoded.operation {
            Operation::Halt => false,
            Operation::Interrupt { vector, .. } => *vector != 0x20,
            Operation::Input { .. } | Operation::Output { .. } => true,
            Operation::Pil { .. } => {
                !decoded.label.starts_with("RETF")
                    && !decoded.label.starts_with("IRET")
                    && !decoded.label.starts_with("JMP ")
            }
        };
        if super::engine::answered_by_loop(bytes) {
            if falls_through {
                if let Some(nk) = next_key {
                    queue.push_back(nk);
                }
            }
            continue;
        }
        let Operation::Pil {
            program,
            dst,
            src,
            flow,
            address,
        } = &decoded.operation
        else {
            if falls_through {
                if let Some(nk) = next_key {
                    queue.push_back(nk);
                }
            }
            continue;
        };
        if !is_compilable(program) {
            if falls_through {
                if let Some(nk) = next_key {
                    queue.push_back(nk);
                }
            }
            continue;
        }
        let terminator = classify_terminator(
            &decoded.label,
            program,
            flow,
            load_segment,
            module_base,
            module_end,
        );
        let same_seg = |t: u32| -> Option<CodeKey> {
            (t <= 0xffff).then_some(CodeKey {
                cs_rel: key.cs_rel,
                ip: t as u16,
            })
        };
        match terminator {
            Terminator::Fallthrough => {
                if let Some(k) = same_seg(flow.next_eip) {
                    queue.push_back(k);
                }
            }
            Terminator::ConditionalBranch(t) => {
                if let Some(k) = same_seg(t) {
                    queue.push_back(k);
                }
                if let Some(k) = same_seg(flow.next_eip) {
                    queue.push_back(k);
                }
            }
            Terminator::UnconditionalJump(t) => {
                if let Some(k) = same_seg(t) {
                    queue.push_back(k);
                }
            }
            Terminator::Call(target) => {
                if let Some(k) = target.and_then(same_seg) {
                    queue.push_back(k);
                }
                if let Some(k) = same_seg(flow.next_eip) {
                    queue.push_back(k);
                }
            }
            Terminator::FarCall(target) => {
                if let Some(tk) = target {
                    queue.push_back(tk);
                }
                if let Some(k) = same_seg(flow.next_eip) {
                    queue.push_back(k);
                }
            }
            Terminator::FarJump(target) => {
                if let Some(tk) = target {
                    queue.push_back(tk);
                }
            }
            Terminator::FarReturn | Terminator::DynamicJump | Terminator::Return => {}
        }
        sites.insert(
            key,
            CompiledSite {
                cs_rel: key.cs_rel,
                ip,
                length: decoded.length,
                program: program.clone(),
                dst: *dst,
                src: *src,
                flow: *flow,
                address: *address,
                terminator,
            },
        );
    }

    let mut leaders = BTreeSet::new();
    let mut predecessors: BTreeMap<CodeKey, usize> = BTreeMap::new();
    if sites.contains_key(&entry) {
        leaders.insert(entry);
    }
    for (&key, site) in &sites {
        let same_seg = |t: u32| -> Option<CodeKey> {
            (t <= 0xffff).then_some(CodeKey {
                cs_rel: key.cs_rel,
                ip: t as u16,
            })
        };
        match site.terminator {
            Terminator::Fallthrough => {
                if let Some(nk) = same_seg(site.flow.next_eip).filter(|k| sites.contains_key(k)) {
                    if nk <= key {
                        leaders.insert(nk);
                    } else {
                        *predecessors.entry(nk).or_default() += 1;
                    }
                }
            }
            Terminator::ConditionalBranch(t) => {
                if let Some(tk) = same_seg(t).filter(|k| sites.contains_key(k)) {
                    leaders.insert(tk);
                }
                if let Some(nk) = same_seg(site.flow.next_eip).filter(|k| sites.contains_key(k)) {
                    leaders.insert(nk);
                }
            }
            Terminator::UnconditionalJump(t) => {
                if let Some(tk) = same_seg(t).filter(|k| sites.contains_key(k)) {
                    leaders.insert(tk);
                }
            }
            Terminator::Call(target) => {
                if let Some(tk) = target.and_then(same_seg).filter(|k| sites.contains_key(k)) {
                    leaders.insert(tk);
                }
                if let Some(nk) = same_seg(site.flow.next_eip).filter(|k| sites.contains_key(k)) {
                    leaders.insert(nk);
                }
            }
            Terminator::FarCall(target) => {
                if let Some(tk) = target.filter(|k| sites.contains_key(k)) {
                    leaders.insert(tk);
                }
                if let Some(nk) = same_seg(site.flow.next_eip).filter(|k| sites.contains_key(k)) {
                    leaders.insert(nk);
                }
            }
            Terminator::FarJump(target) => {
                if let Some(tk) = target.filter(|k| sites.contains_key(k)) {
                    leaders.insert(tk);
                }
            }
            Terminator::FarReturn | Terminator::DynamicJump | Terminator::Return => {}
        }
    }
    for &key in sites.keys() {
        if predecessors.get(&key) != Some(&1) {
            leaders.insert(key);
        }
    }

    let mut blocks = BTreeMap::new();
    let mut covered = BTreeSet::new();
    for &start in &leaders {
        if covered.contains(&start) || !sites.contains_key(&start) {
            continue;
        }
        let mut seq = Vec::new();
        let mut cur = start;
        loop {
            let site = &sites[&cur];
            seq.push(cur);
            covered.insert(cur);
            if seq.len() >= MAX_BLOCK_INSTRUCTIONS
                || !matches!(site.terminator, Terminator::Fallthrough)
                || site.flow.next_eip > 0xffff
            {
                break;
            }
            let next = CodeKey {
                cs_rel: cur.cs_rel,
                ip: site.flow.next_eip as u16,
            };
            if next <= cur
                || !sites.contains_key(&next)
                || leaders.contains(&next)
                || covered.contains(&next)
            {
                break;
            }
            cur = next;
        }
        blocks.insert(start, seq);
    }

    let segments: BTreeSet<u16> = blocks.keys().map(|k| k.cs_rel).collect();
    let mut code_mask = vec![0u8; module_len];
    for site in sites.values() {
        let cs = load_segment.wrapping_add(site.cs_rel);
        let Some(linear) = (u32::from(cs) << 4).checked_add(site.ip) else {
            continue;
        };
        let off = (linear - module_base) as usize;
        for b in off..(off + site.length as usize).min(module_len) {
            code_mask[b] = 1;
        }
    }

    DiscoveredCfg {
        load_segment,
        entry,
        segments,
        sites,
        blocks,
        code_mask,
    }
}

fn static_segment(address: Option<AddressContext>) -> u8 {
    match address {
        Some(a) => a
            .effective
            .resolve(&[0; 8], a.override_segment)
            .map_or(255, |r| r.segment.index() as u8),
        None => 255,
    }
}

fn emit_effective_offset(address: Option<AddressContext>) -> String {
    let Some(a) = address else {
        return "0u64".to_string();
    };
    let gpr_expr = |g: Gpr| format!("(r{} as u32)", g as usize);
    let gpr_bare = |g: Gpr| format!("r{} as u32", g as usize);
    match a.effective {
        EffectiveAddress::Bits16 { form, displacement } => {
            let (base, index) = match form {
                Address16::BxSi => (Some(Gpr::Ebx), Some(Gpr::Esi)),
                Address16::BxDi => (Some(Gpr::Ebx), Some(Gpr::Edi)),
                Address16::BpSi => (Some(Gpr::Ebp), Some(Gpr::Esi)),
                Address16::BpDi => (Some(Gpr::Ebp), Some(Gpr::Edi)),
                Address16::Si => (None, Some(Gpr::Esi)),
                Address16::Di => (None, Some(Gpr::Edi)),
                Address16::Bp => (Some(Gpr::Ebp), None),
                Address16::Bx => (Some(Gpr::Ebx), None),
                Address16::Direct => (None, None),
            };
            let mut sum = match (base, index) {
                (Some(b), Some(i)) => format!("{}.wrapping_add({})", gpr_expr(b), gpr_bare(i)),
                (Some(b), None) => gpr_expr(b),
                (None, Some(i)) => gpr_expr(i),
                (None, None) => "0u32".to_string(),
            };
            if displacement != 0 {
                sum = format!("{sum}.wrapping_add({displacement}u32)");
            }
            format!("({sum} & 0xffffu32) as u64")
        }
        EffectiveAddress::Bits32 {
            base,
            index,
            scale,
            displacement,
        } => {
            let mut sum = base.map_or_else(|| "0u32".to_string(), gpr_expr);
            if let Some(i) = index {
                let term = if scale == 1 {
                    if base.is_none() { gpr_expr(i) } else { gpr_bare(i) }
                } else {
                    format!("{}.wrapping_mul({scale}u32)", gpr_expr(i))
                };
                sum = if base.is_none() && scale == 1 {
                    term
                } else if base.is_none() {
                    term
                } else {
                    format!("{sum}.wrapping_add({term})")
                };
            }
            if displacement != 0 {
                sum = format!("{sum}.wrapping_add({displacement}u32)");
            }
            format!("{sum} as u64")
        }
    }
}

fn emit_site_body(
    out: &mut String,
    site: &CompiledSite,
    idx_in_block: usize,
    is_last: bool,
    multi_region: bool,
    region_cs_rel: u16,
    has_start_steps: bool,
) {
    let p = &site.program;
    let inst_ip = site.ip;
    let mem_seg = static_segment(site.address);
    let exit_expr = if has_start_steps {
        "*steps = cur_steps; (*ctx).exit_kind = 1; exit_rc = 1; break 'dispatch;"
    } else {
        "(*ctx).exit_kind = 1; break 'dispatch;"
    };
    let fault_exit = |out: &mut String, idx: usize| {
        if has_start_steps {
            writeln!(
                out,
                "ip = {inst_ip}u32; cur_steps += {idx}u32; *steps = cur_steps; (*ctx).exit_kind = 3; (*ctx).fault_eip = {inst_ip}u32; exit_rc = 3; break 'dispatch;"
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "ip = {inst_ip}u32; steps += {idx}u32; (*ctx).exit_kind = 3; (*ctx).fault_eip = {inst_ip}u32; break 'dispatch;"
            )
            .unwrap();
        }
    };
    let has_stores = p.tokens.iter().any(|t| {
        matches!(
            t,
            Token::Store(..) | Token::StoreIf(..) | Token::StoreSegment(..)
        )
    });
    let has_dst_write = p
        .tokens
        .iter()
        .any(|t| matches!(t, Token::Write(..) | Token::WriteIf(..)));
    let has_src_write = p
        .tokens
        .iter()
        .any(|t| matches!(t, Token::WriteSource(..) | Token::WriteSourceIf(..)));
    let has_flag_write = p
        .tokens
        .iter()
        .any(|t| matches!(t, Token::Flag(..) | Token::FlagIf(..) | Token::UndefFlag(..)));
    let has_branch = p.tokens.iter().any(|t| matches!(t, Token::BranchIf(..)));
    let has_far_transfer = p
        .tokens
        .iter()
        .any(|t| matches!(t, Token::FarJump(..) | Token::FarCall(..) | Token::FarReturn(..)));
    let has_far_sp = p.tokens.iter().any(|t| matches!(t, Token::FarReturn(..)));
    let has_next_ip = has_branch || has_far_transfer;
    let steps_var = if has_start_steps { "cur_steps" } else { "steps" };

    writeln!(out, "{{").unwrap();
    if has_stores {
        writeln!(out, "let mut modified: bool = false;").unwrap();
    }
    if has_dst_write {
        writeln!(out, "let mut pending_dst: Option<u64> = None;").unwrap();
    }
    if has_src_write {
        writeln!(out, "let mut pending_src: Option<u64> = None;").unwrap();
    }
    if has_flag_write {
        writeln!(
            out,
            "let mut pending_flags: u32 = flags;\nlet mut pending_undef: u32 = undef;"
        )
        .unwrap();
    }
    if has_next_ip {
        writeln!(out, "let mut next_ip: u32 = {}u32;", site.flow.next_eip).unwrap();
    }
    if has_far_transfer {
        writeln!(out, "let mut pending_cs: Option<u16> = None;").unwrap();
    }
    if has_far_sp {
        writeln!(out, "let mut pending_far_sp: Option<u32> = None;").unwrap();
    }

    let mut staged_stores: Vec<(u8, String, String, u8, Option<String>)> = Vec::new();
    let mut gpr_commits = String::new();
    let mut carry_skip = 0usize;

    for (index, t) in p.tokens.iter().enumerate() {
        if carry_skip > 0 {
            carry_skip -= 1;
            continue;
        }
        if let Some(code) = crate::rust_codegen::carry_window(p, index) {
            out.push_str(&code);
            carry_skip = 3;
            continue;
        }
        match *t {
            Token::PreserveFlags(_) => {}
            Token::RequireRealMode => {}
            Token::Require(c) => {
                writeln!(
                    out,
                    "if v{c} == 0 {{ ip = {inst_ip}u32; {steps_var} += {idx_in_block}u32; {exit_expr} }}"
                )
                .unwrap();
            }
            Token::RaiseUnless(c, v) => {
                if v == 13 {
                    writeln!(
                        out,
                        "if v{c} == 0 {{ ip = {inst_ip}u32; {steps_var} += {idx_in_block}u32; {exit_expr} }}"
                    )
                    .unwrap();
                } else if has_start_steps {
                    writeln!(
                        out,
                        "if v{c} == 0 {{ ip = {inst_ip}u32; cur_steps += {idx_in_block}u32; *steps = cur_steps; (*ctx).exit_kind = 4; (*ctx).fault_eip = {inst_ip}u32; (*ctx).fault_vector = {v}u8; exit_rc = 4; break 'dispatch; }}"
                    )
                    .unwrap();
                } else {
                    writeln!(
                        out,
                        "if v{c} == 0 {{ ip = {inst_ip}u32; steps += {idx_in_block}u32; (*ctx).exit_kind = 4; (*ctx).fault_eip = {inst_ip}u32; (*ctx).fault_vector = {v}u8; break 'dispatch; }}"
                    )
                    .unwrap();
                }
            }
            Token::Read(o, b, w) => {
                let view = match b {
                    Binding::Dst => site.dst,
                    Binding::Src => site.src,
                };
                let m = mask(w);
                if view.low == 0 {
                    writeln!(out, "let v{o}: u64 = (r{} as u64) & {m}u64;", view.register).unwrap();
                } else {
                    writeln!(
                        out,
                        "let v{o}: u64 = ((r{} as u64) >> {}) & {m}u64;",
                        view.register, view.low
                    )
                    .unwrap();
                }
            }
            Token::ReadGpr(o, reg, w) => {
                writeln!(out, "let v{o}: u64 = (r{reg} as u64) & {}u64;", mask(w)).unwrap();
            }
            Token::WriteGpr(reg, a, w) => {
                let m = mask(w);
                writeln!(out, "let pending_gpr_{index}: u64 = v{a};").unwrap();
                writeln!(
                    &mut gpr_commits,
                    "r{reg} = ((r{reg} as u64 & !{m}u64) | (pending_gpr_{index} & {m}u64)) as u32;"
                )
                .unwrap();
            }
            Token::ReadSegment(o, seg) => {
                writeln!(out, "let v{o}: u64 = selectors[{seg}] as u64;").unwrap();
            }
            Token::ReadFlag(o, f) => {
                writeln!(out, "let v{o}: u64 = ((flags >> {}) & 1) as u64;", f.bit()).unwrap();
            }
            Token::ReadFlags(o) => {
                writeln!(out, "let v{o}: u64 = flags as u64;").unwrap();
            }
            Token::ReadIp(o, next, w) => {
                let v = if next { site.flow.next_eip } else { inst_ip };
                writeln!(out, "let v{o}: u64 = ({v}u64) & {}u64;", mask(w)).unwrap();
            }
            Token::ReadDisp(o, w) => {
                let v = (site.flow.displacement as u32 as u64) & mask(w);
                writeln!(out, "let v{o}: u64 = {v}u64;").unwrap();
            }
            Token::ReadAddress(o) => {
                writeln!(
                    out,
                    "let v{o}: u64 = {};",
                    emit_effective_offset(site.address)
                )
                .unwrap();
            }
            Token::Const(o, val) => {
                writeln!(out, "let v{o}: u64 = {}u64;", val.bits).unwrap();
            }
            Token::Extend(o, a, w) | Token::Truncate(o, a, w) => {
                writeln!(out, "let v{o}: u64 = v{a} & {}u64;", mask(w)).unwrap();
            }
            Token::SignExtend(o, a, w) => {
                let in_w = p.widths[a];
                writeln!(
                    out,
                    "let v{o}: u64 = (if v{a} & {}u64 != 0 {{ v{a} | !{}u64 }} else {{ v{a} }}) & {}u64;",
                    1u64 << (in_w - 1),
                    mask(in_w),
                    mask(w)
                )
                .unwrap();
            }
            Token::Binary(o, op, a, b) => {
                let expr = match op {
                    Binary::Add => format!("v{a}.wrapping_add(v{b})"),
                    Binary::Sub => format!("v{a}.wrapping_sub(v{b})"),
                    Binary::Div => format!("if v{b} == 0 {{ 0 }} else {{ v{a} / v{b} }}"),
                    Binary::Rem => format!("if v{b} == 0 {{ 0 }} else {{ v{a} % v{b} }}"),
                    Binary::Mul => format!("v{a}.wrapping_mul(v{b})"),
                    Binary::Xor => format!("v{a} ^ v{b}"),
                    Binary::And => format!("v{a} & v{b}"),
                    Binary::Or => format!("v{a} | v{b}"),
                };
                writeln!(out, "let v{o}: u64 = ({expr}) & {}u64;", mask(p.widths[o])).unwrap();
            }
            Token::Shift(o, kind, a, b) => {
                let w = p.widths[a];
                let expr = match kind {
                    Shift::Left => format!("if v{b} >= {w}u64 {{ 0 }} else {{ v{a} << v{b} }}"),
                    Shift::Right => format!("if v{b} >= {w}u64 {{ 0 }} else {{ v{a} >> v{b} }}"),
                    Shift::Arithmetic => format!(
                        "((((v{a} << {}) as i64) >> {}) >> v{b}.min({}u64)) as u64",
                        64 - w,
                        64 - w,
                        w - 1
                    ),
                };
                writeln!(out, "let v{o}: u64 = ({expr}) & {}u64;", mask(w)).unwrap();
            }
            Token::Less(o, a, b) => {
                writeln!(out, "let v{o}: u64 = u64::from(v{a} < v{b});").unwrap();
            }
            Token::Not(o, a) => {
                writeln!(out, "let v{o}: u64 = (!v{a}) & {}u64;", mask(p.widths[o])).unwrap();
            }
            Token::Bit(o, a, bit) => {
                writeln!(out, "let v{o}: u64 = (v{a} >> {bit}) & 1;").unwrap();
            }
            Token::Zero(o, a) => {
                writeln!(out, "let v{o}: u64 = u64::from(v{a} == 0);").unwrap();
            }
            Token::Parity(o, a) => {
                writeln!(
                    out,
                    "let v{o}: u64 = u64::from((v{a} as u8).count_ones() & 1 == 0);"
                )
                .unwrap();
            }
            Token::Select(o, c, a, b) => {
                writeln!(
                    out,
                    "let v{o}: u64 = if v{c} != 0 {{ v{a} }} else {{ v{b} }};"
                )
                .unwrap();
            }
            Token::Write(a) => {
                writeln!(out, "pending_dst = Some(v{a});").unwrap();
            }
            Token::WriteIf(a, c) => {
                writeln!(out, "if v{c} != 0 {{ pending_dst = Some(v{a}); }}").unwrap();
            }
            Token::WriteSource(a) => {
                writeln!(out, "pending_src = Some(v{a});").unwrap();
            }
            Token::WriteSourceIf(a, c) => {
                writeln!(out, "if v{c} != 0 {{ pending_src = Some(v{a}); }}").unwrap();
            }
            Token::Flag(f, a) => {
                let m = 1u32 << f.bit();
                writeln!(
                    out,
                    "pending_undef &= !{m}u32; pending_flags = (pending_flags & !{m}u32) | ((v{a} as u32) << {});",
                    f.bit()
                )
                .unwrap();
            }
            Token::FlagIf(f, a, d, c) => {
                let m = 1u32 << f.bit();
                writeln!(
                    out,
                    "if v{c} != 0 {{ pending_flags &= !{m}u32; if v{d} != 0 {{ pending_flags |= (v{a} as u32) << {}; pending_undef &= !{m}u32; }} else {{ pending_undef |= {m}u32; }} }}",
                    f.bit()
                )
                .unwrap();
            }
            Token::UndefFlag(f) => {
                let m = 1u32 << f.bit();
                writeln!(
                    out,
                    "pending_flags &= !{m}u32; pending_undef |= {m}u32;"
                )
                .unwrap();
            }
            Token::Load(o, a, w) => {
                let bytes = w.bytes();
                writeln!(
                    out,
                    "let mut tmp_{o}: u32 = 0;\nif fast_read(host, selectors[1], &seg_bases, seg_fast_mask, ram_ptr, ram_fast_limit, {mem_seg}u8, v{a} as u32, {bytes}u8, {inst_ip}u32, &mut tmp_{o}) < 0 {{"
                )
                .unwrap();
                fault_exit(out, idx_in_block);
                writeln!(out, "}}\nlet v{o}: u64 = tmp_{o} as u64;").unwrap();
            }
            Token::LoadSegment(o, a, w, seg) => {
                let bytes = w.bytes();
                writeln!(
                    out,
                    "let mut tmp_{o}: u32 = 0;\nif fast_read(host, selectors[1], &seg_bases, seg_fast_mask, ram_ptr, ram_fast_limit, {seg}u8, v{a} as u32, {bytes}u8, {inst_ip}u32, &mut tmp_{o}) < 0 {{"
                )
                .unwrap();
                fault_exit(out, idx_in_block);
                writeln!(out, "}}\nlet v{o}: u64 = tmp_{o} as u64;").unwrap();
            }
            Token::CheckSegment(seg, a, bytes) => {
                writeln!(
                    out,
                    "if fast_check_span(host, selectors[1], seg_fast_mask, {seg}u8, v{a}, {bytes}u32, {inst_ip}u32) < 0 {{"
                )
                .unwrap();
                fault_exit(out, idx_in_block);
                writeln!(out, "}}").unwrap();
            }
            Token::CheckWriteSegment(seg, a, w) => {
                let bytes = w.bytes();
                writeln!(
                    out,
                    "if fast_check_write(host, selectors[1], &seg_bases, seg_fast_mask, ram_fast_limit, {seg}u8, v{a} as u32, {bytes}u8, {inst_ip}u32) < 0 {{"
                )
                .unwrap();
                fault_exit(out, idx_in_block);
                writeln!(out, "}}").unwrap();
            }
            Token::Store(a, v, w) => {
                let bytes = w.bytes();
                writeln!(
                    out,
                    "let store_addr_{index}: u32 = v{a} as u32;\nlet store_val_{index}: u32 = v{v} as u32;\nif fast_check_write(host, selectors[1], &seg_bases, seg_fast_mask, ram_fast_limit, {mem_seg}u8, store_addr_{index}, {bytes}u8, {inst_ip}u32) < 0 {{"
                )
                .unwrap();
                fault_exit(out, idx_in_block);
                writeln!(out, "}}").unwrap();
                staged_stores.push((
                    mem_seg,
                    format!("store_addr_{index}"),
                    format!("store_val_{index}"),
                    bytes,
                    None,
                ));
            }
            Token::StoreIf(a, v, c, w) => {
                let bytes = w.bytes();
                writeln!(
                    out,
                    "let store_cond_{index}: bool = v{c} != 0;\nlet store_addr_{index}: u32 = v{a} as u32;\nlet store_val_{index}: u32 = v{v} as u32;\nif store_cond_{index} && fast_check_write(host, selectors[1], &seg_bases, seg_fast_mask, ram_fast_limit, {mem_seg}u8, store_addr_{index}, {bytes}u8, {inst_ip}u32) < 0 {{"
                )
                .unwrap();
                fault_exit(out, idx_in_block);
                writeln!(out, "}}").unwrap();
                staged_stores.push((
                    mem_seg,
                    format!("store_addr_{index}"),
                    format!("store_val_{index}"),
                    bytes,
                    Some(format!("store_cond_{index}")),
                ));
            }
            Token::StoreSegment(a, v, w, seg) => {
                let bytes = w.bytes();
                writeln!(
                    out,
                    "let store_addr_{index}: u32 = v{a} as u32;\nlet store_val_{index}: u32 = v{v} as u32;\nif fast_check_write(host, selectors[1], &seg_bases, seg_fast_mask, ram_fast_limit, {seg}u8, store_addr_{index}, {bytes}u8, {inst_ip}u32) < 0 {{"
                )
                .unwrap();
                fault_exit(out, idx_in_block);
                writeln!(out, "}}").unwrap();
                staged_stores.push((
                    seg as u8,
                    format!("store_addr_{index}"),
                    format!("store_val_{index}"),
                    bytes,
                    None,
                ));
            }
            Token::BranchIf(a, c) => {
                writeln!(
                    out,
                    "if v{c} != 0 {{ if (v{a} as u32) > code_limit {{ ip = {inst_ip}u32; {steps_var} += {idx_in_block}u32; {exit_expr} }} next_ip = v{a} as u32; }}"
                )
                .unwrap();
            }
            Token::FarJump(sel, off) | Token::FarCall(sel, off) => {
                writeln!(
                    out,
                    "if (v{off} as u32) > 0xffffu32 {{ ip = {inst_ip}u32; {steps_var} += {idx_in_block}u32; {exit_expr} }}\npending_cs = Some(v{sel} as u16);\nnext_ip = v{off} as u32;"
                )
                .unwrap();
            }
            Token::FarReturn(sel, off, outer, ..) => {
                writeln!(
                    out,
                    "if (v{off} as u32) > 0xffffu32 {{ ip = {inst_ip}u32; {steps_var} += {idx_in_block}u32; {exit_expr} }}\npending_cs = Some(v{sel} as u16);\nnext_ip = v{off} as u32;\npending_far_sp = Some(v{outer} as u32);"
                )
                .unwrap();
            }
            Token::Retire => {
                if has_dst_write
                    && has_src_write
                    && site.dst.register == site.src.register
                    && ((mask(p.operand_width) << site.dst.low)
                        & (mask(p.source_width) << site.src.low))
                        != 0
                {
                    let overlap = (mask(p.operand_width) << site.dst.low)
                        & (mask(p.source_width) << site.src.low);
                    writeln!(
                        out,
                        "if let (Some(a), Some(b)) = (pending_dst, pending_src) {{ if (((a << {}) ^ (b << {})) & {overlap}u64) != 0 {{ ip = {inst_ip}u32; {steps_var} += {idx_in_block}u32; {exit_expr} }} }}",
                        site.dst.low, site.src.low
                    )
                    .unwrap();
                }
                for (seg, addr, val, bytes, cond) in &staged_stores {
                    writeln!(
                        out,
                        "{}let rc = fast_write(host, selectors[1], &seg_bases, seg_fast_mask, ram_ptr, watch_ptr, ram_fast_limit, code_start, code_end, {seg}u8, {addr}, {bytes}u8, {val}, {inst_ip}u32);\nif rc < 0 {{",
                        cond.as_deref().map_or_else(String::new, |c| format!("if {c} {{\n"))
                    )
                    .unwrap();
                    fault_exit(out, idx_in_block);
                    writeln!(out, "}}\nmodified |= rc == 1;").unwrap();
                    if cond.is_some() {
                        writeln!(out, "}}").unwrap();
                    }
                }
                if has_dst_write {
                    let m = mask(p.operand_width) << site.dst.low;
                    writeln!(
                        out,
                        "if let Some(value) = pending_dst {{ r{} = (((r{} as u64) & !{m}u64) | ((value << {}) & {m}u64)) as u32; }}",
                        site.dst.register, site.dst.register, site.dst.low
                    )
                    .unwrap();
                }
                if has_src_write {
                    let m = mask(p.source_width) << site.src.low;
                    writeln!(
                        out,
                        "if let Some(value) = pending_src {{ r{} = (((r{} as u64) & !{m}u64) | ((value << {}) & {m}u64)) as u32; }}",
                        site.src.register, site.src.register, site.src.low
                    )
                    .unwrap();
                }
                if !gpr_commits.is_empty() {
                    out.push_str(&gpr_commits);
                }
                if has_far_sp {
                    writeln!(
                        out,
                        "if let Some(sp) = pending_far_sp {{ r4 = (r4 & !0xffffu32) | (sp & 0xffffu32); }}"
                    )
                    .unwrap();
                }
                if has_flag_write {
                    writeln!(out, "flags = pending_flags;\nundef = pending_undef;").unwrap();
                }
                if has_far_transfer {
                    writeln!(
                        out,
                        "if let Some(new_cs) = pending_cs {{\n\
                         selectors[1] = new_cs;\n\
                         seg_bases[1] = (new_cs as u32) << 4;\n\
                         seg_fast_mask |= 2u8;\n\
                         code_limit = 0xffffu32;\n\
                         }}"
                    )
                    .unwrap();
                }
                let next_expr = if has_next_ip {
                    "next_ip".to_string()
                } else {
                    format!("{}u32", site.flow.next_eip)
                };
                if has_stores {
                    if has_start_steps {
                        writeln!(
                            out,
                            "if modified {{ ip = {next_expr}; cur_steps += {}u32; *steps = cur_steps; (*ctx).exit_kind = 2; exit_rc = 2; break 'dispatch; }}",
                            idx_in_block + 1
                        )
                        .unwrap();
                    } else {
                        writeln!(
                            out,
                            "if modified {{ ip = {next_expr}; steps += {}u32; (*ctx).exit_kind = 2; break 'dispatch; }}",
                            idx_in_block + 1
                        )
                        .unwrap();
                    }
                }
                if is_last {
                    writeln!(out, "ip = {next_expr};").unwrap();
                }
                if multi_region && has_far_transfer {
                    writeln!(
                        out,
                        "if selectors[1].wrapping_sub(load_segment) != {region_cs_rel}u16 {{\n\
                         ip = {next_expr};\n\
                         cur_steps += {}u32;\n\
                         *steps = cur_steps;\n\
                         exit_rc = 5;\n\
                         break 'dispatch;\n\
                         }}",
                        idx_in_block + 1
                    )
                    .unwrap();
                }
            }
            _ => unreachable!("is_compilable filtered all unsupported tokens"),
        }
    }
    writeln!(out, "}}").unwrap();
}

struct Region {
    id: usize,
    cs_rel: u16,
    min_ip: u16,
    max_ip: u16,
    blocks: Vec<CodeKey>,
}

fn partition_regions(cfg: &DiscoveredCfg) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut current: Option<Region> = None;
    let mut current_insns = 0usize;
    for (&key, seq) in &cfg.blocks {
        let should_split = match &current {
            Some(r) => {
                r.cs_rel != key.cs_rel
                    || r.blocks.len() >= REGION_MAX_BLOCKS
                    || (current_insns + seq.len() > REGION_MAX_INSNS && !r.blocks.is_empty())
            }
            None => false,
        };
        if should_split {
            if let Some(r) = current.take() {
                regions.push(r);
            }
            current_insns = 0;
        }
        let r = current.get_or_insert_with(|| Region {
            id: regions.len(),
            cs_rel: key.cs_rel,
            min_ip: key.ip,
            max_ip: key.ip,
            blocks: Vec::new(),
        });
        r.max_ip = r.max_ip.max(key.ip);
        r.blocks.push(key);
        current_insns += seq.len();
    }
    if let Some(r) = current {
        regions.push(r);
    }
    regions
}

/// Render a complete Rust source module implementing `run_compiled` and linking `dine`.
pub fn emit_application_rust(cfg: &DiscoveredCfg, img_include: &str, name: &str) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "// Generated PIL-lowered Rust blocks; DOS/BIOS/memory/fallback services stay in Dine.\n\
         use std::ffi::{{c_char, c_void, CString}};\n\
         #[repr(C)]\n\
         pub struct DinePilCtx {{\n\
             pub regs: [u32; 8],\n\
             pub eflags: u32,\n\
             pub undefined: u32,\n\
             pub eip: u32,\n\
             pub selectors: [u16; 6],\n\
             pub load_segment: u16,\n\
             pub seg_bases: [u32; 6],\n\
             pub seg_fast_mask: u8,\n\
             pub code_limit: u32,\n\
             pub ram_ptr: *mut u8,\n\
             pub watch_ptr: *mut u64,\n\
             pub ram_fast_limit: u32,\n\
             pub code_start: u32,\n\
             pub code_end: u32,\n\
             pub steps_done: u32,\n\
             pub exit_kind: u32,\n\
             pub fault_eip: u32,\n\
             pub fault_vector: u8,\n\
             pub host: *mut c_void,\n\
         }}\n\
         #[cfg_attr(windows, link(name = \"dine\", kind = \"raw-dylib\"))]\n\
         #[cfg_attr(not(windows), link(name = \"dine\"))]\n\
         #[allow(dead_code)]\n\
         extern \"C\" {{\n\
             fn dine_run_pil_v1(data: *const u8, len: usize, name: *const c_char, tail: *const c_char, entry: unsafe extern \"C\" fn(*mut DinePilCtx, u32)) -> i32;\n\
             fn dine_session_pil_v1(data: *const u8, len: usize, name: *const c_char, tail: *const c_char, entry: unsafe extern \"C\" fn(*mut DinePilCtx, u32)) -> i32;\n\
             fn dine_pil_read(host: *mut c_void, cs_sel: u16, seg: u8, offset: u32, bytes: u8, eip: u32, out: *mut u32) -> i32;\n\
             fn dine_pil_write(host: *mut c_void, cs_sel: u16, seg: u8, offset: u32, bytes: u8, value: u32, eip: u32) -> i32;\n\
             fn dine_pil_check_write(host: *mut c_void, cs_sel: u16, seg: u8, offset: u32, bytes: u8, eip: u32) -> i32;\n\
             fn dine_pil_check_span(host: *mut c_void, cs_sel: u16, seg: u8, offset: u64, bytes: u32, eip: u32) -> i32;\n\
         }}\n\
         #[cold]\n\
         #[inline(never)]\n\
         #[allow(dead_code)]\n\
         unsafe fn slow_read(host: *mut c_void, cs_sel: u16, seg: u8, offset: u32, bytes: u8, eip: u32, out: *mut u32) -> i32 {{\n\
             dine_pil_read(host, cs_sel, seg, offset, bytes, eip, out)\n\
         }}\n\
         #[cold]\n\
         #[inline(never)]\n\
         #[allow(dead_code)]\n\
         unsafe fn slow_write(host: *mut c_void, cs_sel: u16, seg: u8, offset: u32, bytes: u8, value: u32, eip: u32) -> i32 {{\n\
             dine_pil_write(host, cs_sel, seg, offset, bytes, value, eip)\n\
         }}\n\
         #[cold]\n\
         #[inline(never)]\n\
         #[allow(dead_code)]\n\
         unsafe fn slow_check_write(host: *mut c_void, cs_sel: u16, seg: u8, offset: u32, bytes: u8, eip: u32) -> i32 {{\n\
             dine_pil_check_write(host, cs_sel, seg, offset, bytes, eip)\n\
         }}\n\
         #[cold]\n\
         #[inline(never)]\n\
         #[allow(dead_code)]\n\
         unsafe fn slow_check_span(host: *mut c_void, cs_sel: u16, seg: u8, offset: u64, bytes: u32, eip: u32) -> i32 {{\n\
             dine_pil_check_span(host, cs_sel, seg, offset, bytes, eip)\n\
         }}\n\
         #[inline(always)]\n\
         #[allow(dead_code)]\n\
         unsafe fn fast_read(host: *mut c_void, cs_sel: u16, seg_bases: &[u32; 6], seg_fast_mask: u8, ram_ptr: *mut u8, ram_fast_limit: u32, seg: u8, offset: u32, bytes: u8, eip: u32, out: *mut u32) -> i32 {{\n\
             if seg < 6 && (seg_fast_mask & (1u8 << seg)) != 0 && offset <= 0x10000u32 - bytes as u32 {{\n\
                 let lin = seg_bases[seg as usize].wrapping_add(offset);\n\
                 if lin.wrapping_add(bytes as u32) <= ram_fast_limit {{\n\
                     let p = ram_ptr.add(lin as usize);\n\
                     *out = match bytes {{\n\
                         1 => *p as u32,\n\
                         2 => (p as *const u16).read_unaligned().to_le() as u32,\n\
                         4 => (p as *const u32).read_unaligned().to_le(),\n\
                         _ => return slow_read(host, cs_sel, seg, offset, bytes, eip, out),\n\
                     }};\n\
                     return 0;\n\
                 }}\n\
             }}\n\
             slow_read(host, cs_sel, seg, offset, bytes, eip, out)\n\
         }}\n\
         #[inline(always)]\n\
         #[allow(dead_code)]\n\
         unsafe fn fast_check_write(host: *mut c_void, cs_sel: u16, seg_bases: &[u32; 6], seg_fast_mask: u8, ram_fast_limit: u32, seg: u8, offset: u32, bytes: u8, eip: u32) -> i32 {{\n\
             if seg < 6 && (seg_fast_mask & (1u8 << seg)) != 0 && offset <= 0x10000u32 - bytes as u32 {{\n\
                 let lin = seg_bases[seg as usize].wrapping_add(offset);\n\
                 if lin.wrapping_add(bytes as u32) <= ram_fast_limit {{\n\
                     return 0;\n\
                 }}\n\
             }}\n\
             slow_check_write(host, cs_sel, seg, offset, bytes, eip)\n\
         }}\n\
         #[inline(always)]\n\
         #[allow(dead_code)]\n\
         unsafe fn fast_write(host: *mut c_void, cs_sel: u16, seg_bases: &[u32; 6], seg_fast_mask: u8, ram_ptr: *mut u8, watch_ptr: *mut u64, ram_fast_limit: u32, code_start: u32, code_end: u32, seg: u8, offset: u32, bytes: u8, value: u32, eip: u32) -> i32 {{\n\
             if seg < 6 && (seg_fast_mask & (1u8 << seg)) != 0 && offset <= 0x10000u32 - bytes as u32 {{\n\
                 let lin = seg_bases[seg as usize].wrapping_add(offset);\n\
                 let end = lin.wrapping_add(bytes as u32);\n\
                 if end <= ram_fast_limit && (end <= code_start || lin >= code_end) {{\n\
                     let p = ram_ptr.add(lin as usize);\n\
                     match bytes {{\n\
                         1 => *p = value as u8,\n\
                         2 => (p as *mut u16).write_unaligned((value as u16).to_le()),\n\
                         4 => (p as *mut u32).write_unaligned(value.to_le()),\n\
                         _ => return slow_write(host, cs_sel, seg, offset, bytes, value, eip),\n\
                     }}\n\
                     let l0 = (lin >> 6) as usize;\n\
                     let l1 = ((end - 1) >> 6) as usize;\n\
                     *watch_ptr.add(l0) = (*watch_ptr.add(l0)).wrapping_add(1);\n\
                     if l1 != l0 {{\n\
                         *watch_ptr.add(l1) = (*watch_ptr.add(l1)).wrapping_add(1);\n\
                     }}\n\
                     return 0;\n\
                 }}\n\
             }}\n\
             slow_write(host, cs_sel, seg, offset, bytes, value, eip)\n\
         }}\n\
         #[inline(always)]\n\
         #[allow(dead_code)]\n\
         unsafe fn fast_check_span(host: *mut c_void, cs_sel: u16, seg_fast_mask: u8, seg: u8, offset: u64, bytes: u32, eip: u32) -> i32 {{\n\
             if seg < 6 && (seg_fast_mask & (1u8 << seg)) != 0 && bytes > 0 && offset <= 0x10000u64 - bytes as u64 {{\n\
                 return 0;\n\
             }}\n\
             slow_check_span(host, cs_sel, seg, offset, bytes, eip)\n\
         }}"
    )
    .unwrap();

    let regions = partition_regions(cfg);
    let has_far = cfg.sites.values().any(|s| {
        matches!(
            s.terminator,
            Terminator::FarCall(..) | Terminator::FarJump(..) | Terminator::FarReturn
        )
    });
    let single_fast = regions.len() == 1 && !has_far;

    if single_fast {
        let region = &regions[0];
        writeln!(
            out,
            "#[allow(unused_variables, unused_mut, unused_assignments)]\n\
             pub unsafe extern \"C\" fn run_compiled(ctx: *mut DinePilCtx, budget: u32) {{\n\
                 let mut selectors = (*ctx).selectors;\n\
                 let load_segment = (*ctx).load_segment;\n\
                 if selectors[1].wrapping_sub(load_segment) != {}u16 {{\n\
                     (*ctx).exit_kind = 1;\n\
                     return;\n\
                 }}\n\
                 let mut r0 = (*ctx).regs[0];\n\
                 let mut r1 = (*ctx).regs[1];\n\
                 let mut r2 = (*ctx).regs[2];\n\
                 let mut r3 = (*ctx).regs[3];\n\
                 let mut r4 = (*ctx).regs[4];\n\
                 let mut r5 = (*ctx).regs[5];\n\
                 let mut r6 = (*ctx).regs[6];\n\
                 let mut r7 = (*ctx).regs[7];\n\
                 let mut flags = (*ctx).eflags;\n\
                 let mut undef = (*ctx).undefined;\n\
                 let mut ip = (*ctx).eip;\n\
                 let mut seg_bases = (*ctx).seg_bases;\n\
                 let mut seg_fast_mask = (*ctx).seg_fast_mask;\n\
                 let mut code_limit = (*ctx).code_limit;\n\
                 let ram_ptr = (*ctx).ram_ptr;\n\
                 let watch_ptr = (*ctx).watch_ptr;\n\
                 let ram_fast_limit = (*ctx).ram_fast_limit;\n\
                 let code_start = (*ctx).code_start;\n\
                 let code_end = (*ctx).code_end;\n\
                 let host = (*ctx).host;\n\
                 let mut steps: u32 = 0;\n\
                 'dispatch: loop {{\n\
                     match ip {{",
            region.cs_rel
        )
        .unwrap();
        for &key in &region.blocks {
            let seq = &cfg.blocks[&key];
            let k = seq.len();
            writeln!(
                out,
                "{}u32 => {{\n\
                 if steps + {k}u32 > budget {{ (*ctx).exit_kind = 0; break 'dispatch; }}",
                key.ip
            )
            .unwrap();
            for (idx, &site_key) in seq.iter().enumerate() {
                let site = &cfg.sites[&site_key];
                emit_site_body(&mut out, site, idx, idx + 1 == k, false, region.cs_rel, false);
            }
            writeln!(out, "steps += {k}u32;\n}}").unwrap();
        }
        writeln!(
            out,
            "_ => {{ (*ctx).exit_kind = 1; break 'dispatch; }}\n\
                 }}\n\
             }}\n\
             (*ctx).regs[0] = r0;\n\
             (*ctx).regs[1] = r1;\n\
             (*ctx).regs[2] = r2;\n\
             (*ctx).regs[3] = r3;\n\
             (*ctx).regs[4] = r4;\n\
             (*ctx).regs[5] = r5;\n\
             (*ctx).regs[6] = r6;\n\
             (*ctx).regs[7] = r7;\n\
             (*ctx).eflags = flags;\n\
             (*ctx).undefined = undef;\n\
             (*ctx).eip = ip;\n\
             (*ctx).steps_done = steps;\n\
         }}"
        )
        .unwrap();
    } else {
        for region in &regions {
            writeln!(
                out,
                "#[inline(never)]\n\
                 #[allow(unused_variables, unused_mut, unused_assignments)]\n\
                 unsafe fn region_{id}(ctx: *mut DinePilCtx, budget: u32, steps: &mut u32) -> u32 {{\n\
                     let mut r0 = (*ctx).regs[0];\n\
                     let mut r1 = (*ctx).regs[1];\n\
                     let mut r2 = (*ctx).regs[2];\n\
                     let mut r3 = (*ctx).regs[3];\n\
                     let mut r4 = (*ctx).regs[4];\n\
                     let mut r5 = (*ctx).regs[5];\n\
                     let mut r6 = (*ctx).regs[6];\n\
                     let mut r7 = (*ctx).regs[7];\n\
                     let mut flags = (*ctx).eflags;\n\
                     let mut undef = (*ctx).undefined;\n\
                     let mut ip = (*ctx).eip;\n\
                     let mut selectors = (*ctx).selectors;\n\
                     let load_segment = (*ctx).load_segment;\n\
                     let mut seg_bases = (*ctx).seg_bases;\n\
                     let mut seg_fast_mask = (*ctx).seg_fast_mask;\n\
                     let mut code_limit = (*ctx).code_limit;\n\
                     let ram_ptr = (*ctx).ram_ptr;\n\
                     let watch_ptr = (*ctx).watch_ptr;\n\
                     let ram_fast_limit = (*ctx).ram_fast_limit;\n\
                     let code_start = (*ctx).code_start;\n\
                     let code_end = (*ctx).code_end;\n\
                     let host = (*ctx).host;\n\
                     let start_steps: u32 = *steps;\n\
                     let mut cur_steps: u32 = start_steps;\n\
                     let mut exit_rc: u32 = 1;\n\
                     'dispatch: loop {{\n\
                         match ip {{",
                id = region.id
            )
            .unwrap();
            for &key in &region.blocks {
                let seq = &cfg.blocks[&key];
                let k = seq.len();
                writeln!(
                    out,
                    "{}u32 => {{\n\
                     if cur_steps + {k}u32 > budget {{ *steps = cur_steps; (*ctx).exit_kind = 0; exit_rc = 0; break 'dispatch; }}",
                    key.ip
                )
                .unwrap();
                for (idx, &site_key) in seq.iter().enumerate() {
                    let site = &cfg.sites[&site_key];
                    emit_site_body(
                        &mut out,
                        site,
                        idx,
                        idx + 1 == k,
                        true,
                        region.cs_rel,
                        true,
                    );
                }
                writeln!(out, "cur_steps += {k}u32;\n}}").unwrap();
            }
            writeln!(
                out,
                "_ => {{\n\
                     *steps = cur_steps;\n\
                     if cur_steps == start_steps {{ (*ctx).exit_kind = 1; exit_rc = 1; }} else {{ exit_rc = 5; }}\n\
                     break 'dispatch;\n\
                 }}\n\
                 }}\n\
             }}\n\
             (*ctx).regs[0] = r0;\n\
             (*ctx).regs[1] = r1;\n\
             (*ctx).regs[2] = r2;\n\
             (*ctx).regs[3] = r3;\n\
             (*ctx).regs[4] = r4;\n\
             (*ctx).regs[5] = r5;\n\
             (*ctx).regs[6] = r6;\n\
             (*ctx).regs[7] = r7;\n\
             (*ctx).eflags = flags;\n\
             (*ctx).undefined = undef;\n\
             (*ctx).eip = ip;\n\
             (*ctx).selectors[1] = selectors[1];\n\
             (*ctx).seg_bases[1] = seg_bases[1];\n\
             (*ctx).seg_fast_mask = seg_fast_mask;\n\
             (*ctx).code_limit = code_limit;\n\
             exit_rc\n\
         }}"
            )
            .unwrap();
        }
        writeln!(
            out,
            "#[allow(unused_variables)]\n\
             pub unsafe extern \"C\" fn run_compiled(ctx: *mut DinePilCtx, budget: u32) {{\n\
                 let mut steps: u32 = 0;\n\
                 loop {{\n\
                     let cs_rel = (*ctx).selectors[1].wrapping_sub((*ctx).load_segment);\n\
                     let key = ((cs_rel as u32) << 16) | ((*ctx).eip & 0xffffu32);\n\
                     let rc = match key {{"
        )
        .unwrap();
        for region in &regions {
            let min_key = CodeKey {
                cs_rel: region.cs_rel,
                ip: region.min_ip,
            }
            .packed();
            let max_key = CodeKey {
                cs_rel: region.cs_rel,
                ip: region.max_ip,
            }
            .packed();
            writeln!(
                out,
                "        {min_key}u32..={max_key}u32 => region_{}(ctx, budget, &mut steps),",
                region.id
            )
            .unwrap();
        }
        writeln!(
            out,
            "        _ => {{ (*ctx).exit_kind = 1; 1 }}\n\
                     }};\n\
                     if rc != 5 {{ break; }}\n\
                 }}\n\
                 (*ctx).steps_done = steps;\n\
             }}"
        )
        .unwrap();
    }

    writeln!(
        out,
        "fn main() {{\n\
             let data = include_bytes!({img_include:?});\n\
             let name = CString::new({name:?}).unwrap();\n\
             let protocol = std::env::var(\"DINE_PROTOCOL\").is_ok_and(|v| v == \"1\");\n\
             let tail = std::env::args().skip(1).map(|arg| {{\n\
                 if arg.is_empty() || arg.chars().any(char::is_whitespace) {{ format!(\"\\\"{{arg}}\\\"\") }} else {{ arg }}\n\
             }}).collect::<Vec<_>>().join(\" \");\n\
             let tail = CString::new(tail).expect(\"command tail contains NUL\");\n\
             let status = unsafe {{\n\
                 if protocol {{\n\
                     dine_session_pil_v1(data.as_ptr(), data.len(), name.as_ptr(), tail.as_ptr(), run_compiled)\n\
                 }} else {{\n\
                     dine_run_pil_v1(data.as_ptr(), data.len(), name.as_ptr(), tail.as_ptr(), run_compiled)\n\
                 }}\n\
             }};\n\
             std::process::exit(if status < 0 {{ 1 }} else {{ status }});\n\
         }}"
    )
    .unwrap();

    out
}
