//! Partial opt-in 80387 backend: register stack, IEEE memory loads and stores.
//! Unsupported encodings, unmodelled exception/rounding behaviours, and
//! arithmetic stop explicitly instead of fabricating successful execution.
use super::{address, classify, convert, Cpu, Extended80, Instruction, Operation, Outcome, Ram, StackError, X87State};
use crate::{guest::Image, memory::MemWidth};

#[derive(Clone, Debug, Default)]
pub struct PresentFpu {
    pub state: X87State,
}
impl PresentFpu {
    pub fn execute(&mut self, instruction: &Instruction, cpu: &mut Cpu,
                   ram: &mut Ram, image: &Image) -> Outcome {
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
            Operation::FldM32 | Operation::FstpM32 |
            Operation::FldM64 | Operation::FstpM64 =>
                return self.execute_memory(operation,instruction,cpu,ram,image),
        };
        match result {
            Ok(()) => Outcome::Completed,
            Err(error) => Outcome::StackFault(error),
        }
    }

    /// Load/store one IEEE operand. Segment permission and complete span are
    /// checked before RAM access; FSTP commits a pop only after a successful write.
    fn execute_memory(&mut self,operation:Operation,instruction:&Instruction,
                      cpu:&Cpu,ram:&mut Ram,image:&Image)->Outcome {
        let write=matches!(operation,Operation::FstpM32|Operation::FstpM64);
        let bytes: u32=if matches!(operation,Operation::FldM32|Operation::FstpM32){4}else{8};
        let Ok((offset,context))=address::resolve(instruction,cpu,image) else {
            return Outcome::MemoryFault;
        };
        let Ok(linear)=context.translate_span(u64::from(offset),bytes,write,cpu.eip) else {
            return Outcome::MemoryFault;
        };
        let Some(high)=linear.checked_add(4) else {return Outcome::MemoryFault;};
        if !write {
            let Ok(low)=ram.load(linear,MemWidth::Dword) else {
                return Outcome::MemoryFault;
            };
            let converted=if bytes==4 {
                convert::from_f32_bits(low)
            }else{
                let Ok(upper)=ram.load(high,MemWidth::Dword) else {
                    return Outcome::MemoryFault;
                };
                convert::from_f64_bits(u64::from(low)|(u64::from(upper)<<32))
            };
            return match converted {
                Ok(value)=>match self.state.push(value){
                    Ok(())=>Outcome::Completed,
                    Err(err)=>Outcome::StackFault(err),
                },
                Err(_)=>Outcome::Unsupported,
            };
        }
        let Some(value)=self.state.st(0) else {
            return Outcome::StackFault(StackError::Underflow);
        };
        let converted=if bytes==4{
            convert::to_f32_bits(value,self.state.control)
        }else{
            convert::to_f64_bits(value,self.state.control)
        };
        let Ok(result)=converted else {return Outcome::Unsupported;};
        // Correct unmasked exception delivery (#MF) has not been implemented.
        // Refuse this case rather than retiring an instruction spuriously.
        let exceptions=u16::from(result.inexact)<<5 | u16::from(result.underflow)<<4;
        if exceptions & !self.state.control & 0x003f != 0 {
            return Outcome::Unsupported;
        }
        // Check every destination byte first. Partial writes must never be
        // reported as successful stores or pop an x87 register.
        for i in 0..bytes {
            let Some(address)=linear.checked_add(i) else {return Outcome::MemoryFault;};
            if ram.check_write(address,MemWidth::Byte).is_err(){
                return Outcome::MemoryFault;
            }
        }
        if ram.store(linear,MemWidth::Dword,result.bits as u32).is_err(){
            return Outcome::MemoryFault;
        }
        if bytes==8&&ram.store(high,MemWidth::Dword,(result.bits>>32)as u32).is_err(){
            return Outcome::MemoryFault;
        }
        self.state.status|=exceptions;
        self.state.pop().expect("FSTP requires checked ST(0)");
        Outcome::Completed
    }
}
