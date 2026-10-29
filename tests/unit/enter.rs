use super::*;

#[test]
fn test_enter_creates_stack_frame_and_allocates() {
    // ENTER 0x0010, 0x00 (0xc8, 0x10, 0x00, 0x00), HLT (0xf4)
    let bytes = vec![0xc8, 0x10, 0x00, 0x00, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_enter").unwrap();

    d.cpu.regs[4] = 0x800; // SP = 0x800
    d.cpu.regs[5] = 0x900; // BP = 0x900
    let initial_flags = d.cpu.eflags;

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);

    // 1. PUSH BP: SP -> 0x7fe, uusi BP -> 0x7fe
    assert_eq!(d.cpu.regs[5] & 0xffff, 0x7fe);

    // 2. SP varaa 0x10 tavua: 0x7fe - 0x10 = 0x7ee
    assert_eq!(d.cpu.regs[4] & 0xffff, 0x7ee);

    // 3. Vanha BP (0x900) tallennettu pinoon osoitteeseen SS:0x7fe
    let ss_base = u32::from(d.image.as_ref().unwrap().selectors[2]) * 16;
    let saved_bp = d.ram.load(ss_base + 0x7fe, super::super::memory::MemWidth::Word).unwrap();
    assert_eq!(saved_bp, 0x900);

    // 4. Liput säilyvät
    assert_eq!(d.cpu.eflags, initial_flags);
}
