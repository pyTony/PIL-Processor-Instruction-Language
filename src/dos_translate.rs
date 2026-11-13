//! Static DOS frontend: loaded image -> reachable validated PIL graph.
//! No Rust source rendering or host execution is performed here. The admitted
//! services/effects describe the current standalone runtime, not arbitrary DOS.
use crate::{address,dos,guest,Program,Token,View};
use crate::dos_blocks::{Site,InstructionProfile};
use std::collections::BTreeMap;

pub(crate) struct Unit {
    pub(crate) loaded:dos::Loaded,
    pub(crate) instructions:BTreeMap<u32,guest::Decoded>,
    pub(crate) sites:BTreeMap<u32,Site>,
}
pub(crate) fn prepare(bytes:&[u8],name:&str)->Result<Unit,String>{
    let loaded=dos::load(bytes,name)?;
    let instructions=discover_direct(&loaded)?;
    let mut sites=BTreeMap::new();
    for (&ip,decoded) in &instructions {
        if let guest::Operation::Pil{program,dst,src,flow,address}=&decoded.operation{
            let signature=loaded.image.fetch_signature(&loaded.ram,ip,decoded.length).map_err(|e|format!("standalone signature: {e:?}"))?;
            sites.insert(ip,Site{signature:signature[..decoded.length as usize].to_vec(),program:program.clone(),dst:*dst,src:*src,next:flow.next_eip,flow:*flow,address:*address,machine:loaded.image.segment_state(),profile:InstructionProfile::of(program,*address)});
        }
    }
    Ok(Unit{loaded,instructions,sites})
}

// Extend the existing register-only whitelist with ordinary addressed RAM effects.
// Only the stack/string forms identified below are admitted; mode changes
// and partial execution remain excluded. Removing memory effects for classification
// does not change the PIL program passed to the shared Rust generator.
pub(crate) fn near_call(program:&Program)->bool{
    program.uses_displacement&&program.tokens.iter().any(|t|matches!(t,Token::StoreSegment(_,_,_,2)))
}
pub(crate) fn near_return(program:&Program)->bool{
    program.uses_flow&&!program.uses_displacement&&program.tokens.iter().any(|t|matches!(t,Token::LoadSegment(_,_,_,2)))
}
fn scalar_lods(program:&Program)->bool{
    !program.partial&&!program.uses_flow&&
        program.tokens.iter().filter(|t|matches!(t,Token::LoadSegment(..))).count()==1&&
        program.tokens.iter().any(|t|matches!(t,Token::WriteGpr(6,..)))&&
        program.tokens.iter().any(|t|matches!(t,Token::WriteGpr(0,..)))
}
fn standalone_ram(program:&Program,address:Option<address::AddressContext>)->bool{
    // CS stays fixed throughout static discovery. Other real-mode segment MOVs
    // do not alter decoding; the shared machine wrapper resolves live caches.
    if program.tokens.iter().any(|t|matches!(t,Token::WriteSegment(1,..))){return false;}
    let call=near_call(program);let ret=near_return(program);let lods=scalar_lods(program);
    if program.uses_memory&&!program.has_read_address&&!call&&!ret&&!lods {
        // The decoder supplies a fixed SS context for register/immediate PUSH
        // and register POP. Strings and segment transfers retain other effects.
        if !address.is_some_and(|a|a.override_segment==Some(address::SegmentRegister::Ss))||
            !program.tokens.iter().any(|t|matches!(t,Token::WriteGpr(4,..))){return false;}
    }
    let mut register_part=program.clone();
    register_part.tokens.retain(|t|!matches!(t,Token::ReadAddress(..)|Token::Load(..)|Token::Store(..)|Token::StoreIf(..)|Token::RaiseUnless(_,0)|Token::ReadSegment(..)|Token::WriteSegment(..)|Token::RequireRealMode));
    if call{register_part.tokens.retain(|t|!matches!(t,Token::StoreSegment(_,_,_,2)));}
    if ret{register_part.tokens.retain(|t|!matches!(t,Token::LoadSegment(_,_,_,2)));}
    if lods{register_part.tokens.retain(|t|!matches!(t,Token::LoadSegment(..)));}
    // This is the register-only projection used for classification. Preserve
    // the original program and its memory metadata for code generation.
    register_part.uses_memory=false;
    crate::dos_blocks::unpaged_register_only(&register_part)
}

