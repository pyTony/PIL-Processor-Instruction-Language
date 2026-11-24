//! INS/OUTS through Dine.
use super::*;
use crate::pc::ports::string_io::*;
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
