//! Exact bit-level IEEE binary32/binary64 to x87 80-bit conversion, and
//! finite x87 to IEEE conversion using round-to-nearest, ties-to-even.
//! No host floating-point values are used. Other x87 rounding modes and
//! signalling NaNs deliberately remain unsupported.
use super::Extended80;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionError { UnsupportedSpecial, UnsupportedRounding, OutOfRange }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Narrowed {
    pub bits: u64,
    pub inexact: bool,
    pub underflow: bool,
}
fn widen(bits: u64, fractional: u32, exponent_bits: u32, bias: i32)
    -> Result<Extended80, ConversionError> {
    let exponent_mask=(1u64<<exponent_bits)-1;
    let fraction_mask=(1u64<<fractional)-1;
    let sign=((bits>>(fractional+exponent_bits))&1)as u16;
    let exp=(bits>>fractional)&exponent_mask;
    let fraction=bits&fraction_mask;
    let signbit=sign<<15;
    if exp==exponent_mask {
        if fraction!=0 { return Err(ConversionError::UnsupportedSpecial); }
        return Ok(Extended80::from_bits(1u64<<63,signbit|0x7fff));
    }
    if exp==0&&fraction==0 {return Ok(Extended80::from_bits(0,signbit));}
    let(mut significand,mut e)=if exp==0{
        (fraction<<(63-fractional),1-bias)
    }else{
        ((fraction|(1u64<<fractional))<<(63-fractional),exp as i32-bias)
    };
    if exp==0 {
        while significand&(1u64<<63)==0 {significand<<=1;e-=1;}
    }
    Ok(Extended80::from_bits(significand,signbit|((e+16383)as u16)))
}
pub fn from_f32_bits(bits:u32)->Result<Extended80,ConversionError>{
    widen(u64::from(bits),23,8,127)
}
pub fn from_f64_bits(bits:u64)->Result<Extended80,ConversionError>{
    widen(bits,52,11,1023)
}
pub fn from_i32(value:i32)->Extended80{from_i64(i64::from(value))}
pub fn from_i64(value:i64)->Extended80{
    if value==0{return Extended80::default();}
    let negative=value<0;
    let mag=value.unsigned_abs();
    let lz=mag.leading_zeros() as u16;
    let significand=mag<<lz;
    let exp=16383+(63-lz);
    Extended80::from_bits(significand,exp|if negative{0x8000}else{0})
}

fn round_even(significand:u64,shift:u32)->(u64,bool){
    if shift==0{return (significand,false);}
    if shift<64 {
        let base=significand>>shift;
        let lost=significand&((1u64<<shift)-1);
        let half=1u64<<(shift-1);
        let up=lost>half||(lost==half&&base&1!=0);
        return (base+u64::from(up),lost!=0);
    }
    if shift==64 {
        let half=1u64<<63;
        return (u64::from(significand>half),significand!=0);
    }
    (0,significand!=0)
}
fn narrow(value:Extended80,fractional:u32,exponent_bits:u32,bias:i32,control:u16)
    ->Result<Narrowed,ConversionError>{
    let sign=u64::from(value.sign_exponent>>15)<<(fractional+exponent_bits);
    let exp=u32::from(value.sign_exponent&0x7fff);
    let all_exp=(1u64<<exponent_bits)-1;
    if exp==0&&value.significand==0 {
        return Ok(Narrowed{bits:sign,inexact:false,underflow:false});
    }
    if exp==0x7fff {
        if value.significand!=1u64<<63{return Err(ConversionError::UnsupportedSpecial);}
        return Ok(Narrowed{bits:sign|(all_exp<<fractional),inexact:false,underflow:false});
    }
    if control&0x0c00!=0{return Err(ConversionError::UnsupportedRounding);}
    if exp==0||value.significand&(1u64<<63)==0{
        return Err(ConversionError::UnsupportedSpecial);
    }
    let mut e=exp as i32-16383;
    let min_e=1-bias;
    let max_e=((1<<exponent_bits)-2)as i32-bias;
    if e>max_e{return Err(ConversionError::OutOfRange);}
    let shift=(63-fractional)+(min_e-e).max(0)as u32;
    let(mut significand,inexact)=round_even(value.significand,shift);
    let exp_field;
    let fraction_mask=(1u64<<fractional)-1;
    if e<min_e {
        if significand>=1u64<<fractional{
            // A subnormal rounded up to the smallest normal.
            exp_field=1;
            significand=0;
        }else{
            exp_field=0;
        }
    }else{
        if significand>=1u64<<(fractional+1){significand>>=1;e+=1;}
        if e>max_e{return Err(ConversionError::OutOfRange);}
        exp_field=(e+bias)as u64;
    }
    Ok(Narrowed{
        bits:sign|(exp_field<<fractional)|(significand&fraction_mask),
        inexact,
        underflow:inexact&&exp_field==0,
    })
}
pub fn to_f32_bits(value:Extended80,control:u16)->Result<Narrowed,ConversionError>{
    narrow(value,23,8,127,control)
}
pub fn to_f64_bits(value:Extended80,control:u16)->Result<Narrowed,ConversionError>{
    narrow(value,52,11,1023,control)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn ieee_exact_normals_and_signed_zero(){
        for bits in [0u32,0x80000000,0x3f800000,0xbfc00000,0x40800000,
                     0x00000001,0x007fffff,0x7f7fffff,0x7f800000,0xff800000]{
            let ext=from_f32_bits(bits).unwrap();
            assert_eq!(to_f32_bits(ext,0x037f).unwrap().bits as u32,bits,"{bits:08x}");
        }
        for bits in [0u64,1u64<<63,0x3ff0000000000000,0xbff8000000000000,
                     0x0000000000000001,0x000fffffffffffff,0x7fefffffffffffff,
                     0x7ff0000000000000,0xfff0000000000000]{
            let ext=from_f64_bits(bits).unwrap();
            assert_eq!(to_f64_bits(ext,0x037f).unwrap().bits,bits,"{bits:016x}");
        }
    }
    #[test]
    fn ieee_narrow_rounds_ties_to_even(){
        // Exactly halfway between 1.0 and next binary32 value rounds to even 1.0.
        let half=Extended80::from_bits((1u64<<63)+(1u64<<39),0x3fff);
        let r=to_f32_bits(half,0x037f).unwrap();
        assert_eq!(r.bits,0x3f800000);assert!(r.inexact);
        let more=Extended80::from_bits((1u64<<63)+(1u64<<39)+1,0x3fff);
        assert_eq!(to_f32_bits(more,0x037f).unwrap().bits,0x3f800001);
        // Round a slightly smaller-magnitude negative value to -1.
        assert_eq!(to_f32_bits(Extended80::from_bits(half.significand,0xbfff),0x037f).unwrap().bits,0xbf800000);
    }
    #[test]
    fn ieee_special_values_do_not_fabricate_nan_semantics(){
        assert_eq!(from_f32_bits(0x7fc00000),Err(ConversionError::UnsupportedSpecial));
        assert_eq!(to_f32_bits(Extended80::from_bits(0xC000000000000001,0x7fff),0x037f),
                   Err(ConversionError::UnsupportedSpecial));
        assert_eq!(to_f64_bits(Extended80::from_bits(1u64<<63,0x3fff),0x0b7f),
                   Err(ConversionError::UnsupportedRounding));
    }
}
