//! Local stdin command / NDJSON snapshot adapter for the existing debugger.
//! No CPU implementation lives in the frontend. One bounded command per reply.
use super::*;
use std::fmt::Write as _;
use std::io::Read;
const MAX_LINE:usize=4096;
const MAX_QUANTUM:usize=256;
fn quote(text:&str)->String{
 let mut out=String::from("\"");for c in text.chars(){match c{'"'=>out.push_str("\\\""),'\\'=>out.push_str("\\\\"),'\n'=>out.push_str("\\n"),'\r'=>out.push_str("\\r"),'\t'=>out.push_str("\\t"),c if c<' '=>{write!(out,"\\u{:04x}",c as u32).unwrap();},c=>out.push(c)}}out.push('"');out
}
fn numbers<T:ToString>(values:impl IntoIterator<Item=T>)->String{format!("[{}]",values.into_iter().map(|v|v.to_string()).collect::<Vec<_>>().join(","))}
fn status(stop:&Stop)->&'static str{match stop{Stop::Advanced=>"stepped",Stop::Budget(_)=>"budget",Stop::Breakpoint(_)=>"breakpoint",Stop::WaitingRx(_)=>"waiting_rx",Stop::TxBlocked(_)=>"tx_blocked",Stop::Halt(_)=>"halt",Stop::Fault(_)=>"fault",Stop::Decode{..}=>"decode",Stop::MissingInstruction(_)=>"missing_instruction"}}
fn dump(d:&Debugger,at:u32,size:usize)->String{
 let available=d.ram.bytes().len().saturating_sub(at as usize).min(size);
 if available==0{return "outside RAM".into();}
 d.dump_memory(at,available as u64,super::super::memory::MemWidth::Byte,false).unwrap_or_else(|e|e)
}
fn reply(d:&Debugger,seq:u64,ok:bool,quit:bool,text:&str,kind:&str,bytes:&[u8],view:u32)->String{
 let selectors=d.image.as_ref().map(|i|i.selectors).unwrap_or([0;6]);
 let segments=d.image.as_ref().map(|image|image.segments.iter().enumerate().map(|(n,s)|format!("{}={:04x} base={:08x} limit={:08x}",["ES","CS","SS","DS","FS","GS"][n],image.selectors[n],s.base,s.limit)).collect::<Vec<_>>().join("\n")).unwrap_or_else(||"predecoded monitor demo".into());
 let stack=d.image.as_ref().and_then(|i|{let ss=i.segments[2];let big=matches!(ss.access,super::super::address::SegmentAccess::Data{big:true,..});ss.base.checked_add(if big{d.cpu.regs[4]}else{d.cpu.regs[4]&0xffff})}).unwrap_or(d.cpu.regs[4]);
 let listing=d.listing(d.cpu.eip,16).unwrap_or_else(|e|e);
 format!("{{\"protocol\":1,\"seq\":{seq},\"ok\":{ok},\"quit\":{quit},\"text\":{},\"status\":{},\"guest_output\":{},\"state\":{{\"version\":{},\"demo\":{},\"registers\":{},\"eip\":{},\"flags\":{},\"undefined_flags\":{},\"selectors\":{},\"segments\":{},\"breakpoints\":{},\"listing\":{},\"memory_address\":{view},\"memory\":{},\"stack_address\":{stack},\"stack\":{},\"ram_size\":{},\"cache\":{}}}}}",quote(text),quote(kind),numbers(bytes.iter()),quote(env!("CARGO_PKG_VERSION")),quote(&d.demo),numbers(d.cpu.regs),d.cpu.eip,d.cpu.eflags,d.cpu.undefined_flags,numbers(selectors),quote(&segments),numbers(d.breakpoints.iter()),quote(&listing),quote(&dump(d,view,64)),quote(&dump(d,stack,32)),d.ram.bytes().len(),quote(&d.cache.summary()))
}
fn command(d:&mut Debugger,line:&str,view:&mut u32)->Result<(String,bool,&'static str),String>{
 let words:Vec<_>=line.split_whitespace().collect();
 match words.as_slice(){
  ["snapshot"]|[]=>Ok((String::new(),false,"ready")),
  ["step"]|["s"]=>{let stop=d.step();Ok((Debugger::stop_text(&stop),false,status(&stop)))},
  ["run"]|["continue"]|["c"]=>{let(n,stop)=d.run(MAX_QUANTUM)?;Ok((format!("{n} completed steps; {}",Debugger::stop_text(&stop)),false,status(&stop)))},
  ["run",n]|["continue",n]|["c",n]=>{let n=number(n)?;if n==0||n>MAX_QUANTUM as u64{return Err("service run budget must be1..256".into());}let(n,stop)=d.run(n as usize)?;Ok((format!("{n} completed steps; {}",Debugger::stop_text(&stop)),false,status(&stop)))},
  ["view",at]=>{let at=u32::try_from(number(at)?).map_err(|_|"view address must fit32 bits")?;if at as usize>=d.ram.bytes().len(){return Err("view address outside RAM".into());}*view=at;Ok((format!("memory view at {at:08x}"),false,"ready"))},
  ["help"]=>Ok(("step | run [1..256] | snapshot | demo NAME | reset | regs | flags | list | view ADDRESS | mem/mem8/mem16/mem32 | set REG VALUE | break/delete ADDRESS | breaks | serial rx HEX... | serial tx/status/clear | cache | check | quit".into(),false,"ready")),
  [name,..] if matches!(*name,"demo"|"reset"|"regs"|"flags"|"eip"|"set"|"break"|"delete"|"breaks"|"serial"|"cache"|"check"|"list"|"mem"|"mem8"|"mem16"|"mem32"|"quit"|"exit"|"q")=>{let(text,quit)=d.command_mode(line,false)?;if *view as usize>=d.ram.bytes().len(){*view=0;}Ok((text,quit,"ready"))},
  _=>Err("command is not available in the bounded debugger service; type help".into())
 }
}
pub fn serve<R:BufRead,W:Write>(mut input:R,mut output:W,path:Option<&str>)->io::Result<()>{
 let mut d=Debugger::new("counter").map_err(io::Error::other)?;
 if let Some(path)=path{let bytes=std::fs::read(path)?;d.load_image(&bytes,path).map_err(io::Error::other)?;}
 let mut view=if d.ram.bytes().len()>0x100{0x100}else{0};let mut seq=0u64;
 writeln!(output,"{}",reply(&d,seq,true,false,"PIL debugger connected","ready",&[],view))?;output.flush()?;
 loop{
  let mut bytes=Vec::new();input.by_ref().take((MAX_LINE+1)as u64).read_until(b'\n',&mut bytes)?;
  if bytes.is_empty(){break;}seq+=1;
  if bytes.len()>MAX_LINE{writeln!(output,"{}",reply(&d,seq,false,true,"command exceeds4096-byte limit","error",&[],view))?;output.flush()?;break;}
  let before=d.tx_events.len();let mut quit=false;
  let (ok,text,kind)=match std::str::from_utf8(&bytes){Ok(line)=>match command(&mut d,line.trim(),&mut view){Ok((text,q,kind))=>{quit=q;(true,text,kind)},Err(e)=>(false,e,"error")},Err(_)=>(false,"commands must be UTF-8".into(),"error")};
  let fresh=d.tx_events.get(before..).unwrap_or(&[]);
  writeln!(output,"{}",reply(&d,seq,ok,quit,&text,kind,fresh,view))?;output.flush()?;
  if quit{break;}
 }
 Ok(())
}
#[cfg(test)]
#[path = "../tests/unit/debug_service.rs"]
mod tests;
