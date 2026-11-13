//! Program-independent pieces of the former block precompilers, kept for
//! integration into the general engine.
//!
//! PIL and Dine are program-independent libraries, used the same way
//! everywhere. Nothing in this repository builds an executable for one guest
//! program any more: the linked shadow engine (`--emit-dos-backend`,
//! `PIL_SHADOW_SOURCE`), the standalone translator (`--emit-dos-standalone`)
//! and the `Dine_<program>` launchers were removed. What they had in common,
//! and what does not depend on any program, is here:
//!
//! - `sites`: `Site` (one decoded PIL instruction with its bindings, flow and
//!   segment context), `InstructionProfile` (register-only / memory /
//!   machine-context tags derived from the PIL tokens) and `BlockPlan`
//!   (partition into blocks of at most 16 instructions, with one-instruction
//!   alternate entries for interior targets).
//! - `codegen`: Rust rendering of a PIL program's body (`emit_rust_functions`)
//!   and of a planned block through a `BlockAdapter`.
//! - `guards`: the entry checks a compiled block needs (segment state
//!   compatible, code bytes unchanged and inside an executable window) and
//!   the 256-instruction chaining bound.
//! - `recorder`: collects executed sites from any DOS session through the
//!   `dos::InstructionBackend` hook, grouped by code segment base.
//!
//! The hook itself (`dos::InstructionBackend`, `dos::run_with_backend`,
//! `Session::with_backend`) stays in Dine; with no backend the interpreter
//! runs everything. None of this is wired into a command yet. Integration is
//! the subject of docs/development/BLOCK_COMPILER.md.
#![allow(dead_code)]
pub(crate) mod sites;
pub(crate) mod codegen;
pub(crate) mod guards;
pub(crate) mod recorder;

#[cfg(test)]
mod tests {
    use super::{codegen::{self,BlockAdapter,Bindings},recorder,sites::{BlockPlan,Site}};
    use std::collections::BTreeMap;
    struct Plain;
    impl BlockAdapter for Plain {
        fn header(&self,start:u32)->String{format!("fn block_{start:x}(){{\n")}
        fn instruction(&self,ip:u32,site:&Site,_:usize)->String{
            format!("{};\n",codegen::instruction_call(ip,site,&Bindings{regs:"r",flags:"f",undefined:"u",eip:"e",ram:"m",machine:"s"},true))
        }
        fn footer(&self,_:usize)->String{"}\n".into()}
    }
    fn compile(bytes:&[u8],name:&str)->(BTreeMap<u32,Site>,String){
        let (r,_)=recorder::record(bytes,name,&[],100_000,65_536,None,None).unwrap();
        let s=r.segments.into_values().find(|s|!s.sites.is_empty()).unwrap();
        let plan=BlockPlan::new(&s.sites,s.entries.iter().copied());
        let text=codegen::emit_blocks(&s.sites,&plan,&Plain);
        (s.sites,text)
    }
    fn block(text:&str,start:&str)->String{text.split(&format!("fn block_{start}(")).nth(1).unwrap().split("\n}\n").next().unwrap().to_string()}
    #[test]
    fn observed_indirect_target_splits_a_previously_linear_block(){
        // The first pass falls through INC AX to INC DX; later JMP BX enters
        // INC DX. No direct relative edge names that interior target.
        let bytes=[0xb9,2,0,0x40,0x42,0x49,0x74,5,0xbb,4,1,0xff,0xe3,0xb4,0x4c,0xcd,0x21];
        let (_,text)=compile(&bytes,"T.COM");
        let first=block(&text,"100");
        assert!(first.contains("ins_100::")&&first.contains("ins_103::")&&!first.contains("ins_104::"));
        let target=block(&text,"104");
        for ip in ["104","105","106"]{assert!(target.contains(&format!("ins_{ip}::")));}
    }
    #[test]
    fn profiles_choose_the_ram_free_entry_only_when_proven(){
        // Register MOV, memory MOV, segment MOV, then DOS exit.
        let bytes=[0xb8,0,0,0xa1,0,1,0x8e,0xd8,0xb8,0,0x4c,0xcd,0x21];
        let (sites,text)=compile(&bytes,"T.COM");
        assert_eq!(sites[&0x100].profile.tag(),"register-only");
        assert_eq!(sites[&0x103].profile.tag(),"memory");
        assert_eq!(sites[&0x106].profile.tag(),"machine-context");
        let b=block(&text,"100");
        assert!(b.contains("ins_100::execute_unpaged_register"));
        assert!(b.contains("ins_103::execute_with_machine"));
        assert!(b.contains("ins_106::execute_with_machine"));
        assert!(!b.contains("ins_108::"),"a segment write ends the block");
        let module=codegen::instruction_module(0x100,&sites[&0x100].program);
        assert!(module.contains("fn execute_unpaged_register"));
        assert!(!codegen::instruction_module(0x103,&sites[&0x103].program).contains("fn execute_unpaged_register"));
    }
    #[test]
    fn a_branch_is_the_last_instruction_of_its_block(){
        // MOV AX,1; INC AX; JMP over two NOPs; exit via DOS.
        let bytes=[0xb8,1,0,0x40,0xeb,2,0x90,0x90,0xb4,0x4c,0xcd,0x21];
        let (_,text)=compile(&bytes,"T.COM");
        let b=block(&text,"100");
        for ip in ["100","103","104"]{assert!(b.contains(&format!("ins_{ip}::execute_unpaged_register")));}
        assert!(!b.contains("ins_108::"));
    }
    #[test]
    fn hexadecimal_f_offset_cannot_form_the_rust_keyword_if(){
        let mut bytes=crate::dos::hello("exe").unwrap();
        bytes[20..22].copy_from_slice(&15u16.to_le_bytes());
        bytes[47..52].copy_from_slice(&[0xb8,0,0x4c,0xcd,0x21]);
        let (sites,text)=compile(&bytes,"T.EXE");
        assert!(text.contains("ins_f::"));
        assert!(codegen::instruction_module(0xf,&sites[&0xf].program).contains("mod ins_f {"));
    }
}
