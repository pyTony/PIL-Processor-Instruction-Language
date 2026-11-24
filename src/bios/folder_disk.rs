//! A host folder as a fixed disk for a booted machine (`--hd-folder DIR`).
//!
//! A booted operating system reads sectors, not host files, so the folder
//! is laid out as a real disk: a master boot record with one primary FAT16
//! partition (type 04h below 32 MiB, else 06h) from cylinder 0, head 1,
//! holding the folder's files and subdirectories under 8.3 names (the
//! names DOS lists for them in Dine's `--mount`, BASIS~N aliases for long
//! ones). The image is a temporary file behind a fixed disk like
//! `--hd-image`, so INT 13h and the AT controller read and write it as any
//! other disk; nothing on the host changes while the machine runs.
//!
//! When the run ends, `write_back` reads the FAT volume back and copies to
//! the host folder each file the guest created or changed, and creates its
//! new directories (new names in lower case, as Linux's msdos file system
//! shows them). A host file that itself changed during the run is left as
//! it is and reported. Nothing on the host is deleted or renamed: a file
//! the guest deleted stays, and a renamed one appears under its new name
//! next to the old one. The guest must have written its buffers out first
//! (`sync` or `umount` under Linux).
use std::collections::{BTreeMap,BTreeSet};
use std::path::{Path,PathBuf};

const SECTOR:usize=512;
/// 16 heads, 63 sectors per track: the fixed-disk geometry `bios_disk` uses.
const CYLINDER:usize=16*63;
/// The partition starts on the second track, as DOS FDISK placed it.
const START:usize=63;
const ROOT_ENTRIES:usize=512;
/// Limits that `containers::fat16_tree` reads back.
const MAX_FILE:usize=crate::containers::MAX_ENTRY;
const MAX_TOTAL:usize=128*1024*1024;
const MAX_ENTRIES:usize=8000;
const MAX_DEPTH:usize=16;

type Name=[u8;11];

struct Node{name:Name,host:PathBuf,time:u16,date:u16,kind:Kind}
enum Kind{File(Vec<u8>),Dir(Vec<Node>)}

/// What the folder held when the disk was made: each file's DOS path
/// (`SUB/FILE.TXT`) with its host path, length and CRC-32, and each
/// directory's DOS path with its host path.
#[derive(Debug,Default,Clone)]
pub(crate) struct Snapshot{files:BTreeMap<String,(PathBuf,usize,u32)>,dirs:BTreeMap<String,PathBuf>}

/// The disk image of a folder and what it was made from.
#[derive(Debug)]
pub(crate) struct Built{pub(crate) image:Vec<u8>,pub(crate) snapshot:Snapshot,pub(crate) notes:Vec<String>}

fn fcb_name(n:&Name)->String{
    let base=String::from_utf8_lossy(&n[..8]).trim_end().to_string();
    let ext=String::from_utf8_lossy(&n[8..]).trim_end().to_string();
    if ext.is_empty(){base}else{format!("{base}.{ext}")}
}
fn join(prefix:&str,name:&str)->String{if prefix.is_empty(){name.to_string()}else{format!("{prefix}/{name}")}}

