use super::*;
#[test]
fn counter_runs_to_harness_halt() {
    let mut d=Debugger::new("counter").unwrap();
    assert_eq!(d.run(100).unwrap(),(6,Stop::Halt(0x108)));
    assert_eq!(d.cpu.regs[0],0);assert_eq!(d.cpu.regs[3],1);
}
#[test]
fn breakpoint_stops_before_execution_and_continue_skips_it_once() {
    let mut d=Debugger::new("counter").unwrap();d.command("break 0x102").unwrap();
    assert_eq!(d.run(100).unwrap(),(1,Stop::Breakpoint(0x102)));assert_eq!(d.cpu.regs[0],2);
    assert_eq!(d.run(100).unwrap(),(2,Stop::Breakpoint(0x102)));assert_eq!(d.cpu.regs[0],1);
    assert_eq!(d.run(100).unwrap(),(2,Stop::Breakpoint(0x102)));assert_eq!(d.cpu.regs[0],0);
    assert_eq!(d.run(100).unwrap(),(1,Stop::Halt(0x108)));
}
#[test]
fn explicit_step_ignores_current_breakpoint() {
    let mut d=Debugger::new("counter").unwrap();d.command("break 0x100").unwrap();
    assert_eq!(d.run(100).unwrap(),(0,Stop::Breakpoint(0x100)));
    assert_eq!(d.step(),Stop::Advanced);assert_eq!(d.cpu.eip,0x102);
}
#[test]
fn budget_is_bounded_and_invalid_budget_preserves_stop_state() {
    let mut d=Debugger::new("counter").unwrap();d.command("break 0x100").unwrap();
    d.run(1).unwrap();let before=d.cpu.clone();let marker=d.stopped_at_break;
    assert!(d.run(0).is_err());assert!(d.run(MAX_RUN+1).is_err());
    assert_eq!(d.cpu,before);assert_eq!(d.stopped_at_break,marker);
    assert_eq!(d.run(1).unwrap(),(1,Stop::Budget(0x102)));
}
#[test]
fn echo_waits_for_rx_and_transmits_binary_zero() {
    let mut d=Debugger::new("echo").unwrap();let before=d.cpu.clone();
    assert_eq!(d.step(),Stop::WaitingRx(0x200));assert_eq!(d.cpu,before);
    d.command("serial rx 00").unwrap();
    assert_eq!(d.run(10).unwrap(),(2,Stop::Halt(0x202)));
    assert_eq!(d.serial.take_tx(),vec![0]);assert_eq!(d.cpu.eflags,before.eflags);
}
#[test]
fn tx_backpressure_retries_same_harness_operation() {
    let mut d=Debugger::new("echo").unwrap();d.serial=MockSerial::new(1,1);
    d.serial.try_write(99).unwrap();d.serial.inject_rx(&[65]).unwrap();
    assert_eq!(d.step(),Stop::Advanced);let before=d.cpu.clone();
    assert_eq!(d.step(),Stop::TxBlocked(0x201));assert_eq!(d.cpu,before);
    assert_eq!(d.serial.take_tx(),vec![99]);
    assert_eq!(d.step(),Stop::Advanced);assert_eq!(d.serial.take_tx(),vec![65]);
    assert_eq!(d.step(),Stop::Halt(0x202));assert!(d.serial.take_tx().is_empty());
}
#[test]
fn reset_clears_queues_and_preserves_breakpoints() {
    let mut d=Debugger::new("echo").unwrap();d.command("break 0x201").unwrap();
    d.command("serial rx 41").unwrap();d.run(10).unwrap();
    d.reset();assert_eq!(d.cpu,d.initial);assert_eq!(d.serial.lengths(),(0,0));
    assert!(d.breakpoints.contains(&0x201));assert_eq!(d.stopped_at_break,None);
}
#[test]
fn malformed_commands_and_rx_chunks_do_not_partially_apply() {
    let mut d=Debugger::new("counter").unwrap();let before=d.cpu.clone();
    for command in ["set nope 1","set eax 0x100000000","set eax -1","run 0",
                    "run 10001","break 0x999","serial rx 41 zz","serial rx",
                    "demo missing","step unexpected"] {
        assert!(d.command(command).is_err(),"{command}");
        assert_eq!(d.cpu,before);assert_eq!(d.serial.lengths(),(0,0));
    }
    d.serial=MockSerial::new(1,1);
    assert!(d.command("serial rx 41 42").is_err());assert_eq!(d.serial.lengths(),(0,0));
}
#[test]
fn register_and_flag_inspection_preserves_concrete_undefined_bits() {
    let mut d=Debugger::new("counter").unwrap();d.command("set EAX 0x12345678").unwrap();
    assert!(d.command("regs").unwrap().0.contains("EAX=12345678"));
    d.cpu.eflags|=16;d.cpu.undefined_flags=16;
    assert!(d.command("flags").unwrap().0.contains("AF=1?"));
    d.command("set eflags 0x202").unwrap();assert_eq!(d.cpu.undefined_flags,0);
}
#[test]
fn scripted_repl_separates_console_and_guest_serial_data() {
    let input=b"demo echo\nrun\nserial rx 41\nrun\nserial tx\nserial status\nquit\n";
    let mut output=Vec::new();repl(&input[..],&mut output).unwrap();
    let text=String::from_utf8(output).unwrap();
    assert!(text.contains("RX empty"));assert!(text.contains("TX: 41"));
    assert!(text.contains("RX=0/256"));assert!(text.contains("bye"));
    let mut output=Vec::new();repl(&b""[..],&mut output).unwrap();
    assert!(String::from_utf8(output).unwrap().ends_with("pil> "));
}
#[test]
fn missing_predecoded_instruction_is_not_misreported_as_a_guest_page_fault() {
    let mut d=Debugger::new("counter").unwrap();d.command("set eip 0x999").unwrap();
    let before=d.cpu.clone();assert_eq!(d.step(),Stop::MissingInstruction(0x999));assert_eq!(d.cpu,before);
}
#[test]
fn guest_branch_fault_preserves_cpu_at_source_eip() {
    let mut d=Debugger::new("counter").unwrap();assert_eq!(d.step(),Stop::Advanced);
    if let Action::Cpu{context,..}=&mut d.instructions.get_mut(&0x102).unwrap().action {
        context.code_limit=0;
    } else {panic!("expected JNE");}
    let before=d.cpu.clone();
    assert_eq!(d.step(),Stop::Fault(ExecError::GeneralProtection{eip:0x102,error_code:0}));
    assert_eq!(d.cpu,before);
}
#[test]
fn invalid_demo_selection_retains_program_and_breakpoints() {
    let mut d=Debugger::new("counter").unwrap();d.command("break 0x100").unwrap();
    assert!(d.load_demo("missing").is_err());
    assert_eq!(d.demo,"counter");assert!(d.breakpoints.contains(&0x100));
    d.load_demo("echo").unwrap();assert_eq!(d.cpu.eip,0x200);assert!(d.breakpoints.is_empty());
}

