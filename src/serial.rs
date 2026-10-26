//! Nonblocking byte transport. No UART register or PC port emulation yet.
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialError {
    WouldBlock,
}

pub trait SerialTransport {
    fn rx_ready(&self) -> bool;
    fn try_read(&mut self) -> Option<u8>;
    fn try_write(&mut self, byte: u8) -> Result<(), SerialError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockSerial {
    rx: VecDeque<u8>,
    tx: VecDeque<u8>,
    rx_capacity: usize,
    tx_capacity: usize,
}

impl MockSerial {
    pub fn new(rx_capacity: usize, tx_capacity: usize) -> Self {
        Self { rx: VecDeque::new(), tx: VecDeque::new(), rx_capacity, tx_capacity }
    }
    // Host/test side: queue a complete chunk or nothing.
    pub fn inject_rx(&mut self, bytes: &[u8]) -> Result<(), SerialError> {
        if bytes.len() > self.rx_capacity - self.rx.len() {
            return Err(SerialError::WouldBlock);
        }
        self.rx.extend(bytes.iter().copied());
        Ok(())
    }
    // Host/test side: collect guest-transmitted bytes, including binary zero.
    pub fn take_tx(&mut self) -> Vec<u8> { self.tx.drain(..).collect() }
    pub fn lengths(&self) -> (usize, usize) { (self.rx.len(), self.tx.len()) }
    pub fn capacities(&self) -> (usize, usize) { (self.rx_capacity, self.tx_capacity) }
    pub fn clear(&mut self) { self.rx.clear(); self.tx.clear(); }
}

impl SerialTransport for MockSerial {
    fn rx_ready(&self) -> bool { !self.rx.is_empty() }
    fn try_read(&mut self) -> Option<u8> { self.rx.pop_front() }
    fn try_write(&mut self, byte: u8) -> Result<(), SerialError> {
        if self.tx.len() >= self.tx_capacity { return Err(SerialError::WouldBlock); }
        self.tx.push_back(byte);
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/serial.rs"]
mod tests;
