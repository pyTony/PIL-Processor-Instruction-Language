//! Two bound register effects, snapshots, alias conflicts and precise retirement.
use super::*;
fn v(r:usize,low:u8,w:u8)->View{View{register:r,low,width:w}}
fn context()->FlowContext{FlowContext{next_eip:0x102,displacement:0,displacement_bits:32,code_limit:0x1ff}}
fn cpu()->Cpu{Cpu{regs:[0x12345678,0xabcdef01,2,3,4,5,6,7],eflags:0xad7,undefined_flags:0x8d5,eip:0x100}}
const OVERLAP:&str="const a 16 0xaa55\nconst b 8 0xaa\nwrite dst a\nwrite src b\nread_address addr\nstore addr a\npreserve_flags all\nretire";
const LATE:&str="read a dst 32\nwrite src a\nread b src 32\nwrite dst b\nread_address addr\nstore addr a\nconst carry 1 0\nflag CF carry\nconst target 32 0x200\nconst yes 1 1\nbranch_if target yes\npreserve_flags remaining\nretire";
#[test]
fn source_write_validation_rejects_duplicates_widths_and_bad_predicates(){
    for text in [
        "const a 8 1\nwrite src a\nwrite src a\npreserve_flags all\nretire",
        "const a 8 1\nconst no 1 0\nwrite_if src a no\nwrite_if src a no\npreserve_flags all\nretire",
        "read a src 16\nconst b 8 1\nwrite src b\npreserve_flags all\nretire",
        "const a 8 1\nwrite_if src a a\npreserve_flags all\nretire",
        "const a 64 1\nwrite src a\npreserve_flags all\nretire",
    ]{assert!(compile(text,16).is_err(),"{text}");}
}
#[test]
fn exchange_bytes_exhaustive_and_disjoint_high_byte_aliases(){
    let p=compile(&instantiate(description("xchg").unwrap(),8),8).unwrap();
    for a in 0..256{for b in 0..256{
        for shared in [false,true]{let mut c=cpu();let before=c.clone();
            c.regs[0]=0xabcd0000|a|(b<<8);c.regs[1]=0x12340000|b;
            let src=if shared{v(0,8,8)}else{v(1,0,8)};
            execute_flow(&p,&mut c,v(0,0,8),src,context()).unwrap();
            assert_eq!(c.regs[0],if shared{0xabcd0000|b|(a<<8)}else{0xabcd0000|b|(b<<8)});
            assert_eq!(c.regs[1],if shared{0x12340000|b}else{0x12340000|a});
            assert_eq!((c.eflags,c.undefined_flags,c.eip),(before.eflags,before.undefined_flags,0x102));
        }
    }}
}
#[test]
fn overlapping_effects_agree_or_fail_before_any_ram_or_cpu_commit(){
    for equal in [false,true]{let text=if equal{OVERLAP.to_owned()}else{OVERLAP.replace("8 0xaa","8 0xbb")};
        let p=compile(&text,16).unwrap();let mut c=cpu();let old=c.clone();let mut ram=Ram::new(8);let oldram=ram.clone();
        let result=execute_memory(&p,&mut c,v(0,0,16),v(0,8,8),context(),&mut ram,0);
        if equal{result.unwrap();assert_eq!(c.regs[0],0x1234aa55);assert_eq!(ram.load(0,MemWidth::Word).unwrap(),0xaa55);}
        else{assert_eq!(result,Err(ExecError::InvalidBinding));assert_eq!(c,old);assert_eq!(ram,oldram);}
    }
}
#[test]
fn late_fault_cancels_both_registers_flags_and_store_reads_are_snapshots(){
    let p=compile(LATE,32).unwrap();
    for allowed in [false,true]{let mut c=cpu();let old=c.clone();let mut ram=Ram::new(8);let oldram=ram.clone();
        let f=FlowContext{code_limit:if allowed{0x200}else{0x1ff},..context()};
        let result=execute_memory(&p,&mut c,v(0,0,32),v(1,0,32),f,&mut ram,0);
        if allowed{result.unwrap();assert_eq!((c.regs[0],c.regs[1]),(old.regs[1],old.regs[0]));assert_eq!(c.eip,0x200);assert_eq!(ram.load(0,MemWidth::Dword).unwrap(),old.regs[0]);}
        else{assert_eq!(result,Err(ExecError::GeneralProtection{eip:0x100,error_code:0}));assert_eq!(c,old);assert_eq!(ram,oldram);}
    }
}
#[test]
fn inactive_source_write_does_not_conflict_but_binding_is_still_validated(){
    let only=compile("const a 8 0xaa\nwrite src a\npreserve_flags all\nretire",32).unwrap();assert_eq!(only.source_width,8);
    let mut c=cpu();execute_flow(&only,&mut c,v(1,0,32),v(0,8,8),context()).unwrap();assert_eq!(c.regs[0],0x1234aa78);assert_eq!(c.regs[1],0xabcdef01);
    let p=compile("const a 16 0xaa55\nwrite dst a\nconst b 8 0xbb\nconst no 1 0\nwrite_if src b no\npreserve_flags all\nretire",16).unwrap();
    let mut c=cpu();execute_flow(&p,&mut c,v(0,0,16),v(0,8,8),context()).unwrap();assert_eq!(c.regs[0],0x1234aa55);
    let old=c.clone();assert_eq!(execute_flow(&p,&mut c,v(0,0,16),v(8,0,8),context()),Err(ExecError::InvalidBinding));assert_eq!(c,old);
}
#[test]
fn generated_two_effects_match_all_state_aliases_conditions_and_faults(){
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-effects-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&dir).unwrap();
    let mut source=String::from("#![allow(dead_code,unused_mut,unused_variables)]\n");let mut checks=String::new();let mut id=0;
    let mut fixtures=Vec::new();
    for w in [8,16,32]{let text=instantiate(description("xchg").unwrap(),w);
        for src in [v(0,0,w),v(1,0,w)]{fixtures.push((text.clone(),w,v(0,0,w),src,false,context()));}
        if w==8{fixtures.push((text,w,v(0,0,8),v(0,8,8),false,context()));}
    }
    for text in [OVERLAP.to_owned(),OVERLAP.replace("8 0xaa","8 0xbb")]{for protect in [false,true]{fixtures.push((text.clone(),16,v(0,0,16),v(0,8,8),protect,context()));}}
    for allowed in [false,true]{for protect in [false,true]{fixtures.push((LATE.to_owned(),32,v(0,0,32),v(1,0,32),protect,FlowContext{code_limit:if allowed{0x200}else{0x1ff},..context()}));}}
    for active in [0,1]{let text=format!("const a 16 0xaa55\nwrite dst a\nconst b 8 0xbb\nconst c 1 {active}\nwrite_if src b c\npreserve_flags all\nretire");fixtures.push((text,16,v(0,0,16),v(0,8,8),false,context()));}
    for(text,w,dst,src,protect,flow)in fixtures{
        let p=compile(&text,w).unwrap();let initial=cpu();let mut actual=initial.clone();let mut ram=Ram::new(8);if protect{ram.protect(0,8).unwrap();}
        let status=format!("{:?}",execute_memory(&p,&mut actual,dst,src,flow,&mut ram,0));
        source.push_str(&format!("mod p{id} {{ {} }}\n",emit_rust(&p)));
        checks.push_str(&format!("{{let mut regs={:?};let mut flags={};let mut u={};let mut ip={};let mut ram=p{id}::Ram::new(8);if {protect}{{ram.protect(0,8).unwrap();}}let f=p{id}::{flow:?};let status=p{id}::execute_with_memory(&mut regs,&mut flags,&mut u,&mut ip,({},{}),({},{}),f,&mut ram,0);assert_eq!(format!(\"{{:?}}\",status),{:?});assert_eq!(regs,{:?});assert_eq!((flags,u,ip),({},{},{}));assert_eq!(ram.bytes(),&{:?});}}\n",initial.regs,initial.eflags,initial.undefined_flags,initial.eip,dst.register,dst.low,src.register,src.low,status,actual.regs,actual.eflags,actual.undefined_flags,actual.eip,ram.bytes()));id+=1;
    }
    source.push_str(&format!("fn main(){{{checks}}}"));let file=dir.join("proof.rs");fs::write(&file,source).unwrap();let bin=dir.join(if cfg!(windows){"proof.exe"}else{"proof"});
    let out=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).args(["--edition=2021","-O"]).arg(file).arg("-o").arg(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));assert!(Command::new(bin).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
}