#[test]
fn memory_demo_commits_store_then_loads_value() {
    let mut d=Debugger::new("memory").unwrap();
    assert_eq!(d.run(10).unwrap(),(2,Stop::Halt(0x304)));
    assert_eq!(d.cpu.regs[0],0x12345678);
    assert_eq!(d.command("mem 0x10 4").unwrap().0,"00000010: 78 56 34 12");
}
#[test]
fn debugger_poke_is_atomic_and_reset_restores_ram_permissions() {
    let mut d=Debugger::new("memory").unwrap();d.command("poke 0x10 aa bb cc dd").unwrap();
    d.command("protect 0x12 1").unwrap();let before=d.ram.clone();
    assert!(d.command("poke 0x10 00 00 00 00").is_err());assert_eq!(d.ram,before);
    assert!(d.command("poke 0x10 41 zz").is_err());assert_eq!(d.ram,before);
    assert!(d.command("mem 0xff 2").is_err());assert!(d.command("mem 0 257").is_err());
    d.reset();assert!(d.command("poke 0x10 01 02 03 04").is_ok());
    assert_eq!(d.command("mem 0x10 4").unwrap().0,"00000010: 01 02 03 04");
}
#[test]
fn protected_memory_demo_faults_at_store_without_changing_state() {
    let mut d=Debugger::new("memory").unwrap();d.command("protect 0x10 4").unwrap();
    let cpu=d.cpu.clone();let ram=d.ram.clone();
    assert_eq!(d.step(),Stop::Fault(ExecError::MemoryAccess{eip:0x300,address:0x10,bytes:4,write:true}));
    assert_eq!(d.cpu,cpu);assert_eq!(d.ram,ram);
}
