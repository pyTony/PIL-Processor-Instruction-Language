//! Development-only SingleStepTests/80386 adapter for the real-mode MOO files.
//! The suite data stays outside the repository: tools/verify_sst80386.py fetches
//! a pinned revision into target/external-tests and points SST80386_DIR at it.
//! Each case runs one instruction through the PIL interpreter and, separately,
//! through generated Rust; both are compared with the captured 386EX state.
use super::*;
use std::{collections::{BTreeMap,HashMap,HashSet},fs,io::Write,path::{Path,PathBuf},process::Command,time::Instant};

// MOO RG32 order: cr0 cr3 eax ebx ecx edx esi edi ebp esp cs ds es fs gs ss eip eflags dr6 dr7.
const NAMES:[&str;20]=["cr0","cr3","eax","ebx","ecx","edx","esi","edi","ebp","esp","cs","ds","es","fs","gs","ss","eip","eflags","dr6","dr7"];
// PIL register index (EAX ECX EDX EBX ESP EBP ESI EDI) for MOO slots 2..=9.
const GPR:[usize;8]=[0,3,1,2,6,7,5,4];
// PIL segment index (ES CS SS DS FS GS) for MOO slots 10..=15.
const SEG:[usize;6]=[1,3,0,4,5,2];
const RAM_SIZE:usize=16*1024*1024;
const REP_LIMIT:usize=1024;

#[derive(Clone,Default)]
struct Case{index:u32,name:String,bytes:Vec<u8>,init:[u32;20],ram:Vec<(u32,u8)>,fin:[Option<u32>;20],mask:[u32;20],fin_ram:Vec<(u32,u8)>,exception:Option<u8>,hash:String}

