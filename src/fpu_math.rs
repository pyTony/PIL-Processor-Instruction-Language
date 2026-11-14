//! Finite, normalized x87 register arithmetic without host floating point.
//! Phase 3 deliberately supports the default 64-bit precision and
//! round-to-nearest-even only. NaNs, infinities, denormals, exceptional
//! exponent ranges and other control-word modes must not execute silently.
use super::{Arithmetic, Extended80};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathError { Unsupported }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Computed {
    pub value: Extended80,
    pub inexact: bool,
    pub rounded_up: bool,
    pub zero_divide: bool,
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
    Computed { value, inexact: false, rounded_up: false, zero_divide: false }
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
          rem_nonzero: bool, rem_gt_half: bool, rem_eq_half: bool,
          drop_bits: u32) -> Result<Computed, MathError> {
    let (inexact, rounded_up) = if drop_bits == 0 {
        let up = rem_gt_half || (rem_eq_half && (significand & 1 != 0));
        if up {
            let (value, carry) = significand.overflowing_add(1);
            significand = if carry { 1u64 << 63 } else { value };
            if carry { exponent += 1; }
        }
        (rem_nonzero, up)
    } else {
        let low_mask = (1u64 << drop_bits) - 1;
        let low = significand & low_mask;
        let half = 1u64 << (drop_bits - 1);
        let sig_trunc = significand & !low_mask;
        let inexact = low != 0 || rem_nonzero;
        let up = low > half || (low == half && (rem_nonzero || ((sig_trunc >> drop_bits) & 1 != 0)));
        significand = sig_trunc;
        if up {
            let (value, carry) = significand.overflowing_add(1u64 << drop_bits);
            significand = if carry { 1u64 << 63 } else { value };
            if carry { exponent += 1; }
        }
        (inexact, up)
    };
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
        zero_divide: false,
    })
}

