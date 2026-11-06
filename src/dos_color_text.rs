//! Mode-3 colour text layout shared by the BIOS console and ANSI driver.
//! Guest RAM remains authoritative; this module adds no display device.
pub(super) const VIDEO:u32=0xb8000;
pub(super) const PAGE:u32=4096;
pub(super) const COLS:u32=80;
pub(super) const ROWS:u32=25;
pub(super) const CELLS:u32=COLS*ROWS;
