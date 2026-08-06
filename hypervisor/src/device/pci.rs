//! PCI Bus Controller (Intel i440FX + PIIX3 Southbridge) emulation.
//!
//! Emulates the x86 PCI Configuration Space Mechanism #1:
//! - I/O Port 0xCF8: `CONFIG_ADDRESS` (32-bit register specifying Bus/Device/Function/Offset)
//! - I/O Port 0xCFC: `CONFIG_DATA` (32-bit data window to the selected PCI config space)
//!
//! Enumerates all virtual PCI devices present in NovaVM:
//! - **00:00.0** — Intel i440FX Host Bridge (0x8086:0x1237)
//! - **00:01.0** — PIIX3 ISA Bridge (0x8086:0x7000)
//! - **00:01.1** — PIIX3 IDE Controller (0x8086:0x7010)
//! - **00:01.2** — PIIX3 USB UHCI Controller (0x8086:0x7020)
//! - **00:01.3** — PIIX3 ACPI Power Controller (0x8086:0x7113)
//! - **00:02.0** — Standard VGA Graphics Adapter (0x1234:0x1111)
//! - **00:03.0** — vTPM 2.0 Security Device (0x1014:0x0001)

/// PCI Configuration Space Address Register (0xCF8) parser
#[derive(Debug, Clone, Copy, Default)]
pub struct PciAddress {
    pub enabled: bool,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub register: u8,
}

impl PciAddress {
    pub fn from_u32(val: u32) -> Self {
        Self {
            enabled: (val & 0x8000_0000) != 0,
            bus: ((val >> 16) & 0xFF) as u8,
            device: ((val >> 11) & 0x1F) as u8,
            function: ((val >> 8) & 0x07) as u8,
            register: (val & 0xFC) as u8,
        }
    }
}

/// PCI Device Configuration Space Header (256 bytes)
#[derive(Debug, Clone)]
pub struct PciHeader {
    pub vendor_id: u16,
    pub device_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub class_code: [u8; 3], // Base class, Subclass, Prog IF
    pub header_type: u8,
    pub bar: [u32; 6],
}

impl PciHeader {
    pub fn new(vendor_id: u16, device_id: u16, class_code: [u8; 3], header_type: u8) -> Self {
        Self {
            vendor_id,
            device_id,
            command: 0x0007, // I/O Space, Memory Space, Bus Master enable
            status: 0x0280,  // Fast devsel, medium timing
            revision_id: 0x01,
            class_code,
            header_type,
            bar: [0u32; 6],
        }
    }

    pub fn read_u32(&self, reg: u8) -> u32 {
        match reg {
            0x00 => (self.device_id as u32) << 16 | (self.vendor_id as u32),
            0x04 => (self.status as u32) << 16 | (self.command as u32),
            0x08 => (self.class_code[0] as u32) << 24
                | (self.class_code[1] as u32) << 16
                | (self.class_code[2] as u32) << 8
                | (self.revision_id as u32),
            0x0C => (self.header_type as u32) << 16,
            0x10 => self.bar[0],
            0x14 => self.bar[1],
            0x18 => self.bar[2],
            0x1C => self.bar[3],
            0x20 => self.bar[4],
            0x24 => self.bar[5],
            _ => 0x0000_0000,
        }
    }

    pub fn write_u32(&mut self, reg: u8, val: u32) {
        match reg {
            0x04 => self.command = (val & 0xFFFF) as u16,
            0x10 => self.bar[0] = val,
            0x14 => self.bar[1] = val,
            0x18 => self.bar[2] = val,
            0x1C => self.bar[3] = val,
            0x20 => self.bar[4] = val,
            0x24 => self.bar[5] = val,
            _ => {}
        }
    }
}

/// Virtual PCI Bus Manager
#[derive(Debug)]
pub struct PciBus {
    /// Currently selected config address written to 0xCF8
    pub config_addr: u32,
    /// Registered PCI devices indexed by (bus, dev, fn)
    devices: Vec<(u8, u8, u8, PciHeader)>,
}

impl PciBus {
    pub fn new() -> Self {
        let mut bus = Self {
            config_addr: 0,
            devices: Vec::new(),
        };

        // 00:00.0 — Intel i440FX Host Bridge (0x8086:0x1237)
        bus.devices.push((0, 0, 0, PciHeader::new(0x8086, 0x1237, [0x06, 0x00, 0x00], 0x00)));

        // 00:01.0 — PIIX3 ISA/PCI Bridge (0x8086:0x7000)
        bus.devices.push((0, 1, 0, PciHeader::new(0x8086, 0x7000, [0x06, 0x01, 0x00], 0x80)));

        // 00:01.1 — PIIX3 IDE Controller (0x8086:0x7010)
        let mut ide = PciHeader::new(0x8086, 0x7010, [0x01, 0x01, 0x80], 0x00);
        ide.bar[4] = 0x0000_C041; // Bus Master IDE I/O BAR
        bus.devices.push((0, 1, 1, ide));

        // 00:01.2 — PIIX3 USB UHCI Controller (0x8086:0x7020)
        let mut usb = PciHeader::new(0x8086, 0x7020, [0x0C, 0x03, 0x00], 0x00);
        usb.bar[4] = 0x0000_C001; // UHCI I/O Ports BAR (0xC000)
        bus.devices.push((0, 1, 2, usb));

        // 00:01.3 — PIIX3 ACPI Power Management Controller (0x8086:0x7113)
        let mut acpi = PciHeader::new(0x8086, 0x7113, [0x06, 0x80, 0x00], 0x00);
        acpi.bar[4] = 0x0000_0601; // ACPI PM Ports BAR (0x0600)
        bus.devices.push((0, 1, 3, acpi));

        // 00:02.0 — Standard VGA Display Adapter (0x1234:0x1111)
        let mut vga = PciHeader::new(0x1234, 0x1111, [0x03, 0x00, 0x00], 0x00);
        vga.bar[0] = 0xE000_0008; // VRAM Linear Framebuffer BAR (0xE0000000)
        bus.devices.push((0, 2, 0, vga));

        // 00:03.0 — vTPM 2.0 Security Controller (0x1014:0x0001)
        let mut tpm = PciHeader::new(0x1014, 0x0001, [0x0B, 0x00, 0x00], 0x00);
        tpm.bar[0] = 0xFED4_0000; // TPM TIS MMIO BAR
        bus.devices.push((0, 3, 0, tpm));

        bus
    }

    /// Handle read from Port 0xCFC (CONFIG_DATA)
    pub fn read_config_data(&self) -> u32 {
        let addr = PciAddress::from_u32(self.config_addr);
        if !addr.enabled {
            return 0xFFFF_FFFF;
        }

        for (b, d, f, header) in &self.devices {
            if *b == addr.bus && *d == addr.device && *f == addr.function {
                return header.read_u32(addr.register);
            }
        }
        0xFFFF_FFFF // Non-existent PCI device returns 0xFFFFFFFF
    }

    /// Handle write to Port 0xCFC (CONFIG_DATA)
    pub fn write_config_data(&mut self, val: u32) {
        let addr = PciAddress::from_u32(self.config_addr);
        if !addr.enabled {
            return;
        }

        for (b, d, f, header) in &mut self.devices {
            if *b == addr.bus && *d == addr.device && *f == addr.function {
                header.write_u32(addr.register, val);
                return;
            }
        }
    }
}

impl Default for PciBus {
    fn default() -> Self {
        Self::new()
    }
}
