//! Binary trace format shared by interpreter verification and emitted runners.
use super::flow::ExecError;
pub fn fault(eip:u32,error:ExecError)->[u32;5] {
    match error {
        ExecError::GeneralProtection{eip,error_code}=>[4,eip,error_code as u32,0,0],
        ExecError::StackSegment{eip,error_code}=>[5,eip,error_code as u32,0,0],
        ExecError::MemoryAccess{eip,address,bytes,write}=>[6,eip,address,bytes as u32,u32::from(write)],
        ExecError::InvalidBinding=>[8,eip,0,0,0],ExecError::InvalidContext=>[9,eip,0,0,0],
    }
}
#[cfg(test)]
pub fn record(out:&mut Vec<u8>,case:u32,status:[u32;5],regs:&[u32;8],flags:u32,undefined:u32,eip:u32,ram:&[u8],rx:&[u8],tx:&[u8]) {
    record_state(out,case,status,regs,flags,undefined,eip,ram,rx,tx,&[]);
}
pub fn record_machine(out:&mut Vec<u8>,case:u32,status:[u32;5],regs:&[u32;8],flags:u32,undefined:u32,eip:u32,ram:&[u8],rx:&[u8],tx:&[u8],machine:&super::address::SegmentState){
    let mut state=Vec::new();
    for (selector,cache) in machine.selectors.iter().zip(machine.segments){
        state.extend_from_slice(&selector.to_le_bytes());state.extend_from_slice(&cache.base.to_le_bytes());state.extend_from_slice(&cache.limit.to_le_bytes());
        let access=match cache.access{super::address::SegmentAccess::Unusable=>0,super::address::SegmentAccess::Code{readable}=>1|(u8::from(readable)<<1),super::address::SegmentAccess::Data{writable,expand_down,big}=>4|(u8::from(writable)<<3)|(u8::from(expand_down)<<4)|(u8::from(big)<<5)};state.push(access);
    }
    state.extend_from_slice(&[u8::from(machine.real_mode),machine.ss_shadow]);
    record_state(out,case,status,regs,flags,undefined,eip,ram,rx,tx,&state);
}
fn record_state(out:&mut Vec<u8>,case:u32,status:[u32;5],regs:&[u32;8],flags:u32,undefined:u32,eip:u32,ram:&[u8],rx:&[u8],tx:&[u8],state:&[u8]){
    out.extend_from_slice(&case.to_le_bytes());
    for n in status.into_iter().chain(regs.iter().copied()).chain([flags,undefined,eip]) {out.extend_from_slice(&n.to_le_bytes());}
    for bytes in [ram,rx,tx,state] {out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());out.extend_from_slice(bytes);}
}