/// One host directory's entries with their 8.3 names, sorted by host name;
/// valid 8.3 names first keep their own, the rest get BASIS~N aliases.
fn read_dir(dir:&Path,depth:usize,count:&mut usize,total:&mut usize,notes:&mut Vec<String>)->Result<Vec<Node>,String>{
    if depth>MAX_DEPTH{return Err(format!("{}: more than {MAX_DEPTH} directory levels",dir.display()));}
    let mut hosts=Vec::new();
    for item in std::fs::read_dir(dir).map_err(|e|format!("{}: {e}",dir.display()))?{
        let item=item.map_err(|e|format!("{}: {e}",dir.display()))?;
        let meta=std::fs::symlink_metadata(item.path()).map_err(|e|format!("{}: {e}",item.path().display()))?;
        if !(meta.is_file()||meta.is_dir()){notes.push(format!("left out (not a file or directory): {}",item.path().display()));continue;}
        let Some(name)=item.file_name().to_str().map(str::to_owned)else{notes.push(format!("left out (name not UTF-8): {}",item.path().display()));continue};
        hosts.push((name,item.path(),meta));
    }
    hosts.sort_by(|a,b|a.0.cmp(&b.0));
    let mut taken=BTreeSet::new();
    let mut names:Vec<Option<Name>>=hosts.iter().map(|(n,..)|super::files::short(n).filter(|s|taken.insert(*s))).collect();
    for(slot,(n,..))in names.iter_mut().zip(&hosts){
        if slot.is_none(){*slot=super::files::alias(n,&taken);if let Some(s)=slot{taken.insert(*s);}}
    }
    let mut nodes=Vec::new();
    for((_,host,meta),name)in hosts.into_iter().zip(names){
        let name=name.ok_or_else(||format!("{}: no free 8.3 name",host.display()))?;
        *count+=1;if *count>MAX_ENTRIES{return Err(format!("the folder has more than {MAX_ENTRIES} files and directories"));}
        let(time,date)=super::files::stamp(meta.modified().ok());
        let kind=if meta.is_dir(){Kind::Dir(read_dir(&host,depth+1,count,total,notes)?)}else{
            if meta.len()>MAX_FILE as u64{return Err(format!("{}: larger than {} MiB",host.display(),MAX_FILE>>20));}
            let bytes=std::fs::read(&host).map_err(|e|format!("{}: {e}",host.display()))?;
            *total+=bytes.len();if *total>MAX_TOTAL{return Err(format!("the folder holds more than {} MiB",MAX_TOTAL>>20));}
            Kind::File(bytes)
        };
        nodes.push(Node{name,host,time,date,kind});
    }
    Ok(nodes)
}

/// Clusters a directory or file of `bytes` takes (a directory at least one).
fn clusters(bytes:usize,unit:usize)->usize{bytes.div_ceil(unit)}
fn needed(nodes:&[Node],unit:usize)->usize{
    nodes.iter().map(|n|match &n.kind{
        Kind::File(b)=>clusters(b.len(),unit),
        Kind::Dir(c)=>clusters((c.len()+2)*32,unit).max(1)+needed(c,unit),
    }).sum()
}
fn content(nodes:&[Node])->usize{
    nodes.iter().map(|n|match &n.kind{Kind::File(b)=>b.len(),Kind::Dir(c)=>(c.len()+2)*32+content(c)}).sum()
}

/// The FAT16 layout of a partition of `sectors`: sectors per cluster, sectors
/// per FAT and the number of clusters. The smallest cluster that keeps the
/// count under 65525.
fn layout(sectors:usize)->Option<(usize,usize,usize)>{
    let root=ROOT_ENTRIES*32/SECTOR;
    [1usize,2,4,8,16,32,64].into_iter().find_map(|spc|{
        let fat=((sectors/spc+2)*2).div_ceil(SECTOR);
        let count=(sectors.checked_sub(1+2*fat+root)?)/spc;
        (4085..65525).contains(&count).then_some((spc,fat,count))
    })
}

/// Build the disk image of `dir`: at least 65 cylinders, with 16 MiB free beyond
/// the folder's contents, in whole cylinders (up to 1024).
pub(crate) fn build(dir:&Path)->Result<Built,String>{
    let meta=std::fs::metadata(dir).map_err(|e|format!("{}: {e}",dir.display()))?;
    if !meta.is_dir(){return Err(format!("{}: not a directory",dir.display()));}
    let mut notes=Vec::new();let(mut count,mut total)=(0,0);
    let root=read_dir(dir,0,&mut count,&mut total,&mut notes)?;
    if root.len()+1>ROOT_ENTRIES{return Err(format!("{}: {} entries; the root directory holds {}",dir.display(),root.len(),ROOT_ENTRIES-1));}
    // 65 cylinders (32 MiB less a little) keep the partition below 65536
    // sectors: a 16-bit sector count (type 04h), as early FAT16 readers expect.
    let want=(65*CYLINDER*SECTOR).max(content(&root)+content(&root)/8+(16<<20));
    let mut cylinders=want.div_ceil(CYLINDER*SECTOR);
    loop{
        if cylinders>1024{return Err(format!("{}: does not fit a 504 MB disk",dir.display()));}
        let part=cylinders*CYLINDER-START;
        if let Some((spc,fat,count))=layout(part){
            if needed(&root,spc*SECTOR)<=count{return Ok(write(dir,&root,cylinders,part,spc,fat,notes));}
        }
        cylinders+=cylinders/8+1;
    }
}

