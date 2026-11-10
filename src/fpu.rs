//! Replaceable numerics-coprocessor boundary. The default machine has no FPU.
//! No host floating-point operations or fabricated x87 results are used.
use crate::{Cpu, guest::Image, memory::Ram};
#[path = "fpu_state.rs"]
mod state;
#[allow(unused_imports)]
pub use state::{Extended80, StackError, Tag, X87State};

#[derive(Clone, Debug, Default)]
pub struct AbsentFpu;
/// Replace this alias with a stateful backend (or backend enum) when x87 is added.
/// Loaded owns and clones the backend, so state survives session slices.
pub type Fpu = AbsentFpu;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Completed, NotAvailable }

#[derive(Clone, Debug)]
pub struct Instruction {
    pub bytes: [u8; 15],
    pub length: u32,
    pub opcode: u8,
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
    if opcode != 0x9b {
        if length == 15 { return Ok(None); }
        let m = fetch(length as u32)?;
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
    Ok(Some(Instruction { bytes, length: length as u32, opcode }))
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
