//! Bounded DOS executable importer and diagnostic runner, not a DOS/BIOS kernel.
use std::{collections::BTreeMap,fmt::Write as _,io::Read,path::Path};
use super::Cpu;
use super::address::{MemoryContext,SegmentCache,SegmentRegister};
use super::guest::{GuestError,Image,Operation};
use super::memory::{MemWidth,Ram};
#[path="dos_memory.rs"]
mod memory_arena;
use memory_arena::{Arena,memory_service};
// Declared compatibility version of this bounded host shim, not an OS boot.
const DOS_COMPAT_VERSION:u16=0x1606; // major6, minor22 decimal
const PSP:u16=0x1000;
const LOAD:u16=PSP+0x10;
const LIMIT:usize=0xa0000;
/// Paragraphs of the environment block the probe gives each program.
const ENV_PARAS:u16=0x10;
const ENV:u16=PSP-1-ENV_PARAS;
/// Segment of Dine's DOS data: the critical-error and InDOS bytes (AH34) at
/// offsets 0 and 1, and the AH52 list of lists at SYSVARS with the first MCB
/// segment in the word before it.
const DOSDATA:u16=0x0070;
const INDOS:u16=0x0001;
const SYSVARS:u16=0x0026;
/// Offset in DOSDATA of the 128-byte line buffer CON reads (AH3F) edit into.
const CON_LINE:u16=0x0300;
/// BIOS timer ticks in 24 hours (INT1A, BDA 0040:006C).
const TICKS_PER_DAY:u32=0x0018_00b0;
pub const MAX_FILE:usize=1024*1024;
pub const MAX_STEPS:usize=1_000_000;
#[cfg(test)]
const MAX_OUTPUT:usize=crate::host_io::DOS_OUTPUT_LIMIT;
#[path="dos_services.rs"]
mod services;
#[path="dos_console.rs"]
mod console;
#[path="dos_session.rs"]
pub mod session;
#[path="dos_files.rs"]
mod files;
#[path="dos_nls.rs"]
mod nls;
#[path="dos_exec.rs"]
mod exec;
use services::dispatch as service;
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Format{Com,Mz}
#[derive(Clone,Debug)]
pub struct Loaded{
    arena:Arena,
    pub format:Format,pub cpu:Cpu,pub ram:Ram,pub image:Image,
    pub selectors:[u16;6],pub relocations:usize,pub module_bytes:usize,
    pub header_bytes:usize,pub module_base:u32,pub allocated_end:u32,pub trailing_bytes:usize,
    pub timer_ticks:u32,pub open_handles:u32,
    pub dta:(u16,u16),
    /// DOS console state: scan code owed after an ASCII-0 key, and an AH0C flush
    /// already performed for the read that is waiting.
    pub pending_scan:Option<u8>,pub flushed:bool,
    /// AH0A line being edited across waits: bytes so far, entry column, first key.
    pub line:Option<(Vec<u8>,usize,bool)>,
    /// Set by a service that consumed what input there was and must wait.
    pub input_wait:bool,
    /// AH58 allocation strategy (first fit only is modelled) and the AH31
    /// paragraphs kept resident when the program terminated that way.
    pub alloc_strategy:u8,pub resident:Option<u16>,
    /// The program's own file, which opens read-only by its upper-case base
    /// name from any directory, and the drive with the files AH3D opened.
    pub own_file:std::sync::Arc<Vec<u8>>,pub own_name:Vec<u8>,pub drive:files::Drives,
    /// The running program's PSP, the parents waiting in EXEC, the last
    /// child's return code for AH4D, and console bytes AH3F read but did not
    /// return yet.
    pub psp:u16,exec:Vec<exec::Frame>,pub return_code:u16,con_pending:Vec<u8>,
}
impl Loaded{
    /// Inject one BIOS scan/ASCII word; false means the bounded ring is full.
    #[allow(dead_code)]
    pub fn enqueue_key(&mut self,key:u16)->Result<bool,String>{console::enqueue(self,key)}
}
fn word(bytes:&[u8],at:usize)->Result<u16,String>{
    let b=bytes.get(at..at+2).ok_or("truncated DOS header/relocation word")?;Ok(u16::from_le_bytes([b[0],b[1]]))
}
fn put_word(bytes:&mut[u8],at:usize,n:u16){bytes[at..at+2].copy_from_slice(&n.to_le_bytes());}
/// C000:0000..F000:FFFF reads as an empty bus (FFh, so no option-ROM 55AAh
/// signature) and is read-only. The top 16 bytes carry what a PC/AT-class BIOS
/// identifies itself with: the reset jump, a BIOS date at F000:FFF5 and model
/// byte FCh at F000:FFFE. No BIOS code is present; services are answered by Dine.
/// The list of lists holds only what Dine models: the first MCB, 512-byte
/// sectors and a NUL device header ending the device chain. Pointers to
/// structures Dine has no (DPB, SFT, CLOCK$, CON, buffers, CDS, FCBs) are
/// FFFF:FFFF, and there are no block devices or drives yet.
fn dos_data(ram:&mut Ram)->Result<(),String>{
    let mut data=vec![0u8;usize::from(SYSVARS)+0x40];
    put_word(&mut data,usize::from(SYSVARS)-2,ENV-1);
    for field in [0x00,0x04,0x08,0x0c,0x12,0x16,0x1a]{data[usize::from(SYSVARS)+field..][..4].fill(0xff);}
    put_word(&mut data,usize::from(SYSVARS)+0x10,512);
    let nul=usize::from(SYSVARS)+0x22;data[nul..nul+4].fill(0xff);put_word(&mut data,nul+4,0x8004);data[nul+10..nul+18].copy_from_slice(b"NUL     ");
    ram.write_bytes(u32::from(DOSDATA)*16,&data).map_err(|e|format!("DOS data area: {e:?}"))?;
    nls::install(ram)
}
fn rom(ram:&mut Ram)->Result<(),String>{
    let mut top=[0xffu8;16];
    top[..5].copy_from_slice(&[0xea,0x5b,0xe0,0x00,0xf0]);
    top[5..13].copy_from_slice(b"01/10/84");top[13]=0;top[14]=0xfc;top[15]=0;
    ram.write_bytes(0xc0000,&vec![0xff;0x3fff0]).map_err(|e|format!("{e:?}"))?;
    ram.write_bytes(0xffff0,&top).map_err(|e|format!("{e:?}"))?;
    ram.protect(0xc0000,0x40000).map_err(|e|format!("{e:?}"))
}
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
fn base_name(path:&[u8])->Vec<u8>{
    let start=path.iter().rposition(|&b|matches!(b,b'/'|b'\\'|b':')).map_or(0,|i|i+1);
    path[start..].iter().filter(|b|b.is_ascii_graphic()).map(|b|b.to_ascii_uppercase()).take(12).collect()
}
/// DOS 3+ environment: COMSPEC, an empty string ending the list, the count
/// word 1, then the program's own path: `path` on the default drive.
fn environment(path:&[u8],drive:u8)->Vec<u8>{
    let drive=char::from(drive);
    let mut env=format!("COMSPEC={drive}:\\COMMAND.COM\0\0\x01\0{drive}:\\").into_bytes();env.extend(path);env.push(0);
    env
}
pub use files::Source;
/// Loads a program with no drives: A: is an empty read-only drive.
pub fn load(bytes:&[u8],name:&str)->Result<Loaded,String>{load_with_drives(bytes,name,Vec::new(),None)}
/// Loads a program with `drives` as read-only C:, D:, ...; `path` is where the
/// program lies on C: (else its base name in the root).
pub fn load_with_drives(bytes:&[u8],name:&str,drives:Vec<Source>,path:Option<&str>)->Result<Loaded,String>{
    for source in &drives{if let Source::Host(root)=source{if !root.is_dir(){return Err(format!("mount {} is not a folder",root.display()));}}}
    let drive=files::Drives::new(drives);
    let program=match path{
        Some(path)=>path.bytes().map(|b|if b==b'/'{b'\\'}else{b.to_ascii_uppercase()}).filter(|b|b.is_ascii_graphic()).take(64).collect(),
        None=>base_name(name.as_bytes()),
    };
    if bytes.is_empty()||bytes.len()>MAX_FILE{return Err("DOS input must contain 1..1048576 bytes".into());}
    let mz=bytes.starts_with(b"MZ")||bytes.starts_with(b"ZM");
    if !mz&&Path::new(name).extension().is_some_and(|x|x.to_str().is_some_and(|s|s.eq_ignore_ascii_case("exe"))){
        return Err(".EXE input requires a DOS MZ/ZM header; not a raw monitor image".into());
    }
    let(mut format,mut cpu,mut selectors,mut header_bytes,mut relocations)=(Format::Com,Cpu::default(),[PSP;6],0,0);
    let(mut module_base,mut module_bytes,mut allocation_end,mut trailing)=(u32::from(PSP)*16+0x100,bytes.len(),LIMIT,0);
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
        cpu.eip=u32::from(ip);cpu.regs[4]=u32::from(sp);
    }else if bytes.len()>0xff00{return Err("COM input exceeds 65280-byte single-segment limit".into());}
    let mut ram=Ram::new(0x100000);
    console::initialize(&mut ram)?;
    rom(&mut ram)?;
    let psp_base=u32::from(PSP)*16;let mut psp=[0u8;256];psp[..2].copy_from_slice(&[0xcd,0x20]);
    put_word(&mut psp,2,(allocation_end/16)as u16);psp[0x18..0x1d].copy_from_slice(&[0,1,2,3,4]);psp[0x1d..0x2c].fill(0xff);
    put_word(&mut psp,0x40,DOS_COMPAT_VERSION);psp[0x81]=0x0d;put_word(&mut psp,0x2c,ENV);
    put_word(&mut psp,0x32,20);put_word(&mut psp,0x34,0x18);put_word(&mut psp,0x36,PSP);
    ram.write_bytes(psp_base,&psp).map_err(|e|format!("{e:?}"))?;
    ram.write_bytes(u32::from(ENV)*16,&environment(&program,drive.letter())).map_err(|e|format!("{e:?}"))?;
    ram.write_bytes(module_base,&module).map_err(|e|format!("{e:?}"))?;
    if format==Format::Com{ram.store(psp_base+0xfffe,MemWidth::Word,0).map_err(|e|format!("{e:?}"))?;}
    let segments=selectors.map(SegmentCache::real_mode);
    let arena=Arena::new((allocation_end/16)as u16);arena.write_initial(&mut ram)?;
    dos_data(&mut ram)?;
    ram.store(0x460,MemWidth::Word,0x0607).map_err(|e|format!("BIOS cursor initialization: {e:?}"))?;
    // Code may run anywhere in the block DOS gave the program (PSP to allocation
    // end), e.g. self-relocating unpackers; fetches elsewhere stay a Boundary.
    Ok(Loaded{arena,format,cpu,ram,image:Image{fetch_base:psp_base,start:0,end:allocation_end as u32-psp_base,system:nls::CODE,segments,selectors,msw:0x10,real_mode:true,ss_shadow:0,sti_shadow:0,flag_checkpoint:None},selectors,relocations,module_bytes,header_bytes,module_base,allocated_end:allocation_end as u32,trailing_bytes:trailing,timer_ticks:0,open_handles:0x1f,dta:(PSP,0x80),pending_scan:None,flushed:false,line:None,input_wait:false,alloc_strategy:0,resident:None,own_file:std::sync::Arc::new(bytes.to_vec()),own_name:base_name(name.as_bytes()),drive,psp:PSP,exec:Vec::new(),return_code:0,con_pending:Vec::new()})
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Stop{Exit(u8),MissingEncoding{key:String,detail:String},Service(String),Fault(String),Boundary(String),Budget,WaitingForInput,Monitor(String)}
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