fn write(dir:&Path,root:&[Node],cylinders:usize,part:usize,spc:usize,fat_sectors:usize,notes:Vec<String>)->Built{
    let total=cylinders*CYLINDER;
    let mut image=vec![0u8;total*SECTOR];
    // Master boot record: INT 18h (no bootable disk) as its code, one
    // partition from cylinder 0 head 1 sector 1 to the last cylinder.
    image[0..2].copy_from_slice(&[0xcd,0x18]);
    let last=total-1;let(c,h,s)=(last/CYLINDER,last/63%16,last%63+1);let c=c.min(1023);
    let e=&mut image[446..462];
    e[1]=1;e[2]=1;e[3]=0;
    e[4]=if part<65536{0x04}else{0x06};
    e[5]=h as u8;e[6]=(s as u8)|((c>>2) as u8&0xc0);e[7]=c as u8;
    e[8..12].copy_from_slice(&(START as u32).to_le_bytes());e[12..16].copy_from_slice(&(part as u32).to_le_bytes());
    image[510]=0x55;image[511]=0xaa;
    // The partition's boot sector: a BIOS parameter block for FAT16.
    let base=START*SECTOR;let b=&mut image[base..base+SECTOR];
    b[0..3].copy_from_slice(&[0xeb,0x3c,0x90]);b[3..11].copy_from_slice(b"PIL 1.0 ");
    b[11..13].copy_from_slice(&512u16.to_le_bytes());b[13]=spc as u8;b[14..16].copy_from_slice(&1u16.to_le_bytes());
    b[16]=2;b[17..19].copy_from_slice(&(ROOT_ENTRIES as u16).to_le_bytes());
    if part<65536{b[19..21].copy_from_slice(&(part as u16).to_le_bytes());}else{b[32..36].copy_from_slice(&(part as u32).to_le_bytes());}
    b[21]=0xf8;b[22..24].copy_from_slice(&(fat_sectors as u16).to_le_bytes());
    b[24..26].copy_from_slice(&63u16.to_le_bytes());b[26..28].copy_from_slice(&16u16.to_le_bytes());
    b[28..32].copy_from_slice(&(START as u32).to_le_bytes());
    b[36]=0x80;b[38]=0x29;b[39..43].copy_from_slice(&crate::containers::crc32(dir.to_string_lossy().as_bytes()).to_le_bytes());
    b[43..54].copy_from_slice(b"PIL FOLDER ");b[54..62].copy_from_slice(b"FAT16   ");
    b[62..64].copy_from_slice(&[0xcd,0x18]);b[510]=0x55;b[511]=0xaa;
    let fat_at=base+SECTOR;let root_at=fat_at+2*fat_sectors*SECTOR;let data_at=root_at+ROOT_ENTRIES*32;
    let unit=spc*SECTOR;
    let mut w=Writer{image,fat:vec![0u16;fat_sectors*SECTOR/2],next:2,data_at,unit,snapshot:Snapshot::default()};
    w.fat[0]=0xfff8;w.fat[1]=0xffff;
    w.snapshot.dirs.insert(String::new(),dir.to_path_buf());
    let mut entries=vec![entry(b"PIL FOLDER ",0x08,0,0,0,0)];
    entries.extend(w.place(root,"",0));
    w.image[root_at..root_at+entries.len()*32].copy_from_slice(&entries.concat());
    for copy in 0..2{
        let at=fat_at+copy*fat_sectors*SECTOR;
        for(i,v)in w.fat.iter().enumerate(){w.image[at+2*i..at+2*i+2].copy_from_slice(&v.to_le_bytes());}
    }
    Built{image:w.image,snapshot:w.snapshot,notes}
}

