use super::*;

#[test]
fn test_lds_loads_offset_and_ds() {
    // LDS AX, [0x80] (0xc5, 0x06, 0x80, 0x00) -> DS:0x80 (linear 0x180)
    let bytes = vec![0xc5, 0x06, 0x80, 0x00, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_lds").unwrap();
    // Kirjoitetaan far-osoitin: offset=0x1234, selector=0x2000
    d.ram.store(0x180, super::super::memory::MemWidth::Word, 0x1234).unwrap();
    d.ram.store(0x182, super::super::memory::MemWidth::Word, 0x2000).unwrap();

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[0] & 0xffff, 0x1234); // AX
    assert_eq!(d.image.unwrap().selectors[3], 0x2000); // DS-rekisteri
}

#[test]
fn test_pusha_pushes_all_eight_registers() {
    // PUSHA (0x60), HLT (0xf4)
    let bytes = vec![0x60, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_pusha").unwrap();
    for i in 0..8 { d.cpu.regs[i] = (i as u32 + 1) * 0x1111; }
    d.cpu.regs[4] = 0x800; // SP = 0x800

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    // 8 sanaa = 16 tavua dekrementointi SP:hen -> 0x800 - 16 = 0x7f0
    assert_eq!(d.cpu.regs[4] & 0xffff, 0x7f0);

    // Tarkistetaan, että vanha SP (0x800) meni pinoon viidentenä (SS:SP+6)
    let ss_base = u32::from(d.image.unwrap().selectors[2]) * 16;
    let pushed_sp = d.ram.load(ss_base + 0x7f0 + 6, super::super::memory::MemWidth::Word).unwrap();
    assert_eq!(pushed_sp, 0x800);
}
