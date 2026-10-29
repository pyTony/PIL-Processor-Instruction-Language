use super::*;

#[test]
fn test_rcl_rotates_through_carry() {
    let bytes = vec![0xd0, 0xd0, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_rcl").unwrap();

    d.cpu.regs[0] = 0x80; // AL = 0x80
    d.cpu.eflags |= 1;    // CF = 1

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[0] & 0xff, 0x01);
    assert_eq!(d.cpu.eflags & 1, 1);
}

#[test]
fn test_rcr_rotates_through_carry() {
    let bytes = vec![0xd1, 0xdb, 0xf4];
    let mut d = Debugger::new("counter").unwrap();
    d.load_image(&bytes, "test_rcr").unwrap();

    d.cpu.regs[3] = 0x0001; // BX = 0x0001
    d.cpu.eflags |= 1;      // CF = 1

    let stop = d.step();
    assert_eq!(stop, Stop::Advanced);
    assert_eq!(d.cpu.regs[3] & 0xffff, 0x8000);
    assert_eq!(d.cpu.eflags & 1, 1);
}