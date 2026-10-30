//! Dine's drives: a FAT16 image and a host folder, both read-only, with C:
//! the first of them; with neither, A: is an empty drive. Host names that are
//! not valid 8.3 names get BASIS~N aliases per directory, the way a DOS 7
//! volume shows them to DOS 6; the long-name API (AH71) stays unsupported.
//! Symbolic links in a host folder are not followed.
use super::*;
use super::services::jft;
use crate::containers::{FatNode,FatTree};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime,UNIX_EPOCH};
/// Host entries one directory may show, and files AH36 walks.
const MAX_ENTRIES:usize=4096;
/// Searches kept for AH4F; the oldest slot is reused after that.
const MAX_SEARCHES:usize=64;
/// Largest host file AH3D reads into memory.
const MAX_OPEN:u64=16*1024*1024;
/// LASTDRIVE: AH0E reports drives A: to E:, as FreeDOS does by default.
const LAST_DRIVE:u8=5;
type Name=[u8;11];
/// What a drive shows: nothing, a host folder or a FAT16 image.
#[derive(Clone,Debug)]
pub enum Source{Empty,Host(PathBuf),Image(Arc<FatTree>)}
#[derive(Clone,Debug)]
struct Drive{letter:u8,source:Source,cwd:Vec<Name>}
#[derive(Clone,Debug)]
pub struct Drives{
    list:Vec<Drive>,current:usize,
    files:Vec<OpenFile>,searches:Vec<(usize,Vec<Name>)>,next_search:usize,
}
#[derive(Clone,Debug)]
struct OpenFile{handle:u16,data:Arc<Vec<u8>>,pos:u32,drive:u8}
/// A directory on some drive.
#[derive(Clone,Debug)]
enum Dir{Empty,Host(PathBuf),Image(usize)}
#[derive(Clone,Debug)]
enum Node{Host(PathBuf),Image(FatNode)}
#[derive(Clone,Debug)]
struct Entry{name:Name,attr:u8,size:u32,time:u16,date:u16,node:Node}
impl Entry{
    fn dir(&self)->Option<Dir>{
        if self.attr&0x10==0{return None;}
        match &self.node{Node::Host(path)=>Some(Dir::Host(path.clone())),Node::Image(FatNode::Dir(i))=>Some(Dir::Image(*i)),Node::Image(FatNode::File(_))=>None}
    }
}
impl Drives{
    /// Drives in letter order from C:, or an empty A: when there are none.
    pub fn new(sources:Vec<Source>)->Self{
        let list=if sources.is_empty(){vec![Drive{letter:b'A',source:Source::Empty,cwd:Vec::new()}]}
            else{sources.into_iter().zip(b'C'..).map(|(source,letter)|Drive{letter,source,cwd:Vec::new()}).collect()};
        Self{list,current:0,files:Vec::new(),searches:Vec::new(),next_search:0}
    }
    /// The default drive's letter.
    pub fn letter(&self)->u8{self.list[self.current].letter}
    /// Handles AH3D opened that are still open.
    #[allow(dead_code)]
    pub fn open_files(&self)->usize{self.files.len()}
    /// The drive with DOS number `n` (0 default, 1 A:), as a list index.
    fn number(&self,n:u8)->Option<usize>{
        if n==0{return Some(self.current);}
        self.list.iter().position(|d|d.letter-b'A'+1==n)
    }
}
fn ok(p:&mut Loaded){p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;}
fn fail(p:&mut Loaded,code:u16){p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|u32::from(code);p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;}
fn set_ax(p:&mut Loaded,ax:u32){p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|(ax&0xffff);}
fn ds()->SegmentRegister{SegmentRegister::Ds}
/// ASCIIZ path at DS:offset, at most 128 bytes before the NUL.
fn asciiz(p:&Loaded,offset:u32)->Result<Vec<u8>,String>{
    let context=MemoryContext{segment:ds(),cache:p.image.segments[ds().index()]};
    let mut path=Vec::new();
    for i in 0..128u32{
        let Some(at)=offset.checked_add(i).filter(|&a|a<=0xffff)else{return Err("DOS path exceeds segment end before NUL terminator".into());};
        let addr=context.translate(at,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS path access: {e:?}"))?;
        let b=p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS path RAM access: {e:?}"))?as u8;
        if b==0{return Ok(path);}
        path.push(b);
    }
    Err("DOS path has no NUL terminator within 128 bytes".into())
}
/// Writes `bytes` at DS:offset after checking every byte can be written.
fn store_ds(p:&mut Loaded,offset:u32,bytes:&[u8])->Result<(),String>{
    if offset as usize+bytes.len()>0x10000{return Err("DOS buffer exceeds segment end".into());}
    let context=MemoryContext{segment:ds(),cache:p.image.segments[ds().index()]};
    let mut addrs=Vec::with_capacity(bytes.len());
    for n in 0..bytes.len()as u32{
        let addr=context.translate(offset+n,MemWidth::Byte,true,p.cpu.eip).map_err(|e|format!("DOS buffer access: {e:?}"))?;
        p.ram.check_write(addr,MemWidth::Byte).map_err(|e|format!("DOS buffer RAM access: {e:?}"))?;addrs.push(addr);
    }
    for(addr,&b)in addrs.into_iter().zip(bytes){p.ram.store(addr,MemWidth::Byte,u32::from(b)).map_err(|e|format!("DOS buffer RAM access: {e:?}"))?;}
    Ok(())
}
fn dta_write(p:&mut Loaded,at:u32,bytes:&[u8])->Result<(),String>{
    let(segment,offset)=p.dta;
    if u32::from(offset)+at+bytes.len()as u32>0x10000{return Err("DOS DTA exceeds segment end".into());}
    let base=u32::from(segment)*16+u32::from(offset)+at;
    for n in 0..bytes.len()as u32{p.ram.check_write(base+n,MemWidth::Byte).map_err(|e|format!("DOS DTA write: {e:?}"))?;}
    p.ram.write_bytes(base,bytes).map_err(|e|format!("DOS DTA write: {e:?}"))
}
fn dta_read(p:&Loaded,at:u32,n:u32)->Result<Vec<u8>,String>{
    let(segment,offset)=p.dta;
    if u32::from(offset)+at+n>0x10000{return Err("DOS DTA exceeds segment end".into());}
    let base=u32::from(segment)*16+u32::from(offset)+at;
    (0..n).map(|i|p.ram.load(base+i,MemWidth::Byte).map(|b|b as u8).map_err(|e|format!("DOS DTA read: {e:?}"))).collect()
}
fn valid(c:u8)->bool{c.is_ascii_alphanumeric()||b"!#$%&'()-@^_`{}~".contains(&c)}
/// A component in the 11-byte directory form DOS compares: upper case, the name
/// cut to 8 and the extension to 3, `*` filling the rest of its field with `?`.
pub(super) fn fcb(component:&[u8])->Name{
    let mut name=[b' ';11];
    if component==b"."||component==b".."{name[..component.len()].copy_from_slice(component);return name;}
    let(base,ext)=match component.iter().position(|&c|c==b'.'){Some(i)=>(&component[..i],&component[i+1..]),None=>(component,&[][..])};
    for(field,start,width)in [(base,0,8),(ext,8,3)]{
        for(i,&c)in field.iter().take(width).enumerate(){
            if c==b'*'{name[start+i..start+width].fill(b'?');break;}
            name[start+i]=c.to_ascii_uppercase();
        }
    }
    name
}
/// `NAME.EXT` as DOS prints and returns it.
pub(super) fn display(name:&Name)->Vec<u8>{
    let mut out:Vec<u8>=name[..8].iter().copied().filter(|&c|c!=b' ').collect();
    let ext:Vec<u8>=name[8..].iter().copied().filter(|&c|c!=b' ').collect();
    if !ext.is_empty(){out.push(b'.');out.extend(ext);}
    out
}
/// Host name already a valid 8.3 name, in upper case.
pub(super) fn short(host:&str)->Option<Name>{
    let bytes=host.as_bytes();
    let(base,ext)=match bytes.iter().position(|&c|c==b'.'){Some(i)=>(&bytes[..i],&bytes[i+1..]),None=>(bytes,&[][..])};
    let fits=(1..=8).contains(&base.len())&&ext.len()<=3&&base.iter().chain(ext).all(|&c|valid(c));
    (fits&&!(bytes.ends_with(b".")&&ext.is_empty())).then(||fcb(&bytes.to_ascii_uppercase()))
}
/// BASIS~N alias: leading dots and spaces dropped, other characters DOS cannot
/// hold become `_`, the base cut to make room for ~N, the extension to 3.
pub(super) fn alias(host:&str,taken:&std::collections::BTreeSet<Name>)->Option<Name>{
    let clean=|part:&str|->Vec<u8>{part.chars().filter(|&c|c!=' '&&c!='.').map(|c|if c.is_ascii()&&valid(c as u8){c.to_ascii_uppercase()as u8}else{b'_'}).collect()};
    let trimmed=host.trim_start_matches('.');
    let(base,ext)=match trimmed.rfind('.'){Some(i)=>(&trimmed[..i],&trimmed[i+1..]),None=>(trimmed,"")};
    let(mut base,mut ext)=(clean(base),clean(ext));
    if base.is_empty(){base=b"_".to_vec();}ext.truncate(3);
    for n in 1..1_000_000u32{
        let tail=format!("~{n}");let mut name=base.clone();name.truncate(8-tail.len());name.extend(tail.bytes());
        if !ext.is_empty(){name.push(b'.');name.extend(&ext);}
        let candidate=fcb(&name);
        if !taken.contains(&candidate){return Some(candidate);}
    }
    None
}
/// DOS packed time and date of a host modification time (UTC), held to 1980..2107.
pub(super) fn stamp(modified:Option<SystemTime>)->(u16,u16){
    let secs=modified.and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map_or(0,|d|d.as_secs()).clamp(315_532_800,4_354_819_198);
    let(days,rest)=((secs/86400)as i64,secs%86400);
    // Days since 1970-01-01 to a civil date (H. Hinnant's algorithm).
    let z=days+719_468;let era=z/146_097;let doe=z-era*146_097;
    let yoe=(doe-doe/1460+doe/36_524-doe/146_096)/365;let doy=doe-(365*yoe+yoe/4-yoe/100);
    let mp=(5*doy+2)/153;let day=doy-(153*mp+2)/5+1;let month=if mp<10{mp+3}else{mp-9};
    let year=yoe+era*400+i64::from(month<=2);
    let time=(rest/3600)<<11|(rest%3600/60)<<5|(rest%60)/2;
    (time as u16,((year-1980)<<9|month<<5|day)as u16)
}
/// One host directory as DOS sees it, sorted by host name.
fn host_listing(dir:&Path)->Result<Vec<Entry>,u16>{
    let mut hosts=Vec::new();
    for item in std::fs::read_dir(dir).map_err(|_|3u16)?{
        let Ok(item)=item else{continue};
        let Ok(meta)=std::fs::symlink_metadata(item.path())else{continue};
        if !(meta.is_file()||meta.is_dir()){continue;}
        let Some(name)=item.file_name().to_str().map(str::to_owned)else{continue};
        hosts.push((name,item.path(),meta));
        if hosts.len()>16*MAX_ENTRIES{break;}
    }
    hosts.sort_by(|a,b|a.0.cmp(&b.0));hosts.truncate(MAX_ENTRIES);
    let mut taken=std::collections::BTreeSet::new();
    let mut names:Vec<Option<Name>>=hosts.iter().map(|(name,..)|short(name).filter(|n|taken.insert(*n))).collect();
    for(slot,(name,..))in names.iter_mut().zip(&hosts){
        if slot.is_none(){*slot=alias(name,&taken);if let Some(n)=slot{taken.insert(*n);}}
    }
    Ok(hosts.into_iter().zip(names).filter_map(|((_,host,meta),name)|{
        let(time,date)=stamp(meta.modified().ok());
        Some(Entry{name:name?,attr:if meta.is_dir(){0x10}else{0x20},size:if meta.is_dir(){0}else{meta.len().min(u64::from(u32::MAX))as u32},time,date,node:Node::Host(host)})
    }).collect())
}
fn listing(d:&Drive,dir:&Dir)->Result<Vec<Entry>,u16>{
    match(dir,&d.source){
        (Dir::Host(path),_)=>host_listing(path),
        (Dir::Image(i),Source::Image(tree))=>Ok(tree.dirs.get(*i).ok_or(3u16)?.iter().map(|e|Entry{name:e.name,attr:e.attr,size:e.size,time:e.time,date:e.date,node:Node::Image(e.node)}).collect()),
        _=>Ok(Vec::new()),
    }
}
/// The drive (list index) and absolute 8.3 components of a guest path;
/// `Err(3)` for a drive that does not exist or `..` above the root.
fn components(d:&Drives,raw:&[u8])->Result<(usize,Vec<Name>),u16>{
    let mut rest=raw;let mut drive=d.current;
    if rest.len()>=2&&rest[1]==b':'{
        drive=d.list.iter().position(|x|rest[0].eq_ignore_ascii_case(&x.letter)).ok_or(3u16)?;
        rest=&rest[2..];
    }
    let mut path=if rest.first().is_some_and(|&c|c==b'\\'||c==b'/'){Vec::new()}else{d.list[drive].cwd.clone()};
    for part in rest.split(|&c|c==b'\\'||c==b'/'){
        match part{b""|b"."=>{},b".."=>{if path.pop().is_none(){return Err(3);}},part=>path.push(fcb(part))}
    }
    Ok((drive,path))
}
/// The directory an absolute component list names, with its own time and
/// date (01-01-1980 for the root), or `Err(3)`.
fn dir_of(d:&Drive,path:&[Name])->Result<(Dir,u16,u16),u16>{
    let mut dir=match &d.source{Source::Empty=>Dir::Empty,Source::Host(root)=>Dir::Host(root.clone()),Source::Image(_)=>Dir::Image(0)};
    let mut stamp=(0,0x21);
    for name in path{
        let entry=listing(d,&dir)?.into_iter().find(|e|&e.name==name).ok_or(3u16)?;
        dir=entry.dir().ok_or(3u16)?;stamp=(entry.time,entry.date);
    }
    Ok((dir,stamp.0,stamp.1))
}
/// The entry a path names: Err(3) if its directory is missing, Err(2) if it is.
fn lookup(d:&Drives,raw:&[u8])->Result<(usize,Entry),u16>{
    let(drive,path)=components(d,raw)?;
    let Some((last,parent))=path.split_last()else{return Err(3)};
    if last.contains(&b'?'){return Err(2);}
    let(dir,..)=dir_of(&d.list[drive],parent)?;
    listing(&d.list[drive],&dir)?.into_iter().find(|e|&e.name==last).map(|e|(drive,e)).ok_or(2)
}
fn matches(pattern:&Name,name:&Name)->bool{pattern.iter().zip(name).all(|(&p,&c)|p==b'?'||p==c)}
/// Fills the DTA from search state and a found entry.
fn found(p:&mut Loaded,state:&[u8],next:usize,entry:&Entry,name:&[u8])->Result<(),String>{
    let mut out=[0u8;0x2b];out[..0x15].copy_from_slice(state);
    out[13..15].copy_from_slice(&(next.min(0xffff)as u16).to_le_bytes());
    out[0x15]=entry.attr;out[0x16..0x18].copy_from_slice(&entry.time.to_le_bytes());out[0x18..0x1a].copy_from_slice(&entry.date.to_le_bytes());
    out[0x1a..0x1e].copy_from_slice(&entry.size.to_le_bytes());out[0x1e..0x1e+name.len()].copy_from_slice(name);
    dta_write(p,0,&out)?;ok(p);Ok(())
}
/// DTA reserved bytes: 00 drive number, 01..0C pattern, 0C attribute, 0D next
/// entry, 0F search slot.
fn find_next(p:&mut Loaded)->Result<(),String>{
    let state=dta_read(p,0,0x15)?;
    let pattern:Name=state[1..12].try_into().unwrap();let attr=state[12];
    let next=usize::from(u16::from_le_bytes([state[13],state[14]]));let slot=usize::from(u16::from_le_bytes([state[15],state[16]]));
    let Some((drive,path))=p.drive.searches.get(slot).cloned().filter(|(drive,_)|p.drive.list.get(*drive).is_some_and(|d|d.letter-b'A'+1==state[0]))else{fail(p,0x12);return Ok(())};
    let d=&p.drive.list[drive];
    let entries=match dir_of(d,&path){
        Ok((dir,time,date))=>{
            let mut all=Vec::new();
            if !path.is_empty(){for dots in [&b"."[..],b".."]{all.push(Entry{name:fcb(dots),attr:0x10,size:0,time,date,node:Node::Host(PathBuf::new())});}}
            all.extend(listing(d,&dir).unwrap_or_default());
            all
        },
        Err(_)=>Vec::new(),
    };
    for(i,entry)in entries.iter().enumerate().skip(next){
        // Hidden, system and directory entries need their bit in the search
        // attribute; the volume label is found only by a label search.
        if entry.attr&0x16&!attr!=0||!matches(&pattern,&entry.name){continue;}
        return found(p,&state,i+1,entry,&display(&entry.name));
    }
    fail(p,0x12);Ok(())
}
fn find_first(p:&mut Loaded)->Result<(),String>{
    let raw=asciiz(p,p.cpu.regs[2]&0xffff)?;let attr=p.cpu.regs[1]as u8;
    let(drive,path)=match components(&p.drive,&raw){Ok(found)=>found,Err(code)=>{fail(p,code);return Ok(())}};
    let Some((pattern,parent))=path.split_last()else{fail(p,3);return Ok(())};
    if dir_of(&p.drive.list[drive],parent).is_err(){fail(p,3);return Ok(());}
    let mut state=[0u8;0x15];state[0]=p.drive.list[drive].letter-b'A'+1;state[1..12].copy_from_slice(pattern);state[12]=attr;
    if attr==0x08{
        // A label-only search finds the volume label, as 11 characters with
        // a dot after the eighth, like MS-DOS and FreeDOS.
        let label=match &p.drive.list[drive].source{Source::Image(tree)=>tree.label,_=>None};
        let Some(label)=label else{fail(p,0x12);return Ok(())};
        let mut name=label[..8].to_vec();if label[8..].iter().any(|&c|c!=b' '){name.push(b'.');name.extend(&label[8..]);}
        while name.last()==Some(&b' '){name.pop();}
        let entry=Entry{name:label,attr:0x08,size:0,time:0,date:0x21,node:Node::Host(PathBuf::new())};
        state[13..15].copy_from_slice(&0xffffu16.to_le_bytes());
        return found(p,&state,0xffff,&entry,&name);
    }
    let slot=p.drive.next_search;p.drive.next_search=(slot+1)%MAX_SEARCHES;
    if slot<p.drive.searches.len(){p.drive.searches[slot]=(drive,parent.to_vec());}else{p.drive.searches.push((drive,parent.to_vec()));}
    state[15..17].copy_from_slice(&(slot as u16).to_le_bytes());
    dta_write(p,0,&state)?;
    find_next(p)
}
/// Clusters (512 bytes) the mounted files fill, walking at most MAX_ENTRIES entries.
fn used_clusters(root:&Path)->u32{
    let(mut stack,mut seen,mut clusters)=(vec![root.to_path_buf()],0usize,0u64);
    while let Some(dir)=stack.pop(){
        let Ok(items)=std::fs::read_dir(&dir)else{continue};
        for item in items.flatten(){
            seen+=1;if seen>MAX_ENTRIES{return clusters.min(0xffff)as u32;}
            let Ok(meta)=std::fs::symlink_metadata(item.path())else{continue};
            if meta.is_dir(){stack.push(item.path());}else if meta.is_file(){clusters+=meta.len().div_ceil(512);}
        }
    }
    clusters.min(0xffff)as u32
}
fn read_entry(d:&Drive,entry:&Entry)->Result<Arc<Vec<u8>>,String>{
    match(&entry.node,&d.source){
        (Node::Image(FatNode::File(i)),Source::Image(tree))=>tree.files.get(*i).cloned().ok_or_else(||"DOS open of a missing image file".into()),
        (Node::Host(path),_)=>{
            let file=std::fs::File::open(path).map_err(|e|format!("DOS open of host file: {e}"))?;
            let mut data=Vec::new();file.take(MAX_OPEN+1).read_to_end(&mut data).map_err(|e|format!("DOS read of host file: {e}"))?;
            if data.len()as u64>MAX_OPEN{return Err("DOS open of a host file over 16 MiB is not modelled".into());}
            Ok(Arc::new(data))
        },
        _=>Err("DOS open of a directory entry that is not a file".into()),
    }
}
fn open(p:&mut Loaded)->Result<(),String>{
    let mode=(p.cpu.regs[0]&7)as u8;
    if mode>2{fail(p,0x0c);return Ok(());}
    let raw=asciiz(p,p.cpu.regs[2]&0xffff)?;
    let own=!raw.is_empty()&&base_name(&raw)==p.own_name;
    let(data,drive)=if own{
        // The program's own file opens by its name from any directory.
        if mode!=0{return Err("opening the program file for writing is not modelled".into());}
        if p.drive.files.iter().any(|f|Arc::ptr_eq(&f.data,&p.own_file)){return Err("a second open of the program file is not modelled".into());}
        (p.own_file.clone(),p.drive.letter())
    }else{
        if raw.is_empty(){fail(p,3);return Ok(());}
        match lookup(&p.drive,&raw){
            Err(code)=>{fail(p,code);return Ok(());},
            Ok((_,entry)) if entry.attr&0x18!=0||mode!=0=>{fail(p,5);return Ok(());},
            Ok((drive,entry))=>(read_entry(&p.drive.list[drive],&entry)?,p.drive.list[drive].letter),
        }
    };
    let limit=p.ram.load(u32::from(PSP)*16+0x32,MemWidth::Word).map_err(|e|format!("DOS JFT size read: {e:?}"))?.min(20)as u16;
    let Some(handle)=(5..limit).find(|&h|p.open_handles&(1<<h)==0)else{fail(p,4);return Ok(())};
    // The JFT entry is the system file number; Dine numbers files by handle.
    p.ram.store(jft(p,handle)?,MemWidth::Byte,u32::from(handle)).map_err(|e|format!("DOS open JFT write: {e:?}"))?;
    p.open_handles|=1<<handle;p.drive.files.push(OpenFile{handle,data,pos:0,drive});
    set_ax(p,u32::from(handle));ok(p);Ok(())
}
fn open_file(p:&mut Loaded,service:&str)->Result<Option<usize>,String>{
    let handle=(p.cpu.regs[3]&0xffff)as u16;
    if let Some(i)=p.drive.files.iter().position(|f|f.handle==handle){return Ok(Some(i));}
    if handle<20&&p.open_handles&(1<<handle)!=0{return Err(format!("DOS {service} handle {handle} is not modelled"));}
    fail(p,6);Ok(None)
}
pub(super) fn service(p:&mut Loaded,ah:u8)->Result<(),String>{
    let al=p.cpu.regs[0]as u8;let dl=p.cpu.regs[2]as u8;
    match ah{
        0x0e=>{
            // DL=0 is A:. A drive that does not exist leaves the current one.
            if let Some(i)=p.drive.list.iter().position(|d|d.letter-b'A'==dl){p.drive.current=i;}
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(LAST_DRIVE.max(p.drive.list.last().map_or(0,|d|d.letter-b'A'+1)));
        },
        0x19=>p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(p.drive.letter()-b'A'),
        0x36=>{
            let Some(drive)=p.drive.number(dl)else{set_ax(p,0xffff);return Ok(())};
            // Read-only drives: no cluster is free. An image reports its own
            // geometry; a host folder 512-byte clusters for the files it holds.
            let(per_cluster,clusters)=match &p.drive.list[drive].source{
                Source::Image(tree)=>(u32::from(tree.sectors_per_cluster),u32::from(tree.clusters)),
                Source::Host(root)=>(1,used_clusters(root)),Source::Empty=>(1,0),
            };
            set_ax(p,per_cluster);p.cpu.regs[3]&=0xffff0000;p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|512;p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|clusters;
        },
        0x47=>{
            let Some(drive)=p.drive.number(dl)else{fail(p,0x0f);return Ok(())};
            let mut text=p.drive.list[drive].cwd.iter().map(display).collect::<Vec<_>>().join(&b'\\');text.push(0);
            store_ds(p,p.cpu.regs[6]&0xffff,&text)?;set_ax(p,0x0100);ok(p);
        },
        0x3b=>{
            let raw=asciiz(p,p.cpu.regs[2]&0xffff)?;
            match components(&p.drive,&raw).and_then(|(drive,path)|dir_of(&p.drive.list[drive],&path).map(|_|(drive,path))){
                Ok((drive,path)) if !raw.is_empty()=>{p.drive.list[drive].cwd=path;ok(p);},
                _=>fail(p,3),
            }
        },
        0x43=>{
            let raw=asciiz(p,p.cpu.regs[2]&0xffff)?;
            // The program's own file answers by name, as AH3D opens it.
            let own=!raw.is_empty()&&base_name(&raw)==p.own_name;
            let found=if own{Ok(0x20)}else{lookup(&p.drive,&raw).map(|(_,e)|e.attr)};
            match(al,found){
                (0,Ok(attr))=>{p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|u32::from(attr);ok(p);},
                (0,Err(code))=>fail(p,code),
                (1,_)=>fail(p,5),
                _=>fail(p,1),
            }
        },
        0x3d=>open(p)?,
        0x3f=>{
            let Some(i)=open_file(p,"read from")? else{return Ok(())};
            let offset=p.cpu.regs[2]&0xffff;let wanted=p.cpu.regs[1]&0xffff;
            if offset+wanted>0x10000{return Err("DOS read buffer exceeds segment end".into());}
            let file=&p.drive.files[i];let start=(file.pos as usize).min(file.data.len());
            let count=(wanted as usize).min(file.data.len()-start);let bytes=file.data[start..start+count].to_vec();
            store_ds(p,offset,&bytes)?;
            p.drive.files[i].pos+=count as u32;set_ax(p,count as u32);ok(p);
        },
        0x42=>{
            let Some(i)=open_file(p,"seek on")? else{return Ok(())};
            let distance=i64::from(((p.cpu.regs[1]&0xffff)<<16|(p.cpu.regs[2]&0xffff))as i32);
            let file=&p.drive.files[i];
            let origin=match al{0=>0,1=>i64::from(file.pos),2=>file.data.len()as i64,_=>{fail(p,1);return Ok(())}};
            let target=origin+distance;
            if !(0..=i64::from(u32::MAX)).contains(&target){return Err("DOS seek before the start of the file is not modelled".into());}
            p.drive.files[i].pos=target as u32;
            set_ax(p,target as u32);p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|(target as u32>>16);ok(p);
        },
        0x3e=>{
            let handle=(p.cpu.regs[3]&0xffff)as u16;
            if handle<20&&(p.open_handles&(1u32<<handle))!=0{
                p.ram.store(jft(p,handle)?,MemWidth::Byte,0xff).map_err(|e|format!("DOS close JFT write: {e:?}"))?;
                p.open_handles&=!(1u32<<handle);p.drive.files.retain(|f|f.handle!=handle);ok(p);
            }else{fail(p,6);}
        },
        0x44=>{
            // AL=00 device information: CON for 0..2, AUX/PRN for 3..4, with
            // bit 6 (not at end of input) set as FreeDOS opens devices, and an
            // unwritten file on this drive for handles AH3D opened.
            let handle=p.cpu.regs[3]&0xffff;
            let info=if handle<20&&p.open_handles&(1<<handle)!=0{
                if handle<3{Some(0xc3)}else if handle<5{Some(0xc0)}else{p.drive.files.iter().find(|f|u32::from(f.handle)==handle).map(|f|0x40|u32::from(f.drive-b'A'))}
            }else{None};
            match info{Some(info)=>{p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|info;ok(p);},None=>fail(p,6)}
        },
        0x4e=>find_first(p)?,
        0x4f=>find_next(p)?,
        // Make or remove directory, create, delete, rename, temporary and new
        // file: the drive is read-only, so access denied.
        0x39|0x3a|0x3c|0x41|0x56|0x5a|0x5b=>fail(p,5),
        // No media ID (AX=6900h/6901h): invalid function.
        0x69=>fail(p,1),
        // No FAT32 extensions: AL=00h, CF=1, as for AH71.
        0x73=>{p.cpu.regs[0]&=!255;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;},
        _=>return Err(format!("DOS file service AH{ah:02x} has no handler")),
    }
    Ok(())
}
#[cfg(test)]
pub(super) mod probe{
    pub(in crate::dos) use super::{alias,display,fcb,short,stamp};
}
