use super::*;

#[test]
fn test_popa_restores_seven_registers_and_advances_sp() {
    // POPA (0x61), HLT (0xf4)
    let bytes = vec![0x61, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_popa").unwrap();
    d.cpu.regs[4] = 0x700; // SP = 0x700

    let ss_base = u32::from(d.image.as_ref().unwrap().selectors[2]) * 16;
    // Pinossa alhaalta ylöspäin (SP..SP+16): DI, SI, BP, vanha SP, BX, DX, CX, AX
    d.ram.store(ss_base + 0x700, super::super::memory::MemWidth::Word, 0x7777).unwrap(); // DI
    d.ram.store(ss_base + 0x702, super::super::memory::MemWidth::Word, 0x6666).unwrap(); // SI
    d.ram.store(ss_base + 0x704, super::super::memory::MemWidth::Word, 0x5555).unwrap(); // BP
    d.ram.store(ss_base + 0x706, super::super::memory::MemWidth::Word, 0x9999).unwrap(); // skipattu SP
    d.ram.store(ss_base + 0x708, super::super::memory::MemWidth::Word, 0x3333).unwrap(); // BX
    d.ram.store(ss_base + 0x70a, super::super::memory::MemWidth::Word, 0x2222).unwrap(); // DX
    d.ram.store(ss_base + 0x70c, super::super::memory::MemWidth::Word, 0x1111).unwrap(); // CX
    d.ram.store(ss_base + 0x70e, super::super::memory::MemWidth::Word, 0xaaaa).unwrap(); // AX

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[0] & 0xffff, 0xaaaa); // AX
    assert_eq!(d.cpu.regs[1] & 0xffff, 0x1111); // CX
    assert_eq!(d.cpu.regs[2] & 0xffff, 0x2222); // DX
    assert_eq!(d.cpu.regs[3] & 0xffff, 0x3333); // BX
    assert_eq!(d.cpu.regs[5] & 0xffff, 0x5555); // BP
    assert_eq!(d.cpu.regs[6] & 0xffff, 0x6666); // SI
    assert_eq!(d.cpu.regs[7] & 0xffff, 0x7777); // DI
    // SP kasvaa 16 tavulla: 0x700 + 16 = 0x710
    assert_eq!(d.cpu.regs[4] & 0xffff, 0x710);
}
