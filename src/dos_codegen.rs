//! Rust instruction/block rendering and runtime-adapter bindings.
use crate::Program;
use std::collections::BTreeMap;
use crate::dos_blocks::{Site,BlockPlan,unpaged_register_only};

/// Both backends compile the same PIL bodies and expose the same checked and
/// unpaged-register entry points. Selection is a block planning decision.
pub(crate) fn instruction_module(ip:u32,p:&Program)->String{
    let functions=crate::emit_rust_functions(p)
        .replace("fn execute_state(","#[inline(always)] fn execute_state(")
        .replace("pub fn execute_with_machine(","#[inline(always)] pub fn execute_with_machine(");
    let mut out=format!("mod ins_{ip:x} {{use crate::{{FlowContext,ExecError,Ram,MemWidth,address,privilege,task}}; {functions}\n");
    if unpaged_register_only(p){
        out.push_str("#[inline(always)] pub fn execute_unpaged_register(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,_ram:&mut Ram,_address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,Some(machine),None)}\n");
    }
    out.push_str("}\n");out
}

/// Runtime adapters provide Rust bindings, not instruction semantics.
pub(crate) struct Bindings {
    pub(crate) regs:&'static str, pub(crate) flags:&'static str,
    pub(crate) undefined:&'static str, pub(crate) eip:&'static str,
    pub(crate) ram:&'static str, pub(crate) machine:&'static str,
}
pub(crate) fn instruction_call(ip:u32,site:&Site,b:&Bindings,fast:bool)->String{
    let method=if fast&&site.profile.register_only{"execute_unpaged_register"}else{"execute_with_machine"};
    format!("ins_{ip:x}::{method}({},{},{},{},({},{}),({},{}),{:?},{},{:?},{})",b.regs,b.flags,b.undefined,b.eip,
        site.dst.register,site.dst.low,site.src.register,site.src.low,site.flow,b.ram,site.address,b.machine)
}

/// Runtime-specific fault reporting, retirement and call metadata are kept at
/// this boundary. Both adapters receive the identical planned instruction order.
pub(crate) trait BlockAdapter {
    fn header(&self,start:u32)->String;
    fn instruction(&self,ip:u32,site:&Site,retired_prefix:usize)->String;
    fn footer(&self,length:usize)->String;
}
pub(crate) fn emit_blocks(sites:&BTreeMap<u32,Site>,plan:&BlockPlan,adapter:&impl BlockAdapter)->String{
    let mut out=format!("// Shared block plan: {}\n",plan.summary());
    for &start in sites.keys(){
        let sequence=plan.sequence(sites,start);if sequence.is_empty(){continue;}
        out.push_str(&adapter.header(start));
        for (n,(ip,site)) in sequence.iter().enumerate(){out.push_str(&adapter.instruction(*ip,site,n));}
        out.push_str(&adapter.footer(sequence.len()));
    }
    out
}
