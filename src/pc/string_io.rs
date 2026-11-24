//! DOS string I/O connects INS/OUTS to the same port handlers as IN/OUT.
//!
//! The PIL instruction decoder already models these CPU opcodes, including
//! REP, index arithmetic, and direction flags. Dine must perform actual device
//! I/O instead of relying on PIL's open-bus placeholder. Process a single
//! element per step, so a budget, interrupt or fault between elements does not
//! replay an earlier transfer.
use super::*;
use crate::flow::ExecError;

#[derive(Debug)]
pub(crate) enum Error { Device(String), Fault(ExecError) }

#[derive(Clone, Copy)]
struct StringOp {
    length: u32,
    input: bool,
    repeated: bool,
    addr32: bool,
    segment: SegmentRegister,
    width: MemWidth,
}
impl StringOp {
    fn mask(self) -> u32 { if self.addr32 { u32::MAX } else { 0xffff } }
}

/// Decode only an INS/OUTS opcode at the current CS:EIP. Other opcodes are
/// owned by the normal PIL decoder. Prefixes are not treated as I/O by
/// themselves; lock/invalid forms still reach the normal decoder.
fn decode<F>(p: &Machine<F>) -> Option<StringOp> {
    let mut at = 0u32;
    let mut addr_override = false;
    let mut operand_override = false;
    let mut repeated = false;
    let mut segment = SegmentRegister::Ds;
    let opcode = loop {
        if at >= 15 { return None; }
        let b = p.image.fetch_signature(&p.ram, p.cpu.eip, at + 1).ok()?[at as usize];
        at += 1;
        match b {
            0x66 => operand_override = true,
            0x67 => addr_override = true,
            0xf2 | 0xf3 => repeated = true,
            0x26 => segment = SegmentRegister::Es,
            0x2e => segment = SegmentRegister::Cs,
            0x36 => segment = SegmentRegister::Ss,
            0x3e => segment = SegmentRegister::Ds,
            0x64 => segment = SegmentRegister::Fs,
            0x65 => segment = SegmentRegister::Gs,
            other => break other,
        }
    };
    if !(0x6c..=0x6f).contains(&opcode) { return None; }
    let default32 = p.image.segments[SegmentRegister::Cs.index()].default_32();
    let bits = if opcode & 1 == 0 { 8 } else if default32 ^ operand_override { 32 } else { 16 };
    Some(StringOp {
        length: at,
        input: opcode & 2 == 0,
        repeated,
        addr32: default32 ^ addr_override,
        segment,
        width: MemWidth::from_bits(bits).unwrap(),
    })
}

/// For protected-mode I/O bitmap checks. A zero-count REP performs no I/O.
pub(crate) fn access<F>(p: &Machine<F>) -> Option<(u16, u8)> {
    let op = decode(p)?;
    if op.repeated && p.cpu.regs[1] & op.mask() == 0 { return None; }
    Some(((p.cpu.regs[2] & 0xffff) as u16, op.width.bytes()))
}

/// Returns (IP increment, partial REP) if a string-port opcode was handled.
/// A partial REP does one element, updates SI/DI and CX/ECX, but leaves EIP
/// on this opcode until the last element has completed.
pub(crate) fn io<F: Firmware>(p: &mut Machine<F>) -> Result<Option<(u32, bool)>, Error> {
    let Some(op) = decode(p) else { return Ok(None); };
    let mask = op.mask();
    let count = p.cpu.regs[1] & mask;
    if op.repeated && count == 0 { return Ok(Some((op.length, false))); }

    let port = (p.cpu.regs[2] & 0xffff) as u16;
    let index = if op.input { 7 } else { 6 };
    let offset = p.cpu.regs[index] & mask;
    let segment = if op.input { SegmentRegister::Es } else { op.segment };
    let context = MemoryContext { segment, cache: p.image.segments[segment.index()] };
    let ip = p.cpu.eip;
    let linear = context.translate(offset, op.width, op.input, ip).map_err(Error::Fault)?;

    // On INS, check the entire destination before a device read consumes
    // state (e.g. UART receive FIFO, VGA DAC read position).
    if op.input {
        p.ram.check_write(linear, op.width).map_err(|e| Error::Fault(e.error(ip)))?;
    }
    let old_ax = p.cpu.regs[0];
    let value_mask = match op.width {
        MemWidth::Byte => 0xff,
        MemWidth::Word => 0xffff,
        MemWidth::Dword => u32::MAX,
    };
    if !op.input {
        let value = p.ram.load(linear, op.width).map_err(|e| Error::Fault(e.error(ip)))?;
        p.cpu.regs[0] = (old_ax & !value_mask) | (value & value_mask);
    }
    let first = match (op.input, op.width) {
        (true, MemWidth::Byte) => 0xec,
        (true, _) => 0xed,
        (false, MemWidth::Byte) => 0xee,
        (false, _) => 0xef,
    };
    // Reuse scalar device handling without permanently modifying EAX.
    let result = super::transfer(p, first, port, op.width.bytes());
    let value = p.cpu.regs[0] & value_mask;
    p.cpu.regs[0] = old_ax;
    result.map_err(Error::Device)?;
    if op.input {
        p.ram.store(linear, op.width, value).map_err(|e| Error::Fault(e.error(ip)))?;
    }

    let delta = u32::from(op.width.bytes());
    let next = if p.cpu.eflags & 0x400 != 0 {
        offset.wrapping_sub(delta)
    } else {
        offset.wrapping_add(delta)
    } & mask;
    p.cpu.regs[index] = (p.cpu.regs[index] & !mask) | next;
    let partial = if op.repeated {
        let remaining = count.wrapping_sub(1) & mask;
        p.cpu.regs[1] = (p.cpu.regs[1] & !mask) | remaining;
        remaining != 0
    } else { false };
    Ok(Some((if partial { 0 } else { op.length }, partial)))
}
