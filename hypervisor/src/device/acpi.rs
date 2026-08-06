//! ACPI Power Management & System Shutdown Controller emulation.
//!
//! Emulates standard PC ACPI power management registers:
//! - Ports 0x0600–0x0603: PM1a_EVT (Power Management Event Register)
//! - Ports 0x0604–0x0605: PM1a_CNT (Power Management Control Register — guest shutdown trigger)
//! - Ports 0x0608–0x060B: PM_TMR (24-bit 3.579545 MHz ACPI Timer)
//!
//! Guest OSes write `0x2000` (SLP_TYPa = 5, SLP_EN = 1) to PM1a_CNT to initiate clean ACPI shutdown.

use std::time::Instant;

/// ACPI PM1a Control Bits
const ACPI_SLP_EN: u16 = 1 << 13;
const ACPI_SLP_TYP_MASK: u16 = 0x1C00;

/// ACPI PM Controller
#[derive(Debug)]
pub struct AcpiDevice {
    pm1a_evt: u32,
    pm1a_cnt: u16,
    start_time: Instant,
    /// Shutdown signal requested by guest OS via ACPI
    pub shutdown_requested: bool,
}

impl AcpiDevice {
    pub fn new() -> Self {
        Self {
            pm1a_evt: 0,
            pm1a_cnt: 0,
            start_time: Instant::now(),
            shutdown_requested: false,
        }
    }

    /// Read ACPI PM Register (ports 0x0600–0x060B)
    pub fn read(&mut self, port: u16) -> u32 {
        match port {
            // PM1a_EVT
            0x0600..=0x0603 => self.pm1a_evt,
            // PM1a_CNT
            0x0604 | 0x0605 => self.pm1a_cnt as u32,
            // PM_TMR: 24-bit timer running at 3.579545 MHz (3579545 ticks per second)
            0x0608..=0x060B => {
                let elapsed_secs = self.start_time.elapsed().as_secs_f64();
                let ticks = (elapsed_secs * 3_579_545.0) as u64;
                (ticks & 0x00FF_FFFF) as u32 // 24-bit mask per ACPI spec
            }
            _ => 0,
        }
    }

    /// Write ACPI PM Register
    pub fn write(&mut self, port: u16, data: u32) {
        match port {
            0x0600..=0x0603 => {
                // Write-1-to-clear event flags
                self.pm1a_evt &= !data;
            }
            0x0604 | 0x0605 => {
                let val = data as u16;
                self.pm1a_cnt = val;
                // Check if guest OS initiated ACPI Sleep / Poweroff (SLP_EN set)
                if val & ACPI_SLP_EN != 0 {
                    let slp_typ = (val & ACPI_SLP_TYP_MASK) >> 10;
                    // SLP_TYP = 5 or 7 is S5 Soft Off (Shutdown)
                    if slp_typ == 5 || slp_typ == 7 || slp_typ == 0 {
                        tracing::info!("ACPI: guest initiated S5 soft-off power shutdown");
                        self.shutdown_requested = true;
                    }
                }
            }
            _ => {}
        }
    }
}

impl Default for AcpiDevice {
    fn default() -> Self {
        Self::new()
    }
}
