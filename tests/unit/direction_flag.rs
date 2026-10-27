//! DF changes are independent of arithmetic flags and host ABI state.
use super::*;
const DF:u32=0x400;
fn initial(f:u32,u:u32)->Cpu{Cpu{regs:[0x12345678,0x89abcdef,2,3,4,5,6,7],eflags:f,undefined_flags:u,eip:0x100}}
#[test]
fn direction_controls_preserve_all_other_state_and_define_df(){
    for (op,set) in [("cld",0),("std",DF)]{for w in [8,16,32]{
        let p=compile(&description(op).unwrap(),w).unwrap();
        let v=View{register:0,low:0,width:w};
        for low in 0..4096{for u in [0,DF,ARITH_FLAGS,u32::MAX]{
            let mut c=initial(0xa5a50000|low,u);let mut want=c.clone();
            want.eflags=(want.eflags&!DF)|set;want.undefined_flags&=!DF;
            execute(&p,&mut c,v,v).unwrap();assert_eq!(c,want);
            execute(&p,&mut c,v,v).unwrap();assert_eq!(c,want);
        }}
    }}
}
#[test]
fn df_classification_keeps_arithmetic_contract_and_duplicate_checks(){
    assert!(compile("const x 1 1\nflag DF x\nretire",16).is_err());
    assert!(compile("const x 1 1\nflag DF x\npreserve_flag DF\npreserve_flags all\nretire",16).is_err());
    assert!(compile("preserve_flag DF\npreserve_flags all\nretire",16).is_ok());
}
#[test]
fn generated_direction_controls_have_independent_full_state_expectations(){
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-direction-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&dir).unwrap();
    let mut source=String::from("#![allow(dead_code,unused_mut,unused_variables)]\n");
    for op in ["cld","std"]{source.push_str(&format!("mod dir_{op} {{ {} }}\n",emit_rust(&compile(&description(op).unwrap(),16).unwrap())));}
    source.push_str(r#"
fn main(){for low in 0..4096u32{for initial_u in [0,0x400,0x8d5,u32::MAX]{
 for set in [false,true]{
  let original=[0x12345678,0x89abcdef,2,3,4,5,6,7];let mut regs=original;let f=0xa5a50000|low;let mut flags=f;let mut u=initial_u;let mut ip=0x100;
  if set{dir_std::execute_with_flow(&mut regs,&mut flags,&mut u,&mut ip,(0,0),(0,0),dir_std::FlowContext{next_eip:0x101,displacement:0,displacement_bits:8,code_limit:0x1ff}).unwrap();}
  else{dir_cld::execute_with_flow(&mut regs,&mut flags,&mut u,&mut ip,(0,0),(0,0),dir_cld::FlowContext{next_eip:0x101,displacement:0,displacement_bits:8,code_limit:0x1ff}).unwrap();}
  assert_eq!(regs,original);assert_eq!((flags,u,ip),((f&!0x400)|if set{0x400}else{0},initial_u&!0x400,0x101));
 }
}}}
"#);
    let path=dir.join("proof.rs");fs::write(&path,source).unwrap();let bin=dir.join(if cfg!(windows){"proof.exe"}else{"proof"});
    let out=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).args(["--edition=2021","-O"]).arg(path).arg("-o").arg(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));assert!(Command::new(bin).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
}
#[test]
#[cfg(target_arch="x86_64")]
fn actual_cpu_direction_controls_restore_df_before_returning_to_rust(){
    // Every seed/target/snapshot/CLD sequence is one asm block. Never expose
    // DF=1 to Rust, libc or an unwinder. Do not claim preserves_flags/nostack.
    macro_rules! hardware{($seed:literal,$op:literal)=>{{
        let before:u64;let after:u64;
        unsafe{std::arch::asm!($seed,"pushfq","pop {before}",$op,"pushfq","pop {after}","cld",
            before=out(reg) before,after=out(reg) after);}
        (before as u32,after as u32)
    }}}
    for (op,set) in [("cld",0),("std",DF)]{for seed in [false,true]{
        let (before,after)=match (seed,set!=0){
            (false,false)=>hardware!("cld","cld"),(false,true)=>hardware!("cld","std"),
            (true,false)=>hardware!("std","cld"),(true,true)=>hardware!("std","std"),
        };
        assert_eq!(before&DF,if seed{DF}else{0});assert_eq!(after,(before&!DF)|set);
        let p=compile(&description(op).unwrap(),16).unwrap();let v=View{register:0,low:0,width:16};
        let mut c=initial(before,DF);execute(&p,&mut c,v,v).unwrap();assert_eq!(c.eflags,after);assert_eq!(c.undefined_flags,0);
    }}
}
