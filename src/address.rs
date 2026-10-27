//! Decoded 386 addresses and already-loaded segment caches. No byte decoder.
use super::{ExecError, MemWidth};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gpr { Eax, Ecx, Edx, Ebx, Esp, Ebp, Esi, Edi }
impl Gpr {
    fn index(self) -> usize { self as usize }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentRegister { Es, Cs, Ss, Ds, Fs, Gs }
impl SegmentRegister {
    pub fn index(self) -> usize { self as usize }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Address16 { BxSi, BxDi, BpSi, BpDi, Si, Di, Bp, Bx, Direct }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveAddress {
    Bits16 { form: Address16, displacement: u16 },
    Bits32 { base: Option<Gpr>, index: Option<Gpr>, scale: u8, displacement: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedAddress { pub offset: u32, pub segment: SegmentRegister }
impl EffectiveAddress {
    // Displacements have already been sign-extended (disp8) and normalized to
    // address size by the caller. Address size is independent of operand size.
    pub fn resolve(self, regs: &[u32;8], override_segment: Option<SegmentRegister>) -> Result<ResolvedAddress, ExecError> {
        let (offset, stack) = match self {
            Self::Bits16 { form, displacement } => {
                let (base, index) = match form {
                    Address16::BxSi => (Some(Gpr::Ebx), Some(Gpr::Esi)),
                    Address16::BxDi => (Some(Gpr::Ebx), Some(Gpr::Edi)),
                    Address16::BpSi => (Some(Gpr::Ebp), Some(Gpr::Esi)),
                    Address16::BpDi => (Some(Gpr::Ebp), Some(Gpr::Edi)),
                    Address16::Si => (None, Some(Gpr::Esi)),
                    Address16::Di => (None, Some(Gpr::Edi)),
                    Address16::Bp => (Some(Gpr::Ebp), None),
                    Address16::Bx => (Some(Gpr::Ebx), None),
                    Address16::Direct => (None, None),
                };
                let sum = base.map_or(0, |r|regs[r.index()]).wrapping_add(index.map_or(0, |r|regs[r.index()])).wrapping_add(u32::from(displacement));
                (sum & 0xffff, base == Some(Gpr::Ebp))
            }
            Self::Bits32 { base, index, scale, displacement } => {
                if ![1,2,4,8].contains(&scale) || index == Some(Gpr::Esp) || (index.is_none() && scale != 1) {
                    return Err(ExecError::InvalidContext);
                }
                let offset = base.map_or(0, |r|regs[r.index()]).wrapping_add(index.map_or(0, |r|regs[r.index()]).wrapping_mul(u32::from(scale))).wrapping_add(displacement);
                (offset, matches!(base, Some(Gpr::Ebp | Gpr::Esp)))
            }
        };
        Ok(ResolvedAddress { offset, segment: override_segment.unwrap_or(if stack {SegmentRegister::Ss} else {SegmentRegister::Ds}) })
    }
}

// These are normalized hidden-cache attributes, not raw GDT/LDT descriptors.
// Caller has already performed descriptor-load, present and privilege checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentAccess {
    // Supplied by callers/tests; the current monitor creates usable caches only.
    #[allow(dead_code)]
    Unusable,
    Data { writable: bool, expand_down: bool, big: bool },
    Code { readable: bool },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentCache { pub base: u32, pub limit: u32, pub access: SegmentAccess }
impl SegmentCache {
    // Ordinary real-mode cache after a segment load; does not model transitions
    // from protected mode with retained hidden caches ("unreal mode").
    pub fn real_mode(selector: u16) -> Self {
        Self { base: u32::from(selector) << 4, limit: 0xffff,
            access: SegmentAccess::Data { writable: true, expand_down: false, big: false } }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryContext { pub segment: SegmentRegister, pub cache: SegmentCache }
impl MemoryContext {
    pub fn translate(self, offset: u32, width: MemWidth, write: bool, eip: u32) -> Result<u32, ExecError> {
        // An invalid loaded SS cache is a caller error: loading SS is not yet
        // implemented and would have rejected null/code/read-only descriptors.
        if self.segment == SegmentRegister::Ss && !matches!(self.cache.access, SegmentAccess::Data { writable: true, .. }) {
            return Err(ExecError::InvalidContext);
        }
        let gp = ExecError::GeneralProtection { eip, error_code: 0 };
        let (permitted, expand_down, big) = match self.cache.access {
            SegmentAccess::Unusable => return Err(gp),
            SegmentAccess::Data { writable, expand_down, big } => (!write || writable, expand_down, big),
            SegmentAccess::Code { readable } => (!write && readable, false, false),
        };
        if !permitted { return Err(gp); }
        // Use a widened end: a multi-byte operand cannot wrap its segment offset.
        let end = u64::from(offset) + u64::from(width.bytes()) - 1;
        let in_bounds = if expand_down {
            offset > self.cache.limit && end <= if big {u64::from(u32::MAX)} else {0xffff}
        } else { end <= u64::from(self.cache.limit) };
        if !in_bounds {
            return Err(if self.segment == SegmentRegister::Ss {
                ExecError::StackSegment { eip, error_code: 0 }
            } else { gp });
        }
        Ok(self.cache.base.wrapping_add(offset))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddressContext {
    pub effective: EffectiveAddress,
    pub override_segment: Option<SegmentRegister>,
    // ES, CS, SS, DS, FS, GS, in SegmentRegister order.
    pub segments: [SegmentCache;6],
}
impl AddressContext {
    pub fn resolve(self, regs: &[u32;8]) -> Result<(u32, MemoryContext), ExecError> {
        let address = self.effective.resolve(regs, self.override_segment)?;
        Ok((address.offset, MemoryContext { segment: address.segment, cache: self.segments[address.segment.index()] }))
    }
}

// Mutable architectural segment state. Only ordinary real-mode loads are admitted.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SegmentState {
    pub segments:[SegmentCache;6],pub selectors:[u16;6],
    pub real_mode:bool,pub ss_shadow:u8,
}
impl SegmentState {
    pub fn retired(&mut self){self.ss_shadow=self.ss_shadow.saturating_sub(1);}
}

#[cfg(test)]
#[path = "../tests/unit/address.rs"]
mod tests;