fn u32_at(b:&[u8],at:usize)->Result<u32,String>{b.get(at..at+4).map(|s|u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(||format!("truncated MOO at {at}"))}
fn chunks(b:&[u8])->Result<Vec<([u8;4],&[u8])>,String>{
    let mut out=Vec::new();let mut at=0;
    while at<b.len(){
        let id:[u8;4]=b.get(at..at+4).ok_or("truncated chunk id")?.try_into().unwrap();let n=u32_at(b,at+4)? as usize;
        let body=b.get(at+8..at+8+n).ok_or_else(||format!("chunk {} overruns its parent",String::from_utf8_lossy(&id)))?;
        out.push((id,body));at+=8+n;
    }Ok(out)
}
fn reg_list(b:&[u8])->Result<[Option<u32>;20],String>{
    let bits=u32_at(b,0)?;let mut out=[None;20];let mut at=4;
    for (i,slot) in out.iter_mut().enumerate(){if bits>>i&1!=0{*slot=Some(u32_at(b,at)?);at+=4;}}Ok(out)
}
fn ram_list(b:&[u8])->Result<Vec<(u32,u8)>,String>{
    let n=u32_at(b,0)? as usize;(0..n).map(|i|Ok((u32_at(b,4+5*i)?,*b.get(8+5*i).ok_or("truncated RAM entry")?))).collect()
}
fn apply_mask(mask:&mut [u32;20],list:[Option<u32>;20]){for (m,v) in mask.iter_mut().zip(list){if let Some(v)=v{*m&=v;}}}
fn gunzip(gz:&[u8])->Result<Vec<u8>,String>{
    if gz.len()<18||gz[0]!=0x1f||gz[1]!=0x8b||gz[2]!=8{return Err("not a gzip/DEFLATE file".into());}
    let flags=gz[3];let mut at=10;
    if flags&4!=0{at+=2+u16::from_le_bytes([gz[at],gz[at+1]]) as usize;}
    for bit in [8,16]{if flags&bit!=0{while *gz.get(at).ok_or("truncated gzip header")?!=0{at+=1;}at+=1;}}
    if flags&2!=0{at+=2;}
    let size=u32_at(gz,gz.len()-4)? as usize;let crc=u32_at(gz,gz.len()-8)?;
    let out=super::inflate::decode(gz.get(at..gz.len()-8).ok_or("truncated gzip body")?,size)?;
    if out.len()!=size||super::containers::crc32(&out)!=crc{return Err("gzip size/CRC mismatch".into());}Ok(out)
}
fn parse(path:&Path)->Result<Vec<Case>,String>{
    let data=gunzip(&fs::read(path).map_err(|e|format!("{}: {e}",path.display()))?)?;
    let mut file_mask=[u32::MAX;20];let mut cases=Vec::new();
    for (id,body) in chunks(&data)?{
        match &id{
            b"MOO "=>{if body.get(8..12)!=Some(b"386E"){return Err(format!("{}: not a 386EX MOO file",path.display()));}}
            b"RM32"=>apply_mask(&mut file_mask,reg_list(body)?),
            b"TEST"=>{
                let mut c=Case{index:u32_at(body,0)?,mask:file_mask,..Case::default()};let mut init=None;
                for (sid,sb) in chunks(&body[4..])?{
                    match &sid{
                        b"NAME"=>c.name=String::from_utf8_lossy(sb.get(4..).unwrap_or(&[])).into_owned(),
                        b"BYTS"=>c.bytes=sb.get(4..).unwrap_or(&[]).to_vec(),
                        b"EXCP"=>c.exception=sb.first().copied(),
                        b"HASH"=>c.hash=sb.iter().map(|b|format!("{b:02x}")).collect(),
                        b"INIT"|b"FINA"=>for (xid,xb) in chunks(sb)?{match (&sid,&xid){
                            (b"INIT",b"RG32")=>init=Some(reg_list(xb)?),
                            (b"INIT",b"RAM ")=>c.ram=ram_list(xb)?,
                            (b"FINA",b"RG32")=>c.fin=reg_list(xb)?,
                            (b"FINA",b"RAM ")=>c.fin_ram=ram_list(xb)?,
                            (b"FINA",b"RM32")=>apply_mask(&mut c.mask,reg_list(xb)?),
                            _=>{},
                        }},
                        _=>{},
                    }
                }
                let init=init.ok_or_else(||format!("{} test {}: no INIT RG32",path.display(),c.index))?;
                if init.iter().any(Option::is_none){return Err(format!("{} test {}: incomplete INIT RG32",path.display(),c.index));}
                c.init=init.map(Option::unwrap);cases.push(c);
            }
            _=>{},
        }
    }Ok(cases)
}

/// State after one instruction; the same record is produced by both backends.
#[derive(Clone,Debug,PartialEq,Eq)]
struct Actual{fault:Option<String>,regs:[u32;8],eflags:u32,undefined:u32,eip:u32,selectors:[u16;6],msw:u16,real_mode:bool,ram:Vec<u8>}
struct Prepared{image:guest::Image,cpu:Cpu,decoded:Result<guest::Decoded,guest::GuestError>,readback:Vec<u32>}
fn prepare(c:&Case,ram:&mut Ram)->Prepared{
    let mut selectors=[0u16;6];for (i,s) in SEG.iter().enumerate(){selectors[*s]=c.init[10+i] as u16;}
    let image=guest::Image{fetch_base:0,start:0,end:RAM_SIZE as u32,system:(0,0),segments:selectors.map(address::SegmentCache::real_mode),selectors,msw:c.init[0] as u16,real_mode:true,ss_shadow:0,sti_shadow:0,flag_checkpoint:None};
    let mut regs=[0;8];for (i,r) in GPR.iter().enumerate(){regs[*r]=c.init[2+i];}
    let cpu=Cpu{regs,eflags:c.init[17],undefined_flags:0,eip:c.init[16]};
    for (a,v) in &c.ram{ram.write_bytes(*a,&[*v]).unwrap();}
    let mut readback:Vec<u32>=c.ram.iter().chain(&c.fin_ram).map(|e|e.0).collect();readback.sort();readback.dedup();
    let decoded=image.decode(ram,cpu.eip);
    Prepared{image,cpu,decoded,readback}
}
fn clear(c:&Case,ram:&mut Ram){for (a,_) in c.ram.iter().chain(&c.fin_ram){ram.write_bytes(*a,&[0]).unwrap();}}
fn interpret(p:&mut Prepared,ram:&mut Ram)->Option<Actual>{
    let Ok(d)=&p.decoded else{return None};
    let guest::Operation::Pil{program,dst,src,flow,address}=&d.operation else{return None};
    let mut fault=None;
    for _ in 0..REP_LIMIT{
        let old=p.cpu.eip;let mut machine=p.image.segment_state();
        let r=execute_machine(program,&mut p.cpu,*dst,*src,*flow,ram,*address,&mut machine);p.image.set_segment_state(machine);
        if let Err(e)=r{fault=Some(fault_kind(&format!("{e:?}")));break;}
        if !(program.partial&&p.cpu.eip==old){break;}
    }
    Some(Actual{fault,regs:p.cpu.regs,eflags:p.cpu.eflags,undefined:p.cpu.undefined_flags,eip:p.cpu.eip,selectors:p.image.selectors,msw:p.image.msw,real_mode:p.image.real_mode,ram:p.readback.iter().map(|a|ram.bytes()[*a as usize]).collect()})
}

#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
enum Outcome{Pass,Revoked,UndefinedSib,HltFetchFault,NotDecoded(String),MonitorOperation,ExceptionNotDelivered(u8),UndefinedFlagsOnly,Fail(String)}
fn fault_kind(f:&str)->String{f.split(|c:char|c==' '||c=='{'||c=='(').next().unwrap_or(f).to_string()}
fn repeated_prefixes(bytes:&[u8])->bool{
    let mut seen=HashSet::new();let mut segments=0;
    for b in bytes{match b{0x26|0x2e|0x36|0x3e|0x64|0x65=>{segments+=1;if !seen.insert(*b){return true;}},0xf0|0xf2|0xf3|0x66|0x67=>{if !seen.insert(*b){return true;}},_=>break}}
    segments>1
}
// SIB index 100b with a nonzero scale: the 386 scales the base register, the
// Intel tables call the form "none". The suite README says to filter these
// unless that behaviour is emulated; PIL does not, so they are counted apart.
fn undefined_sib(bytes:&[u8],address:&Option<address::AddressContext>)->bool{
    if !matches!(address,Some(address::AddressContext{effective:address::EffectiveAddress::Bits32{index:None,..},..})){return false;}
    let mut at=bytes.iter().position(|b|!matches!(b,0x26|0x2e|0x36|0x3e|0x64|0x65|0xf0|0xf2|0xf3|0x66|0x67)).unwrap_or(bytes.len());
    if matches!(bytes.get(at),Some(0xa0..=0xa3)){return false;}
    if bytes.get(at)==Some(&0x0f){at+=1;}at+=1;
    let (Some(m),Some(sib))=(bytes.get(at),bytes.get(at+1)) else{return false};
    m>>6!=3&&m&7==4&&(sib>>3)&7==4&&sib>>6!=0
}
fn decode_family(c:&Case,detail:&str)->String{
    if repeated_prefixes(&c.bytes){return "repeated or multiple segment prefixes".into();}
    if let Some(at)=detail.find(" at "){return detail[..at].to_string();}
    if detail.starts_with("fetch outside"){return "fetch outside image".into();}
    if detail.starts_with("ENTER nesting level"){return "ENTER nesting level above 0".into();}
    detail.to_string()
}
fn judge(c:&Case,actual:&Actual,readback:&[u32])->Outcome{
    if let Some(n)=c.exception{
        // The instruction completed, but the injected HLT lies past the CS
        // limit, so hardware raised #GP fetching it. Not a PIL result.
        if n==13&&actual.fault.is_none()&&actual.eip>0xffff{return Outcome::HltFetchFault;}
        return if actual.fault.is_some(){Outcome::ExceptionNotDelivered(n)}else{Outcome::Fail(format!("missed exception #{n}"))};
    }
    if let Some(f)=&actual.fault{return Outcome::Fail(format!("unexpected fault {f}"));}
    let mut got=c.init;
    for (i,r) in GPR.iter().enumerate(){got[2+i]=actual.regs[*r];}
    for (i,s) in SEG.iter().enumerate(){got[10+i]=(got[10+i]&!0xffff)|u32::from(actual.selectors[*s]);}
    got[0]=(c.init[0]&!0xffff)|u32::from(actual.msw&!1)|u32::from(!actual.real_mode);
    // The suite ends each case after the following HLT; its final EIP is past it.
    got[16]=actual.eip.wrapping_add(1);got[17]=actual.eflags;
    let mut flags_only=None;
    for i in 0..20{
        let want=c.fin[i].unwrap_or(c.init[i]);let mut mask=c.mask[i];if (10..16).contains(&i){mask&=0xffff;}
        let diff=(want^got[i])&mask;
        if diff!=0{
            if i==17{flags_only=Some(diff);continue;}
            return Outcome::Fail(format!("register {}",NAMES[i]));
        }
    }
    let index:HashMap<u32,usize>=readback.iter().enumerate().map(|(i,a)|(*a,i)).collect();
    let mut want:HashMap<u32,u8>=c.ram.iter().copied().collect();want.extend(c.fin_ram.iter().copied());
    for (a,v) in &want{if actual.ram[index[a]]!=*v{return Outcome::Fail(if c.fin_ram.iter().any(|e|e.0==*a){"memory write".into()}else{"memory changed unexpectedly".into()});}}
    match flags_only{
        None=>Outcome::Pass,
        Some(diff) if diff&!actual.undefined==0=>Outcome::UndefinedFlagsOnly,
        Some(diff)=>Outcome::Fail(format!("eflags {}",flag_names(diff&!actual.undefined))),
    }
}
fn flag_names(bits:u32)->String{
    let names=[(0,"CF"),(2,"PF"),(4,"AF"),(6,"ZF"),(7,"SF"),(8,"TF"),(9,"IF"),(10,"DF"),(11,"OF"),(14,"NT"),(16,"RF"),(17,"VM")];
    let mut v:Vec<String>=names.iter().filter(|(b,_)|bits>>b&1!=0).map(|(_,n)|n.to_string()).collect();
    let rest=bits&!names.iter().fold(0,|m,(b,_)|m|1<<b);if rest!=0{v.push(format!("{rest:#x}"));}v.join("+")
}

// Generated-backend input/output records; little-endian u32 words.
fn encode(out:&mut Vec<u8>,variant:u32,p:&Prepared,c:&Case){
    let guest::Operation::Pil{dst,src,flow,address,program,..}=&p.decoded.as_ref().unwrap().operation else{unreachable!()};
    let mut w=vec![variant,dst.register as u32,u32::from(dst.low),src.register as u32,u32::from(src.low),flow.next_eip,flow.displacement as u32,u32::from(flow.displacement_bits),flow.code_limit];
    match address{
        None=>w.extend([0,0,0,0,0,6]),
        Some(a)=>{
            match a.effective{
                address::EffectiveAddress::Bits16{form,displacement}=>w.extend([1,form as u32,u32::from(displacement),0,0]),
                address::EffectiveAddress::Bits32{base,index,scale,displacement}=>w.extend([2,base.map_or(8,|g|g as u32),index.map_or(8,|g|g as u32),u32::from(scale),displacement]),
            }
            w.push(a.override_segment.map_or(6,|s|s.index() as u32));
        }
    }
    w.extend(p.image.selectors.map(u32::from));w.push(u32::from(p.image.msw));
    w.extend(p.cpu.regs);w.extend([p.cpu.eflags,p.cpu.eip,u32::from(program.partial),c.ram.len() as u32,p.readback.len() as u32]);
    for (a,v) in &c.ram{w.extend([*a,u32::from(*v)]);}
    w.extend(&p.readback);
    for n in w{out.extend_from_slice(&n.to_le_bytes());}
}
fn decode_actual(words:&mut impl Iterator<Item=u32>,n:usize)->Option<Actual>{
    let mut next=||words.next();
    let status=next()?;let detail=next()?;let mut regs=[0;8];for r in &mut regs{*r=next()?;}
    let (eflags,undefined,eip)=(next()?,next()?,next()?);let mut selectors=[0u16;6];for s in &mut selectors{*s=next()? as u16;}
    let msw=next()? as u16;let real_mode=next()?!=0;let ram=(0..n).map(|_|next().map(|v|v as u8)).collect::<Option<Vec<_>>>()?;
    Some(Actual{fault:(status!=0).then(||FAULTS.get(detail as usize).copied().unwrap_or("Unknown").to_string()),regs,eflags,undefined,eip,selectors,msw,real_mode,ram})
}
// Only the ExecError variant crosses the generated boundary; judge() compares kinds.
const FAULTS:[&str;5]=["MemoryAccess","StackSegment","InvalidBinding","InvalidContext","GeneralProtection"];
const RUNNER:&str=r#"
fn main(){
 let mut args=std::env::args().skip(1);let input=std::fs::read(args.next().unwrap()).unwrap();let output=args.next().unwrap();
 let words:Vec<u32>=input.chunks_exact(4).map(|c|u32::from_le_bytes(c.try_into().unwrap())).collect();let mut at=0;
 let mut out:Vec<u32>=Vec::new();let mut ram=Ram::new(16*1024*1024);
 let gpr=[Eax,Ecx,Edx,Ebx,Esp,Ebp,Esi,Edi];let segs=[Es,Cs,Ss,Ds,Fs,Gs];let forms=[BxSi,BxDi,BpSi,BpDi,Si,Di,Bp,Bx,Direct];
 while at<words.len(){
  let mut next=||{let v=words[at];at+=1;v};
  let variant=next();let dst=(next() as usize,next() as u8);let src=(next() as usize,next() as u8);
  let flow=FlowContext{next_eip:next(),displacement:next() as i32,displacement_bits:next() as u8,code_limit:next()};
  let (kind,a,b,c,d,o)=(next(),next(),next(),next(),next(),next());
  let mut selectors=[0u16;6];for s in &mut selectors{*s=next() as u16;}let msw=next() as u16;
  let mut machine=SegmentState{segments:selectors.map(SegmentCache::real_mode),selectors,msw,real_mode:true,ss_shadow:0,sti_shadow:0,flag_checkpoint:None};
  let effective=match kind{1=>Some(Bits16{form:forms[a as usize],displacement:b as u16}),2=>Some(Bits32{base:gpr.get(a as usize).copied(),index:gpr.get(b as usize).copied(),scale:c as u8,displacement:d}),_=>None};
  let address=effective.map(|effective|AddressContext{effective,override_segment:segs.get(o as usize).copied(),segments:machine.segments});
  let mut regs=[0u32;8];for r in &mut regs{*r=next();}let mut flags=next();let mut eip=next();let partial=next()!=0;let mut undefined=0u32;
  let (n_ram,n_back)=(next() as usize,next() as usize);
  let mut written=Vec::new();for _ in 0..n_ram{let a=next();let v=next() as u8;ram.write_bytes(a,&[v]).unwrap();written.push(a);}
  let back:Vec<u32>=(0..n_back).map(|_|next()).collect();
  let (mut status,mut detail)=(0,0);
  for _ in 0..1024{let old=eip;
   if let Err(e)=dispatch(variant,&mut regs,&mut flags,&mut undefined,&mut eip,dst,src,flow,&mut ram,address,&mut machine){status=1;detail=match e{ExecError::MemoryAccess{..}=>0,ExecError::StackSegment{..}=>1,ExecError::InvalidBinding=>2,ExecError::InvalidContext=>3,ExecError::GeneralProtection{..}=>4};break;}
   if !(partial&&eip==old){break;}
  }
  out.extend([status,detail]);out.extend(regs);out.extend([flags,undefined,eip]);out.extend(machine.selectors.map(u32::from));out.extend([u32::from(machine.msw),u32::from(machine.real_mode)]);
  for a in &back{out.push(ram.bytes()[*a as usize] as u32);}
  for a in written.iter().chain(&back){ram.write_bytes(*a,&[0]).unwrap();}
 }
 let bytes:Vec<u8>=out.iter().flat_map(|w|w.to_le_bytes()).collect();std::fs::write(output,bytes).unwrap();
}
"#;
fn runner_source(variants:&[String])->String{
    let mut s=String::from("// Generated from validated PIL for the SingleStepTests/80386 adapter.\n#![allow(dead_code,unused_variables,unused_mut,unused_imports,unused_parens)]\n");
    s.push_str("mod flow {\n");s.push_str(include_str!("../../src/flow.rs"));s.push_str("\n}\nuse flow::{ExecError,FlowContext};\n");
    s.push_str("mod memory {\n");s.push_str(include_str!("../../src/memory.rs").split("#[cfg(test)]").next().unwrap());s.push_str("\n}\nuse memory::{Ram,MemWidth};\n");
    s.push_str("mod address {\n");s.push_str(include_str!("../../src/address.rs").split("#[cfg(test)]").next().unwrap());s.push_str("\n}\nuse address::*;\nuse address::{EffectiveAddress::*,Address16::*,Gpr::*,SegmentRegister::*};\n");
    for (i,v) in variants.iter().enumerate(){s.push_str(&format!("mod p{i} {{use super::{{FlowContext,ExecError,Ram,MemWidth,address}}; {v} }}\n"));}
    s.push_str("fn dispatch(v:u32,regs:&mut [u32;8],flags:&mut u32,undefined:&mut u32,eip:&mut u32,dst:(usize,u8),src:(usize,u8),flow:FlowContext,ram:&mut Ram,address:Option<AddressContext>,machine:&mut SegmentState)->Result<(),ExecError>{match v{\n");
    for i in 0..variants.len(){s.push_str(&format!("{i}=>p{i}::execute_with_machine(regs,flags,undefined,eip,dst,src,flow,ram,address,machine),\n"));}
    s.push_str("_=>panic!(\"bad variant\")}}\n");s.push_str(RUNNER);s
}

#[derive(Default)]
struct Tally{counts:BTreeMap<Outcome,usize>}
impl Tally{fn add(&mut self,o:&Outcome){*self.counts.entry(o.clone()).or_insert(0)+=1;}
    fn get(&self,f:impl Fn(&Outcome)->bool)->usize{self.counts.iter().filter(|(o,_)|f(o)).map(|(_,n)|n).sum()}
}
fn file_list(dir:&Path)->Result<Vec<PathBuf>,String>{
    let mut files:Vec<_>=fs::read_dir(dir).map_err(|e|format!("{}: {e}",dir.display()))?.filter_map(|e|e.ok().map(|e|e.path())).filter(|p|p.to_string_lossy().ends_with(".MOO.gz")).collect();
    files.sort();
    if let Ok(only)=std::env::var("SST80386_FILES"){let keep:HashSet<String>=only.split(',').map(|s|s.trim().to_uppercase()).collect();files.retain(|p|keep.contains(&stem(p)));}
    Ok(files)
}
fn stem(p:&Path)->String{p.file_name().unwrap().to_string_lossy().trim_end_matches(".MOO.gz").to_uppercase()}

fn run(root:&Path,out_dir:&Path)->Result<String,String>{
    let dir=root.join("v1_ex_real_mode");let files=file_list(&dir)?;
    let revoked:HashSet<String>=fs::read_to_string(root.join("revocation_list.txt")).unwrap_or_default().lines().map(|l|l.trim().to_lowercase()).filter(|l|!l.is_empty()).collect();
    let generated=std::env::var("SST80386_GENERATED").map(|v|v!="0").unwrap_or(true);
    let sample:usize=std::env::var("SST80386_GENERATED_SAMPLE").ok().and_then(|v|v.parse().ok()).unwrap_or(8);
    let gen_dir=out_dir.join("generated");fs::create_dir_all(&gen_dir).map_err(|e|e.to_string())?;
    let mut ram=Ram::new(RAM_SIZE);let mut variants:Vec<String>=Vec::new();let mut variant_ids:HashMap<String,u32>=HashMap::new();
    let mut rows:Vec<(String,String,Tally,Tally)>=Vec::new();let mut interp_results:Vec<Vec<Option<Outcome>>>=Vec::new();
    let mut failures=std::io::BufWriter::new(fs::File::create(out_dir.join("failures.tsv")).map_err(|e|e.to_string())?);
    writeln!(failures,"file\tindex\thash\tbackend\toutcome\tname\tbytes").unwrap();
    let mut disagreements=0usize;let mut stray:Vec<(String,usize)>=Vec::new();let started=Instant::now();
    for path in &files{
        let cases=parse(path)?;let name=stem(path);let mnemonic=cases.first().map(|c|c.name.split_whitespace().next().unwrap_or("").to_string()).unwrap_or_default();
        let mut tally=Tally::default();let mut input=Vec::new();let mut per_case=Vec::new();let mut sampled=0usize;
        for c in &cases{
            let mut in_sample=false;
            let outcome=if revoked.contains(&c.hash){per_case.push(None);Outcome::Revoked}else{
                let mut p=prepare(c,&mut ram);
                let o=match &p.decoded{
                    // Hardware raised #UD too: the rejection is right, delivery is absent.
                    Err(guest::GuestError::Decode(_)) if c.exception==Some(6)=>Outcome::ExceptionNotDelivered(6),
                    Err(guest::GuestError::Decode(d))=>Outcome::NotDecoded(decode_family(c,d)),
                    Err(guest::GuestError::Execution(e))=>{let a=Actual{fault:Some(fault_kind(&format!("{e:?}"))),regs:p.cpu.regs,eflags:p.cpu.eflags,undefined:0,eip:p.cpu.eip,selectors:p.image.selectors,msw:p.image.msw,real_mode:true,ram:Vec::new()};judge(c,&a,&p.readback)},
                    Ok(d)=>match &d.operation{
                        guest::Operation::Pil{address,..} if undefined_sib(&c.bytes,address)=>Outcome::UndefinedSib,
                        guest::Operation::Pil{program,..}=>{
                            // Immediates are folded into each program, so the generated
                            // backend runs a deterministic sample: the first N per file.
                            if generated&&sampled<sample{
                                let key=super::emit_rust_functions(program);let next=variants.len() as u32;
                                let id=*variant_ids.entry(key.clone()).or_insert_with(||{variants.push(key);next});
                                encode(&mut input,id,&p,c);sampled+=1;in_sample=true;
                            }
                            let actual=interpret(&mut p,&mut ram).unwrap();judge(c,&actual,&p.readback)
                        }
                        _=>Outcome::MonitorOperation,
                    },
                };
                clear(c,&mut ram);
                per_case.push(Some(o.clone()).filter(|_|in_sample));o
            };
            if !matches!(outcome,Outcome::Pass|Outcome::Revoked){writeln!(failures,"{name}\t{}\t{}\tinterpreter\t{outcome:?}\t{}\t{}",c.index,c.hash,c.name,c.bytes.iter().map(|b|format!("{b:02x}")).collect::<String>()).unwrap();}
            tally.add(&outcome);
        }
        if generated{fs::write(gen_dir.join(format!("{name}.in")),&input).map_err(|e|e.to_string())?;}
        // Only listed bytes are cleared between cases. Anything left over was
        // written at an address the hardware never touched: count it and reset.
        let stray_bytes=ram.bytes().iter().filter(|b|**b!=0).count();
        if stray_bytes!=0{stray.push((name.clone(),stray_bytes));ram=Ram::new(RAM_SIZE);}
        rows.push((name,mnemonic,tally,Tally::default()));interp_results.push(per_case);
    }
    let interp_seconds=started.elapsed().as_secs_f64();
    let mut gen_note=String::from("Generated backend not run (SST80386_GENERATED=0).");
    if generated{
        let started=Instant::now();
        let src=gen_dir.join("runner.rs");fs::write(&src,runner_source(&variants)).map_err(|e|e.to_string())?;
        let bin=gen_dir.join(if cfg!(windows){"runner.exe"}else{"runner"});let rustc=std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into());
        let built=Command::new(&rustc).args(["--edition=2021","-C","opt-level=1"]).arg(&src).arg("-o").arg(&bin).output().map_err(|e|e.to_string())?;
        fs::write(gen_dir.join("build.stderr"),&built.stderr).map_err(|e|e.to_string())?;
        if !built.status.success(){return Err(format!("generated runner build failed; see {}",gen_dir.join("build.stderr").display()));}
        let build_seconds=started.elapsed().as_secs_f64();let started=Instant::now();
        for (row,path) in rows.iter_mut().zip(&files){
            let cases=parse(path)?;let name=&row.0;let file_index=files.iter().position(|p|p==path).unwrap();
            let (inp,outp)=(gen_dir.join(format!("{name}.in")),gen_dir.join(format!("{name}.out")));
            let status=Command::new(&bin).arg(&inp).arg(&outp).status().map_err(|e|e.to_string())?;
            if !status.success(){return Err(format!("generated runner failed on {name}"));}
            let bytes=fs::read(&outp).map_err(|e|e.to_string())?;let mut words=bytes.chunks_exact(4).map(|c|u32::from_le_bytes(c.try_into().unwrap()));
            for (c,interp) in cases.iter().zip(&interp_results[file_index]){
                let Some(interp)=interp else{continue};
                let mut readback:Vec<u32>=c.ram.iter().chain(&c.fin_ram).map(|e|e.0).collect();readback.sort();readback.dedup();
                let actual=decode_actual(&mut words,readback.len()).ok_or("short generated output")?;
                let o=judge(c,&actual,&readback);
                if &o!=interp{disagreements+=1;writeln!(failures,"{name}\t{}\t{}\tdisagree\tinterpreter={interp:?} generated={o:?}\t{}\t",c.index,c.hash,c.name).unwrap();}
                row.3.add(&o);
            }
            let _=fs::remove_file(&outp);let _=fs::remove_file(&inp);
        }
        gen_note=format!("Generated backend (first {sample} PIL-decoded cases per file): {} distinct PIL programs compiled into one runner in {build_seconds:.1} s, run in {:.1} s. Outcome disagreements with the interpreter: {disagreements}.",variants.len(),started.elapsed().as_secs_f64());
    }
    failures.flush().unwrap();
    let mut text=report(&rows,generated,&gen_note,interp_seconds,files.len());
    text.push_str(&format!("\n## Writes outside the captured bytes\n\n{} files left nonzero RAM outside the listed addresses (RAM reset after each such file):\n\n",stray.len()));
    for (f,n) in &stray{text.push_str(&format!("- {f}: {n} bytes\n"));}
    Ok(text)
}
fn report(rows:&[(String,String,Tally,Tally)],generated:bool,gen_note:&str,seconds:f64,files:usize)->String{
    let mut total=Tally::default();for r in rows{for (o,n) in &r.2.counts{*total.counts.entry(o.clone()).or_insert(0)+=n;}}
    let mut gen_total=Tally::default();for r in rows{for (o,n) in &r.3.counts{*gen_total.counts.entry(o.clone()).or_insert(0)+=n;}}
    let all=total.get(|_|true);let pass=total.get(|o|*o==Outcome::Pass);
    let decoded=total.get(|o|!matches!(o,Outcome::NotDecoded(_)|Outcome::MonitorOperation|Outcome::Revoked));
    let mut s=format!("# SingleStepTests/80386 real-mode run\n\n{files} files, {all} cases, interpreter pass {pass} ({:.1}% of all, {:.1}% of decoded). Interpreter time {seconds:.1} s.\n{gen_note}\n\n",100.0*pass as f64/all.max(1) as f64,100.0*pass as f64/decoded.max(1) as f64);
    if generated{s.push_str(&format!("Generated-backend pass {} of {} PIL-decoded cases.\n\n",gen_total.get(|o|*o==Outcome::Pass),gen_total.get(|_|true)));}
    s.push_str("## Outcome totals (interpreter)\n\n| Outcome | Cases |\n|---|---:|\n");
    let mut families:BTreeMap<String,usize>=BTreeMap::new();
    for (o,n) in &total.counts{*families.entry(match o{Outcome::Fail(f)=>format!("fail: {f}"),Outcome::NotDecoded(d)=>format!("not decoded: {d}"),Outcome::ExceptionNotDelivered(_)=>"exception expected; PIL stopped with a fault or #UD rejection (no delivery)".into(),o=>format!("{o:?}")}).or_insert(0)+=n;}
    let mut fam:Vec<_>=families.into_iter().collect();fam.sort_by(|a,b|b.1.cmp(&a.1));
    for (f,n) in fam{s.push_str(&format!("| {f} | {n} |\n"));}
    s.push_str("\n## Per file\n\n| File | Mnemonic | Cases | Pass | Not decoded | Undefined-flag only | Exception, no delivery | Fail | Top failure |");
    if generated{s.push_str(" Generated pass |");}s.push_str("\n|---|---|---:|---:|---:|---:|---:|---:|---|");if generated{s.push_str("---:|");}s.push('\n');
    for (name,mn,t,g) in rows{
        let mut fails:Vec<_>=t.counts.iter().filter_map(|(o,n)|if let Outcome::Fail(f)=o{Some((f,n))}else{None}).collect();fails.sort_by(|a,b|b.1.cmp(a.1));
        let mut missing:Vec<_>=t.counts.iter().filter_map(|(o,n)|if let Outcome::NotDecoded(d)=o{Some((d,n))}else{None}).collect();missing.sort_by(|a,b|b.1.cmp(a.1));
        let top=fails.first().map(|(f,n)|format!("{f} ({n})")).or_else(||missing.first().map(|(d,n)|format!("not decoded: {d} ({n})"))).unwrap_or_default();
        s.push_str(&format!("| {name} | {mn} | {} | {} | {} | {} | {} | {} | {top} |",t.get(|_|true),t.get(|o|*o==Outcome::Pass),t.get(|o|matches!(o,Outcome::NotDecoded(_)|Outcome::MonitorOperation)),t.get(|o|*o==Outcome::UndefinedFlagsOnly),t.get(|o|matches!(o,Outcome::ExceptionNotDelivered(_))),t.get(|o|matches!(o,Outcome::Fail(_)))));
        if generated{s.push_str(&format!(" {} |",g.get(|o|*o==Outcome::Pass)));}s.push('\n');
    }s
}

