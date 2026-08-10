//! Virtual device bus — dispatches I/O port accesses to the right device.
//!
//! Every I/O port read or write from the guest vCPU comes here. The bus
//! owns an instance of each emulated hardware device matching VMware Workstation
//! architecture and routes accesses by standard x86 PC I/O port ranges.
//!
//! # Standard PC & VMware Hardware I/O map:
//!
//! | Range       | Device                              |
//! |-------------|-------------------------------------|
//! | 0x00–0x1F   | 8237 DMA Controller 1               |
//! | 0x20–0x21   | 8259 Master PIC                     |
//! | 0x40–0x43   | 8253 PIT                            |
//! | 0x60–0x64   | PS/2 Keyboard / Mouse Controller    |
//! | 0x70–0x71   | MC146818 CMOS / Real-Time Clock     |
//! | 0x80        | POST diagnostic debug port          |
//! | 0x92        | Fast A20 gate                       |
//! | 0xA0–0xA1   | 8259 Slave PIC                      |
//! | 0xC0–0xDF   | 8237 DMA Controller 2               |
//! | 0x00F0–0x00F7| vTPM 2.0 TIS I/O Interface          |
//! | 0x0170–0x0177| Secondary IDE Controller (CD-ROM)   |
//! | 0x01F0–0x01F7| Primary IDE Controller (Hard Disk)  |
//! | 0x02F8–0x02FF| COM2 Serial Port                    |
//! | 0x0378–0x037F| LPT1 Parallel Port                  |
//! | 0x03C0–0x03DF| VGA Controller                      |
//! | 0x03F8–0x03FF| COM1 Serial Port (Console Log)      |
//! | 0x0510–0x0511| NovaVM BIOS Hypercall Interface     |
//! | 0x0600–0x060B| ACPI PM1a Event/Control & PM Timer  |
//! | 0xCF8       | PCI CONFIG_ADDRESS                  |
//! | 0xCFC       | PCI CONFIG_DATA                     |
//! | 0xC000–0xC01F| UHCI USB 1.1 / 2.0 Host Controller  |

pub mod acpi;
pub mod ide;
pub mod pci;
pub mod pic;
pub mod pit;
pub mod rtc;
pub mod tpm;
pub mod net;
pub mod uart;
pub mod usb;
pub mod vga;

use std::sync::{Arc, Mutex};

pub use acpi::AcpiDevice;
pub use ide::IdeController;
pub use pci::PciBus;
pub use pic::Pic8259;
pub use pit::Pit8253;
pub use rtc::Rtc;
pub use net::VirtioNetDevice;
pub use tpm::TpmDevice;
pub use uart::Uart8250;
pub use usb::UsbController;
pub use vga::VgaDevice;

/// I/O port and MMIO dispatch bus. Shared between vCPU threads via `Arc`.
#[derive(Debug)]
pub struct DeviceBus {
    /// COM1 serial port (0x3F8–0x3FF) — main console
    pub uart_com1: Mutex<Uart8250>,
    /// COM2 serial port (0x2F8–0x2FF) — secondary serial
    pub uart_com2: Mutex<Uart8250>,
    /// Master PIC (0x20–0x21), services IRQ 0–7.
    pub pic_master: Mutex<Pic8259>,
    /// Slave PIC (0xA0–0xA1), services IRQ 8–15.
    pub pic_slave: Mutex<Pic8259>,
    /// Interval timer (0x40–0x43).
    pub pit: Mutex<Pit8253>,
    /// CMOS/RTC (0x70–0x71).
    pub rtc: Mutex<Rtc>,
    /// VGA controller and framebuffer.
    pub vga: Mutex<VgaDevice>,
    /// PCI Configuration Space Bus Controller (0xCF8 / 0xCFC).
    pub pci: Mutex<PciBus>,
    /// Virtual TPM 2.0 TIS Security Controller (0x00F0–0x00F7).
    pub tpm: Mutex<TpmDevice>,
    /// UHCI USB 1.1 / 2.0 Host Controller & HID Hub (0xC000–0xC01F).
    pub usb: Mutex<UsbController>,
    /// ACPI PM1a Power Controller & PM_TMR (0x0600–0x060B).
    pub acpi: Mutex<AcpiDevice>,
    /// Dual Channel IDE / ATA Controller (0x1F0 / 0x170).
    pub ide: Mutex<IdeController>,
    /// Native VirtIO Network Controller (0xC020–0xC03F).
    pub net: Mutex<VirtioNetDevice>,
    /// BIOS disk-read result flag (0=ok, 1=error) — written by BIOS hypercall handler.
    pub disk_status: Mutex<u8>,
}

