//! Context supplied by a future decoder; no instruction fetch is performed.
#[derive(Clone, Copy, Debug)]
pub struct FlowContext {
    pub next_eip: u32,
    pub displacement: i32,
    pub displacement_bits: u8,
    pub code_limit: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecError {
    InvalidBinding,
    InvalidContext,
    GeneralProtection { eip: u32, error_code: u16 },
}

impl FlowContext {
    pub fn validate(self, operand_width: u8, uses_displacement: bool) -> Result<(), ExecError> {
        if uses_displacement {
            let fits = match self.displacement_bits {
                8 => (-128..=127).contains(&self.displacement),
                16 => (-32768..=32767).contains(&self.displacement),
                32 => true,
                _ => false,
            };
            if !fits || (self.displacement_bits != 8 && self.displacement_bits != operand_width) {
                return Err(ExecError::InvalidContext);
            }
        }
        Ok(())
    }
}
