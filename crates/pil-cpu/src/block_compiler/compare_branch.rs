use crate::{
    guest::{Decoded, Operation},
    Binary, Cpu, LazyFlags, PackedOp, Token, View,
};
use std::rc::Rc;

enum Operand {
    Register(View),
    Immediate(u64),
}

#[derive(Clone, Copy)]
enum AluKind {
    Sub { write_cf: bool },
    Add { write_cf: bool },
    And,
    Or,
    Xor,
}

/// Two validated PIL operations with no observable boundary between them.
pub struct CompareBranch {
    left: View,
    right: Operand,
    kind: AluKind,
    write_result: bool,
    condition: u8,
    taken: u32,
    fallthrough: u32,
}

impl CompareBranch {
    pub fn suffix(steps: &[Rc<Decoded>]) -> Option<Self> {
        let [.., arithmetic, branch] = steps else {
            return None;
        };
        let Operation::Pil {
            program,
            dst,
            src,
            address: None,
            ..
        } = &arithmetic.operation
        else {
            return None;
        };
        let (read, kind, r, w, tail) = match program.packed.as_slice() {
            [read, PackedOp::SubLazy { a, b, r, w, write_cf }, tail @ ..] => {
                let valid = match read {
                    PackedOp::ReadDstSrc { a: x, b: y, w: width } => (x, y, width) == (a, b, w),
                    PackedOp::ReadDstConst { a: x, b: y, w: width, .. } => (x, y, width) == (a, b, w),
                    _ => false,
                };
                if !valid {
                    return None;
                }
                (read, AluKind::Sub { write_cf: *write_cf }, *r, *w, tail)
            }
            [read, PackedOp::AddLazy { a, b, r, w, write_cf }, tail @ ..] => {
                let valid = match read {
                    PackedOp::ReadDstSrc { a: x, b: y, w: width } => (x, y, width) == (a, b, w),
                    PackedOp::ReadDstConst { a: x, b: y, w: width, .. } => (x, y, width) == (a, b, w),
                    _ => false,
                };
                if !valid {
                    return None;
                }
                (read, AluKind::Add { write_cf: *write_cf }, *r, *w, tail)
            }
            [
                read,
                PackedOp::Token(Token::Binary(r, op @ (Binary::And | Binary::Or | Binary::Xor), a, b)),
                PackedOp::LogicLazy { r: r2, w },
                tail @ ..,
            ] if r == r2 => {
                let valid = match read {
                    PackedOp::ReadDstSrc { a: x, b: y, w: width } => (x, y, width) == (a, b, w),
                    PackedOp::ReadDstConst { a: x, b: y, w: width, .. } => (x, y, width) == (a, b, w),
                    _ => false,
                };
                if !valid {
                    return None;
                }
                let kind = match op {
                    Binary::And => AluKind::And,
                    Binary::Or => AluKind::Or,
                    Binary::Xor => AluKind::Xor,
                    _ => unreachable!(),
                };
                (read, kind, *r, *w, tail)
            }
            _ => return None,
        };
        let write_result = match tail {
            [PackedOp::Token(Token::Retire)] => false,
            [PackedOp::Token(Token::Write(slot)), PackedOp::Token(Token::Retire)] if *slot == r => {
                true
            }
            _ => return None,
        };
        let right = match read {
            PackedOp::ReadDstSrc { .. } if src.width == w => Operand::Register(*src),
            PackedOp::ReadDstConst { c, .. } if c.width == w => Operand::Immediate(c.bits),
            _ => return None,
        };
        if dst.width != w {
            return None;
        }
        let Operation::Pil {
            program,
            flow,
            address: None,
            ..
        } = &branch.operation
        else {
            return None;
        };
        let [PackedOp::Jcc16 { cond, w }, PackedOp::Token(Token::Retire)] =
            program.packed.as_slice()
        else {
            return None;
        };
        let taken = flow.next_eip.wrapping_add_signed(flow.displacement) & crate::mask(*w) as u32;
        // Leave a potentially faulting branch to the individual instruction path.
        if taken > flow.code_limit {
            return None;
        }
        Some(Self {
            left: *dst,
            right,
            kind,
            write_result,
            condition: *cond,
            taken,
            fallthrough: flow.next_eip,
        })
    }

