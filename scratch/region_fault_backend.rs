// Generated from observed PIL sites; unknown or changed instructions use the interpreter.
use crate::{address,dos,guest,Cpu,Ram,FlowContext};
use address::*;
use address::SegmentAccess::*;
use address::{EffectiveAddress::*,Address16::*,SegmentRegister::*};
const ORIGINAL:&[u8]=&[185, 3, 0, 184, 0, 0, 232, 14, 0, 226, 251, 198, 6, 18, 1, 65, 235, 0, 64, 49, 219, 246, 243, 64, 195];
mod i100 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,false)?;
if false && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
let v0: u64 = 3u64;
pending_dst = Some(v0);
// Preserve flag mask 0x8d5, including undefined status.
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};
execute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|"invalid execution context")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
#[inline(always)] pub fn execute_unpaged_register(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,_address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),0,None,Some(machine),None)}
 }
mod i103 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,false)?;
if false && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
let v0: u64 = 0u64;
pending_dst = Some(v0);
// Preserve flag mask 0x8d5, including undefined status.
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};
execute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|"invalid execution context")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
#[inline(always)] pub fn execute_unpaged_register(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,_address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),0,None,Some(machine),None)}
 }
mod i106 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,true)?;
if true && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
// Preserve flag mask 0x0, including undefined status.
let v0: u64 = (context.next_eip as u64) & 4294967295u64;
let v1: u64 = v0 & 65535u64;
let v2: u64 = (context.displacement as u32 as u64) & 65535u64;
let v3: u64 = (v1.wrapping_add(v2)) & 65535u64;
let v4: u64 = v3 & 4294967295u64;
let v5: u64 = (regs[4] as u64) & 65535u64;
let v6: u64 = 2u64;
let v7: u64 = (v5.wrapping_sub(v6)) & 65535u64;
let v8: u64 = v7 & 4294967295u64;
let cache=machine.as_deref().ok_or(ExecError::InvalidContext)?.segments[2];let at=address::MemoryContext{segment:address::SegmentRegister::Ss,cache}.translate(v8 as u32,MemWidth::Word,true,*eip)?;ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::Word).map_err(|f|f.error(*eip))?;pending_stores[store_index]=Some((at,MemWidth::Word,v1 as u32));store_index+=1;
let v9: u64 = 1u64;
if v9 != 0 { if v4 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v4 as u32; }
pending_registers.push((4,16,v7));
// Preserve flag mask 0x8d5, including undefined status.
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
Err(ExecError::InvalidContext)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
Err("flow context required: use execute_with_flow")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
 }
mod i109 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,true)?;
if false && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
// Preserve flag mask 0x0, including undefined status.
let v0: u64 = (regs[1] as u64) & 65535u64;
let v1: u64 = 1u64;
let v2: u64 = (v0.wrapping_sub(v1)) & 65535u64;
pending_registers.push((1,16,v2));
let v3: u64 = u64::from(v2 == 0);
let v4: u64 = (!v3) & 1u64;
let v5: u64 = (v4 | v4) & 1u64;
let v6: u64 = (context.next_eip as u64) & 4294967295u64;
let v7: u64 = v6 & 65535u64;
let v8: u64 = (context.displacement as u32 as u64) & 65535u64;
let v9: u64 = (v7.wrapping_add(v8)) & 65535u64;
let v10: u64 = v9 & 4294967295u64;
if v5 != 0 { if v10 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v10 as u32; }
// Preserve flag mask 0x8d5, including undefined status.
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
Err("flow context required: use execute_with_flow")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
#[inline(always)] pub fn execute_unpaged_register(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,_address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),0,None,Some(machine),None)}
 }
mod i10b {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 8u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 8u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,false)?;
if true && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
let v0: u64 = address as u64;
let v1: u64 = 65u64;
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Byte,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Byte).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Byte,v1 as u32));store_index+=1;
// Preserve flag mask 0x8d5, including undefined status.
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(255u64<<dst.1)&(255u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 255u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=255u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
Err(ExecError::InvalidContext)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
Err("flow context required: use execute_with_flow")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
 }
