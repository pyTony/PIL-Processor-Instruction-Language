use super::*;

#[test]
fn test_jmp_indirect_register() {
    // JMP AX (0xff, 0xe0)
    let bytes = vec![0xff, 0xe0, 0xf4, 0x90, 0x90, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_jmp_reg").unwrap();
    d.cpu.regs[0] = 0x105; // AX = 0x105

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.eip, 0x105);
}

#[test]
fn test_jmp_indirect_memory() {
    // JMP word [0x80] (0xff, 0x26, 0x80, 0x00) -> DS:0x80
    // DS base in monitor image is 0x100, so DS:0x80 = linear 0x180
    let bytes = vec![0xff, 0x26, 0x80, 0x00, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_jmp_mem").unwrap();
    d.ram.store(0x180, super::super::memory::MemWidth::Word, 0x104).unwrap();

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.eip, 0x104);
}
