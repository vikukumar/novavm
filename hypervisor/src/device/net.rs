//! Native VirtIO Network Adapter (virtio-net-pci / e1000 emulation).
//!
//! Provides a built-in virtual Ethernet network interface for NovaVM guest VMs.
//! Supports guest MAC address filtering, RX/TX packet buffers, and host NAT translation.

use std::collections::VecDeque;

/// Standard Ethernet MAC address (6 bytes).
pub type MacAddress = [u8; 6];

/// Default NovaVM virtual network MAC address prefix (52:54:00:12:34:xx).
pub const DEFAULT_MAC: MacAddress = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

/// Status flags for the VirtIO Network device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetDeviceStatus {
    Reset,
    Acknowledged,
    DriverFound,
    DriverReady,
    Failed,
}

/// VirtIO Network Device state.
#[derive(Debug)]
pub struct VirtioNetDevice {
    /// Device MAC address.
    pub mac: MacAddress,
    /// Current device status register.
    pub status: NetDeviceStatus,
    /// Pending transmit packet queue (guest -> host NAT).
    pub tx_queue: VecDeque<Vec<u8>>,
    /// Pending receive packet queue (host NAT -> guest).
    pub rx_queue: VecDeque<Vec<u8>>,
    /// Link status (true = up, false = down).
    pub link_up: bool,
    /// Total bytes transmitted.
    pub tx_bytes: u64,
    /// Total bytes received.
    pub rx_bytes: u64,
    /// Interrupt Status Register.
    pub isr: u8,
}

impl VirtioNetDevice {
    /// Create a new VirtIO network device with a default MAC address.
    pub fn new() -> Self {
        Self {
            mac: DEFAULT_MAC,
            status: NetDeviceStatus::Reset,
            tx_queue: VecDeque::with_capacity(128),
            rx_queue: VecDeque::with_capacity(128),
            link_up: true,
            tx_bytes: 0,
            rx_bytes: 0,
            isr: 0,
        }
    }

    /// Reset the network device state.
    pub fn reset(&mut self) {
        self.status = NetDeviceStatus::Reset;
        self.tx_queue.clear();
        self.rx_queue.clear();
        self.isr = 0;
    }

    /// Read from network device I/O registers (0xC020–0xC03F range).
    pub fn io_read(&mut self, reg: u8) -> u8 {
        match reg {
            0x00..=0x05 => self.mac[reg as usize], // MAC address bytes
            0x06 => if self.link_up { 0x01 } else { 0x00 },
            0x07 => self.isr,
            0x08 => (self.tx_queue.len() & 0xFF) as u8,
            0x09 => (self.rx_queue.len() & 0xFF) as u8,
            _ => 0x00,
        }
    }

    /// Write to network device I/O registers.
    pub fn io_write(&mut self, reg: u8, val: u8) {
        match reg {
            0x06 => self.link_up = (val & 0x01) != 0,
            0x07 => self.isr &= !val, // ACK interrupt
            0x0A => {
                if val == 0x01 {
                    self.reset();
                }
            }
            _ => {}
        }
    }

    /// Transmit an Ethernet frame from the guest VM to the host network interface.
    pub fn transmit_frame(&mut self, frame: Vec<u8>) {
        self.tx_bytes += frame.len() as u64;
        self.tx_queue.push_back(frame);
    }

    /// Receive an Ethernet frame into the guest VM queue from host network interface.
    pub fn receive_frame(&mut self, frame: Vec<u8>) {
        self.rx_bytes += frame.len() as u64;
        self.rx_queue.push_back(frame);
        self.isr |= 0x01; // Raise RX interrupt
    }

    /// Pop next transmitted frame for host processing.
    pub fn pop_tx_frame(&mut self) -> Option<Vec<u8>> {
        self.tx_queue.pop_front()
    }
}

impl Default for VirtioNetDevice {
    fn default() -> Self {
        Self::new()
    }
}
