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
//! - `engine`: run-time blocks over the precompiled instruction forms
//!   (src/precompiled), used by the shared DOS loop for every program.
//!
//! The hook (`dos::InstructionBackend`, `dos::run_with_backend`,
//! `Session::with_backend`) stays in Dine; with no backend the DOS loop runs
//! `engine` blocks, native forms, then the interpreter. See
//! docs/development/BLOCK_COMPILER.md.
#![allow(dead_code)]
pub(crate) use pil_cpu::block_compiler::{sites,engine,pil_rust};
#[cfg(test)]
use pil_cpu::block_compiler::{codegen,guards};
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

#[cfg(test)]
mod guard_tests {
    use super::guards::*;
    use crate::address::SegmentCache;
    use crate::dos;
    #[test]
    fn real_mode_relocation_is_compatible_but_unreal_and_paging_are_not(){
        let p=dos::load(&[0x40,0xcd,0x20],"T.COM").unwrap();let expected=p.image.segment_state();
        let mut moved=expected;moved.selectors=moved.selectors.map(|s|s.wrapping_add(0x1000));moved.segments=moved.selectors.map(SegmentCache::real_mode);
        assert!(compatible(&moved,&expected));
        moved.paging=true;assert!(!compatible(&moved,&expected));moved.paging=false;
        moved.segments[1].limit=0x1ffff;assert!(!compatible(&moved,&expected));
    }
    #[test]
    fn changed_bytes_limit_and_fetch_window_fail_the_guard(){
        let mut p=dos::load(&[0x40,0x41,0xcd,0x20],"T.COM").unwrap();
        let ip=p.cpu.eip;
        assert!(code_matches(&p.image,&p.ram,ip,&[0x40,0x41]));
        assert!(!code_matches(&p.image,&p.ram,ip,&[0x40,0x42]));
        let mut narrowed=p.image.clone();narrowed.end=narrowed.start;
        assert!(!code_matches(&narrowed,&p.ram,ip,&[0x40]));
        let mut limited=p.image.clone();limited.segments[1].limit=ip;
        assert!(!code_matches(&limited,&p.ram,ip,&[0x40,0x41]));
        let at=p.image.segments[1].base+ip;p.ram.store(at,crate::MemWidth::Byte,0x48).unwrap();
        assert!(!code_matches(&p.image,&p.ram,ip,&[0x40,0x41]));
    }
}

#[cfg(test)]
mod site_tests {
    use super::sites::*;
    use std::collections::BTreeMap;
    fn sites(bytes:&[u8])->BTreeMap<u32,Site>{
        let p=crate::dos::load(bytes,"PLAN.COM").unwrap();let mut sites=BTreeMap::new();let mut ip=0x100;
        while ip<0x100+bytes.len()as u32{
            let decoded=p.image.decode(&p.ram,ip).unwrap();
            if let crate::guest::Operation::Pil{program,dst,src,flow,address}=decoded.operation{
                let profile=InstructionProfile::of(&program,address);
                sites.insert(ip,Site{signature:bytes[(ip-0x100)as usize..(ip-0x100+decoded.length)as usize].to_vec(),program,dst,src,next:flow.next_eip,flow,address,machine:p.image.segment_state(),profile});
            }
            ip+=decoded.length;
        }
        sites
    }
    #[test]
    fn interior_entries_stay_available_without_repeating_full_suffixes(){
        let sites=sites(&[0xb8,1,0,0x40,0x41,0x42,0x43,0xcd,0x20]);
        let plan=BlockPlan::new(&sites,[0x100]);
        assert_eq!(plan.sequence(&sites,0x100).len(),5);
        for ip in 0x103..=0x106{assert_eq!(plan.sequence(&sites,ip).iter().map(|(ip,_)|*ip).collect::<Vec<_>>(),[ip]);}
        assert_eq!(plan.entries.values().map(Vec::len).sum::<usize>(),9); // old suffixes: 15
        let hinted=BlockPlan::new(&sites,[0x100,0x105]);
        assert_eq!(hinted.sequence(&sites,0x100).len(),3);
        assert_eq!(hinted.sequence(&sites,0x105).len(),2);
    }
    #[test]
    fn direct_targets_and_store_boundaries_split_primary_blocks(){
        let sites=sites(&[0xb8,1,0,0x40,0x41,0x75,0xfd,0xc6,6,0,2,1,0x42,0xcd,0x20]);
        let plan=BlockPlan::new(&sites,[0x100]);
        assert_eq!(plan.sequence(&sites,0x100).iter().map(|(ip,_)|*ip).collect::<Vec<_>>(),[0x100,0x103]);
        assert_eq!(plan.sequence(&sites,0x104).iter().map(|(ip,_)|*ip).collect::<Vec<_>>(),[0x104,0x105]);
        assert_eq!(plan.sequence(&sites,0x107).len(),1); // store ends its block
        assert_eq!(plan.sequence(&sites,0x10c).len(),1);
    }
    #[test]
    fn primary_blocks_keep_the_limit_context_and_partial_boundaries(){
        let mut sites=sites(&[vec![0x90;35],vec![0xcd,0x20]].concat());
        let plan=BlockPlan::new(&sites,[0x100]);
        assert_eq!(plan.sequence(&sites,0x100).len(),16);
        assert_eq!(plan.sequence(&sites,0x110).len(),16);
        assert_eq!(plan.sequence(&sites,0x120).len(),3);
        sites.get_mut(&0x102).unwrap().program.partial=true;
        sites.get_mut(&0x105).unwrap().machine.ss_shadow=1;
        let plan=BlockPlan::new(&sites,[0x100]);
        assert_eq!(plan.sequence(&sites,0x100).len(),2);assert!(!plan.contains(0x102));
        assert_eq!(plan.sequence(&sites,0x103).len(),2);
        assert_eq!(plan.sequence(&sites,0x105).len(),1);
    }
}
