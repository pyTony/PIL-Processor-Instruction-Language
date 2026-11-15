//! Keep the project's own DOS programs current on a drive folder.
//!
//! The configuration's `own_programs` drive (default D:) is a host folder
//! the user also fills with programs of their own, and git does not track
//! it, so a pull never reached the copies of programs/dos placed there.
//! At start, every file directly in programs/dos (next to the configuration)
//! is copied to that folder when it is missing there, and replaces a file
//! of the same name (any letter case) only when that file's SHA-256 is
//! listed for the name in programs/dos/versions.sha256: an earlier version
//! of ours. Any other file is left as it is, and nothing is ever deleted.
//! Nothing here knows which programs exist: the folder and its list decide.
use std::collections::{HashMap,HashSet};
use std::path::{Path,PathBuf};

/// The list of known versions inside the source folder.
pub(crate) const MANIFEST:&str="versions.sha256";
/// Larger files are not copied (the folder holds small DOS programs).
const LARGEST:u64=16<<20;

/// What a refresh did, by file name.
#[derive(Debug,Default,PartialEq,Eq)]
pub(crate) struct Report{
    /// Missing on the drive, copied.
    pub(crate) added:Vec<String>,
    /// An earlier version of ours, replaced.
    pub(crate) updated:Vec<String>,
    /// A different file of that name, left alone.
    pub(crate) kept:Vec<String>,
    /// Could not be read or written, with the reason.
    pub(crate) failed:Vec<String>,
}
impl Report{
    /// One line for a status bar, or None when nothing changed or stood out.
    pub(crate) fn summary(&self,letter:u8)->Option<String>{
        let mut parts=Vec::new();
        let list=|v:&[String]|v.join(", ");
        if !self.updated.is_empty(){parts.push(format!("updated {}",list(&self.updated)));}
        if !self.added.is_empty(){parts.push(format!("added {}",list(&self.added)));}
        if !self.kept.is_empty(){parts.push(format!("kept your own {}",list(&self.kept)));}
        if !self.failed.is_empty(){parts.push(format!("could not copy {}",list(&self.failed)));}
        if parts.is_empty(){return None;}
        Some(format!("{}: from programs/dos: {}.",char::from(letter),parts.join("; ")))
    }
}

/// SHA-256 (FIPS 180-4).
pub(crate) fn sha256(data:&[u8])->[u8;32]{
    const K:[u32;64]=[
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
    let mut h:[u32;8]=[0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    let mut message=data.to_vec();
    message.push(0x80);
    while message.len()%64!=56{message.push(0);}
    message.extend_from_slice(&((data.len() as u64)*8).to_be_bytes());
    for block in message.chunks(64){
        let mut w=[0u32;64];
        for i in 0..16{w[i]=u32::from_be_bytes(block[i*4..i*4+4].try_into().unwrap());}
        for i in 16..64{
            let s0=w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3);
            let s1=w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10);
            w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let mut v=h;
        for i in 0..64{
            let s1=v[4].rotate_right(6)^v[4].rotate_right(11)^v[4].rotate_right(25);
            let ch=(v[4]&v[5])^(!v[4]&v[6]);
            let t1=v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0=v[0].rotate_right(2)^v[0].rotate_right(13)^v[0].rotate_right(22);
            let maj=(v[0]&v[1])^(v[0]&v[2])^(v[1]&v[2]);
            let t2=s0.wrapping_add(maj);
            v=[t1.wrapping_add(t2),v[0],v[1],v[2],v[3].wrapping_add(t1),v[4],v[5],v[6]];
        }
        for i in 0..8{h[i]=h[i].wrapping_add(v[i]);}
    }
    let mut out=[0u8;32];
    for i in 0..8{out[i*4..i*4+4].copy_from_slice(&h[i].to_be_bytes());}
    out
}
pub(crate) fn hex(digest:&[u8;32])->String{digest.iter().map(|b|format!("{b:02x}")).collect()}

/// Upper-case name -> known hashes, from the manifest text.
fn known_versions(text:&str)->HashMap<String,HashSet<String>>{
    let mut known:HashMap<String,HashSet<String>>=HashMap::new();
    for line in text.lines(){
        let line=line.trim();
        if line.is_empty()||line.starts_with('#'){continue;}
        let Some((hash,name))=line.split_once(char::is_whitespace) else{continue};
        if hash.len()!=64||!hash.bytes().all(|b|b.is_ascii_hexdigit()){continue;}
        known.entry(name.trim().to_ascii_uppercase()).or_default().insert(hash.to_ascii_lowercase());
    }
    known
}

/// Write `data` to `path` through a temporary file beside it, so a failed
/// write never leaves half a program.
fn replace(path:&Path,data:&[u8])->std::io::Result<()>{
    let mut temporary=path.as_os_str().to_owned();temporary.push(".pil-new");
    let temporary=PathBuf::from(temporary);
    std::fs::write(&temporary,data)?;
    std::fs::rename(&temporary,path).inspect_err(|_|{let _=std::fs::remove_file(&temporary);})
}

/// Copy the files of `source` (programs/dos) into `target` as described
/// above. A missing `source` or `target` is not an error: nothing happens.
pub(crate) fn refresh(source:&Path,target:&Path)->Report{
    let mut report=Report::default();
    if !source.is_dir()||!target.is_dir(){return report;}
    let known=std::fs::read_to_string(source.join(MANIFEST)).map(|t|known_versions(&t)).unwrap_or_default();
    let Ok(entries)=std::fs::read_dir(source) else{return report};
    let mut files:Vec<(String,PathBuf)>=entries.flatten()
        .filter(|e|e.file_type().is_ok_and(|t|t.is_file()))
        .filter_map(|e|Some((e.file_name().into_string().ok()?,e.path())))
        .filter(|(n,_)|n!=MANIFEST&&!n.starts_with('.')&&!n.ends_with(".pil-new"))
        .collect();
    files.sort();
    // Existing names on the drive, by upper case, as DOS sees them.
    let present:HashMap<String,PathBuf>=std::fs::read_dir(target).map(|d|d.flatten()
        .filter_map(|e|Some((e.file_name().into_string().ok()?.to_ascii_uppercase(),e.path()))).collect()).unwrap_or_default();
    for (name,path) in files{
        let fail=|r:&mut Report,e:&dyn std::fmt::Display|r.failed.push(format!("{name} ({e})"));
        if std::fs::metadata(&path).is_ok_and(|m|m.len()>LARGEST){continue;}
        let data=match std::fs::read(&path){Ok(d)=>d,Err(e)=>{fail(&mut report,&e);continue;}};
        let upper=name.to_ascii_uppercase();
        let Some(existing)=present.get(&upper) else{
            match replace(&target.join(&name),&data){Ok(())=>report.added.push(name),Err(e)=>fail(&mut report,&e)}
            continue;
        };
        if !existing.is_file(){report.kept.push(name);continue;}
        let old=match std::fs::read(existing){Ok(d)=>d,Err(e)=>{fail(&mut report,&e);continue;}};
        if old==data{continue;}
        if !known.get(&upper).is_some_and(|k|k.contains(&hex(&sha256(&old)))){report.kept.push(name);continue;}
        match replace(existing,&data){Ok(())=>report.updated.push(name),Err(e)=>fail(&mut report,&e)}
    }
    report
}

#[cfg(test)]
#[path="../tests/unit/own_programs.rs"]
mod tests;