mod i110 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,true)?;
if false && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
// Preserve flag mask 0x0, including undefined status.
let v0: u64 = (context.next_eip as u64) & 4294967295u64;
let v1: u64 = v0 & 65535u64;
let v2: u64 = (context.displacement as u32 as u64) & 65535u64;
let v3: u64 = (v1.wrapping_add(v2)) & 65535u64;
let v4: u64 = v3 & 4294967295u64;
let v5: u64 = 1u64;
if v5 != 0 { if v4 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v4 as u32; }
// Preserve flag mask 0x8d5, including undefined status.
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
Err("flow context required: use execute_with_flow")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
#[inline(always)] pub fn execute_unpaged_register(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,_address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),0,None,Some(machine),None)}
 }
mod i112 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,false)?;
if false && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 65535u64;
let v1: u64 = 1u64;
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_add(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (!v7) & 65535u64;
let v9: u64 = (v0 ^ v5) & 65535u64;
let v10: u64 = (v8 & v9) & 65535u64;
let v11: u64 = (v10 >> 15) & 1;
let v12: u64 = (v7 ^ v5) & 65535u64;
let v13: u64 = (v12 >> 4) & 1;
let v14: u64 = (v5 >> 15) & 1;
let v15: u64 = u64::from(v5 == 0);
let v16: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_dst = Some(v5);
// Preserve flag mask 0x1, including undefined status.
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v16 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v13 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v15 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v14 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v11 as u32) << 11);
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};
execute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|"invalid execution context")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
#[inline(always)] pub fn execute_unpaged_register(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,_address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),0,None,Some(machine),None)}
 }
mod i113 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,false)?;
if false && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 65535u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
let v2: u64 = (v0 ^ v1) & 65535u64;
let v3: u64 = 0u64;
let v4: u64 = (v2 >> 15) & 1;
let v5: u64 = u64::from(v2 == 0);
let v6: u64 = u64::from((v2 as u8).count_ones() % 2 == 0);
pending_dst = Some(v2);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v3 as u32) << 0);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v3 as u32) << 11);
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v4 as u32) << 7);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v5 as u32) << 6);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v6 as u32) << 2);
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};
execute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|"invalid execution context")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
#[inline(always)] pub fn execute_unpaged_register(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,_address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),0,None,Some(machine),None)}
 }
mod i115 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 8u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 8u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,false)?;
if false && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
let v0: u64 = (regs[0] as u64) & 65535u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
let v2: u64 = u64::from(v1 == 0);
let v3: u64 = (!v2) & 1u64;
if v3==0{return Err(ExecError::Exception{eip:*eip,vector:0});}
let v4: u64 = v1 & 65535u64;
let v5: u64 = (if v4 == 0 { 0 } else { v0 / v4 }) & 65535u64;
let v6: u64 = (if v4 == 0 { 0 } else { v0 % v4 }) & 65535u64;
let v7: u64 = v5 & 255u64;
let v8: u64 = v6 & 255u64;
let v9: u64 = v7 & 65535u64;
let v10: u64 = u64::from(v9<v5);
let v11: u64 = (!v10) & 1u64;
if v11==0{return Err(ExecError::Exception{eip:*eip,vector:0});}
let v12: u64 = v7 & 65535u64;
let v13: u64 = v8 & 65535u64;
let v14: u64 = 8u64;
let v15: u64 = (if v14>=16u64 {0}else{v13<<v14}) & 65535u64;
let v16: u64 = (v15 | v12) & 65535u64;
pending_registers.push((0,16,v16));
pending_flags &= !1u32; pending_undefined |= 1u32;
pending_flags &= !2048u32; pending_undefined |= 2048u32;
pending_flags &= !128u32; pending_undefined |= 128u32;
pending_flags &= !64u32; pending_undefined |= 64u32;
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_flags &= !4u32; pending_undefined |= 4u32;
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(255u64<<dst.1)&(255u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 255u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=255u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};
execute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|"invalid execution context")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
 }