// Track independently proven AL/AH and DX constants from PIL writes.
// An unknown overlapping effect invalidates the affected byte or port.
type Known=(Option<u8>,Option<u8>,Option<u16>); // AL, AH, DX
fn next_known(program:&Program,dst:View,src:View,mut known:Known)->Known{
    let mut constants=std::collections::BTreeMap::new();
    let mut sources=std::collections::BTreeMap::new();
    for token in &program.tokens {
        if let Token::Const(id,value)=token{constants.insert(*id,value.bits);}
        if let Token::Read(id,binding,width)=token {
            let view=match binding{crate::Binding::Dst=>dst,crate::Binding::Src=>src};
            sources.insert(*id,(view.register,view.low,*width));
        }
        // XOR of a source with itself yields a literal zero regardless of
        // incoming values. Prove it from PIL operand reads, not the x86 label.
        if let Token::Binary(id,crate::Binary::Xor,a,b)=token {
            if a==b || sources.get(a).is_some()&&sources.get(a)==sources.get(b){
                constants.insert(*id,0);
            }
        }
        let write=match *token {
            Token::Write(id)=>Some((dst,Some(id))),
            Token::WriteSource(id)=>Some((src,Some(id))),
            Token::WriteGpr(reg,id,width)=>Some((View{register:reg,low:0,width},Some(id))),
            Token::WriteIf(..)=>Some((dst,None)),
            Token::WriteSourceIf(..)=>Some((src,None)),
            _=>None,
        };
        if let Some((view,id))=write {
            if view.register==0 {
                if view.low==0&&view.width>=8 {
                    known.0=id.and_then(|id|constants.get(&id)).map(|bits|*bits as u8);
                }
                if view.low<=8&&u16::from(view.low)+u16::from(view.width)>=16 {
                    known.1=id.and_then(|id|constants.get(&id)).map(|bits|(bits>>(8-view.low))as u8);
                }
            }else if view.register==2 && view.low<16{
                known.2=if view.low==0&&view.width>=16 {
                    id.and_then(|id|constants.get(&id)).map(|bits|*bits as u16)
                }else{None};
            }
        }
    }
    known
}

/// IN/OUT through DX that the standalone runtime answers with Dine's VGA
/// sequencer and graphics controller rules, as (input, word).
pub(crate) fn vga_port_access(label:&str)->Option<(bool,bool)>{
    match label{"IN AL,DX"=>Some((true,false)),"IN AX,DX"=>Some((true,true)),"OUT DX,AL"=>Some((false,false)),"OUT DX,AX"=>Some((false,true)),_=>None}
}

