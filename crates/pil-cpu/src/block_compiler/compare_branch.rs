use crate::{
    guest::{Decoded, Operation},
    Cpu, LazyFlags, PackedOp, Token, View,
};
use std::rc::Rc;

enum Operand {
    Register(View),
    Immediate(u64),
}

/// Two validated PIL operations with no observable boundary between them.
pub struct CompareBranch {
    left: View,
    right: Operand,
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
        let [read, PackedOp::SubLazy {
            a,
            b,
            r,
            w,
            write_cf: true,
        }, tail @ ..] = program.packed.as_slice()
        else {
            return None;
        };
        let write_result = match tail {
            [PackedOp::Token(Token::Retire)] => false,
            [PackedOp::Token(Token::Write(slot)), PackedOp::Token(Token::Retire)] if slot == r => {
                true
            }
            _ => return None,
        };
        let right = match read {
            PackedOp::ReadDstSrc {
                a: x,
                b: y,
                w: width,
            } if (x, y, width) == (a, b, w) && src.width == *w => Operand::Register(*src),
            PackedOp::ReadDstConst {
                a: x,
                b: y,
                w: width,
                c,
            } if (x, y, width) == (a, b, w) && c.width == *w => Operand::Immediate(c.bits),
            _ => return None,
        };
        if dst.width != *w {
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
        let result = crate::value(a.wrapping_sub(b), self.left.width);
        if self.write_result {
            cpu.write(self.left, result);
        }
        let mut flags = LazyFlags::Sub {
            a,
            b,
            r: result.bits,
            w: self.left.width,
            write_cf: true,
        };
        let taken = flags.eval_jcc(self.condition, &mut cpu.eflags, &mut cpu.undefined_flags);
        cpu.eip = if taken { self.taken } else { self.fallthrough };
        flags
    }
}
