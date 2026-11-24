//! Ordered, read-only installation media. Paths are relative to the manifest.
use std::path::{Path,PathBuf};
use std::io::Read;

pub(super) struct DiskSet{pub(super) images:Vec<PathBuf>,pub(super) current:usize}
impl DiskSet{
    #[cfg(any(windows,test))]
    pub(super) fn numbered(image:&Path)->Result<Option<Self>,String>{
        if !image.is_file(){return Err("selected disk image is not a file".into());}
        let stem=image.file_stem().and_then(|s|s.to_str()).ok_or("image name is not Unicode")?;
        let prefix=stem.trim_end_matches(|c:char|c.is_ascii_digit());
        let digits=stem.len()-prefix.len();
        if digits==0{return Ok(None);}
        let parent=image.parent().unwrap_or(Path::new("."));
        let extension=image.extension().and_then(|s|s.to_str()).unwrap_or("");
        let mut images=Vec::new();
        for entry in std::fs::read_dir(parent).map_err(|e|e.to_string())?{
            let entry=entry.map_err(|e|e.to_string())?;let path=entry.path();
            if !path.is_file()||!path.extension().and_then(|s|s.to_str()).is_some_and(|s|s.eq_ignore_ascii_case(extension)){continue;}
            let Some(name)=path.file_stem().and_then(|s|s.to_str())else{continue;};
            let head=name.trim_end_matches(|c:char|c.is_ascii_digit());
            if head.eq_ignore_ascii_case(prefix)&&name.len()-head.len()==digits{
                let number=name[head.len()..].parse::<u64>().map_err(|_|"disk number too large")?;
                images.push((number,path));if images.len()>99{return Err("disk set exceeds 99 images".into());}
            }
        }
        if images.len()<2{return Ok(None);}
        images.sort_by_key(|(number,_)|*number);
        if images.windows(2).any(|pair|pair[0].0==pair[1].0){return Err("ambiguous duplicate disk number".into());}
        Ok(Some(Self{images:images.into_iter().map(|(_,path)|path).collect(),current:0}))
    }
    pub(super) fn read(manifest:&Path)->Result<Self,String>{
        let file=std::fs::File::open(manifest).map_err(|e|format!("disk set {}: {e}",manifest.display()))?;
        let mut bytes=Vec::new();
        file.take(65537).read_to_end(&mut bytes).map_err(|e|format!("disk-set manifest: {e}"))?;
        if bytes.len()>65536{return Err("disk-set manifest exceeds 64 KiB".into());}
        let text=std::str::from_utf8(&bytes).map_err(|_|"disk-set manifest must be UTF-8")?;
        let parent=manifest.parent().unwrap_or(Path::new("."));
        let mut images=Vec::new();
        for line in text.trim_start_matches('\u{feff}').lines(){
            let line=line.trim();
            if line.is_empty()||line.starts_with('#')||line.starts_with(';'){continue;}
            let path=crate::dos_config::host_path(line);
            let path=if path.is_absolute(){path}else{parent.join(path)};
            if !path.extension().and_then(|s|s.to_str()).is_some_and(|s|s.eq_ignore_ascii_case("img")||s.eq_ignore_ascii_case("ima")){
                return Err("disk-set entries must be .img or .ima images".into());
            }
            if !path.is_file(){return Err(format!("disk-set image {} is not a file",path.display()));}
            if images.contains(&path){return Err("duplicate disk-set image".into());}
            images.push(path);
            if images.len()>99{return Err("disk set exceeds 99 images".into());}
        }
        if images.is_empty(){return Err("disk set has no images".into());}
        Ok(Self{images,current:0})
    }
    pub(super) fn select(&self,selection:&str)->Result<usize,String>{
        let index=match selection{
            "next"=>self.current.checked_add(1),
            "prev"=>self.current.checked_sub(1),
            n=>n.parse::<usize>().ok().and_then(|n|n.checked_sub(1)),
        };
        index.filter(|&i|i<self.images.len()).ok_or_else(||format!("disk must be 1..{}; no disk beyond the end of the set",self.images.len()))
    }
    pub(super) fn description(&self,letter:u8)->String{
        format!("{}: disk {}/{} - {}",char::from(letter),self.current+1,self.images.len(),self.images[self.current].display())
    }
}

#[cfg(test)]
#[path="../../tests/unit/disk_set.rs"]
mod tests;
