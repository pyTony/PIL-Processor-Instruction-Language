//! Finite, normalized x87 register arithmetic without host floating point.
//! Phase 3 deliberately supports the default 64-bit precision and
//! round-to-nearest-even only. NaNs, infinities, denormals, exceptional
//! exponent ranges and other control-word modes must not execute silently.
use super::{Arithmetic, Extended80};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathError { Unsupported }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Computed {
    pub value: Extended80,
    pub inexact: bool,
    pub rounded_up: bool,
}

fn split(value: Extended80) -> Result<(bool, i32, u64), MathError> {
    let sign = value.sign_exponent & 0x8000 != 0;
    let exp = i32::from(value.sign_exponent & 0x7fff);
    if exp == 0 && value.significand == 0 {
        return Ok((sign, 0, 0));
    }
    if exp == 0 || exp == 0x7fff || value.significand & (1u64 << 63) == 0 {
        return Err(MathError::Unsupported);
    }
    Ok((sign, exp, value.significand))
}

fn exact(value: Extended80) -> Computed {
    Computed { value, inexact: false, rounded_up: false }
}

fn signed_zero(negative: bool) -> Computed {
    exact(Extended80::from_bits(0, if negative { 0x8000 } else { 0 }))
}

/// Preserve a sticky bit when aligning significands; shifts can span the
/// entire 80387 exponent range, far beyond the host integer width.
fn shift_jam(value: u128, shift: u32) -> u128 {
    if shift == 0 { return value; }
    if shift >= 128 { return u128::from(value != 0); }
    (value >> shift) | u128::from(value & ((1u128 << shift) - 1) != 0)
}

fn finish(negative: bool, mut exponent: i32, mut significand: u64,
          inexact: bool, rounded_up: bool) -> Result<Computed, MathError> {
    if rounded_up {
        let (value, carry) = significand.overflowing_add(1);
        significand = if carry { 1u64 << 63 } else { value };
        if carry { exponent += 1; }
    }
    // Underflow/overflow and denormal handling require real x87 exception
    // semantics. Do not modify the destination for these cases.
    if !(1..0x7fff).contains(&exponent) {
        return Err(MathError::Unsupported);
    }
    Ok(Computed {
        value: Extended80::from_bits(significand,
            exponent as u16 | if negative { 0x8000 } else { 0 }),
        inexact,
        rounded_up,
    })
}

