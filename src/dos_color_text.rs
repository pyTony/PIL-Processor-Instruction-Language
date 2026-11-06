//! Existing colour text display memory layout, shared with the DOS console.
//! The guest RAM remains authoritative; this module adds no display device.
pub(super) const VIDEO:u32=0xb8000;
pub(super) const PAGE:u32=4096;
pub(super) const CELLS:u32=80*25;