/// Discover the complete reachable instruction graph for a deliberately small
/// ordinary-RAM DOS subset. No byte is classified as code unless reached by
/// a direct edge from the entry point. Unsupported or overlapping code fails
/// closed instead of claiming complete standalone coverage.
fn discover_direct(p:&dos::Loaded)->Result<std::collections::BTreeMap<u32,guest::Decoded>,String>{
    use std::collections::{BTreeMap,BTreeSet};
    // Summaries are keyed by entry, proven AL/AH/DX and return width.
    // Recursive callers wait for a base return instead of growing call strings.
    // A newly discovered return value wakes every caller until a fixed point.
    type Context=(u32,Known,u8);
    let root:Context=(p.cpu.eip,(None,None,None),0);
    let mut waiting:BTreeMap<Context,BTreeSet<(u32,Context)>>=BTreeMap::new();
    let mut returns:BTreeMap<Context,BTreeSet<Known>>=BTreeMap::new();
    let mut pending=vec![(p.cpu.eip,(None,None,None),root)];let mut visited=BTreeSet::new();let mut found=BTreeMap::new();let mut occupied=BTreeMap::new();let mut exit=false;
    while let Some((ip,known,context))=pending.pop(){
        if !visited.insert((ip,known,context)){continue;}
        if visited.len()>65_536{return Err("standalone service analysis exceeds 65536 states".into());}
        if found.len()>=4096&&!found.contains_key(&ip){return Err("standalone control-flow graph exceeds 4096 instructions".into());}
        let physical=u64::from(p.image.segments[1].base)+u64::from(ip);
        let module_end=u64::from(p.module_base)+p.module_bytes as u64;
        if physical<u64::from(p.module_base)||physical>=module_end{return Err(format!("standalone control-flow target {ip:04x} is outside the loaded module"));}
        let decoded=p.image.decode(&p.ram,ip).map_err(|e|format!("standalone decode at {ip:04x}: {e:?}"))?;
        let end=ip.checked_add(decoded.length).ok_or("standalone instruction address overflow")?;
        if physical+u64::from(decoded.length)>module_end{return Err(format!("standalone instruction at {ip:04x} crosses loaded module end"));}
        for byte in ip..end {if let Some(old)=occupied.insert(byte,ip){if old!=ip{return Err(format!("overlapping standalone instructions at {old:04x} and {ip:04x}"));}}}
        // The decoder's generic I/O PIL is an open bus. The VGA sequencer and
        // graphics controller ports, reached through a DX proven on every
        // incoming path, use the runtime's device model instead; every other
        // port operation fails closed, including a dynamic or clobbered DX.
        let vga=vga_port_access(&decoded.label);
        if decoded.label.starts_with("IN ")||decoded.label.starts_with("OUT "){
            // A word access must start at an index port, as in Dine.
            let index=matches!(known.2,Some(0x3c4|0x3ce));
            if vga.is_none()||!(index||!vga.unwrap().1&&matches!(known.2,Some(0x3c5|0x3cf)))||!p.image.real_mode{
                return Err(format!("standalone port I/O needs IN/OUT through a proven DX of 03C4h or 03CEh (bytes also 03C5h or 03CFh) at {ip:04x}: {}",decoded.label));
            }
        }
        match &decoded.operation {
            guest::Operation::Pil{program,flow,dst,src,address}
                if vga.is_some() || standalone_ram(program,*address)=>{
                // A validated VGA port access is a native device boundary
                // with one successor. OUT keeps AL/AH/DX; IN replaces AL (and
                // AH for a word) with a value unknown at build time.
                if let Some((input,word))=vga{
                    let next=match(input,word){(false,_)=>known,(true,false)=>(None,known.1,known.2),(true,true)=>(None,None,known.2)};
                    pending.push((flow.next_eip,next,context));
                }else{
                let next=next_known(program,*dst,*src,known);
                if near_return(program){
                    if context.2==0{return Err("standalone RET has no proven caller".into());}
                    if context.2!=program.operand_width{return Err("standalone CALL/RET operand widths must match".into());}
                    if returns.entry(context).or_default().insert(next){
                        if let Some(callers)=waiting.get(&context){for &(target,parent) in callers{pending.push((target,next,parent));}}
                    }
                }else if program.uses_flow {
                    if !program.uses_displacement||!program.tokens.iter().any(|t|matches!(t,Token::BranchIf(..))){return Err(format!("unsupported standalone control flow at {ip:04x}"));}
                    let target=flow.next_eip.wrapping_add_signed(flow.displacement)&crate::mask(program.operand_width)as u32;
                    if near_call(program){
                        let callee=(target,next,program.operand_width);
                        let continuation=flow.next_eip&crate::mask(program.operand_width)as u32;
                        waiting.entry(callee).or_default().insert((continuation,context));
                        if let Some(values)=returns.get(&callee){for &value in values{pending.push((continuation,value,context));}}
                        pending.push((target,next,callee));
                    }else{
                    pending.push((target,next,context));
                    let unconditional=program.tokens.iter().any(|t|if let Token::BranchIf(_,condition)=t {
                        program.tokens.iter().any(|t|matches!(t,Token::Const(id,value) if id==condition&&value.bits==1))
                    }else{false});
                    if !unconditional{pending.push((flow.next_eip,next,context));}
                    }
                }else{pending.push((flow.next_eip,next,context));}
                }
            },
            guest::Operation::Interrupt{vector:0x20,overflow:false,..}=>exit=true,
            guest::Operation::Interrupt{vector:0x21,overflow:false,next}=>match known.1{
                Some(0x02)=>pending.push((*next,(None,known.1,known.2),context)),
                Some(0x09)=>pending.push((*next,(Some(0x24),known.1,known.2),context)),
                Some(0x4c)=>exit=true,
                _=>return Err(format!("standalone INT 21h at {ip:04x} requires proven AH=02h,09h or 4Ch on every incoming path")),
            },
            // No arbitrary BIOS dispatch: only the statically known mode-set
            // and cursor services, with runtime argument validation below.
            guest::Operation::Interrupt{vector:0x10,overflow:false,next}=>match known{
                (Some(0x03|0x12),Some(0),_)|(_,Some(2),_)=>pending.push((*next,known,context)),
                _=>return Err(format!("standalone INT 10h at {ip:04x} requires proven AX=0003h/0012h or AH=02h")),
            },
            // Key wait returns a supplied scan/ASCII AX; its value is unknown
            // to compile-time service analysis. No implicit default key.
            guest::Operation::Interrupt{vector:0x16,overflow:false,next}=>match known.1 {
                Some(0)=>pending.push((*next,(None,None,known.2),context)),
                _=>return Err(format!("standalone INT 16h at {ip:04x} requires proven AH=00h")),
            },
            _=>return Err(format!("unsupported standalone operation at {ip:04x}")),
        }
        found.insert(ip,decoded);
    }
    if !exit{return Err("standalone graph has no supported termination".into());}
    // Duplicate targets are harmless. Reached data and fall-through beyond the
    // file were rejected above; unreachable trailing bytes stay data.
    Ok(found)
}
