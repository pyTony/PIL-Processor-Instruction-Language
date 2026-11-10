//! First explicitly selectable 80387 backend: register-stack instructions.
//! The absent device remains the default. This backend deliberately rejects
//! memory operands and arithmetic until their exception/rounding semantics exist.
use super::{classify, Cpu, Extended80, Instruction, Operation, Outcome, Ram, StackError, X87State};
use crate::guest::Image;

#[derive(Clone, Debug, Default)]
pub struct PresentFpu {
    pub state: X87State,
}
impl PresentFpu {
    pub fn execute(&mut self, instruction: &Instruction, cpu: &mut Cpu,
                   _ram: &mut Ram, image: &Image) -> Outcome {
        // The CPU owns #NM delivery. EM or TS forces #NM on ESC even with a
        // physically installed 80387; WAIT traps only if MP and TS are set.
        let msw = image.msw;
        let trap = if instruction.opcode == 0x9b {
            msw & 0x0a == 0x0a
        } else {
            msw & 0x0c != 0
        };
        if trap { return Outcome::NotAvailable; }
        if instruction.opcode == 0x9b { return Outcome::Completed; }

        let Some(modrm) = instruction.modrm else { return Outcome::Unsupported; };
        let Ok(operation) = classify(instruction.opcode, modrm) else {
            return Outcome::Unsupported;
        };
        let one = Extended80::from_bits(0x8000_0000_0000_0000, 0x3fff);
        let result: Result<(), StackError> = match operation {
            Operation::Finit => { self.state.finit(); Ok(()) },
            Operation::Fnclex => { self.state.fnclex(); Ok(()) },
            Operation::FnstswAx => {
                cpu.regs[0] = (cpu.regs[0] & 0xffff_0000) | u32::from(self.state.status);
                Ok(())
            },
            Operation::Fld1 => self.state.push(one),
            Operation::Fldz => self.state.push(Extended80::default()),
            Operation::FldSt(i) => self.state.st(usize::from(i))
                .ok_or(StackError::Underflow)
                .and_then(|value| self.state.push(value)),
            Operation::Fxch(i) => self.state.fxch(usize::from(i)),
            Operation::FstpSt(i) => {
                // FSTP permits an empty destination register. The source ST0
                // must exist. No modifications occur on an empty ST0.
                self.state.st(0).ok_or(StackError::Underflow)
                    .and_then(|value| {
                        self.state.store(usize::from(i), value);
                        self.state.pop().map(|_| ())
                    })
            },
            // These require effective-address and IEEE conversion handling.
            // Never silently retire them without executing their memory effects.
            Operation::FldM32 | Operation::FstpM32 |
            Operation::FldM64 | Operation::FstpM64 => return Outcome::Unsupported,
        };
        match result {
            Ok(()) => Outcome::Completed,
            Err(error) => Outcome::StackFault(error),
        }
    }
}
