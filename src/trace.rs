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
pub fn record(out:&mut Vec<u8>,case:u32,status:[u32;5],regs:&[u32;8],flags:u32,undefined:u32,eip:u32,ram:&[u8],rx:&[u8],tx:&[u8]) {
    out.extend_from_slice(&case.to_le_bytes());
    for n in status.into_iter().chain(regs.iter().copied()).chain([flags,undefined,eip]) {out.extend_from_slice(&n.to_le_bytes());}
    for bytes in [ram,rx,tx] {out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());out.extend_from_slice(bytes);}
}
