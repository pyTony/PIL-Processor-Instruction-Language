//! Replaceable numerics-coprocessor boundary. The default machine has no FPU.
//! The absent backend remains default; present execution is explicitly opt-in.
use crate::{Cpu, guest::Image, memory::Ram};
#[path = "fpu_decode.rs"]
mod operations;
#[allow(unused_imports)]
pub use operations::{classify, DecodeError, Operation};
#[path = "fpu_state.rs"]
mod state;
#[allow(unused_imports)]
pub use state::{Extended80, StackError, Tag, X87State};
#[path = "fpu_present.rs"]
mod present;
#[allow(unused_imports)]
pub use present::PresentFpu;

#[derive(Clone, Debug, Default)]
pub struct AbsentFpu;
/// DOS sessions clone their selected device. The default is always absent,
/// preserving existing machine and probe behaviour until explicitly enabled.
#[derive(Clone, Debug)]
pub enum Fpu {
    Absent(AbsentFpu),
    Present(PresentFpu),
}
impl Default for Fpu {
    fn default() -> Self { Self::Absent(AbsentFpu) }
}
impl Fpu {
    pub fn present() -> Self { Self::Present(PresentFpu::default()) }
    pub fn state(&self) -> Option<&X87State> {
        match self { Self::Absent(_) => None, Self::Present(fpu) => Some(&fpu.state) }
    }
    pub fn available(&self) -> bool { matches!(self, Self::Present(_)) }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Absent(device) => device.name(),
            Self::Present(_) => "80387 register backend (partial)",
        }
    }
    pub fn equipment(&self, value: u16) -> u16 {
        (value & !2) | if self.available() { 2 } else { 0 }
    }
    pub fn execute(&mut self, instruction: &Instruction, cpu: &mut Cpu,
                   ram: &mut Ram, image: &Image) -> Outcome {
        match self {
            Self::Absent(device) => device.execute(instruction, cpu, ram, image),
            Self::Present(device) => device.execute(instruction, cpu, ram, image),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Completed, NotAvailable, Unsupported, StackFault(StackError) }

#[derive(Clone, Debug)]
pub struct Instruction {
    pub bytes: [u8; 15],
    pub length: u32,
    pub opcode: u8,
    /// None for WAIT; ESC ModR/M is retained after consuming any prefixes.
    pub modrm: Option<u8>,
}

/// Decode the CPU's ESC envelope, without interpreting the coprocessor opcode.
/// Fetch only bytes belonging to this instruction; WAIT is a separate instruction.
/// Prefix/address sizes follow the existing real-mode decoder's profile.
pub fn decode<E>(mut fetch: impl FnMut(u32) -> Result<u8, E>) -> Result<Option<Instruction>, E> {
    let mut bytes = [0; 15];
    let mut length = 0usize;
    let mut addr32 = false;
    let mut wide = false;
    let mut segment = false;
    let opcode = loop {
        if length == 15 { return Ok(None); }
        let b = fetch(length as u32)?;
        bytes[length] = b;
        length += 1;
        match b {
            0x66 if !wide => wide = true,
            0x67 if !addr32 => addr32 = true,
            0x26 | 0x2e | 0x36 | 0x3e | 0x64 | 0x65 if !segment => segment = true,
            // Leave LOCK, REP and duplicate prefixes to the ordinary decoder.
            _ => break b,
        }
    };
    if opcode != 0x9b && !(0xd8..=0xdf).contains(&opcode) { return Ok(None); }
    let mut modrm = None;
    if opcode != 0x9b {
        if length == 15 { return Ok(None); }
        let m = fetch(length as u32)?;
        modrm = Some(m);
        bytes[length] = m;
        length += 1;
        let mode = m >> 6;
        let rm = m & 7;
        let mut displacement = 0;
        if mode != 3 {
            if addr32 {
                let mut base = rm;
                if rm == 4 {
                    if length == 15 { return Ok(None); }
                    let sib = fetch(length as u32)?;
                    bytes[length] = sib;
                    length += 1;
                    base = sib & 7;
                }
                displacement = match mode { 0 if base == 5 => 4, 1 => 1, 2 => 4, _ => 0 };
            } else {
                displacement = match mode { 0 if rm == 6 => 2, 1 => 1, 2 => 2, _ => 0 };
            }
        }
        if length + displacement > 15 { return Ok(None); }
        for _ in 0..displacement {
            bytes[length] = fetch(length as u32)?;
            length += 1;
        }
    }
    Ok(Some(Instruction { bytes, length: length as u32, opcode, modrm }))
}

impl AbsentFpu {
    pub fn available(&self) -> bool { false }
    pub fn name(&self) -> &'static str { "80387 unavailable (no coprocessor installed)" }
    /// BIOS equipment bit 1 is shared by INT 11h, the BDA and CMOS.
    pub fn equipment(&self, value: u16) -> u16 {
        (value & !2) | if self.available() { 2 } else { 0 }
    }
    /// Hardware-absent policy: BUSY#/ERROR# inactive; ESC has no numerics effects.
    /// In particular FNSTCW/FNSTSW do not write RAM or AX, preserving probe
    /// sentinels. Never clear EM/TS or advance IP here: the CPU owns retirement
    /// and fault delivery. CPU/RAM/image are explicit future backend inputs.
    pub fn execute(&mut self, instruction: &Instruction, _cpu: &mut Cpu,
                   _ram: &mut Ram, image: &Image) -> Outcome {
        let msw = image.msw;
        // Intel 80386 PRM 11.1.3 / 11.1.5: ESC traps on EM or TS;
        // WAIT ignores EM and only traps on MP together with TS.
        let trap = if instruction.opcode == 0x9b {
            msw & 0x0a == 0x0a
        } else {
            msw & 0x0c != 0
        };
        if trap { Outcome::NotAvailable } else { Outcome::Completed }
    }
}

#[cfg(test)]
#[path = "../tests/unit/fpu.rs"]
mod tests;
