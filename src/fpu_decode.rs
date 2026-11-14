//! 80387 instruction classification, independent of the CPU executor.
//! Unsupported encodings are explicitly rejected, never treated as NOPs.
//! The caller must supply the ESC opcode and ModR/M byte after prefix decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Finit,
    Fnclex,
    FnstswAx,
    /// Store the 16-bit status word to a memory operand (DD /7).
    FnstswM16,
    /// Load the 16-bit control word from a memory operand (D9 /5).
    Fldcw,
    /// Store the current 16-bit control word to a memory operand (D9 /7).
    Fnstcw,
    /// Store the 14-byte (16-bit) or 28-byte (32-bit) x87 environment (D9 /6).
    Fnstenv,
    /// Load the 14-byte (16-bit) or 28-byte (32-bit) x87 environment (D9 /4).
    Fldenv,
    Fchs,
    Fld1,
    Fldz,
    Fyl2xp1,
    FldSt(u8),
    Fxch(u8),
    FstpSt(u8),
    FldM32,
    FstpM32,
    FldM64,
    FstpM64,
    FldM80,
    FstpM80,
    FildM16,
    FildM32,
    FildM64,
    Compare { index: u8, pop: bool },
    ComparePop2,
    CompareMemory { bytes: u32, pop: bool },
    /// Register arithmetic, either ST(0) op ST(i) or ST(i) op ST(0), pop.
    Arithmetic { kind: Arithmetic, index: u8, pop: bool },
    /// ST(0) op IEEE m32/m64, source is read only.
    MemoryArithmetic { kind: Arithmetic, bytes: u32 },
}
/// Order is explicit: Subtract is destination-source, ReverseSubtract is source-destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arithmetic { Add, Multiply, Subtract, ReverseSubtract, Divide, ReverseDivide }
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
            (0xd9, 0xe0) => Some(Operation::Fchs),
            (0xd9, 0xe8) => Some(Operation::Fld1),
            (0xd9, 0xee) => Some(Operation::Fldz),
            (0xd9, 0xf9) => Some(Operation::Fyl2xp1),
            (0xd9, 0xc0..=0xc7) => Some(Operation::FldSt(rm)),
            (0xd9, 0xc8..=0xcf) => Some(Operation::Fxch(rm)),
            (0xdd, 0xd8..=0xdf) => Some(Operation::FstpSt(rm)),
            (0xd8, 0xc0..=0xc7) => Some(Operation::Arithmetic { kind: Arithmetic::Add, index: rm, pop: false }),
            (0xd8, 0xc8..=0xcf) => Some(Operation::Arithmetic { kind: Arithmetic::Multiply, index: rm, pop: false }),
            (0xd8, 0xe0..=0xe7) => Some(Operation::Arithmetic { kind: Arithmetic::Subtract, index: rm, pop: false }),
            (0xd8, 0xe8..=0xef) => Some(Operation::Arithmetic { kind: Arithmetic::ReverseSubtract, index: rm, pop: false }),
            (0xd8, 0xf0..=0xf7) => Some(Operation::Arithmetic { kind: Arithmetic::Divide, index: rm, pop: false }),
            (0xd8, 0xf8..=0xff) => Some(Operation::Arithmetic { kind: Arithmetic::ReverseDivide, index: rm, pop: false }),
            (0xd8, 0xd0..=0xd7) => Some(Operation::Compare { index: rm, pop: false }),
            (0xd8, 0xd8..=0xdf) => Some(Operation::Compare { index: rm, pop: true }),
            (0xde, 0xd9) => Some(Operation::ComparePop2),
            (0xde, 0xc0..=0xc7) => Some(Operation::Arithmetic { kind: Arithmetic::Add, index: rm, pop: true }),
            (0xde, 0xc8..=0xcf) => Some(Operation::Arithmetic { kind: Arithmetic::Multiply, index: rm, pop: true }),
            (0xde, 0xe0..=0xe7) => Some(Operation::Arithmetic { kind: Arithmetic::ReverseSubtract, index: rm, pop: true }),
            (0xde, 0xe8..=0xef) => Some(Operation::Arithmetic { kind: Arithmetic::Subtract, index: rm, pop: true }),
            (0xde, 0xf0..=0xf7) => Some(Operation::Arithmetic { kind: Arithmetic::ReverseDivide, index: rm, pop: true }),
            (0xde, 0xf8..=0xff) => Some(Operation::Arithmetic { kind: Arithmetic::Divide, index: rm, pop: true }),
            _ => None,
        }
    } else {
        match (opcode, reg) {
            (0xd9, 0) => Some(Operation::FldM32),
            (0xd9, 3) => Some(Operation::FstpM32),
            (0xd9, 4) => Some(Operation::Fldenv),
            (0xd9, 5) => Some(Operation::Fldcw),
            (0xd9, 6) => Some(Operation::Fnstenv),
            (0xd9, 7) => Some(Operation::Fnstcw),
            (0xdd, 0) => Some(Operation::FldM64),
            (0xdd, 3) => Some(Operation::FstpM64),
            (0xdd, 7) => Some(Operation::FnstswM16),
            (0xdb, 0) => Some(Operation::FildM32),
            (0xdb, 5) => Some(Operation::FldM80),
            (0xdb, 7) => Some(Operation::FstpM80),
            (0xdf, 0) => Some(Operation::FildM16),
            (0xdf, 5) => Some(Operation::FildM64),
            (0xd8 | 0xdc, 2) => Some(Operation::CompareMemory { bytes: if opcode == 0xd8 { 4 } else { 8 }, pop: false }),
            (0xd8 | 0xdc, 3) => Some(Operation::CompareMemory { bytes: if opcode == 0xd8 { 4 } else { 8 }, pop: true }),
            (0xd8, 0) => Some(Operation::MemoryArithmetic { kind: Arithmetic::Add, bytes: 4 }),
            (0xd8, 1) => Some(Operation::MemoryArithmetic { kind: Arithmetic::Multiply, bytes: 4 }),
            (0xd8, 4) => Some(Operation::MemoryArithmetic { kind: Arithmetic::Subtract, bytes: 4 }),
            (0xd8, 5) => Some(Operation::MemoryArithmetic { kind: Arithmetic::ReverseSubtract, bytes: 4 }),
            (0xdc, 0) => Some(Operation::MemoryArithmetic { kind: Arithmetic::Add, bytes: 8 }),
            (0xdc, 1) => Some(Operation::MemoryArithmetic { kind: Arithmetic::Multiply, bytes: 8 }),
            (0xdc, 4) => Some(Operation::MemoryArithmetic { kind: Arithmetic::Subtract, bytes: 8 }),
            (0xdc, 5) => Some(Operation::MemoryArithmetic { kind: Arithmetic::ReverseSubtract, bytes: 8 }),
            (0xd8 | 0xdc, 6) => Some(Operation::MemoryArithmetic { kind: Arithmetic::Divide, bytes: if opcode == 0xd8 { 4 } else { 8 } }),
            (0xd8 | 0xdc, 7) => Some(Operation::MemoryArithmetic { kind: Arithmetic::ReverseDivide, bytes: if opcode == 0xd8 { 4 } else { 8 } }),
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
        assert_eq!(classify(0xd9,0xf9),Ok(Operation::Fyl2xp1));
        assert_eq!(classify(0xd9,0x26),Ok(Operation::Fldenv)); // FLDENV [disp16]
        assert_eq!(classify(0xd9,0x2f),Ok(Operation::Fldcw)); // FLDCW [BX]
        assert_eq!(classify(0xd9,0x36),Ok(Operation::Fnstenv)); // FNSTENV [disp16]
        assert_eq!(classify(0xd9,0x3f),Ok(Operation::Fnstcw)); // FNSTCW [BX]
        // /5 and /7 require memory operands, not ModR/M register mode.
        assert_eq!(classify(0xd9,0xef),Err(DecodeError::Unsupported{opcode:0xd9,modrm:0xef}));
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
    fn register_arithmetic_opcode_order_and_pop_semantics() {
        for i in 0..8 {
            for (op,base,kind,pop) in [
                (0xd8,0xc0,Arithmetic::Add,false),
                (0xd8,0xc8,Arithmetic::Multiply,false),
                (0xd8,0xe0,Arithmetic::Subtract,false),
                (0xd8,0xe8,Arithmetic::ReverseSubtract,false),
                (0xde,0xc0,Arithmetic::Add,true),
                (0xde,0xc8,Arithmetic::Multiply,true),
                (0xde,0xe0,Arithmetic::ReverseSubtract,true),
                (0xde,0xe8,Arithmetic::Subtract,true),
                (0xd8,0xf0,Arithmetic::Divide,false),
                (0xd8,0xf8,Arithmetic::ReverseDivide,false),
                (0xde,0xf0,Arithmetic::ReverseDivide,true),
                (0xde,0xf8,Arithmetic::Divide,true),
            ] {
                assert_eq!(classify(op,base+i),Ok(Operation::Arithmetic{kind,index:i,pop}));
            }
        }
    }
    #[test]
    fn memory_arithmetic_opcodes_are_distinct_from_store() {
        for (opcode,bytes) in [(0xd8,4),(0xdc,8)] {
            for (reg,kind) in [(0,Arithmetic::Add),(1,Arithmetic::Multiply),
                               (4,Arithmetic::Subtract),(5,Arithmetic::ReverseSubtract),
                               (6,Arithmetic::Divide),(7,Arithmetic::ReverseDivide)] {
                assert_eq!(classify(opcode,reg<<3),
                    Ok(Operation::MemoryArithmetic{kind,bytes}));
            }
        }
        assert_eq!(classify(0xdb,0x28), Ok(Operation::FldM80));
        assert_eq!(classify(0xdb,0x38), Ok(Operation::FstpM80));
        assert_eq!(classify(0xd8,0x10), Ok(Operation::CompareMemory{bytes:4,pop:false}));
        assert_eq!(classify(0xdc,0x18), Ok(Operation::CompareMemory{bytes:8,pop:true}));
    }
    #[test]
    fn compare_register_encodings_and_double_pop() {
        for i in 0..8 {
            assert_eq!(classify(0xd8,0xd0+i), Ok(Operation::Compare{index:i,pop:false}));
            assert_eq!(classify(0xd8,0xd8+i), Ok(Operation::Compare{index:i,pop:true}));
        }
        assert_eq!(classify(0xde,0xd9),Ok(Operation::ComparePop2));
    }
    #[test]
    fn unsupported_is_not_silent_success() {
        assert_eq!(classify(0xd9,0xe0),Ok(Operation::Fchs));
        assert_eq!(classify(0xdd,0x3f),Ok(Operation::FnstswM16));
        for (op,modrm) in [(0xd9,0xe1),(0xdb,0xe4),(0xdf,0xe1),(0xd9,0x08),(0xdd,0x10)] {
            assert_eq!(classify(op,modrm),Err(DecodeError::Unsupported{opcode:op,modrm}));
        }
    }
}
