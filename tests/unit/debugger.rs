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

#[test]
fn actual_guest_selftest_runs_through_decoder_and_pil_to_serial() {
    let mut d=Debugger::new("selftest").unwrap();let (_,stop)=d.run(1000).unwrap();
    assert!(matches!(stop,Stop::WaitingRx(_)),"{stop:?}");
    assert!(d.command("check").is_err());d.command("serial rx 41").unwrap();
    let text=d.command("run").unwrap().0;assert!(text.contains("serial output: 41 | A"));
    assert_eq!(d.cpu.regs[2]&0xffff,0x600d);assert!(d.command("check").unwrap().0.starts_with("PASS"));
    assert_eq!(d.command("serial tx").unwrap().0,"TX: 41");assert!(d.command("check").is_ok());
}
#[test]
fn executable_breakpoint_and_reset_restore_program_and_data() {
    let mut d=Debugger::new("selftest").unwrap();d.command("break 0x100").unwrap();
    assert_eq!(d.run(100).unwrap(),(0,Stop::Breakpoint(0x100)));assert_eq!(d.run(1).unwrap(),(1,Stop::Budget(0x103)));
    d.command("poke 0x100 90").unwrap();d.reset();assert_eq!(d.ram.load(0x100,super::super::memory::MemWidth::Byte),Ok(0xb8));
    assert_eq!(d.cpu.eip,0x100);assert!(d.breakpoints.contains(&0x100));
}
#[test]
fn corrupted_guest_check_reaches_failure_marker() {
    let mut d=Debugger::new("selftest").unwrap();d.command("poke 0x101 00 00").unwrap();
    let (_,stop)=d.run(1000).unwrap();assert!(matches!(stop,Stop::Halt(_)));assert_eq!(d.cpu.regs[2]&0xffff,0xdead);assert!(d.command("check").is_err());
}
#[test]
fn binary_decode_error_preserves_cpu_and_ram() {
    let mut d=Debugger::new("selftest").unwrap();d.command("poke 0x100 90").unwrap();let c=d.cpu.clone();let m=d.ram.clone();
    assert!(matches!(d.step(),Stop::Decode{eip:0x100,..}));assert_eq!(d.cpu,c);assert_eq!(d.ram,m);
    assert!(d.load_image(&[],"bad").is_err());assert_eq!(d.cpu,c);assert_eq!(d.ram,m);
}
#[test]
fn monitor_input_and_output_backpressure_retry_without_duplicate_output() {
    let mut d=Debugger::new("counter").unwrap();d.load_image(&[0xe4,0xe9,0xe6,0xe9,0xf4],"io").unwrap();
    d.serial=MockSerial::new(1,1);assert_eq!(d.step(),Stop::WaitingRx(0x100));d.serial.inject_rx(&[0]).unwrap();d.serial.try_write(99).unwrap();
    assert_eq!(d.step(),Stop::Advanced);let before=d.cpu.clone();assert_eq!(d.step(),Stop::TxBlocked(0x102));assert_eq!(d.cpu,before);assert!(d.tx_events.is_empty());
    d.serial.take_tx();let out=d.command("step").unwrap().0;assert!(out.contains("serial output: 00 | \\x00"));assert_eq!(d.tx_events,vec![0]);assert_eq!(d.serial.take_tx(),vec![0]);
}

#[test]
fn hello_guest_outputs_text_without_input() {
    let mut d=Debugger::new("hello").unwrap();
    assert_eq!(d.run(100).unwrap(),(28,Stop::Halt(0x138)));
    assert_eq!(d.serial.take_tx(),b"Hello, world!\n");
    assert_eq!(d.serial.lengths(),(0,0));assert_eq!(d.cpu.eflags,0x202);
    assert!(d.command("check").is_err());
}
#[test]
fn hello_reset_replays_console_output_and_standalone_image() {
    let mut d=Debugger::new("counter").unwrap();let bytes=super::super::guest::hello_bytes().unwrap();
    assert_eq!(bytes.len(),57);d.load_image(&bytes,"hello.bin").unwrap();
    let output=d.command("run").unwrap().0;assert!(output.contains("Hello, world!\\n"));
    d.reset();assert!(d.tx_events.is_empty());assert_eq!(d.serial.lengths(),(0,0));
    d.command("run").unwrap();assert_eq!(d.serial.take_tx(),b"Hello, world!\n");
}

