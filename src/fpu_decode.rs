//! 80387 instruction classification, independent of the CPU executor.
//! Unsupported encodings are explicitly rejected, never treated as NOPs.
//! The caller must supply the ESC opcode and ModR/M byte after prefix decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Finit,
    Fnclex,
    FnstswAx,
    Fld1,
    Fldz,
    FldSt(u8),
    Fxch(u8),
    FstpSt(u8),
    FldM32,
    FstpM32,
    FldM64,
    FstpM64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError { Unsupported { opcode: u8, modrm: u8 } }

/// Distinguishes register and memory encodings before dispatch.
/// This intentionally does not resolve effective addresses or touch memory.
pub fn classify(opcode: u8, modrm: u8) -> Result<Operation, DecodeError> {
    let mode = modrm >> 6;
    let reg = (modrm >> 3) & 7;
    let rm = modrm & 7;
    let operation = if mode == 3 {
        match (opcode, modrm) {
            (0xdb, 0xe3) => Some(Operation::Finit),
            (0xdb, 0xe2) => Some(Operation::Fnclex),
            (0xdf, 0xe0) => Some(Operation::FnstswAx),
            (0xd9, 0xe8) => Some(Operation::Fld1),
            (0xd9, 0xee) => Some(Operation::Fldz),
            (0xd9, 0xc0..=0xc7) => Some(Operation::FldSt(rm)),
            (0xd9, 0xc8..=0xcf) => Some(Operation::Fxch(rm)),
            (0xdd, 0xd8..=0xdf) => Some(Operation::FstpSt(rm)),
            _ => None,
        }
    } else {
        match (opcode, reg) {
            (0xd9, 0) => Some(Operation::FldM32),
            (0xd9, 3) => Some(Operation::FstpM32),
            (0xdd, 0) => Some(Operation::FldM64),
            (0xdd, 3) => Some(Operation::FstpM64),
            _ => None,
        }
    };
    operation.ok_or(DecodeError::Unsupported { opcode, modrm })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_and_constant_instructions() {
        assert_eq!(classify(0xdb,0xe3),Ok(Operation::Finit));
        assert_eq!(classify(0xdb,0xe2),Ok(Operation::Fnclex));
        assert_eq!(classify(0xdf,0xe0),Ok(Operation::FnstswAx));
        assert_eq!(classify(0xd9,0xe8),Ok(Operation::Fld1));
        assert_eq!(classify(0xd9,0xee),Ok(Operation::Fldz));
    }
    #[test]
    fn register_stack_indices() {
        for i in 0..8 {
            assert_eq!(classify(0xd9,0xc0+i),Ok(Operation::FldSt(i)));
            assert_eq!(classify(0xd9,0xc8+i),Ok(Operation::Fxch(i)));
            assert_eq!(classify(0xdd,0xd8+i),Ok(Operation::FstpSt(i)));
        }
    }
    #[test]
    fn memory_width_and_modrm_reg_fields() {
        for mode in [0u8,1,2] {
            let m=mode<<6;
            assert_eq!(classify(0xd9,m),Ok(Operation::FldM32));
            assert_eq!(classify(0xd9,m|0x18),Ok(Operation::FstpM32));
            assert_eq!(classify(0xdd,m),Ok(Operation::FldM64));
            assert_eq!(classify(0xdd,m|0x18),Ok(Operation::FstpM64));
        }
    }
    #[test]
    fn unsupported_is_not_silent_success() {
        for (op,modrm) in [(0xd8,0xc0),(0xd9,0xe0),(0xdb,0xe4),(0xdf,0xe1),(0xd9,0x08),(0xdd,0x10)] {
            assert_eq!(classify(op,modrm),Err(DecodeError::Unsupported{opcode:op,modrm}));
        }
    }
}
