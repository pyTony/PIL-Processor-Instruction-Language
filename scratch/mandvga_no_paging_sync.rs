// Generated from observed PIL sites; unknown or changed instructions use the interpreter.
use crate::{address,dos,guest,Cpu,Ram,FlowContext};
use address::*;
use address::SegmentAccess::*;
use address::{EffectiveAddress::*,Address16::*,SegmentRegister::*};
const ORIGINAL:&[u8]=&[252, 184, 18, 0, 205, 16, 184, 0, 160, 142, 192, 186, 206, 3, 184, 5, 2, 239, 199, 6, 65, 2, 80, 236, 199, 6, 71, 2, 0, 0, 199, 6, 67, 2, 0, 220, 199, 6, 69, 2, 0, 0, 232, 128, 0, 162, 73, 2, 139, 30, 69, 2, 136, 217, 128, 225, 3, 208, 225, 180, 192, 210, 236, 176, 8, 186, 206, 3, 239, 139, 62, 71, 2, 209, 235, 209, 235, 1, 223, 38, 138, 5, 160, 73, 2, 38, 136, 5, 38, 138, 101, 80, 38, 136, 69, 80, 131, 6, 67, 2, 42, 255, 6, 69, 2, 129, 62, 69, 2, 64, 1, 114, 185, 131, 6, 65, 2, 42, 129, 6, 71, 2, 160, 0, 129, 62, 71, 2, 0, 150, 114, 154, 186, 206, 3, 184, 8, 255, 239, 184, 5, 0, 239, 180, 2, 48, 255, 186, 0, 29, 205, 16, 186, 28, 2, 180, 9, 205, 33, 48, 228, 205, 22, 184, 3, 0, 205, 16, 184, 0, 76, 205, 33, 49, 246, 49, 255, 189, 30, 0, 129, 254, 0, 32, 125, 73, 129, 254, 0, 224, 126, 67, 129, 255, 0, 32, 125, 61, 129, 255, 0, 224, 126, 55, 137, 240, 247, 238, 232, 62, 0, 137, 195, 137, 248, 247, 239, 232, 53, 0, 137, 217, 1, 193, 129, 249, 0, 64, 127, 29, 41, 195, 3, 30, 67, 2, 137, 240, 247, 239, 232, 30, 0, 209, 224, 3, 6, 65, 2, 137, 199, 137, 222, 77, 117, 180, 48, 192, 195, 184, 30, 0, 41, 232, 179, 15, 246, 243, 136, 224, 254, 192, 195, 177, 12, 211, 232, 177, 4, 211, 226, 9, 208, 195, 86, 71, 65, 32, 54, 52, 48, 120, 52, 56, 48, 32, 77, 97, 110, 100, 101, 108, 98, 114, 111, 116, 32, 32, 97, 110, 121, 32, 107, 101, 121, 61, 101, 120, 105, 116, 36, 0, 0, 0, 0, 0, 0, 0, 0, 0];
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
let v0: u64 = 0u64;
pending_undefined &= !1024u32; pending_flags = (pending_flags & !1024u32) | ((v0 as u32) << 10);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i101 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 18u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i106 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 40960u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i109 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
let (selector,cache,accessed)={let m=machine.as_deref().ok_or(ExecError::InvalidContext)?;let r=ram.as_deref();let e=*eip;m.load_segment(0,v0 as u16,e,&mut |at|privilege::read_descriptor(r.ok_or(ExecError::InvalidContext)?,at,e))?};if let Some((at,v))=accessed{ram.as_deref().ok_or(ExecError::InvalidContext)?.check_write(at,MemWidth::Byte).map_err(|f|f.error(*eip))?;pending_stores[store_index]=Some((at,MemWidth::Byte,u32::from(v)));store_index+=1;}pending_segment=Some((0,selector,cache));
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i10b {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 974u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i10e {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 517u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i112 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let v1: u64 = 60496u64;
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Word).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Word,v1 as u32));store_index+=1;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
let v0: u64 = address as u64;
let v1: u64 = 0u64;
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Word).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Word,v1 as u32));store_index+=1;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i11e {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let v1: u64 = 56320u64;
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Word).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Word,v1 as u32));store_index+=1;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i124 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let v1: u64 = 0u64;
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Word).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Word,v1 as u32));store_index+=1;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i12a {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i12d {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i130 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
pending_dst = Some(v1);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i134 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i136 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 255u64;
let v1: u64 = 3u64;
let v2: u64 = (v0 & v1) & 255u64;
let v3: u64 = 0u64;
let v4: u64 = (v2 >> 7) & 1;
let v5: u64 = u64::from(v2 == 0);
let v6: u64 = u64::from((v2 as u8).count_ones() % 2 == 0);
pending_dst = Some(v2);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v3 as u32) << 0);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v3 as u32) << 11);
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v4 as u32) << 7);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v5 as u32) << 6);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v6 as u32) << 2);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i139 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 255u64;
let v1: u64 = 1u64;
let v2: u64 = 31u64;
let v3: u64 = (v1 & v2) & 255u64;
let v4: u64 = 1u64;
let v5: u64 = 8u64;
let v6: u64 = 1u64;
let v7: u64 = 0u64;
let v8: u64 = u64::from(v3 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = (v3 ^ v4) & 255u64;
let v11: u64 = u64::from(v10 == 0);
let v12: u64 = (v3.wrapping_sub(v4)) & 255u64;
let v13: u64 = (if v3>=8u64 {0}else{v0<<v3}) & 255u64;
let v14: u64 = (if v12>=8u64 {0}else{v0<<v12}) & 255u64;
let v15: u64 = (v14 >> 7) & 1;
let v16: u64 = u64::from(v3<v5);
let v17: u64 = (v13 >> 7) & 1;
let v18: u64 = u64::from(v13 == 0);
let v19: u64 = u64::from((v13 as u8).count_ones() % 2 == 0);
let v20: u64 = (v17 ^ v15) & 1u64;
pending_dst = Some(v13);
if v9!=0 {pending_flags &= !1u32; if v16!=0 {pending_flags |= (v15 as u32)<<0;pending_undefined &= !1u32;}else{pending_undefined |= 1u32;}}
if v9!=0 {pending_flags &= !2048u32; if v11!=0 {pending_flags |= (v20 as u32)<<11;pending_undefined &= !2048u32;}else{pending_undefined |= 2048u32;}}
if v9!=0 {pending_flags &= !16u32; if v7!=0 {pending_flags |= (v7 as u32)<<4;pending_undefined &= !16u32;}else{pending_undefined |= 16u32;}}
if v9!=0 {pending_flags &= !128u32; if v6!=0 {pending_flags |= (v17 as u32)<<7;pending_undefined &= !128u32;}else{pending_undefined |= 128u32;}}
if v9!=0 {pending_flags &= !64u32; if v6!=0 {pending_flags |= (v18 as u32)<<6;pending_undefined &= !64u32;}else{pending_undefined |= 64u32;}}
if v9!=0 {pending_flags &= !4u32; if v6!=0 {pending_flags |= (v19 as u32)<<2;pending_undefined &= !4u32;}else{pending_undefined |= 4u32;}}
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i13b {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 192u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i13d {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 255u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
let v2: u64 = 31u64;
let v3: u64 = (v1 & v2) & 255u64;
let v4: u64 = 1u64;
let v5: u64 = 8u64;
let v6: u64 = 1u64;
let v7: u64 = 0u64;
let v8: u64 = u64::from(v3 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = (v3 ^ v4) & 255u64;
let v11: u64 = u64::from(v10 == 0);
let v12: u64 = (v3.wrapping_sub(v4)) & 255u64;
let v13: u64 = (if v3>=8u64 {0}else{v0>>v3}) & 255u64;
let v14: u64 = (if v12>=8u64 {0}else{v0>>v12}) & 255u64;
let v15: u64 = (v14 >> 0) & 1;
let v16: u64 = u64::from(v3<v5);
let v17: u64 = (v13 >> 7) & 1;
let v18: u64 = u64::from(v13 == 0);
let v19: u64 = u64::from((v13 as u8).count_ones() % 2 == 0);
let v20: u64 = (v0 >> 7) & 1;
pending_dst = Some(v13);
if v9!=0 {pending_flags &= !1u32; if v16!=0 {pending_flags |= (v15 as u32)<<0;pending_undefined &= !1u32;}else{pending_undefined |= 1u32;}}
if v9!=0 {pending_flags &= !2048u32; if v11!=0 {pending_flags |= (v20 as u32)<<11;pending_undefined &= !2048u32;}else{pending_undefined |= 2048u32;}}
if v9!=0 {pending_flags &= !16u32; if v7!=0 {pending_flags |= (v7 as u32)<<4;pending_undefined &= !16u32;}else{pending_undefined |= 16u32;}}
if v9!=0 {pending_flags &= !128u32; if v6!=0 {pending_flags |= (v17 as u32)<<7;pending_undefined &= !128u32;}else{pending_undefined |= 128u32;}}
if v9!=0 {pending_flags &= !64u32; if v6!=0 {pending_flags |= (v18 as u32)<<6;pending_undefined &= !64u32;}else{pending_undefined |= 64u32;}}
if v9!=0 {pending_flags &= !4u32; if v6!=0 {pending_flags |= (v19 as u32)<<2;pending_undefined &= !4u32;}else{pending_undefined |= 4u32;}}
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i13f {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 8u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i141 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 974u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i145 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
pending_dst = Some(v1);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i149 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v2: u64 = 31u64;
let v3: u64 = (v1 & v2) & 65535u64;
let v4: u64 = 1u64;
let v5: u64 = 16u64;
let v6: u64 = 1u64;
let v7: u64 = 0u64;
let v8: u64 = u64::from(v3 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = (v3 ^ v4) & 65535u64;
let v11: u64 = u64::from(v10 == 0);
let v12: u64 = (v3.wrapping_sub(v4)) & 65535u64;
let v13: u64 = (if v3>=16u64 {0}else{v0>>v3}) & 65535u64;
let v14: u64 = (if v12>=16u64 {0}else{v0>>v12}) & 65535u64;
let v15: u64 = (v14 >> 0) & 1;
let v16: u64 = u64::from(v3<v5);
let v17: u64 = (v13 >> 15) & 1;
let v18: u64 = u64::from(v13 == 0);
let v19: u64 = u64::from((v13 as u8).count_ones() % 2 == 0);
let v20: u64 = (v0 >> 15) & 1;
pending_dst = Some(v13);
if v9!=0 {pending_flags &= !1u32; if v16!=0 {pending_flags |= (v15 as u32)<<0;pending_undefined &= !1u32;}else{pending_undefined |= 1u32;}}
if v9!=0 {pending_flags &= !2048u32; if v11!=0 {pending_flags |= (v20 as u32)<<11;pending_undefined &= !2048u32;}else{pending_undefined |= 2048u32;}}
if v9!=0 {pending_flags &= !16u32; if v7!=0 {pending_flags |= (v7 as u32)<<4;pending_undefined &= !16u32;}else{pending_undefined |= 16u32;}}
if v9!=0 {pending_flags &= !128u32; if v6!=0 {pending_flags |= (v17 as u32)<<7;pending_undefined &= !128u32;}else{pending_undefined |= 128u32;}}
if v9!=0 {pending_flags &= !64u32; if v6!=0 {pending_flags |= (v18 as u32)<<6;pending_undefined &= !64u32;}else{pending_undefined |= 64u32;}}
if v9!=0 {pending_flags &= !4u32; if v6!=0 {pending_flags |= (v19 as u32)<<2;pending_undefined &= !4u32;}else{pending_undefined |= 4u32;}}
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i14b {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v2: u64 = 31u64;
let v3: u64 = (v1 & v2) & 65535u64;
let v4: u64 = 1u64;
let v5: u64 = 16u64;
let v6: u64 = 1u64;
let v7: u64 = 0u64;
let v8: u64 = u64::from(v3 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = (v3 ^ v4) & 65535u64;
let v11: u64 = u64::from(v10 == 0);
let v12: u64 = (v3.wrapping_sub(v4)) & 65535u64;
let v13: u64 = (if v3>=16u64 {0}else{v0>>v3}) & 65535u64;
let v14: u64 = (if v12>=16u64 {0}else{v0>>v12}) & 65535u64;
let v15: u64 = (v14 >> 0) & 1;
let v16: u64 = u64::from(v3<v5);
let v17: u64 = (v13 >> 15) & 1;
let v18: u64 = u64::from(v13 == 0);
let v19: u64 = u64::from((v13 as u8).count_ones() % 2 == 0);
let v20: u64 = (v0 >> 15) & 1;
pending_dst = Some(v13);
if v9!=0 {pending_flags &= !1u32; if v16!=0 {pending_flags |= (v15 as u32)<<0;pending_undefined &= !1u32;}else{pending_undefined |= 1u32;}}
if v9!=0 {pending_flags &= !2048u32; if v11!=0 {pending_flags |= (v20 as u32)<<11;pending_undefined &= !2048u32;}else{pending_undefined |= 2048u32;}}
if v9!=0 {pending_flags &= !16u32; if v7!=0 {pending_flags |= (v7 as u32)<<4;pending_undefined &= !16u32;}else{pending_undefined |= 16u32;}}
if v9!=0 {pending_flags &= !128u32; if v6!=0 {pending_flags |= (v17 as u32)<<7;pending_undefined &= !128u32;}else{pending_undefined |= 128u32;}}
if v9!=0 {pending_flags &= !64u32; if v6!=0 {pending_flags |= (v18 as u32)<<6;pending_undefined &= !64u32;}else{pending_undefined |= 64u32;}}
if v9!=0 {pending_flags &= !4u32; if v6!=0 {pending_flags |= (v19 as u32)<<2;pending_undefined &= !4u32;}else{pending_undefined |= 4u32;}}
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i14d {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i14f {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Byte,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Byte).map_err(|f|f.error(*eip))? as u64;
pending_dst = Some(v1);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i152 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Byte,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Byte).map_err(|f|f.error(*eip))? as u64;
pending_dst = Some(v1);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i155 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i158 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Byte,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Byte).map_err(|f|f.error(*eip))? as u64;
pending_dst = Some(v1);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i15c {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i160 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
let v2: u64 = 42u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = v2 & 131071u64;
let v5: u64 = (v3.wrapping_add(v4)) & 131071u64;
let v6: u64 = v5 & 65535u64;
let v7: u64 = (v5 >> 16) & 1;
let v8: u64 = (v1 ^ v2) & 65535u64;
let v9: u64 = (!v8) & 65535u64;
let v10: u64 = (v1 ^ v6) & 65535u64;
let v11: u64 = (v9 & v10) & 65535u64;
let v12: u64 = (v11 >> 15) & 1;
let v13: u64 = (v8 ^ v6) & 65535u64;
let v14: u64 = (v13 >> 4) & 1;
let v15: u64 = (v6 >> 15) & 1;
let v16: u64 = u64::from(v6 == 0);
let v17: u64 = u64::from((v6 as u8).count_ones() % 2 == 0);
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Word).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Word,v6 as u32));store_index+=1;
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v7 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v17 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v14 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v16 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v15 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v12 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i165 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
let v2: u64 = 1u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = v2 & 131071u64;
let v5: u64 = (v3.wrapping_add(v4)) & 131071u64;
let v6: u64 = v5 & 65535u64;
let v7: u64 = (v5 >> 16) & 1;
let v8: u64 = (v1 ^ v2) & 65535u64;
let v9: u64 = (!v8) & 65535u64;
let v10: u64 = (v1 ^ v6) & 65535u64;
let v11: u64 = (v9 & v10) & 65535u64;
let v12: u64 = (v11 >> 15) & 1;
let v13: u64 = (v8 ^ v6) & 65535u64;
let v14: u64 = (v13 >> 4) & 1;
let v15: u64 = (v6 >> 15) & 1;
let v16: u64 = u64::from(v6 == 0);
let v17: u64 = u64::from((v6 as u8).count_ones() % 2 == 0);
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Word).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Word,v6 as u32));store_index+=1;
// Preserve flag mask 0x1, including undefined status.
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v17 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v14 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v16 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v15 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v12 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i169 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
let v2: u64 = 320u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = v2 & 131071u64;
let v5: u64 = (v3.wrapping_sub(v4)) & 131071u64;
let v6: u64 = v5 & 65535u64;
let v7: u64 = (v5 >> 16) & 1;
let v8: u64 = (v1 ^ v2) & 65535u64;
let v9: u64 = (v1 ^ v6) & 65535u64;
let v10: u64 = (v8 & v9) & 65535u64;
let v11: u64 = (v10 >> 15) & 1;
let v12: u64 = (v8 ^ v6) & 65535u64;
let v13: u64 = (v12 >> 4) & 1;
let v14: u64 = (v6 >> 15) & 1;
let v15: u64 = u64::from(v6 == 0);
let v16: u64 = u64::from((v6 as u8).count_ones() % 2 == 0);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v7 as u32) << 0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i16f {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((*flags >> 11) & 1) as u64;
let v1: u64 = (!v0) & 1u64;
let v2: u64 = ((*flags >> 0) & 1) as u64;
let v3: u64 = (!v2) & 1u64;
let v4: u64 = ((*flags >> 6) & 1) as u64;
let v5: u64 = (!v4) & 1u64;
let v6: u64 = (v2 | v4) & 1u64;
let v7: u64 = (!v6) & 1u64;
let v8: u64 = ((*flags >> 7) & 1) as u64;
let v9: u64 = (!v8) & 1u64;
let v10: u64 = ((*flags >> 2) & 1) as u64;
let v11: u64 = (!v10) & 1u64;
let v12: u64 = (v8 ^ v0) & 1u64;
let v13: u64 = (!v12) & 1u64;
let v14: u64 = (v12 | v4) & 1u64;
let v15: u64 = (!v14) & 1u64;
// Preserve flag mask 0x0, including undefined status.
let v16: u64 = (context.next_eip as u64) & 4294967295u64;
let v17: u64 = v16 & 65535u64;
let v18: u64 = (context.displacement as u32 as u64) & 65535u64;
let v19: u64 = (v17.wrapping_add(v18)) & 65535u64;
let v20: u64 = v19 & 4294967295u64;
if v2 != 0 { if v20 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v20 as u32; }
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i171 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
let v2: u64 = 42u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = v2 & 131071u64;
let v5: u64 = (v3.wrapping_add(v4)) & 131071u64;
let v6: u64 = v5 & 65535u64;
let v7: u64 = (v5 >> 16) & 1;
let v8: u64 = (v1 ^ v2) & 65535u64;
let v9: u64 = (!v8) & 65535u64;
let v10: u64 = (v1 ^ v6) & 65535u64;
let v11: u64 = (v9 & v10) & 65535u64;
let v12: u64 = (v11 >> 15) & 1;
let v13: u64 = (v8 ^ v6) & 65535u64;
let v14: u64 = (v13 >> 4) & 1;
let v15: u64 = (v6 >> 15) & 1;
let v16: u64 = u64::from(v6 == 0);
let v17: u64 = u64::from((v6 as u8).count_ones() % 2 == 0);
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Word).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Word,v6 as u32));store_index+=1;
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v7 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v17 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v14 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v16 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v15 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v12 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i176 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
let v2: u64 = 160u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = v2 & 131071u64;
let v5: u64 = (v3.wrapping_add(v4)) & 131071u64;
let v6: u64 = v5 & 65535u64;
let v7: u64 = (v5 >> 16) & 1;
let v8: u64 = (v1 ^ v2) & 65535u64;
let v9: u64 = (!v8) & 65535u64;
let v10: u64 = (v1 ^ v6) & 65535u64;
let v11: u64 = (v9 & v10) & 65535u64;
let v12: u64 = (v11 >> 15) & 1;
let v13: u64 = (v8 ^ v6) & 65535u64;
let v14: u64 = (v13 >> 4) & 1;
let v15: u64 = (v6 >> 15) & 1;
let v16: u64 = u64::from(v6 == 0);
let v17: u64 = u64::from((v6 as u8).count_ones() % 2 == 0);
let store_addr=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,true,*eip)?,None=>v0 as u32}; ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.check_write(store_addr,MemWidth::Word).map_err(|f|f.error(*eip))?; pending_stores[store_index]=Some((store_addr,MemWidth::Word,v6 as u32));store_index+=1;
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v7 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v17 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v14 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v16 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v15 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v12 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i17c {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = address as u64;
let addr1=match memory_context {Some(c)=>c.translate(v0 as u32,MemWidth::Word,false,*eip)?,None=>v0 as u32}; let v1: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr1,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
let v2: u64 = 38400u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = v2 & 131071u64;
let v5: u64 = (v3.wrapping_sub(v4)) & 131071u64;
let v6: u64 = v5 & 65535u64;
let v7: u64 = (v5 >> 16) & 1;
let v8: u64 = (v1 ^ v2) & 65535u64;
let v9: u64 = (v1 ^ v6) & 65535u64;
let v10: u64 = (v8 & v9) & 65535u64;
let v11: u64 = (v10 >> 15) & 1;
let v12: u64 = (v8 ^ v6) & 65535u64;
let v13: u64 = (v12 >> 4) & 1;
let v14: u64 = (v6 >> 15) & 1;
let v15: u64 = u64::from(v6 == 0);
let v16: u64 = u64::from((v6 as u8).count_ones() % 2 == 0);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v7 as u32) << 0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i182 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((*flags >> 11) & 1) as u64;
let v1: u64 = (!v0) & 1u64;
let v2: u64 = ((*flags >> 0) & 1) as u64;
let v3: u64 = (!v2) & 1u64;
let v4: u64 = ((*flags >> 6) & 1) as u64;
let v5: u64 = (!v4) & 1u64;
let v6: u64 = (v2 | v4) & 1u64;
let v7: u64 = (!v6) & 1u64;
let v8: u64 = ((*flags >> 7) & 1) as u64;
let v9: u64 = (!v8) & 1u64;
let v10: u64 = ((*flags >> 2) & 1) as u64;
let v11: u64 = (!v10) & 1u64;
let v12: u64 = (v8 ^ v0) & 1u64;
let v13: u64 = (!v12) & 1u64;
let v14: u64 = (v12 | v4) & 1u64;
let v15: u64 = (!v14) & 1u64;
// Preserve flag mask 0x0, including undefined status.
let v16: u64 = (context.next_eip as u64) & 4294967295u64;
let v17: u64 = v16 & 65535u64;
let v18: u64 = (context.displacement as u32 as u64) & 65535u64;
let v19: u64 = (v17.wrapping_add(v18)) & 65535u64;
let v20: u64 = v19 & 4294967295u64;
if v2 != 0 { if v20 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v20 as u32; }
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i184 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 974u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i187 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 65288u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i18b {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 5u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i18f {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 2u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i191 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 255u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
let v2: u64 = (v0 ^ v1) & 255u64;
let v3: u64 = 0u64;
let v4: u64 = (v2 >> 7) & 1;
let v5: u64 = u64::from(v2 == 0);
let v6: u64 = u64::from((v2 as u8).count_ones() % 2 == 0);
pending_dst = Some(v2);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v3 as u32) << 0);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v3 as u32) << 11);
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v4 as u32) << 7);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v5 as u32) << 6);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v6 as u32) << 2);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i193 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 7424u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i198 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 540u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i19b {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 9u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i19f {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 255u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
let v2: u64 = (v0 ^ v1) & 255u64;
let v3: u64 = 0u64;
let v4: u64 = (v2 >> 7) & 1;
let v5: u64 = u64::from(v2 == 0);
let v6: u64 = u64::from((v2 as u8).count_ones() % 2 == 0);
pending_dst = Some(v2);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v3 as u32) << 0);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v3 as u32) << 11);
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v4 as u32) << 7);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v5 as u32) << 6);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v6 as u32) << 2);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1ad {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1af {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1b1 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 30u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1b4 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v1: u64 = 8192u64;
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_sub(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (v0 ^ v5) & 65535u64;
let v9: u64 = (v7 & v8) & 65535u64;
let v10: u64 = (v9 >> 15) & 1;
let v11: u64 = (v7 ^ v5) & 65535u64;
let v12: u64 = (v11 >> 4) & 1;
let v13: u64 = (v5 >> 15) & 1;
let v14: u64 = u64::from(v5 == 0);
let v15: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v15 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v12 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v14 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v13 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v10 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1b8 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((*flags >> 11) & 1) as u64;
let v1: u64 = (!v0) & 1u64;
let v2: u64 = ((*flags >> 0) & 1) as u64;
let v3: u64 = (!v2) & 1u64;
let v4: u64 = ((*flags >> 6) & 1) as u64;
let v5: u64 = (!v4) & 1u64;
let v6: u64 = (v2 | v4) & 1u64;
let v7: u64 = (!v6) & 1u64;
let v8: u64 = ((*flags >> 7) & 1) as u64;
let v9: u64 = (!v8) & 1u64;
let v10: u64 = ((*flags >> 2) & 1) as u64;
let v11: u64 = (!v10) & 1u64;
let v12: u64 = (v8 ^ v0) & 1u64;
let v13: u64 = (!v12) & 1u64;
let v14: u64 = (v12 | v4) & 1u64;
let v15: u64 = (!v14) & 1u64;
// Preserve flag mask 0x0, including undefined status.
let v16: u64 = (context.next_eip as u64) & 4294967295u64;
let v17: u64 = v16 & 65535u64;
let v18: u64 = (context.displacement as u32 as u64) & 65535u64;
let v19: u64 = (v17.wrapping_add(v18)) & 65535u64;
let v20: u64 = v19 & 4294967295u64;
if v13 != 0 { if v20 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v20 as u32; }
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1ba {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v1: u64 = 57344u64;
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_sub(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (v0 ^ v5) & 65535u64;
let v9: u64 = (v7 & v8) & 65535u64;
let v10: u64 = (v9 >> 15) & 1;
let v11: u64 = (v7 ^ v5) & 65535u64;
let v12: u64 = (v11 >> 4) & 1;
let v13: u64 = (v5 >> 15) & 1;
let v14: u64 = u64::from(v5 == 0);
let v15: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v15 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v12 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v14 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v13 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v10 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1be {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((*flags >> 11) & 1) as u64;
let v1: u64 = (!v0) & 1u64;
let v2: u64 = ((*flags >> 0) & 1) as u64;
let v3: u64 = (!v2) & 1u64;
let v4: u64 = ((*flags >> 6) & 1) as u64;
let v5: u64 = (!v4) & 1u64;
let v6: u64 = (v2 | v4) & 1u64;
let v7: u64 = (!v6) & 1u64;
let v8: u64 = ((*flags >> 7) & 1) as u64;
let v9: u64 = (!v8) & 1u64;
let v10: u64 = ((*flags >> 2) & 1) as u64;
let v11: u64 = (!v10) & 1u64;
let v12: u64 = (v8 ^ v0) & 1u64;
let v13: u64 = (!v12) & 1u64;
let v14: u64 = (v12 | v4) & 1u64;
let v15: u64 = (!v14) & 1u64;
// Preserve flag mask 0x0, including undefined status.
let v16: u64 = (context.next_eip as u64) & 4294967295u64;
let v17: u64 = v16 & 65535u64;
let v18: u64 = (context.displacement as u32 as u64) & 65535u64;
let v19: u64 = (v17.wrapping_add(v18)) & 65535u64;
let v20: u64 = v19 & 4294967295u64;
if v14 != 0 { if v20 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v20 as u32; }
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1c0 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v1: u64 = 8192u64;
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_sub(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (v0 ^ v5) & 65535u64;
let v9: u64 = (v7 & v8) & 65535u64;
let v10: u64 = (v9 >> 15) & 1;
let v11: u64 = (v7 ^ v5) & 65535u64;
let v12: u64 = (v11 >> 4) & 1;
let v13: u64 = (v5 >> 15) & 1;
let v14: u64 = u64::from(v5 == 0);
let v15: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v15 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v12 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v14 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v13 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v10 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1c4 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((*flags >> 11) & 1) as u64;
let v1: u64 = (!v0) & 1u64;
let v2: u64 = ((*flags >> 0) & 1) as u64;
let v3: u64 = (!v2) & 1u64;
let v4: u64 = ((*flags >> 6) & 1) as u64;
let v5: u64 = (!v4) & 1u64;
let v6: u64 = (v2 | v4) & 1u64;
let v7: u64 = (!v6) & 1u64;
let v8: u64 = ((*flags >> 7) & 1) as u64;
let v9: u64 = (!v8) & 1u64;
let v10: u64 = ((*flags >> 2) & 1) as u64;
let v11: u64 = (!v10) & 1u64;
let v12: u64 = (v8 ^ v0) & 1u64;
let v13: u64 = (!v12) & 1u64;
let v14: u64 = (v12 | v4) & 1u64;
let v15: u64 = (!v14) & 1u64;
// Preserve flag mask 0x0, including undefined status.
let v16: u64 = (context.next_eip as u64) & 4294967295u64;
let v17: u64 = v16 & 65535u64;
let v18: u64 = (context.displacement as u32 as u64) & 65535u64;
let v19: u64 = (v17.wrapping_add(v18)) & 65535u64;
let v20: u64 = v19 & 4294967295u64;
if v13 != 0 { if v20 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v20 as u32; }
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1c6 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v1: u64 = 57344u64;
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_sub(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (v0 ^ v5) & 65535u64;
let v9: u64 = (v7 & v8) & 65535u64;
let v10: u64 = (v9 >> 15) & 1;
let v11: u64 = (v7 ^ v5) & 65535u64;
let v12: u64 = (v11 >> 4) & 1;
let v13: u64 = (v5 >> 15) & 1;
let v14: u64 = u64::from(v5 == 0);
let v15: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v15 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v12 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v14 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v13 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v10 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1ca {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((*flags >> 11) & 1) as u64;
let v1: u64 = (!v0) & 1u64;
let v2: u64 = ((*flags >> 0) & 1) as u64;
let v3: u64 = (!v2) & 1u64;
let v4: u64 = ((*flags >> 6) & 1) as u64;
let v5: u64 = (!v4) & 1u64;
let v6: u64 = (v2 | v4) & 1u64;
let v7: u64 = (!v6) & 1u64;
let v8: u64 = ((*flags >> 7) & 1) as u64;
let v9: u64 = (!v8) & 1u64;
let v10: u64 = ((*flags >> 2) & 1) as u64;
let v11: u64 = (!v10) & 1u64;
let v12: u64 = (v8 ^ v0) & 1u64;
let v13: u64 = (!v12) & 1u64;
let v14: u64 = (v12 | v4) & 1u64;
let v15: u64 = (!v14) & 1u64;
// Preserve flag mask 0x0, including undefined status.
let v16: u64 = (context.next_eip as u64) & 4294967295u64;
let v17: u64 = v16 & 65535u64;
let v18: u64 = (context.displacement as u32 as u64) & 65535u64;
let v19: u64 = (v17.wrapping_add(v18)) & 65535u64;
let v20: u64 = v19 & 4294967295u64;
if v14 != 0 { if v20 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v20 as u32; }
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1cc {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1ce {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = (regs[0] as u64) & 65535u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
let v2: u64 = (if v0 & 32768u64 != 0 {v0 | !65535u64} else {v0}) & 4294967295u64;
let v3: u64 = (if v1 & 32768u64 != 0 {v1 | !65535u64} else {v1}) & 4294967295u64;
let v4: u64 = (v2.wrapping_mul(v3)) & 4294967295u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (if v5 & 32768u64 != 0 {v5 | !65535u64} else {v5}) & 4294967295u64;
let v7: u64 = (v4 ^ v6) & 4294967295u64;
let v8: u64 = u64::from(v7 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = 16u64;
let v11: u64 = (if v10>=32u64 {0}else{v4>>v10}) & 4294967295u64;
let v12: u64 = v11 & 65535u64;
pending_registers.push((0,16,v5));
pending_registers.push((2,16,v12));
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v9 as u32) << 0);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v9 as u32) << 11);
pending_flags &= !128u32; pending_undefined |= 128u32;
pending_flags &= !64u32; pending_undefined |= 64u32;
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_flags &= !4u32; pending_undefined |= 4u32;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1d0 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1d3 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1d5 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1d7 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = (regs[0] as u64) & 65535u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
let v2: u64 = (if v0 & 32768u64 != 0 {v0 | !65535u64} else {v0}) & 4294967295u64;
let v3: u64 = (if v1 & 32768u64 != 0 {v1 | !65535u64} else {v1}) & 4294967295u64;
let v4: u64 = (v2.wrapping_mul(v3)) & 4294967295u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (if v5 & 32768u64 != 0 {v5 | !65535u64} else {v5}) & 4294967295u64;
let v7: u64 = (v4 ^ v6) & 4294967295u64;
let v8: u64 = u64::from(v7 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = 16u64;
let v11: u64 = (if v10>=32u64 {0}else{v4>>v10}) & 4294967295u64;
let v12: u64 = v11 & 65535u64;
pending_registers.push((0,16,v5));
pending_registers.push((2,16,v12));
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v9 as u32) << 0);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v9 as u32) << 11);
pending_flags &= !128u32; pending_undefined |= 128u32;
pending_flags &= !64u32; pending_undefined |= 64u32;
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_flags &= !4u32; pending_undefined |= 4u32;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1d9 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1dc {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1de {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1e0 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v1: u64 = 16384u64;
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_sub(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (v0 ^ v5) & 65535u64;
let v9: u64 = (v7 & v8) & 65535u64;
let v10: u64 = (v9 >> 15) & 1;
let v11: u64 = (v7 ^ v5) & 65535u64;
let v12: u64 = (v11 >> 4) & 1;
let v13: u64 = (v5 >> 15) & 1;
let v14: u64 = u64::from(v5 == 0);
let v15: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v15 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v12 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v14 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v13 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v10 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1e4 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((*flags >> 11) & 1) as u64;
let v1: u64 = (!v0) & 1u64;
let v2: u64 = ((*flags >> 0) & 1) as u64;
let v3: u64 = (!v2) & 1u64;
let v4: u64 = ((*flags >> 6) & 1) as u64;
let v5: u64 = (!v4) & 1u64;
let v6: u64 = (v2 | v4) & 1u64;
let v7: u64 = (!v6) & 1u64;
let v8: u64 = ((*flags >> 7) & 1) as u64;
let v9: u64 = (!v8) & 1u64;
let v10: u64 = ((*flags >> 2) & 1) as u64;
let v11: u64 = (!v10) & 1u64;
let v12: u64 = (v8 ^ v0) & 1u64;
let v13: u64 = (!v12) & 1u64;
let v14: u64 = (v12 | v4) & 1u64;
let v15: u64 = (!v14) & 1u64;
// Preserve flag mask 0x0, including undefined status.
let v16: u64 = (context.next_eip as u64) & 4294967295u64;
let v17: u64 = v16 & 65535u64;
let v18: u64 = (context.displacement as u32 as u64) & 65535u64;
let v19: u64 = (v17.wrapping_add(v18)) & 65535u64;
let v20: u64 = v19 & 4294967295u64;
if v15 != 0 { if v20 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v20 as u32; }
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1e6 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_sub(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (v0 ^ v5) & 65535u64;
let v9: u64 = (v7 & v8) & 65535u64;
let v10: u64 = (v9 >> 15) & 1;
let v11: u64 = (v7 ^ v5) & 65535u64;
let v12: u64 = (v11 >> 4) & 1;
let v13: u64 = (v5 >> 15) & 1;
let v14: u64 = u64::from(v5 == 0);
let v15: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_dst = Some(v5);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v15 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v12 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v14 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v13 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v10 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1e8 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 65535u64;
let v1: u64 = address as u64;
let addr2=match memory_context {Some(c)=>c.translate(v1 as u32,MemWidth::Word,false,*eip)?,None=>v1 as u32}; let v2: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr2,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
let v3: u64 = v0 & 131071u64;
let v4: u64 = v2 & 131071u64;
let v5: u64 = (v3.wrapping_add(v4)) & 131071u64;
let v6: u64 = v5 & 65535u64;
let v7: u64 = (v5 >> 16) & 1;
let v8: u64 = (v0 ^ v2) & 65535u64;
let v9: u64 = (!v8) & 65535u64;
let v10: u64 = (v0 ^ v6) & 65535u64;
let v11: u64 = (v9 & v10) & 65535u64;
let v12: u64 = (v11 >> 15) & 1;
let v13: u64 = (v8 ^ v6) & 65535u64;
let v14: u64 = (v13 >> 4) & 1;
let v15: u64 = (v6 >> 15) & 1;
let v16: u64 = u64::from(v6 == 0);
let v17: u64 = u64::from((v6 as u8).count_ones() % 2 == 0);
pending_dst = Some(v6);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v7 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v17 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v14 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v16 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v15 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v12 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1ec {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1ee {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = (regs[0] as u64) & 65535u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
let v2: u64 = (if v0 & 32768u64 != 0 {v0 | !65535u64} else {v0}) & 4294967295u64;
let v3: u64 = (if v1 & 32768u64 != 0 {v1 | !65535u64} else {v1}) & 4294967295u64;
let v4: u64 = (v2.wrapping_mul(v3)) & 4294967295u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (if v5 & 32768u64 != 0 {v5 | !65535u64} else {v5}) & 4294967295u64;
let v7: u64 = (v4 ^ v6) & 4294967295u64;
let v8: u64 = u64::from(v7 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = 16u64;
let v11: u64 = (if v10>=32u64 {0}else{v4>>v10}) & 4294967295u64;
let v12: u64 = v11 & 65535u64;
pending_registers.push((0,16,v5));
pending_registers.push((2,16,v12));
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v9 as u32) << 0);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v9 as u32) << 11);
pending_flags &= !128u32; pending_undefined |= 128u32;
pending_flags &= !64u32; pending_undefined |= 64u32;
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_flags &= !4u32; pending_undefined |= 4u32;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1f0 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1f3 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v2: u64 = 31u64;
let v3: u64 = (v1 & v2) & 65535u64;
let v4: u64 = 1u64;
let v5: u64 = 16u64;
let v6: u64 = 1u64;
let v7: u64 = 0u64;
let v8: u64 = u64::from(v3 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = (v3 ^ v4) & 65535u64;
let v11: u64 = u64::from(v10 == 0);
let v12: u64 = (v3.wrapping_sub(v4)) & 65535u64;
let v13: u64 = (if v3>=16u64 {0}else{v0<<v3}) & 65535u64;
let v14: u64 = (if v12>=16u64 {0}else{v0<<v12}) & 65535u64;
let v15: u64 = (v14 >> 15) & 1;
let v16: u64 = u64::from(v3<v5);
let v17: u64 = (v13 >> 15) & 1;
let v18: u64 = u64::from(v13 == 0);
let v19: u64 = u64::from((v13 as u8).count_ones() % 2 == 0);
let v20: u64 = (v17 ^ v15) & 1u64;
pending_dst = Some(v13);
if v9!=0 {pending_flags &= !1u32; if v16!=0 {pending_flags |= (v15 as u32)<<0;pending_undefined &= !1u32;}else{pending_undefined |= 1u32;}}
if v9!=0 {pending_flags &= !2048u32; if v11!=0 {pending_flags |= (v20 as u32)<<11;pending_undefined &= !2048u32;}else{pending_undefined |= 2048u32;}}
if v9!=0 {pending_flags &= !16u32; if v7!=0 {pending_flags |= (v7 as u32)<<4;pending_undefined &= !16u32;}else{pending_undefined |= 16u32;}}
if v9!=0 {pending_flags &= !128u32; if v6!=0 {pending_flags |= (v17 as u32)<<7;pending_undefined &= !128u32;}else{pending_undefined |= 128u32;}}
if v9!=0 {pending_flags &= !64u32; if v6!=0 {pending_flags |= (v18 as u32)<<6;pending_undefined &= !64u32;}else{pending_undefined |= 64u32;}}
if v9!=0 {pending_flags &= !4u32; if v6!=0 {pending_flags |= (v19 as u32)<<2;pending_undefined &= !4u32;}else{pending_undefined |= 4u32;}}
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1f5 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 65535u64;
let v1: u64 = address as u64;
let addr2=match memory_context {Some(c)=>c.translate(v1 as u32,MemWidth::Word,false,*eip)?,None=>v1 as u32}; let v2: u64 = ram.as_deref_mut().ok_or(ExecError::InvalidContext)?.load(addr2,MemWidth::Word).map_err(|f|f.error(*eip))? as u64;
let v3: u64 = v0 & 131071u64;
let v4: u64 = v2 & 131071u64;
let v5: u64 = (v3.wrapping_add(v4)) & 131071u64;
let v6: u64 = v5 & 65535u64;
let v7: u64 = (v5 >> 16) & 1;
let v8: u64 = (v0 ^ v2) & 65535u64;
let v9: u64 = (!v8) & 65535u64;
let v10: u64 = (v0 ^ v6) & 65535u64;
let v11: u64 = (v9 & v10) & 65535u64;
let v12: u64 = (v11 >> 15) & 1;
let v13: u64 = (v8 ^ v6) & 65535u64;
let v14: u64 = (v13 >> 4) & 1;
let v15: u64 = (v6 >> 15) & 1;
let v16: u64 = u64::from(v6 == 0);
let v17: u64 = u64::from((v6 as u8).count_ones() % 2 == 0);
pending_dst = Some(v6);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v7 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v17 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v14 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v16 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v15 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v12 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1f9 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1fb {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 65535u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1fd {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v4: u64 = (v2.wrapping_sub(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (v0 ^ v5) & 65535u64;
let v9: u64 = (v7 & v8) & 65535u64;
let v10: u64 = (v9 >> 15) & 1;
let v11: u64 = (v7 ^ v5) & 65535u64;
let v12: u64 = (v11 >> 4) & 1;
let v13: u64 = (v5 >> 15) & 1;
let v14: u64 = u64::from(v5 == 0);
let v15: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_dst = Some(v5);
// Preserve flag mask 0x1, including undefined status.
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v15 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v12 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v14 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v13 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v10 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i1fe {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((*flags >> 11) & 1) as u64;
let v1: u64 = (!v0) & 1u64;
let v2: u64 = ((*flags >> 0) & 1) as u64;
let v3: u64 = (!v2) & 1u64;
let v4: u64 = ((*flags >> 6) & 1) as u64;
let v5: u64 = (!v4) & 1u64;
let v6: u64 = (v2 | v4) & 1u64;
let v7: u64 = (!v6) & 1u64;
let v8: u64 = ((*flags >> 7) & 1) as u64;
let v9: u64 = (!v8) & 1u64;
let v10: u64 = ((*flags >> 2) & 1) as u64;
let v11: u64 = (!v10) & 1u64;
let v12: u64 = (v8 ^ v0) & 1u64;
let v13: u64 = (!v12) & 1u64;
let v14: u64 = (v12 | v4) & 1u64;
let v15: u64 = (!v14) & 1u64;
// Preserve flag mask 0x0, including undefined status.
let v16: u64 = (context.next_eip as u64) & 4294967295u64;
let v17: u64 = v16 & 65535u64;
let v18: u64 = (context.displacement as u32 as u64) & 65535u64;
let v19: u64 = (v17.wrapping_add(v18)) & 65535u64;
let v20: u64 = v19 & 4294967295u64;
if v5 != 0 { if v20 as u32 > context.code_limit { return Err(ExecError::GeneralProtection{eip:*eip,error_code:0}); } pending_eip = v20 as u32; }
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i200 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 255u64;
let v1: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
let v2: u64 = (v0 ^ v1) & 255u64;
let v3: u64 = 0u64;
let v4: u64 = (v2 >> 7) & 1;
let v5: u64 = u64::from(v2 == 0);
let v6: u64 = u64::from((v2 as u8).count_ones() % 2 == 0);
pending_dst = Some(v2);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v3 as u32) << 0);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v3 as u32) << 11);
pending_flags &= !16u32; pending_undefined |= 16u32;
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v4 as u32) << 7);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v5 as u32) << 6);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v6 as u32) << 2);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i202 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i203 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 30u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i206 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v2: u64 = v0 & 131071u64;
let v3: u64 = v1 & 131071u64;
let v4: u64 = (v2.wrapping_sub(v3)) & 131071u64;
let v5: u64 = v4 & 65535u64;
let v6: u64 = (v4 >> 16) & 1;
let v7: u64 = (v0 ^ v1) & 65535u64;
let v8: u64 = (v0 ^ v5) & 65535u64;
let v9: u64 = (v7 & v8) & 65535u64;
let v10: u64 = (v9 >> 15) & 1;
let v11: u64 = (v7 ^ v5) & 65535u64;
let v12: u64 = (v11 >> 4) & 1;
let v13: u64 = (v5 >> 15) & 1;
let v14: u64 = u64::from(v5 == 0);
let v15: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_dst = Some(v5);
pending_undefined &= !1u32; pending_flags = (pending_flags & !1u32) | ((v6 as u32) << 0);
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v15 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v12 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v14 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v13 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v10 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i208 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 15u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i20a {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i20c {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[src.0] as u64) >> src.1) & 255u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i20e {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = ((regs[dst.0] as u64) >> dst.1) & 255u64;
let v1: u64 = 1u64;
let v2: u64 = v0 & 511u64;
let v3: u64 = v1 & 511u64;
let v4: u64 = (v2.wrapping_add(v3)) & 511u64;
let v5: u64 = v4 & 255u64;
let v6: u64 = (v4 >> 8) & 1;
let v7: u64 = (v0 ^ v1) & 255u64;
let v8: u64 = (!v7) & 255u64;
let v9: u64 = (v0 ^ v5) & 255u64;
let v10: u64 = (v8 & v9) & 255u64;
let v11: u64 = (v10 >> 7) & 1;
let v12: u64 = (v7 ^ v5) & 255u64;
let v13: u64 = (v12 >> 4) & 1;
let v14: u64 = (v5 >> 7) & 1;
let v15: u64 = u64::from(v5 == 0);
let v16: u64 = u64::from((v5 as u8).count_ones() % 2 == 0);
pending_dst = Some(v5);
// Preserve flag mask 0x1, including undefined status.
pending_undefined &= !4u32; pending_flags = (pending_flags & !4u32) | ((v16 as u32) << 2);
pending_undefined &= !16u32; pending_flags = (pending_flags & !16u32) | ((v13 as u32) << 4);
pending_undefined &= !64u32; pending_flags = (pending_flags & !64u32) | ((v15 as u32) << 6);
pending_undefined &= !128u32; pending_flags = (pending_flags & !128u32) | ((v14 as u32) << 7);
pending_undefined &= !2048u32; pending_flags = (pending_flags & !2048u32) | ((v11 as u32) << 11);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i210 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i211 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 12u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i213 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v2: u64 = 31u64;
let v3: u64 = (v1 & v2) & 65535u64;
let v4: u64 = 1u64;
let v5: u64 = 16u64;
let v6: u64 = 1u64;
let v7: u64 = 0u64;
let v8: u64 = u64::from(v3 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = (v3 ^ v4) & 65535u64;
let v11: u64 = u64::from(v10 == 0);
let v12: u64 = (v3.wrapping_sub(v4)) & 65535u64;
let v13: u64 = (if v3>=16u64 {0}else{v0>>v3}) & 65535u64;
let v14: u64 = (if v12>=16u64 {0}else{v0>>v12}) & 65535u64;
let v15: u64 = (v14 >> 0) & 1;
let v16: u64 = u64::from(v3<v5);
let v17: u64 = (v13 >> 15) & 1;
let v18: u64 = u64::from(v13 == 0);
let v19: u64 = u64::from((v13 as u8).count_ones() % 2 == 0);
let v20: u64 = (v0 >> 15) & 1;
pending_dst = Some(v13);
if v9!=0 {pending_flags &= !1u32; if v16!=0 {pending_flags |= (v15 as u32)<<0;pending_undefined &= !1u32;}else{pending_undefined |= 1u32;}}
if v9!=0 {pending_flags &= !2048u32; if v11!=0 {pending_flags |= (v20 as u32)<<11;pending_undefined &= !2048u32;}else{pending_undefined |= 2048u32;}}
if v9!=0 {pending_flags &= !16u32; if v7!=0 {pending_flags |= (v7 as u32)<<4;pending_undefined &= !16u32;}else{pending_undefined |= 16u32;}}
if v9!=0 {pending_flags &= !128u32; if v6!=0 {pending_flags |= (v17 as u32)<<7;pending_undefined &= !128u32;}else{pending_undefined |= 128u32;}}
if v9!=0 {pending_flags &= !64u32; if v6!=0 {pending_flags |= (v18 as u32)<<6;pending_undefined &= !64u32;}else{pending_undefined |= 64u32;}}
if v9!=0 {pending_flags &= !4u32; if v6!=0 {pending_flags |= (v19 as u32)<<2;pending_undefined &= !4u32;}else{pending_undefined |= 4u32;}}
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i215 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v0: u64 = 4u64;
pending_dst = Some(v0);
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i217 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v2: u64 = 31u64;
let v3: u64 = (v1 & v2) & 65535u64;
let v4: u64 = 1u64;
let v5: u64 = 16u64;
let v6: u64 = 1u64;
let v7: u64 = 0u64;
let v8: u64 = u64::from(v3 == 0);
let v9: u64 = (!v8) & 1u64;
let v10: u64 = (v3 ^ v4) & 65535u64;
let v11: u64 = u64::from(v10 == 0);
let v12: u64 = (v3.wrapping_sub(v4)) & 65535u64;
let v13: u64 = (if v3>=16u64 {0}else{v0<<v3}) & 65535u64;
let v14: u64 = (if v12>=16u64 {0}else{v0<<v12}) & 65535u64;
let v15: u64 = (v14 >> 15) & 1;
let v16: u64 = u64::from(v3<v5);
let v17: u64 = (v13 >> 15) & 1;
let v18: u64 = u64::from(v13 == 0);
let v19: u64 = u64::from((v13 as u8).count_ones() % 2 == 0);
let v20: u64 = (v17 ^ v15) & 1u64;
pending_dst = Some(v13);
if v9!=0 {pending_flags &= !1u32; if v16!=0 {pending_flags |= (v15 as u32)<<0;pending_undefined &= !1u32;}else{pending_undefined |= 1u32;}}
if v9!=0 {pending_flags &= !2048u32; if v11!=0 {pending_flags |= (v20 as u32)<<11;pending_undefined &= !2048u32;}else{pending_undefined |= 2048u32;}}
if v9!=0 {pending_flags &= !16u32; if v7!=0 {pending_flags |= (v7 as u32)<<4;pending_undefined &= !16u32;}else{pending_undefined |= 16u32;}}
if v9!=0 {pending_flags &= !128u32; if v6!=0 {pending_flags |= (v17 as u32)<<7;pending_undefined &= !128u32;}else{pending_undefined |= 128u32;}}
if v9!=0 {pending_flags &= !64u32; if v6!=0 {pending_flags |= (v18 as u32)<<6;pending_undefined &= !64u32;}else{pending_undefined |= 64u32;}}
if v9!=0 {pending_flags &= !4u32; if v6!=0 {pending_flags |= (v19 as u32)<<2;pending_undefined &= !4u32;}else{pending_undefined |= 4u32;}}
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i219 {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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
let v2: u64 = (v0 | v1) & 65535u64;
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
mod i21b {use crate::{FlowContext,ExecError,Ram,MemWidth,address,privilege}; 
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

let result=execute_state(regs,flags,undefined,eip,dst,src,context,Some(ram),offset,memory,Some(&mut *machine),address.map(|a|a.effective));

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
if let Err(e)=i100::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 257, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i101::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 260, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_106(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i106::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 265, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i109::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 267, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_10b(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i10b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 270, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i10e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 273, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_130(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i130::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(3,0) ,FlowContext { next_eip: 308, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 581 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i134::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(3,0) ,FlowContext { next_eip: 310, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i136::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(0,8) ,FlowContext { next_eip: 313, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i139::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 315, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i13b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(0,8) ,FlowContext { next_eip: 317, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
if let Err(e)=i13d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(1,0) ,FlowContext { next_eip: 319, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (5,Err(e));}
if let Err(e)=i13f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 321, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (6,Err(e));}
if let Err(e)=i141::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 324, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (7,Err(e));}
(8,Ok(()))
}
#[inline(always)] fn block_134(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i134::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(3,0) ,FlowContext { next_eip: 310, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i136::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(0,8) ,FlowContext { next_eip: 313, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i139::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 315, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i13b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(0,8) ,FlowContext { next_eip: 317, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i13d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(1,0) ,FlowContext { next_eip: 319, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
if let Err(e)=i13f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 321, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (5,Err(e));}
if let Err(e)=i141::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 324, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (6,Err(e));}
(7,Ok(()))
}
#[inline(always)] fn block_136(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i136::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(0,8) ,FlowContext { next_eip: 313, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i139::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 315, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i13b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(0,8) ,FlowContext { next_eip: 317, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i13d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(1,0) ,FlowContext { next_eip: 319, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i13f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 321, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
if let Err(e)=i141::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 324, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (5,Err(e));}
(6,Ok(()))
}
#[inline(always)] fn block_139(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i139::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 315, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i13b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(0,8) ,FlowContext { next_eip: 317, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i13d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(1,0) ,FlowContext { next_eip: 319, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i13f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 321, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i141::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 324, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
(5,Ok(()))
}
#[inline(always)] fn block_13b(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i13b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(0,8) ,FlowContext { next_eip: 317, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i13d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(1,0) ,FlowContext { next_eip: 319, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i13f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 321, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i141::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 324, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_13d(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i13d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(1,0) ,FlowContext { next_eip: 319, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i13f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 321, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i141::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 324, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_13f(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i13f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 321, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i141::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 324, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_145(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i145::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(7,0) ,FlowContext { next_eip: 329, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 583 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i149::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(1,0) ,FlowContext { next_eip: 331, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i14b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(1,0) ,FlowContext { next_eip: 333, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i14d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(3,0) ,FlowContext { next_eip: 335, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i14f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 338, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (4,Err(e));}
if let Err(e)=i152::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 341, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 585 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (5,Err(e));}
if let Err(e)=i155::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 344, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (6,Err(e));}
(7,Ok(()))
}
#[inline(always)] fn block_149(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i149::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(1,0) ,FlowContext { next_eip: 331, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i14b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(1,0) ,FlowContext { next_eip: 333, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i14d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(3,0) ,FlowContext { next_eip: 335, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i14f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 338, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (3,Err(e));}
if let Err(e)=i152::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 341, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 585 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (4,Err(e));}
if let Err(e)=i155::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 344, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (5,Err(e));}
(6,Ok(()))
}
#[inline(always)] fn block_14b(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i14b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(1,0) ,FlowContext { next_eip: 333, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i14d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(3,0) ,FlowContext { next_eip: 335, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i14f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 338, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (2,Err(e));}
if let Err(e)=i152::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 341, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 585 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (3,Err(e));}
if let Err(e)=i155::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 344, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (4,Err(e));}
(5,Ok(()))
}
#[inline(always)] fn block_14d(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i14d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(3,0) ,FlowContext { next_eip: 335, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i14f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 338, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (1,Err(e));}
if let Err(e)=i152::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 341, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 585 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (2,Err(e));}
if let Err(e)=i155::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 344, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_14f(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i14f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 338, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i152::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 341, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 585 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (1,Err(e));}
if let Err(e)=i155::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 344, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_152(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i152::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 341, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 585 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i155::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 344, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 0 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_158(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i158::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(0,8) ,FlowContext { next_eip: 348, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 80 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i15c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 352, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Di, displacement: 80 }, override_segment: Some(Es), segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_169(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i169::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(7,0) ,FlowContext { next_eip: 367, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 581 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i16f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 369, displacement: -71, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_17c(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i17c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(7,0) ,FlowContext { next_eip: 386, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 583 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i182::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 388, displacement: -102, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_184(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i184::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 391, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i187::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 394, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_18f(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i18f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(0,8) ,FlowContext { next_eip: 401, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i191::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,8) ,(3,8) ,FlowContext { next_eip: 403, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i193::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 406, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_191(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i191::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,8) ,(3,8) ,FlowContext { next_eip: 403, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i193::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 406, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_198(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i198::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(2,0) ,FlowContext { next_eip: 411, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i19b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,8) ,(0,8) ,FlowContext { next_eip: 413, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1ad(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1ad::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(6,0) ,FlowContext { next_eip: 431, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1af::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(7,0) ,FlowContext { next_eip: 433, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1b1::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(5,0) ,(5,0) ,FlowContext { next_eip: 436, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1b4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(7,0) ,FlowContext { next_eip: 440, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i1b8::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 442, displacement: 73, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
(5,Ok(()))
}
#[inline(always)] fn block_1af(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1af::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(7,0) ,FlowContext { next_eip: 433, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1b1::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(5,0) ,(5,0) ,FlowContext { next_eip: 436, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1b4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(7,0) ,FlowContext { next_eip: 440, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1b8::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 442, displacement: 73, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_1b1(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1b1::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(5,0) ,(5,0) ,FlowContext { next_eip: 436, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1b4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(7,0) ,FlowContext { next_eip: 440, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1b8::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 442, displacement: 73, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_1b4(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1b4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(7,0) ,FlowContext { next_eip: 440, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1b8::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 442, displacement: 73, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1ba(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1ba::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(7,0) ,FlowContext { next_eip: 446, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1be::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 448, displacement: 67, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1c0(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1c0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(7,0) ,FlowContext { next_eip: 452, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1c4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 454, displacement: 61, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1c6(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1c6::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(7,0) ,FlowContext { next_eip: 458, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1ca::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 460, displacement: 55, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1cc(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1cc::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(6,0) ,FlowContext { next_eip: 462, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1ce::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(6,0) ,FlowContext { next_eip: 464, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1d0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 467, displacement: 62, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_1ce(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1ce::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(6,0) ,FlowContext { next_eip: 464, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1d0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 467, displacement: 62, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1d3(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1d3::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(0,0) ,FlowContext { next_eip: 469, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1d5::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 471, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1d7::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 473, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1d9::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 476, displacement: 53, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_1d5(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1d5::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 471, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1d7::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 473, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1d9::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 476, displacement: 53, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_1d7(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1d7::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 473, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1d9::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 476, displacement: 53, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1dc(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1dc::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(3,0) ,FlowContext { next_eip: 478, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1de::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(0,0) ,FlowContext { next_eip: 480, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1e0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(7,0) ,FlowContext { next_eip: 484, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1e4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 486, displacement: 29, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_1de(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1de::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(0,0) ,FlowContext { next_eip: 480, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1e0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(7,0) ,FlowContext { next_eip: 484, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1e4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 486, displacement: 29, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_1e0(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1e0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(7,0) ,FlowContext { next_eip: 484, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1e4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 486, displacement: 29, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1e6(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1e6::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(0,0) ,FlowContext { next_eip: 488, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1e8::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(3,0) ,FlowContext { next_eip: 492, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 579 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (1,Err(e));}
if let Err(e)=i1ec::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(6,0) ,FlowContext { next_eip: 494, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1ee::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 496, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i1f0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 499, displacement: 30, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
(5,Ok(()))
}
#[inline(always)] fn block_1e8(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1e8::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(3,0) ,FlowContext { next_eip: 492, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 579 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i1ec::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(6,0) ,FlowContext { next_eip: 494, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1ee::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 496, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1f0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 499, displacement: 30, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_1ec(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1ec::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(6,0) ,FlowContext { next_eip: 494, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1ee::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 496, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1f0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 499, displacement: 30, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_1ee(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1ee::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(7,0) ,FlowContext { next_eip: 496, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1f0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 499, displacement: 30, displacement_bits: 16, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_1f3(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1f3::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(1,0) ,FlowContext { next_eip: 501, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1f5::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 505, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 577 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (1,Err(e));}
if let Err(e)=i1f9::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(0,0) ,FlowContext { next_eip: 507, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1fb::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(3,0) ,FlowContext { next_eip: 509, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i1fd::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(5,0) ,(5,0) ,FlowContext { next_eip: 510, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
if let Err(e)=i1fe::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 512, displacement: -76, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (5,Err(e));}
(6,Ok(()))
}
#[inline(always)] fn block_1f5(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1f5::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 505, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,Some(AddressContext { effective: Bits16 { form: Direct, displacement: 577 }, override_segment: None, segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }] }),machine){return (0,Err(e));}
if let Err(e)=i1f9::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(0,0) ,FlowContext { next_eip: 507, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1fb::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(3,0) ,FlowContext { next_eip: 509, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1fd::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(5,0) ,(5,0) ,FlowContext { next_eip: 510, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i1fe::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 512, displacement: -76, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
(5,Ok(()))
}
#[inline(always)] fn block_1f9(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1f9::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(7,0) ,(0,0) ,FlowContext { next_eip: 507, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1fb::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(3,0) ,FlowContext { next_eip: 509, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1fd::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(5,0) ,(5,0) ,FlowContext { next_eip: 510, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i1fe::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 512, displacement: -76, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_1fb(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1fb::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(6,0) ,(3,0) ,FlowContext { next_eip: 509, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1fd::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(5,0) ,(5,0) ,FlowContext { next_eip: 510, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i1fe::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 512, displacement: -76, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_1fd(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i1fd::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(5,0) ,(5,0) ,FlowContext { next_eip: 510, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i1fe::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 512, displacement: -76, displacement_bits: 8, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_200(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i200::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 514, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i202::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 515, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_203(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i203::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 518, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i206::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(5,0) ,FlowContext { next_eip: 520, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i208::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(3,0) ,FlowContext { next_eip: 522, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i20a::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(3,0) ,FlowContext { next_eip: 524, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i20c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,8) ,FlowContext { next_eip: 526, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
if let Err(e)=i20e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 528, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (5,Err(e));}
if let Err(e)=i210::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 529, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (6,Err(e));}
(7,Ok(()))
}
#[inline(always)] fn block_206(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i206::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(5,0) ,FlowContext { next_eip: 520, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i208::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(3,0) ,FlowContext { next_eip: 522, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i20a::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(3,0) ,FlowContext { next_eip: 524, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i20c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,8) ,FlowContext { next_eip: 526, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i20e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 528, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
if let Err(e)=i210::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 529, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (5,Err(e));}
(6,Ok(()))
}
#[inline(always)] fn block_208(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i208::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(3,0) ,(3,0) ,FlowContext { next_eip: 522, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i20a::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(3,0) ,FlowContext { next_eip: 524, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i20c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,8) ,FlowContext { next_eip: 526, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i20e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 528, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i210::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 529, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
(5,Ok(()))
}
#[inline(always)] fn block_20a(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i20a::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(3,0) ,FlowContext { next_eip: 524, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i20c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,8) ,FlowContext { next_eip: 526, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i20e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 528, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i210::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 529, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_20c(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i20c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,8) ,FlowContext { next_eip: 526, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i20e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 528, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i210::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 529, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_20e(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i20e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 528, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i210::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 529, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
}
#[inline(always)] fn block_211(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i211::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 531, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i213::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(1,0) ,FlowContext { next_eip: 533, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i215::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 535, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i217::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(1,0) ,FlowContext { next_eip: 537, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i219::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(2,0) ,FlowContext { next_eip: 539, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
if let Err(e)=i21b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 540, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (5,Err(e));}
(6,Ok(()))
}
#[inline(always)] fn block_213(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i213::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(1,0) ,FlowContext { next_eip: 533, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i215::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 535, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i217::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(1,0) ,FlowContext { next_eip: 537, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i219::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(2,0) ,FlowContext { next_eip: 539, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
if let Err(e)=i21b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 540, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (4,Err(e));}
(5,Ok(()))
}
#[inline(always)] fn block_215(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i215::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(1,0) ,(1,0) ,FlowContext { next_eip: 535, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i217::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(1,0) ,FlowContext { next_eip: 537, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i219::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(2,0) ,FlowContext { next_eip: 539, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
if let Err(e)=i21b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 540, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (3,Err(e));}
(4,Ok(()))
}
#[inline(always)] fn block_217(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i217::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(2,0) ,(1,0) ,FlowContext { next_eip: 537, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i219::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(2,0) ,FlowContext { next_eip: 539, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
if let Err(e)=i21b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 540, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (2,Err(e));}
(3,Ok(()))
}
#[inline(always)] fn block_219(cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->(usize,Result<(),crate::ExecError>){
if let Err(e)=i219::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(2,0) ,FlowContext { next_eip: 539, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (0,Err(e));}
if let Err(e)=i21b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(0,0) ,(0,0) ,FlowContext { next_eip: 540, displacement: 0, displacement_bits: 32, code_limit: 65535 },ram,None,machine){return (1,Err(e));}
(2,Ok(()))
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
  let result=match cpu.eip{
256 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(256).and_then(|at|ram.contiguous_slice(at,4)).is_some_and(|s|s==[252, 184, 18, 0]) => Some(block_100(cpu,ram,machine)),
262 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [4096, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(262).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[184, 0, 160, 142, 192]) => Some(block_106(cpu,ram,machine)),
267 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(267).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[186, 206, 3, 184, 5, 2]) => Some(block_10b(cpu,ram,machine)),
304 if budget>=8 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(304).and_then(|at|ram.contiguous_slice(at,20)).is_some_and(|s|s==[139, 30, 69, 2, 136, 217, 128, 225, 3, 208, 225, 180, 192, 210, 236, 176, 8, 186, 206, 3]) => Some(block_130(cpu,ram,machine)),
308 if budget>=7 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(308).and_then(|at|ram.contiguous_slice(at,16)).is_some_and(|s|s==[136, 217, 128, 225, 3, 208, 225, 180, 192, 210, 236, 176, 8, 186, 206, 3]) => Some(block_134(cpu,ram,machine)),
310 if budget>=6 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(310).and_then(|at|ram.contiguous_slice(at,14)).is_some_and(|s|s==[128, 225, 3, 208, 225, 180, 192, 210, 236, 176, 8, 186, 206, 3]) => Some(block_136(cpu,ram,machine)),
313 if budget>=5 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(313).and_then(|at|ram.contiguous_slice(at,11)).is_some_and(|s|s==[208, 225, 180, 192, 210, 236, 176, 8, 186, 206, 3]) => Some(block_139(cpu,ram,machine)),
315 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(315).and_then(|at|ram.contiguous_slice(at,9)).is_some_and(|s|s==[180, 192, 210, 236, 176, 8, 186, 206, 3]) => Some(block_13b(cpu,ram,machine)),
317 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(317).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[210, 236, 176, 8, 186, 206, 3]) => Some(block_13d(cpu,ram,machine)),
319 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(319).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[176, 8, 186, 206, 3]) => Some(block_13f(cpu,ram,machine)),
325 if budget>=7 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(325).and_then(|at|ram.contiguous_slice(at,19)).is_some_and(|s|s==[139, 62, 71, 2, 209, 235, 209, 235, 1, 223, 38, 138, 5, 160, 73, 2, 38, 136, 5]) => Some(block_145(cpu,ram,machine)),
329 if budget>=6 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(329).and_then(|at|ram.contiguous_slice(at,15)).is_some_and(|s|s==[209, 235, 209, 235, 1, 223, 38, 138, 5, 160, 73, 2, 38, 136, 5]) => Some(block_149(cpu,ram,machine)),
331 if budget>=5 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(331).and_then(|at|ram.contiguous_slice(at,13)).is_some_and(|s|s==[209, 235, 1, 223, 38, 138, 5, 160, 73, 2, 38, 136, 5]) => Some(block_14b(cpu,ram,machine)),
333 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(333).and_then(|at|ram.contiguous_slice(at,11)).is_some_and(|s|s==[1, 223, 38, 138, 5, 160, 73, 2, 38, 136, 5]) => Some(block_14d(cpu,ram,machine)),
335 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(335).and_then(|at|ram.contiguous_slice(at,9)).is_some_and(|s|s==[38, 138, 5, 160, 73, 2, 38, 136, 5]) => Some(block_14f(cpu,ram,machine)),
338 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(338).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[160, 73, 2, 38, 136, 5]) => Some(block_152(cpu,ram,machine)),
344 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(344).and_then(|at|ram.contiguous_slice(at,8)).is_some_and(|s|s==[38, 138, 101, 80, 38, 136, 69, 80]) => Some(block_158(cpu,ram,machine)),
361 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(361).and_then(|at|ram.contiguous_slice(at,8)).is_some_and(|s|s==[129, 62, 69, 2, 64, 1, 114, 185]) => Some(block_169(cpu,ram,machine)),
380 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(380).and_then(|at|ram.contiguous_slice(at,8)).is_some_and(|s|s==[129, 62, 71, 2, 0, 150, 114, 154]) => Some(block_17c(cpu,ram,machine)),
388 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(388).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[186, 206, 3, 184, 8, 255]) => Some(block_184(cpu,ram,machine)),
399 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(399).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[180, 2, 48, 255, 186, 0, 29]) => Some(block_18f(cpu,ram,machine)),
401 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(401).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[48, 255, 186, 0, 29]) => Some(block_191(cpu,ram,machine)),
408 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(408).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[186, 28, 2, 180, 9]) => Some(block_198(cpu,ram,machine)),
429 if budget>=5 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(429).and_then(|at|ram.contiguous_slice(at,13)).is_some_and(|s|s==[49, 246, 49, 255, 189, 30, 0, 129, 254, 0, 32, 125, 73]) => Some(block_1ad(cpu,ram,machine)),
431 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(431).and_then(|at|ram.contiguous_slice(at,11)).is_some_and(|s|s==[49, 255, 189, 30, 0, 129, 254, 0, 32, 125, 73]) => Some(block_1af(cpu,ram,machine)),
433 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(433).and_then(|at|ram.contiguous_slice(at,9)).is_some_and(|s|s==[189, 30, 0, 129, 254, 0, 32, 125, 73]) => Some(block_1b1(cpu,ram,machine)),
436 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(436).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[129, 254, 0, 32, 125, 73]) => Some(block_1b4(cpu,ram,machine)),
442 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(442).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[129, 254, 0, 224, 126, 67]) => Some(block_1ba(cpu,ram,machine)),
448 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(448).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[129, 255, 0, 32, 125, 61]) => Some(block_1c0(cpu,ram,machine)),
454 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(454).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[129, 255, 0, 224, 126, 55]) => Some(block_1c6(cpu,ram,machine)),
460 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(460).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[137, 240, 247, 238, 232, 62, 0]) => Some(block_1cc(cpu,ram,machine)),
462 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(462).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[247, 238, 232, 62, 0]) => Some(block_1ce(cpu,ram,machine)),
467 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(467).and_then(|at|ram.contiguous_slice(at,9)).is_some_and(|s|s==[137, 195, 137, 248, 247, 239, 232, 53, 0]) => Some(block_1d3(cpu,ram,machine)),
469 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(469).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[137, 248, 247, 239, 232, 53, 0]) => Some(block_1d5(cpu,ram,machine)),
471 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(471).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[247, 239, 232, 53, 0]) => Some(block_1d7(cpu,ram,machine)),
476 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(476).and_then(|at|ram.contiguous_slice(at,10)).is_some_and(|s|s==[137, 217, 1, 193, 129, 249, 0, 64, 127, 29]) => Some(block_1dc(cpu,ram,machine)),
478 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(478).and_then(|at|ram.contiguous_slice(at,8)).is_some_and(|s|s==[1, 193, 129, 249, 0, 64, 127, 29]) => Some(block_1de(cpu,ram,machine)),
480 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(480).and_then(|at|ram.contiguous_slice(at,6)).is_some_and(|s|s==[129, 249, 0, 64, 127, 29]) => Some(block_1e0(cpu,ram,machine)),
486 if budget>=5 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(486).and_then(|at|ram.contiguous_slice(at,13)).is_some_and(|s|s==[41, 195, 3, 30, 67, 2, 137, 240, 247, 239, 232, 30, 0]) => Some(block_1e6(cpu,ram,machine)),
488 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(488).and_then(|at|ram.contiguous_slice(at,11)).is_some_and(|s|s==[3, 30, 67, 2, 137, 240, 247, 239, 232, 30, 0]) => Some(block_1e8(cpu,ram,machine)),
492 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(492).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[137, 240, 247, 239, 232, 30, 0]) => Some(block_1ec(cpu,ram,machine)),
494 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(494).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[247, 239, 232, 30, 0]) => Some(block_1ee(cpu,ram,machine)),
499 if budget>=6 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(499).and_then(|at|ram.contiguous_slice(at,13)).is_some_and(|s|s==[209, 224, 3, 6, 65, 2, 137, 199, 137, 222, 77, 117, 180]) => Some(block_1f3(cpu,ram,machine)),
501 if budget>=5 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(501).and_then(|at|ram.contiguous_slice(at,11)).is_some_and(|s|s==[3, 6, 65, 2, 137, 199, 137, 222, 77, 117, 180]) => Some(block_1f5(cpu,ram,machine)),
505 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(505).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[137, 199, 137, 222, 77, 117, 180]) => Some(block_1f9(cpu,ram,machine)),
507 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(507).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[137, 222, 77, 117, 180]) => Some(block_1fb(cpu,ram,machine)),
509 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(509).and_then(|at|ram.contiguous_slice(at,3)).is_some_and(|s|s==[77, 117, 180]) => Some(block_1fd(cpu,ram,machine)),
512 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(512).and_then(|at|ram.contiguous_slice(at,3)).is_some_and(|s|s==[48, 192, 195]) => Some(block_200(cpu,ram,machine)),
515 if budget>=7 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(515).and_then(|at|ram.contiguous_slice(at,14)).is_some_and(|s|s==[184, 30, 0, 41, 232, 179, 15, 246, 243, 136, 224, 254, 192, 195]) => Some(block_203(cpu,ram,machine)),
518 if budget>=6 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(518).and_then(|at|ram.contiguous_slice(at,11)).is_some_and(|s|s==[41, 232, 179, 15, 246, 243, 136, 224, 254, 192, 195]) => Some(block_206(cpu,ram,machine)),
520 if budget>=5 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(520).and_then(|at|ram.contiguous_slice(at,9)).is_some_and(|s|s==[179, 15, 246, 243, 136, 224, 254, 192, 195]) => Some(block_208(cpu,ram,machine)),
522 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(522).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[246, 243, 136, 224, 254, 192, 195]) => Some(block_20a(cpu,ram,machine)),
524 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(524).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[136, 224, 254, 192, 195]) => Some(block_20c(cpu,ram,machine)),
526 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(526).and_then(|at|ram.contiguous_slice(at,3)).is_some_and(|s|s==[254, 192, 195]) => Some(block_20e(cpu,ram,machine)),
529 if budget>=6 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(529).and_then(|at|ram.contiguous_slice(at,11)).is_some_and(|s|s==[177, 12, 211, 232, 177, 4, 211, 226, 9, 208, 195]) => Some(block_211(cpu,ram,machine)),
531 if budget>=5 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(531).and_then(|at|ram.contiguous_slice(at,9)).is_some_and(|s|s==[211, 232, 177, 4, 211, 226, 9, 208, 195]) => Some(block_213(cpu,ram,machine)),
533 if budget>=4 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(533).and_then(|at|ram.contiguous_slice(at,7)).is_some_and(|s|s==[177, 4, 211, 226, 9, 208, 195]) => Some(block_215(cpu,ram,machine)),
535 if budget>=3 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(535).and_then(|at|ram.contiguous_slice(at,5)).is_some_and(|s|s==[211, 226, 9, 208, 195]) => Some(block_217(cpu,ram,machine)),
537 if budget>=2 && *machine==SegmentState { segments: [SegmentCache { base: 655360, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }, SegmentCache { base: 65536, limit: 65535, access: Data { writable: true, expand_down: false, big: false, dpl: 0 } }], selectors: [40960, 4096, 4096, 4096, 4096, 4096], msw: 16, real_mode: true, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0), (1023, 0)], controls: [0, 0], flag_checkpoint: None, ldt_tr: [SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }, SystemSegment { selector: 0, base: 0, limit: 0, kind: 0 }], paging: false } && (image.fetch_base,image.start,image.end,image.system)==(65536, 0, 589824, [(1908, 1932), (1044480, 1045270), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]) && !machine.paging && image.segments[1].base.checked_add(537).and_then(|at|ram.contiguous_slice(at,3)).is_some_and(|s|s==[9, 208, 195]) => Some(block_219(cpu,ram,machine)),

   _=>None,
  };
  if let Some((n,_))=&result{self.blocks+=1;self.block_instructions+=*n as u64;}
  result
 }
 fn execute(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,decoded:&std::rc::Rc<guest::Decoded>,cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->Option<Result<(),crate::flow::ExecError>> {
  if !self.matched.as_ref().is_some_and(|(bytes,_)|std::sync::Arc::ptr_eq(bytes,&active.bytes)){
   self.matched=Some((active.bytes.clone(),active.bytes.as_slice()==ORIGINAL));
  }
  if !self.matched.as_ref().unwrap().1{return None;}
  let guest::Operation::Pil{program,dst,src,flow,address}=&decoded.operation else{return None};
  let result=match cpu.eip {
256 if image.fetch_signature(ram,cpu.eip,1).ok().is_some_and(|s|s[..1]==[252]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 0, width: 1 }), Flag(DF, 0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 0, width: 1 })), Token(Flag(DF, 0)), Token(Retire)], widths: [1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i100::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
257 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[184, 18, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 18, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 18, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i101::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
262 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[184, 0, 160]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 40960, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 40960, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i106::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
265 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[142, 192]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 16), WriteSegment(0, 0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 16)), Token(WriteSegment(0, 0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i109::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
267 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[186, 206, 3]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 974, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 974, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i10b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
270 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[184, 5, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 517, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 517, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i10e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
274 if image.fetch_signature(ram,cpu.eip,6).ok().is_some_and(|s|s[..6]==[199, 6, 65, 2, 80, 236]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Const(1, Value { bits: 60496, width: 16 }), Store(0, 1, Word), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Const(1, Value { bits: 60496, width: 16 })), Token(Store(0, 1, Word)), Token(Retire)], widths: [32, 16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i112::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
280 if image.fetch_signature(ram,cpu.eip,6).ok().is_some_and(|s|s[..6]==[199, 6, 71, 2, 0, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Const(1, Value { bits: 0, width: 16 }), Store(0, 1, Word), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Const(1, Value { bits: 0, width: 16 })), Token(Store(0, 1, Word)), Token(Retire)], widths: [32, 16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i118::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
286 if image.fetch_signature(ram,cpu.eip,6).ok().is_some_and(|s|s[..6]==[199, 6, 67, 2, 0, 220]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Const(1, Value { bits: 56320, width: 16 }), Store(0, 1, Word), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Const(1, Value { bits: 56320, width: 16 })), Token(Store(0, 1, Word)), Token(Retire)], widths: [32, 16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i11e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
292 if image.fetch_signature(ram,cpu.eip,6).ok().is_some_and(|s|s[..6]==[199, 6, 69, 2, 0, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Const(1, Value { bits: 0, width: 16 }), Store(0, 1, Word), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Const(1, Value { bits: 0, width: 16 })), Token(Store(0, 1, Word)), Token(Retire)], widths: [32, 16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i124::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
298 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[232, 128, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadIp(0, true, 32), Truncate(1, 0, 16), ReadDisp(2, 16), Binary(3, Add, 1, 2), Extend(4, 3, 32), ReadGpr(5, 4, 16), Const(6, Value { bits: 2, width: 16 }), Binary(7, Sub, 5, 6), Extend(8, 7, 32), StoreSegment(8, 1, Word, 2), Const(9, Value { bits: 1, width: 1 }), BranchIf(4, 9), WriteGpr(4, 7, 16), PreserveFlags(2261), Retire], packed: [Token(ReadIp(0, true, 32)), Token(Truncate(1, 0, 16)), Token(ReadDisp(2, 16)), Token(Binary(3, Add, 1, 2)), Token(Extend(4, 3, 32)), Token(ReadGpr(5, 4, 16)), Token(Const(6, Value { bits: 2, width: 16 })), Token(Binary(7, Sub, 5, 6)), Token(Extend(8, 7, 32)), Token(StoreSegment(8, 1, Word, 2)), Token(Const(9, Value { bits: 1, width: 1 })), Token(BranchIf(4, 9)), Token(WriteGpr(4, 7, 16)), Token(Retire)], widths: [32, 16, 16, 16, 32, 16, 16, 16, 32, 1], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i12a::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
301 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[162, 73, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Read(1, Src, 8), Store(0, 1, Byte), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Read(1, Src, 8)), Token(Store(0, 1, Byte)), Token(Retire)], widths: [32, 8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i12d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
304 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[139, 30, 69, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Word), Write(1), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Word)), Token(Write(1)), Token(Retire)], widths: [32, 16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i130::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
308 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[136, 217]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 8), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 8)), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i134::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
310 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[128, 225, 3]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 8), Const(1, Value { bits: 3, width: 8 }), Binary(2, And, 0, 1), Const(3, Value { bits: 0, width: 1 }), Bit(4, 2, 7), Zero(5, 2), Parity(6, 2), Write(2), Flag(CF, 3), Flag(OF, 3), UndefFlag(AF), Flag(SF, 4), Flag(ZF, 5), Flag(PF, 6), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 8, c: Value { bits: 3, width: 8 } }, Token(Binary(2, And, 0, 1)), LogicLazy { r: 2, w: 8 }, Token(Write(2)), Token(Retire)], widths: [8, 8, 8, 1, 1, 1, 1], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i136::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
313 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[208, 225]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 8), Const(1, Value { bits: 1, width: 8 }), Const(2, Value { bits: 31, width: 8 }), Binary(3, And, 1, 2), Const(4, Value { bits: 1, width: 8 }), Const(5, Value { bits: 8, width: 8 }), Const(6, Value { bits: 1, width: 1 }), Const(7, Value { bits: 0, width: 1 }), Zero(8, 3), Not(9, 8), Binary(10, Xor, 3, 4), Zero(11, 10), Binary(12, Sub, 3, 4), Shift(13, Left, 0, 3), Shift(14, Left, 0, 12), Bit(15, 14, 7), Less(16, 3, 5), Bit(17, 13, 7), Zero(18, 13), Parity(19, 13), Binary(20, Xor, 17, 15), Write(13), FlagIf(CF, 15, 16, 9), FlagIf(OF, 20, 11, 9), FlagIf(AF, 7, 7, 9), FlagIf(SF, 17, 6, 9), FlagIf(ZF, 18, 6, 9), FlagIf(PF, 19, 6, 9), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 8, c: Value { bits: 1, width: 8 } }, ShiftSetup { b: 1, count: 3, width_slot: 5, yes: 6, no: 7, active: 9, single: 11, last: 12, w: 8 }, Token(Shift(13, Left, 0, 3)), Token(Shift(14, Left, 0, 12)), Token(Bit(15, 14, 7)), Token(Less(16, 3, 5)), Token(Bit(17, 13, 7)), Token(Zero(18, 13)), Token(Parity(19, 13)), Token(Binary(20, Xor, 17, 15)), Token(Write(13)), ShiftFlags { cf: 15, cf_defined: 16, of: 20, single: 11, sf: 17, zf: 18, pf: 19, active: 9 }, Token(Retire)], widths: [8, 8, 8, 8, 8, 8, 1, 1, 1, 1, 8, 1, 8, 8, 8, 1, 1, 1, 1, 1, 1], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i139::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
315 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[180, 192]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 192, width: 8 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 192, width: 8 })), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i13b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
317 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[210, 236]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 8), Read(1, Src, 8), Const(2, Value { bits: 31, width: 8 }), Binary(3, And, 1, 2), Const(4, Value { bits: 1, width: 8 }), Const(5, Value { bits: 8, width: 8 }), Const(6, Value { bits: 1, width: 1 }), Const(7, Value { bits: 0, width: 1 }), Zero(8, 3), Not(9, 8), Binary(10, Xor, 3, 4), Zero(11, 10), Binary(12, Sub, 3, 4), Shift(13, Right, 0, 3), Shift(14, Right, 0, 12), Bit(15, 14, 0), Less(16, 3, 5), Bit(17, 13, 7), Zero(18, 13), Parity(19, 13), Bit(20, 0, 7), Write(13), FlagIf(CF, 15, 16, 9), FlagIf(OF, 20, 11, 9), FlagIf(AF, 7, 7, 9), FlagIf(SF, 17, 6, 9), FlagIf(ZF, 18, 6, 9), FlagIf(PF, 19, 6, 9), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 8 }, ShiftSetup { b: 1, count: 3, width_slot: 5, yes: 6, no: 7, active: 9, single: 11, last: 12, w: 8 }, Token(Shift(13, Right, 0, 3)), Token(Shift(14, Right, 0, 12)), Token(Bit(15, 14, 0)), Token(Less(16, 3, 5)), Token(Bit(17, 13, 7)), Token(Zero(18, 13)), Token(Parity(19, 13)), Token(Bit(20, 0, 7)), Token(Write(13)), ShiftFlags { cf: 15, cf_defined: 16, of: 20, single: 11, sf: 17, zf: 18, pf: 19, active: 9 }, Token(Retire)], widths: [8, 8, 8, 8, 8, 8, 1, 1, 1, 1, 8, 1, 8, 8, 8, 1, 1, 1, 1, 1, 1], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i13d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
319 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[176, 8]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 8, width: 8 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 8, width: 8 })), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i13f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
321 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[186, 206, 3]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 974, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 974, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i141::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
325 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[139, 62, 71, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Word), Write(1), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Word)), Token(Write(1)), Token(Retire)], widths: [32, 16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i145::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
329 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[209, 235]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 1, width: 16 }), Const(2, Value { bits: 31, width: 16 }), Binary(3, And, 1, 2), Const(4, Value { bits: 1, width: 16 }), Const(5, Value { bits: 16, width: 16 }), Const(6, Value { bits: 1, width: 1 }), Const(7, Value { bits: 0, width: 1 }), Zero(8, 3), Not(9, 8), Binary(10, Xor, 3, 4), Zero(11, 10), Binary(12, Sub, 3, 4), Shift(13, Right, 0, 3), Shift(14, Right, 0, 12), Bit(15, 14, 0), Less(16, 3, 5), Bit(17, 13, 15), Zero(18, 13), Parity(19, 13), Bit(20, 0, 15), Write(13), FlagIf(CF, 15, 16, 9), FlagIf(OF, 20, 11, 9), FlagIf(AF, 7, 7, 9), FlagIf(SF, 17, 6, 9), FlagIf(ZF, 18, 6, 9), FlagIf(PF, 19, 6, 9), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 1, width: 16 } }, ShiftSetup { b: 1, count: 3, width_slot: 5, yes: 6, no: 7, active: 9, single: 11, last: 12, w: 16 }, Token(Shift(13, Right, 0, 3)), Token(Shift(14, Right, 0, 12)), Token(Bit(15, 14, 0)), Token(Less(16, 3, 5)), Token(Bit(17, 13, 15)), Token(Zero(18, 13)), Token(Parity(19, 13)), Token(Bit(20, 0, 15)), Token(Write(13)), ShiftFlags { cf: 15, cf_defined: 16, of: 20, single: 11, sf: 17, zf: 18, pf: 19, active: 9 }, Token(Retire)], widths: [16, 16, 16, 16, 16, 16, 1, 1, 1, 1, 16, 1, 16, 16, 16, 1, 1, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i149::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
331 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[209, 235]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 1, width: 16 }), Const(2, Value { bits: 31, width: 16 }), Binary(3, And, 1, 2), Const(4, Value { bits: 1, width: 16 }), Const(5, Value { bits: 16, width: 16 }), Const(6, Value { bits: 1, width: 1 }), Const(7, Value { bits: 0, width: 1 }), Zero(8, 3), Not(9, 8), Binary(10, Xor, 3, 4), Zero(11, 10), Binary(12, Sub, 3, 4), Shift(13, Right, 0, 3), Shift(14, Right, 0, 12), Bit(15, 14, 0), Less(16, 3, 5), Bit(17, 13, 15), Zero(18, 13), Parity(19, 13), Bit(20, 0, 15), Write(13), FlagIf(CF, 15, 16, 9), FlagIf(OF, 20, 11, 9), FlagIf(AF, 7, 7, 9), FlagIf(SF, 17, 6, 9), FlagIf(ZF, 18, 6, 9), FlagIf(PF, 19, 6, 9), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 1, width: 16 } }, ShiftSetup { b: 1, count: 3, width_slot: 5, yes: 6, no: 7, active: 9, single: 11, last: 12, w: 16 }, Token(Shift(13, Right, 0, 3)), Token(Shift(14, Right, 0, 12)), Token(Bit(15, 14, 0)), Token(Less(16, 3, 5)), Token(Bit(17, 13, 15)), Token(Zero(18, 13)), Token(Parity(19, 13)), Token(Bit(20, 0, 15)), Token(Write(13)), ShiftFlags { cf: 15, cf_defined: 16, of: 20, single: 11, sf: 17, zf: 18, pf: 19, active: 9 }, Token(Retire)], widths: [16, 16, 16, 16, 16, 16, 1, 1, 1, 1, 16, 1, 16, 16, 16, 1, 1, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i14b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
333 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[1, 223]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Add, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Not(8, 7), Binary(9, Xor, 0, 5), Binary(10, And, 8, 9), Bit(11, 10, 15), Binary(12, Xor, 7, 5), Bit(13, 12, 4), Bit(14, 5, 15), Zero(15, 5), Parity(16, 5), Write(5), Flag(CF, 6), Flag(PF, 16), Flag(AF, 13), Flag(ZF, 15), Flag(SF, 14), Flag(OF, 11), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, AddLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Write(5)), Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i14d::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
335 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[38, 138, 5]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Byte), Write(1), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Byte)), Token(Write(1)), Token(Retire)], widths: [32, 8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i14f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
338 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[160, 73, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Byte), Write(1), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Byte)), Token(Write(1)), Token(Retire)], widths: [32, 8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i152::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
341 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[38, 136, 5]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Read(1, Src, 8), Store(0, 1, Byte), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Read(1, Src, 8)), Token(Store(0, 1, Byte)), Token(Retire)], widths: [32, 8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i155::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
344 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[38, 138, 101, 80]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Byte), Write(1), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Byte)), Token(Write(1)), Token(Retire)], widths: [32, 8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i158::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
348 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[38, 136, 69, 80]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Read(1, Src, 8), Store(0, 1, Byte), PreserveFlags(2261), Retire], packed: [Token(ReadAddress(0)), Token(Read(1, Src, 8)), Token(Store(0, 1, Byte)), Token(Retire)], widths: [32, 8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i15c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
352 if image.fetch_signature(ram,cpu.eip,5).ok().is_some_and(|s|s[..5]==[131, 6, 67, 2, 42]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Word), Const(2, Value { bits: 42, width: 16 }), Extend(3, 1, 17), Extend(4, 2, 17), Binary(5, Add, 3, 4), Truncate(6, 5, 16), Bit(7, 5, 16), Binary(8, Xor, 1, 2), Not(9, 8), Binary(10, Xor, 1, 6), Binary(11, And, 9, 10), Bit(12, 11, 15), Binary(13, Xor, 8, 6), Bit(14, 13, 4), Bit(15, 6, 15), Zero(16, 6), Parity(17, 6), Store(0, 6, Word), Flag(CF, 7), Flag(PF, 17), Flag(AF, 14), Flag(ZF, 16), Flag(SF, 15), Flag(OF, 12), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Word)), Token(Const(2, Value { bits: 42, width: 16 })), AddLazy { a: 1, b: 2, r: 6, w: 16, write_cf: true }, Token(Store(0, 6, Word)), Token(Retire)], widths: [32, 16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i160::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
357 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[255, 6, 69, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Word), Const(2, Value { bits: 1, width: 16 }), Extend(3, 1, 17), Extend(4, 2, 17), Binary(5, Add, 3, 4), Truncate(6, 5, 16), Bit(7, 5, 16), Binary(8, Xor, 1, 2), Not(9, 8), Binary(10, Xor, 1, 6), Binary(11, And, 9, 10), Bit(12, 11, 15), Binary(13, Xor, 8, 6), Bit(14, 13, 4), Bit(15, 6, 15), Zero(16, 6), Parity(17, 6), Store(0, 6, Word), PreserveFlags(1), Flag(PF, 17), Flag(AF, 14), Flag(ZF, 16), Flag(SF, 15), Flag(OF, 12), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Word)), Token(Const(2, Value { bits: 1, width: 16 })), AddLazy { a: 1, b: 2, r: 6, w: 16, write_cf: false }, Token(Store(0, 6, Word)), Token(Retire)], widths: [32, 16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i165::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
361 if image.fetch_signature(ram,cpu.eip,6).ok().is_some_and(|s|s[..6]==[129, 62, 69, 2, 64, 1]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Word), Const(2, Value { bits: 320, width: 16 }), Extend(3, 1, 17), Extend(4, 2, 17), Binary(5, Sub, 3, 4), Truncate(6, 5, 16), Bit(7, 5, 16), Binary(8, Xor, 1, 2), Binary(9, Xor, 1, 6), Binary(10, And, 8, 9), Bit(11, 10, 15), Binary(12, Xor, 8, 6), Bit(13, 12, 4), Bit(14, 6, 15), Zero(15, 6), Parity(16, 6), Flag(CF, 7), Flag(PF, 16), Flag(AF, 13), Flag(ZF, 15), Flag(SF, 14), Flag(OF, 11), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Word)), Token(Const(2, Value { bits: 320, width: 16 })), SubLazy { a: 1, b: 2, r: 6, w: 16, write_cf: true }, Token(Retire)], widths: [32, 16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i169::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
367 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[114, 185]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadFlag(0, OF), Not(1, 0), ReadFlag(2, CF), Not(3, 2), ReadFlag(4, ZF), Not(5, 4), Binary(6, Or, 2, 4), Not(7, 6), ReadFlag(8, SF), Not(9, 8), ReadFlag(10, PF), Not(11, 10), Binary(12, Xor, 8, 0), Not(13, 12), Binary(14, Or, 12, 4), Not(15, 14), PreserveFlags(0), ReadIp(16, true, 32), Truncate(17, 16, 16), ReadDisp(18, 16), Binary(19, Add, 17, 18), Extend(20, 19, 32), BranchIf(20, 2), PreserveFlags(2261), Retire], packed: [Jcc16 { cond: 2, w: 16 }, Token(Retire)], widths: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i16f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
369 if image.fetch_signature(ram,cpu.eip,5).ok().is_some_and(|s|s[..5]==[131, 6, 65, 2, 42]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Word), Const(2, Value { bits: 42, width: 16 }), Extend(3, 1, 17), Extend(4, 2, 17), Binary(5, Add, 3, 4), Truncate(6, 5, 16), Bit(7, 5, 16), Binary(8, Xor, 1, 2), Not(9, 8), Binary(10, Xor, 1, 6), Binary(11, And, 9, 10), Bit(12, 11, 15), Binary(13, Xor, 8, 6), Bit(14, 13, 4), Bit(15, 6, 15), Zero(16, 6), Parity(17, 6), Store(0, 6, Word), Flag(CF, 7), Flag(PF, 17), Flag(AF, 14), Flag(ZF, 16), Flag(SF, 15), Flag(OF, 12), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Word)), Token(Const(2, Value { bits: 42, width: 16 })), AddLazy { a: 1, b: 2, r: 6, w: 16, write_cf: true }, Token(Store(0, 6, Word)), Token(Retire)], widths: [32, 16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i171::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
374 if image.fetch_signature(ram,cpu.eip,6).ok().is_some_and(|s|s[..6]==[129, 6, 71, 2, 160, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Word), Const(2, Value { bits: 160, width: 16 }), Extend(3, 1, 17), Extend(4, 2, 17), Binary(5, Add, 3, 4), Truncate(6, 5, 16), Bit(7, 5, 16), Binary(8, Xor, 1, 2), Not(9, 8), Binary(10, Xor, 1, 6), Binary(11, And, 9, 10), Bit(12, 11, 15), Binary(13, Xor, 8, 6), Bit(14, 13, 4), Bit(15, 6, 15), Zero(16, 6), Parity(17, 6), Store(0, 6, Word), Flag(CF, 7), Flag(PF, 17), Flag(AF, 14), Flag(ZF, 16), Flag(SF, 15), Flag(OF, 12), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Word)), Token(Const(2, Value { bits: 160, width: 16 })), AddLazy { a: 1, b: 2, r: 6, w: 16, write_cf: true }, Token(Store(0, 6, Word)), Token(Retire)], widths: [32, 16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i176::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
380 if image.fetch_signature(ram,cpu.eip,6).ok().is_some_and(|s|s[..6]==[129, 62, 71, 2, 0, 150]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadAddress(0), Load(1, 0, Word), Const(2, Value { bits: 38400, width: 16 }), Extend(3, 1, 17), Extend(4, 2, 17), Binary(5, Sub, 3, 4), Truncate(6, 5, 16), Bit(7, 5, 16), Binary(8, Xor, 1, 2), Binary(9, Xor, 1, 6), Binary(10, And, 8, 9), Bit(11, 10, 15), Binary(12, Xor, 8, 6), Bit(13, 12, 4), Bit(14, 6, 15), Zero(15, 6), Parity(16, 6), Flag(CF, 7), Flag(PF, 16), Flag(AF, 13), Flag(ZF, 15), Flag(SF, 14), Flag(OF, 11), Retire], packed: [Token(ReadAddress(0)), Token(Load(1, 0, Word)), Token(Const(2, Value { bits: 38400, width: 16 })), SubLazy { a: 1, b: 2, r: 6, w: 16, write_cf: true }, Token(Retire)], widths: [32, 16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i17c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
386 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[114, 154]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadFlag(0, OF), Not(1, 0), ReadFlag(2, CF), Not(3, 2), ReadFlag(4, ZF), Not(5, 4), Binary(6, Or, 2, 4), Not(7, 6), ReadFlag(8, SF), Not(9, 8), ReadFlag(10, PF), Not(11, 10), Binary(12, Xor, 8, 0), Not(13, 12), Binary(14, Or, 12, 4), Not(15, 14), PreserveFlags(0), ReadIp(16, true, 32), Truncate(17, 16, 16), ReadDisp(18, 16), Binary(19, Add, 17, 18), Extend(20, 19, 32), BranchIf(20, 2), PreserveFlags(2261), Retire], packed: [Jcc16 { cond: 2, w: 16 }, Token(Retire)], widths: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i182::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
388 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[186, 206, 3]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 974, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 974, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i184::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
391 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[184, 8, 255]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 65288, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 65288, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i187::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
395 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[184, 5, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 5, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 5, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i18b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
399 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[180, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 2, width: 8 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 2, width: 8 })), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i18f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
401 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[48, 255]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 8), Read(1, Src, 8), Binary(2, Xor, 0, 1), Const(3, Value { bits: 0, width: 1 }), Bit(4, 2, 7), Zero(5, 2), Parity(6, 2), Write(2), Flag(CF, 3), Flag(OF, 3), UndefFlag(AF), Flag(SF, 4), Flag(ZF, 5), Flag(PF, 6), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 8 }, Token(Binary(2, Xor, 0, 1)), LogicLazy { r: 2, w: 8 }, Token(Write(2)), Token(Retire)], widths: [8, 8, 8, 1, 1, 1, 1], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i191::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
403 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[186, 0, 29]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 7424, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 7424, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i193::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
408 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[186, 28, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 540, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 540, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i198::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
411 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[180, 9]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 9, width: 8 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 9, width: 8 })), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i19b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
415 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[48, 228]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 8), Read(1, Src, 8), Binary(2, Xor, 0, 1), Const(3, Value { bits: 0, width: 1 }), Bit(4, 2, 7), Zero(5, 2), Parity(6, 2), Write(2), Flag(CF, 3), Flag(OF, 3), UndefFlag(AF), Flag(SF, 4), Flag(ZF, 5), Flag(PF, 6), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 8 }, Token(Binary(2, Xor, 0, 1)), LogicLazy { r: 2, w: 8 }, Token(Write(2)), Token(Retire)], widths: [8, 8, 8, 1, 1, 1, 1], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i19f::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
429 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[49, 246]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Binary(2, Xor, 0, 1), Const(3, Value { bits: 0, width: 1 }), Bit(4, 2, 15), Zero(5, 2), Parity(6, 2), Write(2), Flag(CF, 3), Flag(OF, 3), UndefFlag(AF), Flag(SF, 4), Flag(ZF, 5), Flag(PF, 6), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, Token(Binary(2, Xor, 0, 1)), LogicLazy { r: 2, w: 16 }, Token(Write(2)), Token(Retire)], widths: [16, 16, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1ad::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
431 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[49, 255]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Binary(2, Xor, 0, 1), Const(3, Value { bits: 0, width: 1 }), Bit(4, 2, 15), Zero(5, 2), Parity(6, 2), Write(2), Flag(CF, 3), Flag(OF, 3), UndefFlag(AF), Flag(SF, 4), Flag(ZF, 5), Flag(PF, 6), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, Token(Binary(2, Xor, 0, 1)), LogicLazy { r: 2, w: 16 }, Token(Write(2)), Token(Retire)], widths: [16, 16, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1af::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
433 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[189, 30, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 30, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 30, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1b1::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
436 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[129, 254, 0, 32]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 8192, width: 16 }), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Sub, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Binary(8, Xor, 0, 5), Binary(9, And, 7, 8), Bit(10, 9, 15), Binary(11, Xor, 7, 5), Bit(12, 11, 4), Bit(13, 5, 15), Zero(14, 5), Parity(15, 5), Flag(CF, 6), Flag(PF, 15), Flag(AF, 12), Flag(ZF, 14), Flag(SF, 13), Flag(OF, 10), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 8192, width: 16 } }, SubLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1b4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
440 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[125, 73]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadFlag(0, OF), Not(1, 0), ReadFlag(2, CF), Not(3, 2), ReadFlag(4, ZF), Not(5, 4), Binary(6, Or, 2, 4), Not(7, 6), ReadFlag(8, SF), Not(9, 8), ReadFlag(10, PF), Not(11, 10), Binary(12, Xor, 8, 0), Not(13, 12), Binary(14, Or, 12, 4), Not(15, 14), PreserveFlags(0), ReadIp(16, true, 32), Truncate(17, 16, 16), ReadDisp(18, 16), Binary(19, Add, 17, 18), Extend(20, 19, 32), BranchIf(20, 13), PreserveFlags(2261), Retire], packed: [Jcc16 { cond: 13, w: 16 }, Token(Retire)], widths: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1b8::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
442 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[129, 254, 0, 224]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 57344, width: 16 }), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Sub, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Binary(8, Xor, 0, 5), Binary(9, And, 7, 8), Bit(10, 9, 15), Binary(11, Xor, 7, 5), Bit(12, 11, 4), Bit(13, 5, 15), Zero(14, 5), Parity(15, 5), Flag(CF, 6), Flag(PF, 15), Flag(AF, 12), Flag(ZF, 14), Flag(SF, 13), Flag(OF, 10), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 57344, width: 16 } }, SubLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1ba::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
446 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[126, 67]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadFlag(0, OF), Not(1, 0), ReadFlag(2, CF), Not(3, 2), ReadFlag(4, ZF), Not(5, 4), Binary(6, Or, 2, 4), Not(7, 6), ReadFlag(8, SF), Not(9, 8), ReadFlag(10, PF), Not(11, 10), Binary(12, Xor, 8, 0), Not(13, 12), Binary(14, Or, 12, 4), Not(15, 14), PreserveFlags(0), ReadIp(16, true, 32), Truncate(17, 16, 16), ReadDisp(18, 16), Binary(19, Add, 17, 18), Extend(20, 19, 32), BranchIf(20, 14), PreserveFlags(2261), Retire], packed: [Jcc16 { cond: 14, w: 16 }, Token(Retire)], widths: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1be::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
448 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[129, 255, 0, 32]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 8192, width: 16 }), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Sub, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Binary(8, Xor, 0, 5), Binary(9, And, 7, 8), Bit(10, 9, 15), Binary(11, Xor, 7, 5), Bit(12, 11, 4), Bit(13, 5, 15), Zero(14, 5), Parity(15, 5), Flag(CF, 6), Flag(PF, 15), Flag(AF, 12), Flag(ZF, 14), Flag(SF, 13), Flag(OF, 10), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 8192, width: 16 } }, SubLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1c0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
452 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[125, 61]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadFlag(0, OF), Not(1, 0), ReadFlag(2, CF), Not(3, 2), ReadFlag(4, ZF), Not(5, 4), Binary(6, Or, 2, 4), Not(7, 6), ReadFlag(8, SF), Not(9, 8), ReadFlag(10, PF), Not(11, 10), Binary(12, Xor, 8, 0), Not(13, 12), Binary(14, Or, 12, 4), Not(15, 14), PreserveFlags(0), ReadIp(16, true, 32), Truncate(17, 16, 16), ReadDisp(18, 16), Binary(19, Add, 17, 18), Extend(20, 19, 32), BranchIf(20, 13), PreserveFlags(2261), Retire], packed: [Jcc16 { cond: 13, w: 16 }, Token(Retire)], widths: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1c4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
454 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[129, 255, 0, 224]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 57344, width: 16 }), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Sub, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Binary(8, Xor, 0, 5), Binary(9, And, 7, 8), Bit(10, 9, 15), Binary(11, Xor, 7, 5), Bit(12, 11, 4), Bit(13, 5, 15), Zero(14, 5), Parity(15, 5), Flag(CF, 6), Flag(PF, 15), Flag(AF, 12), Flag(ZF, 14), Flag(SF, 13), Flag(OF, 10), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 57344, width: 16 } }, SubLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1c6::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
458 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[126, 55]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadFlag(0, OF), Not(1, 0), ReadFlag(2, CF), Not(3, 2), ReadFlag(4, ZF), Not(5, 4), Binary(6, Or, 2, 4), Not(7, 6), ReadFlag(8, SF), Not(9, 8), ReadFlag(10, PF), Not(11, 10), Binary(12, Xor, 8, 0), Not(13, 12), Binary(14, Or, 12, 4), Not(15, 14), PreserveFlags(0), ReadIp(16, true, 32), Truncate(17, 16, 16), ReadDisp(18, 16), Binary(19, Add, 17, 18), Extend(20, 19, 32), BranchIf(20, 14), PreserveFlags(2261), Retire], packed: [Jcc16 { cond: 14, w: 16 }, Token(Retire)], widths: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1ca::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
460 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[137, 240]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 16), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 16)), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1cc::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
462 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[247, 238]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadGpr(0, 0, 16), Read(1, Src, 16), SignExtend(2, 0, 32), SignExtend(3, 1, 32), Binary(4, Mul, 2, 3), Truncate(5, 4, 16), SignExtend(6, 5, 32), Binary(7, Xor, 4, 6), Zero(8, 7), Not(9, 8), Const(10, Value { bits: 16, width: 32 }), Shift(11, Right, 4, 10), Truncate(12, 11, 16), WriteGpr(0, 5, 16), WriteGpr(2, 12, 16), Flag(CF, 9), Flag(OF, 9), UndefFlag(SF), UndefFlag(ZF), UndefFlag(AF), UndefFlag(PF), Retire], packed: [Token(ReadGpr(0, 0, 16)), Token(Read(1, Src, 16)), Token(SignExtend(2, 0, 32)), Token(SignExtend(3, 1, 32)), Token(Binary(4, Mul, 2, 3)), Token(Truncate(5, 4, 16)), Token(SignExtend(6, 5, 32)), Token(Binary(7, Xor, 4, 6)), Token(Zero(8, 7)), Token(Not(9, 8)), Token(Const(10, Value { bits: 16, width: 32 })), Token(Shift(11, Right, 4, 10)), Token(Truncate(12, 11, 16)), Token(WriteGpr(0, 5, 16)), Token(WriteGpr(2, 12, 16)), Token(Flag(CF, 9)), Token(Flag(OF, 9)), Token(UndefFlag(SF)), Token(UndefFlag(ZF)), Token(UndefFlag(AF)), Token(UndefFlag(PF)), Token(Retire)], widths: [16, 16, 32, 32, 32, 16, 32, 32, 1, 1, 32, 32, 16], register_effects: 2, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i1ce::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
464 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[232, 62, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadIp(0, true, 32), Truncate(1, 0, 16), ReadDisp(2, 16), Binary(3, Add, 1, 2), Extend(4, 3, 32), ReadGpr(5, 4, 16), Const(6, Value { bits: 2, width: 16 }), Binary(7, Sub, 5, 6), Extend(8, 7, 32), StoreSegment(8, 1, Word, 2), Const(9, Value { bits: 1, width: 1 }), BranchIf(4, 9), WriteGpr(4, 7, 16), PreserveFlags(2261), Retire], packed: [Token(ReadIp(0, true, 32)), Token(Truncate(1, 0, 16)), Token(ReadDisp(2, 16)), Token(Binary(3, Add, 1, 2)), Token(Extend(4, 3, 32)), Token(ReadGpr(5, 4, 16)), Token(Const(6, Value { bits: 2, width: 16 })), Token(Binary(7, Sub, 5, 6)), Token(Extend(8, 7, 32)), Token(StoreSegment(8, 1, Word, 2)), Token(Const(9, Value { bits: 1, width: 1 })), Token(BranchIf(4, 9)), Token(WriteGpr(4, 7, 16)), Token(Retire)], widths: [32, 16, 16, 16, 32, 16, 16, 16, 32, 1], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1d0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
467 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[137, 195]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 16), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 16)), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1d3::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
469 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[137, 248]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 16), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 16)), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1d5::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
471 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[247, 239]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadGpr(0, 0, 16), Read(1, Src, 16), SignExtend(2, 0, 32), SignExtend(3, 1, 32), Binary(4, Mul, 2, 3), Truncate(5, 4, 16), SignExtend(6, 5, 32), Binary(7, Xor, 4, 6), Zero(8, 7), Not(9, 8), Const(10, Value { bits: 16, width: 32 }), Shift(11, Right, 4, 10), Truncate(12, 11, 16), WriteGpr(0, 5, 16), WriteGpr(2, 12, 16), Flag(CF, 9), Flag(OF, 9), UndefFlag(SF), UndefFlag(ZF), UndefFlag(AF), UndefFlag(PF), Retire], packed: [Token(ReadGpr(0, 0, 16)), Token(Read(1, Src, 16)), Token(SignExtend(2, 0, 32)), Token(SignExtend(3, 1, 32)), Token(Binary(4, Mul, 2, 3)), Token(Truncate(5, 4, 16)), Token(SignExtend(6, 5, 32)), Token(Binary(7, Xor, 4, 6)), Token(Zero(8, 7)), Token(Not(9, 8)), Token(Const(10, Value { bits: 16, width: 32 })), Token(Shift(11, Right, 4, 10)), Token(Truncate(12, 11, 16)), Token(WriteGpr(0, 5, 16)), Token(WriteGpr(2, 12, 16)), Token(Flag(CF, 9)), Token(Flag(OF, 9)), Token(UndefFlag(SF)), Token(UndefFlag(ZF)), Token(UndefFlag(AF)), Token(UndefFlag(PF)), Token(Retire)], widths: [16, 16, 32, 32, 32, 16, 32, 32, 1, 1, 32, 32, 16], register_effects: 2, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i1d7::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
473 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[232, 53, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadIp(0, true, 32), Truncate(1, 0, 16), ReadDisp(2, 16), Binary(3, Add, 1, 2), Extend(4, 3, 32), ReadGpr(5, 4, 16), Const(6, Value { bits: 2, width: 16 }), Binary(7, Sub, 5, 6), Extend(8, 7, 32), StoreSegment(8, 1, Word, 2), Const(9, Value { bits: 1, width: 1 }), BranchIf(4, 9), WriteGpr(4, 7, 16), PreserveFlags(2261), Retire], packed: [Token(ReadIp(0, true, 32)), Token(Truncate(1, 0, 16)), Token(ReadDisp(2, 16)), Token(Binary(3, Add, 1, 2)), Token(Extend(4, 3, 32)), Token(ReadGpr(5, 4, 16)), Token(Const(6, Value { bits: 2, width: 16 })), Token(Binary(7, Sub, 5, 6)), Token(Extend(8, 7, 32)), Token(StoreSegment(8, 1, Word, 2)), Token(Const(9, Value { bits: 1, width: 1 })), Token(BranchIf(4, 9)), Token(WriteGpr(4, 7, 16)), Token(Retire)], widths: [32, 16, 16, 16, 32, 16, 16, 16, 32, 1], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1d9::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
476 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[137, 217]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 16), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 16)), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1dc::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
478 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[1, 193]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Add, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Not(8, 7), Binary(9, Xor, 0, 5), Binary(10, And, 8, 9), Bit(11, 10, 15), Binary(12, Xor, 7, 5), Bit(13, 12, 4), Bit(14, 5, 15), Zero(15, 5), Parity(16, 5), Write(5), Flag(CF, 6), Flag(PF, 16), Flag(AF, 13), Flag(ZF, 15), Flag(SF, 14), Flag(OF, 11), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, AddLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Write(5)), Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1de::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
480 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[129, 249, 0, 64]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 16384, width: 16 }), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Sub, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Binary(8, Xor, 0, 5), Binary(9, And, 7, 8), Bit(10, 9, 15), Binary(11, Xor, 7, 5), Bit(12, 11, 4), Bit(13, 5, 15), Zero(14, 5), Parity(15, 5), Flag(CF, 6), Flag(PF, 15), Flag(AF, 12), Flag(ZF, 14), Flag(SF, 13), Flag(OF, 10), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 16384, width: 16 } }, SubLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1e0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
484 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[127, 29]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadFlag(0, OF), Not(1, 0), ReadFlag(2, CF), Not(3, 2), ReadFlag(4, ZF), Not(5, 4), Binary(6, Or, 2, 4), Not(7, 6), ReadFlag(8, SF), Not(9, 8), ReadFlag(10, PF), Not(11, 10), Binary(12, Xor, 8, 0), Not(13, 12), Binary(14, Or, 12, 4), Not(15, 14), PreserveFlags(0), ReadIp(16, true, 32), Truncate(17, 16, 16), ReadDisp(18, 16), Binary(19, Add, 17, 18), Extend(20, 19, 32), BranchIf(20, 15), PreserveFlags(2261), Retire], packed: [Jcc16 { cond: 15, w: 16 }, Token(Retire)], widths: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1e4::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
486 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[41, 195]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Sub, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Binary(8, Xor, 0, 5), Binary(9, And, 7, 8), Bit(10, 9, 15), Binary(11, Xor, 7, 5), Bit(12, 11, 4), Bit(13, 5, 15), Zero(14, 5), Parity(15, 5), Write(5), Flag(CF, 6), Flag(PF, 15), Flag(AF, 12), Flag(ZF, 14), Flag(SF, 13), Flag(OF, 10), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, SubLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Write(5)), Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1e6::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
488 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[3, 30, 67, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), ReadAddress(1), Load(2, 1, Word), Extend(3, 0, 17), Extend(4, 2, 17), Binary(5, Add, 3, 4), Truncate(6, 5, 16), Bit(7, 5, 16), Binary(8, Xor, 0, 2), Not(9, 8), Binary(10, Xor, 0, 6), Binary(11, And, 9, 10), Bit(12, 11, 15), Binary(13, Xor, 8, 6), Bit(14, 13, 4), Bit(15, 6, 15), Zero(16, 6), Parity(17, 6), Write(6), Flag(CF, 7), Flag(PF, 17), Flag(AF, 14), Flag(ZF, 16), Flag(SF, 15), Flag(OF, 12), Retire], packed: [Token(Read(0, Dst, 16)), Token(ReadAddress(1)), Token(Load(2, 1, Word)), AddLazy { a: 0, b: 2, r: 6, w: 16, write_cf: true }, Token(Write(6)), Token(Retire)], widths: [16, 32, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1e8::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
492 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[137, 240]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 16), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 16)), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1ec::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
494 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[247, 239]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadGpr(0, 0, 16), Read(1, Src, 16), SignExtend(2, 0, 32), SignExtend(3, 1, 32), Binary(4, Mul, 2, 3), Truncate(5, 4, 16), SignExtend(6, 5, 32), Binary(7, Xor, 4, 6), Zero(8, 7), Not(9, 8), Const(10, Value { bits: 16, width: 32 }), Shift(11, Right, 4, 10), Truncate(12, 11, 16), WriteGpr(0, 5, 16), WriteGpr(2, 12, 16), Flag(CF, 9), Flag(OF, 9), UndefFlag(SF), UndefFlag(ZF), UndefFlag(AF), UndefFlag(PF), Retire], packed: [Token(ReadGpr(0, 0, 16)), Token(Read(1, Src, 16)), Token(SignExtend(2, 0, 32)), Token(SignExtend(3, 1, 32)), Token(Binary(4, Mul, 2, 3)), Token(Truncate(5, 4, 16)), Token(SignExtend(6, 5, 32)), Token(Binary(7, Xor, 4, 6)), Token(Zero(8, 7)), Token(Not(9, 8)), Token(Const(10, Value { bits: 16, width: 32 })), Token(Shift(11, Right, 4, 10)), Token(Truncate(12, 11, 16)), Token(WriteGpr(0, 5, 16)), Token(WriteGpr(2, 12, 16)), Token(Flag(CF, 9)), Token(Flag(OF, 9)), Token(UndefFlag(SF)), Token(UndefFlag(ZF)), Token(UndefFlag(AF)), Token(UndefFlag(PF)), Token(Retire)], widths: [16, 16, 32, 32, 32, 16, 32, 32, 1, 1, 32, 32, 16], register_effects: 2, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i1ee::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
496 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[232, 30, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadIp(0, true, 32), Truncate(1, 0, 16), ReadDisp(2, 16), Binary(3, Add, 1, 2), Extend(4, 3, 32), ReadGpr(5, 4, 16), Const(6, Value { bits: 2, width: 16 }), Binary(7, Sub, 5, 6), Extend(8, 7, 32), StoreSegment(8, 1, Word, 2), Const(9, Value { bits: 1, width: 1 }), BranchIf(4, 9), WriteGpr(4, 7, 16), PreserveFlags(2261), Retire], packed: [Token(ReadIp(0, true, 32)), Token(Truncate(1, 0, 16)), Token(ReadDisp(2, 16)), Token(Binary(3, Add, 1, 2)), Token(Extend(4, 3, 32)), Token(ReadGpr(5, 4, 16)), Token(Const(6, Value { bits: 2, width: 16 })), Token(Binary(7, Sub, 5, 6)), Token(Extend(8, 7, 32)), Token(StoreSegment(8, 1, Word, 2)), Token(Const(9, Value { bits: 1, width: 1 })), Token(BranchIf(4, 9)), Token(WriteGpr(4, 7, 16)), Token(Retire)], widths: [32, 16, 16, 16, 32, 16, 16, 16, 32, 1], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1f0::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
499 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[209, 224]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 1, width: 16 }), Const(2, Value { bits: 31, width: 16 }), Binary(3, And, 1, 2), Const(4, Value { bits: 1, width: 16 }), Const(5, Value { bits: 16, width: 16 }), Const(6, Value { bits: 1, width: 1 }), Const(7, Value { bits: 0, width: 1 }), Zero(8, 3), Not(9, 8), Binary(10, Xor, 3, 4), Zero(11, 10), Binary(12, Sub, 3, 4), Shift(13, Left, 0, 3), Shift(14, Left, 0, 12), Bit(15, 14, 15), Less(16, 3, 5), Bit(17, 13, 15), Zero(18, 13), Parity(19, 13), Binary(20, Xor, 17, 15), Write(13), FlagIf(CF, 15, 16, 9), FlagIf(OF, 20, 11, 9), FlagIf(AF, 7, 7, 9), FlagIf(SF, 17, 6, 9), FlagIf(ZF, 18, 6, 9), FlagIf(PF, 19, 6, 9), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 1, width: 16 } }, ShiftSetup { b: 1, count: 3, width_slot: 5, yes: 6, no: 7, active: 9, single: 11, last: 12, w: 16 }, Token(Shift(13, Left, 0, 3)), Token(Shift(14, Left, 0, 12)), Token(Bit(15, 14, 15)), Token(Less(16, 3, 5)), Token(Bit(17, 13, 15)), Token(Zero(18, 13)), Token(Parity(19, 13)), Token(Binary(20, Xor, 17, 15)), Token(Write(13)), ShiftFlags { cf: 15, cf_defined: 16, of: 20, single: 11, sf: 17, zf: 18, pf: 19, active: 9 }, Token(Retire)], widths: [16, 16, 16, 16, 16, 16, 1, 1, 1, 1, 16, 1, 16, 16, 16, 1, 1, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i1f3::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
501 if image.fetch_signature(ram,cpu.eip,4).ok().is_some_and(|s|s[..4]==[3, 6, 65, 2]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), ReadAddress(1), Load(2, 1, Word), Extend(3, 0, 17), Extend(4, 2, 17), Binary(5, Add, 3, 4), Truncate(6, 5, 16), Bit(7, 5, 16), Binary(8, Xor, 0, 2), Not(9, 8), Binary(10, Xor, 0, 6), Binary(11, And, 9, 10), Bit(12, 11, 15), Binary(13, Xor, 8, 6), Bit(14, 13, 4), Bit(15, 6, 15), Zero(16, 6), Parity(17, 6), Write(6), Flag(CF, 7), Flag(PF, 17), Flag(AF, 14), Flag(ZF, 16), Flag(SF, 15), Flag(OF, 12), Retire], packed: [Token(Read(0, Dst, 16)), Token(ReadAddress(1)), Token(Load(2, 1, Word)), AddLazy { a: 0, b: 2, r: 6, w: 16, write_cf: true }, Token(Write(6)), Token(Retire)], widths: [16, 32, 16, 17, 17, 17, 16, 1, 16, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: true, has_read_address: true, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1f5::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
505 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[137, 199]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 16), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 16)), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1f9::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
507 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[137, 222]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 16), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 16)), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1fb::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
509 if image.fetch_signature(ram,cpu.eip,1).ok().is_some_and(|s|s[..1]==[77]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Const(1, Value { bits: 1, width: 16 }), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Sub, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Binary(8, Xor, 0, 5), Binary(9, And, 7, 8), Bit(10, 9, 15), Binary(11, Xor, 7, 5), Bit(12, 11, 4), Bit(13, 5, 15), Zero(14, 5), Parity(15, 5), Write(5), PreserveFlags(1), Flag(PF, 15), Flag(AF, 12), Flag(ZF, 14), Flag(SF, 13), Flag(OF, 10), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 16, c: Value { bits: 1, width: 16 } }, SubLazy { a: 0, b: 1, r: 5, w: 16, write_cf: false }, Token(Write(5)), Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1fd::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
510 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[117, 180]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadFlag(0, OF), Not(1, 0), ReadFlag(2, CF), Not(3, 2), ReadFlag(4, ZF), Not(5, 4), Binary(6, Or, 2, 4), Not(7, 6), ReadFlag(8, SF), Not(9, 8), ReadFlag(10, PF), Not(11, 10), Binary(12, Xor, 8, 0), Not(13, 12), Binary(14, Or, 12, 4), Not(15, 14), PreserveFlags(0), ReadIp(16, true, 32), Truncate(17, 16, 16), ReadDisp(18, 16), Binary(19, Add, 17, 18), Extend(20, 19, 32), BranchIf(20, 5), PreserveFlags(2261), Retire], packed: [Jcc16 { cond: 5, w: 16 }, Token(Retire)], widths: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 32, 16, 16, 16, 32], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: true, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i1fe::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
512 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[48, 192]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 8), Read(1, Src, 8), Binary(2, Xor, 0, 1), Const(3, Value { bits: 0, width: 1 }), Bit(4, 2, 7), Zero(5, 2), Parity(6, 2), Write(2), Flag(CF, 3), Flag(OF, 3), UndefFlag(AF), Flag(SF, 4), Flag(ZF, 5), Flag(PF, 6), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 8 }, Token(Binary(2, Xor, 0, 1)), LogicLazy { r: 2, w: 8 }, Token(Write(2)), Token(Retire)], widths: [8, 8, 8, 1, 1, 1, 1], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i200::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
514 if image.fetch_signature(ram,cpu.eip,1).ok().is_some_and(|s|s[..1]==[195]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadGpr(0, 4, 16), Extend(1, 0, 32), LoadSegment(2, 1, Word, 2), Extend(3, 2, 32), Const(4, Value { bits: 1, width: 1 }), BranchIf(3, 4), Const(5, Value { bits: 2, width: 16 }), Const(6, Value { bits: 0, width: 16 }), Binary(7, Add, 0, 5), Binary(8, Add, 7, 6), WriteGpr(4, 8, 16), PreserveFlags(2261), Retire], packed: [Token(ReadGpr(0, 4, 16)), Token(Extend(1, 0, 32)), Token(LoadSegment(2, 1, Word, 2)), Token(Extend(3, 2, 32)), Token(Const(4, Value { bits: 1, width: 1 })), Token(BranchIf(3, 4)), Token(Const(5, Value { bits: 2, width: 16 })), Token(Const(6, Value { bits: 0, width: 16 })), Token(Binary(7, Add, 0, 5)), Token(Binary(8, Add, 7, 6)), Token(WriteGpr(4, 8, 16)), Token(Retire)], widths: [16, 32, 16, 32, 1, 16, 16, 16, 16], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: false, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i202::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
515 if image.fetch_signature(ram,cpu.eip,3).ok().is_some_and(|s|s[..3]==[184, 30, 0]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 30, width: 16 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 30, width: 16 })), Token(Write(0)), Token(Retire)], widths: [16], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i203::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
518 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[41, 232]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Extend(2, 0, 17), Extend(3, 1, 17), Binary(4, Sub, 2, 3), Truncate(5, 4, 16), Bit(6, 4, 16), Binary(7, Xor, 0, 1), Binary(8, Xor, 0, 5), Binary(9, And, 7, 8), Bit(10, 9, 15), Binary(11, Xor, 7, 5), Bit(12, 11, 4), Bit(13, 5, 15), Zero(14, 5), Parity(15, 5), Write(5), Flag(CF, 6), Flag(PF, 15), Flag(AF, 12), Flag(ZF, 14), Flag(SF, 13), Flag(OF, 10), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, SubLazy { a: 0, b: 1, r: 5, w: 16, write_cf: true }, Token(Write(5)), Token(Retire)], widths: [16, 16, 17, 17, 17, 16, 1, 16, 16, 16, 1, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i206::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
520 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[179, 15]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 15, width: 8 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 15, width: 8 })), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i208::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
522 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[246, 243]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [ReadGpr(0, 0, 16), Read(1, Src, 8), Zero(2, 1), Not(3, 2), RaiseUnless(3, 0), Extend(4, 1, 16), Binary(5, Div, 0, 4), Binary(6, Rem, 0, 4), Truncate(7, 5, 8), Truncate(8, 6, 8), Extend(9, 7, 16), Less(10, 9, 5), Not(11, 10), RaiseUnless(11, 0), Extend(12, 7, 16), Extend(13, 8, 16), Const(14, Value { bits: 8, width: 16 }), Shift(15, Left, 13, 14), Binary(16, Or, 15, 12), WriteGpr(0, 16, 16), UndefFlag(CF), UndefFlag(OF), UndefFlag(SF), UndefFlag(ZF), UndefFlag(AF), UndefFlag(PF), Retire], packed: [Token(ReadGpr(0, 0, 16)), Token(Read(1, Src, 8)), Token(Zero(2, 1)), Token(Not(3, 2)), Token(RaiseUnless(3, 0)), Token(Extend(4, 1, 16)), Token(Binary(5, Div, 0, 4)), Token(Binary(6, Rem, 0, 4)), Token(Truncate(7, 5, 8)), Token(Truncate(8, 6, 8)), Token(Extend(9, 7, 16)), Token(Less(10, 9, 5)), Token(Not(11, 10)), Token(RaiseUnless(11, 0)), Token(Extend(12, 7, 16)), Token(Extend(13, 8, 16)), Token(Const(14, Value { bits: 8, width: 16 })), Token(Shift(15, Left, 13, 14)), Token(Binary(16, Or, 15, 12)), Token(WriteGpr(0, 16, 16)), Token(UndefFlag(CF)), Token(UndefFlag(OF)), Token(UndefFlag(SF)), Token(UndefFlag(ZF)), Token(UndefFlag(AF)), Token(UndefFlag(PF)), Token(Retire)], widths: [16, 8, 1, 1, 16, 16, 16, 8, 8, 16, 1, 1, 16, 16, 16, 16, 16], register_effects: 1, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i20a::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
524 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[136, 224]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Src, 8), Write(0), PreserveFlags(2261), Retire], packed: [Token(Read(0, Src, 8)), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i20c::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
526 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[254, 192]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 8), Const(1, Value { bits: 1, width: 8 }), Extend(2, 0, 9), Extend(3, 1, 9), Binary(4, Add, 2, 3), Truncate(5, 4, 8), Bit(6, 4, 8), Binary(7, Xor, 0, 1), Not(8, 7), Binary(9, Xor, 0, 5), Binary(10, And, 8, 9), Bit(11, 10, 7), Binary(12, Xor, 7, 5), Bit(13, 12, 4), Bit(14, 5, 7), Zero(15, 5), Parity(16, 5), Write(5), PreserveFlags(1), Flag(PF, 16), Flag(AF, 13), Flag(ZF, 15), Flag(SF, 14), Flag(OF, 11), Retire], packed: [ReadDstConst { a: 0, b: 1, w: 8, c: Value { bits: 1, width: 8 } }, AddLazy { a: 0, b: 1, r: 5, w: 8, write_cf: false }, Token(Write(5)), Token(Retire)], widths: [8, 8, 9, 9, 9, 8, 1, 8, 8, 8, 8, 1, 8, 1, 1, 1, 1], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i20e::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
528 if image.fetch_signature(ram,cpu.eip,1).ok().is_some_and(|s|s[..1]==[195]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadGpr(0, 4, 16), Extend(1, 0, 32), LoadSegment(2, 1, Word, 2), Extend(3, 2, 32), Const(4, Value { bits: 1, width: 1 }), BranchIf(3, 4), Const(5, Value { bits: 2, width: 16 }), Const(6, Value { bits: 0, width: 16 }), Binary(7, Add, 0, 5), Binary(8, Add, 7, 6), WriteGpr(4, 8, 16), PreserveFlags(2261), Retire], packed: [Token(ReadGpr(0, 4, 16)), Token(Extend(1, 0, 32)), Token(LoadSegment(2, 1, Word, 2)), Token(Extend(3, 2, 32)), Token(Const(4, Value { bits: 1, width: 1 })), Token(BranchIf(3, 4)), Token(Const(5, Value { bits: 2, width: 16 })), Token(Const(6, Value { bits: 0, width: 16 })), Token(Binary(7, Add, 0, 5)), Token(Binary(8, Add, 7, 6)), Token(WriteGpr(4, 8, 16)), Token(Retire)], widths: [16, 32, 16, 32, 1, 16, 16, 16, 16], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: false, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i210::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
529 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[177, 12]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 12, width: 8 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 12, width: 8 })), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i211::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
531 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[211, 232]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Const(2, Value { bits: 31, width: 16 }), Binary(3, And, 1, 2), Const(4, Value { bits: 1, width: 16 }), Const(5, Value { bits: 16, width: 16 }), Const(6, Value { bits: 1, width: 1 }), Const(7, Value { bits: 0, width: 1 }), Zero(8, 3), Not(9, 8), Binary(10, Xor, 3, 4), Zero(11, 10), Binary(12, Sub, 3, 4), Shift(13, Right, 0, 3), Shift(14, Right, 0, 12), Bit(15, 14, 0), Less(16, 3, 5), Bit(17, 13, 15), Zero(18, 13), Parity(19, 13), Bit(20, 0, 15), Write(13), FlagIf(CF, 15, 16, 9), FlagIf(OF, 20, 11, 9), FlagIf(AF, 7, 7, 9), FlagIf(SF, 17, 6, 9), FlagIf(ZF, 18, 6, 9), FlagIf(PF, 19, 6, 9), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, ShiftSetup { b: 1, count: 3, width_slot: 5, yes: 6, no: 7, active: 9, single: 11, last: 12, w: 16 }, Token(Shift(13, Right, 0, 3)), Token(Shift(14, Right, 0, 12)), Token(Bit(15, 14, 0)), Token(Less(16, 3, 5)), Token(Bit(17, 13, 15)), Token(Zero(18, 13)), Token(Parity(19, 13)), Token(Bit(20, 0, 15)), Token(Write(13)), ShiftFlags { cf: 15, cf_defined: 16, of: 20, single: 11, sf: 17, zf: 18, pf: 19, active: 9 }, Token(Retire)], widths: [16, 16, 16, 16, 16, 16, 1, 1, 1, 1, 16, 1, 16, 16, 16, 1, 1, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i213::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
533 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[177, 4]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Const(0, Value { bits: 4, width: 8 }), Write(0), PreserveFlags(2261), Retire], packed: [Token(Const(0, Value { bits: 4, width: 8 })), Token(Write(0)), Token(Retire)], widths: [8], register_effects: 0, operand_width: 8, source_width: 8, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i215::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
535 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[211, 226]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Const(2, Value { bits: 31, width: 16 }), Binary(3, And, 1, 2), Const(4, Value { bits: 1, width: 16 }), Const(5, Value { bits: 16, width: 16 }), Const(6, Value { bits: 1, width: 1 }), Const(7, Value { bits: 0, width: 1 }), Zero(8, 3), Not(9, 8), Binary(10, Xor, 3, 4), Zero(11, 10), Binary(12, Sub, 3, 4), Shift(13, Left, 0, 3), Shift(14, Left, 0, 12), Bit(15, 14, 15), Less(16, 3, 5), Bit(17, 13, 15), Zero(18, 13), Parity(19, 13), Binary(20, Xor, 17, 15), Write(13), FlagIf(CF, 15, 16, 9), FlagIf(OF, 20, 11, 9), FlagIf(AF, 7, 7, 9), FlagIf(SF, 17, 6, 9), FlagIf(ZF, 18, 6, 9), FlagIf(PF, 19, 6, 9), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, ShiftSetup { b: 1, count: 3, width_slot: 5, yes: 6, no: 7, active: 9, single: 11, last: 12, w: 16 }, Token(Shift(13, Left, 0, 3)), Token(Shift(14, Left, 0, 12)), Token(Bit(15, 14, 15)), Token(Less(16, 3, 5)), Token(Bit(17, 13, 15)), Token(Zero(18, 13)), Token(Parity(19, 13)), Token(Binary(20, Xor, 17, 15)), Token(Write(13)), ShiftFlags { cf: 15, cf_defined: 16, of: 20, single: 11, sf: 17, zf: 18, pf: 19, active: 9 }, Token(Retire)], widths: [16, 16, 16, 16, 16, 16, 1, 1, 1, 1, 16, 1, 16, 16, 16, 1, 1, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: true, partial: false, checkpoint_flags: false }") => i217::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
537 if image.fetch_signature(ram,cpu.eip,2).ok().is_some_and(|s|s[..2]==[9, 208]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [Read(0, Dst, 16), Read(1, Src, 16), Binary(2, Or, 0, 1), Const(3, Value { bits: 0, width: 1 }), Bit(4, 2, 15), Zero(5, 2), Parity(6, 2), Write(2), Flag(CF, 3), Flag(OF, 3), UndefFlag(AF), Flag(SF, 4), Flag(ZF, 5), Flag(PF, 6), Retire], packed: [ReadDstSrc { a: 0, b: 1, w: 16 }, Token(Binary(2, Or, 0, 1)), LogicLazy { r: 2, w: 16 }, Token(Write(2)), Token(Retire)], widths: [16, 16, 16, 1, 1, 1, 1], register_effects: 0, operand_width: 16, source_width: 16, uses_flow: false, uses_displacement: false, uses_memory: false, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i219::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),
539 if image.fetch_signature(ram,cpu.eip,1).ok().is_some_and(|s|s[..1]==[195]) && self.matches(cpu.eip,decoded,||format!("{program:?}")=="Program { tokens: [PreserveFlags(0), ReadGpr(0, 4, 16), Extend(1, 0, 32), LoadSegment(2, 1, Word, 2), Extend(3, 2, 32), Const(4, Value { bits: 1, width: 1 }), BranchIf(3, 4), Const(5, Value { bits: 2, width: 16 }), Const(6, Value { bits: 0, width: 16 }), Binary(7, Add, 0, 5), Binary(8, Add, 7, 6), WriteGpr(4, 8, 16), PreserveFlags(2261), Retire], packed: [Token(ReadGpr(0, 4, 16)), Token(Extend(1, 0, 32)), Token(LoadSegment(2, 1, Word, 2)), Token(Extend(3, 2, 32)), Token(Const(4, Value { bits: 1, width: 1 })), Token(BranchIf(3, 4)), Token(Const(5, Value { bits: 2, width: 16 })), Token(Const(6, Value { bits: 0, width: 16 })), Token(Binary(7, Add, 0, 5)), Token(Binary(8, Add, 7, 6)), Token(WriteGpr(4, 8, 16)), Token(Retire)], widths: [16, 32, 16, 32, 1, 16, 16, 16, 16], register_effects: 1, operand_width: 16, source_width: 16, uses_flow: true, uses_displacement: false, uses_memory: true, has_read_address: false, flushes_lazy: false, partial: false, checkpoint_flags: false }") => i21b::execute_with_machine(&mut cpu.regs,&mut cpu.eflags,&mut cpu.undefined_flags,&mut cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,ram,*address,machine),

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
  let at=p.image.segments[1].base+p.cpu.eip;
  let old=p.ram.load(at,crate::MemWidth::Byte).unwrap();p.ram.store(at,crate::MemWidth::Byte,old^1).unwrap();
  assert!(backend.block(&p.active_program,&p.image,&mut p.cpu,&mut p.ram,&mut m,16).is_none());
 }
 #[test]
 fn generated_blocks_preserve_state_across_small_budgets() {
  let mut a=dos::load(ORIGINAL,"TEST.COM").unwrap();let mut b=a.clone();let mut backend=Backend::default();
  for budget in [1,2,3,16,33,127,1000]{
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