pub fn run(p:Loaded,budget:usize)->Result<Probe,String>{
    let mut output=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let mut probe=run_with_output(p,budget,&mut output)?;
    probe.output=output.into_bytes();Ok(probe)
}
pub fn run_with_output(mut p:Loaded,budget:usize,output:&mut impl crate::host_io::Output)->Result<Probe,String>{
    if !(1..=MAX_STEPS).contains(&budget){return Err("DOS step budget must be 1..1000000".into());}
    let(mut completed,mut pil_steps,mut dos_calls)=(0,0,0);let mut execution_steps=0;let mut instructions=BTreeMap::new();
    let mut cache=super::cache::DecodeCache::default();
    let stop=loop{
        if execution_steps==budget{break Stop::Budget;}
        if let Ok(encoded)=bytes_at(&p,2){if encoded[0]==0xcd{
            let vector=encoded[1];let key=format!("DOS INT{vector:02x}/AH{:02x}",(p.cpu.regs[0]>>8)&255);
            match console::waiting(&mut p,vector){
                Ok(true)=>break Stop::WaitingForInput,
                Err(message)=>break Stop::Service(message),
                Ok(false)=>{},
            }
            match service(&mut p,vector,output){
                Ok(_) if p.input_wait=>{p.input_wait=false;break Stop::WaitingForInput},
                Ok(exit)=>{p.cpu.eip=(p.cpu.eip+2)&0xffff;p.image.retired();completed+=1;execution_steps+=1;dos_calls+=1;*instructions.entry(key).or_insert(0)+=1;
                    if let Some(code)=exit{
                        // A child returns to the parent that ran it; the first program ends the run.
                        if p.exec.is_empty(){break Stop::Exit(code);}
                        let kind=if p.resident.is_some(){3}else{0};
                        if let Err(message)=exec::terminate(&mut p,code,kind){break Stop::Service(message);}
                    }continue;},
                Err(message)=>break Stop::Service(message),
            }
        }}
        let decoded=match cache.fetch(&p.image,&p.ram,p.cpu.eip){
            Ok(d)=>d,Err(GuestError::Execution(e))=>break Stop::Fault(format!("{e:?}")),
            Err(GuestError::Decode(detail))=>break if detail.starts_with("fetch outside loaded image"){Stop::Boundary(detail)}else{Stop::MissingEncoding{key:opcode_key(&site_bytes(&p)),detail}},
        };
        let old_ip=p.cpu.eip;let old_count=p.cpu.regs[1];
        let result=match &decoded.operation{
            Operation::Pil{program,dst,src,flow,address}=>{
                let mut machine=p.image.segment_state();let result=super::execute_machine(program,&mut p.cpu,*dst,*src,*flow,&mut p.ram,*address,&mut machine);
                p.image.set_segment_state(machine);p.selectors=machine.selectors;result
            },
            Operation::Halt if p.cpu.eflags&0x200!=0=>{
                // With IF=1 the timer interrupt ends HLT: one tick passes, then
                // execution continues after it. Programs use this to idle.
                p.timer_ticks=(p.timer_ticks+1)%TICKS_PER_DAY;
                p.cpu.eip=(p.cpu.eip+decoded.length)&0xffff;p.image.retired();
                completed+=1;execution_steps+=1;*instructions.entry("HLT (idle tick)".into()).or_insert(0)+=1;continue;
            },
            Operation::Halt=>break Stop::Monitor("HLT with IF=0 never resumes; monitor stop is not DOS process termination".into()),
            _=>break Stop::Monitor("monitor-port I/O is outside the DOS probe shim".into()),
        };
        if let Err(e)=result{break Stop::Fault(format!("{e:?}"));}
        execution_steps+=1;pil_steps+=1;
        let repeated=matches!(&decoded.operation,Operation::Pil{program,..}if program.partial);
        let partial=repeated&&p.cpu.eip==old_ip;
        if repeated&&p.cpu.regs[1]!=old_count{*instructions.entry(format!("{} elements",decoded.label)).or_insert(0)+=1;}
        if !partial{completed+=1;let key=if repeated{decoded.label.clone()}else{decoded.label.split_whitespace().next().unwrap_or("?").to_string()};*instructions.entry(key).or_insert(0)+=1;}

    };
    let stop_bytes=site_bytes(&p);let linear=p.image.segments[SegmentRegister::Cs.index()].base.checked_add(p.cpu.eip);
    let file_offset=linear.and_then(|a|a.checked_sub(p.module_base)).filter(|n|(*n as usize)<p.module_bytes).map(|n|n as usize+p.header_bytes);
    Ok(Probe{loaded:p,stop,completed,pil_steps,dos_calls,output:output.bytes().to_vec(),instructions,stop_bytes,file_offset})
}
fn ranked(counts:&BTreeMap<String,usize>)->Vec<(&String,&usize)>{let mut v:Vec<_>=counts.iter().collect();v.sort_by(|a,b|b.1.cmp(a.1).then(a.0.cmp(b.0)));v}
pub fn report(paths:&[String],budget:usize)->Result<String,String>{
    if paths.is_empty()||paths.len()>256{return Err("DOS probe accepts 1..256 file paths".into());}
    if !(1..=MAX_STEPS).contains(&budget){return Err("DOS step budget must be 1..1000000".into());}
    let mut report=format!("DOS executable probe v{}; step budget={} per program: concrete PIL interpreter plus minimal host DOS-call shim.\nSHIM PROFILE version6.22, OEM FF, serial0, no ROM; PSP:0040 supplies AH30 and AH33/06 version; default drive A: (empty, read-only; images and host folders become drives only in --dos-session); country 001, code page 437; AH67 moves the handle table to an AH48 block when it grows; AH52 list of lists at 0070:0026 (first MCB, NUL device; no drives, DPB, SFT or CDS); AH34 InDOS at 0070:0001; INT1A/AH00 advances deterministic tick counter; PSP:002C environment with COMSPEC and the program path; INT21/AH3D validates DS:DX and returns file-not-found except for the program's own file, which opens read-only for AH3F/AH42; INT21/AH3E tracks open PSP handles 0..4; INT21/AH40 writes CON handles 0..2; INT21/AH31 ends the run as a TSR exit; date 1980-01-01 (no clock driver); first-fit allocation only; FFh read-only bus C000-FFFF with PC/AT model byte FCh and BIOS date 01/10/84 at F000:FFF5; HLT with IF=1 idles one timer tick; no LFN API, extended memory or UMBs; INT2F installation checks find no PRINT, ASSIGN, APPEND, redirector, Windows, DPMI, ANSI.SYS or XMS; CGA-class INT10 (no EGA/VGA AH12, no VBE). Not a running DOS kernel.\n",env!("CARGO_PKG_VERSION"),budget);
    let mut inputs:Vec<(String,Result<Vec<u8>,String>)>=Vec::new();
    let mut fingerprints:BTreeMap<u64,Vec<Vec<u8>>>=BTreeMap::new();
    for path in paths{
        if let Some((container,inner))=super::containers::split_container_path(path){
            match super::containers::read_entry(Path::new(container),inner){
                Ok(bytes)=>inputs.push((path.clone(),Ok(bytes))),
                Err(e)=>inputs.push((path.clone(),Err(e))),
            }continue;
        }
        match super::containers::open(Path::new(path)){
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
        if let Some(paragraphs)=result.loaded.resident{writeln!(report,"RESIDENT INT21/AH31 kept {paragraphs} paragraphs; no resident code runs after this probe stop").unwrap();}
        writeln!(report,"completed={} PIL={} DOS_calls={} EAX={:08x} ESP={:08x}",result.completed,result.pil_steps,result.dos_calls,result.loaded.cpu.regs[0],result.loaded.cpu.regs[4]).unwrap();
        writeln!(report,"OUTPUT (escaped bytes): {}",result.output.iter().map(|b|std::ascii::escape_default(*b).map(char::from).collect::<String>()).collect::<String>()).unwrap();
    }
    writeln!(report,"\nSUMMARY imported={imported}/{} exited={exited}; first missing-encoding blockers (program count):",total).unwrap();
    if blockers.is_empty(){report.push_str("none encountered on executed paths\n");}
    for(k,n)in ranked(&blockers){writeln!(report,"{n} {k}").unwrap();}
    report.push_str("Reached instruction mix (retired instructions; REP elements and DOS bridges listed separately):\n");
    for(k,n)in ranked(&mix).into_iter().take(20){writeln!(report,"{n} {k}").unwrap();}
    report.push_str("Budget counts execution steps (including REP elements);completed counts fully retired instructions.\nStops at the first blocker per program; no skipping unknown bytes or static scanning of data. This is not whole-program coverage or a full DOS/BIOS implementation.\n");
    Ok(report)
}
#[cfg(test)]
#[path="../tests/unit/dos.rs"]
mod tests;
