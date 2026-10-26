use super::*;
#[test]
fn generated_suite_matches_full_interpreter_traces_and_benchmarks() {
    let dir=std::env::temp_dir().join(format!("pil-native-proof-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    let report=compare_at(&dir,16,2).unwrap();
    assert!(report.contains("byte-for-byte after every instruction across 14 cases"));
    assert!(report.contains("GENERATED verify readonly_store: steps=0 stop=6"));
    assert!(report.contains("GENERATED verify branch_fault: steps=0 stop=4"));
    assert!(report.contains("GENERATED verify code_guard: steps=0 stop=7"));
    assert!(report.contains("GENERATED verify rx_wait: steps=0 stop=2"));
    assert!(report.contains("GENERATED verify tx_blocked: steps=0 stop=3"));
    assert!(report.contains("GENERATED verify readonly_test: steps=1 stop=1"));
    assert!(report.contains("GENERATED verify readonly_neg: steps=0 stop=6"));
    for case in ["alu8","alu16","alu32"]{assert!(report.contains(&format!("GENERATED verify {case}: steps=31 stop=1")),"{report}");}
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
