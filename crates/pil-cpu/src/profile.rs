//! Opt-in host timings. Phase times are inclusive and must not be added together.
use std::{cell::RefCell,collections::BTreeMap,time::Instant};
#[derive(Default)]
struct Data { phases:BTreeMap<&'static str,(u64,u128)>, sites:BTreeMap<(u16,u32),u64>, overflow:u64 }
thread_local! { static DATA:RefCell<Option<Data>>=const { RefCell::new(None) }; }
pub struct Report { file:std::fs::File, started:Instant }
pub fn start(path:&str)->Result<Report,String>{
    let file=std::fs::File::create(path).map_err(|e|format!("profile {path}: {e}"))?;
    DATA.with(|d|*d.borrow_mut()=Some(Data::default()));
    Ok(Report{file,started:Instant::now()})
}
pub struct Span { name:&'static str, started:Option<Instant> }
pub fn span(name:&'static str)->Span { Span{name,started:DATA.with(|d|d.borrow().as_ref().map(|_|Instant::now()))} }
impl Drop for Span { fn drop(&mut self){if let Some(t)=self.started{let ns=t.elapsed().as_nanos();DATA.with(|d|{if let Some(d)=d.borrow_mut().as_mut(){let v=d.phases.entry(self.name).or_default();v.0+=1;v.1+=ns;}});}} }
pub fn site(cs:u16,ip:u32){DATA.with(|d|{if let Some(d)=d.borrow_mut().as_mut(){if let Some(n)=d.sites.get_mut(&(cs,ip)){*n+=1;}else if d.sites.len()<4096{d.sites.insert((cs,ip),1);}else{d.overflow+=1;}}});}
impl Drop for Report {fn drop(&mut self){
    use std::io::Write;
    let data=DATA.with(|d|d.borrow_mut().take()).unwrap_or_default();
    let mut out=format!("{{\"format\":1,\"elapsed_ns\":{},\"inclusive_phases\":[",self.started.elapsed().as_nanos());
    for (i,(name,(calls,ns))) in data.phases.iter().enumerate(){if i>0{out.push(',');}out.push_str(&format!("{{\"name\":\"{name}\",\"calls\":{calls},\"ns\":{ns}}}"));}
    out.push_str("],\"sites\":[");let mut sites:Vec<_>=data.sites.into_iter().collect();sites.sort_by_key(|(_,n)|std::cmp::Reverse(*n));
    for(i,((cs,ip),attempts))in sites.iter().enumerate(){if i>0{out.push(',');}out.push_str(&format!("{{\"cs\":{cs},\"ip\":{ip},\"attempts\":{attempts}}}"));}
    out.push_str(&format!("],\"untracked_site_attempts\":{}}}\n",data.overflow));
    if let Err(e)=self.file.write_all(out.as_bytes()){eprintln!("profile report write failed: {e}");}
}}
