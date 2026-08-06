//! MC146818 CMOS/RTC emulation (ports 0x70–0x71).
//!
//! The RTC provides:
//! - Real wall-clock time (seconds, minutes, hours, day, month, year) in BCD.
//! - 128 bytes of battery-backed CMOS RAM used by BIOS to store configuration.
//! - Status registers controlling interrupt modes.
//!
//! Linux and Windows read the RTC during boot to set the system clock and to
//! validate CMOS configuration bytes. Returning plausible values here prevents
//! the guest from hanging or complaining about invalid CMOS.

use std::time::{SystemTime, UNIX_EPOCH};

/// MC146818 CMOS/RTC emulation.
#[derive(Debug)]
pub struct Rtc {
    /// Currently selected CMOS register index (bits 0–6).
    index: u8,
    /// Disable NMI when true (bit 7 of the index port write).
    nmi_disable: bool,
    /// 128-byte CMOS RAM (registers + battery-backed storage).
    ram: [u8; 128],
}

impl Rtc {
    pub fn new() -> Self {
        let mut rtc = Self {
            index: 0,
            nmi_disable: false,
            ram: [0u8; 128],
        };
        // Register B: 24-hour mode, BCD format, periodic interrupt disabled
        rtc.ram[0x0B] = 0x02;
        // Register C: no pending interrupts
        rtc.ram[0x0C] = 0x00;
        // Register D: VRT (valid RAM and time) bit set — battery is good
        rtc.ram[0x0D] = 0x80;
        // Equipment byte: VGA display, 2 floppy drives (not that we have them)
        rtc.ram[0x14] = 0x2F;
        // Base memory: 640KB (0x280 paragraphs)
        rtc.ram[0x15] = 0x80;
        rtc.ram[0x16] = 0x02;
        rtc
    }

    /// Port 0x70 read: returns the current index register value.
    pub fn read_index(&self) -> u8 {
        self.index | if self.nmi_disable { 0x80 } else { 0 }
    }

    /// Port 0x70 write: select a CMOS register.
    pub fn write_index(&mut self, data: u8) {
        self.nmi_disable = data & 0x80 != 0;
        self.index = data & 0x7F;
    }

    /// Port 0x71 read: return current register value.
    pub fn read_data(&self) -> u8 {
        // Read real wall-clock time from the host for time registers
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let to_bcd = |v: u8| -> u8 { ((v / 10) << 4) | (v % 10) };
        let sec = (secs % 60) as u8;
        let min = ((secs / 60) % 60) as u8;
        let hour = ((secs / 3600) % 24) as u8;
        let wday = ((secs / 86400 + 4) % 7 + 1) as u8; // day of week (1=Sunday)
        let mday = ((secs / 86400) % 31 + 1) as u8;    // approximate
        let mon = ((secs / (86400 * 31)) % 12 + 1) as u8;
        let year = (((secs / (86400 * 365)) + 1970) % 100) as u8;

        match self.index {
            0x00 => to_bcd(sec),
            0x02 => to_bcd(min),
            0x04 => to_bcd(hour),
            0x06 => to_bcd(wday),
            0x07 => to_bcd(mday),
            0x08 => to_bcd(mon),
            0x09 => to_bcd(year),
            // Status Register A: 32kHz crystal, 1024Hz interrupt rate, not updating
            0x0A => 0x26,
            0x0B => self.ram[0x0B],
            0x0C => 0x00, // No pending interrupts
            0x0D => 0x80, // VRT: battery good
            _ => {
                let idx = self.index as usize;
                if idx < self.ram.len() { self.ram[idx] } else { 0xFF }
            }
        }
    }

    /// Port 0x71 write: store value in CMOS RAM.
    pub fn write_data(&mut self, data: u8) {
        let idx = self.index as usize;
        if idx < self.ram.len() {
            self.ram[idx] = data;
        }
    }
}

impl Default for Rtc {
    fn default() -> Self {
        Self::new()
    }
}