mod i117 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,false)?;
if false && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 65535u64;
let v1: u64 = 1u64;
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_add(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (!v7) & 65535u64;
let v9: u64 = (v0 ^ v5) & 65535u64;
let v10: u64 = (v8 & v9) & 65535u64;
let v11: u64 = (v10 >> 15) & 1;
let v12: u64 = (v7 ^ v5) & 65535u64;
let v13: u64 = (v12 >> 4) & 1;
let v14: u64 = (v5 >> 15) & 1;
let v15: u64 = u64::from(v5 == 0);
let v16: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_dst = Some(v5);
// Preserve flag mask 0x1, including undefined status.
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v16 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v13 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v15 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v14 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v11 as u32) << 11);
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,None,0,None,None,None)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
let mut eip=0; let context=FlowContext{next_eip:0,displacement:0,displacement_bits:32,code_limit:u32::MAX};
execute_with_flow(regs,flags,undefined,&mut eip,dst,src,context).map_err(|_|"invalid execution context")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
#[inline(always)] pub fn execute_unpaged_register(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,_address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),0,None,Some(machine),None)}
 }
mod i118 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
#[inline(always)] fn execute_state(regs: &mut [u32;8], flags: &mut u32, undefined: &mut u32, eip: &mut u32, dst: (usize,u8), src: (usize,u8), context: FlowContext, mut ram: Option<&mut Ram>, address: u32, memory_context: Option<address::MemoryContext>, mut machine: Option<&mut address::SegmentState>, effective: Option<address::EffectiveAddress>) -> Result<(), ExecError> {
let w = 16u8;
if dst.0 >= 8 || src.0 >= 8 || dst.1 as u16 + w as u16 > 32 || src.1 as u16 + 16u16 > 32 { return Err(ExecError::InvalidBinding); }
context.validate(w,false)?;
if true && ram.is_none() { return Err(ExecError::InvalidContext); }
let mut pending_control: Option<(usize,u32)> = None;
let mut pending_table: Option<(usize,u16,u32)> = None;
let mut pending_system: Option<(usize,address::SystemSegment)> = None;
let mut pending_segment: Option<(usize,u16,address::SegmentCache)> = None;
let mut pending_transfer: Option<privilege::Transfer> = None;
let mut pending_stores: [Option<(u32,MemWidth,u32)>;1] = [None;1];let mut store_index=0;
let mut pending_eip = context.next_eip;
let mut pending_resume = false;
let mut pending_interrupt_shadow = false;
let mut pending_flags = *flags;
let mut pending_undefined = *undefined;
let mut pending_dst: Option<u64> = None;
let mut pending_src: Option<u64> = None;
let mut pending_registers: Vec<(usize,u8,u64)> = Vec::new();
// Preserve flag mask 0x0, including undefined status.
let v0: u64 = (regs[4] as u64) & 65535u64;
let v1: u64 = v0 & 4294967295u64;
let cache=machine.as_deref().ok_or(ExecError::InvalidContext)?.segments[2];let at=address::MemoryContext{segment:address::SegmentRegister::Ss,cache}.translate(v1 as u32,MemWidth::Word,false,*eip)?;let v2:u64=ram.as_deref().ok_or(ExecError::InvalidContext)?.load_staged(at,MemWidth::Word,&pending_stores[..store_index],&[]).map_err(|f|f.error(*eip))? as u64;
let v3: u64 = v2 & 4294967295u64;
let v4: u64 = 1u64;
if v4 != 0 { if v3 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v3 as u32; }
let v5: u64 = 2u64;
let v6: u64 = 0u64;
let v7: u64 = (v0.wrapping_add(v5)) & 65535u64;
let v8: u64 = (v7.wrapping_add(v6)) & 65535u64;
pending_registers.push((4,16,v8));
// Preserve flag mask 0x8d5, including undefined status.
if let(Some(a),Some(b))=(pending_dst,pending_src) { let overlap=(65535u64<<dst.1)&(65535u64<<src.1); if dst.0==src.0&&((a<<dst.1)^(b<<src.1))&overlap!=0 {return Err(ExecError::InvalidBinding);} }
for (addr,w,v) in pending_stores.into_iter().flatten() { ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.store(addr,w,v).map_err(|f|f.error(*eip))?; }
if let Some(value) = pending_dst { let m = 65535u64 << dst.1;
regs[dst.0] = (((regs[dst.0] as u64) & !m) | ((value << dst.1) & m)) as u32; }
if let Some(value)=pending_src {let m=65535u64<<src.1;regs[src.0]=(((regs[src.0] as u64)&!m)|((value<<src.1)&m)) as u32;}
for (reg,width,value) in pending_registers {let m=(1u64<<width)-1;regs[reg]=((regs[reg] as u64 & !m)|(value&m)) as u32;}
*flags = pending_flags;
*undefined = pending_undefined;
*eip = pending_eip;
if let Some(m)=machine.as_deref_mut(){if let Some((r,v))=pending_control{m.commit_control(r,v);}if let Some((t,l,b))=pending_table{m.tables[t]=(l,b);}if let Some((t,loaded))=pending_system{m.ldt_tr[t]=loaded;}if !pending_resume{m.retired();}if pending_interrupt_shadow{m.sti_shadow=1;}if let Some((reg,selector,cache))=pending_segment{m.selectors[reg]=selector;m.segments[reg]=cache;if reg==2{m.ss_shadow=1;}}if let Some(t)=pending_transfer{t.commit(regs,eip,m,ram.as_deref_mut().ok_or(ExecError::InvalidContext)?)?;}}
Ok(())
}
pub fn execute_with_flow(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext)->Result<(),ExecError>{
Err(ExecError::InvalidContext)
}
pub fn execute_with_memory(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:u32)->Result<(),ExecError>{
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),address,None,None,None)
}
pub fn execute_with_segments(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,address:address::AddressContext)->Result<(),ExecError>{
let (offset,memory_context)=address.resolve(regs)?;
execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,Some(memory_context),None,Some(address.effective))
}
#[inline(always)] pub fn execute_with_machine(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),context:FlowContext,ram:&mut Ram,mut address:Option<address::AddressContext>,machine:&mut address::SegmentState)->Result<(),ExecError>{
let (offset,memory)=if let Some(a)=address.as_mut(){a.segments=machine.segments;let(o,c)=a.resolve(regs)?;(o,Some(c))}else{(0,None)};
if false{machine.checkpoint_flags(*eip,*flags,*undefined);}
ram.set_paging(machine.ram_paging());
let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));
ram.set_paging(machine.ram_paging());
if result.is_err()&&false{if let Some((f,u))=machine.recover_flags(){*flags=f;*undefined=u;}}
result
}
pub fn execute_with_metadata(regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,dst:(usize,u8),src:(usize,u8))->Result<(), &'static str> {
Err("flow context required: use execute_with_flow")
}
// Compatibility helper: discards diagnostic undefined-flag metadata.
pub fn execute(regs: &mut [u32;8], flags: &mut u32, dst: (usize,u8), src: (usize,u8)) -> Result<(), &'static str> {
let mut undefined = 0; execute_with_metadata(regs,flags,&mut undefined,dst,src)
}
 }

