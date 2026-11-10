//! Architectural 80387 register state. No host floating-point arithmetic.
//! This module is deliberately independent of DOS and the x86 instruction decoder.
//! Registers retain all 80 bits, including the explicit integer bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Extended80 {
    pub significand: u64,
    pub sign_exponent: u16,
}
impl Extended80 {
    pub const fn from_bits(significand: u64, sign_exponent: u16) -> Self {
        Self { significand, sign_exponent }
    }
    pub fn from_le_bytes(bytes: [u8; 10]) -> Self {
        let mut sig = [0; 8];
        sig.copy_from_slice(&bytes[..8]);
        Self { significand: u64::from_le_bytes(sig),
               sign_exponent: u16::from_le_bytes([bytes[8], bytes[9]]) }
    }
    pub fn to_le_bytes(self) -> [u8; 10] {
        let mut result = [0; 10];
        result[..8].copy_from_slice(&self.significand.to_le_bytes());
        result[8..].copy_from_slice(&self.sign_exponent.to_le_bytes());
        result
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Tag { Valid = 0, Zero = 1, Special = 2, Empty = 3 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackError { Overflow, Underflow }
/// Logical ST(i) maps to physical register (TOP+i) modulo eight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X87State {
    registers: [Extended80; 8],
    tags: [Tag; 8],
    pub control: u16,
    pub status: u16,
    pub instruction_pointer: u32,
    pub instruction_selector: u16,
    pub operand_pointer: u32,
    pub operand_selector: u16,
    pub last_opcode: u16,
}
impl Default for X87State {
    fn default() -> Self { Self::new() }
}
impl X87State {
    pub fn new() -> Self {
        Self { registers: [Extended80::default(); 8], tags: [Tag::Empty; 8],
               control: 0x037f, status: 0, instruction_pointer: 0,
               instruction_selector: 0, operand_pointer: 0,
               operand_selector: 0, last_opcode: 0 }
    }
    pub fn finit(&mut self) { *self = Self::new(); }
    pub fn fnclex(&mut self) {
        // Clear busy, exception summary, stack fault and exception flags.
        self.status &= !0x80ff;
    }
    pub fn top(&self) -> usize { ((self.status >> 11) & 7) as usize }
    fn physical(&self, logical: usize) -> usize { (self.top() + logical) & 7 }
    fn set_top(&mut self, top: usize) {
        self.status = (self.status & !(7 << 11)) | (((top & 7) as u16) << 11);
    }
    pub fn tag(&self, logical: usize) -> Tag { self.tags[self.physical(logical)] }
    pub fn st(&self, logical: usize) -> Option<Extended80> {
        let p = self.physical(logical);
        if self.tags[p] == Tag::Empty { None } else { Some(self.registers[p]) }
    }
    pub fn tag_word(&self) -> u16 {
        self.tags.iter().enumerate().fold(0u16, |w, (i, tag)| w | ((*tag as u16) << (i * 2)))
    }
    fn classify(value: Extended80) -> Tag {
        let exponent = value.sign_exponent & 0x7fff;
        if exponent == 0 && value.significand == 0 { Tag::Zero }
        else if exponent == 0 || exponent == 0x7fff ||
                value.significand & (1u64 << 63) == 0 { Tag::Special }
        else { Tag::Valid }
    }
    pub fn push(&mut self, value: Extended80) -> Result<(), StackError> {
        let next = (self.top() + 7) & 7;
        if self.tags[next] != Tag::Empty { return Err(StackError::Overflow); }
        self.set_top(next);
        self.registers[next] = value;
        self.tags[next] = Self::classify(value);
        Ok(())
    }
    pub fn pop(&mut self) -> Result<Extended80, StackError> {
        let top = self.top();
        if self.tags[top] == Tag::Empty { return Err(StackError::Underflow); }
        let value = self.registers[top];
        self.tags[top] = Tag::Empty;
        self.set_top((top + 1) & 7);
        Ok(value)
    }
    pub fn fxch(&mut self, logical: usize) -> Result<(), StackError> {
        let a = self.top();
        let b = self.physical(logical);
        if self.tags[a] == Tag::Empty || self.tags[b] == Tag::Empty {
            return Err(StackError::Underflow);
        }
        self.registers.swap(a, b);
        self.tags.swap(a, b);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn one() -> Extended80 { Extended80::from_bits(0x8000_0000_0000_0000, 0x3fff) }
    #[test]
    fn reset_and_clear_exceptions() {
        let mut s = X87State::new();
        assert_eq!(s.control, 0x037f);
        assert_eq!(s.tag_word(), 0xffff);
        s.status = 0xffff;
        s.fnclex();
        assert_eq!(s.status & 0x80ff, 0);
        s.finit();
        assert_eq!(s, X87State::new());
    }
    #[test]
    fn push_pop_wrap_and_overflow_are_atomic() {
        let mut s = X87State::new();
        for _ in 0..8 { s.push(one()).unwrap(); }
        assert_eq!(s.top(), 0);
        let before = s.clone();
        assert_eq!(s.push(one()), Err(StackError::Overflow));
        assert_eq!(s, before);
        for _ in 0..8 { assert_eq!(s.pop(), Ok(one())); }
        assert_eq!(s.top(), 0);
        assert_eq!(s.pop(), Err(StackError::Underflow));
        assert_eq!(s.tag_word(), 0xffff);
    }
    #[test]
    fn physical_tag_word_and_exchange() {
        let mut s = X87State::new();
        let zero = Extended80::default();
        s.push(one()).unwrap();
        s.push(zero).unwrap();
        assert_eq!(s.tag(0), Tag::Zero);
        assert_eq!(s.tag(1), Tag::Valid);
        let word = s.tag_word();
        s.fxch(1).unwrap();
        assert_eq!(s.st(0), Some(one()));
        assert_eq!(s.st(1), Some(zero));
        assert_ne!(s.tag_word(), word);
    }
    #[test]
    fn raw_extended_precision_roundtrip() {
        let bytes = [1,2,3,4,5,6,7,8,9,10];
        assert_eq!(Extended80::from_le_bytes(bytes).to_le_bytes(), bytes);
    }
}
