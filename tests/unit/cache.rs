use super::*;
use super::super::guest::ENTRY;
use super::super::address::{SegmentAccess,SegmentRegister};

#[test]
fn valid_instruction_reuses_the_same_decoded_program() {
    let (image,ram)=Image::load(&[0xb8,1,0,0xf4]).unwrap();let mut c=DecodeCache::default();
    let a=c.fetch(&image,&ram,ENTRY).unwrap();let b=c.fetch(&image,&ram,ENTRY).unwrap();
    assert!(Rc::ptr_eq(&a,&b));assert_eq!(c.stats(),Stats{hits:1,misses:1,..Stats::default()});
}
#[test]
fn changed_bytes_redecode_immediates_lengths_and_unsupported_opcodes() {
    let (image,mut ram)=Image::load(&[0xb8,1,0,0xf4]).unwrap();let mut c=DecodeCache::default();
    let old=c.fetch(&image,&ram,ENTRY).unwrap();
    ram.write_bytes(ENTRY,&[0xb0,2]).unwrap();let new=c.fetch(&image,&ram,ENTRY).unwrap();
    assert!(!Rc::ptr_eq(&old,&new));assert_eq!(new.length,2);assert_eq!(c.stats().invalidations,1);
    ram.write_bytes(ENTRY,&[0x62]).unwrap();assert!(matches!(c.fetch(&image,&ram,ENTRY),Err(GuestError::Decode(_))));
    assert!(!c.entries.contains_key(&ENTRY));assert_eq!(c.stats().invalidations,2);
}
#[test]
fn changed_fetch_limits_permissions_and_image_extent_are_rechecked() {
    let (original,ram)=Image::load(&[0xb8,1,0,0xf4]).unwrap();
    for kind in 0..3 {
        let mut c=DecodeCache::default();c.fetch(&original,&ram,ENTRY).unwrap();let mut image=original.clone();
        match kind {
            0=>image.segments[SegmentRegister::Cs.index()].limit=ENTRY,
            1=>image.segments[SegmentRegister::Cs.index()].access=SegmentAccess::Unusable,
            _=>image.end=ENTRY+1,
        }
        assert_eq!(c.fetch(&image,&ram,ENTRY).unwrap_err(),image.decode(&ram,ENTRY).unwrap_err());
        assert_eq!(c.stats().hits,0);assert_eq!(c.stats().invalidations,1);
    }
}
#[test]
fn data_segment_context_changes_replace_decoded_memory_binding() {
    let (mut image,ram)=Image::load(&[0x8b,0x07,0xf4]).unwrap();let mut c=DecodeCache::default();
    let old=c.fetch(&image,&ram,ENTRY).unwrap();image.segments[SegmentRegister::Ds.index()].base=0x500;
    let new=c.fetch(&image,&ram,ENTRY).unwrap();assert!(!Rc::ptr_eq(&old,&new));assert_eq!(c.stats().invalidations,1);
    if let super::super::guest::Operation::Pil{address:Some(a),..}=&new.operation {
        assert_eq!(a.segments[SegmentRegister::Ds.index()].base,0x500);
    }else{panic!("memory operation");}
}
#[test]
fn entry_capacity_is_bounded_and_flushes_without_changing_decode() {
    let (image,ram)=Image::load(&vec![0xf4;CAPACITY+1]).unwrap();let mut c=DecodeCache::default();
    for i in 0..=CAPACITY {assert_eq!(c.fetch(&image,&ram,ENTRY+i as u32).unwrap().length,1);}
    assert_eq!(c.entries.len(),1);assert_eq!(c.stats().evictions,CAPACITY as u64);assert_eq!(c.stats().misses,(CAPACITY+1) as u64);
}
#[test]
fn data_writes_and_readonly_code_keep_valid_cached_programs() {
    let (image,mut ram)=Image::load(&[0xb8,1,0,0xf4]).unwrap();let mut c=DecodeCache::default();
    let first=c.fetch(&image,&ram,ENTRY).unwrap();ram.write_bytes(0x500,&[42]).unwrap();ram.protect(ENTRY,3).unwrap();
    let second=c.fetch(&image,&ram,ENTRY).unwrap();assert!(Rc::ptr_eq(&first,&second));assert_eq!(c.stats().invalidations,0);
}
#[test]
fn disabling_and_clearing_reset_entries_and_counters() {
    let (image,ram)=Image::load(&[0xf4]).unwrap();let mut c=DecodeCache::default();c.fetch(&image,&ram,ENTRY).unwrap();
    c.set_enabled(false);assert!(!c.enabled());assert!(c.entries.is_empty());
    let a=c.fetch(&image,&ram,ENTRY).unwrap();let b=c.fetch(&image,&ram,ENTRY).unwrap();assert!(!Rc::ptr_eq(&a,&b));assert_eq!(c.stats(),Stats::default());
    c.set_enabled(true);c.fetch(&image,&ram,ENTRY).unwrap();c.clear();assert!(c.enabled());assert!(c.entries.is_empty());assert_eq!(c.stats(),Stats::default());
}
#[test]
fn failed_decodes_are_not_stored_or_reused() {
    let (image,ram)=Image::load(&[0x62]).unwrap();let mut c=DecodeCache::default();
    for _ in 0..2 {assert!(c.fetch(&image,&ram,ENTRY).is_err());}
    assert!(c.entries.is_empty());assert_eq!(c.stats().misses,2);assert_eq!(c.stats().hits,0);
}
