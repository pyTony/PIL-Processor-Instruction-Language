//! Standalone COM/MZ translation with ordinary RAM effects and direct control flow.
//! PIL semantics are emitted by the existing backend; unsupported code fails at
//! build time. No guest instruction decoder or token interpreter is emitted.
use crate::guest;
use crate::dos_translate::{self,near_call,near_return};
#[cfg(test)] use crate::{dos,Token};
use std::fmt::Write as _;
use crate::dos_codegen::{self,Bindings,BlockAdapter};
use crate::dos_blocks::{Site,BlockPlan};
use std::collections::BTreeSet;
#[cfg(test)] use std::collections::BTreeMap;

// Standalone control metadata and retirement belong to this runtime adapter;
// the instruction call and bounded block plan are shared with Dine.
fn standalone_instruction(ip:u32,site:&Site,fast:bool,vga_port:bool)->String{
    // Statically verified OUT DX,AX is a native device boundary, not the
    // generic open-bus PIL operation. It preserves all registers and flags.
    if vga_port{
        return format!("c.out_vga_word()?;c.eip={};c.machine.retired();\n",site.flow.next_eip);
    }
    let program=&site.program;let flow=site.flow;
    let before=if near_return(program){format!("let expected=*c.calls.last().ok_or(\"standalone RET has no proven caller\")?; if expected.1!={}{{return Err(\"standalone CALL/RET operand widths differ\".into());}} let before=(c.regs,c.flags,c.undefined,c.eip,c.machine);",program.operand_width)}else{String::new()};
    let after=if near_return(program){String::from("if c.eip!=expected.0{let target=c.eip;(c.regs,c.flags,c.undefined,c.eip,c.machine)=before;return Err(format!(\"unproven standalone return target {target:04x}, expected {:04x}\",expected.0));} c.calls.pop();")}
        else if near_call(program){format!("c.calls.push(({},{}));",flow.next_eip&crate::mask(program.operand_width)as u32,program.operand_width)}else{String::new()};
    let call=dos_codegen::instruction_call(ip,site,&Bindings{regs:"&mut c.regs",flags:"&mut c.flags",undefined:"&mut c.undefined",eip:"&mut c.eip",ram:"&mut c.ram",machine:"&mut c.machine"},fast);
    format!("{before} {call}.map_err(|e|format!(\"guest instruction: {{e:?}}\"))?; {after} c.machine.retired();\n")
}
struct StandaloneBlocks<'a>{vga_ports:&'a BTreeSet<u32>}
impl BlockAdapter for StandaloneBlocks<'_> {
    fn header(&self,start:u32)->String{format!("#[inline(never)] fn block_{start:x}(c:&mut dos_runtime::Context)->Result<(),String>{{\n")}
    fn instruction(&self,ip:u32,site:&Site,_:usize)->String{standalone_instruction(ip,site,true,self.vga_ports.contains(&ip))}
    fn footer(&self,_:usize)->String{String::from("Ok(())\n}\n")}
}

