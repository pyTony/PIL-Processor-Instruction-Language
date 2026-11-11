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
