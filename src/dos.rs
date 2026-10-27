//! Bounded DOS executable importer and diagnostic runner, not a DOS/BIOS kernel.
use std::{collections::BTreeMap,fmt::Write as _,io::Read,path::Path};
use super::{Cpu,execute_flow,execute_segmented};
use super::address::{MemoryContext,SegmentAccess,SegmentCache,SegmentRegister};
use super::guest::{GuestError,Image,Operation};
use super::memory::{MemWidth,Ram};
const PSP:u16=0x1000;
const LOAD:u16=PSP+0x10;
const LIMIT:usize=0xa0000;
pub const MAX_FILE:usize=1024*1024;
pub const MAX_STEPS:usize=1_000_000;
const MAX_OUTPUT:usize=16384;
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Format{Com,Mz}
#[derive(Clone,Debug)]
pub struct Loaded{
    pub format:Format,pub cpu:Cpu,pub ram:Ram,pub image:Image,
    pub selectors:[u16;6],pub relocations:usize,pub module_bytes:usize,
    pub header_bytes:usize,pub module_base:u32,pub allocated_end:u32,pub trailing_bytes:usize,
}
fn word(bytes:&[u8],at:usize)->Result<u16,String>{
    let b=bytes.get(at..at+2).ok_or("truncated DOS header/relocation word")?;Ok(u16::from_le_bytes([b[0],b[1]]))
}
fn put_word(bytes:&mut[u8],at:usize,n:u16){bytes[at..at+2].copy_from_slice(&n.to_le_bytes());}
pub fn hello(format:&str)->Result<Vec<u8>,String>{
    // Both original examples address their string relative to the initial PSP DS.
    let mut code=vec![0xba,0x0c,0x01,0xb4,9,0xcd,0x21,0xb8,0,0x4c,0xcd,0x21];
    code.extend_from_slice(b"Hello, DOS!\r\n$");
    match format{
        "com"=>Ok(code),
        "exe"=>{let mut h=vec![0;32];h[..2].copy_from_slice(b"MZ");let size=h.len()+code.len();
            put_word(&mut h,2,(size%512)as u16);put_word(&mut h,4,((size+511)/512)as u16);
            put_word(&mut h,8,2);put_word(&mut h,12,0xffff);put_word(&mut h,16,0xfffe);put_word(&mut h,24,28);
            h.extend(code);Ok(h)
        },_=>Err("hello format must be com or exe".into()),
    }
}
pub fn read_file(path:&Path)->Result<Vec<u8>,String>{
    let file=std::fs::File::open(path).map_err(|e|e.to_string())?;let mut bytes=Vec::new();
    file.take((MAX_FILE+1)as u64).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len()>MAX_FILE{return Err("DOS input exceeds 1 MiB file limit".into());}Ok(bytes)
}
pub fn load(bytes:&[u8],name:&str)->Result<Loaded,String>{
    if bytes.is_empty()||bytes.len()>MAX_FILE{return Err("DOS input must contain 1..1048576 bytes".into());}
    let mz=bytes.starts_with(b"MZ")||bytes.starts_with(b"ZM");
    if !mz&&Path::new(name).extension().is_some_and(|x|x.to_str().is_some_and(|s|s.eq_ignore_ascii_case("exe"))){
        return Err(".EXE input requires a DOS MZ/ZM header; not a raw monitor image".into());
    }
    let(mut format,mut cpu,mut selectors,mut header_bytes,mut relocations)=(Format::Com,Cpu::default(),[PSP;6],0,0);
    let(mut module_base,mut module_bytes,mut start,mut end,mut allocation_end,mut trailing)=(u32::from(PSP)*16+0x100,bytes.len(),0u32,0x100+bytes.len()as u32,LIMIT,0);
    let mut module=bytes.to_vec();
    cpu.eip=0x100;cpu.regs[4]=0xfffe;cpu.eflags=0x202;
    if mz{
        format=Format::Mz;
        if bytes.len()<28{return Err("truncated MZ header (minimum 28 bytes)".into());}
        let last=usize::from(word(bytes,2)?);let pages=usize::from(word(bytes,4)?);
        if pages==0||last>511{return Err("invalid MZ page/last-page size".into());}
        let declared=(pages-1)*512+if last==0{512}else{last};
        header_bytes=usize::from(word(bytes,8)?)*16;
        if header_bytes<28||header_bytes>=declared||declared>bytes.len(){return Err("MZ header/module sizes exceed file or describe an empty module".into());}
        if word(bytes,26)?!=0{return Err("MZ overlays are not supported".into());}
        if header_bytes>=64{
            let at=u32::from_le_bytes(bytes[60..64].try_into().unwrap())as usize;
            if let Some(sig)=bytes.get(at..at.saturating_add(2)){
                if [b"NE".as_slice(),b"LE",b"LX",b"PE",b"W3",b"W4"].contains(&sig){return Err("NE/LE/LX/PE/Windows executables are not DOS MZ inputs for this probe".into());}
            }
        }
        let min=usize::from(word(bytes,10)?);let max=usize::from(word(bytes,12)?);
        if max<min{return Err("MZ maximum allocation is below minimum".into());}
        if min==0&&max==0{return Err("MZ high-load convention (min=max=0) is not supported".into());}
        module=bytes[header_bytes..declared].to_vec();module_bytes=module.len();trailing=bytes.len()-declared;
        let resident=(module_bytes+15)/16;let available=(LIMIT-usize::from(PSP)*16)/16;
        if 16+resident+min>available{return Err("MZ image/minimum allocation exceeds conventional probe memory".into());}
        allocation_end=(usize::from(PSP)+(16+resident+max).min(available))*16;
        module_base=u32::from(LOAD)*16;
        relocations=usize::from(word(bytes,6)?);let table=usize::from(word(bytes,24)?);
        if relocations!=0&&(table<28||table+relocations*4>header_bytes){return Err("MZ relocation table is outside header".into());}
        for i in 0..relocations{
            let at=table+i*4;let offset=usize::from(word(bytes,at)?);let seg=usize::from(word(bytes,at+2)?);
            let target=seg*16+offset;
            if target+2>module.len(){return Err(format!("MZ relocation {i} is outside load module"));}
            let n=word(&module,target)?.wrapping_add(LOAD);put_word(&mut module,target,n);
        }
        let cs=word(bytes,22)?;let ip=word(bytes,20)?;let ss=word(bytes,14)?;let sp=word(bytes,16)?;
        // MZ segment values are relocated as16-bit words. FFF0:0100 commonly
        // wraps CS to the PSP while still entering the module at LOAD:0000.
        let cs=LOAD.wrapping_add(cs);let ss=LOAD.wrapping_add(ss);
        let cs_base=usize::from(cs)*16;let entry=cs_base+usize::from(ip);
        if entry<module_base as usize||entry>=module_base as usize+module.len(){return Err("MZ initial CS:IP is outside load module".into());}
        if usize::from(ss)*16<usize::from(PSP)*16||usize::from(ss)*16+usize::from(sp)>allocation_end{return Err("MZ initial SS:SP is outside allocated probe block".into());}
        selectors[SegmentRegister::Cs.index()]=cs;selectors[SegmentRegister::Ss.index()]=ss;
        cpu.eip=u32::from(ip);cpu.regs[4]=u32::from(sp);start=(module_base as usize).saturating_sub(cs_base)as u32;end=(module_base as usize+module.len()-cs_base).min(65536)as u32;
    }else if bytes.len()>0xff00{return Err("COM input exceeds 65280-byte single-segment limit".into());}
    let mut ram=Ram::new(allocation_end);
    let psp_base=u32::from(PSP)*16;let mut psp=[0u8;256];psp[..2].copy_from_slice(&[0xcd,0x20]);
    put_word(&mut psp,2,(allocation_end/16)as u16);psp[0x81]=0x0d;
    ram.write_bytes(psp_base,&psp).map_err(|e|format!("{e:?}"))?;
    ram.write_bytes(module_base,&module).map_err(|e|format!("{e:?}"))?;
    if format==Format::Com{ram.store(psp_base+0xfffe,MemWidth::Word,0).map_err(|e|format!("{e:?}"))?;}
    let mut segments=selectors.map(SegmentCache::real_mode);
    segments[SegmentRegister::Cs.index()].access=SegmentAccess::Code{readable:true};
    Ok(Loaded{format,cpu,ram,image:Image{start,end,segments,selectors},selectors,relocations,module_bytes,header_bytes,module_base,allocated_end:allocation_end as u32,trailing_bytes:trailing})
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Stop{Exit(u8),MissingEncoding{key:String,detail:String},Service(String),Fault(String),Boundary(String),Budget,Monitor(String)}
#[derive(Clone,Debug)]
pub struct Probe{
    pub loaded:Loaded,pub stop:Stop,pub completed:usize,pub pil_steps:usize,pub dos_calls:usize,
    pub output:Vec<u8>,pub instructions:BTreeMap<String,usize>,pub stop_bytes:Vec<u8>,pub file_offset:Option<usize>,
}
fn bytes_at(p:&Loaded,n:u32)->Result<Vec<u8>,String>{
    p.image.fetch_signature(&p.ram,p.cpu.eip,n).map(|b|b[..n as usize].to_vec()).map_err(|e|format!("{e:?}"))
}
fn site_bytes(p:&Loaded)->Vec<u8>{
    (1..=8).filter_map(|n|bytes_at(p,n).ok().and_then(|b|b.last().copied())).collect()
}
fn opcode_key(bytes:&[u8])->String{
    let at=bytes.iter().position(|b|!matches!(*b,0x26|0x2e|0x36|0x3e|0x64|0x65|0x66|0x67)).unwrap_or(0);
    match bytes.get(at){Some(0x0f)=>format!("opcode 0f {}",bytes.get(at+1).map(|x|format!("{x:02x}")).unwrap_or_else(||"??".into())),Some(b)=>format!("opcode {b:02x}"),None=>"fetch/truncated encoding".into()}
}
fn service(p:&mut Loaded,vector:u8,out:&mut Vec<u8>)->Result<Option<u8>,String>{
    match(vector,(p.cpu.regs[0]>>8)as u8){
        (0x20,_)=>if p.selectors[SegmentRegister::Cs.index()]==PSP{Ok(Some(0))}else{Err("INT20 requires CS=PSP in this shim".into())},
        (0x21,0x4c)=>Ok(Some(p.cpu.regs[0]as u8)),
        (0x21,9)=>{
            let offset=p.cpu.regs[2]&0xffff;let mut text=Vec::new();
            let context=MemoryContext{segment:SegmentRegister::Ds,cache:p.image.segments[SegmentRegister::Ds.index()]};
            for n in offset..=0xffff{
                let addr=context.translate(n,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS string access: {e:?}"))?;
                let b=p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS string RAM access: {e:?}"))?as u8;
                if b==b'$'{
                    out.extend(text);p.cpu.regs[0]=(p.cpu.regs[0]&!255)|0x24;return Ok(None);
                }
                if out.len()+text.len()>=MAX_OUTPUT{return Err("DOS output limit reached before string terminator".into());}
                text.push(b);
            }Err("DOS string has no $ terminator before segment end".into())
        },_=>Err(format!("unimplemented DOS/BIOS service INT {vector:02x}, AH={:02x}",(p.cpu.regs[0]>>8)&255)),
    }
}
pub fn run(mut p:Loaded,budget:usize)->Result<Probe,String>{
    if !(1..=MAX_STEPS).contains(&budget){return Err("DOS step budget must be 1..1000000".into());}
    let(mut completed,mut pil_steps,mut dos_calls)=(0,0,0);let mut output=Vec::new();let mut instructions=BTreeMap::new();
    let mut cache=super::cache::DecodeCache::default();
    let stop=loop{
        if completed==budget{break Stop::Budget;}
        if let Ok(encoded)=bytes_at(&p,2){if encoded[0]==0xcd{
            let vector=encoded[1];let key=format!("DOS INT{vector:02x}/AH{:02x}",(p.cpu.regs[0]>>8)&255);
            match service(&mut p,vector,&mut output){
                Ok(exit)=>{p.cpu.eip=(p.cpu.eip+2)&0xffff;completed+=1;dos_calls+=1;*instructions.entry(key).or_insert(0)+=1;
                    if let Some(code)=exit{break Stop::Exit(code);}continue;},
                Err(message)=>break Stop::Service(message),
            }
        }}
        let decoded=match cache.fetch(&p.image,&p.ram,p.cpu.eip){
            Ok(d)=>d,Err(GuestError::Execution(e))=>break Stop::Fault(format!("{e:?}")),
            Err(GuestError::Decode(detail))=>break if detail.starts_with("fetch outside loaded image"){Stop::Boundary(detail)}else{Stop::MissingEncoding{key:opcode_key(&site_bytes(&p)),detail}},
        };
        let result=match &decoded.operation{
            Operation::Pil{program,dst,src,flow,address}=>match address{
                Some(a)=>execute_segmented(program,&mut p.cpu,*dst,*src,*flow,&mut p.ram,*a),
                None=>execute_flow(program,&mut p.cpu,*dst,*src,*flow),
            },
            Operation::Halt=>break Stop::Monitor("HLT reached;monitor stop is not DOS process termination".into()),
            _=>break Stop::Monitor("monitor-port I/O is outside the DOS probe shim".into()),
        };
        if let Err(e)=result{break Stop::Fault(format!("{e:?}"));}
        completed+=1;pil_steps+=1;*instructions.entry(decoded.label.split_whitespace().next().unwrap_or("?").to_string()).or_insert(0)+=1;
    };
    let stop_bytes=site_bytes(&p);let linear=p.image.segments[SegmentRegister::Cs.index()].base.checked_add(p.cpu.eip);
    let file_offset=linear.and_then(|a|a.checked_sub(p.module_base)).filter(|n|(*n as usize)<p.module_bytes).map(|n|n as usize+p.header_bytes);
    Ok(Probe{loaded:p,stop,completed,pil_steps,dos_calls,output,instructions,stop_bytes,file_offset})
}
fn ranked(counts:&BTreeMap<String,usize>)->Vec<(&String,&usize)>{let mut v:Vec<_>=counts.iter().collect();v.sort_by(|a,b|b.1.cmp(a.1).then(a.0.cmp(b.0)));v}
pub fn report(paths:&[String],budget:usize)->Result<String,String>{
    if paths.is_empty()||paths.len()>256{return Err("DOS probe accepts 1..256 file paths".into());}
    if !(1..=MAX_STEPS).contains(&budget){return Err("DOS step budget must be 1..1000000".into());}
    let mut report=String::from("DOS executable probe: concrete PIL interpreter plus minimal host DOS-call shim.\n");
    let mut inputs:Vec<(String,Result<Vec<u8>,String>)>=Vec::new();
    let mut fingerprints:BTreeMap<u64,Vec<Vec<u8>>>=BTreeMap::new();
    for path in paths{match super::containers::open(Path::new(path)){
        Ok(Some(archive))=>{
            let mut candidates=0;let mut added=0;
            for note in archive.notes{writeln!(report,"CONTAINER NOTE {path}: {note}").unwrap();}
            for e in archive.files{if !super::containers::executable(&e.path){continue;}candidates+=1;
                use std::hash::{Hash,Hasher};let mut h=std::collections::hash_map::DefaultHasher::new();e.bytes.hash(&mut h);
                let bucket=fingerprints.entry(h.finish()).or_default();if bucket.iter().any(|b|b==&e.bytes){continue;}bucket.push(e.bytes.clone());
                inputs.push((format!("{path}::{}",e.path),Ok(e.bytes)));added+=1;
                if inputs.len()>256{return Err("more than256 unique container executables;extract and select a smaller corpus".into());}
            }
            writeln!(report,"CONTAINER {path}: candidates={candidates} added_unique={added}").unwrap();
        },
        Ok(None)=>inputs.push((path.clone(),read_file(Path::new(path)))),
        Err(e)=>inputs.push((path.clone(),Err(e))),
    }}
    if inputs.len()>256{return Err("probe input count exceeds256 after container expansion".into());}
    let total=inputs.len();

    let(mut exited,mut imported)=(0,0);let mut blockers=BTreeMap::new();let mut mix=BTreeMap::new();
    for(path,bytes)in inputs{
        writeln!(report,"\nFILE {path}").unwrap();
        let p=match bytes.and_then(|b|load(&b,&path)){
            Ok(p)=>p,Err(e)=>{writeln!(report,"IMPORT ERROR: {e}").unwrap();continue;}
        };imported+=1;
        writeln!(report,"IMPORT {:?}: module={} bytes header={} relocations={} trailing={} PSP={PSP:04x} allocation_end={:05x}",p.format,p.module_bytes,p.header_bytes,p.relocations,p.trailing_bytes,p.allocated_end).unwrap();
        writeln!(report,"ENTRY CS:IP={:04x}:{:04x} DS={:04x} ES={:04x} SS:SP={:04x}:{:04x}",p.selectors[1],p.cpu.eip,p.selectors[3],p.selectors[0],p.selectors[2],p.cpu.regs[4]).unwrap();
        let result=run(p,budget)?;
        if matches!(result.stop,Stop::Exit(_)){exited+=1;}
        if let Stop::MissingEncoding{key,..}=&result.stop{*blockers.entry(key.clone()).or_insert(0)+=1;}
        for(k,n)in &result.instructions{*mix.entry(k.clone()).or_insert(0)+=n;}
        writeln!(report,"STOP {:?}; CS:IP={:04x}:{:04x}; file_offset={:?}; bytes={}",result.stop,result.loaded.selectors[1],result.loaded.cpu.eip,result.file_offset,result.stop_bytes.iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join(" ")).unwrap();
        writeln!(report,"completed={} PIL={} DOS_calls={} EAX={:08x} ESP={:08x}",result.completed,result.pil_steps,result.dos_calls,result.loaded.cpu.regs[0],result.loaded.cpu.regs[4]).unwrap();
        writeln!(report,"OUTPUT (escaped bytes): {}",result.output.iter().map(|b|std::ascii::escape_default(*b).map(char::from).collect::<String>()).collect::<String>()).unwrap();
    }
    writeln!(report,"\nSUMMARY imported={imported}/{} exited={exited}; first missing-encoding blockers (program count):",total).unwrap();
    if blockers.is_empty(){report.push_str("none encountered on executed paths\n");}
    for(k,n)in ranked(&blockers){writeln!(report,"{n} {k}").unwrap();}
    report.push_str("Reached instruction mix (executed count; DOS bridges listed separately):\n");
    for(k,n)in ranked(&mix).into_iter().take(20){writeln!(report,"{n} {k}").unwrap();}
    report.push_str("Stops at the first blocker per program; no skipping unknown bytes or static scanning of data. This is not whole-program coverage or a full DOS/BIOS implementation.\n");
    Ok(report)
}
#[cfg(test)]
#[path="../tests/unit/dos.rs"]
mod tests;
