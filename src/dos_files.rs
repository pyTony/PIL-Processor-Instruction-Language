//! Dine's drives: a FAT16 image, read-only, and a host folder, which DOS can
//! write: create, write, delete, rename, make and remove directories reach
//! the host folder itself. C: is the first of them; with neither, A: is an
//! empty drive. Host names that are
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
    files:Vec<OpenFile>,searches:Vec<Search>,next_search:usize,
}
#[derive(Clone,Debug)]
/// An open file by its system file number (the JFT entry), which handles
/// duplicated with AH45/46 share with their position.
/// `data` is the whole file; a host file (`host`) opened for writing is
/// written through at each AH40. `mode` is the AH3D access mode: 0 read, 1
/// write, 2 both. `stamp` is the time and date an image file shows.
struct OpenFile{sft:u8,data:Arc<Vec<u8>>,pos:u32,drive:u8,host:Option<PathBuf>,mode:u8,written:bool,stamp:(u16,u16)}
/// An AH4E search: drive, directory and the host name last found there, so
/// AH4F goes on after it when files were deleted or made in between.
#[derive(Clone,Debug)]
struct Search{drive:usize,path:Vec<Name>,last:Option<std::ffi::OsString>}
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
    /// The first drive's letter, which Dine reports as the boot drive.
    pub fn first_letter(&self)->u8{self.list[0].letter}
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
pub(super) fn asciiz(p:&Loaded,offset:u32)->Result<Vec<u8>,String>{
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
fn store_ds(p:&mut Loaded,offset:u32,bytes:&[u8])->Result<(),String>{store(p,ds(),offset,bytes)}
fn store(p:&mut Loaded,segment:SegmentRegister,offset:u32,bytes:&[u8])->Result<(),String>{
    if offset as usize+bytes.len()>0x10000{return Err("DOS buffer exceeds segment end".into());}
    let context=MemoryContext{segment,cache:p.image.segments[segment.index()]};
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
/// The host time of a DOS packed time and date, the inverse of `stamp`.
pub(super) fn unstamp(time:u16,date:u16)->SystemTime{
    let(year,month,day)=(i64::from(date>>9)+1980,i64::from(date>>5&15).clamp(1,12),i64::from(date&31).max(1));
    // A civil date to days since 1970-01-01 (H. Hinnant's algorithm).
    let y=year-i64::from(month<=2);let era=y/400;let yoe=y-era*400;
    let doy=(153*(if month>2{month-3}else{month+9})+2)/5+day-1;let doe=yoe*365+yoe/4-yoe/100+doy;
    let days=era*146_097+doe-719_468;
    let secs=u64::from(time>>11)*3600+u64::from(time>>5&63)*60+u64::from(time&31)*2;
    UNIX_EPOCH+std::time::Duration::from_secs(days as u64*86400+secs)
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
    let Some(Search{drive,path,last})=p.drive.searches.get(slot).cloned().filter(|s|p.drive.list.get(s.drive).is_some_and(|d|d.letter-b'A'+1==state[0]))else{fail(p,0x12);return Ok(())};
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
    // A host directory may have changed since the last find: go on after
    // the host name found last, as a FAT slot would stay where it was.
    let host_name=|e:&Entry|match &e.node{Node::Host(path)=>path.file_name().map(|n|n.to_os_string()),Node::Image(_)=>None};
    let next=match &last{Some(last)=>entries.iter().position(|e|host_name(e).is_some_and(|n|n>*last)).unwrap_or(entries.len()),None=>next};
    for(i,entry)in entries.iter().enumerate().skip(next){
        // Hidden, system and directory entries need their bit in the search
        // attribute; the volume label is found only by a label search.
        if entry.attr&0x16&!attr!=0||!matches(&pattern,&entry.name){continue;}
        p.drive.searches[slot].last=host_name(entry);
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
    let search=Search{drive,path:parent.to_vec(),last:None};
    if slot<p.drive.searches.len(){p.drive.searches[slot]=search;}else{p.drive.searches.push(search);}
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
    let(data,drive,host,stamp)=if own{
        // The program's own file opens by its name from any directory.
        if mode!=0{return Err("opening the program file for writing is not modelled".into());}
        if p.drive.files.iter().any(|f|Arc::ptr_eq(&f.data,&p.own_file)){return Err("a second open of the program file is not modelled".into());}
        (p.own_file.clone(),p.drive.letter(),None,(0,0x21))
    }else{
        if raw.is_empty(){fail(p,3);return Ok(());}
        match lookup(&p.drive,&raw){
            Err(code)=>{fail(p,code);return Ok(());},
            Ok((_,entry)) if entry.attr&0x18!=0=>{fail(p,5);return Ok(());},
            // Writing needs a host file without the read-only attribute.
            Ok((_,Entry{node:Node::Image(_),..})) if mode!=0=>{fail(p,5);return Ok(());},
            Ok((_,entry)) if mode!=0&&entry.attr&1!=0=>{fail(p,5);return Ok(());},
            Ok((drive,entry))=>{
                let host=match &entry.node{Node::Host(path)=>Some(path.clone()),Node::Image(_)=>None};
                (read_entry(&p.drive.list[drive],&entry)?,p.drive.list[drive].letter,host,(entry.time,entry.date))
            },
        }
    };
    install(p,OpenFile{sft:0,data,pos:0,drive,host,mode,written:false,stamp})
}
/// Gives an open file the lowest free handle and system file number, and
/// returns the handle in AX; error 4 when either runs out.
fn install(p:&mut Loaded,mut file:OpenFile)->Result<(),String>{
    let limit=p.ram.load(u32::from(p.psp)*16+0x32,MemWidth::Word).map_err(|e|format!("DOS JFT size read: {e:?}"))?.min(20)as u16;
    // FreeCOM redirects by closing handle 0 or 1 and opening: the file takes it.
    let Some(handle)=(0..limit).find(|&h|p.open_handles&(1<<h)==0)else{fail(p,4);return Ok(())};
    // System file numbers 0..4 are the devices; files take the lowest free one above.
    let Some(sft)=(5..=0xfeu8).find(|n|!p.drive.files.iter().any(|f|f.sft==*n))else{fail(p,4);return Ok(())};
    p.ram.store(jft(p,handle)?,MemWidth::Byte,u32::from(sft)).map_err(|e|format!("DOS open JFT write: {e:?}"))?;
    file.sft=sft;p.open_handles|=1<<handle;p.drive.files.push(file);
    set_ax(p,u32::from(handle));ok(p);Ok(())
}
/// Where a new entry would go: drive, host directory, its 8.3 name and the
/// entry already there. Error 3 for a missing directory or a name DOS cannot
/// hold, 5 on a drive DOS cannot write.
fn place(d:&Drives,raw:&[u8])->Result<(usize,PathBuf,Name,Option<Entry>),u16>{
    let(drive,path)=components(d,raw)?;
    let Some((last,parent))=path.split_last()else{return Err(3)};
    if last[0]==b' '||last[0]==b'.'||!last.iter().all(|&c|c==b' '||valid(c)){return Err(3);}
    let(dir,..)=dir_of(&d.list[drive],parent)?;
    let Dir::Host(folder)=dir else{return Err(5)};
    let existing=host_listing(&folder)?.into_iter().find(|e|&e.name==last);
    Ok((drive,folder,*last,existing))
}
/// The host file name DOS makes for an 8.3 name: `NAME.EXT` in upper case.
fn host_name(name:&Name)->String{display(name).into_iter().map(char::from).collect()}
/// AH3C creates or truncates, AH5B creates only a new file (error 50h if it
/// exists), AH5A makes a new file with a unique name it appends to the path.
/// The attribute in CX is not kept; the handle reads and writes.
fn create(p:&mut Loaded,ah:u8)->Result<(),String>{
    let dx=p.cpu.regs[2]&0xffff;
    let mut raw=asciiz(p,dx)?;
    if raw.is_empty(){fail(p,3);return Ok(());}
    if ah==0x5a{
        if !matches!(raw.last(),Some(b'\\'|b'/'|b':')){raw.push(b'\\');}
        let(drive,path)=match components(&p.drive,&raw){Ok(found)=>found,Err(code)=>{fail(p,code);return Ok(())}};
        let folder=match dir_of(&p.drive.list[drive],&path){Ok((Dir::Host(folder),..))=>folder,Ok(_)=>{fail(p,5);return Ok(())},Err(code)=>{fail(p,code);return Ok(())}};
        let taken:std::collections::BTreeSet<Name>=match host_listing(&folder){Ok(list)=>list.iter().map(|e|e.name).collect(),Err(code)=>{fail(p,code);return Ok(())}};
        let seed=SystemTime::now().duration_since(UNIX_EPOCH).map_or(0,|d|d.subsec_nanos()^d.as_secs()as u32);
        let Some(name)=(0..0x10000u32).map(|n|fcb(format!("{:08X}",seed.wrapping_add(n)).as_bytes())).find(|n|!taken.contains(n))else{fail(p,5);return Ok(())};
        raw.extend(display(&name));
        let mut stored=raw.clone();stored.push(0);store_ds(p,dx,&stored)?;
    }
    let(drive,folder,name,existing)=match place(&p.drive,&raw){Ok(found)=>found,Err(code)=>{fail(p,code);return Ok(())}};
    let path=match existing{
        Some(_) if ah!=0x3c=>{fail(p,0x50);return Ok(());},
        Some(entry) if entry.attr&0x19!=0=>{fail(p,5);return Ok(());},
        Some(Entry{node:Node::Host(path),..})=>path,
        _=>folder.join(host_name(&name)),
    };
    if std::fs::File::create(&path).is_err(){fail(p,5);return Ok(());}
    let letter=p.drive.list[drive].letter;
    install(p,OpenFile{sft:0,data:Arc::new(Vec::new()),pos:0,drive:letter,host:Some(path),mode:2,written:false,stamp:(0,0x21)})
}
/// AH40 to a file: CX bytes at the file position, written through to the
/// host; CX=0 truncates or extends the file to the position. Error 5 on a
/// handle opened for reading.
pub(super) fn write(p:&mut Loaded,sft:u8,bytes:&[u8])->Result<Result<(),u16>,String>{
    use std::io::{Seek,SeekFrom,Write};
    let Some(i)=p.drive.files.iter().position(|f|f.sft==sft)else{return Ok(Err(6))};
    let file=&mut p.drive.files[i];
    let Some(host)=file.host.clone().filter(|_|file.mode!=0)else{return Ok(Err(5))};
    let pos=file.pos as usize;let end=pos+bytes.len();
    if end as u64>MAX_OPEN{return Err("DOS write past 16 MiB in a host file is not modelled".into());}
    let mut out=std::fs::OpenOptions::new().write(true).open(&host).map_err(|e|format!("DOS write to host file: {e}"))?;
    if bytes.is_empty(){out.set_len(pos as u64)}else{out.seek(SeekFrom::Start(pos as u64)).and_then(|_|out.write_all(bytes))}.map_err(|e|format!("DOS write to host file: {e}"))?;
    let data=Arc::make_mut(&mut file.data);
    if bytes.is_empty(){data.resize(pos,0);}else{if data.len()<end{data.resize(end,0);}data[pos..end].copy_from_slice(bytes);}
    file.pos=end as u32;file.written=true;Ok(Ok(()))
}
/// The open file standard output (handle 1) refers to, when it is a file.
pub(super) fn redirected_output(p:&Loaded)->Result<Option<u8>,String>{
    Ok(system_file(p,1)?.filter(|&sft|sft>=5&&p.drive.files.iter().any(|f|f.sft==sft)))
}
/// AH41 deletes a file, AH56 renames a file or directory (DS:DX to ES:DI on
/// the same drive), AH39 makes and AH3A removes an empty directory.
fn change(p:&mut Loaded,ah:u8)->Result<(),String>{
    let raw=asciiz(p,p.cpu.regs[2]&0xffff)?;
    if raw.is_empty(){fail(p,3);return Ok(());}
    let host=|e:&Entry|match &e.node{Node::Host(path)=>Some(path.clone()),Node::Image(_)=>None};
    let result=match ah{
        0x39=>place(&p.drive,&raw).and_then(|(_,folder,name,existing)|{
            if existing.is_some(){return Err(5);}
            std::fs::create_dir(folder.join(host_name(&name))).map_err(|_|5)
        }),
        0x3a=>lookup(&p.drive,&raw).and_then(|(drive,entry)|{
            let path=host(&entry).filter(|_|entry.attr&0x10!=0).ok_or(5u16)?;
            // The current directory of a drive cannot go: error 10h.
            let(_,names)=components(&p.drive,&raw)?;
            if p.drive.list[drive].cwd.starts_with(&names){return Err(0x10);}
            std::fs::remove_dir(path).map_err(|_|5)
        }),
        0x41=>lookup(&p.drive,&raw).and_then(|(_,entry)|{
            let path=host(&entry).filter(|_|entry.attr&0x19==0).ok_or(5u16)?;
            std::fs::remove_file(path).map_err(|_|5)
        }),
        _=>{
            let es=SegmentRegister::Es;
            let saved=p.image.segments[ds().index()];p.image.segments[ds().index()]=p.image.segments[es.index()];
            let to=asciiz(p,p.cpu.regs[7]&0xffff);p.image.segments[ds().index()]=saved;
            let to=to?;
            lookup(&p.drive,&raw).and_then(|(drive,entry)|{
                let from=host(&entry).filter(|_|entry.attr&0x0d==0).ok_or(5u16)?;
                let(to_drive,folder,name,existing)=place(&p.drive,&to)?;
                if to_drive!=drive{return Err(0x11);}
                if existing.is_some(){return Err(5);}
                std::fs::rename(from,folder.join(host_name(&name))).map_err(|_|5)
            })
        },
    };
    match result{Ok(())=>ok(p),Err(code)=>fail(p,code)}
    Ok(())
}
fn open_file(p:&mut Loaded,service:&str)->Result<Option<usize>,String>{
    let handle=(p.cpu.regs[3]&0xffff)as u16;
    let Some(sft)=system_file(p,handle)? else{fail(p,6);return Ok(None)};
    if let Some(i)=p.drive.files.iter().position(|f|f.sft==sft){return Ok(Some(i));}
    if sft<5{return Err(format!("DOS {service} handle {handle} (device {sft}) is not modelled"));}
    fail(p,6);Ok(None)
}
/// The file an EXEC path names and its full path (`C:\DIR\NAME.EXT`), or a
/// DOS error: 2 or 3 when it is missing, 5 for a directory or label.
pub(super) fn program(p:&Loaded,raw:&[u8])->Result<Result<(Arc<Vec<u8>>,Vec<u8>),u16>,String>{
    let(drive,path)=match components(&p.drive,raw){Ok(found)=>found,Err(code)=>return Ok(Err(code))};
    let entry=match lookup(&p.drive,raw){Ok((_,entry))=>entry,Err(code)=>return Ok(Err(code))};
    if entry.attr&0x18!=0{return Ok(Err(5));}
    let mut full=vec![p.drive.list[drive].letter,b':',b'\\'];full.extend(path.iter().map(display).collect::<Vec<_>>().join(&b'\\'));
    Ok(Ok((read_entry(&p.drive.list[drive],&entry)?,full)))
}
/// The system file number behind an open handle of the running program.
pub(super) fn system_file(p:&Loaded,handle:u16)->Result<Option<u8>,String>{
    if handle>=20||p.open_handles&(1<<handle)==0{return Ok(None);}
    p.ram.load(jft(p,handle)?,MemWidth::Byte).map(|b|Some(b as u8)).map_err(|e|format!("DOS JFT read: {e:?}"))
}
/// Drops the files no handle refers to any more, in the running program or
/// in the parents waiting for it, as DOS frees an SFT entry at count zero.
pub(super) fn drop_unreferenced(p:&mut Loaded)->Result<(),String>{
    let mut live=std::collections::BTreeSet::new();
    for(psp,handles)in exec::chain(p){
        let base=u32::from(psp)*16;let word=|at|p.ram.load(at,MemWidth::Word).map_err(|e|format!("DOS JFT pointer read: {e:?}"));
        let table=word(base+0x36)?*16+word(base+0x34)?;
        for h in (0..20).filter(|h|handles&(1<<h)!=0){live.insert(p.ram.load(table+h,MemWidth::Byte).map_err(|e|format!("DOS JFT read: {e:?}"))?as u8);}
    }
    p.drive.files.retain(|f|live.contains(&f.sft));Ok(())
}
/// AH45 duplicates BX into the lowest free handle; AH46 makes CX refer to
/// BX's file, closing CX first. Both handles then share the file position.
fn duplicate(p:&mut Loaded,ah:u8)->Result<(),String>{
    let from=(p.cpu.regs[3]&0xffff)as u16;
    let Some(sft)=system_file(p,from)? else{fail(p,6);return Ok(())};
    let limit=p.ram.load(u32::from(p.psp)*16+0x32,MemWidth::Word).map_err(|e|format!("DOS JFT size read: {e:?}"))?.min(20)as u16;
    let to=if ah==0x45{
        let Some(to)=(0..limit).find(|&h|p.open_handles&(1<<h)==0)else{fail(p,4);return Ok(())};to
    }else{
        let to=(p.cpu.regs[1]&0xffff)as u16;
        if to>=limit{fail(p,6);return Ok(());}
        to
    };
    p.ram.store(jft(p,to)?,MemWidth::Byte,u32::from(sft)).map_err(|e|format!("DOS duplicate JFT write: {e:?}"))?;
    p.open_handles|=1<<to;drop_unreferenced(p)?;
    if ah==0x45{set_ax(p,u32::from(to));}
    ok(p);Ok(())
}
/// AH29 parses the name at DS:SI into the FCB at ES:DI, after FreeDOS
/// FcbParseFname: AL bit 0 skips leading separators, bit 1 keeps the drive
/// byte, bits 2 and 3 keep the name and extension when none is given. AL
/// returns 0, 1 (wildcards) or FFh (no such drive); SI points past the name.
fn parse_name(p:&mut Loaded)->Result<(),String>{
    let mode=p.cpu.regs[0]as u8;
    let si=p.cpu.regs[6]&0xffff;
    let context=MemoryContext{segment:ds(),cache:p.image.segments[ds().index()]};
    let mut text=Vec::new();
    for n in 0..128u32{
        if si+n>0xffff{break;}
        let addr=context.translate(si+n,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS parse name access: {e:?}"))?;
        let b=p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS parse name read: {e:?}"))?as u8;
        text.push(b);if b==0{break;}
    }
    text.push(0);
    let at=|i:usize|text.get(i).copied().unwrap_or(0);
    let field_sep=|c:u8|c<=b' '||b"/\\\"[]<>|.:;,=+\t".contains(&c);
    let mut i=0;
    if mode&1!=0{while at(i)!=0&&b":;,=+ \t".contains(&at(i)){i+=1;}}
    while matches!(at(i),b' '|b'\t'){i+=1;}
    let es=SegmentRegister::Es;
    let fcb_at=MemoryContext{segment:es,cache:p.image.segments[es.index()]};
    let di=p.cpu.regs[7]&0xffff;
    let mut fcb=[0u8;16];
    for(n,b)in fcb.iter_mut().enumerate(){
        if di+n as u32>0xffff{return Err("DOS parse name FCB exceeds segment end".into());}
        let addr=fcb_at.translate(di+n as u32,MemWidth::Byte,false,p.cpu.eip).map_err(|e|format!("DOS parse name FCB access: {e:?}"))?;
        *b=p.ram.load(addr,MemWidth::Byte).map_err(|e|format!("DOS parse name FCB read: {e:?}"))?as u8;
    }
    let mut bad_drive=false;
    if !field_sep(at(i))&&at(i+1)==b':'{
        let drive=nls::upper(at(i)).wrapping_sub(b'A');
        bad_drive=p.drive.number(drive.wrapping_add(1)).is_none()||drive>25;
        fcb[0]=drive.wrapping_add(1);i+=2;
    }else if mode&2==0{fcb[0]=0;}
    fcb[12..16].fill(0);
    if mode&4==0{fcb[1..9].fill(b' ');}
    if mode&8==0{fcb[9..12].fill(b' ');}
    let mut wild=false;
    if at(i)==b'.'{
        fcb[1]=b'.';i+=1;
        if at(i)==b'.'{fcb[2]=b'.';i+=1;}
        mode_result(p,0);
    }else{
        let mut field=|i:&mut usize,start:usize,width:usize,fcb:&mut [u8;16]|{
            let(mut n,mut fill)=(0,b' ');
            while at(*i)!=0&&!field_sep(at(*i))&&n<width{
                if at(*i)==b'*'{wild=true;fill=b'?';*i+=1;break;}
                if at(*i)==b'?'{wild=true;}
                fcb[start+n]=nls::upper(at(*i));*i+=1;n+=1;
            }
            fcb[start+n..start+width].fill(fill);
        };
        field(&mut i,1,8,&mut fcb);
        if at(i)==b'.'{i+=1;field(&mut i,9,3,&mut fcb);}
        mode_result(p,if bad_drive{0xff}else if wild{1}else{0});
    }
    let addrs=(0..16u32).map(|n|fcb_at.translate(di+n,MemWidth::Byte,true,p.cpu.eip).map_err(|e|format!("DOS parse name FCB access: {e:?}"))).collect::<Result<Vec<_>,_>>()?;
    for &a in &addrs{p.ram.check_write(a,MemWidth::Byte).map_err(|e|format!("DOS parse name FCB write: {e:?}"))?;}
    for(a,&b)in addrs.into_iter().zip(&fcb){p.ram.store(a,MemWidth::Byte,u32::from(b)).map_err(|e|format!("DOS parse name FCB write: {e:?}"))?;}
    p.cpu.regs[6]=(p.cpu.regs[6]&0xffff0000)|((si+i as u32)&0xffff);
    Ok(())
}
fn mode_result(p:&mut Loaded,al:u8){p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(al);}
pub(super) fn service(p:&mut Loaded,ah:u8)->Result<(),String>{
    let al=p.cpu.regs[0]as u8;let dl=p.cpu.regs[2]as u8;
    match ah{
        0x0e=>{
            // DL=0 is A:. A drive that does not exist leaves the current one.
            if let Some(i)=p.drive.list.iter().position(|d|d.letter-b'A'==dl){p.drive.current=i;}
            p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(LAST_DRIVE.max(p.drive.list.last().map_or(0,|d|d.letter-b'A'+1)));
        },
        0x29=>parse_name(p)?,
        0x45|0x46=>duplicate(p,ah)?,
        0x19=>p.cpu.regs[0]=(p.cpu.regs[0]&!255)|u32::from(p.drive.letter()-b'A'),
        0x36=>{
            let Some(drive)=p.drive.number(dl)else{set_ax(p,0xffff);return Ok(())};
            // No cluster is reported free: the image is read-only, and the
            // host's free space is not read (Dine has no statvfs). An image
            // reports its own geometry; a host folder 512-byte clusters for
            // the files it holds.
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
                // Setting attributes is not modelled: access denied.
                (1,_)=>fail(p,5),
                _=>fail(p,1),
            }
        },
        0x3d=>open(p)?,
        0x3f=>{
            let Some(i)=open_file(p,"read from")? else{return Ok(())};
            if p.drive.files[i].mode==1{fail(p,5);return Ok(());}
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
                p.open_handles&=!(1u32<<handle);drop_unreferenced(p)?;ok(p);
            }else{fail(p,6);}
        },
        0x44=>{
            // AL=00 device information: CON for 0..2, AUX/PRN for 3..4, with
            // bit 6 (not at end of input) set as FreeDOS opens devices, and a
            // file's drive, bit 6 set until it is written.
            let info=match system_file(p,(p.cpu.regs[3]&0xffff)as u16)?{
                Some(0..=2)=>Some(0xc3),Some(3|4)=>Some(0xc0),
                Some(sft)=>p.drive.files.iter().find(|f|f.sft==sft).map(|f|if f.written{0}else{0x40}|u32::from(f.drive-b'A')),
                None=>None,
            };
            match info{Some(info)=>{p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|info;ok(p);},None=>fail(p,6)}
        },
        0x4e=>find_first(p)?,
        0x4f=>find_next(p)?,
        0x3c|0x5a|0x5b=>create(p,ah)?,
        0x57=>{
            // File time and date: AL=00 gets them into CX and DX, AL=01 sets
            // the host file's modification time. Image files cannot change.
            let Some(i)=open_file(p,"time and date of")? else{return Ok(())};
            let file=&p.drive.files[i];
            match al{
                0=>{
                    let(time,date)=match &file.host{Some(path)=>stamp(std::fs::metadata(path).and_then(|m|m.modified()).ok()),None=>file.stamp};
                    p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|u32::from(time);p.cpu.regs[2]=(p.cpu.regs[2]&0xffff0000)|u32::from(date);ok(p);
                },
                1=>{
                    let when=unstamp(p.cpu.regs[1]as u16,p.cpu.regs[2]as u16);
                    let set=file.host.as_ref().is_some_and(|path|std::fs::OpenOptions::new().write(true).open(path).and_then(|f|f.set_modified(when)).is_ok());
                    if set{ok(p);}else{fail(p,5);}
                },
                _=>fail(p,1),
            }
        },
        0x60=>{
            // Truename: the full path of DS:SI at ES:DI, upper case, with `.`
            // and `..` resolved and `*` as `?`, whether or not it exists.
            let raw=asciiz(p,p.cpu.regs[6]&0xffff)?;
            if raw.len()>=2&&raw[1]==b':'&&!p.drive.list.iter().any(|d|raw[0].eq_ignore_ascii_case(&d.letter)){fail(p,0x0f);return Ok(());}
            let(drive,path)=match components(&p.drive,&raw){Ok(found)=>found,Err(code)=>{fail(p,code);return Ok(())}};
            if raw.is_empty(){fail(p,3);return Ok(());}
            let mut full=vec![p.drive.list[drive].letter,b':',b'\\'];full.extend(path.iter().map(display).collect::<Vec<_>>().join(&b'\\'));full.push(0);
            if full.len()>128{fail(p,3);return Ok(());}
            store(p,SegmentRegister::Es,p.cpu.regs[7]&0xffff,&full)?;ok(p);
        },
        0x39|0x3a|0x41|0x56=>change(p,ah)?,
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
