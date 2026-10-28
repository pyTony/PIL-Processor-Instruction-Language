//! Bounded decoded-instruction/PIL cache. Guest execution still interprets tokens.
use std::collections::BTreeMap;
use std::rc::Rc;
use super::guest::{Decoded,GuestError,Image};
use super::memory::Ram;

pub const CAPACITY:usize=256;
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct Stats {pub hits:u64,pub misses:u64,pub invalidations:u64,pub evictions:u64}
#[derive(Clone)]
struct Entry {image:Image,signature:[u8;15],decoded:Rc<Decoded>}
pub struct DecodeCache {enabled:bool,entries:BTreeMap<u32,Entry>,fast:Box<[Option<(u32,Entry)>;CAPACITY]>,stats:Stats}
impl Default for DecodeCache {
    fn default()->Self {Self{enabled:true,entries:BTreeMap::new(),fast:Box::new(std::array::from_fn(|_|None)),stats:Stats::default()}}
}
impl DecodeCache {
    pub fn enabled(&self)->bool {self.enabled}
    pub fn stats(&self)->Stats {self.stats}
    pub fn clear(&mut self) {self.entries.clear();self.fast.fill(None);self.stats=Stats::default();}
    pub fn set_enabled(&mut self,enabled:bool) {self.enabled=enabled;self.clear();}
    pub fn summary(&self)->String {
        format!("cache={} entries={}/{} hits={} misses={} invalidations={} evictions={}",if self.enabled{"on"}else{"off"},self.entries.len(),CAPACITY,self.stats.hits,self.stats.misses,self.stats.invalidations,self.stats.evictions)
    }
    pub fn fetch(&mut self,image:&Image,ram:&Ram,eip:u32)->Result<Rc<Decoded>,GuestError> {
        if !self.enabled {return image.decode(ram,eip).map(Rc::new);}
        let slot=((eip^(eip>>8)) as usize)&(CAPACITY-1);
        if let Some((cached_eip,entry))=&self.fast[slot] {
            if *cached_eip==eip && entry.image==*image {
                if let Ok(signature)=image.fetch_signature(ram,eip,entry.decoded.length) {
                    if signature==entry.signature {
                        self.stats.hits+=1;return Ok(Rc::clone(&entry.decoded));
                    }
                }
            }
        }
        if let Some(entry)=self.entries.get(&eip) {
            if entry.image==*image {
                // Re-fetch every encoded byte through the current fetch checks.
                // This catches poke, guest stores, overlap and changed lengths
                // without requiring every future RAM writer to notify the cache.
                match image.fetch_signature(ram,eip,entry.decoded.length) {
                    Ok(signature) if signature==entry.signature=>{
                        self.fast[slot]=Some((eip,entry.clone()));
                        self.stats.hits+=1;return Ok(Rc::clone(&entry.decoded));
                    }
                    Err(error)=>{
                        if self.fast[slot].as_ref().is_some_and(|(k,_)|*k==eip){self.fast[slot]=None;}
                        self.entries.remove(&eip);self.stats.invalidations+=1;return Err(error);
                    }
                    _=>{},
                }
            }
            if self.fast[slot].as_ref().is_some_and(|(k,_)|*k==eip){self.fast[slot]=None;}
            self.entries.remove(&eip);self.stats.invalidations+=1;
        }
        self.stats.misses+=1;
        let decoded=Rc::new(image.decode(ram,eip)?);
        let signature=image.fetch_signature(ram,eip,decoded.length)?;
        if self.entries.len()==CAPACITY {
            self.stats.evictions+=self.entries.len() as u64;self.entries.clear();self.fast.fill(None);
        }
        let entry=Entry{image:image.clone(),signature,decoded:Rc::clone(&decoded)};
        self.fast[slot]=Some((eip,entry.clone()));
        self.entries.insert(eip,entry);
        Ok(decoded)
    }
}
#[cfg(test)]
#[path="../tests/unit/cache.rs"]
mod tests;
