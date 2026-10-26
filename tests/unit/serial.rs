use super::*;
#[test]
fn rx_is_fifo_and_zero_is_not_empty() {
    let mut s=MockSerial::new(4,4);
    assert!(!s.rx_ready()); assert_eq!(s.try_read(),None);
    s.inject_rx(&[0,255,65]).unwrap();
    assert!(s.rx_ready());
    assert_eq!(s.try_read(),Some(0)); assert_eq!(s.try_read(),Some(255));
    assert_eq!(s.try_read(),Some(65)); assert_eq!(s.try_read(),None);
}
#[test]
fn rx_overflow_rejects_entire_chunk() {
    let mut s=MockSerial::new(2,2); s.inject_rx(&[1]).unwrap();
    let before=s.clone();
    assert_eq!(s.inject_rx(&[2,3]),Err(SerialError::WouldBlock));
    assert_eq!(s,before);
}
#[test]
fn tx_backpressure_does_not_drop_or_duplicate_bytes() {
    let mut s=MockSerial::new(2,2);
    s.try_write(0).unwrap(); s.try_write(255).unwrap();
    assert_eq!(s.try_write(65),Err(SerialError::WouldBlock));
    assert_eq!(s.take_tx(),vec![0,255]);
    s.try_write(65).unwrap(); assert_eq!(s.take_tx(),vec![65]);
    assert!(s.take_tx().is_empty());
}
#[test]
fn clear_resets_both_directions() {
    let mut s=MockSerial::new(2,3);s.inject_rx(&[1]).unwrap();s.try_write(2).unwrap();
    s.clear();assert_eq!(s.lengths(),(0,0));assert_eq!(s.capacities(),(2,3));
}
#[test]
fn zero_capacity_is_nonblocking() {
    let mut s=MockSerial::new(0,0);
    assert_eq!(s.try_read(),None);assert!(!s.rx_ready());
    assert_eq!(s.try_write(0),Err(SerialError::WouldBlock));
    assert_eq!(s.inject_rx(&[0]),Err(SerialError::WouldBlock));
    assert!(s.inject_rx(&[]).is_ok());
}
