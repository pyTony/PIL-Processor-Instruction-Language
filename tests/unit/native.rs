use super::*;
#[test]
fn generated_suite_matches_full_interpreter_traces_and_benchmarks() {
    let dir=std::env::temp_dir().join(format!("pil-native-proof-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    let report=compare_at(&dir,16,2).unwrap();
    assert!(report.contains("byte-for-byte after every execution step across 240 cases"));
    assert!(report.contains("GENERATED verify readonly_store: steps=0 stop=6"));
    assert!(report.contains("GENERATED verify branch_fault: steps=0 stop=4"));
    assert!(report.contains("GENERATED verify code_guard: steps=0 stop=7"));
    assert!(report.contains("GENERATED verify rx_wait: steps=0 stop=2"));
    assert!(report.contains("GENERATED verify tx_blocked: steps=0 stop=3"));
    assert!(report.contains("GENERATED verify readonly_test: steps=1 stop=1"));
    assert!(report.contains("GENERATED verify readonly_neg: steps=0 stop=6"));
    for case in ["alu8","alu16","alu32"]{assert!(report.contains(&format!("GENERATED verify {case}: steps=31 stop=1")),"{report}");}
    for case in ["imul16","imul32"]{assert!(report.contains(&format!("GENERATED verify {case}: steps=7 stop=1")),"{report}");}
    for case in ["imul_fault16","imul_fault32"]{assert!(report.contains(&format!("GENERATED verify {case}: steps=0 stop=4")),"{report}");}
    for case in ["shift8","shift16","shift32"]{assert!(report.contains(&format!("GENERATED verify {case}: steps=39 stop=1")),"{report}");}
    for case in ["shift_readonly_zero","shift_readonly_one"]{assert!(report.contains(&format!("GENERATED verify {case}: steps=0 stop=6")),"{report}");}
    assert!(report.contains("GENERATED verify carry_controls: steps=9 stop=1"));
    assert!(report.contains("GENERATED verify direction_controls: steps=5 stop=1"));
    assert!(report.contains("GENERATED verify byte_flags: steps=7 stop=1"));
    assert!(report.contains("GENERATED verify mandelbrot2:"));
    assert!(report.contains("GENERATED verify near_branch_fault: steps=0 stop=4"));
    assert!(report.contains("GENERATED verify jump_fault: steps=0 stop=4"));
    for (name,steps,stop) in [("extend16",4,1),("extend32",6,1),("extend_memory16",2,1),("extend_memory32",4,1),("extend_fault_byte",0,6),("extend_fault_word",0,4),("lea16",3,1),("lea32",2,1)]{
        assert!(report.contains(&format!("GENERATED verify {name}: steps={steps} stop={stop}")),"{report}");
    }
    for(name,done,stop)in [("extensions_demo",15,1),("same_width_readonly",4,1),("same_width_gp",0,4),("same_width_ss",0,5),("same_width_ram",0,6)]{
        assert!(report.contains(&format!("GENERATED verify {name}: steps={done} stop={stop}")),"{report}");
    }
    for (name,steps,stop) in [("cs_real_write",1,1),("cs_protected_write",0,4),("cs_readonly",0,6)]{assert!(report.contains(&format!("GENERATED verify {name}: steps={steps} stop={stop}")),"{report}");}
    for name in ["retf16_ss16","retfi16_ss16","retf16_ss32","retfi16_ss32","retf32_ss16","retfi32_ss16","retf32_ss32","retfi32_ss32","retf_alias_branch","retf_alias_cs_store"]{assert!(report.contains(&format!("GENERATED verify {name}: steps=2 stop=1")),"{report}");}
    for (name,steps,stop) in [("retf_target_fault",0,4),("retf_frame_fault",0,5),("retf_mode_reject",0,9),("retf_outside_window",1,7)]{assert!(report.contains(&format!("GENERATED verify {name}: steps={steps} stop={stop}")),"{report}");}
    for name in ["popmem16_ss16","popmem16_ss32","popmem32_ss16","popmem32_ss32","popmem_fs16","popmem_fs32"]{assert!(report.contains(&format!("GENERATED verify {name}: steps=1 stop=1")),"{report}");}
    for name in ["smsw_zero","smsw_supplied","smsw_protected"]{assert!(report.contains(&format!("GENERATED verify {name}: steps=2 stop=1")),"{report}");}
    for (name,steps,stop) in [("smsw_memory",3,1),("popmem_readonly",0,6),("popmem_ss_fault",0,5),("smsw_readonly",0,6),("smsw_boundary",0,4)]{assert!(report.contains(&format!("GENERATED verify {name}: steps={steps} stop={stop}")),"{report}");}
    for name in ["pushmem16_ss16","pushmem16_ss32","pushmem32_ss16","pushmem32_ss32","pushmem_fs16","pushmem_fs32","pushmem_readonly_source","pushmem_overlap"]{assert!(report.contains(&format!("GENERATED verify {name}: steps=1 stop=1")),"{report}");}
    for(name,stop)in [("pushmem_readonly_stack",6),("pushmem_ss_fault",5),("pushmem_source_gp",4)]{assert!(report.contains(&format!("GENERATED verify {name}: steps=0 stop={stop}")),"{report}");}
    for name in ["far_imm16_ss16","far_imm16_ss32","far_imm32_ss16","far_imm32_ss32","far_mem16_ss16","far_mem16_ss32","far_mem32_ss16","far_mem32_ss32","far_esp16","far_esp32"]{assert!(report.contains(&format!("GENERATED verify {name}: steps=3 stop=1")),"{report}");}
    for(name,done,stop)in [("far_second_store_fault",0,6),("far_pointer_boundary",0,4),("far_target_fault",0,4),("far_mode_reject",0,9),("far_outside_window",1,7)]{assert!(report.contains(&format!("GENERATED verify {name}: steps={done} stop={stop}")),"{report}");}
    for(name,done,stop)in [("div8",4,1),("div16",3,1),("div32",3,1),("div_zero8",0,9),("div_overflow8",0,9),("div_overflow16",0,9),("div_overflow32",0,9),("div_gp_fault",0,4),("mul8",3,1),("mul16",3,1),("mul32",2,1),("mul_gp_fault",0,4),("loop16",8,1),("loop_ecx",3,1),("loop_target_fault",0,4),("jmp_indirect",3,1),("jmp_indirect_limit_fault",0,4),("lds16",1,1),("lds32",1,1),("enter16",1,1),("enter32",1,1),("enter_readonly_stack",0,6),("jmpfar_imm16",2,1),("jmpfar_mem16",2,1),("jmpfar_mem32",2,1),("jmpfar_pointer_boundary",0,4),("jmpfar_mode_reject",0,9),("imul1",4,1),("imul1_gp_fault",0,4),("xlat",3,1),("bit_test",6,1),("bit_test_readonly",0,6),("double_shift",4,1),("double_shift_reject",0,9),("rcl_rcr",6,1),("rcl_memory_readonly",0,6),("daa_das",5,1),("pusha16",4,1),("pushad32",3,1),("pusha_readonly_stack",0,6),("popa_high_stack_fault",0,6)]{assert!(report.contains(&format!("GENERATED verify {name}: steps={done} stop={stop}")),"{report}");}
    assert!(report.contains("Generated Rust median="));
    let expected=fs::read(dir.join("interpreter.trace")).unwrap();let actual=fs::read(dir.join("generated.trace")).unwrap();assert_eq!(expected,actual);
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn trace_comparison_rejects_modified_and_truncated_state() {
    let mut bytes=Vec::new();super::super::trace::record(&mut bytes,0,[0;5],&[0;8],0x202,0,0x100,&[1,2,3],&[65],&[66]);
    let mut changed=bytes.clone();*changed.last_mut().unwrap()=67;assert!(verify_traces(&bytes,&changed).is_err());
    changed=bytes.clone();changed.pop();assert!(verify_traces(&bytes,&changed).is_err());assert!(verify_traces(&bytes,&bytes).is_ok());
}
#[test]
fn invalid_comparison_limits_are_rejected_before_creating_artifacts() {
    for (n,s) in [(0,1),(100001,1),(16,0),(16,10)] {assert!(emit_suite(n,s).is_err());assert!(compare(n,s).is_err());}
}

#[test]
fn native_cpu_baseline_is_checked_or_explicitly_unavailable() {
    let report=cpu_benchmark(100,2).unwrap();
    #[cfg(target_arch="x86_64")] assert!(report.contains("PC_NATIVE x86_64 ADD/SUB/JNZ"));
    #[cfg(not(target_arch="x86_64"))] assert!(report.contains("PC_NATIVE unavailable"));
    assert!(cpu_benchmark(0,2).is_err());
}

#[test]
fn verifier_names_state_fields_and_first_divergent_instruction() {
    fn sample()->Vec<u8>{let mut out=Vec::new();for ip in [0x102,0x104]{super::super::trace::record(&mut out,3,[0;5],&[0;8],0x202,0,ip,&[1,2,3],&[65],&[66]);}out}
    let original=sample();let size=original.len()/2;
    for (offset,label) in [(4,"stop.kind"),(24,"EAX"),(56,"EFLAGS"),(60,"undefined_flags"),(64,"EIP"),(73,"RAM[0x1]"),(79,"RX[0x0]"),(84,"TX[0x0]")] {
        let mut altered=original.clone();altered[size+offset]^=1;
        let error=verify_traces(&original,&altered).unwrap_err();
        assert!(error.contains("case 3 (loop17), step 2, record 2, instruction EIP=00000102"),"{error}");
        assert!(error.contains(label),"expected {label}: {error}");
    }
}
#[test]
fn verifier_rejects_malformed_equal_traces_and_extra_records() {
    let mut bytes=Vec::new();super::super::trace::record(&mut bytes,0,[0;5],&[0;8],0x202,0,0x100,&[1,2,3],&[65],&[66]);
    for length in 1..bytes.len(){assert!(verify_traces(&bytes[..length],&bytes[..length]).is_err(),"length {length}");}
    assert!(verify_traces(&[],&[]).is_err());
    let mut bad=bytes.clone();bad[68..72].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(verify_traces(&bad,&bad).unwrap_err().contains("truncated trace"));
    let mut extra=bytes.clone();extra.extend_from_slice(&bytes);
    assert!(verify_traces(&bytes,&extra).unwrap_err().contains("interpreter record missing"));
    assert!(verify_traces(&extra,&bytes).unwrap_err().contains("generated record missing"));
}

#[test]
fn full_image_backend_compares_pixels_and_serial_boundary_state(){
    let dir=std::env::temp_dir().join(format!("pil-mandel-proof-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    let report=compare_mandelbrot_at(&dir,32,8).unwrap();
    assert!(report.contains("1024 pixels match interpreter and independent Q12 reference"),"{report}");
    assert!(report.contains("4 serial-drain/halt checkpoints"),"{report}");
    assert!(report.contains("3 TX backpressure retries in each engine"));
    assert_eq!(fs::read(dir.join("generated.checkpoints")).unwrap(),fs::read(dir.join("interpreter.checkpoints")).unwrap());
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn checkpoint_diagnostics_name_mandelbrot_and_changed_state(){
    let mut bytes=Vec::new();super::super::trace::record(&mut bytes,0,[3,0x120,0,0,0],&[0;8],0x202,0,0x120,&[1,2],&[],&[65]);
    let mut changed=bytes.clone();changed[24]^=1;
    let error=verify_named_traces(&bytes,&changed,&["mandelbrot"],"checkpoint").unwrap_err();
    assert!(error.contains("(mandelbrot), checkpoint 1"));assert!(error.contains("EAX"));
    assert!(emit_mandelbrot(3,32).is_err());assert!(emit_mandelbrot(256,65).is_err());
}

