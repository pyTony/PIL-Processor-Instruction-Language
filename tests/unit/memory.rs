use super::*;
#[test]
fn little_endian_unaligned_round_trips() {
    let mut m=Ram::new(8);
    for w in [MemWidth::Byte,MemWidth::Word,MemWidth::Dword] {
        m.store(1,w,0x12345678).unwrap();
        assert_eq!(m.load(1,w).unwrap(),0x12345678&match w {MemWidth::Byte=>255,MemWidth::Word=>65535,MemWidth::Dword=>u32::MAX});
    }
    assert_eq!(&m.bytes()[1..5],&[0x78,0x56,0x34,0x12]);
}
#[test]
fn full_range_is_checked_before_storing() {
    let mut m=Ram::new(8);m.store(4,MemWidth::Dword,1).unwrap();let old=m.clone();
    assert_eq!(m.store(5,MemWidth::Dword,2),Err(AccessFault{address:5,bytes:4,write:true}));
    assert_eq!(m,old);assert!(m.load(7,MemWidth::Word).is_err());
}
#[test]
fn readonly_overlap_is_atomic_and_reads_still_work() {
    let mut m=Ram::new(8);m.write_bytes(0,&[1,2,3,4,5,6,7,8]).unwrap();m.protect(2,1).unwrap();
    let before=m.clone();assert!(m.store(0,MemWidth::Dword,0).is_err());assert_eq!(m,before);
    assert_eq!(m.load(0,MemWidth::Dword).unwrap(),0x04030201);
    m.store(4,MemWidth::Dword,0).unwrap();
}
#[test]
fn guest_address_does_not_wrap_or_panic() {
    let mut m=Ram::new(8);let before=m.clone();
    assert!(m.store(u32::MAX,MemWidth::Dword,1).is_err());assert_eq!(m,before);
    assert!(m.load(u32::MAX,MemWidth::Dword).is_err());
    assert!(m.protect(1,usize::MAX).is_err());
}
#[test]
fn empty_ram_has_no_readable_byte() {
    let mut m=Ram::new(0);
    assert!(m.load(0,MemWidth::Byte).is_err());assert!(m.store(0,MemWidth::Byte,0).is_err());
    assert!(m.protect(0,0).is_ok());assert!(m.bytes().is_empty());
}
#[test]
fn debugger_bulk_writes_check_whole_chunk() {
    let mut m=Ram::new(4);let before=m.clone();
    assert!(m.write_bytes(3,&[1,2]).is_err());assert_eq!(m,before);
    m.protect(1,1).unwrap();let before=m.clone();
    assert!(m.write_bytes(0,&[1,2]).is_err());assert_eq!(m,before);
}
#[test]
fn invalid_protection_does_not_change_ram() {
    let mut m=Ram::new(4);let before=m.clone();
    assert!(m.protect(3,2).is_err());assert_eq!(m,before);
}