#[inline(always)] fn block_100(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i100::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 259, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i103::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 262, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i106::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 265, displacement: 14, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_103(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i103::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 262, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i106::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 265, displacement: 14, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_106(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i106::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 265, displacement: 14, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
(1,Ok(()))
}
#[inline(always)] fn block_109(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i109::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 267, displacement: -5, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
(1,Ok(()))
}
#[inline(always)] fn block_10b(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i10b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,8) ,(2,8) ,FlowContext { next_eip: 272, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 274 }, override_segment: None, segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
(1,Ok(()))
}
#[inline(always)] fn block_110(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i110::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 274, displacement: 0, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
(1,Ok(()))
}
#[inline(always)] fn block_112(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i112::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 275, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i113::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(3,0) ,FlowContext { next_eip: 277, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i115::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(3,0) ,FlowContext { next_eip: 279, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i117::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 280, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i118::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 281, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
(5,Ok(()))
}
#[inline(always)] fn block_113(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i113::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(3,0) ,FlowContext { next_eip: 277, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i115::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(3,0) ,FlowContext { next_eip: 279, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i117::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 280, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i118::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 281, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_115(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i115::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(3,0) ,FlowContext { next_eip: 279, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i117::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 280, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i118::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 281, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_117(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i117::execute_unpaged_register(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 280, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i118::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 281, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_118(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i118::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 281, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
(1,Ok(()))
}

