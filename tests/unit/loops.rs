use super::*;

#[test]
fn test_loop_counter_and_termination() {
    let bytes = vec![0xe2, 0xfe, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_loop").unwrap();
    d.cpu.regs[1] = 2; // ECX/CX = 2
    d.cpu.eflags = 0x202;

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[1] & 0xffff, 1);
    assert_eq!(d.cpu.eip, 0x100);
    assert_eq!(d.cpu.eflags, 0x202);

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[1] & 0xffff, 0);
    assert_eq!(d.cpu.eip, 0x102);
    assert_eq!(d.cpu.eflags, 0x202);
}

#[test]
fn test_loope_condition() {
    let bytes = vec![0xe1, 0xfe, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_loope").unwrap();
    d.cpu.regs[1] = 5;
    d.cpu.eflags = 0x202; // ZF = 0

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[1] & 0xffff, 4);
    assert_eq!(d.cpu.eip, 0x102);

    d.cpu.eip = 0x100;
    d.cpu.eflags |= 0x40; // ZF = 1
    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[1] & 0xffff, 3);
    assert_eq!(d.cpu.eip, 0x100);
}

#[test]
fn test_loopne_condition() {
    let bytes = vec![0xe0, 0xfe, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_loopne").unwrap();
    d.cpu.regs[1] = 5;
    d.cpu.eflags = 0x242; // ZF = 1

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[1] & 0xffff, 4);
    assert_eq!(d.cpu.eip, 0x102);

    d.cpu.eip = 0x100;
    d.cpu.eflags &= !0x40; // ZF = 0
    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[1] & 0xffff, 3);
    assert_eq!(d.cpu.eip, 0x100);
}
