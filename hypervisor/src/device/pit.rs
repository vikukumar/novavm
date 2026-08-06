//! Intel 8253/8254 Programmable Interval Timer (PIT) emulation.
//!
//! Ports 0x40–0x43. Three independent counter channels:
//! - Channel 0: System timer (generates IRQ0 at ~18.2 Hz in default BIOS mode,
//!              reprogrammed by the OS to 1000 Hz for Linux scheduler ticks).
//! - Channel 1: Historically memory refresh — ignored.
//! - Channel 2: PC speaker — ignored.
//!
//! The emulation supports Mode 2 (rate generator) and Mode 3 (square wave)
//! which are the two modes used by all real operating systems.

use std::time::Instant;

const PIT_CLOCK_HZ: u64 = 1_193_182; // 1.19318 MHz PIT clock frequency

/// One PIT counter channel.
#[derive(Debug)]
struct PitChannel {
    /// Mode (0–5).
    mode: u8,
    /// Reload value written by guest.
    reload: u16,
    /// Latch: set by read-back or latch command, cleared after two reads.
    latch: Option<u16>,
    /// Which byte to read/write next: false = low byte, true = high byte.
    read_msb: bool,
    write_msb: bool,
    /// Low byte of reload already written; waiting for high byte.
    write_low: Option<u8>,
    /// When this counter was last loaded.
    started: Option<Instant>,
}

impl PitChannel {
    fn new() -> Self {
        Self {
            mode: 0,
            reload: 0,
            latch: None,
            read_msb: false,
            write_msb: false,
            write_low: None,
            started: None,
        }
    }

    fn current_count(&self) -> u16 {
        if let (Some(start), reload) = (self.started, self.reload) {
            if reload == 0 {
                return 0;
            }
            let elapsed_ticks =
                (start.elapsed().as_nanos() as u64 * PIT_CLOCK_HZ / 1_000_000_000) as u16;
            reload.wrapping_sub(elapsed_ticks)
        } else {
            self.reload
        }
    }
}

/// Intel 8253/8254 PIT emulation.
#[derive(Debug)]
pub struct Pit8253 {
    channels: [PitChannel; 3],
}

impl Pit8253 {
    pub fn new() -> Self {
        Self {
            channels: [PitChannel::new(), PitChannel::new(), PitChannel::new()],
        }
    }

    /// Read from a PIT port. `offset` is 0–3 relative to base port 0x40.
    pub fn read(&mut self, offset: u8) -> u8 {
        if offset >= 3 {
            return 0; // control word is not readable
        }
        let ch = &mut self.channels[offset as usize];
        let count = ch.latch.unwrap_or_else(|| ch.current_count());

        if !ch.read_msb {
            // Return low byte
            ch.read_msb = true;
            if ch.latch.is_some() {
                // Only clear latch after reading both bytes
            }
            (count & 0xFF) as u8
        } else {
            // Return high byte and clear latch
            ch.read_msb = false;
            ch.latch = None;
            ((count >> 8) & 0xFF) as u8
        }
    }

    /// Write to a PIT port.
    pub fn write(&mut self, offset: u8, data: u8) {
        if offset == 3 {
            // Control word
            let ch_idx = ((data >> 6) & 0x03) as usize;
            if ch_idx == 3 {
                // Read-back command — not implemented
                return;
            }
            let rw = (data >> 4) & 0x03;
            if rw == 0 {
                // Counter latch command
                let count = self.channels[ch_idx].current_count();
                self.channels[ch_idx].latch = Some(count);
            } else {
                // Mode set
                self.channels[ch_idx].mode = (data >> 1) & 0x07;
                self.channels[ch_idx].reload = 0;
                self.channels[ch_idx].started = None;
                self.channels[ch_idx].read_msb = false;
                self.channels[ch_idx].write_msb = false;
                self.channels[ch_idx].write_low = None;
            }
        } else {
            let ch = &mut self.channels[offset as usize];
            match ch.write_low {
                None => {
                    // Expect low byte first
                    ch.write_low = Some(data);
                }
                Some(lo) => {
                    // High byte — load the counter
                    ch.reload = lo as u16 | ((data as u16) << 8);
                    ch.write_low = None;
                    ch.started = Some(Instant::now());
                    tracing::trace!(
                        channel = offset,
                        reload = ch.reload,
                        "PIT channel loaded"
                    );
                }
            }
        }
    }
}

impl Default for Pit8253 {
    fn default() -> Self {
        Self::new()
    }
}