#[derive(Default)]
pub(crate) struct Backend { matched:Option<(std::sync::Arc<Vec<u8>>,bool)>, checks:std::collections::BTreeMap<u32,(std::rc::Rc<guest::Decoded>,bool)>,blocks:u64,block_instructions:u64 }
impl Backend {
 // The decode cache returns the same immutable Rc only while its image and
 // instruction bytes remain valid. Retain it so address reuse cannot turn a
 // different decode into a false match; a new decode is checked again.
 fn matches(&mut self,ip:u32,decoded:&std::rc::Rc<guest::Decoded>,check:impl FnOnce()->bool)->bool {
  if let Some((old,result))=self.checks.get(&ip) { if std::rc::Rc::ptr_eq(old,decoded) {return *result;} }
  let result=check();self.checks.insert(ip,(decoded.clone(),result));result
 }
}
impl dos::InstructionBackend for Backend {
 fn block_stats(&self)->(u64,u64){(self.blocks,self.block_instructions)}
 fn block(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState,budget:usize)->Option<(usize,Result<(),crate::ExecError>)>{
  if !self.matched.as_ref().is_some_and(|(bytes,_)|std::sync::Arc::ptr_eq(bytes,&active.bytes)){self.matched=Some((active.bytes.clone(),active.bytes.as_slice()==ORIGINAL));}
  if !self.matched.as_ref().unwrap().1{return None;}
  // Follow observed branches/calls/returns without returning to Dine after
  // each basic block. Every entry retains its full byte and context guard.
  // Cap the region so input and external interrupt checks remain bounded.
  let limit=budget.min(256);let mut completed=0;
  loop{
  let remaining=limit-completed;
  let result=match cpu.eip{
256 if remaining>=3 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(256).and_then(|at|ram.contiguous_slice(at,9)).is_some_and(|s|s==[185, 3, 0, 184, 0, 0, 232, 14, 0]) => Some(block_100(cpu,ram,machine)),
259 if remaining>=2 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(259).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[184, 0, 0, 232, 14, 0]) => Some(block_103(cpu,ram,machine)),
262 if remaining>=1 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(262).and_then(|at|ram.contiguous_slice(at,3)).is_some_and(|s|s==[232, 14, 0]) => Some(block_106(cpu,ram,machine)),
265 if remaining>=1 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(265).and_then(|at|ram.contiguous_slice(at,2)).is_some_and(|s|s==[226, 251]) => Some(block_109(cpu,ram,machine)),
267 if remaining>=1 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(267).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[198, 6, 18, 1, 65]) => Some(block_10b(cpu,ram,machine)),
272 if remaining>=1 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(272).and_then(|at|ram.contiguous_slice(at,2)).is_some_and(|s|s==[235, 0]) => Some(block_110(cpu,ram,machine)),
274 if remaining>=5 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(274).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[65, 49, 219, 246, 243, 64, 195]) => Some(block_112(cpu,ram,machine)),
275 if remaining>=4 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(275).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[49, 219, 246, 243, 64, 195]) => Some(block_113(cpu,ram,machine)),
277 if remaining>=3 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(277).and_then(|at|ram.contiguous_slice(at,4)).is_some_and(|s|s==[246, 243, 64, 195]) => Some(block_115(cpu,ram,machine)),
279 if remaining>=2 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(279).and_then(|at|ram.contiguous_slice(at,2)).is_some_and(|s|s==[64, 195]) => Some(block_117(cpu,ram,machine)),
280 if remaining>=1 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(280).and_then(|at|ram.contiguous_slice(at,1)).is_some_and(|s|s==[195]) => Some(block_118(cpu,ram,machine)),

   _=>None,
  };
  let Some((n,result))=result else{return if completed==0{None}else{Some((completed,Ok(())))};};
  self.blocks+=1;self.block_instructions+=n as u64;completed+=n;
  if result.is_err(){return Some((completed,result));}
  if n==0||completed==limit||machine.ss_shadow!=0||machine.sti_shadow!=0||machine.paging{return Some((completed,Ok(())));}
  }
 }
 fn execute(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,decoded:&std::rc::Rc<guest::Decoded>,cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->Option<Result<(),crate::flow::ExecError>> {
  if !self.matched.as_ref().is_some_and(|(bytes,_)|std::sync::Arc::ptr_eq(bytes,&active.bytes)){
   self.matched=Some((active.bytes.clone(),active.bytes.as_slice()==ORIGINAL));
  }
  if !self.matched.as_ref().unwrap().1{return None;}
  let guest::Operation::Pil{program,dst,src,flow,address}=&decoded.operation else{return None};
  let result=match cpu.eip {
256 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[185, 3, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 3, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 3, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i100::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
259 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[184, 0, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 0, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 0, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i103::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
262 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[232, 14, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadIp(0, true, 32), Truncate(1, 0, 16), ReadDisp(2, 16), Binary(3, Add, 1, 2), Extend(4, 3, 32), ReadGpr(5, 4, 16), Const(6, Value { bits: 2, width: 16 }), Binary(7, Sub, 5, 6), Extend(8, 7, 32), StoreSegment(8, 1, Word, 2), Const(9, Value { bits: 1, width: 1 }), BranchIf(4, 9), WriteGpr(4, 7, 16), PreserveFlags(2261), Retire], packed: [Token(ReadIp(0, true, 32)), Token(Truncate(1, 0, 16)), Token(ReadDisp(2, 16)), Token(Binary(3, Add, 1, 2)), Token(Extend(4, 3, 32)), Token(ReadGpr(5, 4, 16)), Token(Const(6, Value { bits: 2, width: 16 })), Token(Binary(7, Sub, 5, 6)), Token(Extend(8, 7, 32)), Token(StoreSegment(8, 1, Word, 2)), Token(Const(9, Value { bits: 1, width: 1 })), Token(BranchIf(4, 9)), Token(WriteGpr(4, 7, 16)), Token(Retire)], widths: [32, 16, 16, 16, 32, 16, 16, 16, 32, 1], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i106::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
265 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[226, 251]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadGpr(0, 1, 16), Const(1, Value { bits: 1, width: 16 }), Binary(2, Sub, 0, 1), WriteGpr(1, 2, 16), Zero(3, 2), Not(4, 3), Binary(5, Or, 4, 4), ReadIp(6, true, 32), Truncate(7, 6, 16), ReadDisp(8, 16), Binary(9, Add, 7, 8), Extend(10, 9, 32), BranchIf(10, 5), PreserveFlags(2261), Retire], packed: [Token(ReadGpr(0, 1, 16)), Token(Const(1, Value { bits: 1, width: 16 })), Token(Binary(2, Sub, 0, 1)), Token(WriteGpr(1, 2, 16)), Token(Zero(3, 2)), Token(Not(4, 3)), Token(Binary(5, Or, 4, 4)), Token(ReadIp(6, true, 32)), Token(Truncate(7, 6, 16)), Token(ReadDisp(8, 16)), Token(Binary(9, Add, 7, 8)), Token(Extend(10, 9, 32)), Token(BranchIf(10, 5)), Token(Retire)], widths: [16, 16, 16, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i109::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
267 if image.fetch_signature(ram,cpu.eip,5).ok().is_some_and(|s|s[..5]==[198, 6, 18, 1, 65]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Const(1, Value { bits: 65, width: 8 }), Store(0, 1, Byte), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Const(1, Value { bits: 65, width: 8 })), Token(Store(0, 1, Byte)), Token(Retire)], widths: [32, 8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i10b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
272 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[235, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadIp(0, true, 32), Truncate(1, 0, 16), ReadDisp(2, 16), Binary(3, Add, 1, 2), Extend(4, 3, 32), Const(5, Value { bits: 1, width: 1 }), BranchIf(4, 5), PreserveFlags(2261), Retire], packed: [RelJmp { w: 16 }, Token(Retire)], widths: [32, 16, 16, 16, 32, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i110::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
274 if image.fetch_signature(ram,cpu.eip,1).ok().is_some_and(|s|s[..1]==[65]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 1, width: 16 }), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Add, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Not(8, 7), Binary(9, Xor, 0, 5), Binary(10, And, 8, 9), Bit(11, 10, 15), Binary(12, Xor, 7, 5), Bit(13, 12, 4), Bit(14, 5, 15), Zero(15, 5), Parity(16, 5), Write(5), PreserveFlags(1), Flag(PF, 16), Flag(AF, 13), Flag(ZF, 15), Flag(SF, 14), Flag(OF, 11), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 1, width: 16 } }, AddLazy { a: 0, b: 1, r: 5, w: 16, write_cf: false }, Token(Write(5)), Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i112::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
275 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[49, 219]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Binary(2, Xor, 0, 1), Const(3, Value { bits: 0, width: 1 }), Bit(4, 2, 15), Zero(5, 2), Parity(6, 2), Write(2), Flag(CF, 3), Flag(OF, 3), UndefFlag(AF), Flag(SF, 4), Flag(ZF, 5), Flag(PF, 6), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, Token(Binary(2, Xor, 0, 1)), LogicLazy { r: 2, w: 16 }, Token(Write(2)), Token(Retire)], widths: [16, 16, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i113::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
277 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[246, 243]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadGpr(0, 0, 16), Read(1, Src, 8), Zero(2, 1), Not(3, 2), RaiseUnless(3, 0), Extend(4, 1, 16), Binary(5, Div, 0, 4), Binary(6, Rem, 0, 4), Truncate(7, 5, 8), Truncate(8, 6, 8), Extend(9, 7, 16), Less(10, 9, 5), Not(11, 10), RaiseUnless(11, 0), Extend(12, 7, 16), Extend(13, 8, 16), Const(14, Value { bits: 8, width: 16 }), Shift(15, Left, 13, 14), Binary(16, Or, 15, 12), WriteGpr(0, 16, 16), UndefFlag(CF), UndefFlag(OF), UndefFlag(SF), UndefFlag(ZF), UndefFlag(AF), UndefFlag(PF), Retire], packed: [Token(ReadGpr(0, 0, 16)), Token(Read(1, Src, 8)), Token(Zero(2, 1)), Token(Not(3, 2)), Token(RaiseUnless(3, 0)), Token(Extend(4, 1, 16)), Token(Binary(5, Div, 0, 4)), Token(Binary(6, Rem, 0, 4)), Token(Truncate(7, 5, 8)), Token(Truncate(8, 6, 8)), Token(Extend(9, 7, 16)), Token(Less(10, 9, 5)), Token(Not(11, 10)), Token(RaiseUnless(11, 0)), Token(Extend(12, 7, 16)), Token(Extend(13, 8, 16)), Token(Const(14, Value { bits: 8, width: 16 })), Token(Shift(15, Left, 13, 14)), Token(Binary(16, Or, 15, 12)), Token(WriteGpr(0, 16, 16)), Token(UndefFlag(CF)), Token(UndefFlag(OF)), Token(UndefFlag(SF)), Token(UndefFlag(ZF)), Token(UndefFlag(AF)), Token(UndefFlag(PF)), Token(Retire)], widths: [16, 8, 1, 1, 16, 16, 16, 8, 8, 16, 1, 1, 16, 16, 16, 16, 16], register_effects: 1, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i115::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
279 if image.fetch_signature(ram,cpu.eip,1).ok().is_some_and(|s|s[..1]==[64]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 1, width: 16 }), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Add, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Not(8, 7), Binary(9, Xor, 0, 5), Binary(10, And, 8, 9), Bit(11, 10, 15), Binary(12, Xor, 7, 5), Bit(13, 12, 4), Bit(14, 5, 15), Zero(15, 5), Parity(16, 5), Write(5), PreserveFlags(1), Flag(PF, 16), Flag(AF, 13), Flag(ZF, 15), Flag(SF, 14), Flag(OF, 11), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 1, width: 16 } }, AddLazy { a: 0, b: 1, r: 5, w: 16, write_cf: false }, Token(Write(5)), Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i117::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
280 if image.fetch_signature(ram,cpu.eip,1).ok().is_some_and(|s|s[..1]==[195]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadGpr(0, 4, 16), Extend(1, 0, 32), LoadSegment(2, 1, Word, 2), Extend(3, 2, 32), Const(4, Value { bits: 1, width: 1 }), BranchIf(3, 4), Const(5, Value { bits: 2, width: 16 }), Const(6, Value { bits: 0, width: 16 }), Binary(7, Add, 0, 5), Binary(8, Add, 7, 6), WriteGpr(4, 8, 16), PreserveFlags(2261), Retire], packed: [Token(ReadGpr(0, 4, 16)), Token(Extend(1, 0, 32)), Token(LoadSegment(2, 1, Word, 2)), Token(Extend(3, 2, 32)), Token(Const(4, Value { bits: 1, width: 1 })), Token(BranchIf(3, 4)), Token(Const(5, Value { bits: 2, width: 16 })), Token(Const(6, Value { bits: 0, width: 16 })), Token(Binary(7, Add, 0, 5)), Token(Binary(8, Add, 7, 6)), Token(WriteGpr(4, 8, 16)), Token(Retire)], widths: [16, 32, 16, 32, 1, 16, 16, 16, 16], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: false, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i118::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),

   _=>return None,
  };
  Some(result)
 }
}
#[cfg(test)]
mod guard_tests {
 use super::*;
 use crate::host_io::Output;
 use crate::dos::InstructionBackend;
 #[test]
 fn changed_code_or_fetch_window_prevents_block_entry() {
  let mut p=dos::load_with_drives(ORIGINAL,"TEST.COM",vec![dos::Source::Empty,dos::Source::Memory(dos::MemDir::default())],None).unwrap();
  let mut backend=Backend::default();let mut c=p.cpu.clone();let mut r=p.ram.clone();let mut m=p.image.segment_state();
  assert!(backend.block(&p.active_program,&p.image,&mut c,&mut r,&mut m,16).is_some(),"fixture must enter a compiled block");
  let mut narrowed=p.image.clone();narrowed.end=narrowed.start;
  let mut m=p.image.segment_state();
  assert!(backend.block(&p.active_program,&narrowed,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
  // A stale paged RAM view must not enter the unpaged register fast path.
  p.ram.set_paging(Some(crate::memory::Paging{directory:0,user:false}));
  assert!(backend.block(&p.active_program,&p.image,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
  p.ram.set_paging(None);
  let at=p.image.segments[1].base+p.cpu.eip;
  let old=p.ram.load(at,crate::MemWidth::Byte).unwrap();p.ram.store(at,crate::MemWidth::Byte,old^1).unwrap();
  assert!(backend.block(&p.active_program,&p.image,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
 }
 #[test]
 fn generated_blocks_preserve_state_across_small_budgets() {
  let mut a=dos::load(ORIGINAL,"TEST.COM").unwrap();let mut b=a.clone();let mut backend=Backend::default();
  for budget in [1,2,3,16,33,127,255,256,257,1000].into_iter().cycle().take(160){
   let mut x=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
   let mut y=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
   let left=dos::run_with_backend(a,budget,&mut x,None).unwrap();
   let right=dos::run_with_backend(b,budget,&mut y,Some(&mut backend)).unwrap();
   assert_eq!(left.loaded.cpu,right.loaded.cpu,"budget {budget}");
   assert_eq!(left.loaded.image,right.loaded.image);
   assert_eq!(left.loaded.ram.bytes(),right.loaded.ram.bytes());
   assert_eq!(left.loaded.ram.planar(),right.loaded.ram.planar());
   assert_eq!(left.stop,right.stop);assert_eq!(left.completed,right.completed);
   assert_eq!(x.bytes(),y.bytes());
   if !matches!(left.stop,dos::Stop::Budget){break;}
   a=left.loaded;b=right.loaded;
  }
 }
 #[test]
 fn cached_decode_reuses_guard_but_a_new_decode_rechecks_it() {
  let mut backend=Backend::default();
  let decoded=std::rc::Rc::new(guest::Decoded{label:String::new(),length:1,operation:guest::Operation::Halt});
  let calls=std::cell::Cell::new(0);
  assert!(backend.matches(7,&decoded,||{calls.set(calls.get()+1);true}));
  assert!(backend.matches(7,&decoded.clone(),||{calls.set(calls.get()+1);false}));
  assert_eq!(calls.get(),1);
  let changed=std::rc::Rc::new((*decoded).clone());
  assert!(!backend.matches(7,&changed,||{calls.set(calls.get()+1);false}));
  assert_eq!(calls.get(),2);
  assert!(!backend.matches(7,&changed,||panic!("cached failure was not reused")));
 }
}
