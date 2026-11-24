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
pub(in crate::dos) enum Error { Device(String), Fault(ExecError) }

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
fn decode(p: &Loaded) -> Option<StringOp> {
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
pub(in crate::dos) fn access(p: &Loaded) -> Option<(u16, u8)> {
    let op = decode(p)?;
    if op.repeated && p.cpu.regs[1] & op.mask() == 0 { return None; }
    Some(((p.cpu.regs[2] & 0xffff) as u16, op.width.bytes()))
}

/// Returns (IP increment, partial REP) if a string-port opcode was handled.
/// A partial REP does one element, updates SI/DI and CX/ECX, but leaves EIP
/// on this opcode until the last element has completed.
pub(in crate::dos) fn io(p: &mut Loaded) -> Result<Option<(u32, bool)>, Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wolfenstein_style_rep_outsb_feeds_vga_dac_one_element_at_a_time() {
        // REP OUTSB; INT 20h. Each byte is written to VGA palette port 3C9h.
        let mut p = load(&[0xf3, 0x6e, 0xcd, 0x20], "palette.com").unwrap();
        p.cpu.regs[2] = 0x3c9;
        p.cpu.regs[6] = 0x1234_0200;
        p.cpu.regs[1] = 0xabcd_0003;
        p.cpu.regs[0] = 0x8765_4321;
        p.ram.write_bytes(0x10200, &[7, 8, 9]).unwrap();

        let r = run(p, 2).unwrap();
        assert_eq!(r.stop, Stop::Budget);
        assert_eq!((r.completed, r.loaded.cpu.eip), (0, 0x100));
        assert_eq!((r.loaded.cpu.regs[1], r.loaded.cpu.regs[6]), (0xabcd_0001, 0x1234_0202));
        let r = run(r.loaded, 3).unwrap();
        assert_eq!(r.stop, Stop::Exit(0));
        assert_eq!((r.loaded.cpu.regs[1], r.loaded.cpu.regs[6], r.loaded.cpu.regs[0]),
                   (0xabcd_0000, 0x1234_0203, 0x8765_4321));
        let mut vga = r.loaded.vga.clone();
        vga.io(0x3c7, false, 0).unwrap();
        for expected in [7, 8, 9] {
            assert_eq!(vga.io(0x3c9, true, 0).unwrap(), Some(expected));
        }
    }

    #[test]
    fn zero_count_rep_skips_memory_and_unimplemented_device() {
        let mut p = load(&[0xf3, 0x6e, 0xcd, 0x20], "zero.com").unwrap();
        p.cpu.regs[1] = 0xabcd_0000;
        p.cpu.regs[2] = 0x226; // Not yet modelled: Sound Blaster DSP
        p.cpu.regs[6] = 0xffff;
        p.image.segments[SegmentRegister::Ds.index()].limit = 0;
        let r = run(p, 4).unwrap();
        assert_eq!(r.stop, Stop::Exit(0));
        assert_eq!((r.loaded.cpu.regs[1], r.loaded.cpu.regs[6]), (0xabcd_0000, 0xffff));
    }

    #[test]
    fn insb_writes_to_es_without_changing_eax() {
        let mut p = load(&[0x6c, 0xcd, 0x20], "input.com").unwrap();
        p.cpu.regs[2] = 0x201; // Absent joystick: deterministic open bus FFh
        p.cpu.regs[7] = 0x200;
        p.cpu.regs[0] = 0x1234_5678;
        let r = run(p, 4).unwrap();
        assert_eq!(r.stop, Stop::Exit(0));
        assert_eq!(r.loaded.cpu.regs[0], 0x1234_5678);
        assert_eq!(r.loaded.cpu.regs[7], 0x201);
        assert_eq!(r.loaded.ram.load(0x10200, MemWidth::Byte).unwrap(), 0xff);
    }

    #[test]
    fn direction_flag_and_address_override_step_full_esi() {
        let mut p = load(&[0x67, 0xf3, 0x6e, 0xcd, 0x20], "backwards.com").unwrap();
        p.cpu.regs[2] = 0x3c9;
        p.cpu.regs[6] = 0x200;
        p.cpu.regs[1] = 2;
        p.cpu.eflags |= 0x400;
        p.ram.store(0x10200, MemWidth::Byte, 42).unwrap();
        p.ram.store(0x101ff, MemWidth::Byte, 43).unwrap();
        let r = run(p, 8).unwrap();
        assert_eq!(r.stop, Stop::Exit(0));
        assert_eq!((r.loaded.cpu.regs[6], r.loaded.cpu.regs[1]), (0x1fe, 0));
        let mut vga = r.loaded.vga.clone();
        vga.io(0x3c7, false, 0).unwrap();
        assert_eq!(vga.io(0x3c9, true, 0).unwrap(), Some(42));
        assert_eq!(vga.io(0x3c9, true, 0).unwrap(), Some(43));
    }

    #[test]
    fn source_segment_fault_does_not_consume_a_port_cycle() {
        let mut p = load(&[0x6e], "fault.com").unwrap();
        p.cpu.regs[2] = 0x3c9;
        p.cpu.regs[6] = 0x200;
        p.image.segments[SegmentRegister::Ds.index()].limit = 0x1ff;
        let original_vga = p.vga.clone();
        let original_cpu = p.cpu.clone();
        assert!(matches!(io(&mut p), Err(Error::Fault(_))));
        assert_eq!(p.cpu, original_cpu);
        assert_eq!(p.vga, original_vga);
    }
}
