use super::*;

#[test]
fn test_mul8_overflow_and_no_overflow() {
    // MUL CL (0xf6, 0xe1), HLT (0xf4)
    let bytes = vec![0xf6, 0xe1, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_mul8").unwrap();

    d.cpu.regs[0] = 10; // AL = 10
    d.cpu.regs[1] = 20; // CL = 20
    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[0] & 0xffff, 200);
    assert_eq!(d.cpu.eflags & 1, 0);
    assert_eq!((d.cpu.eflags >> 11) & 1, 0);

    d.cpu.eip = 0x100;
    d.cpu.regs[0] = 200;
    d.cpu.regs[1] = 2;
    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[0] & 0xffff, 400);
    assert_eq!(d.cpu.eflags & 1, 1);
    assert_eq!((d.cpu.eflags >> 11) & 1, 1);
}

#[test]
fn test_mul16_splits_dx_ax() {
    // MUL CX (0xf7, 0xe1), HLT (0xf4)
    let bytes = vec![0xf7, 0xe1, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_mul16").unwrap();

    d.cpu.regs[0] = 0x1000; // AX
    d.cpu.regs[1] = 0x2000; // CX
    d.cpu.regs[2] = 0;      // DX
    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[0] & 0xffff, 0x0000); // AX
    assert_eq!(d.cpu.regs[2] & 0xffff, 0x0200); // DX
    assert_eq!(d.cpu.eflags & 1, 1);
}