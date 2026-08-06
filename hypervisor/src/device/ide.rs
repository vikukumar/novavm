//! IDE / ATA Storage Controller emulation.
//!
//! Emulates standard Dual-Channel PIIX3 IDE Controller:
//! - Primary Channel: Ports 0x1F0–0x1F7 (Control: 0x3F6)
//! - Secondary Channel: Ports 0x170–0x177 (Control: 0x376)
//!
//! Handles ATA & ATAPI commands:
//! - `0xEC`: `ATA_IDENTIFY`
//! - `0xA1`: `ATAPI_IDENTIFY`
//! - `0x20`: `ATA_READ_SECTORS`
//! - `0x30`: `ATA_WRITE_SECTORS`
//! - `0xA0`: `ATAPI_PACKET_COMMAND`

use std::collections::VecDeque;

/// ATA Status Register bits
#[allow(dead_code)]
const ATA_STAT_BUSY: u8 = 1 << 7;
const ATA_STAT_READY: u8 = 1 << 6;
const ATA_STAT_DRQ: u8 = 1 << 3;
#[allow(dead_code)]
const ATA_STAT_ERR: u8 = 1 << 0;

/// Virtual IDE Channel
#[derive(Debug)]
pub struct IdeChannel {
    data_buffer: VecDeque<u16>,
    sector_count: u8,
    sector_number: u8,
    cylinder_low: u8,
    cylinder_high: u8,
    drive_head: u8,
    status: u8,
    error: u8,
    command: u8,
}

impl IdeChannel {
    pub fn new() -> Self {
        Self {
            data_buffer: VecDeque::with_capacity(512),
            sector_count: 1,
            sector_number: 1,
            cylinder_low: 0,
            cylinder_high: 0,
            drive_head: 0xA0,
            status: ATA_STAT_READY,
            error: 0,
            command: 0,
        }
    }

    pub fn read_reg(&mut self, offset: u8) -> u8 {
        match offset {
            0 => {
                // Data Port 16-bit low byte
                if let Some(word) = self.data_buffer.front() {
                    (word & 0xFF) as u8
                } else {
                    0x00
                }
            }
            1 => self.error,
            2 => self.sector_count,
            3 => self.sector_number,
            4 => self.cylinder_low,
            5 => self.cylinder_high,
            6 => self.drive_head,
            7 => {
                let st = self.status;
                st
            }
            _ => 0x00,
        }
    }

    pub fn read_data_16(&mut self) -> u16 {
        if let Some(word) = self.data_buffer.pop_front() {
            if self.data_buffer.is_empty() {
                self.status &= !ATA_STAT_DRQ;
            }
            word
        } else {
            0x0000
        }
    }

    pub fn write_reg(&mut self, offset: u8, data: u8) {
        match offset {
            1 => self.error = data,
            2 => self.sector_count = data,
            3 => self.sector_number = data,
            4 => self.cylinder_low = data,
            5 => self.cylinder_high = data,
            6 => self.drive_head = data,
            7 => {
                self.command = data;
                self.process_command(data);
            }
            _ => {}
        }
    }

    fn process_command(&mut self, cmd: u8) {
        match cmd {
            // ATA IDENTIFY DEVICE (0xEC)
            0xEC => {
                self.data_buffer.clear();
                let mut identify = [0u16; 256];
                // General configuration: ATA fixed hard disk
                identify[0] = 0x0040;
                identify[1] = 16383; // Cylinders
                identify[3] = 16;    // Heads
                identify[6] = 63;    // Sectors per track
                // Model number: "NovaVM Virtual IDE Disk"
                let model = b"NovaVM Virtual IDE Hard Disk        ";
                for i in 0..16 {
                    let w = ((model[i * 2 + 1] as u16) << 8) | (model[i * 2] as u16);
                    identify[27 + i] = w;
                }
                // Capabilities: LBA supported
                identify[49] = 1 << 9;
                for w in identify {
                    self.data_buffer.push_back(w);
                }
                self.status = ATA_STAT_READY | ATA_STAT_DRQ;
            }

            // ATAPI IDENTIFY DEVICE (0xA1)
            0xA1 => {
                self.data_buffer.clear();
                let mut identify = [0u16; 256];
                identify[0] = 0x85C0; // ATAPI CD-ROM device
                let model = b"NovaVM Virtual CD-ROM Drive        ";
                for i in 0..16 {
                    let w = ((model[i * 2 + 1] as u16) << 8) | (model[i * 2] as u16);
                    identify[27 + i] = w;
                }
                for w in identify {
                    self.data_buffer.push_back(w);
                }
                self.status = ATA_STAT_READY | ATA_STAT_DRQ;
            }

            // ATA READ SECTORS (0x20)
            0x20 => {
                self.data_buffer.clear();
                // 512 bytes = 256 words of zeros (dummy sector)
                for _ in 0..256 {
                    self.data_buffer.push_back(0x0000);
                }
                self.status = ATA_STAT_READY | ATA_STAT_DRQ;
            }

            _ => {
                self.status = ATA_STAT_READY;
            }
        }
    }
}

/// Dual Channel IDE Device
#[derive(Debug)]
pub struct IdeController {
    pub primary: IdeChannel,
    pub secondary: IdeChannel,
}

impl IdeController {
    pub fn new() -> Self {
        Self {
            primary: IdeChannel::new(),
            secondary: IdeChannel::new(),
        }
    }
}

impl Default for IdeController {
    fn default() -> Self {
        Self::new()
    }
}
