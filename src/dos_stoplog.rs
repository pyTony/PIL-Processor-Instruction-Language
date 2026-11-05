//! Engine-owned stop diagnostics; never writes to the JSON protocol stream.
use super::*;
use std::io::Write;
pub(super) fn quote(text:&str)->String{session::quote(text)}
pub(super) fn diagnostic(p:&Loaded)->String{
    let mut bytes=Vec::new();
    for n in 1..=15{match bytes_at(p,n){Ok(b)=>bytes.push(b[n as usize-1]),Err(_)=>break}}
    format!("\"cs\":{},\"eip\":{},\"location\":\"{:04X}:{:04X}\",\"instruction_bytes\":{},\"registers\":{:?},\"selectors\":{:?},\"eflags\":{},\"msw\":{},\"control_registers\":[{},{},{}]",
        p.selectors[1],p.cpu.eip,p.selectors[1],p.cpu.eip,quote(&bytes.iter().map(|b|format!("{b:02X}")).collect::<Vec<_>>().join(" ")),p.cpu.regs,p.selectors,p.cpu.eflags,p.image.msw,p.image.segment_state().machine_status(),p.image.controls[0],p.image.controls[1])
}
pub(crate) struct Logger{path:String,program:String,last:Option<String>}
impl Logger{
    pub fn new(path:&str,program:&str)->Self{Self{path:path.into(),program:program.into(),last:None}}
    pub fn record(&mut self,p:&Loaded,status:&str,detail:&str,steps:usize,exceptions:&[ExceptionSite],exception_count:usize){
        if self.path.is_empty(){return;}
        let sites=exceptions.iter().map(|s|format!("{{\"vector\":{},\"detail\":{},{}}}",s.vector,quote(&s.detail),s.diagnostic)).collect::<Vec<_>>().join(",");
        let body=format!("\"status\":{},\"detail\":{},\"steps\":{steps},\"program\":{},{},\"exception_count\":{exception_count},\"exceptions\":[{sites}]",quote(status),quote(detail),quote(&self.program),diagnostic(p));
        if self.last.as_ref()==Some(&body){return;}
        self.last=Some(body.clone());
        let time=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
        let line=format!("{{\"time_unix_ms\":{time},{body}}}\n");
        if let Err(e)=std::fs::OpenOptions::new().create(true).append(true).open(&self.path).and_then(|mut f|f.write_all(line.as_bytes())){eprintln!("Cannot write stop log {}: {e}",self.path);}
    }
}
