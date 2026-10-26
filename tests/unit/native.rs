use super::*;
#[test]
fn generated_suite_matches_full_interpreter_traces_and_benchmarks() {
    let dir=std::env::temp_dir().join(format!("pil-native-proof-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    let report=compare_at(&dir,16,2).unwrap();
    assert!(report.contains("byte-for-byte after every instruction across 9 cases"));
    assert!(report.contains("GENERATED verify readonly_store: steps=0 stop=6"));
    assert!(report.contains("GENERATED verify branch_fault: steps=0 stop=4"));
    assert!(report.contains("GENERATED verify code_guard: steps=0 stop=7"));
    assert!(report.contains("GENERATED verify rx_wait: steps=0 stop=2"));
    assert!(report.contains("GENERATED verify tx_blocked: steps=0 stop=3"));
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
