//! Partial opt-in 80387 backend: register stack, IEEE loads/stores,
//! and finite register/memory arithmetic. Unsupported exceptional/rounding
//! behaviours stop explicitly rather than fabricating successful execution.
use super::{address, classify, convert, math, Arithmetic, Cpu, Extended80, Instruction, Operation, Outcome, Ram, StackError, X87State};
use crate::{guest::Image, memory::MemWidth};
use std::cmp::Ordering;

/// FLD/FSTP m80 is an exact, non-rounded transfer. Do not treat pseudo-
/// denormals, unnormals or NaNs as ordinary numbers until their exceptional
/// x87 behaviour is modelled. Signed zero and infinities have exact bits.
fn supported_m80(value: Extended80) -> bool {
    let exp=value.sign_exponent&0x7fff;
    (exp==0&&value.significand==0) ||
    (exp>0&&exp<0x7fff&&value.significand&(1u64<<63)!=0) ||
    (exp==0x7fff&&value.significand==1u64<<63)
}


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
            Operation::Fldcw | Operation::Fnstcw =>
                return self.execute_control_word_memory(operation,instruction,cpu,ram,image),
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
            Operation::Arithmetic { kind, index, pop } =>
                return self.execute_arithmetic(kind, index, pop),
            Operation::MemoryArithmetic { kind, bytes } =>
                return self.execute_arithmetic_memory(kind, bytes, instruction, cpu, ram, image),
            Operation::Compare { index, pop } =>
                return self.compare_register(index, if pop {1}else{0}),
            Operation::ComparePop2 =>
                return self.compare_register(1, 2),
            Operation::CompareMemory { bytes, pop } =>
                return self.compare_memory(bytes, pop, instruction, cpu, ram, image),
            Operation::FldM32 | Operation::FstpM32 |
            Operation::FldM64 | Operation::FstpM64 |
            Operation::FldM80 | Operation::FstpM80 =>
                return self.execute_memory(operation,instruction,cpu,ram,image),
        };
        match result {
            Ok(()) => Outcome::Completed,
            Err(error) => Outcome::StackFault(error),
        }
    }



    /// FLDCW/FNSTCW use the same segment and effective-address model as all
    /// other x87 memory operands. An unsuccessful read/write cannot change the
    /// control word, RAM or instruction retirement state.
    fn execute_control_word_memory(&mut self, operation:Operation,
                                   instruction:&Instruction, cpu:&Cpu,
                                   ram:&mut Ram, image:&Image) -> Outcome {
        let write=matches!(operation,Operation::Fnstcw);
        let Ok((offset,context))=address::resolve(instruction,cpu,image) else {
            return Outcome::MemoryFault;
        };
        let Ok(linear)=context.translate_span(u64::from(offset),2,write,cpu.eip) else {
            return Outcome::MemoryFault;
        };
        if write {
            if ram.check_write(linear,MemWidth::Word).is_err() {
                return Outcome::MemoryFault;
            }
            if ram.store(linear,MemWidth::Word,u32::from(self.state.control)).is_err() {
                return Outcome::MemoryFault;
            }
        } else {
            let Ok(control)=ram.load(linear,MemWidth::Word) else {
                return Outcome::MemoryFault;
            };
            // Preserve the guest control word. The finite arithmetic backend
            // still explicitly rejects unsupported precision/rounding modes.
            self.state.control=control as u16;
        }
        Outcome::Completed
    }

    /// Register arithmetic validates both operands before any state change.
    fn execute_arithmetic(&mut self, kind: Arithmetic, index: u8, pop: bool) -> Outcome {
        let dst = if pop { usize::from(index) } else { 0 };
        let src = if pop { 0 } else { usize::from(index) };
        let Some(lhs) = self.state.st(dst) else {
            return Outcome::StackFault(StackError::Underflow);
        };
        let Some(rhs) = self.state.st(src) else {
            return Outcome::StackFault(StackError::Underflow);
        };
        self.commit_arithmetic(kind, lhs, rhs, dst, pop)
    }

    /// Memory arithmetic uses the same effective-address and segment checks
    /// as FLD/FSTP, without writing RAM or changing the x87 stack until
    /// the entire IEEE source and arithmetic result are valid.
    fn execute_arithmetic_memory(&mut self, kind: Arithmetic, bytes: u32,
                                 instruction: &Instruction, cpu: &Cpu,
                                 ram: &mut Ram, image: &Image) -> Outcome {
        let Some(lhs) = self.state.st(0) else {
            return Outcome::StackFault(StackError::Underflow);
        };
        let Ok((offset, context)) = address::resolve(instruction, cpu, image) else {
            return Outcome::MemoryFault;
        };
        let Ok(linear) = context.translate_span(u64::from(offset), bytes, false, cpu.eip) else {
            return Outcome::MemoryFault;
        };
        let Ok(low) = ram.load(linear, MemWidth::Dword) else {
            return Outcome::MemoryFault;
        };
        let rhs = if bytes == 4 {
            convert::from_f32_bits(low)
        } else {
            let Some(high) = linear.checked_add(4) else {
                return Outcome::MemoryFault;
            };
            let Ok(upper) = ram.load(high, MemWidth::Dword) else {
                return Outcome::MemoryFault;
            };
            convert::from_f64_bits(u64::from(low) | (u64::from(upper) << 32))
        };
        let Ok(rhs) = rhs else { return Outcome::Unsupported; };
        self.commit_arithmetic(kind, lhs, rhs, 0, false)
    }

    /// Atomic retirement shared by register and memory arithmetic. For an
    /// unmasked PE, report an unsupported operation until #MF is implemented.
    fn commit_arithmetic(&mut self, kind: Arithmetic, lhs: Extended80,
                         rhs: Extended80, dst: usize, pop: bool) -> Outcome {
        let Ok(computed) = math::calculate(kind, lhs, rhs, self.state.control) else {
            return Outcome::Unsupported;
        };
        if computed.inexact && self.state.control & 0x0020 == 0 {
            return Outcome::Unsupported;
        }
        self.state.store(dst, computed.value);
        if pop { self.state.pop().expect("checked arithmetic ST(0)"); }
        // C1 describes upward rounding; PE records a masked inexact result.
        self.state.status &= !0x0200;
        if computed.rounded_up { self.state.status |= 0x0200; }
        if computed.inexact { self.state.status |= 0x0020; }
        Outcome::Completed
    }

    /// Comparisons change only C0/C1/C2/C3 and optionally pop the stack.
    /// Never change condition codes or TOP for missing operands/NaNs.
    fn commit_compare(&mut self, lhs:Extended80, rhs:Extended80,
                      pops:u8) -> Outcome {
        let Ok(order)=math::compare(lhs,rhs) else {
            return Outcome::Unsupported;
        };
        self.state.status &= !0x4700;
        match order {
            Ordering::Less=>self.state.status|=0x0100,
            Ordering::Equal=>self.state.status|=0x4000,
            Ordering::Greater=>{},
        }
        for _ in 0..pops {
            self.state.pop().expect("all comparison pops validated before commit");
        }
        Outcome::Completed
    }

    fn compare_register(&mut self, index:u8, pops:u8) -> Outcome {
        let Some(lhs)=self.state.st(0) else {
            return Outcome::StackFault(StackError::Underflow);
        };
        let Some(rhs)=self.state.st(usize::from(index)) else {
            return Outcome::StackFault(StackError::Underflow);
        };
        self.commit_compare(lhs,rhs,pops)
    }

    fn compare_memory(&mut self, bytes:u32, pop:bool,
                      instruction:&Instruction, cpu:&Cpu,
                      ram:&mut Ram, image:&Image)->Outcome {
        let Some(lhs)=self.state.st(0) else {
            return Outcome::StackFault(StackError::Underflow);
        };
        let Ok((offset,context))=address::resolve(instruction,cpu,image) else {
            return Outcome::MemoryFault;
        };
        let Ok(linear)=context.translate_span(u64::from(offset),bytes,false,cpu.eip) else {
            return Outcome::MemoryFault;
        };
        let Ok(low)=ram.load(linear,MemWidth::Dword) else {
            return Outcome::MemoryFault;
        };
        let rhs=if bytes==4{
            convert::from_f32_bits(low)
        }else{
            let Some(high)=linear.checked_add(4) else {
                return Outcome::MemoryFault;
            };
            let Ok(upper)=ram.load(high,MemWidth::Dword) else {
                return Outcome::MemoryFault;
            };
            convert::from_f64_bits(u64::from(low)|(u64::from(upper)<<32))
        };
        let Ok(rhs)=rhs else {return Outcome::Unsupported;};
        self.commit_compare(lhs,rhs,u8::from(pop))
    }

    /// Load/store IEEE m32/m64 or exact extended m80. Segment permission and complete span are
    /// checked before RAM access; FSTP commits a pop only after a successful write.
    fn execute_memory(&mut self,operation:Operation,instruction:&Instruction,
                      cpu:&Cpu,ram:&mut Ram,image:&Image)->Outcome {
        let write=matches!(operation,Operation::FstpM32|Operation::FstpM64|Operation::FstpM80);
        let bytes: u32=match operation {
            Operation::FldM32|Operation::FstpM32=>4,
            Operation::FldM64|Operation::FstpM64=>8,
            Operation::FldM80|Operation::FstpM80=>10,
            _=>return Outcome::Unsupported,
        };
        let Ok((offset,context))=address::resolve(instruction,cpu,image) else {
            return Outcome::MemoryFault;
        };
        let Ok(linear)=context.translate_span(u64::from(offset),bytes,write,cpu.eip) else {
            return Outcome::MemoryFault;
        };
        let Some(high)=linear.checked_add(4) else {return Outcome::MemoryFault;};
        if bytes==10 {
            let Some(top)=linear.checked_add(8) else {return Outcome::MemoryFault;};
            if !write {
                let Ok(lo)=ram.load(linear,MemWidth::Dword) else {
                    return Outcome::MemoryFault;
                };
                let Ok(hi)=ram.load(high,MemWidth::Dword) else {
                    return Outcome::MemoryFault;
                };
                let Ok(se)=ram.load(top,MemWidth::Word) else {
                    return Outcome::MemoryFault;
                };
                let value=Extended80::from_bits(u64::from(lo)|(u64::from(hi)<<32),se as u16);
                if !supported_m80(value){return Outcome::Unsupported;}
                return match self.state.push(value) {
                    Ok(())=>Outcome::Completed,
                    Err(e)=>Outcome::StackFault(e),
                };
            }
            let Some(value)=self.state.st(0) else {
                return Outcome::StackFault(StackError::Underflow);
            };
            if !supported_m80(value){return Outcome::Unsupported;}
            // Validate every byte of the 10-byte destination before the
            // first write, so protection failures never pop ST(0).
            for i in 0..10u32 {
                if ram.check_write(linear+i,MemWidth::Byte).is_err() {
                    return Outcome::MemoryFault;
                }
            }
            if ram.store(linear,MemWidth::Dword,value.significand as u32).is_err()
                ||ram.store(high,MemWidth::Dword,(value.significand>>32)as u32).is_err()
                ||ram.store(top,MemWidth::Word,u32::from(value.sign_exponent)).is_err() {
                return Outcome::MemoryFault;
            }
            self.state.pop().expect("FSTP m80 checked ST(0)");
            return Outcome::Completed;
        }
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
