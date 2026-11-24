//! Entry guards for a precompiled block. A block may run only when the live
//! machine is the one it was compiled for and its code bytes are unchanged;
//! otherwise the shared interpreter executes the instruction. These checks
//! read guest state only: they know nothing about which program is running.
use crate::{address::{SegmentCache,SegmentState},guest,Ram};

/// The live segment state matches the recorded one. Ordinary real-mode
/// relocation (every cache a plain real-mode cache) is accepted; paging,
/// unreal-mode limits and any other difference are not.
pub fn compatible(live:&SegmentState,expected:&SegmentState)->bool{
    if live.paging{return false;}
    if live==expected{return true;}
    if !live.real_mode||!expected.real_mode{return false;}
    let plain=|s:&SegmentState|s.segments.iter().zip(s.selectors).all(|(c,sel)|*c==SegmentCache::real_mode(sel));
    if !plain(live)||!plain(expected){return false;}
    let mut normalized=*live;normalized.selectors=expected.selectors;normalized.segments=expected.segments;
    normalized==*expected
}

/// `bytes` are at CS:`ip`, inside the CS limit, inside an executable window
/// of the image, and unchanged in RAM.
pub fn code_matches(image:&guest::Image,ram:&Ram,ip:u32,bytes:&[u8])->bool{
    let cs=image.segments[1];let Some(at)=cs.base.checked_add(ip)else{return false};
    let last=u64::from(ip)+bytes.len() as u64;
    if last>u64::from(cs.limit)+1{return false;}
    let physical=u64::from(ram.physical_address(at));let end=physical+bytes.len() as u64;
    let start=u64::from(image.fetch_base)+u64::from(image.start);let bound=u64::from(image.fetch_base)+u64::from(image.end);
    if !((physical>=start&&end<=bound)||image.system.iter().any(|&(s,e)|physical>=u64::from(s)&&end<=u64::from(e))){return false;}
    ram.contiguous_slice(at,bytes.len()).is_some_and(|s|s==bytes)
}

/// A dispatcher runs consecutive blocks without returning to the DOS loop
/// for at most this many instructions, so input and IRQ checks stay bounded.
pub const MAX_CHAIN:usize=256;