#[test]
fn factorial_guest_computes_all_twelve_results_and_detects_corruption() {
    let mut d=Debugger::new("factorials").unwrap();assert!(d.command("check").is_err());
    assert_eq!(d.run(1000).unwrap(),(336,Stop::Halt(0x146)));
    assert!(d.command("check").unwrap().0.contains("12!=479001600"));
    assert_eq!(d.cpu.regs[0],479001600);assert!(d.tx_events.is_empty());
    d.command("poke 0x510 00").unwrap();assert!(d.command("check").is_err());
    d.reset();assert_eq!(d.run(1000).unwrap().0,336);assert!(d.command("check").is_ok());
}
#[test]
fn counted_loop_iterations_steps_and_flags_match_for_varied_counts() {
    for n in [1,2,17,257] {
        let mut d=Debugger::new("counter").unwrap();d.load_image(&super::super::guest::loop_bytes(n).unwrap(),"loop.bin").unwrap();
        assert_eq!(d.run(1000).unwrap(),(3*n as usize+3,Stop::Halt(0x11a)));
        assert_eq!(d.cpu.regs[0],n);assert_eq!(d.cpu.regs[1],0);assert!(d.command("check").is_ok());
        d.cpu.eflags|=1;assert!(d.command("check").is_err());
    }
}
#[test]
fn timed_run_honours_breakpoints_and_invalid_budget_preserves_continuation() {
    let mut d=Debugger::new("loop").unwrap();d.command("break 0x112").unwrap();
    let output=d.command("time 10000").unwrap().0;
    assert!(output.contains("3 completed steps; breakpoint"));let c=d.cpu.clone();let marker=d.stopped_at_break;
    for bad in ["time 0","time 1000001","time 0x10000000000000000"] {assert!(d.command(bad).is_err());assert_eq!(d.cpu,c);assert_eq!(d.stopped_at_break,marker);}
    let output=d.command("time 1").unwrap().0;assert!(output.contains("1 completed steps"));assert!(output.contains("elapsed="));assert_eq!(d.cpu.eip,0x115);
}
#[test]
fn timed_run_zero_steps_has_no_division_by_zero_or_state_change() {
    let mut d=Debugger::new("factorials").unwrap();d.run(1000).unwrap();let c=d.cpu.clone();let ram=d.ram.clone();
    let output=d.command("time 1").unwrap().0;assert!(output.contains("0 completed steps"));assert!(output.contains("rate unavailable"));assert_eq!(d.cpu,c);assert_eq!(d.ram,ram);
}
#[test]
fn large_run_batches_keep_exact_total_budget() {
    let mut d=Debugger::new("loop").unwrap();let (steps,stop)=d.run_large(10001).unwrap();
    assert_eq!(steps,10001);assert!(matches!(stop,Stop::Budget(_)));
    let (_,stop)=d.run_large(4).unwrap();assert!(matches!(stop,Stop::Budget(_)));
}
#[test]
fn benchmark_is_repeatable_and_does_not_replace_debugger_state() {
    let mut d=Debugger::new("echo").unwrap();d.serial.inject_rx(&[65]).unwrap();d.command("break 0x201").unwrap();let c=d.cpu.clone();let ram=d.ram.clone();let serial=d.serial.clone();
    let output=d.command("bench 16 2").unwrap().0;
    assert!(output.contains("51 completed instructions/sample"));assert!(output.contains("2 measured samples, 1 excluded warmup"));assert!(output.contains("median="));
    assert_eq!(d.cpu,c);assert_eq!(d.ram,ram);assert_eq!(d.serial,serial);assert!(d.breakpoints.contains(&0x201));
}
#[test]
fn numeric_stress_verifies_every_fresh_run_and_rejects_invalid_counts() {
    assert!(factorial_stress(2).unwrap().contains("24 results verified; 672 completed instructions"));
    let mut d=Debugger::new("loop").unwrap();let c=d.cpu.clone();
    for command in ["bench 0","bench 100001","bench 16 0","bench 16 10","bench 4294967296","stress 0","stress 1001","stress 4294967296"] {
        assert!(d.command(command).is_err(),"{command}");assert_eq!(d.cpu,c);
    }
}

