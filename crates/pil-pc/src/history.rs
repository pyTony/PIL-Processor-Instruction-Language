//! Bounded generic DOS CPU dispatch history (opt-in).
//! Neither opcode semantics nor guest-program contents are changed.
use super::*;
use std::collections::VecDeque;
pub const MAX_HISTORY:usize=256;
#[derive(Clone,Debug,Default)]
pub struct History{capacity:usize,next:u64,entries:VecDeque<Dispatch>}
impl History{
    pub fn new(capacity:usize)->Self{
        let capacity=capacity.min(MAX_HISTORY);
        Self{capacity,next:0,entries:VecDeque::with_capacity(capacity)}
    }
    pub fn from_env()->Self{
        let n=std::env::var("PIL_DOS_HISTORY").ok()
            .and_then(|v|v.parse::<usize>().ok()).unwrap_or(0);
        Self::new(n)
    }
    pub fn enabled(&self)->bool{self.capacity>0}
    pub fn record(&mut self,mut entry:Dispatch){
        if !self.enabled(){return;}
        entry.sequence=self.next;self.next=self.next.saturating_add(1);
        if self.entries.len()==self.capacity{self.entries.pop_front();}
        self.entries.push_back(entry);
    }
    pub fn json(&self)->String{
        format!("[{}]",self.entries.iter().map(Dispatch::json).collect::<Vec<_>>().join(","))
    }
    pub fn capacity(&self)->usize{self.capacity}
    /// Most recent dispatches, chronological and capped at the requested size.
    pub fn recent_json(&self,count:usize)->String{
        let skip=self.entries.len().saturating_sub(count);
        format!("[{}]",self.entries.iter().skip(skip).map(Dispatch::json).collect::<Vec<_>>().join(","))
    }
    #[cfg(any(test,feature="test-support"))]
    pub fn len(&self)->usize{self.entries.len()}
}
/// Pre-dispatch snapshot. It may be repeated by interrupt/retry, and
/// presence here does not prove the instruction retired.
#[derive(Clone,Debug)]
pub struct Dispatch{
    sequence:u64,cs:u16,eip:u32,bytes:[u8;8],length:usize,
    registers:[u32;8],selectors:[u16;6],eflags:u32,msw:u16,
}
impl Dispatch{
    pub fn capture<F>(p:&Machine<F>)->Self{
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
    pub fn json(&self)->String{
        let bytes=self.bytes[..self.length].iter()
            .map(|v|format!("{v:02X}")).collect::<Vec<_>>().join(" ");
        format!(r#"{{"sequence":{},"location":"{:04X}:{:04X}","instruction_bytes":{},"registers":{:?},"selectors":{:?},"eflags":{},"msw":{}}}"#,
            self.sequence,self.cs,self.eip,super::json_quote(&bytes),
            self.registers,self.selectors,self.eflags,self.msw)
    }
}

/// Optional, bounded record of emulated DOS/BIOS service calls.
/// This is independent of CPU dispatch history, so native block execution
/// remains available while service tracing is enabled.
#[derive(Clone,Debug,Default)]
pub struct ServiceTrace{
    next:u64,
    entries:VecDeque<ServiceCall>,
    enabled:bool,
}
#[derive(Clone,Debug)]
struct ServiceCall{
    sequence:u64,vector:u8,cs:u16,ip:u32,
    ax:u16,bx:u16,cx:u16,dx:u16,ds:u16,es:u16,
    result_ax:u16,carry:bool,video_mode:Option<u8>,
}
impl ServiceTrace{
    const CAPACITY:usize=256;
    pub fn enable(&mut self){self.enabled=true;self.entries.clear();self.next=0;}
    pub fn disable(&mut self){self.enabled=false;self.entries.clear();}
    pub fn enabled(&self)->bool{self.enabled}
    pub fn record<F>(&mut self,p:&Machine<F>,vector:u8,site:(u16,u32),before:[u16;6]){
        if !self.enabled{return;}
        let entry=ServiceCall{
            sequence:self.next,vector,cs:site.0,ip:site.1,
            ax:before[0],bx:before[1],cx:before[2],dx:before[3],
            ds:before[4],es:before[5],
            result_ax:p.cpu.regs[0] as u16,carry:p.cpu.eflags&1!=0,
            video_mode:p.ram.load(0x449,MemWidth::Byte).ok().map(|v|v as u8),
        };
        self.next=self.next.saturating_add(1);
        if self.entries.len()==Self::CAPACITY{self.entries.pop_front();}
        self.entries.push_back(entry);
    }
    pub fn json(&self)->String{
        if !self.enabled{return r#"{"enabled":false}"#.to_string();}
        let entries=self.entries.iter().map(|e|format!(
            r#"{{"sequence":{},"vector":"{:02X}","location":"{:04X}:{:04X}","ax":{},"bx":{},"cx":{},"dx":{},"ds":{},"es":{},"result_ax":{},"carry":{},"video_mode":{}}}"#,
            e.sequence,e.vector,e.cs,e.ip,e.ax,e.bx,e.cx,e.dx,e.ds,e.es,e.result_ax,
            e.carry,e.video_mode.map_or_else(||"null".to_string(),|v|v.to_string())
        )).collect::<Vec<_>>().join(",");
        format!(r#"{{"enabled":true,"capacity":{},"total":{},"calls":[{}]}}"#,Self::CAPACITY,self.next,entries)
    }
}