/// Lower every reachable direct edge to native dispatch; all semantics come
/// from the existing PIL Rust emitter. No decoder is included in the result.
fn emit_direct(unit:&dos_translate::Unit)->Result<String,String>{
    let p=&unit.loaded;let graph=&unit.instructions;let sites=&unit.sites;
    // dos_translate has verified the exact DX port on every incoming path.
    let vga_ports:BTreeSet<u32>=graph.iter().filter_map(|(&ip,decoded)|
        (decoded.label=="OUT DX,AX").then_some(ip)).collect();
    let plan=BlockPlan::new(sites,std::iter::once(p.cpu.eip));
    let blocks=dos_codegen::emit_blocks(sites,&plan,&StandaloneBlocks{vga_ports:&vga_ports});
    let mut modules=String::new();let mut cases=String::new();let mut block_cases=String::new();
    for (&ip,decoded) in graph {
        match &decoded.operation {
            guest::Operation::Pil{program,..}=>{
                if !vga_ports.contains(&ip){modules.push_str(&dos_codegen::instruction_module(ip,program));}
                let body=standalone_instruction(ip,&sites[&ip],false,vga_ports.contains(&ip));
                writeln!(cases,"{ip} => {{ {body} Ok(None) }},").unwrap();
                writeln!(block_cases,"{ip} => {{block_{ip:x}(c)?;Ok(None)}},").unwrap();
            },
            guest::Operation::Interrupt{vector:0x20,..}=>{
                let case=format!("{ip} => Ok(Some(0)),\n");cases.push_str(&case);block_cases.push_str(&case);
            },
            guest::Operation::Interrupt{vector,next,..} if matches!(*vector,0x10|0x21)=>{
                let case=format!("{ip} => {{ let exit=c.interrupt({vector},output)?; c.eip={next}; c.machine.retired(); Ok(exit) }},\n");cases.push_str(&case);block_cases.push_str(&case);
            },
            _=>return Err("unsupported operation in verified graph".into()),
        }
    }
    let mut source=crate::emit_rust_support();
    for (name,text) in [("dos_encoding",include_str!("dos_encoding.rs")),("host_io",include_str!("host_io.rs")),("dos_runtime",include_str!("dos_runtime.rs"))]{
        writeln!(source,"\nmod {name} {{\n{}\n}}",text.split("#[cfg(test)]").next().unwrap()).unwrap();
    }
    source.push_str(&modules);source.push_str(&blocks);
    writeln!(source,"\nfn initial_context()->Result<dos_runtime::Context,String>{{\nlet mut ram=Ram::new({});",p.ram.bytes().len()).unwrap();
    let memory=p.ram.bytes();let mut at=0;
    while at<memory.len(){
        if memory[at]==0{at+=1;continue;}
        let start=at;while at<memory.len()&&memory[at]!=0{at+=1;}
        writeln!(source,"ram.write_bytes({start},&{:?}).map_err(|e|format!(\"initial memory: {{e:?}}\"))?;",&memory[start..at]).unwrap();
    }
    // Preserve the loader's bus protections, then protect precisely the bytes
    // whose semantics were translated. Adjacent instructions share one span;
    // unreachable data in the same module remains writable. RAM performs the
    // physical checks, so aliases and partial-overlap writes are covered too.
    let mut spans:Vec<std::ops::Range<usize>>=p.ram.readonly_ranges().to_vec();
    let mut code:Vec<std::ops::Range<usize>>=Vec::new();
    for (&ip,decoded) in graph {
        let start=(p.image.segments[1].base+ip)as usize;
        let end=start+decoded.length as usize;
        if let Some(last)=code.last_mut().filter(|last|last.end==start){last.end=end;}
        else{code.push(start..end);}
    }
    spans.extend(code);
    for span in spans {
        writeln!(source,"ram.protect({},{}).map_err(|e|format!(\"initial protection: {{e:?}}\"))?;",span.start,span.len()).unwrap();
    }
    writeln!(source,"ram.set_a20_enabled({});\nOk(dos_runtime::Context{{regs:{:?},flags:{},undefined:{},eip:{},ram,machine:{:?},video:dos_runtime::VideoState::default(),calls:Vec::new()}})\n}}",p.ram.a20_enabled(),p.cpu.regs,p.cpu.eflags,p.cpu.undefined_flags,p.cpu.eip,p.image.segment_state()).unwrap();
    source.push_str("#[cfg(test)] fn step(c:&mut dos_runtime::Context,output:&mut host_io::CaptureOutput)->Result<Option<u8>,String>{match c.eip {\n");
    source.push_str(&cases);
    source.push_str("_=>Err(format!(\"unexpected compiled target {:04x}\",c.eip)),\n}}\n");
    source.push_str("fn block_step(c:&mut dos_runtime::Context,output:&mut host_io::CaptureOutput)->Result<Option<u8>,String>{match c.eip {\n");
    source.push_str(&block_cases);
    writeln!(source,"_=>Err(format!(\"unexpected compiled target {{:04x}}\",c.eip)),\n}}}}\nfn run_with_tail(tail:Option<&str>)->Result<(u8,Vec<u8>),String>{{let mut c=initial_context()?;if let Some(text)=tail{{dos_runtime::command_tail(&mut c.ram,{},text)?;}}let mut output=host_io::CaptureOutput::new(host_io::DOS_OUTPUT_LIMIT);loop{{if let Some(code)=block_step(&mut c,&mut output)?{{return Ok((code,output.into_bytes()));}}}}}}",p.image.segments[3].base).unwrap();
    source.push_str("fn run()->Result<(u8,Vec<u8>),String>{let args=std::env::args_os().skip(1).map(|a|a.into_string().map_err(|_|\"host argument is not valid Unicode\".to_string())).collect::<Result<Vec<_>,_>>()?;let text=args.join(\" \");run_with_tail(if args.is_empty(){None}else{Some(&text)})}\n");
    source.push_str("fn main(){use std::io::Write;match run(){Ok((code,bytes))=>{let raw=std::env::var_os(\"PIL_DOS_RAW_OUTPUT\").is_some_and(|v|v==\"1\");if let Err(e)=std::io::stdout().lock().write_all(&dos_encoding::output_bytes(&bytes,raw)){eprintln!(\"output: {e}\");std::process::exit(1);}std::process::exit(i32::from(code));},Err(e)=>{eprintln!(\"{e}\");std::process::exit(1);}}}\n");
    source.push_str("\nuse address::*; use address::SegmentAccess::*; use address::{EffectiveAddress::*,Address16::*,SegmentRegister::*,Gpr::*};\n");
    Ok(source)
}