fn entry(name:&Name,attr:u8,time:u16,date:u16,cluster:usize,size:usize)->[u8;32]{
    let mut e=[0u8;32];e[..11].copy_from_slice(name);e[11]=attr;
    e[22..24].copy_from_slice(&time.to_le_bytes());e[24..26].copy_from_slice(&date.to_le_bytes());
    e[26..28].copy_from_slice(&(cluster as u16).to_le_bytes());e[28..32].copy_from_slice(&(size as u32).to_le_bytes());
    e
}

struct Writer{image:Vec<u8>,fat:Vec<u16>,next:usize,data_at:usize,unit:usize,snapshot:Snapshot}
impl Writer{
    /// Allocate a contiguous chain for `bytes` and copy them in; the first
    /// cluster, 0 for nothing.
    fn store(&mut self,bytes:&[u8],at_least_one:bool)->usize{
        let n=clusters(bytes.len(),self.unit).max(usize::from(at_least_one));
        if n==0{return 0;}
        let first=self.next;self.next+=n;
        for c in first..first+n{self.fat[c]=if c+1<first+n{(c+1) as u16}else{0xffff};}
        let at=self.data_at+(first-2)*self.unit;self.image[at..at+bytes.len()].copy_from_slice(bytes);
        first
    }
    /// Write a directory's children; their entries, to be written by the caller.
    fn place(&mut self,nodes:&[Node],prefix:&str,parent:usize)->Vec<[u8;32]>{
        let mut out=Vec::new();
        for n in nodes{
            let path=join(prefix,&fcb_name(&n.name));
            match &n.kind{
                Kind::File(bytes)=>{
                    let first=self.store(bytes,false);
                    self.snapshot.files.insert(path,(n.host.clone(),bytes.len(),crate::containers::crc32(bytes)));
                    out.push(entry(&n.name,0x20,n.time,n.date,first,bytes.len()));
                },
                Kind::Dir(children)=>{
                    self.snapshot.dirs.insert(path.clone(),n.host.clone());
                    // Reserve the directory's clusters, then fill them in.
                    let size=clusters((children.len()+2)*32,self.unit).max(1)*self.unit;
                    let first=self.store(&vec![0;size],true);
                    let mut raw=vec![entry(b".          ",0x10,n.time,n.date,first,0),entry(b"..         ",0x10,n.time,n.date,parent,0)];
                    raw.extend(self.place(children,&path,first));
                    let at=self.data_at+(first-2)*self.unit;let bytes=raw.concat();
                    self.image[at..at+bytes.len()].copy_from_slice(&bytes);
                    out.push(entry(&n.name,0x10,n.time,n.date,first,0));
                },
            }
        }
        out
    }
}

/// A folder disk for one run: the image in a temporary file and the
/// snapshot it was made from.
pub(crate) struct FolderDisk{pub(crate) image:PathBuf,dir:PathBuf,snapshot:Snapshot}
impl FolderDisk{
    /// Build the disk of `dir` into a new temporary image file.
    pub(crate) fn create(dir:&Path)->Result<(Self,Vec<String>),String>{
        let built=build(dir)?;
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0,|d|d.as_nanos());
        let image=std::env::temp_dir().join(format!("pil-folder-{}-{stamp}.img",std::process::id()));
        use std::io::Write;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&image).map_err(|e|format!("{}: {e}",image.display()))?;
        f.write_all(&built.image).map_err(|e|format!("{}: {e}",image.display()))?;
        Ok((Self{image,dir:dir.to_path_buf(),snapshot:built.snapshot},built.notes))
    }
    /// Copy the guest's changes back to the folder (see the module notes)
    /// and remove the image; on an error the image stays and is named.
    pub(crate) fn finish(self)->Result<String,String>{
        let bytes=std::fs::read(&self.image).map_err(|e|format!("{}: {e}",self.image.display()))?;
        match write_back(&bytes,&self.snapshot){
            Ok(report)=>{let _=std::fs::remove_file(&self.image);Ok(format!("{}: {report}",self.dir.display()))},
            Err(e)=>Err(format!("{}: {e}; the disk image stays at {}",self.dir.display(),self.image.display())),
        }
    }
}