/// Compute a op b. The caller checks precision exception masks before
/// committing either the destination or the x87 status word.
pub fn calculate(operation: Arithmetic, a: Extended80, b: Extended80,
                 control: u16) -> Result<Computed, MathError> {
    // PC=11 (64-bit), PC=10 (53-bit), PC=00 (24-bit), and RC=00 (nearest-even).
    let drop_bits = match control & 0x0f00 {
        0x0300 => 0,
        0x0200 => 11,
        0x0000 => 40,
        _ => return Err(MathError::Unsupported),
    };
    let (a, b, subtract, multiply, divide) = match operation {
        Arithmetic::Add => (a, b, false, false, false),
        Arithmetic::Multiply => (a, b, false, true, false),
        Arithmetic::Subtract => (a, b, true, false, false),
        Arithmetic::ReverseSubtract => (b, a, true, false, false),
        Arithmetic::Divide => (a, b, false, false, true),
        Arithmetic::ReverseDivide => (b, a, false, false, true),
    };
    let (asign, ae, av) = split(a)?;
    let (bsign, be, bv) = split(b)?;
    if divide {
        // av and bv are normalized 64-bit integers. Long division in u128
        // yields the 64 significand bits and the exact remainder, which
        // suffices for correct nearest-even rounding without host floats.
        // 0/0 needs invalid-operation modelling; finite nonzero / 0 produces
        // signed infinity when ZE is masked in the caller.
        if av == 0 && bv == 0 { return Err(MathError::Unsupported); }
        if bv == 0 {
            return Ok(Computed {
                value: Extended80::from_bits(1u64 << 63,
                    0x7fff | if asign ^ bsign { 0x8000 } else { 0 }),
                inexact: false,
                rounded_up: false,
                zero_divide: true,
            });
        }
        if av == 0 { return Ok(signed_zero(asign ^ bsign)); }
        let low = av < bv;
        let exponent = ae - be + 16383 - i32::from(low);
        let numerator = u128::from(av) << if low { 64 } else { 63 };
        let q = numerator / u128::from(bv);
        let rem = numerator % u128::from(bv);
        debug_assert!((1u128 << 63..1u128 << 64).contains(&q));
        let doubled = rem * 2;
        return finish(asign ^ bsign, exponent, q as u64,
                      rem != 0, doubled > u128::from(bv), doubled == u128::from(bv), drop_bits);
    }
    if multiply {
        let negative = asign ^ bsign;
        if av == 0 || bv == 0 { return Ok(signed_zero(negative)); }
        let product = u128::from(av) * u128::from(bv);
        let shift = if product & (1u128 << 127) != 0 { 64 } else { 63 };
        let exponent = ae + be - 16383 + if shift == 64 { 1 } else { 0 };
        let significand = (product >> shift) as u64;
        let discarded = product & ((1u128 << shift) - 1);
        let midpoint = 1u128 << (shift - 1);
        return finish(negative, exponent, significand,
                      discarded != 0, discarded > midpoint, discarded == midpoint, drop_bits);
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
    finish(negative, exponent, significand, discarded != 0, discarded > 4, discarded == 4, drop_bits)
}

fn split_comparable(value: Extended80) -> Result<(bool, i32, u64), MathError> {
    let sign = value.sign_exponent & 0x8000 != 0;
    let exp = i32::from(value.sign_exponent & 0x7fff);
    if exp == 0x7fff && value.significand == (1u64 << 63) {
        return Ok((sign, 0x7fff, value.significand));
    }
    split(value)
}

/// Ordered comparisons of finite normalized values and affine infinities.
/// Reject NaNs and unmodelled denormals rather than inventing x87 condition codes.
pub fn compare(a: Extended80, b: Extended80) -> Result<Ordering, MathError> {
    let (negative_a, exponent_a, significand_a) = split_comparable(a)?;
    let (negative_b, exponent_b, significand_b) = split_comparable(b)?;
    if significand_a == 0 && significand_b == 0 { return Ok(Ordering::Equal); }
    if negative_a != negative_b {
        return Ok(if negative_a { Ordering::Less } else { Ordering::Greater });
    }
    let mag = (exponent_a, significand_a).cmp(&(exponent_b, significand_b));
    Ok(if negative_a { mag.reverse() } else { mag })
}

fn int_to_ext80(value: i32) -> Extended80 {
    if value == 0 { return Extended80::default(); }
    let mag = value.unsigned_abs();
    let shift = mag.leading_zeros();
    let sig = u64::from(mag) << (shift + 32);
    let exp = (0x3fff + 31 - shift as i32) as u16 | if value < 0 { 0x8000 } else { 0 };
    Extended80::from_bits(sig, exp)
}

/// Compute `y * log2(1 + x)` for FYL2XP1 using pure integer x87 arithmetic.
pub fn fyl2xp1(y: Extended80, x: Extended80, control: u16) -> Result<Computed, MathError> {
    if (control & 0x0c00) != 0 || (control & 0x0300) == 0x0100 {
        return Err(MathError::Unsupported);
    }
    let (y_neg, _, y_sig) = split(y)?;
    let (x_neg, _, x_sig) = split(x)?;
    if x_sig == 0 || y_sig == 0 {
        return Ok(signed_zero(y_neg ^ x_neg));
    }
    const CW: u16 = 0x037f;
    let one = Extended80::from_bits(1u64 << 63, 0x3fff);
    let two = Extended80::from_bits(1u64 << 63, 0x4000);
    let one_plus_x = calculate(Arithmetic::Add, one, x, CW)?.value;
    let (neg, exp, sig) = split(one_plus_x)?;
    if neg || sig == 0 {
        return Err(MathError::Unsupported);
    }
    let mut k = exp - 0x3fff;
    let mut m_exp = 0x3fffu16;
    if sig >= 0xb504_f333_f9de_6484 {
        m_exp = 0x3ffe;
        k += 1;
    }
    let m = Extended80::from_bits(sig, m_exp);
    let t = calculate(Arithmetic::Subtract, m, one, CW)?.value;
    let denom = calculate(Arithmetic::Add, two, t, CW)?.value;
    let u = calculate(Arithmetic::Divide, t, denom, CW)?.value;
    let u2 = calculate(Arithmetic::Multiply, u, u, CW)?.value;
    let mut term = u;
    let mut acc = u;
    for n in 1..=14i32 {
        term = calculate(Arithmetic::Multiply, term, u2, CW)?.value;
        let part = calculate(Arithmetic::Divide, term, int_to_ext80(2 * n + 1), CW)?.value;
        acc = calculate(Arithmetic::Add, acc, part, CW)?.value;
    }
    let ln_m = calculate(Arithmetic::Multiply, two, acc, CW)?.value;
    let log2_e = Extended80::from_bits(0xb8aa_3b29_5c17_f0bc, 0x3fff);
    let log2_m = calculate(Arithmetic::Multiply, ln_m, log2_e, CW)?.value;
    let log2_1px = if k == 0 {
        log2_m
    } else {
        calculate(Arithmetic::Add, int_to_ext80(k), log2_m, CW)?.value
    };
    let mut result = calculate(Arithmetic::Multiply, y, log2_1px, control)?;
    result.inexact = true;
    Ok(result)
}

/// Round `x` to an integer according to the x87 rounding control (`RC`).
pub fn frndint(x: Extended80, control: u16) -> Result<Computed, MathError> {
    let (negative, exp, sig) = split(x)?;
    if exp == 0 && sig == 0 {
        return Ok(exact(x));
    }
    let unbiased = exp - 16383;
    if unbiased >= 63 {
        return Ok(exact(x));
    }
    let rc = (control >> 10) & 3;
    if unbiased < 0 {
        let round_away = match rc {
            0 => unbiased == -1 && sig > (1u64 << 63),
            1 => negative,
            2 => !negative,
            _ => false,
        };
        let value = if round_away {
            Extended80::from_bits(1u64 << 63, 0x3fff | if negative { 0x8000 } else { 0 })
        } else {
            Extended80::from_bits(0, if negative { 0x8000 } else { 0 })
        };
        return Ok(Computed { value, inexact: true, rounded_up: round_away, zero_divide: false });
    }
    let frac_bits = (63 - unbiased) as u32;
    let mask = (1u64 << frac_bits) - 1;
    let frac = sig & mask;
    if frac == 0 {
        return Ok(exact(x));
    }
    let mut int_sig = sig & !mask;
    let mut out_exp = exp;
    let half = 1u64 << (frac_bits - 1);
    let round_away = match rc {
        0 => frac > half || (frac == half && ((int_sig >> frac_bits) & 1 != 0)),
        1 => negative,
        2 => !negative,
        _ => false,
    };
    if round_away {
        let (v, carry) = int_sig.overflowing_add(1u64 << frac_bits);
        int_sig = if carry { 1u64 << 63 } else { v };
        if carry { out_exp += 1; }
    }
    Ok(Computed {
        value: Extended80::from_bits(int_sig, out_exp as u16 | if negative { 0x8000 } else { 0 }),
        inexact: true,
        rounded_up: round_away,
        zero_divide: false,
    })
}

/// Compute `sin(x)` (`cos == false`) or `cos(x)` (`cos == true`) in pure integer x87 arithmetic.
pub fn sin_cos(x: Extended80, cos: bool, control: u16) -> Result<Computed, MathError> {
    if (control & 0x0c00) != 0 || (control & 0x0300) == 0x0100 {
        return Err(MathError::Unsupported);
    }
    let (negative, exp, sig) = split(x)?;
    let one = Extended80::from_bits(1u64 << 63, 0x3fff);
    if exp == 0 && sig == 0 {
        return Ok(if cos { exact(one) } else { signed_zero(negative) });
    }
    if exp - 16383 >= 62 {
        return Err(MathError::Unsupported);
    }
    const CW: u16 = 0x037f;
    let two_over_pi = Extended80::from_bits(0xa2f9_836e_4e44_152a, 0x3ffe);
    let pi_half = Extended80::from_bits(0xc90f_daa2_2168_c235, 0x3fff);
    let scaled = calculate(Arithmetic::Multiply, x, two_over_pi, CW)?.value;
    let q_ext = frndint(scaled, CW)?.value;
    let q = match split(q_ext)? {
        (_, 0, 0) => 0i64,
        (q_neg, q_exp, q_sig) => {
            let mag = (q_sig >> (63 - (q_exp - 16383))) as i64;
            if q_neg { -mag } else { mag }
        }
    };
    let r = if q == 0 {
        x
    } else {
        let q_pi_half = calculate(Arithmetic::Multiply, q_ext, pi_half, CW)?.value;
        calculate(Arithmetic::Subtract, x, q_pi_half, CW)?.value
    };
    let r2 = calculate(Arithmetic::Multiply, r, r, CW)?.value;
    let mut sin_term = r;
    let mut sin_acc = r;
    let mut cos_term = one;
    let mut cos_acc = one;
    for k in 1..=10i32 {
        let c_div = calculate(Arithmetic::Multiply, int_to_ext80(2 * k - 1), int_to_ext80(2 * k), CW)?.value;
        cos_term = calculate(Arithmetic::Divide, calculate(Arithmetic::Multiply, cos_term, r2, CW)?.value, c_div, CW)?.value;
        cos_term.sign_exponent ^= 0x8000;
        cos_acc = calculate(Arithmetic::Add, cos_acc, cos_term, CW)?.value;

        let s_div = calculate(Arithmetic::Multiply, int_to_ext80(2 * k), int_to_ext80(2 * k + 1), CW)?.value;
        sin_term = calculate(Arithmetic::Divide, calculate(Arithmetic::Multiply, sin_term, r2, CW)?.value, s_div, CW)?.value;
        sin_term.sign_exponent ^= 0x8000;
        sin_acc = calculate(Arithmetic::Add, sin_acc, sin_term, CW)?.value;
    }
    let quadrant = q.rem_euclid(4);
    let (mut val, negate) = match (cos, quadrant) {
        (false, 0) | (true, 3) => (sin_acc, false),
        (false, 1) | (true, 0) => (cos_acc, false),
        (false, 2) | (true, 1) => (sin_acc, true),
        _ => (cos_acc, true),
    };
    if negate && val.significand != 0 {
        val.sign_exponent ^= 0x8000;
    }
    let mut result = calculate(Arithmetic::Multiply, val, one, control)?;
    result.inexact = true;
    Ok(result)
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
    fn integer_long_division_rounds_nearest_even() {
        let three = Extended80::from_bits(0xc000_0000_0000_0000, 0x4000);
        let half = Extended80::from_bits(1u64 << 63, 0x3ffe);
        assert_eq!(calculate(Arithmetic::Divide, three, TWO, CW).unwrap().value,
                   Extended80::from_bits(0xc000_0000_0000_0000, 0x3fff));
        assert_eq!(calculate(Arithmetic::ReverseDivide, TWO, three, CW).unwrap().value,
                   Extended80::from_bits(0xc000_0000_0000_0000, 0x3fff));
        let third = calculate(Arithmetic::Divide, ONE, three, CW).unwrap();
        assert_eq!(third.value, Extended80::from_bits(0xaaaaaaaaaaaaaaab, 0x3ffd));
        assert!(third.inexact && third.rounded_up);
        let negative_third = calculate(Arithmetic::Divide,
             Extended80::from_bits(ONE.significand, 0xbfff), three, CW).unwrap();
        assert_eq!(negative_third.value,
             Extended80::from_bits(0xaaaaaaaaaaaaaaab, 0xbffd));
        assert_eq!(calculate(Arithmetic::Divide, ZERO, TWO, CW).unwrap().value, ZERO);
        let pos_inf = Extended80::from_bits(1u64 << 63, 0x7fff);
        let neg_inf = Extended80::from_bits(1u64 << 63, 0xffff);
        assert_eq!(calculate(Arithmetic::Divide, ONE, ZERO, CW),
                   Ok(Computed { value: pos_inf, inexact: false, rounded_up: false, zero_divide: true }));
        assert_eq!(calculate(Arithmetic::Divide, ONE, Extended80::from_bits(0, 0x8000), CW),
                   Ok(Computed { value: neg_inf, inexact: false, rounded_up: false, zero_divide: true }));
        assert_eq!(calculate(Arithmetic::Divide, ZERO, ZERO, CW),
                   Err(MathError::Unsupported));
        assert_eq!(calculate(Arithmetic::Divide, half, TWO, CW).unwrap().value,
                   Extended80::from_bits(1u64 << 63, 0x3ffd));
    }

    #[test]
    fn ordered_compare_handles_exponent_sign_and_signed_zero() {
        let neg = Extended80::from_bits(ONE.significand, 0xbfff);
        let minus_zero = Extended80::from_bits(0, 0x8000);
        assert_eq!(compare(ONE, ONE), Ok(Ordering::Equal));
        assert_eq!(compare(ZERO, minus_zero), Ok(Ordering::Equal));
        assert_eq!(compare(neg, ZERO), Ok(Ordering::Less));
        assert_eq!(compare(ZERO, neg), Ok(Ordering::Greater));
        assert_eq!(compare(ONE, TWO), Ok(Ordering::Less));
        assert_eq!(compare(neg, TWO), Ok(Ordering::Less));
        assert_eq!(compare(neg, Extended80::from_bits(TWO.significand, 0xc000)),
                   Ok(Ordering::Greater));
        assert_eq!(compare(ONE, Extended80::from_bits(0xc000_0000_0000_0000, 0x3fff)),
                   Ok(Ordering::Less));
        let pos_inf = Extended80::from_bits(1u64 << 63, 0x7fff);
        let neg_inf = Extended80::from_bits(1u64 << 63, 0xffff);
        assert_eq!(compare(pos_inf, pos_inf), Ok(Ordering::Equal));
        assert_eq!(compare(neg_inf, neg_inf), Ok(Ordering::Equal));
        assert_eq!(compare(pos_inf, neg_inf), Ok(Ordering::Greater));
        assert_eq!(compare(neg_inf, pos_inf), Ok(Ordering::Less));
        assert_eq!(compare(ONE, pos_inf), Ok(Ordering::Less));
        assert_eq!(compare(neg_inf, ONE), Ok(Ordering::Less));
        assert_eq!(compare(ONE, Extended80::from_bits(0xc000_0000_0000_0000, 0x7fff)),
                   Err(MathError::Unsupported));
    }

    #[test]
    fn unsupported_cases_cannot_fabricate_results() {
        assert_eq!(calculate(Arithmetic::Add, ONE, ONE, 0x017f),
                   Err(MathError::Unsupported)); // reserved precision control PC=01
        let third_53 = calculate(Arithmetic::Divide, ONE, Extended80::from_bits(3u64 << 62, 0x4000), 0x027f).unwrap();
        assert_eq!(third_53.value.significand & ((1u64 << 11) - 1), 0);
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
