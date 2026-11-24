//! Ahead-of-time application calls into the shared native CPU forms.
//!
//! The compiler is the same for every input. Application output contains only
//! prepared state, resources, instruction arguments and an address dispatcher;
//! CPU formulas and DOS execution stay in this library.
use crate::{
    address::{self, Address16, EffectiveAddress, Gpr, SegmentRegister},
    dos,
    guest::{Decoded, GuestError, Image, Operation},
    precompiled, Cpu, ExecError, FlowContext, Program, Ram, View,
};
use crate::dos::LoadedOps;
#[cfg(feature = "dine-library")]
use std::ffi::c_char;
use std::{cell::OnceCell, path::Path, rc::Rc};

const MAGIC: &[u8; 8] = b"PILAPP02";
const MAX_RECORDS: usize = 2_097_152;
const RESOURCE: u32 = 1 << 31;
pub(crate) type Dispatch = unsafe extern "C" fn(u32) -> u32;

#[derive(Clone)]
struct Record {
    offset: u32,
    length: u8,
    partial: bool,
    dst: View,
    src: View,
    displacement: i32,
    displacement_bits: u8,
    effective: Option<(EffectiveAddress, Option<SegmentRegister>)>,
    label: String,
    native: precompiled::Native,
}
pub(crate) struct Application {
    name: String,
    image: Vec<u8>,
    resource: Vec<u8>,
    records: Vec<Record>,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).ok_or("application size overflow")?;
        let out = self
            .data
            .get(self.at..end)
            .ok_or("truncated native application")?;
        self.at = end;
        Ok(out)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn varint(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for shift in (0..35).step_by(7) {
            let b = self.byte()?;
            value |= u32::from(b & 0x7f)
                .checked_shl(shift)
                .filter(|v| v >> shift == u32::from(b & 0x7f))
                .ok_or("native application varint overflow")?;
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("native application varint too long".into())
    }
    fn dword(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn qword(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}
fn register(n: u8) -> Result<Option<Gpr>, String> {
    Ok(match n {
        0 => Some(Gpr::Eax),
        1 => Some(Gpr::Ecx),
        2 => Some(Gpr::Edx),
        3 => Some(Gpr::Ebx),
        4 => Some(Gpr::Esp),
        5 => Some(Gpr::Ebp),
        6 => Some(Gpr::Esi),
        7 => Some(Gpr::Edi),
        255 => None,
        _ => return Err("invalid prepared register".into()),
    })
}
fn segment(n: u8) -> Result<Option<SegmentRegister>, String> {
    Ok(match n {
        0 => Some(SegmentRegister::Es),
        1 => Some(SegmentRegister::Cs),
        2 => Some(SegmentRegister::Ss),
        3 => Some(SegmentRegister::Ds),
        4 => Some(SegmentRegister::Fs),
        5 => Some(SegmentRegister::Gs),
        255 => None,
        _ => return Err("invalid prepared segment".into()),
    })
}
fn view(r: &mut Reader<'_>) -> Result<View, String> {
    let register = r.byte()? as usize;
    let low = r.byte()?;
    let width = r.byte()?;
    if register >= 8
        || !matches!(width, 8 | 16 | 32)
        || low > 24
        || u16::from(low) + u16::from(width) > 32
    {
        return Err("invalid prepared binding".into());
    }
    Ok(View {
        register,
        low,
        width,
    })
}

impl Application {
    pub(crate) fn read(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 128 * 1024 * 1024 {
            return Err("native application exceeds size bound".into());
        }
        let mut r = Reader { data: bytes, at: 0 };
        if r.take(8)? != MAGIC {
            return Err("unsupported native application format".into());
        }
        let image_len = r.dword()? as usize;
        let resource_len = r.dword()? as usize;
        let count = r.dword()? as usize;
        if image_len > dos::MAX_FILE || resource_len > dos::MAX_FILE || count > MAX_RECORDS {
            return Err("native application exceeds image/record bounds".into());
        }
        let name_len = r.word()? as usize;
        let name = std::str::from_utf8(r.take(name_len)?)
            .map_err(|_| "invalid native application name")?
            .to_string();
        let image = r.take(image_len)?.to_vec();
        let resource = r.take(resource_len)?.to_vec();
        let initial = dos::load(&image, &name)?;
        if initial.dos.relocations != 0 {
            return Err("prepared startup must not contain relocation work".into());
        }
        // Identical instruction forms are stored once and referenced by every
        // byte offset (module or resource) that decodes to them.
        let unique_count = r.dword()? as usize;
        if unique_count > count {
            return Err("native application has more forms than entries".into());
        }
        let mut unique: Vec<Record> = Vec::with_capacity(unique_count);
        for _ in 0..unique_count {
            let length = r.byte()?;
            let partial = match r.byte()? {
                0 => false,
                1 => true,
                _ => return Err("invalid prepared repeat flag".into()),
            };
            if length == 0 || length > 15 {
                return Err("invalid prepared instruction extent/order".into());
            }
            let dst = view(&mut r)?;
            let src = view(&mut r)?;
            let displacement = r.dword()? as i32;
            let displacement_bits = r.byte()?;
            if !matches!(displacement_bits, 0 | 8 | 16 | 32) {
                return Err("invalid prepared branch width".into());
            }
            let effective = match r.byte()? {
                0 => None,
                1 => {
                    let form = match r.byte()? {
                        0 => Address16::BxSi,
                        1 => Address16::BxDi,
                        2 => Address16::BpSi,
                        3 => Address16::BpDi,
                        4 => Address16::Si,
                        5 => Address16::Di,
                        6 => Address16::Bp,
                        7 => Address16::Bx,
                        8 => Address16::Direct,
                        _ => return Err("invalid prepared 16-bit address".into()),
                    };
                    let displacement = r.word()?;
                    Some((
                        EffectiveAddress::Bits16 { form, displacement },
                        segment(r.byte()?)?,
                    ))
                }
                2 => {
                    let base = register(r.byte()?)?;
                    let index = register(r.byte()?)?;
                    let scale = r.byte()?;
                    let displacement = r.dword()?;
                    if !matches!(scale, 1 | 2 | 4 | 8)
                        || index == Some(Gpr::Esp)
                        || (index.is_none() && scale != 1)
                    {
                        return Err("invalid prepared 32-bit address".into());
                    }
                    Some((
                        EffectiveAddress::Bits32 {
                            base,
                            index,
                            scale,
                            displacement,
                        },
                        segment(r.byte()?)?,
                    ))
                }
                _ => return Err("invalid prepared address kind".into()),
            };
            let lo = r.qword()?;
            let hi = r.qword()?;
            let n = r.word()? as usize;
            if n > 4096 {
                return Err("too many native constant arguments".into());
            }
            let mut constants = Vec::with_capacity(n);
            for _ in 0..n {
                constants.push(r.qword()?);
            }
            let native = precompiled::prepared(u128::from(lo) | (u128::from(hi) << 64), &constants)
                .ok_or(
                    "shared native form unavailable; rebuild the application with this library",
                )?;
            let len = r.byte()? as usize;
            let label = std::str::from_utf8(r.take(len)?)
                .map_err(|_| "invalid prepared instruction label")?
                .to_string();
            unique.push(Record {
                offset: 0,
                length,
                partial,
                dst,
                src,
                displacement,
                displacement_bits,
                effective,
                label,
                native,
            });
        }
        let mut records = Vec::with_capacity(count);
        let mut previous: Option<u32> = None;
        for _ in 0..count {
            let delta = r.varint()?;
            let offset = match previous {
                None => delta,
                Some(p) if delta > 0 => p
                    .checked_add(delta)
                    .ok_or("invalid prepared instruction extent/order")?,
                Some(_) => return Err("invalid prepared instruction extent/order".into()),
            };
            previous = Some(offset);
            let form = r.varint()? as usize;
            let mut record = unique
                .get(form)
                .ok_or("invalid prepared instruction form index")?
                .clone();
            let extent = if offset & RESOURCE != 0 {
                resource.len()
            } else {
                initial.dos.module_bytes
            };
            if (offset & !RESOURCE) as usize + usize::from(record.length) > extent {
                return Err("invalid prepared instruction extent/order".into());
            }
            record.offset = offset;
            records.push(record);
        }
        if r.at != bytes.len() {
            return Err("trailing native application data".into());
        }
        Ok(Self {
            name,
            image,
            resource,
            records,
        })
    }
    pub(crate) fn attach(
        self,
        session: &mut dos::session::Session,
        dispatch: Option<Dispatch>,
    ) -> Result<(), String> {
        let p = session.machine_mut().ok_or("session state lost")?;
        let start = p.dos.module_base as usize;
        let original = p.ram.bytes()[start..start + p.dos.module_bytes].to_vec();
        // Read-only executable resources (including overlays) remain available
        // to ordinary DOS file reads without an external original file.
        p.dos.own_file = std::sync::Arc::new(self.resource);
        let resource = p.dos.own_file.clone();
        p.ram
            .track_resource(std::sync::Arc::as_ptr(&resource) as usize);
        p.blocks.mode = Some(precompiled::Mode::Off);
        let module_base = p.dos.module_base;
        session.set_backend(Box::new(Backend {
            records: self.records,
            original,
            resource,
            module_base,
            dispatch,
            last: None,
            count: 0,
            blocks: 0,
            instructions: 0,
            reference: [0; 4],
            pending: None,
            sites: Default::default(),
        }));
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Reference {
    Modified,
    Unavailable,
    Unprepared,
    Context,
}
impl Reference {
    fn name(self) -> &'static str {
        match self {
            Self::Modified => "modified",
            Self::Unavailable => "unavailable",
            Self::Unprepared => "unprepared",
            Self::Context => "context",
        }
    }
}
struct Backend {
    records: Vec<Record>,
    original: Vec<u8>,
    resource: std::sync::Arc<Vec<u8>>,
    module_base: u32,
    dispatch: Option<Dispatch>,
    last: Option<Rc<Decoded>>,
    count: u64,
    blocks: u64,
    instructions: u64,
    reference: [u64; 4],
    pending: Option<(Reference, u16, u32)>,
    sites: std::collections::BTreeMap<(u16, u32, Reference), u64>,
}
impl Backend {
    fn reason(&self, image: &Image, ram: &Ram, ip: u32, length: u32) -> Reference {
        if !image.real_mode
            || image.paging
            || image.segments[1].default_32()
            || matches!(
                image.segments[2].access,
                address::SegmentAccess::Data { big: true, .. }
            )
        {
            return Reference::Context;
        }
        let Some(linear) = image.segments[1].base.checked_add(ip) else {
            return Reference::Context;
        };
        let initial = linear
            .checked_sub(self.module_base)
            .filter(|n| (*n as usize) < self.original.len());
        let resource = ram
            .resource_origin(linear)
            .filter(|(id, _)| *id == std::sync::Arc::as_ptr(&self.resource) as usize)
            .map(|(_, offset)| RESOURCE | offset);
        let Ok(current) = image.fetch_signature(ram, ip, length) else {
            return Reference::Unprepared;
        };
        let mut changed = false;
        for offset in [initial, resource].into_iter().flatten() {
            let at = (offset & !RESOURCE) as usize;
            let original = if offset & RESOURCE != 0 {
                self.resource.as_slice()
            } else {
                self.original.as_slice()
            };
            if let Some(expected) = original.get(at..at + length as usize) {
                if current[..length as usize] == *expected {
                    return Reference::Unavailable;
                }
                changed = true;
            }
        }
        if changed {
            Reference::Modified
        } else {
            Reference::Unprepared
        }
    }
    fn record(&self, image: &Image, ram: &Ram, ip: u32) -> Option<Result<&Record, GuestError>> {
        if !image.real_mode
            || image.paging
            || image.segments[1].default_32()
            || matches!(
                image.segments[2].access,
                address::SegmentAccess::Data { big: true, .. }
            )
        {
            return None;
        }
        let linear = image.segments[1].base.checked_add(ip)?;
        let initial = linear
            .checked_sub(self.module_base)
            .filter(|n| (*n as usize) < self.original.len());
        let resource = ram
            .resource_origin(linear)
            .filter(|(id, _)| *id == std::sync::Arc::as_ptr(&self.resource) as usize)
            .map(|(_, offset)| RESOURCE | offset);
        for offset in [initial, resource].into_iter().flatten() {
            let index = match self.dispatch {
                Some(dispatch) => unsafe { dispatch(offset) },
                None => self
                    .records
                    .binary_search_by_key(&offset, |r| r.offset)
                    .map_or(u32::MAX, |i| i as u32),
            };
            if index == u32::MAX {
                continue;
            }
            let Some(record) = self
                .records
                .get(index as usize)
                .filter(|r| r.offset == offset)
            else {
                return Some(Err(GuestError::Decode(
                    "invalid native application dispatch".into(),
                )));
            };
            let bytes = match image.fetch_signature(ram, ip, u32::from(record.length)) {
                Ok(b) => b,
                Err(e) => return Some(Err(e)),
            };
            if crate::block_compiler::engine::answered_by_loop(&bytes[..record.length as usize]) {
                continue;
            }
            let original = if offset & RESOURCE != 0 {
                self.resource.as_slice()
            } else {
                self.original.as_slice()
            };
            let at = (offset & !RESOURCE) as usize;
            if bytes[..record.length as usize] == original[at..at + record.length as usize] {
                return Some(Ok(record));
            }
        }
        None
    }
}
fn context(
    record: &Record,
    image: &Image,
    ip: u32,
) -> (FlowContext, Option<address::AddressContext>) {
    let flow = FlowContext {
        next_eip: ip + u32::from(record.length),
        displacement: record.displacement,
        displacement_bits: record.displacement_bits,
        code_limit: image.segments[1].limit,
    };
    let address = record
        .effective
        .map(|(effective, override_segment)| address::AddressContext {
            effective,
            override_segment,
            segments: image.segments,
        });
    (flow, address)
}
impl Backend {
    fn block_one(
        &mut self,
        image: &Image,
        cpu: &mut Cpu,
        ram: &mut Ram,
        machine: &mut address::SegmentState,
    ) -> Option<(usize, Result<(), ExecError>)> {
        let record = self.record(image, ram, cpu.eip)?.ok()?;
        if record.partial {
            return None;
        }
        let (flow, address) = context(record, image, cpu.eip);
        let dst = record.dst;
        let src = record.src;
        let native = &record.native;
        let result = (native.form)(
            &mut cpu.regs,
            &mut cpu.eflags,
            &mut cpu.undefined_flags,
            &mut cpu.eip,
            (dst.register, dst.low),
            (src.register, src.low),
            flow,
            ram,
            address,
            machine,
            &native.consts,
        );
        Some((usize::from(result.is_ok()), result))
    }
}
impl dos::InstructionBackend for Backend {
    fn reference_instruction(&mut self, image: &Image, ram: &Ram, ip: u32, decoded: &Decoded) {
        self.pending = Some((
            self.reason(image, ram, ip, decoded.length),
            image.selectors[1],
            ip,
        ));
    }
    fn fallback_mode(&self) -> Option<precompiled::Mode> {
        Some(precompiled::Mode::Off)
    }
    fn reference_stats(&self) -> Option<u64> {
        Some(self.reference.iter().sum())
    }
    fn block_stats(&self) -> (u64, u64) {
        (self.blocks, self.instructions)
    }
    fn single_stats(&self) -> u64 {
        self.count
    }
    fn reference_retired(&mut self) {
        let (reason, cs, ip) = self.pending.unwrap_or((Reference::Unprepared, 0, 0));
        self.reference[reason as usize] += 1;
        let site = (cs, ip, reason);
        if let Some(n) = self.sites.get_mut(&site) {
            *n += 1;
        } else if self.sites.keys().filter(|(_, _, r)| *r == reason).count() < 128 {
            self.sites.insert(site, 1);
        }
    }
    fn diagnostics(&self) -> String {
        let mut top = Vec::new();
        for reason in [
            Reference::Modified,
            Reference::Unavailable,
            Reference::Unprepared,
            Reference::Context,
        ] {
            let mut group: Vec<_> = self
                .sites
                .iter()
                .filter(|((_, _, r), _)| *r == reason)
                .collect();
            group.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
            top.extend(group.into_iter().take(8));
        }
        let sites = top
            .into_iter()
            .map(|(&(cs, ip, reason), n)| {
                format!(
                    "{{\"cs\":{cs},\"ip\":{ip},\"reason\":\"{}\",\"count\":{n}}}",
                    reason.name()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(",\"prepared_reference_modified\":{},\"prepared_reference_unavailable\":{},\"prepared_reference_unprepared\":{},\"prepared_reference_context\":{},\"prepared_reference_sites\":[{sites}]",self.reference[0],self.reference[1],self.reference[2],self.reference[3])
    }
    fn block(
        &mut self,
        image: &Image,
        cpu: &mut Cpu,
        ram: &mut Ram,
        machine: &mut address::SegmentState,
        budget: usize,
        chain: bool,
    ) -> Option<(usize, Result<(), ExecError>)> {
        if budget == 0 {
            return None;
        }
        let entry = *machine;
        let entry_eflags = cpu.eflags;
        let (mut total, mut result) = self.block_one(image, cpu, ram, machine)?;
        // Keep going through consecutive prepared instructions without a trip
        // round the DOS loop each. `image` is only refreshed by the caller, so
        // stop at any change to segment/mode/flag state it holds, at an
        // interrupt shadow, at a fault, and at the caller's step budget.
        while chain && result.is_ok() && total < budget {
            if machine.ss_shadow != 0
                || machine.sti_shadow != 0
                || machine.flag_checkpoint.is_some()
                || machine.segments != entry.segments
                || machine.selectors != entry.selectors
                || machine.msw != entry.msw
                || machine.real_mode != entry.real_mode
                || machine.paging != entry.paging
                || (cpu.eflags ^ entry_eflags) & 0x300 != 0
            {
                break;
            }
            match self.block_one(image, cpu, ram, machine) {
                Some((n, r)) => {
                    total += n;
                    result = r;
                }
                None => break,
            }
        }
        self.blocks += 1;
        self.instructions += total as u64;
        Some((total, result))
    }
    fn decode(
        &mut self,
        image: &Image,
        ram: &Ram,
        ip: u32,
    ) -> Option<Result<Rc<Decoded>, GuestError>> {
        self.last = None;
        self.pending = None;
        let record = match self.record(image, ram, ip) {
            Some(Ok(record)) => record,
            Some(Err(error)) => return Some(Err(error)),
            None => {
                return None;
            }
        };
        let program = Program {
            tokens: Vec::new(),
            packed: Vec::new(),
            widths: Vec::new(),
            register_effects: 0,
            operand_width: record.dst.width,
            source_width: record.src.width,
            uses_flow: false,
            uses_displacement: false,
            uses_memory: record.effective.is_some(),
            has_read_address: false,
            flushes_lazy: true,
            partial: record.partial,
            checkpoint_flags: false,
        };
        let (flow, address) = context(record, image, ip);
        let native = OnceCell::new();
        native.set(Some(record.native.clone())).unwrap();
        let decoded = Rc::new(Decoded {
            label: record.label.clone(),
            length: u32::from(record.length),
            operation: Operation::Pil {
                program,
                dst: record.dst,
                src: record.src,
                flow,
                address,
            },
            native,
        });
        self.last = Some(decoded.clone());
        Some(Ok(decoded))
    }
    fn execute(
        &mut self,
        _: &Image,
        decoded: &Rc<Decoded>,
        cpu: &mut Cpu,
        ram: &mut Ram,
        machine: &mut address::SegmentState,
    ) -> Option<Result<(), ExecError>> {
        if !self
            .last
            .as_ref()
            .is_some_and(|last| Rc::ptr_eq(last, decoded))
        {
            return None;
        }
        let Operation::Pil {
            program,
            dst,
            src,
            flow,
            address,
        } = &decoded.operation
        else {
            return None;
        };
        let native = decoded.native()?;
        let old = cpu.eip;
        let result = (native.form)(
            &mut cpu.regs,
            &mut cpu.eflags,
            &mut cpu.undefined_flags,
            &mut cpu.eip,
            (dst.register, dst.low),
            (src.register, src.low),
            *flow,
            ram,
            *address,
            machine,
            &native.consts,
        );
        if result.is_ok() && (!program.partial || cpu.eip != old) {
            self.count += 1;
        }
        Some(result)
    }
}

fn put_varint(out: &mut Vec<u8>, mut n: u32) {
    while n >= 0x80 {
        out.push(n as u8 | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}
fn put16(out: &mut Vec<u8>, n: u16) {
    out.extend(n.to_le_bytes());
}
fn put32(out: &mut Vec<u8>, n: u32) {
    out.extend(n.to_le_bytes());
}
fn put64(out: &mut Vec<u8>, n: u64) {
    out.extend(n.to_le_bytes());
}
fn binding(out: &mut Vec<u8>, v: View) {
    out.extend([v.register as u8, v.low, v.width]);
}
fn address(out: &mut Vec<u8>, a: Option<address::AddressContext>) {
    match a {
        None => out.push(0),
        Some(a) => {
            match a.effective {
                EffectiveAddress::Bits16 { form, displacement } => {
                    out.extend([1, form as u8]);
                    put16(out, displacement);
                }
                EffectiveAddress::Bits32 {
                    base,
                    index,
                    scale,
                    displacement,
                } => {
                    out.extend([
                        2,
                        base.map_or(255, |r| r as u8),
                        index.map_or(255, |r| r as u8),
                        scale,
                    ]);
                    put32(out, displacement);
                }
            }
            out.push(a.override_segment.map_or(255, |r| r as u8));
        }
    }
}

/// Runtime only needs the mnemonic (port-access checks and statistics), so
/// operands are omitted unless `debug-labels` is enabled. A trailing space
/// is kept so "IN " and "OUT " stay distinguishable from other mnemonics.
fn compact_label(label: &str) -> &str {
    if cfg!(feature = "debug-labels") {
        return label;
    }
    let skip = if label.starts_with("REP ") {
        4
    } else if label.starts_with("REPE ") {
        5
    } else if label.starts_with("REPNE ") {
        6
    } else {
        0
    };
    match label[skip..].find(' ') {
        Some(i) => &label[..skip + i + 1],
        None => label,
    }
}

/// Compile every possible byte entry in the initial image, not just a trace.
/// Entry discovery therefore does not remove unvisited functions or indirect
/// targets. Unchanged entries call native forms; generated/modified code has
/// its own reference path. Resources are data, not a second execution engine.
pub(crate) fn compile(bytes: &[u8], name: &str) -> Result<(Vec<u8>, Vec<u32>), String> {
    let mut loaded = dos::load(bytes, name)?;
    // The loader installs DOS=UMB after allocating the process. Undo that
    // optional service setup while recording the original allocation request;
    // runtime configuration installs it normally on the prepared startup.
    loaded.select_xms(false)?;
    let base = loaded.dos.module_base as usize;
    let module = loaded.ram.bytes()[base..base + loaded.dos.module_bytes].to_vec();
    // A new, zero-relocation startup header carries the already relocated image.
    let total = 32 + module.len();
    let mut startup = vec![0u8; 32];
    startup[..2].copy_from_slice(b"MZ");
    let mut word = |at: usize, n: u16| startup[at..at + 2].copy_from_slice(&n.to_le_bytes());
    word(2, (total % 512) as u16);
    word(4, total.div_ceil(512) as u16);
    word(8, 2);
    let psp_base = usize::from(loaded.dos.psp) * 16;
    let load_segment = (loaded.dos.module_base >> 4) as u16;
    let extra = (loaded.dos.allocated_end as usize - psp_base) / 16 - 16 - module.len().div_ceil(16);
    word(12, extra.max(1) as u16);
    word(14, loaded.selectors[2].wrapping_sub(load_segment));
    word(16, loaded.cpu.regs[4] as u16);
    word(20, loaded.cpu.eip as u16);
    word(22, loaded.selectors[1].wrapping_sub(load_segment));
    startup.extend(&module);
    let mut unique: Vec<Vec<u8>> = Vec::new();
    let mut unique_index: std::collections::HashMap<Vec<u8>, u32> =
        std::collections::HashMap::new();
    let mut entries: Vec<(u32, u32)> = Vec::new();
    let mut offsets = Vec::new();
    let mut image = loaded.image.clone();
    let mut code = loaded.ram.clone();
    image.selectors[1] = (loaded.dos.module_base >> 4) as u16;
    image.segments[1] = address::SegmentCache::real_mode(image.selectors[1]);
    image.end = image.end.max(0x10000);
    for (tag, data) in [(0, module.as_slice()), (RESOURCE, bytes)] {
        for offset in 0..data.len() {
            // Normalize only the enumeration CS:IP. Relative branches, return IPs
            // and effective addresses are supplied from live state by the library.
            let remaining = (data.len() - offset).min(15);
            let mut encoded = [0u8; 15];
            encoded[..remaining].copy_from_slice(&data[offset..offset + remaining]);
            code.write_bytes(loaded.dos.module_base, &encoded)
                .map_err(|e| format!("enumeration access: {e:?}"))?;
            let Ok(decoded) = image.decode(&code, 0) else {
                continue;
            };
            if offset + decoded.length as usize > data.len() {
                continue;
            }
            if crate::block_compiler::engine::answered_by_loop(&encoded[..decoded.length as usize])
            {
                continue;
            }
            let Operation::Pil {
                program,
                dst,
                src,
                flow,
                address: operand,
            } = &decoded.operation
            else {
                continue;
            };
            if decoded.native().is_none() {
                continue;
            }
            let (text, constants) = precompiled::shape(program);
            let key = precompiled::key(&text);
            let mut records = Vec::new();
            records.extend([decoded.length as u8, u8::from(program.partial)]);
            binding(&mut records, *dst);
            binding(&mut records, *src);
            put32(&mut records, flow.displacement as u32);
            records.push(flow.displacement_bits);
            address(&mut records, *operand);
            put64(&mut records, key as u64);
            put64(&mut records, (key >> 64) as u64);
            put16(
                &mut records,
                constants
                    .len()
                    .try_into()
                    .map_err(|_| "native argument count overflow")?,
            );
            for c in constants {
                put64(&mut records, c);
            }
            let label = compact_label(&decoded.label).as_bytes();
            records.push(
                label
                    .len()
                    .try_into()
                    .map_err(|_| "instruction label exceeds artifact bounds")?,
            );
            records.extend(label);
            let form = match unique_index.get(&records) {
                Some(&i) => i,
                None => {
                    let i = unique.len() as u32;
                    unique_index.insert(records.clone(), i);
                    unique.push(records);
                    i
                }
            };
            entries.push((tag | offset as u32, form));
            offsets.push(tag | offset as u32);
        }
    }
    if offsets.is_empty() {
        return Err("input has no available native CPU forms".into());
    }
    let mut out = MAGIC.to_vec();
    put32(&mut out, startup.len() as u32);
    put32(&mut out, bytes.len() as u32);
    put32(&mut out, offsets.len() as u32);
    put16(
        &mut out,
        name.len()
            .try_into()
            .map_err(|_| "application name too long")?,
    );
    out.extend(name.as_bytes());
    out.extend(startup);
    out.extend(bytes);
    put32(&mut out, unique.len() as u32);
    for form in &unique {
        out.extend(form);
    }
    let mut previous = 0u32;
    for (n, &(offset, form)) in entries.iter().enumerate() {
        put_varint(&mut out, if n == 0 { offset } else { offset - previous });
        put_varint(&mut out, form);
        previous = offset;
    }
    if out.len() > 128 * 1024 * 1024 || offsets.len() > MAX_RECORDS {
        return Err("compiled application exceeds artifact bounds".into());
    }
    Ok((out, offsets))
}

pub(crate) fn emit(input: &str, output: &str) -> Result<String, String> {
    let bytes = std::fs::read(input).map_err(|e| format!("input: {e}"))?;
    let name = Path::new(input)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid input filename")?;
    let (data, offsets) = compile(&bytes, name)?;
    let path = Path::new(output);
    let binary = path.with_extension("pilapp");
    let map_file = path.with_extension("pilmap");
    if path.exists() || binary.exists() || map_file.exists() {
        return Err("native application output already exists".into());
    }
    let include = binary
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid application output filename")?;
    let initial_len = offsets
        .iter()
        .copied()
        .filter(|n| n & RESOURCE == 0)
        .max()
        .map_or(0, |n| n as usize + 1);
    let resource_len = offsets
        .iter()
        .copied()
        .filter(|n| n & RESOURCE != 0)
        .map(|n| n & !RESOURCE)
        .max()
        .map_or(0, |n| n as usize + 1);
    // Bitmap of prepared byte offsets plus one running count per 64-bit word.
    // Offsets are sorted (module first, then resource), so a form's record
    // index equals the number of set bits before it.
    let total = initial_len + resource_len;
    let words = total.div_ceil(64);
    let mut bits = vec![0u64; words];
    let mut previous: Option<usize> = None;
    for &offset in offsets.iter() {
        let at = if offset & RESOURCE == 0 {
            offset as usize
        } else {
            initial_len + (offset & !RESOURCE) as usize
        };
        if bits[at / 64] >> (at % 64) & 1 != 0 || previous.is_some_and(|p| p >= at) {
            return Err("native application offsets are not strictly ascending".into());
        }
        bits[at / 64] |= 1 << (at % 64);
        previous = Some(at);
    }
    let mut map: Vec<u8> = bits.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut running = 0u32;
    for w in &bits {
        map.extend(running.to_le_bytes());
        running += w.count_ones();
    }
    let map_include = map_file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid application map filename")?;
    let mut source=String::from("// Generated application calls; CPU and DOS implementation is in Dine.\nuse std::ffi::{c_char,CString};\n#[cfg_attr(windows,link(name=\"dine\",kind=\"raw-dylib\"))]\n#[cfg_attr(not(windows),link(name=\"dine\"))]\nextern \"C\" { fn dine_run_application_v1(data:*const u8,len:usize,name:*const c_char,tail:*const c_char,dispatch:unsafe extern \"C\" fn(u32)->u32)->i32; fn dine_session_application_v1(data:*const u8,len:usize,name:*const c_char,tail:*const c_char,dispatch:unsafe extern \"C\" fn(u32)->u32)->i32; }\n");
    source.push_str(&format!("unsafe extern \"C\" fn dispatch(offset:u32)->u32{{\nlet map=include_bytes!({map_include:?});\nlet n=(offset&0x7fffffff)as usize;\nlet at=if offset&0x80000000==0{{if n>={initial_len}{{return u32::MAX;}}n}}else{{if n>={resource_len}{{return u32::MAX;}}n+{initial_len}}};\nlet w=u64::from_le_bytes(map[at/64*8..at/64*8+8].try_into().unwrap());\nlet bit=at%64;\nif w>>bit&1==0{{return u32::MAX;}}\nlet r={words}*8+at/64*4;\nu32::from_le_bytes(map[r..r+4].try_into().unwrap())+(w&((1u64<<bit)-1)).count_ones()\n}}\n"));
    source.push_str(&format!("fn main(){{\nlet data=include_bytes!({include:?});\nlet name=CString::new({name:?}).unwrap();\n"));
    source.push_str(
        r#"let protocol=std::env::var("DINE_PROTOCOL").is_ok_and(|v|v=="1");
let tail=std::env::args().skip(1).map(|arg| {
    if arg.is_empty()||arg.chars().any(char::is_whitespace){format!("\"{arg}\"")}else{arg}
}).collect::<Vec<_>>().join(" ");
let tail=CString::new(tail).expect("command tail contains NUL");
let status=unsafe{if protocol{
    dine_session_application_v1(data.as_ptr(),data.len(),name.as_ptr(),tail.as_ptr(),dispatch)
}else{
    dine_run_application_v1(data.as_ptr(),data.len(),name.as_ptr(),tail.as_ptr(),dispatch)
}};
std::process::exit(if status<0{1}else{status});
}
"#,
    );
    std::fs::write(&binary, data).map_err(|e| e.to_string())?;
    std::fs::write(&map_file, map).map_err(|e| e.to_string())?;
    std::fs::write(path, source).map_err(|e| e.to_string())?;
    Ok(format!("prepared {} native instruction entries; relocation completed at compilation; generated/modified code and executable resources retained separately",offsets.len()))
}

/// Foreign buffers, strings and dispatch callback must remain valid for the
/// synchronous call. The application contains no copy of the system runtime.
#[cfg(feature = "dine-library")]
#[no_mangle]
pub unsafe extern "C" fn dine_run_application_v1(
    data: *const u8,
    len: usize,
    name: *const c_char,
    tail: *const c_char,
    dispatch: Option<Dispatch>,
) -> i32 {
    unsafe { application_entry(data, len, name, tail, dispatch, false) }
}
#[cfg(feature = "dine-library")]
#[no_mangle]
pub unsafe extern "C" fn dine_session_application_v1(
    data: *const u8,
    len: usize,
    name: *const c_char,
    tail: *const c_char,
    dispatch: Option<Dispatch>,
) -> i32 {
    unsafe { application_entry(data, len, name, tail, dispatch, true) }
}
#[cfg(feature = "dine-library")]
unsafe fn application_entry(
    data: *const u8,
    len: usize,
    name: *const c_char,
    tail: *const c_char,
    dispatch: Option<Dispatch>,
    protocol: bool,
) -> i32 {
    let run = || -> Result<i32, String> {
        if data.is_null() || name.is_null() || tail.is_null() || len > 128 * 1024 * 1024 {
            return Err("invalid native application arguments".into());
        }
        let application = Application::read(unsafe { std::slice::from_raw_parts(data, len) })?;
        let name = unsafe { std::ffi::CStr::from_ptr(name) }
            .to_str()
            .map_err(|e| e.to_string())?;
        let tail = unsafe { std::ffi::CStr::from_ptr(tail) }
            .to_str()
            .map_err(|e| e.to_string())?;
        let dispatch = dispatch.ok_or("missing native application dispatcher")?;
        let config = application_config_path();
        let (mut session, mut options) = crate::dine_ffi::prepare(
            &application.image,
            name,
            tail,
            config.as_deref(),
        )?;
        // A native application is a desktop DOS program on Windows. The
        // window frontend supplies host mouse events to INT 33h; terminal
        // callers of the general Dine ABI keep their non-window default.
        options.window = cfg!(windows);
        application.attach(&mut session, Some(dispatch))?;
        if protocol {
            dos::session::serve_native(session, name)?;
            Ok(0)
        } else {
            crate::dos_text_io::run(&mut session, &options, false)
        }
    };
    match std::panic::catch_unwind(run) {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            eprintln!("Dine native application: {e}");
            -1
        }
        Err(_) => {
            eprintln!("Dine native application panicked");
            -1
        }
    }
}

/// Honor PILDOS_CONFIG and a config in the launch directory first (the
/// shared resolver handles both). Native apps are often launched from a
/// nested folder such as driveD/TP/BIN/native, so when neither is present,
/// search executable parents for the checkout's pildos.conf.
#[cfg(feature = "dine-library")]
fn application_config_path() -> Option<String> {
    let (selected, required) = crate::dos_config::selected_path(None);
    if required || selected.is_file() {
        return None;
    }
    let executable = std::env::current_exe().ok()?;
    executable
        .ancestors()
        .skip(1)
        .map(|directory| directory.join("pildos.conf"))
        .find(|candidate| candidate.is_file())
        .and_then(|path| path.to_str().map(str::to_owned))
}

/// Development inspection through the existing session protocol. Native
/// executables supply their own compiled dispatcher to the DLL entry point.
pub(crate) fn open(
    path: &str,
    mount: Option<&str>,
    args: Option<&str>,
    config: Option<&str>,
    fpu: Option<&str>,
) -> Result<dos::session::Session, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let application = Application::read(&bytes)?;
    let mut drives = Vec::new();
    if let Some(mount) = mount {
        drives.push(dos::session::open_mount_source(mount)?);
    }
    drives.push(dos::Source::Memory(dos::MemDir::default()));
    let loaded = dos::load_with_drives(&application.image, &application.name, drives, None)?;
    let mut session = dos::session::configured(loaded, args, config, mount.map(|_| b'C'), fpu)?;
    application.attach(&mut session, None)?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn compare(bytes: &[u8], name: &str, tail: &str) {
        if precompiled::forms() == 0 {
            return;
        }
        let (original, offsets) = compile(bytes, name).unwrap();
        assert!(!offsets.is_empty());
        let app = Application::read(&original).unwrap();
        assert_eq!(app.resource, bytes);
        let mut reference = dos::load(bytes, name).unwrap();
        reference.blocks.mode = Some(precompiled::Mode::Off);
        dos::set_command_tail(&mut reference, tail).unwrap();
        let mut prepared = dos::load(&app.image, name).unwrap();
        assert_eq!(prepared.dos.relocations, 0);
        dos::set_command_tail(&mut prepared, tail).unwrap();
        assert_eq!(prepared.cpu, reference.cpu);
        assert_eq!(prepared.image, reference.image);
        assert_eq!(prepared.ram.bytes(), reference.ram.bytes());
        prepared.dos.own_file = std::sync::Arc::new(Vec::new()); // attach restores resources, never a host file.
        let mut reference = dos::session::Session::new(reference);
        let mut prepared = dos::session::Session::new(prepared);
        app.attach(&mut prepared, None).unwrap();
        for budget in [1, 2, 3, 7, 16, 257, 1000].into_iter().cycle().take(100) {
            let a = reference.run(budget).unwrap();
            let b = prepared.run(budget).unwrap();
            assert_eq!(a, b);
            let a = reference.machine().unwrap();
            let b = prepared.machine().unwrap();
            assert_eq!(a.cpu, b.cpu);
            assert_eq!(a.image, b.image);
            assert_eq!(a.ram.bytes(), b.ram.bytes());
            assert_eq!(reference.exit_code(), prepared.exit_code());
            if reference.exit_code().is_some() {
                let (reply, _) = prepared.command(0, "snapshot").unwrap();
                assert!(reply.contains("\"interpreter_instructions\":0"), "{reply}");
                assert!(
                    !reply.contains("\"native_total_instructions\":0"),
                    "{reply}"
                );
                return;
            }
        }
        panic!("fixture did not terminate");
    }
    #[test]
    fn prepared_com_keeps_register_memory_and_repeat_state() {
        compare(
            &[
                0xb9, 3, 0, 0xbf, 0, 3, 0xb0, b'X', 0xf3, 0xaa, 0xb8, 42, 0x4c, 0xcd, 0x21,
            ],
            "PROOF.COM",
            "",
        );
    }
    #[test]
    fn prepared_mz_relocates_once_and_keeps_calls_segments_and_command_tail() {
        compare(
            include_bytes!("../programs/dos/MZARGS.EXE"),
            "MZARGS.EXE",
            "A B",
        );
    }
    #[test]
    fn file_loaded_code_uses_its_ahead_of_time_resource_entry() {
        // Open our executable resource, seek to its final five bytes, read
        // them to a separate buffer, then enter that loaded routine.
        let mut bytes = vec![
            0xb8, 0, 0x3d, 0xba, 0, 0, 0xcd, 0x21, 0x89, 0xc3, 0xb8, 2, 0x42, 0xb9, 0xff, 0xff,
            0xba, 0xfb, 0xff, 0xcd, 0x21, 0xb4, 0x3f, 0xb9, 5, 0, 0xba, 0, 6, 0xcd, 0x21, 0xe9, 0,
            0,
        ];
        let filename = 0x100 + bytes.len() as u16;
        bytes[4..6].copy_from_slice(&filename.to_le_bytes());
        let displacement = (0x600i32 - (0x100 + bytes.len() as i32)) as i16;
        bytes[32..34].copy_from_slice(&displacement.to_le_bytes());
        bytes.extend(b"RESOURCE.COM\0");
        bytes.extend([0xb8, 42, 0x4c, 0xcd, 0x21]);
        compare(&bytes, "RESOURCE.COM", "");
    }
    #[test]
    fn resource_origin_hints_preserve_overlap_a20_and_architectural_equality() {
        let mut ram = Ram::new(2 * 1024 * 1024);
        let reference = ram.clone();
        ram.track_resource(7);
        ram.resource_read(7, 0x100, 0, 10);
        ram.resource_read(7, 0x102, 100, 4);
        assert_eq!(ram.resource_origin(0x101), Some((7, 1)));
        assert_eq!(ram.resource_origin(0x102), Some((7, 100)));
        assert_eq!(ram.resource_origin(0x105), Some((7, 103)));
        assert_eq!(ram.resource_origin(0x106), Some((7, 6)));
        ram.resource_read(8, 0x100, 500, 10);
        assert_eq!(ram.resource_origin(0x100), Some((7, 0)));
        ram.set_a20_enabled(false);
        ram.resource_read(7, 0xffffe, 50, 4);
        assert_eq!(ram.resource_origin(0xfffff), Some((7, 51)));
        assert_eq!(ram.resource_origin(0x100000), Some((7, 52)));
        ram.set_a20_enabled(true);
        assert_eq!(ram, reference);
    }
    #[test]
    fn changed_code_default_size_uses_the_reference_instruction_width() {
        if precompiled::forms() == 0 {
            return;
        }
        let (bytes, _) = compile(&[0x40, 0xcd, 0x20], "WIDTH.COM").unwrap();
        let app = Application::read(&bytes).unwrap();
        let mut p = dos::load(&app.image, "WIDTH.COM").unwrap();
        p.cpu.regs[0] = 0x1234_ffff;
        p.image.segments[1].access = address::SegmentAccess::Code {
            readable: true,
            conforming: false,
            big: true,
            dpl: 0,
        };
        let mut session = dos::session::Session::new(p);
        app.attach(&mut session, None).unwrap();
        session.run(1).unwrap();
        assert_eq!(session.machine().unwrap().cpu.regs[0], 0x1235_0000);
        assert_eq!(
            session.machine().unwrap().blocks.interpreted_instructions,
            1
        );
    }
    #[test]
    fn prepared_port_instructions_keep_the_shared_dos_service_boundary() {
        if precompiled::forms() == 0 {
            return;
        }
        let bytes = [0x26, 0xe4, 0x91, 0xcd, 0x20];
        let mut original = dos::load(&bytes, "PORTS.COM").unwrap();
        original.blocks.mode = Some(precompiled::Mode::Off);
        let mut out = crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let reference = dos::run_with_backend(original, 20, &mut out, None).unwrap();
        assert!(matches!(reference.stop, dos::Stop::Service(_)));
        let (data, _) = compile(&bytes, "PORTS.COM").unwrap();
        let app = Application::read(&data).unwrap();
        let mut session = dos::session::Session::new(dos::load(&app.image, "PORTS.COM").unwrap());
        app.attach(&mut session, None).unwrap();
        session.run(20).unwrap();
        assert_eq!(session.machine().unwrap().cpu, reference.loaded.cpu);
        assert_eq!(session.machine().unwrap().ram, reference.loaded.ram);
        assert_eq!(session.exit_code(), None);
        assert!(session
            .command(0, "snapshot")
            .unwrap()
            .0
            .contains("unimplemented service:"));
    }
    #[test]
    fn malformed_prepared_inputs_are_rejected_without_execution() {
        assert!(Application::read(b"PILAPP01").is_err());
        assert!(Application::read(b"PILAPP02").is_err());
        if precompiled::forms() == 0 {
            return;
        }
        let (bytes, _) = compile(&[0x40, 0xcd, 0x20], "PROOF.COM").unwrap();
        for n in [0, 7, 19, bytes.len() - 1] {
            assert!(Application::read(&bytes[..n]).is_err());
        }
        let mut invalid = bytes.clone();
        invalid[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Application::read(&invalid).is_err());
        let mut invalid = bytes;
        invalid.push(0);
        assert!(Application::read(&invalid).is_err());
    }
    #[test]
    fn compact_format_deduplicates_forms_and_compacts_labels() {
        if cfg!(feature = "debug-labels") {
            assert_eq!(compact_label("MOV AX, 1234"), "MOV AX, 1234");
        } else {
            assert_eq!(compact_label("MOV AX, 1234"), "MOV ");
            assert_eq!(compact_label("IN AL, 60"), "IN ");
            assert_eq!(compact_label("REP MOVSB"), "REP MOVSB");
            assert_eq!(compact_label("REPE CMPSB"), "REPE CMPSB");
        }
        if precompiled::forms() == 0 {
            return;
        }
        let mut com = vec![0x40; 32]; // 32x INC AX
        com.extend([0xcd, 0x20]);
        let (encoded, offsets) = compile(&com, "REPEAT.COM").unwrap();
        let app = Application::read(&encoded).unwrap();
        assert_eq!(app.records.len(), offsets.len());
        assert!(app.records.len() >= 64);
        assert!(app.records.iter().any(|r| r.offset & RESOURCE != 0));
        assert!(app.records.iter().zip(&offsets).all(|(r, &o)| r.offset == o));
        // Deduplicated forms should keep 64+ records well below 64 * ~30 bytes.
        assert!(encoded.len() < 1024, "encoded len={}", encoded.len());
    }
    #[test]
    fn prepared_backend_chains_consecutive_instructions_and_matches_reference() {
        if precompiled::forms() == 0 {
            return;
        }
        // MOV CX, 20; XOR AX, AX; INC AX; LOOP -3; MOV AX, 4C00h; INT 21h
        let com = [
            0xb9, 20, 0x00, 0x31, 0xc0, 0x40, 0xe2, 0xfd, 0xb8, 0x00, 0x4c, 0xcd, 0x21,
        ];
        let (bytes, _) = compile(&com, "CHAIN.COM").unwrap();
        let app = Application::read(&bytes).unwrap();
        let mut reference = dos::load(&com, "CHAIN.COM").unwrap();
        reference.blocks.mode = Some(precompiled::Mode::Off);
        let mut ref_session = dos::session::Session::new(reference);
        let mut prepared = dos::session::Session::new(dos::load(&app.image, "CHAIN.COM").unwrap());
        app.attach(&mut prepared, None).unwrap();
        ref_session.run(500).unwrap();
        prepared.run(500).unwrap();
        assert_eq!(ref_session.exit_code(), Some(0));
        assert_eq!(prepared.exit_code(), Some(0));
        assert_eq!(
            prepared.machine().unwrap().cpu.regs[0] & 0xffff,
            ref_session.machine().unwrap().cpu.regs[0] & 0xffff
        );
        assert_eq!(
            prepared.machine().unwrap().ram.bytes(),
            ref_session.machine().unwrap().ram.bytes()
        );
        let (reply, _) = prepared.command(0, "snapshot").unwrap();
        let blocks: u64 = reply
            .split("\"native_blocks\":")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.parse().ok())
            .unwrap();
        let total: u64 = reply
            .split("\"native_total_instructions\":")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.parse().ok())
            .unwrap();
        assert!(total >= 40, "expected >=40 native instructions, got {reply}");
        assert!(blocks < total, "expected block chaining (blocks={blocks} < total={total})");
        assert!(reply.contains("\"interpreter_instructions\":0"), "{reply}");
    }
}
