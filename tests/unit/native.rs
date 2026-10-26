use super::*;
#[test]
fn generated_suite_matches_full_interpreter_traces_and_benchmarks() {
    let dir=std::env::temp_dir().join(format!("pil-native-proof-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    let report=compare_at(&dir,16,2).unwrap();
    assert!(report.contains("byte-for-byte after every instruction across 39 cases"));
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
    assert!(report.contains("GENERATED verify mandelbrot2:"));
    assert!(report.contains("GENERATED verify near_branch_fault: steps=0 stop=4"));
    assert!(report.contains("GENERATED verify jump_fault: steps=0 stop=4"));
    for (name,steps,stop) in [("extend16",4,1),("extend32",6,1),("extend_memory16",2,1),("extend_memory32",4,1),("extend_fault_byte",0,6),("extend_fault_word",0,4),("lea16",3,1),("lea32",2,1)]{
        assert!(report.contains(&format!("GENERATED verify {name}: steps={steps} stop={stop}")),"{report}");
    }
    for(name,done,stop)in [("extensions_demo",15,1),("same_width_readonly",4,1),("same_width_gp",0,4),("same_width_ss",0,5),("same_width_ram",0,6)]{
        assert!(report.contains(&format!("GENERATED verify {name}: steps={done} stop={stop}")),"{report}");
    }
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
