use std::{env,fs,path::PathBuf};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(pil_shadow_backend)");
    println!("cargo:rerun-if-env-changed=PIL_SHADOW_SOURCE");
    if let Some(source)=env::var_os("PIL_SHADOW_SOURCE") {
        let source=PathBuf::from(source);
        println!("cargo:rerun-if-changed={}",source.display());
        let dest=PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("pil_shadow.rs");
        fs::copy(&source,&dest).unwrap_or_else(|e|panic!("cannot copy shadow backend {}: {e}",source.display()));
        println!("cargo:rustc-cfg=pil_shadow_backend");
    }
}
