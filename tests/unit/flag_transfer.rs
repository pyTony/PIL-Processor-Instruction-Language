//! Exhaustive byte layout, untouched state and generated execution contract.
use super::*;
const AH:View=View{register:0,low:8,width:8};
fn initial(f:u32,u:u32)->Cpu{Cpu{regs:[0x12345678,0x89abcdef,2,3,4,5,6,7],eflags:f,undefined_flags:u,eip:0x100}}
#[test]
fn lahf_packs_status_and_fixed_bits_preserving_every_flag_marker(){
    let p=compile(&description("lahf").unwrap(),8).unwrap();
    for low in 0..4096{for u in [0,1,ARITH_FLAGS,u32::MAX]{
        let f=0xa5a50000|low;let mut c=initial(f,u);let mut want=c.clone();
        want.regs[0]=(want.regs[0]&!0xff00)|(((f&0xd5)|2)<<8);
        execute(&p,&mut c,AH,AH).unwrap();assert_eq!(c,want);
    }}
}
#[test]
fn sahf_reads_all_ah_bytes_ignores_reserved_bits_and_preserves_of(){
    let p=compile(&description("sahf").unwrap(),8).unwrap();
    for byte in 0..256{for f in [0,0x202,0xad7,u32::MAX]{for u in [0,1,ARITH_FLAGS,u32::MAX]{
        let mut c=initial(f,u);c.regs[0]=(c.regs[0]&!0xff00)|(byte<<8);let mut want=c.clone();
        want.eflags=(f&!0xd5)|(byte&0xd5);want.undefined_flags=u&!0xd5;
        execute(&p,&mut c,AH,AH).unwrap();assert_eq!(c,want);
    }}}
    for op in ["lahf","sahf"]{for w in [16,32]{assert!(compile(&description(op).unwrap(),w).is_err());}}
}
#[test]
fn generated_flag_transfers_match_all_state_including_ah_alias(){
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-flag-transfer-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&dir).unwrap();
    let mut source=String::from("#![allow(dead_code,unused_mut,unused_variables)]\n");
    for op in ["lahf","sahf"]{source.push_str(&format!("mod {op} {{ {} }}\n",emit_rust(&compile(&description(op).unwrap(),8).unwrap())));}
    source.push_str(r#"
fn main(){
 for low in 0..4096u32{for initial_u in [0,1,0x8d5,u32::MAX]{
  let f=0xa5a50000|low;let original=[0x12345678,0x89abcdef,2,3,4,5,6,7];let mut regs=original;let mut flags=f;let mut u=initial_u;let mut ip=0x100;
  lahf::execute_with_flow(&mut regs,&mut flags,&mut u,&mut ip,(0,8),(0,8),lahf::FlowContext{next_eip:0x101,displacement:0,displacement_bits:8,code_limit:0x1ff}).unwrap();
  let mut want=original;want[0]=(want[0]&!0xff00)|(((f&0xd5)|2)<<8);assert_eq!(regs,want);assert_eq!((flags,u,ip),(f,initial_u,0x101));
 }}
 for byte in 0..256u32{for f in [0,0x202,0xad7,u32::MAX]{for initial_u in [0,1,0x8d5,u32::MAX]{
  let original=[0x12340078|(byte<<8),0x89abcdef,2,3,4,5,6,7];let mut regs=original;let mut flags=f;let mut u=initial_u;let mut ip=0x100;
  sahf::execute_with_flow(&mut regs,&mut flags,&mut u,&mut ip,(0,8),(0,8),sahf::FlowContext{next_eip:0x101,displacement:0,displacement_bits:8,code_limit:0x1ff}).unwrap();
  assert_eq!(regs,original);assert_eq!((flags,u,ip),((f&!0xd5)|(byte&0xd5),initial_u&!0xd5,0x101));
 }}}
}
"#);
    let path=dir.join("proof.rs");fs::write(&path,source).unwrap();let bin=dir.join(if cfg!(windows){"proof.exe"}else{"proof"});
    let out=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).args(["--edition=2021","-O"]).arg(path).arg("-o").arg(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));assert!(Command::new(bin).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
}