#[test]
fn cached_and_uncached_execution_match_at_every_instruction() {
    let images=vec![super::super::guest::selftest_bytes().unwrap(),super::super::guest::hello_bytes().unwrap(),super::super::guest::factorial_bytes().unwrap(),super::super::guest::loop_bytes(257).unwrap()];
    for bytes in images {
        let mut cached=Debugger::new("counter").unwrap();let mut plain=Debugger::new("counter").unwrap();
        cached.load_image(&bytes,"comparison").unwrap();plain.load_image(&bytes,"comparison").unwrap();plain.cache.set_enabled(false);
        cached.serial.inject_rx(&[65]).unwrap();plain.serial.inject_rx(&[65]).unwrap();let mut halted=false;
        for _ in 0..1000 {
            let a=cached.step();let b=plain.step();assert_eq!(a,b);assert_eq!(cached.cpu,plain.cpu);assert_eq!(cached.ram,plain.ram);assert_eq!(cached.serial,plain.serial);assert_eq!(cached.tx_events,plain.tx_events);
            if matches!(a,Stop::Halt(_)){halted=true;break;}assert_eq!(a,Stop::Advanced);
        }
        assert!(halted);assert!(cached.cache.stats().misses>0);assert_eq!(plain.cache.stats(),super::super::cache::Stats::default());
    }
}
#[test]
fn guest_store_invalidates_its_previously_cached_instruction() {
    // First MOV AL,1 is cached. A guest store replaces its immediate with 2,
    // then a taken JE returns to execute that instruction again.
    let bytes=[0xb0,1,0x66,0xbb,1,0,0,0,0xb0,2,0x88,0x07,0x38,0xc0,0x74,0xf0];
    let mut d=Debugger::new("counter").unwrap();d.load_image(&bytes,"self-modifying").unwrap();
    assert_eq!(d.step(),Stop::Advanced);assert_eq!(d.cpu.regs[0]&255,1);
    assert!(matches!(d.run(5).unwrap().1,Stop::Budget(0x100)));
    assert_eq!(d.step(),Stop::Advanced);assert_eq!(d.cpu.regs[0]&255,2);assert_eq!(d.cache.stats().invalidations,1);
}
#[test]
fn cached_store_recomputes_addresses_and_rechecks_ram_protection() {
    let mut d=Debugger::new("counter").unwrap();d.load_image(&[0x89,0x07,0xf4],"store").unwrap();d.cpu.regs[0]=0x1234;d.cpu.regs[3]=0x400;
    assert_eq!(d.step(),Stop::Advanced);assert_eq!(d.ram.load(0x500,super::super::memory::MemWidth::Word),Ok(0x1234));
    d.cpu.eip=0x100;d.cpu.regs[3]=0x500;d.ram.protect(0x600,2).unwrap();let cpu=d.cpu.clone();let ram=d.ram.clone();
    assert_eq!(d.step(),Stop::Fault(ExecError::MemoryAccess{eip:0x100,address:0x600,bytes:2,write:true}));
    assert_eq!(d.cpu,cpu);assert_eq!(d.ram,ram);assert_eq!(d.cache.stats().hits,1);
}
#[test]
fn cache_controls_and_reset_load_keep_the_requested_mode() {
    let mut d=Debugger::new("loop").unwrap();d.run(20).unwrap();assert!(d.cache.stats().hits>0);
    let stats=d.cache.stats();assert!(d.command("cache nonsense").is_err());assert_eq!(d.cache.stats(),stats);
    d.command("reset").unwrap();assert_eq!(d.cache.stats(),super::super::cache::Stats::default());assert!(d.cache.enabled());
    d.command("cache off").unwrap();d.run(20).unwrap();assert_eq!(d.cache.stats(),super::super::cache::Stats::default());
    d.command("demo hello").unwrap();assert!(!d.cache.enabled());d.command("cache on").unwrap();d.step();
    assert!(d.command("cache stats").unwrap().0.contains("misses=1"));d.command("cache clear").unwrap();assert!(d.command("cache").unwrap().0.contains("entries=0/256"));
}
#[test]
fn patched_cached_opcode_reports_error_without_cpu_or_ram_mutation() {
    let mut d=Debugger::new("hello").unwrap();d.step();d.cpu.eip=0x100;d.command("poke 0x100 90").unwrap();let cpu=d.cpu.clone();let ram=d.ram.clone();
    assert!(matches!(d.step(),Stop::Decode{eip:0x100,..}));assert_eq!(d.cpu,cpu);assert_eq!(d.ram,ram);assert_eq!(d.cache.stats().invalidations,1);
}
#[test]
fn benchmark_labels_and_counts_both_interpreter_modes() {
    let cached=benchmark_mode(16,2,true).unwrap();assert!(cached.contains("cache=on (cold per sample); last_sample_hits=45 misses=7 invalidations=0"));
    let plain=benchmark_mode(16,2,false).unwrap();assert!(plain.contains("cache=off (cold per sample); last_sample_hits=0 misses=0 invalidations=0"));
    let mut d=Debugger::new("hello").unwrap();d.command("cache off").unwrap();assert!(d.command("bench 2 1").unwrap().0.contains("cache=off"));
}
#[test]
fn full_65536_iteration_loop_passes_with_cache_reuse_and_batched_runs() {
    let mut d=Debugger::new("loop").unwrap();assert_eq!(d.run_large(250000).unwrap(),(196611,Stop::Halt(0x11a)));assert!(d.command("check").is_ok());
    assert_eq!(d.cache.stats(),super::super::cache::Stats{hits:196605,misses:7,invalidations:0,evictions:0});
}
