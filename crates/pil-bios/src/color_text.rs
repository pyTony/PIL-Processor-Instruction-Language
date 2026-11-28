//! Mode-3 colour text layout shared by the BIOS console and ANSI driver.
//! Guest RAM remains authoritative; this module adds no display device.
pub const VIDEO:u32=0xb8000;
pub const PAGE:u32=4096;
pub const COLS:u32=80;
pub const ROWS:u32=25;
pub const CELLS:u32=COLS*ROWS;