    pub fn execute(&self, cpu: &mut Cpu) -> LazyFlags {
        let a = cpu.read(self.left).bits;
        let b = match self.right {
            Operand::Register(view) => cpu.read(view).bits,
            Operand::Immediate(value) => value,
        };
        let w = self.left.width;
        let (raw, mut flags) = match self.kind {
            AluKind::Sub { write_cf } => {
                let res = crate::value(a.wrapping_sub(b), w);
                (res, LazyFlags::Sub { a, b, r: res.bits, w, write_cf })
            }
            AluKind::Add { write_cf } => {
                let res = crate::value(a.wrapping_add(b), w);
                (res, LazyFlags::Add { a, b, r: res.bits, w, write_cf })
            }
            AluKind::And => {
                let res = crate::value(a & b, w);
                (res, LazyFlags::Logic { r: res.bits, w })
            }
            AluKind::Or => {
                let res = crate::value(a | b, w);
                (res, LazyFlags::Logic { r: res.bits, w })
            }
            AluKind::Xor => {
                let res = crate::value(a ^ b, w);
                (res, LazyFlags::Logic { r: res.bits, w })
            }
        };
        if self.write_result {
            cpu.write(self.left, raw);
        }
        let taken = flags.eval_jcc(self.condition, &mut cpu.eflags, &mut cpu.undefined_flags);
        cpu.eip = if taken { self.taken } else { self.fallthrough };
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        flow::FlowContext,
        guest::{Decoded, Operation},
        Program, Value,
    };
    use std::cell::OnceCell;
    use std::rc::Rc;

    fn make_pil_decoded(
        packed: Vec<PackedOp>,
        dst: View,
        src: View,
        next_eip: u32,
        displacement: i32,
        code_limit: u32,
    ) -> Rc<Decoded> {
        let program = Program {
            tokens: Vec::new(),
            packed,
            widths: Vec::new(),
            register_effects: 1,
            operand_width: dst.width,
            source_width: src.width,
            uses_flow: displacement != 0,
            uses_displacement: displacement != 0,
            uses_memory: false,
            has_read_address: false,
            flushes_lazy: false,
            partial: false,
            checkpoint_flags: false,
        };
        Rc::new(Decoded {
            label: "test".to_string(),
            length: 2,
            operation: Operation::Pil {
                program,
                dst,
                src,
                address: None,
                flow: FlowContext {
                    next_eip,
                    displacement,
                    displacement_bits: 32,
                    code_limit,
                },
            },
            native: OnceCell::new(),
        })
    }

    #[test]
    fn fuses_add_lazy_and_logic_lazy_with_jcc32() {
        let eax = View { register: 0, low: 0, width: 32 };
        let ecx = View { register: 1, low: 0, width: 32 };

        // AddLazy (write_cf: false) + Jcc16 (cond 5 = JNE, w = 32)
        let add_step = make_pil_decoded(
            vec![
                PackedOp::ReadDstConst {
                    a: 0,
                    b: 1,
                    w: 32,
                    c: Value { bits: 1, width: 32 },
                },
                PackedOp::AddLazy {
                    a: 0,
                    b: 1,
                    r: 2,
                    w: 32,
                    write_cf: false,
                },
                PackedOp::Token(Token::Write(2)),
                PackedOp::Token(Token::Retire),
            ],
            ecx,
            eax,
            0x1005,
            -0x10,
            0xFFFF,
        );
        let br_step = make_pil_decoded(
            vec![
                PackedOp::Jcc16 { cond: 5, w: 32 },
                PackedOp::Token(Token::Retire),
            ],
            ecx,
            eax,
            0x100A,
            -0x15,
            0xFFFF,
        );

        let fused = CompareBranch::suffix(&[add_step, br_step]).expect("AddLazy + Jcc32 should fuse");
        let mut cpu = Cpu::default();
        cpu.write(ecx, Value { bits: 0xFFFF_FFFF, width: 32 });
        let lazy = fused.execute(&mut cpu);
        assert_eq!(cpu.read(ecx).bits, 0);
        assert_eq!(cpu.eip, 0x100A);
        assert!(matches!(lazy, LazyFlags::Add { w: 32, write_cf: false, .. }));

        // LogicLazy (And) + Jcc16 (cond 4 = JE, w = 32)
        let logic_step = make_pil_decoded(
            vec![
                PackedOp::ReadDstSrc { a: 0, b: 1, w: 32 },
                PackedOp::Token(Token::Binary(2, Binary::And, 0, 1)),
                PackedOp::LogicLazy { r: 2, w: 32 },
                PackedOp::Token(Token::Retire),
            ],
            eax,
            eax,
            0x2004,
            0x20,
            0xFFFF,
        );
        let je_step = make_pil_decoded(
            vec![
                PackedOp::Jcc16 { cond: 4, w: 32 },
                PackedOp::Token(Token::Retire),
            ],
            eax,
            eax,
            0x200A,
            0x20,
            0xFFFF,
        );
        let fused_logic =
            CompareBranch::suffix(&[logic_step, je_step]).expect("LogicLazy + Jcc32 should fuse");
        let mut cpu2 = Cpu::default();
        cpu2.write(eax, Value { bits: 0, width: 32 });
        let lazy2 = fused_logic.execute(&mut cpu2);
        assert_eq!(cpu2.eip, 0x202A);
        assert!(matches!(lazy2, LazyFlags::Logic { r: 0, w: 32 }));
    }
}