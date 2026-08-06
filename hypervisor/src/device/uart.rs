//! 8250/16550 UART serial port emulation (COM1 — ports 0x3F8–0x3FF).
//!
//! The guest OS writes serial console output (boot logs, dmesg, shell output)
//! to the Transmitter Holding Register at port 0x3F8. We capture those bytes
//! in `output` so the Tauri app can forward them to the frontend Console tab.
//! The guest can also read back injected input (stdin) from the Receiver Buffer.

use std::collections::VecDeque;

/// Intel 8250/16550A UART emulation.
#[derive(Debug)]
pub struct Uart8250 {
    /// Enable divisor latch access (bit 7 of LCR).
    dlab: bool,
    /// Baud-rate divisor (16-bit).
    divisor: u16,
    /// Interrupt Enable Register.
    ier: u8,
    /// Line Control Register (data format, DLAB).
    lcr: u8,
    /// Modem Control Register.
    mcr: u8,
    /// Line Status Register: TX always empty (0x60); RX data ready when input present.
    lsr: u8,
    /// Modem Status Register: pretend all control signals asserted.
    msr: u8,
    /// Scratch register.
    scratch: u8,
    /// TX buffer — bytes written by guest to display in the console tab.
    output: VecDeque<u8>,
    /// RX buffer — bytes to feed back to guest as keyboard/stdin input.
    input: VecDeque<u8>,
}

impl Uart8250 {
    pub fn new() -> Self {
        Self {
            dlab: false,
            divisor: 1,
            ier: 0,
            lcr: 0,
            mcr: 0,
            // LSR: Transmitter Empty (bit 6) + Transmitter Holding Register Empty (bit 5)
            // = 0x60 → guest can always send; Data Ready (bit 0) = 0 initially.
            lsr: 0x60,
            // MSR: CTS, DSR, DCD, RI all asserted.
            msr: 0xB0,
            scratch: 0,
            output: VecDeque::new(),
            input: VecDeque::new(),
        }
    }

    /// Read from UART register. `offset` is 0–7 relative to base port 0x3F8.
    pub fn read(&mut self, offset: u16) -> u8 {
        match offset {
            0 => {
                if self.dlab {
                    // Divisor Latch Low
                    (self.divisor & 0xFF) as u8
                } else {
                    // Receiver Buffer Register
                    if let Some(b) = self.input.pop_front() {
                        if self.input.is_empty() {
                            self.lsr &= !0x01; // clear Data Ready
                        }
                        b
                    } else {
                        0
                    }
                }
            }
            1 => {
                if self.dlab {
                    // Divisor Latch High
                    ((self.divisor >> 8) & 0xFF) as u8
                } else {
                    self.ier
                }
            }
            // IIR: no interrupt pending (bit 0 = 1)
            2 => 0x01,
            3 => self.lcr,
            4 => self.mcr,
            5 => self.lsr,
            6 => self.msr,
            7 => self.scratch,
            _ => 0xFF,
        }
    }

    /// Write to UART register.
    pub fn write(&mut self, offset: u16, data: u64) {
        let byte = data as u8;
        match offset {
            0 => {
                if self.dlab {
                    self.divisor = (self.divisor & 0xFF00) | (byte as u16);
                } else {
                    // Transmitter Holding Register — capture guest's serial output
                    self.output.push_back(byte);
                }
            }
            1 => {
                if self.dlab {
                    self.divisor = (self.divisor & 0x00FF) | ((byte as u16) << 8);
                } else {
                    self.ier = byte & 0x0F;
                }
            }
            2 => {}  // FCR — ignore FIFO control writes
            3 => {
                self.lcr = byte;
                self.dlab = byte & 0x80 != 0;
            }
            4 => self.mcr = byte & 0x1F,
            5 | 6 => {}  // LSR/MSR are read-only
            7 => self.scratch = byte,
            _ => {}
        }
    }

    /// Drain all pending serial output written by the guest.
    /// Called by the Tauri `get_vm_serial_output` command.
    pub fn drain_output(&mut self) -> Vec<u8> {
        self.output.drain(..).collect()
    }

    /// Inject bytes into the RX buffer (simulates keyboard/stdin input to guest).
    pub fn inject_input(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.input.push_back(b);
        }
        if !self.input.is_empty() {
            self.lsr |= 0x01; // Data Ready
        }
    }
}

impl Default for Uart8250 {
    fn default() -> Self {
        Self::new()
    }
}
