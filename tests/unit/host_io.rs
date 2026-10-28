use super::*;
#[test]
fn capture_keeps_binary_bytes_and_rejects_a_whole_write(){
 let mut out=CaptureOutput::new(4);out.write_atomic(&[0,255]).unwrap();let old=out.clone();assert_eq!(out.write_atomic(&[1,2,3]),Err(OutputError::Full));assert_eq!(out,old);out.write_atomic(&[13,10]).unwrap();assert_eq!(out.remaining(),0);assert_eq!(out.into_bytes(),[0,255,13,10]);
}
#[test]
fn disconnect_is_distinct_from_full_and_preserves_capture(){
 let mut out=CaptureOutput::new(8);out.write_atomic(b"ab").unwrap();out.disconnect();let old=out.clone();assert_eq!(out.write_atomic(b"x"),Err(OutputError::Disconnected));assert_eq!(out,old);
}
#[test]
fn headless_column_is_derived_from_the_exact_capture(){
 let mut out=CaptureOutput::new(64);out.write_atomic(b"abc\tX\x08\x07\n").unwrap();assert_eq!(out.column(),8);out.write_atomic(b"\rZ").unwrap();assert_eq!(out.column(),1);assert_eq!(out.bytes(),b"abc\tX\x08\x07\n\rZ");
}