/// Compute a op b. The caller checks precision exception masks before
/// committing either the destination or the x87 status word.
pub fn calculate(operation: Arithmetic, a: Extended80, b: Extended80,
                 control: u16) -> Result<Computed, MathError> {
    // PC=11 (64-bit) and RC=00 (nearest-even) only.
    if control & 0x0f00 != 0x0300 { return Err(MathError::Unsupported); }
    let (a, b, subtract, multiply) = match operation {
        Arithmetic::Add => (a, b, false, false),
        Arithmetic::Multiply => (a, b, false, true),
        Arithmetic::Subtract => (a, b, true, false),
        Arithmetic::ReverseSubtract => (b, a, true, false),
    };
    let (asign, ae, av) = split(a)?;
    let (bsign, be, bv) = split(b)?;
    if multiply {
        let negative = asign ^ bsign;
        if av == 0 || bv == 0 { return Ok(signed_zero(negative)); }
        let product = u128::from(av) * u128::from(bv);
        let shift = if product & (1u128 << 127) != 0 { 64 } else { 63 };
        let exponent = ae + be - 16383 + if shift == 64 { 1 } else { 0 };
        let significand = (product >> shift) as u64;
        let discarded = product & ((1u128 << shift) - 1);
        let midpoint = 1u128 << (shift - 1);
        let rounded_up = discarded > midpoint ||
            (discarded == midpoint && significand & 1 != 0);
        return finish(negative, exponent, significand,
                      discarded != 0, rounded_up);
    }

    let bsign = bsign ^ subtract;
    if av == 0 && bv == 0 {
        // Nearest-even cancellation yields +0; -0 + -0 remains -0.
        return Ok(signed_zero(asign && bsign));
    }
    if av == 0 {
        return Ok(exact(Extended80::from_bits(bv,
            be as u16 | if bsign { 0x8000 } else { 0 })));
    }
    if bv == 0 {
        return Ok(exact(Extended80::from_bits(av,
            ae as u16 | if asign { 0x8000 } else { 0 })));
    }

    let (large_e, large, large_sign, small_e, small, small_sign) =
        if ae >= be {
            (ae, av, asign, be, bv, bsign)
        } else {
            (be, bv, bsign, ae, av, asign)
        };
    let left = u128::from(large) << 3;
    let right = shift_jam(u128::from(small) << 3, (large_e - small_e) as u32);
    let mut exponent = large_e;
    let (mut magnitude, negative) = if large_sign == small_sign {
        (left + right, large_sign)
    } else if left >= right {
        (left - right, large_sign)
    } else {
        (right - left, small_sign)
    };
    if magnitude == 0 { return Ok(signed_zero(false)); }
    // The hidden integer bit is at position 66 with three guard/round/sticky
    // bits below it. A carry out of bit 66 increases the exponent.
    if magnitude >= (1u128 << 67) {
        magnitude = shift_jam(magnitude, 1);
        exponent += 1;
    }
    while magnitude < (1u128 << 66) {
        magnitude <<= 1;
        exponent -= 1;
        if exponent <= 0 { return Err(MathError::Unsupported); }
    }
    let significand = (magnitude >> 3) as u64;
    let discarded = magnitude & 7;
    let rounded_up = discarded > 4 ||
        (discarded == 4 && significand & 1 != 0);
    finish(negative, exponent, significand, discarded != 0, rounded_up)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ONE: Extended80 = Extended80::from_bits(1u64 << 63, 0x3fff);
    const TWO: Extended80 = Extended80::from_bits(1u64 << 63, 0x4000);
    const ZERO: Extended80 = Extended80::from_bits(0, 0);
    const CW: u16 = 0x037f;

    #[test]
    fn finite_add_subtract_and_cancellation() {
        let r = calculate(Arithmetic::Add, ONE, ONE, CW).unwrap();
        assert_eq!(r.value, TWO);
        assert!(!r.inexact);
        assert_eq!(calculate(Arithmetic::Subtract, TWO, ONE, CW).unwrap().value, ONE);
        assert_eq!(calculate(Arithmetic::Subtract, ONE, TWO, CW).unwrap().value,
            Extended80::from_bits(1u64 << 63, 0xbfff));
        assert_eq!(calculate(Arithmetic::ReverseSubtract, ONE, TWO, CW).unwrap().value, ONE);
        assert_eq!(calculate(Arithmetic::Subtract, ONE, ONE, CW).unwrap().value, ZERO);
    }

    #[test]
    fn multiplication_uses_full_128_bit_product() {
        let half_again = Extended80::from_bits(0xc000_0000_0000_0000, 0x3fff);
        assert_eq!(calculate(Arithmetic::Multiply, half_again, half_again, CW).unwrap().value,
            Extended80::from_bits(0x9000_0000_0000_0000, 0x4000));
        assert_eq!(calculate(Arithmetic::Multiply, ONE, TWO, CW).unwrap().value, TWO);
        assert_eq!(calculate(Arithmetic::Multiply,
                   Extended80::from_bits(0, 0x8000), ONE, CW).unwrap().value,
                   Extended80::from_bits(0, 0x8000));
    }

    #[test]
    fn guard_round_sticky_ties_nearest_even() {
        let ulp = Extended80::from_bits(1u64 << 63, 0x3fff - 63);
        let half_ulp = Extended80::from_bits(1u64 << 63, 0x3fff - 64);
        assert_eq!(calculate(Arithmetic::Add, ONE, ulp, CW).unwrap().value.significand,
                   (1u64 << 63) + 1);
        let tie = calculate(Arithmetic::Add, ONE, half_ulp, CW).unwrap();
        assert_eq!(tie.value, ONE);
        assert!(tie.inexact);
        assert!(!tie.rounded_up);
        let minus_tiny = calculate(Arithmetic::Subtract, ONE,
            Extended80::from_bits(1u64 << 63, 0x3fff - 100), CW).unwrap();
        assert_eq!(minus_tiny.value, ONE);
        assert!(minus_tiny.inexact);
        assert!(minus_tiny.rounded_up);
    }

    #[test]
    fn unsupported_cases_cannot_fabricate_results() {
        assert_eq!(calculate(Arithmetic::Add, ONE, ONE, 0x027f),
                   Err(MathError::Unsupported)); // 53-bit precision
        assert_eq!(calculate(Arithmetic::Add, ONE, ONE, 0x0b7f),
                   Err(MathError::Unsupported)); // directed rounding
        assert_eq!(calculate(Arithmetic::Add, ONE,
                   Extended80::from_bits(1u64 << 63, 0x7fff), CW),
                   Err(MathError::Unsupported)); // infinity
        assert_eq!(calculate(Arithmetic::Multiply,
                   Extended80::from_bits(1u64 << 63, 1),
                   Extended80::from_bits(1u64 << 63, 0x3ffe), CW),
                   Err(MathError::Unsupported)); // underflow
    }
}