pub(crate) fn emit(bytes:&[u8],name:&str)->Result<String,String>{
    let lower=name.to_ascii_lowercase();
    if !lower.ends_with(".com")&&!lower.ends_with(".exe")&&!bytes.starts_with(b"MZ")&&!bytes.starts_with(b"ZM"){
        return Err("standalone compilation requires DOS .COM or MZ/ZM input".into());
    }
    let unit=dos_translate::prepare(bytes,name)?;
    emit_direct(&unit)
}

#[cfg(test)]
fn discover_direct(bytes:&[u8],name:&str)->Result<BTreeMap<u32,guest::Decoded>,String>{
    Ok(dos_translate::prepare(bytes,name)?.instructions)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_visits_both_branches_and_loop_edges_without_decoding_data(){
        // MOV CX,2; DEC CX; JNZ back; INT 20; trailing data.
        let bytes=[0xb9,2,0,0x49,0x75,0xfd,0xcd,0x20,0xff];
        let graph=discover_direct(&bytes,"TEST.COM").unwrap();
        assert_eq!(graph.keys().copied().collect::<Vec<_>>(),vec![0x100,0x103,0x104,0x106]);
    }
    #[test]
    fn branches_allow_only_proven_services_on_every_incoming_edge(){
        let valid=[0xb9,3,0,0xba,0x11,1,0xb4,9,0xcd,0x21,0xe2,0xfa,0xb4,0x4c,0xcd,0x21,b'X',b'$'];
        assert!(discover_direct(&valid,"T.COM").is_ok());
        // One possible branch bypasses MOV AH before INT 21.
        let bypass=[0x74,2,0xb4,0x4c,0xcd,0x21];
        assert!(discover_direct(&bypass,"T.COM").is_err());
    }
    #[test]
    fn service_analysis_follows_pil_writes_and_preserves_ah_across_branches(){
        // MOV AX,4C07; MOV AL,3; JMP to INT21. AH remains known.
        let bytes=[0xb8,7,0x4c,0xb0,3,0xeb,0,0xcd,0x21];
        assert!(discover_direct(&bytes,"T.COM").is_ok());
        // XCHG AX,BX writes AH through the PIL source binding, too.
        assert!(discover_direct(&[0xb8,0,0x4c,0x93,0xcd,0x21],"T.COM").is_err());
        // INC AX may carry into AH; the original service byte is no proof.
        assert!(discover_direct(&[0xb8,0xff,0x4b,0x40,0xcd,0x21],"T.COM").is_err());
        // An unsupported incoming service must not be hidden by the supported path.
        let mixed=[0x74,4,0xb4,0x30,0xeb,2,0xb4,0x4c,0xcd,0x21];
        assert!(discover_direct(&mixed,"T.COM").is_err());
    }
    #[test]
    fn memory_reads_use_shared_pil_and_keep_implicit_accesses_excluded(){
        for bytes in [
            vec![0xa0,0,2,0xb4,0x4c,0xcd,0x21],
            vec![0x66,0x67,0x8b,0x44,0xb3,0xfc,0xcd,0x20],
            vec![0x64,0x8b,0x46,0,0xcd,0x20],
            vec![0x80,0x3e,0,2,3,0xcd,0x20],
        ]{
            let graph=discover_direct(&bytes,"T.COM").unwrap();
            if let guest::Operation::Pil{program,..}=&graph[&0x100].operation{
                assert!(program.tokens.iter().any(|t|matches!(t,Token::Load(..))));
            }else{panic!("expected shared PIL");}
            let source=emit(&bytes,"T.COM").unwrap();
            assert!(source.contains("&mut c.ram,Some(AddressContext"));
            assert!(source.contains("ram.set_a20_enabled(false)"));
        }
        for bytes in [
            vec![0xa4,0xcd,0x20], // MOVSB
            vec![0xff,0x26,0,2], // indirect memory JMP
            vec![0x8a,0x26,0,2,0xcd,0x21], // loaded AH is not a proven service
        ]{assert!(emit(&bytes,"T.COM").is_err(),"{bytes:?}");}
    }
    #[test]
    fn ordinary_stores_and_rmw_use_existing_pil_effects(){
        for bytes in [
            vec![0xa2,0,2,0xcd,0x20],
            vec![0x66,0x67,0x89,0x44,0xb3,0xfc,0xcd,0x20],
            vec![0x80,0x07,1,0xcd,0x20],
            vec![0x87,0x06,0,2,0xcd,0x20],
        ]{
            let graph=discover_direct(&bytes,"T.COM").unwrap();
            if let guest::Operation::Pil{program,..}=&graph[&0x100].operation{
                assert!(program.tokens.iter().any(|t|matches!(t,Token::Store(..)|Token::StoreIf(..))));
            }else{panic!("expected shared PIL");}
            assert!(emit(&bytes,"T.COM").unwrap().contains("initial protection"));
        }
    }
    #[test]
    fn direct_calls_discover_returns_with_callee_service_state(){
        let bytes=[0xe8,4,0,0xb4,0x4c,0xcd,0x21,0x40,0xc3];
        let graph=discover_direct(&bytes,"T.COM").unwrap();
        assert_eq!(graph.keys().copied().collect::<Vec<_>>(),vec![0x100,0x103,0x105,0x107,0x108]);
        // A callee cannot silently invalidate a caller's proven service byte.
        let clobber=[0xb4,0x4c,0xe8,2,0,0xcd,0x21,0xb4,0x30,0xc3];
        assert!(emit(&clobber,"T.COM").is_err());
        for bytes in [
            vec![0xc3], // no known caller
            vec![0xe8,0xfd,0xff], // unbounded recursion
            vec![0xff,0xd0,0xcd,0x20], // indirect call
            vec![0xe8,2,0,0xcd,0x20,0x66,0xc3], // mismatched return width
        ]{assert!(emit(&bytes,"T.COM").is_err(),"{bytes:?}");}
    }
    #[test]
    fn recursive_returns_reach_a_fixed_point_without_assuming_service_preservation(){
        // A base case sets AH; recursive returns propagate it to INT 21h.
        let mut bytes=vec![0xe8,2,0,0xcd,0x21,0x83,0xf9,0,0x74,5,0x49,0xe8,0xf7,0xff,0xc3,0xb4,0x4c,0xc3];
        assert!(discover_direct(&bytes,"RECUR.COM").is_ok());
        bytes[16]=0x30;
        assert!(discover_direct(&bytes,"RECUR.COM").unwrap_err().contains("requires proven AH"));
    }
    #[test]
    fn register_and_immediate_stack_forms_remain_shared_pil(){
        for bytes in [
            vec![0x50,0x58,0xcd,0x20],
            vec![0x66,0x50,0x66,0x58,0xcd,0x20],
            vec![0x68,0x34,0x12,0x5b,0xcd,0x20],
            vec![0x6a,0xff,0x5b,0xcd,0x20],
        ]{assert!(discover_direct(&bytes,"T.COM").is_ok(),"{bytes:?}");}
    }
    #[test]
    fn scalar_lods_and_checked_division_use_the_original_pil_program(){
        for bytes in [
            vec![0xac,0xcd,0x20],vec![0xad,0xcd,0x20],vec![0x66,0xad,0xcd,0x20],
            vec![0x64,0x67,0xac,0xcd,0x20],
            vec![0xf6,0xf3,0xcd,0x20],vec![0xf7,0xfb,0xcd,0x20],vec![0x66,0xf7,0xf3,0xcd,0x20],
        ]{assert!(discover_direct(&bytes,"T.COM").is_ok(),"{bytes:?}");}
        for bytes in [vec![0xf3,0xac,0xcd,0x20],vec![0xa4,0xcd,0x20],vec![0xa6,0xcd,0x20]]{
            assert!(discover_direct(&bytes,"T.COM").is_err(),"{bytes:?}");
        }
    }
    #[test]
    fn discovery_accepts_near_conditions_and_independent_branch_sizes(){
        for bytes in [
            vec![0x0f,0x84,0,0,0xcd,0x20],
            vec![0x66,0x0f,0x84,0,0,0,0,0xcd,0x20],
            vec![0x67,0xe3,0,0xcd,0x20],
            vec![0x66,0xe9,0,0,0,0,0xcd,0x20],
        ]{assert!(discover_direct(&bytes,"T.COM").is_ok(),"{bytes:?}");}
        // A 32-bit target outside the file is rejected, never truncated to 16 bits.
        assert!(discover_direct(&[0x66,0xe9,0,0,1,0,0xcd,0x20],"T.COM").is_err());
        let mut exact=vec![0x90;4095];exact.extend_from_slice(&[0xcd,0x20]);
        assert_eq!(discover_direct(&exact,"T.COM").unwrap().len(),4096);
        exact.insert(0,0x90);
        assert!(discover_direct(&exact,"T.COM").is_err());
    }
    #[test]
    fn discovery_rejects_invalid_targets_unsupported_paths_and_no_exit(){
        assert!(discover_direct(&[0xeb,0x7f],"T.COM").is_err());
        assert!(discover_direct(&[0xeb,0xfe],"T.COM").is_err());
        // Both JZ successors must be valid, even if initial flags pick one.
        assert!(discover_direct(&[0x74,0x02,0xcd,0x20,0x0f,0xff],"T.COM").is_err());
        // Jump to the immediate byte of another reachable instruction.
        assert!(discover_direct(&[0x74,0x01,0xb0,0x20,0xcd,0x20],"T.COM").is_err());
    }
    #[test]
    fn standalone_has_no_decoder_or_interpreter(){
        let source=emit(&dos::hello("com").unwrap(),"HELLO.COM").unwrap();
        assert!(source.contains("mod dos_runtime"));assert!(source.contains("mod ins_100"));
        assert!(!source.contains("mod guest")&&!source.contains("enum Token")&&!source.contains("InstructionBackend"));
    }
    #[test]
    fn video_bios_boundary_requires_proven_mode_and_service_function(){
        // Mode 12h, cursor row 29, mode 3 restore, INT20 termination.
        let video=[0xb8,0x12,0,0xcd,0x10,0xb4,2,0xbb,0,0,0xba,0,0x1d,0xcd,0x10,0xb8,3,0,0xcd,0x10,0xcd,0x20];
        let source=emit(&video,"VIDEO.COM").unwrap();
        assert!(source.contains("video:dos_runtime::VideoState::default()"));
        assert!(source.contains("c.interrupt(16,output)"));
        for bytes in [
            vec![0xb4,0,0xcd,0x10,0xcd,0x20],
            vec![0xb8,0x13,0,0xcd,0x10,0xcd,0x20],
            vec![0xb4,3,0xcd,0x10,0xcd,0x20],
            vec![0xcd,0x16,0xcd,0x20],
            // An unproven or incorrect graphics-controller port fails:
            vec![0xba,0xcf,3,0xb8,5,2,0xef,0xcd,0x20],
            vec![0xec,0xcd,0x20], // IN AL,DX
        ]{assert!(emit(&bytes,"V.COM").is_err(),"{bytes:?}");}
    }
    #[test]
    fn proven_vga_out_word_calls_native_device_in_both_dispatch_paths(){
        // MOV DX,3CEh; MOV AX,0205h; OUT DX,AX; INT20.
        let guest=[0xba,0xce,3,0xb8,5,2,0xef,0xcd,0x20];
        let source=emit(&guest,"PORT.COM").unwrap();
        assert!(source.contains("c.out_vga_word()?;c.eip="));
        assert!(!source.contains("mod ins_106 {"),"port does not embed open-bus semantics");
        // Clobbering DX or varying its source cannot be considered proven.
        let wrong=[0xba,0xcf,3,0xb8,5,2,0xef,0xcd,0x20];
        assert!(emit(&wrong,"PORT.COM").is_err());
        let clobber=[0xba,0xce,3,0x42,0xb8,5,2,0xef,0xcd,0x20]; // INC DX
        assert!(emit(&clobber,"PORT.COM").is_err());
    }
    #[test]
    fn unsupported_programs_are_rejected(){
        for bytes in [vec![0xeb,0xfe],vec![0x90],vec![0xb4,0x30,0xcd,0x21],vec![0xa2,0,1],vec![0xb8]]{
            assert!(emit(&bytes,"TEST.COM").is_err(),"{bytes:?}");
        }
        assert!(emit(b"not an MZ executable","HELLO.EXE").is_err());
    }
    #[test]
    fn mz_uses_loader_entry_relocations_and_module_bounds(){
        let bytes=dos::hello("exe").unwrap();let p=dos::load(&bytes,"HELLO.EXE").unwrap();
        let graph=discover_direct(&bytes,"HELLO.EXE").unwrap();
        assert!(graph.contains_key(&p.cpu.eip));
        assert!(emit(&bytes,"HELLO.EXE").unwrap().contains(&format!("eip:{}",p.cpu.eip)));
        let mut zm=bytes;zm[..2].copy_from_slice(b"ZM");assert!(emit(&zm,"HELLO.EXE").is_ok());
    }
}