/// Copy new and changed files of the FAT volume in `image` back to the
/// folder `snapshot` was taken of. Returns a summary of what was done.
pub(crate) fn write_back(image:&[u8],snapshot:&Snapshot)->Result<String,String>{
    use crate::containers::FatNode;
    let tree=crate::containers::fat16_tree(image).map_err(|e|format!("the folder disk's FAT volume cannot be read back ({e})"))?;
    // Every directory and file of the volume by DOS path, parents first.
    let mut dirs=vec![(String::new(),0usize)];let mut files=Vec::new();let mut i=0;
    while i<dirs.len(){
        let(prefix,index)=dirs[i].clone();i+=1;
        for e in tree.dirs.get(index).into_iter().flatten(){
            let path=join(&prefix,&fcb_name(&e.name));
            match e.node{FatNode::Dir(d)=>dirs.push((path,d)),FatNode::File(f)=>files.push((path,f))}
        }
    }
    let mut hosts=snapshot.dirs.clone();
    let host_of=|hosts:&BTreeMap<String,PathBuf>,path:&str|->Option<PathBuf>{
        let(parent,name)=path.rsplit_once('/').unwrap_or(("",path));
        Some(hosts.get(parent)?.join(name.to_ascii_lowercase()))
    };
    let(mut made,mut written,mut added,mut kept)=(0,0,0,Vec::new());
    for(path,_) in dirs.iter().skip(1){
        if hosts.contains_key(path){continue;}
        let Some(host)=host_of(&hosts,path)else{continue};
        if !host.is_dir(){
            std::fs::create_dir(&host).map_err(|e|format!("{}: {e}",host.display()))?;made+=1;
        }
        hosts.insert(path.clone(),host);
    }
    let mut seen=BTreeSet::new();
    for(path,index)in &files{
        seen.insert(path.clone());
        let data=&tree.files[*index];
        let same=|bytes:&[u8],len:usize,crc:u32|bytes.len()==len&&crate::containers::crc32(bytes)==crc;
        match snapshot.files.get(path){
            Some((_,len,crc)) if same(data,*len,*crc)=>{},
            Some((host,len,crc))=>{
                let now=std::fs::read(host).ok();
                if now.as_deref().is_some_and(|b|same(b,*len,*crc)){
                    std::fs::write(host,&data[..]).map_err(|e|format!("{}: {e}",host.display()))?;written+=1;
                }else{kept.push(host.display().to_string());}
            },
            None=>{
                let Some(host)=host_of(&hosts,path)else{continue};
                if host.exists(){kept.push(host.display().to_string());continue;}
                std::fs::write(&host,&data[..]).map_err(|e|format!("{}: {e}",host.display()))?;added+=1;
            },
        }
    }
    let gone=snapshot.files.keys().filter(|p|!seen.contains(*p)).count();
    let mut report=format!("{written} changed and {added} new files copied back, {made} new directories");
    if gone>0{report+=&format!("; {gone} files deleted on the disk stay in the folder");}
    if !kept.is_empty(){report+=&format!("; changed on the host during the run, not overwritten: {}",kept.join(", "));}
    Ok(report)
}

#[cfg(test)]
#[path="../../tests/unit/dos_folder_disk.rs"]
mod tests;
