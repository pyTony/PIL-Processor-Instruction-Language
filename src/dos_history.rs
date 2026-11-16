//! Bounded generic DOS CPU dispatch history (opt-in).
//! Neither opcode semantics nor guest-program contents are changed.
use super::*;
use std::collections::VecDeque;
pub(crate) const MAX_HISTORY:usize=256;
#[derive(Clone,Debug,Default)]
pub(crate) struct History{capacity:usize,next:u64,entries:VecDeque<Dispatch>}
impl History{
    pub(crate) fn new(capacity:usize)->Self{
        let capacity=capacity.min(MAX_HISTORY);
        Self{capacity,next:0,entries:VecDeque::with_capacity(capacity)}
    }
    pub(crate) fn from_env()->Self{
        let n=std::env::var("PIL_DOS_HISTORY").ok()
            .and_then(|v|v.parse::<usize>().ok()).unwrap_or(0);
        Self::new(n)
    }
    pub(crate) fn enabled(&self)->bool{self.capacity>0}
    pub(crate) fn record(&mut self,mut entry:Dispatch){
        if !self.enabled(){return;}
        entry.sequence=self.next;self.next=self.next.saturating_add(1);
        if self.entries.len()==self.capacity{self.entries.pop_front();}
        self.entries.push_back(entry);
    }
    pub(crate) fn json(&self)->String{
        format!("[{}]",self.entries.iter().map(Dispatch::json).collect::<Vec<_>>().join(","))
    }
    pub(crate) fn capacity(&self)->usize{self.capacity}
    /// Most recent dispatches, chronological and capped at the requested size.
    pub(crate) fn recent_json(&self,count:usize)->String{
        let skip=self.entries.len().saturating_sub(count);
        format!("[{}]",self.entries.iter().skip(skip).map(Dispatch::json).collect::<Vec<_>>().join(","))
    }
    #[cfg(test)]
    pub(crate) fn len(&self)->usize{self.entries.len()}
}
/// Pre-dispatch snapshot. It may be repeated by interrupt/retry, and
/// presence here does not prove the instruction retired.
#[derive(Clone,Debug)]
pub(crate) struct Dispatch{
    sequence:u64,cs:u16,eip:u32,bytes:[u8;8],length:usize,
    registers:[u32;8],selectors:[u16;6],eflags:u32,msw:u16,
}
impl Dispatch{
    pub(crate) fn capture(p:&Loaded)->Self{
        let mut bytes=[0u8;8];let mut length=0;
        for n in (1..=8).rev(){
            if let Ok(raw)=p.image.fetch_signature(&p.ram,p.cpu.eip,n){
                bytes[..n as usize].copy_from_slice(&raw[..n as usize]);
                length=n as usize;break;
            }
        }
        Self{sequence:0,cs:p.selectors[1],eip:p.cpu.eip,
             bytes,length,registers:p.cpu.regs,selectors:p.selectors,
             eflags:p.cpu.eflags,msw:p.image.msw}
    }
    pub(super) fn json(&self)->String{
        let bytes=self.bytes[..self.length].iter()
            .map(|v|format!("{v:02X}")).collect::<Vec<_>>().join(" ");
        format!(r#"{{"sequence":{},"location":"{:04X}:{:04X}","instruction_bytes":{},"registers":{:?},"selectors":{:?},"eflags":{},"msw":{}}}"#,
            self.sequence,self.cs,self.eip,super::stoplog::quote(&bytes),
            self.registers,self.selectors,self.eflags,self.msw)
    }
}