impl DeviceBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            uart_com1: Mutex::new(Uart8250::new()),
            uart_com2: Mutex::new(Uart8250::new()),
            pic_master: Mutex::new(Pic8259::new(0x08)),
            pic_slave: Mutex::new(Pic8259::new(0x70)),
            pit: Mutex::new(Pit8253::new()),
            rtc: Mutex::new(Rtc::new()),
            vga: Mutex::new(VgaDevice::new()),
            pci: Mutex::new(PciBus::new()),
            tpm: Mutex::new(TpmDevice::new()),
            usb: Mutex::new(UsbController::new()),
            acpi: Mutex::new(AcpiDevice::new()),
            ide: Mutex::new(IdeController::new()),
            net: Mutex::new(VirtioNetDevice::new()),
            disk_status: Mutex::new(0),
        })
    }

    /// Handle a guest I/O port **read**. Returns the value the guest should see.
    pub fn io_read(&self, port: u16) -> u64 {
        match port {
            // PIC
            0x20 | 0x21 => self.pic_master.lock().unwrap().read(port & 1) as u64,
            0xA0 | 0xA1 => self.pic_slave.lock().unwrap().read(port & 1) as u64,
            // PIT
            0x40..=0x42 => self.pit.lock().unwrap().read((port - 0x40) as u8) as u64,
            0x43 => 0, // control register is write-only
            // PS/2 keyboard controller
            0x60 => 0xAA,
            0x61 => 0x00,
            0x64 => 0x10, // output buffer empty
            // CMOS/RTC
            0x70 => self.rtc.lock().unwrap().read_index() as u64,
            0x71 => self.rtc.lock().unwrap().read_data() as u64,
            // vTPM 2.0 TIS
            0x00F0..=0x00F7 => self.tpm.lock().unwrap().read_reg((port - 0x00F0) as u8) as u64,
            // Primary IDE
            0x01F0 => self.ide.lock().unwrap().primary.read_data_16() as u64,
            0x01F1..=0x01F7 => self.ide.lock().unwrap().primary.read_reg((port - 0x01F0) as u8) as u64,
            // Secondary IDE
            0x0170 => self.ide.lock().unwrap().secondary.read_data_16() as u64,
            0x0171..=0x0177 => self.ide.lock().unwrap().secondary.read_reg((port - 0x0170) as u8) as u64,
            // COM2 UART
            0x2F8..=0x2FF => self.uart_com2.lock().unwrap().read(port - 0x2F8) as u64,
            // VGA
            0x3C0..=0x3DF => self.vga.lock().unwrap().io_read(port) as u64,
            // COM1 UART
            0x3F8..=0x3FF => self.uart_com1.lock().unwrap().read(port - 0x3F8) as u64,
            // NovaVM BIOS hypercall status
            0x0511 => *self.disk_status.lock().unwrap() as u64,
            // ACPI PM1a & PM_TMR
            0x0600..=0x060B => self.acpi.lock().unwrap().read(port) as u64,
            // PCI Bus Mechanism #1
            0xCF8 => self.pci.lock().unwrap().config_addr as u64,
            0xCFC..=0xCFF => self.pci.lock().unwrap().read_config_data() as u64,
            // UHCI USB Controller
            0xC000..=0xC01F => self.usb.lock().unwrap().read_io(port - 0xC000, 2) as u64,
            // VirtIO Network Controller
            0xC020..=0xC03F => self.net.lock().unwrap().io_read((port - 0xC020) as u8) as u64,
            // Speaker / DMA / unhandled ISA ports — floating bus returns 0xFF
            _ => 0xFF,
        }
    }

    /// Handle a guest I/O port **write**.
    pub fn io_write(&self, port: u16, data: u64) {
        let b = data as u8;
        match port {
            0x20 | 0x21 => self.pic_master.lock().unwrap().write(port & 1, b),
            0xA0 | 0xA1 => self.pic_slave.lock().unwrap().write(port & 1, b),
            0x40..=0x43 => self.pit.lock().unwrap().write((port - 0x40) as u8, b),
            0x61 => {} // speaker
            0x70 => self.rtc.lock().unwrap().write_index(b),
            0x71 => self.rtc.lock().unwrap().write_data(b),
            // vTPM 2.0 TIS
            0x00F0..=0x00F7 => self.tpm.lock().unwrap().write_reg((port - 0x00F0) as u8, b),
            // Primary IDE
            0x01F0..=0x01F7 => self.ide.lock().unwrap().primary.write_reg((port - 0x01F0) as u8, b),
            // Secondary IDE
            0x0170..=0x0177 => self.ide.lock().unwrap().secondary.write_reg((port - 0x0170) as u8, b),
            // COM2 UART
            0x2F8..=0x2FF => self.uart_com2.lock().unwrap().write(port - 0x2F8, data),
            // VGA I/O
            0x3C0..=0x3DF => self.vga.lock().unwrap().io_write(port, b),
            // COM1 UART — guest serial output captured here
            0x3F8..=0x3FF => self.uart_com1.lock().unwrap().write(port - 0x3F8, data),
            // A20 gate, POST/debug port
            0x80 => tracing::trace!(code = format_args!("{:#04X}", b), "POST code"),
            0x92 => {} // A20
            // ACPI PM1a & PM_TMR
            0x0600..=0x060B => self.acpi.lock().unwrap().write(port, data as u32),
            // PCI Bus Mechanism #1
            0xCF8 => self.pci.lock().unwrap().config_addr = data as u32,
            0xCFC..=0xCFF => self.pci.lock().unwrap().write_config_data(data as u32),
            // UHCI USB Controller
            0xC000..=0xC01F => self.usb.lock().unwrap().write_io(port - 0xC000, data as u32, 2),
            // VirtIO Network Controller
            0xC020..=0xC03F => self.net.lock().unwrap().io_write((port - 0xC020) as u8, b),
            // DMA pages, ISA DMA controller — silently ignore
            0x00..=0x1F | 0x80..=0x9F | 0xC0..=0xDF => {}
            _ => {}
        }
    }

    /// Drain all serial output bytes written by the guest (UART TX buffer).
    pub fn drain_serial_output(&self) -> Vec<u8> {
        self.uart_com1.lock().unwrap().drain_output()
    }

    /// Push bytes into the UART RX buffer (keyboard / stdin for guest).
    pub fn send_input(&self, bytes: &[u8]) {
        self.uart_com1.lock().unwrap().inject_input(bytes);
    }
}
