use super::*;

#[test]
fn test_rcl_rotates_through_carry() {
    // RCL AL, 1 (0xd0, 0xd0), HLT (0xf4)
    let bytes = vec![0xd0, 0xd0, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_rcl").unwrap();

    // AL = 0x80 (1000_0000b), CF = 1
    d.cpu.regs = 0x80;
    d.cpu.eflags |= 1; // CF = 1

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    // AL siirtyy vasemmalle: 0x00 + vanha CF (1) = 0x01
    assert_eq!(d.cpu.regs & 0xff, 0x01);
    // Uusi CF saa vanhan MSB:n (1)
    assert_eq!(d.cpu.eflags & 1, 1);
}

#[test]
fn test_rcr_rotates_through_carry() {
    // RCR BX, 1 (0xd1, 0xdb), HLT (0xf4)
    let bytes = vec![0xd1, 0xdb, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_rcr").unwrap();

    // BX = 0x0001, CF = 1
    d.cpu.regs = 0x0001;
    d.cpu.eflags |= 1; // CF = 1

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    // BX siirtyy oikealle: ylin bitti saa vanhan CF:n (0x8000)
    assert_eq!(d.cpu.regs & 0xffff, 0x8000);
    // Uusi CF saa vanhan LSB:n (1)
    assert_eq!(d.cpu.eflags & 1, 1);
}
