use super::*;

#[test]
fn test_mul8_overflow_and_no_overflow() {
    // MUL CL (0xf6, 0xc1), HLT (0xf4)
    let bytes = vec![0xf6, 0xc1, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_mul8").unwrap();

    // 10 * 20 = 200 (mahtuu AL:ään, AH=0 -> CF=OF=0)
    d.cpu.regs = 10;
    d.cpu.regs = 20;
    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs & 0xffff, 200);
    assert_eq!(d.cpu.eflags & 1, 0); // CF = 0
    assert_eq!((d.cpu.eflags >> 11) & 1, 0); // OF = 0

    // Nollataan ja testataan ylivuoto: 200 * 2 = 400 (0x0190 -> AH=1, AL=0x90 -> CF=OF=1)
    d.cpu.eip = 0x100;
    d.cpu.regs = 200;
    d.cpu.regs = 2;
    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs & 0xffff, 400);
    assert_eq!(d.cpu.eflags & 1, 1); // CF = 1
    assert_eq!((d.cpu.eflags >> 11) & 1, 1); // OF = 1
}

#[test]
fn test_mul16_splits_dx_ax() {
    // MUL CX (0xf7, 0xe1), HLT (0xf4)
    let bytes = vec![0xf7, 0xe1, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_mul16").unwrap();

    // 0x1000 * 0x2000 = 0x0200_0000 -> DX=0x0200, AX=0x0000
    d.cpu.regs = 0x1000; // AX
    d.cpu.regs = 0x2000; // CX
    d.cpu.regs = 0;      // DX
    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs & 0xffff, 0x0000); // AX
    assert_eq!(d.cpu.regs & 0xffff, 0x0200); // DX
    assert_eq!(d.cpu.eflags & 1, 1); // CF = 1 (DX != 0)
}
