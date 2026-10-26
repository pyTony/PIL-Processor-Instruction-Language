use super::*;

fn views(w:u8)->(View,View) {(View{register:0,low:0,width:w},View{register:1,low:0,width:w})}
fn context()->FlowContext {FlowContext{next_eip:0x402,displacement:0,displacement_bits:32,code_limit:u32::MAX}}
fn program(name:&str,w:u8)->Program {compile(&instantiate(description(name).unwrap(),w),w).unwrap()}
fn cpu()->Cpu {let mut c=Cpu::default();c.eip=0x400;c.eflags=0x203;c.undefined_flags=16;c}

#[test]
fn memory_load_store_widths_and_register_views() {
    for w in [8,16,32] {
        let (dst,src)=views(w);let mut c=cpu();c.regs[0]=0xabcd_ff99;c.regs[1]=0x12345678;
        let mut ram=Ram::new(8);let old_flags=c.eflags;
        execute_memory(&program("store_ram",w),&mut c,dst,src,context(),&mut ram,1).unwrap();
        execute_memory(&program("load_ram",w),&mut c,dst,src,context(),&mut ram,1).unwrap();
        let m=mask(w) as u32;
        assert_eq!(c.regs[0],(0xabcd_ff99&!m)|(0x12345678&m));
        assert_eq!(c.eflags,old_flags);assert_eq!(c.undefined_flags,16);assert_eq!(c.eip,0x402);
    }
}
#[test]
fn failed_load_preserves_all_cpu_state() {
    let (dst,src)=views(32);let mut c=cpu();let before=c.clone();let mut ram=Ram::new(8);
    assert_eq!(execute_memory(&program("load_ram",32),&mut c,dst,src,context(),&mut ram,6),
        Err(ExecError::MemoryAccess{eip:0x400,address:6,bytes:4,write:false}));
    assert_eq!(c,before);
}
#[test]
fn failed_store_preserves_cpu_and_entire_ram() {
    let (dst,src)=views(32);let mut c=cpu();c.regs[1]=u32::MAX;let before=c.clone();let mut ram=Ram::new(8);let old=ram.clone();
    assert_eq!(execute_memory(&program("store_ram",32),&mut c,dst,src,context(),&mut ram,6),
        Err(ExecError::MemoryAccess{eip:0x400,address:6,bytes:4,write:true}));
    assert_eq!(c,before);assert_eq!(ram,old);
}
#[test]
fn readonly_rmw_does_not_publish_flags_or_data() {
    let (dst,src)=views(32);let mut c=cpu();c.regs[1]=1;let before=c.clone();
    let mut ram=Ram::new(8);ram.store(0,MemWidth::Dword,u32::MAX).unwrap();ram.protect(2,1).unwrap();let old=ram.clone();
    assert!(execute_memory(&program("add_ram",32),&mut c,dst,src,context(),&mut ram,0).is_err());
    assert_eq!(c,before);assert_eq!(ram,old);
}
#[test]
fn later_branch_fault_cancels_staged_store_and_register_effects() {
    let text="read_address a\nconst v 32 42\nstore a v\nwrite dst v\nconst f 1 0\nflag CF f\nconst t 32 500\nconst take 1 1\nbranch_if t take\npreserve_flags remaining\nretire";
    let p=compile(text,32).unwrap();let (dst,src)=views(32);let mut c=cpu();let before=c.clone();
    let mut ram=Ram::new(8);let old=ram.clone();
    assert_eq!(execute_memory(&p,&mut c,dst,src,FlowContext{code_limit:499,..context()},&mut ram,0),
        Err(ExecError::GeneralProtection{eip:0x400,error_code:0}));
    assert_eq!(c,before);assert_eq!(ram,old);
}
#[test]
fn rmw_carry_flags_commit_with_ram() {
    for w in [8,16,32] {
        let (dst,src)=views(w);let mut c=cpu();c.regs[1]=1;
        let mut ram=Ram::new(8);let mw=MemWidth::from_bits(w).unwrap();ram.store(1,mw,mask(w) as u32).unwrap();
        execute_memory(&program("add_ram",w),&mut c,dst,src,context(),&mut ram,1).unwrap();
        assert_eq!(ram.load(1,mw).unwrap(),0);assert_eq!(c.eflags,0x257);assert_eq!(c.undefined_flags,0);
    }
}
#[test]
fn legacy_apis_reject_missing_memory_context() {
    let p=program("load_ram",32);let (dst,src)=views(32);let mut c=cpu();let before=c.clone();
    assert!(execute(&p,&mut c,dst,src).is_err());
    assert_eq!(execute_flow(&p,&mut c,dst,src,context()),Err(ExecError::InvalidContext));assert_eq!(c,before);
}
#[test]
fn unsupported_memory_effect_shapes_are_rejected() {
    for text in [
        "const a 16 0\nload v a 8\npreserve_flags all\nretire",
        "const a 32 0\nload v a 64\npreserve_flags all\nretire",
        "const a 32 0\nconst v 8 1\nstore a v\nstore a v\npreserve_flags all\nretire",
        "const a 32 0\nconst v 8 1\nstore a v\nload x a 8\npreserve_flags all\nretire",
        "const a 32 0\nconst v 1 1\nstore a v\npreserve_flags all\nretire",
    ] {assert!(compile(text,32).is_err(),"{text}");}
}

