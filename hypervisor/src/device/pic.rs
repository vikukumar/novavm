//! Intel 8259A Programmable Interrupt Controller (PIC) emulation.
//!
//! The PC uses a master/slave cascade of two 8259 chips:
//! - Master PIC: ports 0x20 (command) and 0x21 (data), IRQs 0–7 → INTs 0x08–0x0F
//! - Slave PIC:  ports 0xA0 (command) and 0xA1 (data), IRQs 8–15 → INTs 0x70–0x77
//!
//! Most guest OSes reprogram the PIC during early boot. This emulation handles
//! the ICW1–ICW4 initialization sequence and OCW1–OCW3 operational words so
//! that the guest doesn't hang waiting for PIC acknowledgment.

/// Intel 8259A PIC emulation for one chip (master or slave).
#[derive(Debug)]
pub struct Pic8259 {
    /// Interrupt base vector (set via ICW2).
    base_vector: u8,
    /// Interrupt Mask Register: 1 = masked (disabled).
    imr: u8,
    /// Interrupt Request Register: pending hardware IRQs.
    irr: u8,
    /// In-Service Register: IRQs currently being serviced.
    isr: u8,
    /// True while processing ICW2–ICW4 initialization words.
    init_phase: u8,
    /// Automatic End of Interrupt mode.
    auto_eoi: bool,
    /// OCW3 read mode: true = ISR, false = IRR.
    read_isr: bool,
}

impl Pic8259 {
    /// Create a new PIC with the given base interrupt vector.
    pub fn new(base_vector: u8) -> Self {
        Self {
            base_vector,
            imr: 0xFF, // all IRQs masked initially
            irr: 0,
            isr: 0,
            init_phase: 0,
            auto_eoi: false,
            read_isr: false,
        }
    }

    /// Port read: `port_bit` is 0 for command port, 1 for data port.
    pub fn read(&self, port_bit: u16) -> u8 {
        match port_bit {
            0 => {
                // Command port: return IRR or ISR based on OCW3 read mode
                if self.read_isr { self.isr } else { self.irr }
            }
            1 => self.imr, // Data port: interrupt mask
            _ => 0xFF,
        }
    }

    /// Port write.
    pub fn write(&mut self, port_bit: u16, data: u8) {
        match port_bit {
            0 => self.write_command(data),
            1 => self.write_data(data),
            _ => {}
        }
    }

    fn write_command(&mut self, data: u8) {
        if data & 0x10 != 0 {
            // ICW1: begin initialization sequence
            self.init_phase = 1;
            self.imr = 0;
            self.irr = 0;
            self.isr = 0;
            self.auto_eoi = false;
            self.read_isr = false;
        } else if data & 0x08 != 0 {
            // OCW3: set read register mode
            match data & 0x03 {
                0x02 => self.read_isr = false, // read IRR
                0x03 => self.read_isr = true,  // read ISR
                _ => {}
            }
        } else {
            // OCW2: EOI and priority commands
            match (data >> 5) & 0x07 {
                0x01 | 0x05 => {
                    // Non-specific EOI: clear the highest priority ISR bit
                    if self.isr != 0 {
                        let bit = 1u8 << self.isr.trailing_zeros();
                        self.isr &= !bit;
                    }
                }
                0x03 | 0x07 => {
                    // Specific EOI: clear the specified IR level
                    let level = data & 0x07;
                    self.isr &= !(1u8 << level);
                }
                _ => {}
            }
        }
    }

    fn write_data(&mut self, data: u8) {
        match self.init_phase {
            0 => {
                // OCW1: interrupt mask register
                self.imr = data;
            }
            1 => {
                // ICW2: interrupt vector base
                self.base_vector = data & 0xF8; // low 3 bits are ignored per spec
                self.init_phase = 2;
            }
            2 => {
                // ICW3: cascade configuration — we just skip it
                self.init_phase = 3;
            }
            3 => {
                // ICW4: operating mode
                self.auto_eoi = data & 0x02 != 0;
                self.init_phase = 0; // initialization complete
            }
            _ => {}
        }
    }

    /// Signal an IRQ (hardware interrupt request). Returns the INT vector
    /// if the interrupt is unmasked, or None if masked.
    pub fn assert_irq(&mut self, irq: u8) -> Option<u8> {
        let bit = 1u8 << irq;
        self.irr |= bit;
        if self.imr & bit == 0 {
            Some(self.base_vector + irq)
        } else {
            None
        }
    }
}

impl Default for Pic8259 {
    fn default() -> Self {
        Self::new(0x08)
    }
}
