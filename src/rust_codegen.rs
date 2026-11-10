//! Rust rendering of backend-neutral PIL dataflow idioms.
use crate::Program;
#[cfg(test)] use crate::Token;

pub(crate) fn carry_window(p:&Program,at:usize)->Option<String>{
    let crate::pil_opt::CarryWindow{partial,wide,result,cf,a,b,carry,width:w,op}=crate::pil_opt::carry_window(p,at)?;
    let method=match op{crate::pil_opt::CarryOp::Add=>"add",crate::pil_opt::CarryOp::Sub=>"sub"};
    Some(format!("\
let v{partial}:u64=v{a}.wrapping_{method}(v{b}) & {mask}u64;\n\
let (carry_p{wide},carry_a{wide})=(v{a} as u{w}).overflowing_{method}(v{b} as u{w});\n\
let (carry_r{wide},carry_b{wide})=carry_p{wide}.overflowing_{method}(v{carry} as u{w});\n\
let v{result}:u64=carry_r{wide} as u64;\n\
let v{cf}:u64=u64::from(carry_a{wide}|carry_b{wide});\n\
let v{wide}:u64=v{result}|(v{cf}<<{w});\n",mask=crate::mask(w+1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rewritten(p:&Program)->usize{(0..p.tokens.len()).filter(|&i|carry_window(p,i).is_some()).count()}
    #[test]
    fn carry_rewrite_requires_proven_widths_and_matching_dataflow(){
        for op in ["adc","sbb"]{for w in [8,16,32]{
            let p=crate::compile(&crate::instantiate(crate::description(op).unwrap(),w),w).unwrap();
            assert_eq!(rewritten(&p),1);
        }}
        // A wide input or a multi-bit carry cannot be truncated by the rewrite.
        let template="const a $A 1\nzext aw a 9\nconst b 8 1\nzext bw b 9\nconst c $C 1\nzext cw c 9\nadd p aw bw\nadd q p cw\ntrunc r q 8\nbit carry q 8\nwrite dst r\npreserve_flags all\nretire";
        for (a,c,count) in [(8,1,1),(9,1,0),(8,2,0)]{
            let p=crate::compile(&template.replace("$A",&a.to_string()).replace("$C",&c.to_string()),8).unwrap();
            assert_eq!(rewritten(&p),count);
        }
    }
    #[test]
    fn rewritten_carry_keeps_memory_faults_transactional(){
        use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
        let dir=std::env::temp_dir().join(format!("pil-carry-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&dir).unwrap();let mut source=String::new();
        for op in ["adc","sbb"]{for w in [8,16,32]{
            let text=crate::instantiate(crate::description(op).unwrap(),w)
                .replace(&format!("read a dst {w}"),&format!("read_address addr\nload a addr {w}"))
                .replace("write dst r","store addr r");
            let p=crate::compile(&text,w).unwrap();assert_eq!(rewritten(&p),1);
            source.push_str(&format!("mod {op}{w}{{{}\npub fn check(){{let mut regs=[0x12345678;8];let mut f=0xad7;let mut u=0x8d5;let mut ip=0x100;let mut ram=Ram::new(8);ram.write_bytes(0,&[255;8]).unwrap();ram.protect(0,8).unwrap();let before=(regs,f,u,ip,ram.clone());assert!(matches!(execute_with_memory(&mut regs,&mut f,&mut u,&mut ip,(0,0),(1,0),FlowContext{{next_eip:0x102,displacement:0,displacement_bits:32,code_limit:0xffff}},&mut ram,0),Err(ExecError::MemoryAccess{{write:true,..}})));assert_eq!((regs,f,u,ip,ram),before);}}}}\n",crate::emit_rust(&p)));
        }}
        source.push_str("fn main(){adc8::check();adc16::check();adc32::check();sbb8::check();sbb16::check();sbb32::check();}");
        let path=dir.join("check.rs");fs::write(&path,source).unwrap();let bin=dir.join(if cfg!(windows){"check.exe"}else{"check"});
        let result=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).args(["--edition=2021","-Awarnings","-O"]).arg(path).arg("-o").arg(&bin).output().unwrap();
        assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
        assert!(Command::new(bin).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
    }
}