#[test]
fn sst80386_moo_reader_and_judge_on_a_synthetic_case(){
    // An original two-test MOO file: ADD AL,1 (04 01) at 1000:0100 with AL=7Fh.
    // The second copy records AF clear, which ADD defines, so it must fail.
    fn chunk(id:&[u8;4],body:&[u8])->Vec<u8>{let mut v=id.to_vec();v.extend((body.len() as u32).to_le_bytes());v.extend(body);v}
    fn regs(bits:u32,values:&[u32])->Vec<u8>{let mut v=bits.to_le_bytes().to_vec();for x in values{v.extend(x.to_le_bytes());}v}
    let mut init=vec![0x10,0,0x7f,0,0,0,0,0,0,0x100,0x1000,0,0,0,0,0,0x100,2,0,0];init[2]=0x7f;
    let ram:Vec<u8>=[(0x10100u32,4u8),(0x10101,1),(0x10102,0xf4)].iter().flat_map(|(a,v)|{let mut e=a.to_le_bytes().to_vec();e.push(*v);e}).collect();
    let mut ram_chunk=3u32.to_le_bytes().to_vec();ram_chunk.extend(&ram);
    let mut file=chunk(b"MOO ",&[1,1,0,0,2,0,0,0,b'3',b'8',b'6',b'E']);
    for (i,flags) in [(0u32,0x892u32),(1,0x882)]{
        let mut t=i.to_le_bytes().to_vec();t.extend(chunk(b"NAME",&[4,0,0,0,b'a',b'd',b'd',b' ']));t.extend(chunk(b"BYTS",&[3,0,0,0,4,1,0xf4]));
        t.extend(chunk(b"INIT",&[chunk(b"RG32",&regs(0xfffff,&init)),chunk(b"RAM ",&ram_chunk)].concat()));
        t.extend(chunk(b"FINA",&[chunk(b"RG32",&regs((1<<2)|(1<<16)|(1<<17),&[0x80,0x103,flags])),chunk(b"RAM ",&0u32.to_le_bytes())].concat()));
        t.extend(chunk(b"HASH",&[i as u8;20]));file.extend(chunk(b"TEST",&t));
    }
    // Stored (uncompressed) DEFLATE block wrapped in a gzip member.
    let mut gz=vec![0x1f,0x8b,8,0,0,0,0,0,0,0xff,1];gz.extend((file.len() as u16).to_le_bytes());gz.extend((!(file.len() as u16)).to_le_bytes());gz.extend(&file);
    gz.extend(super::containers::crc32(&file).to_le_bytes());gz.extend((file.len() as u32).to_le_bytes());
    let dir=std::env::temp_dir().join(format!("pil-sst-{}",std::process::id()));fs::create_dir_all(&dir).unwrap();let path=dir.join("04.MOO.gz");fs::write(&path,&gz).unwrap();
    let cases=parse(&path).unwrap();fs::remove_dir_all(&dir).unwrap();
    assert_eq!(cases.len(),2);assert_eq!(cases[0].bytes,[4,1,0xf4]);assert_eq!(cases[0].fin[2],Some(0x80));
    let mut ram=Ram::new(RAM_SIZE);
    let mut outcomes=Vec::new();
    for c in &cases{let mut p=prepare(c,&mut ram);let a=interpret(&mut p,&mut ram).unwrap();outcomes.push(judge(c,&a,&p.readback));clear(c,&mut ram);}
    // 7F+1=80: OF SF AF set, ZF CF clear, PF clear (80h has one bit).
    assert_eq!(outcomes,[Outcome::Pass,Outcome::Fail("eflags AF".into())]);
    assert!(repeated_prefixes(&[0x26,0x2e,0x01,0xc0]));assert!(!repeated_prefixes(&[0x66,0x67,0x26,0x01]));
}

#[test]
#[ignore="needs SingleStepTests/80386 data: run tools/verify_sst80386.py"]
fn sst80386_real_mode_suite(){
    let root=PathBuf::from(std::env::var("SST80386_DIR").expect("SST80386_DIR must name a SingleStepTests/80386 checkout"));
    let out=PathBuf::from(std::env::var("SST80386_OUT").unwrap_or_else(|_|"target/external-tests/sst80386/report".into()));
    fs::create_dir_all(&out).unwrap();
    let report=run(&root,&out).unwrap();fs::write(out.join("report.md"),&report).unwrap();
    println!("{}",report.lines().take(4).collect::<Vec<_>>().join("\n"));println!("SST80386 report: {}",out.join("report.md").display());
}
