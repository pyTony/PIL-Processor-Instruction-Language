//! Read-only FAT16 and ZIP imports with bounded expansion and safe new-directory extraction.
use std::{collections::BTreeSet,fmt::Write as _,io::Read,path::{Path,PathBuf}};
pub const MAX_ENTRY:usize=16*1024*1024;
const MAX_INPUT:usize=64*1024*1024;
const MAX_TOTAL:usize=128*1024*1024;
const MAX_FILES:usize=8192;
#[derive(Clone,Debug)]
pub struct Entry{pub path:String,pub bytes:Vec<u8>}
#[derive(Debug)]
pub struct Archive{pub files:Vec<Entry>,pub notes:Vec<String>,total:usize,names:BTreeSet<String>}
impl Archive{
    fn new()->Self{Self{files:Vec::new(),notes:Vec::new(),total:0,names:BTreeSet::new()}}
    fn add(&mut self,path:String,bytes:Vec<u8>)->Result<(),String>{
        let path=safe_name(&path)?;
        if bytes.len()>MAX_ENTRY||self.total+bytes.len()>MAX_TOTAL||self.files.len()>=MAX_FILES{return Err("container expansion exceeds entry/total/file limits".into());}
        if !self.names.insert(path.to_ascii_lowercase()){return Err(format!("duplicate/case-aliased container path: {path}"));}
        self.total+=bytes.len();self.files.push(Entry{path,bytes});Ok(())
    }
}
fn u16(b:&[u8],at:usize)->Result<u16,String>{let s=b.get(at..at+2).ok_or("truncated container word")?;Ok(u16::from_le_bytes(s.try_into().unwrap()))}
fn u32(b:&[u8],at:usize)->Result<u32,String>{let s=b.get(at..at+4).ok_or("truncated container dword")?;Ok(u32::from_le_bytes(s.try_into().unwrap()))}
fn range(b:&[u8],at:usize,n:usize)->Result<&[u8],String>{b.get(at..at.checked_add(n).ok_or("container offset overflow")?).ok_or_else(||"container span outside file".into())}
fn safe_name(name:&str)->Result<String,String>{
    let s=name.replace('\\',"/");
    if s.is_empty()||s.starts_with('/')||s.len()>1024{return Err("invalid container path".into());}
    for part in s.split('/'){
        let device=part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()||part=="."||part==".."||(part.ends_with('.')||part.ends_with(' '))||part.chars().any(|c|c.is_control()||"<>:\"|?*".contains(c))||["CON","PRN","AUX","NUL"].contains(&device.as_str())||(device.len()==4&&(device.as_bytes().starts_with(b"COM")||device.as_bytes().starts_with(b"LPT"))&&matches!(device.as_bytes()[3],b'1'..=b'9')){
            return Err(format!("unsafe/unsupported container path: {name:?}"));
        }
    }Ok(s)
}
pub fn crc32(data:&[u8])->u32{let mut crc=!0u32;for &b in data{crc^=u32::from(b);for _ in 0..8{crc=(crc>>1)^if crc&1!=0{0xedb88320}else{0};}}!crc}
fn zip(data:&[u8])->Result<Archive,String>{
    if data.len()<22{return Err("truncated ZIP".into());}
    let eocd=(data.len().saturating_sub(65557)..=data.len()-22).rev().find(|&n|data[n..].starts_with(b"PK\x05\x06")&&u16(data,n+20).is_ok_and(|c|n+22+usize::from(c)==data.len())).ok_or("ZIP end directory missing")?;
    if u16(data,eocd+4)?!=0||u16(data,eocd+6)?!=0||u16(data,eocd+8)?!=u16(data,eocd+10)?{return Err("multi-disk ZIP is not supported".into());}
    let count=usize::from(u16(data,eocd+10)?);let length=u32(data,eocd+12)?as usize;let start=u32(data,eocd+16)?as usize;
    if count>=65535||count>MAX_FILES||start.checked_add(length)!=Some(eocd){return Err("ZIP64/nonstandard directory layout is not supported".into());}
    let mut at=start;let mut archive=Archive::new();
    for _ in 0..count{
        if !range(data,at,46)?.starts_with(b"PK\x01\x02"){return Err("invalid ZIP central record".into());}
        let flags=u16(data,at+8)?;let method=u16(data,at+10)?;let crc=u32(data,at+16)?;
        let compressed=u32(data,at+20)?as usize;let size=u32(data,at+24)?as usize;
        let n=usize::from(u16(data,at+28)?);let extra=usize::from(u16(data,at+30)?);let comment=usize::from(u16(data,at+32)?);
        if flags&1!=0||flags&0x40!=0{return Err("encrypted ZIP is not supported".into());}
        if u16(data,at+34)?!=0||compressed==u32::MAX as usize||size==u32::MAX as usize{return Err("ZIP64/multi-disk entry is not supported".into());}
        if (u32(data,at+38)?>>16)&0xf000==0xa000{return Err("ZIP symbolic links are not extracted".into());}
        let namebytes=range(data,at+46,n)?;
        if flags&0x800==0&&!namebytes.is_ascii(){return Err("non-ASCII legacy ZIP filenames are not supported".into());}
        let name=std::str::from_utf8(namebytes).map_err(|_|"invalid ZIP UTF8 filename")?;
        let local=u32(data,at+42)?as usize;at=at.checked_add(46+n+extra+comment).ok_or("ZIP directory overflow")?;
        if at>eocd{return Err("ZIP central directory crosses boundary".into());}
        if name.ends_with('/')||name.ends_with('\\'){safe_name(name.trim_end_matches(|c|c=='/'||c=='\\'))?;continue;}
        let name=safe_name(name)?;
        if size>MAX_ENTRY||archive.total+size>MAX_TOTAL{return Err("ZIP expansion limit exceeded".into());}
        if !range(data,local,30)?.starts_with(b"PK\x03\x04")||u16(data,local+6)?!=flags||u16(data,local+8)?!=method{return Err("ZIP local/central header mismatch".into());}
        let ln=usize::from(u16(data,local+26)?);let le=usize::from(u16(data,local+28)?);
        if range(data,local+30,ln)?!=namebytes{return Err("ZIP local/central filename mismatch".into());}
        if flags&8==0&&(u32(data,local+14)?!=crc||u32(data,local+18)?as usize!=compressed||u32(data,local+22)?as usize!=size){return Err("ZIP local/central size or CRC mismatch".into());}
        let payload=local.checked_add(30+ln+le).ok_or("ZIP local offset overflow")?;
        if payload.checked_add(compressed).is_none_or(|end|end>start){return Err("ZIP payload crosses directory boundary".into());}
        let input=range(data,payload,compressed)?;
        let bytes=match method{0=>{if compressed!=size{return Err("ZIP stored-size mismatch".into());}input.to_vec()},8=>super::inflate::decode(input,size)?,_=>return Err(format!("ZIP compression method{method} is unsupported (use stored/deflate)"))};
        if crc32(&bytes)!=crc{return Err(format!("ZIP CRC32 mismatch: {name}"));}archive.add(name,bytes)?;
    }
    if at!=eocd{return Err("ZIP central directory size mismatch".into());}Ok(archive)
}
struct Fat<'a>{data:&'a[u8],fat:&'a[u8],cluster_base:usize,unit:usize,clusters:usize,dirs:BTreeSet<usize>,archive:Archive}
impl Fat<'_>{
    fn chain(&self,mut c:usize)->Result<Vec<u8>,String>{
        let mut seen=BTreeSet::new();let mut out=Vec::new();
        while c<0xfff8{
            if c<2||c>=self.clusters+2||!seen.insert(c){return Err("invalid/looped FAT16 cluster chain".into());}
            if out.len()+self.unit>MAX_ENTRY{return Err("FAT16 chain exceeds entry limit".into());}
            out.extend_from_slice(range(self.data,self.cluster_base+(c-2)*self.unit,self.unit)?);c=usize::from(u16(self.fat,c*2)?);
        }Ok(out)
    }
    fn walk(&mut self,raw:&[u8],prefix:&str,depth:usize)->Result<(),String>{
        if depth>32{return Err("FAT16 directory depth limit exceeded".into());}
        for e in raw.chunks_exact(32){
            if e[0]==0{break;}if e[0]==0xe5||e[11]==0x0f||e[11]&8!=0{continue;}
            if !e[..11].is_ascii(){return Err("non-ASCII FAT short names are not supported".into());}
            let base=std::str::from_utf8(&e[..8]).unwrap().trim_end();let ext=std::str::from_utf8(&e[8..11]).unwrap().trim_end();
            if base=="."||base==".."{continue;}
            let name=if ext.is_empty(){base.into()}else{format!("{base}.{ext}")};let path=safe_name(&format!("{prefix}{name}"))?;
            let cluster=usize::from(u16(e,26)?);let size=u32(e,28)?as usize;
            if e[11]&16!=0{
                if self.dirs.len()>=MAX_FILES||!self.dirs.insert(cluster){return Err("FAT16 directory loop/count limit".into());}
                let contents=self.chain(cluster)?;self.walk(&contents,&format!("{path}/"),depth+1)?;
            }else{
                if size>MAX_ENTRY{return Err("FAT16 file exceeds entry limit".into());}
                let mut bytes=if cluster==0&&size==0{Vec::new()}else{self.chain(cluster)?};
                if bytes.len()<size{return Err("FAT16 chain shorter than directory size".into());}bytes.truncate(size);self.archive.add(path,bytes)?;
            }
        }Ok(())
    }
}
fn fat16(data:&[u8])->Result<Archive,String>{
    if range(data,510,2)?!=b"\x55\xaa"{return Err("image has no DOS boot signature".into());}
    let mut base=0;let mut partition_end=data.len();
    // A valid FAT16 superfloppy BPB has no partition table requirement.
    if u16(data,11)?!=512||u16(data,17)?==0||u16(data,22)?==0{
        let mut found=None;
        for i in 0..4{let e=range(data,446+i*16,16)?;if matches!(e[4],4|6|0x0e){let start=(u32(e,8)?as usize).checked_mul(512).ok_or("partition start overflow")?;let bytes=(u32(e,12)?as usize).checked_mul(512).ok_or("partition size overflow")?;found=Some((start,start.checked_add(bytes).ok_or("partition overflow")?));break;}}
        let(s,end)=found.ok_or("no supported FAT16 partition;FAT12/FAT32/ISO are not implemented")?;base=s;partition_end=end;
        if partition_end>data.len(){return Err("FAT16 partition extends beyond image".into());}
    }
    let b=range(data,base,512)?;if b[510..512]!=[0x55,0xaa]||u16(b,11)?!=512{return Err("invalid FAT16 boot sector".into());}
    let unit=usize::from(b[13])*512;let reserved=usize::from(u16(b,14)?);let nf=usize::from(b[16]);let roots=usize::from(u16(b,17)?);let spf=usize::from(u16(b,22)?);
    let total=if u16(b,19)?==0{u32(b,32)?as usize}else{usize::from(u16(b,19)?)};
    if unit==0||!unit.is_power_of_two()||unit>65536||reserved==0||!(1..=2).contains(&nf)||roots==0||spf==0{return Err("invalid/unsupported FAT16 BPB".into());}
    let volume_end=base.checked_add(total.checked_mul(512).ok_or("FAT size overflow")?).ok_or("FAT volume overflow")?;
    if volume_end>partition_end{return Err("FAT16 volume extends beyond partition".into());}
    let fat_start=base+reserved*512;let fat=range(data,fat_start,spf*512)?;
    if nf==2&&fat!=range(data,fat_start+spf*512,spf*512)?{return Err("FAT16 copies disagree".into());}
    let root_start=fat_start+nf*spf*512;let root_bytes=((roots*32+511)/512)*512;let cluster_base=root_start+root_bytes;
    let clusters=volume_end.checked_sub(cluster_base).ok_or("FAT16 metadata exceeds volume")?/unit;
    if !(4085..65525).contains(&clusters)||fat.len()/2<clusters+2{return Err("not a valid FAT16 cluster layout".into());}
    let mut fs=Fat{data:range(data,0,volume_end)?,fat,cluster_base,unit,clusters,dirs:BTreeSet::new(),archive:Archive::new()};
    fs.walk(range(data,root_start,roots*32)?,"",0)?;fs.archive.notes.push(format!("FAT16 volume atLBA{};{} files before package expansion",base/512,fs.archive.files.len()));Ok(fs.archive)
}
pub fn parse(data:&[u8],is_zip:bool)->Result<Archive,String>{
    if data.len()>MAX_INPUT{return Err("container exceeds64MiB input limit".into());}
    let mut archive=if is_zip{zip(data)?}else{fat16(data)?};
    // Exactly one nested package level. Keep original ZIP bytes as well.
    let originals=archive.files.len();
    for i in 0..originals{if !archive.files[i].path.to_ascii_lowercase().ends_with(".zip"){continue;}
        match zip(&archive.files[i].bytes){
            Ok(nested)=>{let prefix=format!("{}.contents/",archive.files[i].path);for e in nested.files{archive.add(format!("{prefix}{}",e.path),e.bytes)?;}},
            Err(e)=>archive.notes.push(format!("ZIP package not expanded:{}:{e}",archive.files[i].path)),
        }
    }Ok(archive)
}
pub fn open(path:&Path)->Result<Option<Archive>,String>{
    let ext=path.extension().and_then(|x|x.to_str()).unwrap_or("").to_ascii_lowercase();
    if !["img","ima","zip"].contains(&ext.as_str()){return Ok(None);}
    let file=std::fs::File::open(path).map_err(|e|e.to_string())?;let mut data=Vec::new();file.take((MAX_INPUT+1)as u64).read_to_end(&mut data).map_err(|e|e.to_string())?;
    parse(&data,ext=="zip").map(Some)
}
pub fn executable(path:&str)->bool{let lower=path.to_ascii_lowercase();lower.ends_with(".com")||lower.ends_with(".exe")}
pub fn split_container_path(path:&str)->Option<(&str,&str)>{
    let mut parts=path.splitn(2,"::");
    let container=parts.next()?;let inner=parts.next()?;
    if inner.is_empty(){return None;}Some((container,inner))
}
pub fn read_entry(container:&Path,inner_path:&str)->Result<Vec<u8>,String>{
    let a=open(container)?.ok_or("use .img/.ima FAT16 image or .zip archive")?;
    let target=safe_name(inner_path)?;
    for e in a.files{
        if e.path.eq_ignore_ascii_case(&target){return Ok(e.bytes);}
    }
    Err(format!("file not found in container: {inner_path}"))
}
pub fn listing(path:&Path)->Result<String,String>{listing_filtered(path,true)}
pub fn listing_all(path:&Path)->Result<String,String>{listing_filtered(path,false)}
fn listing_filtered(path:&Path,exec_only:bool)->Result<String,String>{
    let a=open(path)?.ok_or("use .img/.ima FAT16 image or .zip archive")?;let mut s=String::new();
    for n in &a.notes{writeln!(s,"NOTE {n}").unwrap();}
    writeln!(s,"CONTAINER {}: {} files, {} executable candidates",path.display(),a.files.len(),a.files.iter().filter(|e|executable(&e.path)).count()).unwrap();
    for e in &a.files{if !exec_only||executable(&e.path){writeln!(s,"{:>9} {}",e.bytes.len(),e.path).unwrap();}}Ok(s)
}
pub fn extract(path:&Path,destination:&Path)->Result<String,String>{
    let a=open(path)?.ok_or("use .img/.ima FAT16 image or .zip archive")?;
    // Validate file/directory prefix conflicts before creating anything.
    for e in &a.files{let mut p=PathBuf::from(&e.path);while p.pop(){if a.names.contains(&p.to_string_lossy().replace('\\',"/").to_ascii_lowercase()){return Err("container file/directory conflict".into());}}}
    std::fs::create_dir(destination).map_err(|e|format!("extraction requires a new destination directory:{e}"))?;
    for e in &a.files{let target=destination.join(&e.path);if let Some(parent)=target.parent(){std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
        use std::io::Write;let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&target).map_err(|e|e.to_string())?;file.write_all(&e.bytes).map_err(|e|e.to_string())?;}
    let mut s=format!("Extracted {} files to {} (original paths;ZIP packages under .zip.contents)\n",a.files.len(),destination.display());for n in a.notes{writeln!(s,"NOTE {n}").unwrap();}Ok(s)
}
#[cfg(test)]
#[path="../tests/unit/containers.rs"]
mod tests;