#[test]
fn generated_ram_semantics_match_interpreter_including_faults() {
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-memory-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir(&dir).unwrap();let mut source=String::new();let mut dispatch=String::new();let mut cases=Vec::new();
    let mut variants=Vec::new();
    for op in ["load_ram","store_ram","add_ram"] {for w in [8,16,32] {variants.push((op,w));}}
    variants.push(("store_then_fault",32));
    for (id,(op,w)) in variants.into_iter().enumerate() {
        let p=if op=="store_then_fault" {
            compile(&instantiate(STORE_RAM.replace("preserve_flags all",
                "const t 32 1280\nconst c 1 1\nbranch_if t c\npreserve_flags all"),32),32).unwrap()
        } else {program(op,w)};
        let (dst,src)=views(w);
        source.push_str(&format!("mod p{id} {{ {} }}\n",emit_rust(&p)));
        dispatch.push_str(&format!(r#"
{id} => {{
 let mut ram=p{id}::Ram::new(8);ram.write_bytes(0,&[255;8]).unwrap();
 if protected {{ram.protect(4,1).unwrap();}}
 let context=p{id}::FlowContext{{next_eip:0x402,displacement:0,displacement_bits:32,code_limit:0x4ff}};
 let r=p{id}::execute_with_memory(&mut regs,&mut flags,&mut undefined,&mut ip,(0,0),(1,0),context,&mut ram,address);
 output.copy_from_slice(ram.bytes());
 match r {{
  Ok(())=>(0,0,0,0,0),
  Err(p{id}::ExecError::MemoryAccess{{eip,address,bytes,write}})=>(1,eip,address,bytes as u32,u32::from(write)),
  Err(p{id}::ExecError::GeneralProtection{{eip,error_code}})=>(2,eip,error_code as u32,0,0),
  Err(e)=>panic!("unexpected context error: {{e:?}}"),
 }}
}},
"#));
        for address in [0,1,4,5,7,8,u32::MAX] {for input in [0,1,0x7fffffff,u32::MAX] {for protected in [false,true] {
            let mut c=cpu();c.regs[0]=0xabcd_ff99;c.regs[1]=input;c.undefined_flags=ARITH_FLAGS;
            let mut ram=Ram::new(8);ram.write_bytes(0,&[255;8]).unwrap();if protected {ram.protect(4,1).unwrap();}
            let result=execute_memory(&p,&mut c,dst,src,FlowContext{code_limit:0x4ff,..context()},&mut ram,address);
            let status=match result {
                Ok(())=>(0,0,0,0,0),
                Err(ExecError::MemoryAccess{eip,address,bytes,write})=>(1,eip,address,bytes as u32,u32::from(write)),
                Err(ExecError::GeneralProtection{eip,error_code})=>(2,eip,error_code as u32,0,0),
                Err(e)=>panic!("{e:?}"),
            };
            let mem0=u32::from_le_bytes(ram.bytes()[0..4].try_into().unwrap());
            let mem1=u32::from_le_bytes(ram.bytes()[4..8].try_into().unwrap());
            for n in [id as u32,address,input,u32::from(protected),c.regs[0],c.regs[1],c.eflags,c.undefined_flags,c.eip,mem0,mem1,status.0,status.1,status.2,status.3,status.4] {
                cases.extend_from_slice(&n.to_le_bytes());
            }
        }}}
    }
    source.push_str(r#"
fn main() {
 for c in include_bytes!("memory.bin").chunks_exact(64) {
  let n=|i:usize|u32::from_le_bytes(c[i..i+4].try_into().unwrap());
  let address=n(4);let protected=n(12)!=0;let mut regs=[0u32;8];regs[0]=0xabcd_ff99;regs[1]=n(8);
  let mut flags=0x203;let mut undefined=0x8d5;let mut ip=0x400;let mut output=[0u8;8];
  let status=match n(0) {
"#);
    source.push_str(&dispatch);
    source.push_str(r#"
   _=>panic!("unknown variant"),
  };
  assert_eq!(regs[0],n(16));assert_eq!(regs[1],n(20));assert_eq!(flags,n(24));assert_eq!(undefined,n(28));assert_eq!(ip,n(32));
  assert_eq!(u32::from_le_bytes(output[0..4].try_into().unwrap()),n(36));
  assert_eq!(u32::from_le_bytes(output[4..8].try_into().unwrap()),n(40));
  assert_eq!(status,(n(44),n(48),n(52),n(56),n(60)));
 }
}
"#);
    assert_eq!(cases.len()/64,560);
    fs::write(dir.join("generated.rs"),source).unwrap();fs::write(dir.join("memory.bin"),cases).unwrap();
    let bin=dir.join(if cfg!(windows){"memory.exe"}else{"memory"});
    let rustc=std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into());
    let out=Command::new(rustc).arg("--edition=2021").arg("-O").arg(dir.join("generated.rs")).arg("-o").arg(&bin).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
    let out=Command::new(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
    fs::remove_dir_all(dir).unwrap();
}
