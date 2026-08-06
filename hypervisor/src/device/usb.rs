//! USB UHCI (Universal Host Controller Interface) 1.1 controller and USB device emulation.
//!
//! Emulates a standard UHCI controller (PCI vendor 0x8086, device 0x7020 / I/O ports 0xC000–0xC01F)
//! powering virtual USB HID devices (USB Keyboard, USB Mouse) and USB passthrough.
//!
//! # Standard UHCI I/O Registers
//!
//! | Offset | Register                          |
//! |--------|-----------------------------------|
//! | 0x00   | USBCMD (Command Register)          |
//! | 0x02   | USBSTS (Status Register)           |
//! | 0x04   | USBINTR (Interrupt Enable)         |
//! | 0x06   | FRNUM (Frame Number)               |
//! | 0x08   | FLBASEADD (Frame List Base Addr)   |
//! | 0x0C   | SOFMOD (Start of Frame Modify)     |
//! | 0x10   | PORTSC1 (Port 1 Status/Control)    |
//! | 0x12   | PORTSC2 (Port 2 Status/Control)    |

use std::collections::VecDeque;

// PORTSC bits
const PORTSC_CONNECTION_STATUS: u16 = 1 << 0;
const PORTSC_CONNECT_CHANGE: u16 = 1 << 1;
const PORTSC_PORT_ENABLE: u16 = 1 << 2;
const PORTSC_ENABLE_CHANGE: u16 = 1 << 3;
const PORTSC_LOW_SPEED: u16 = 1 << 8;
const PORTSC_PORT_RESET: u16 = 1 << 9;

/// USB Device Descriptor types
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum UsbDescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    Hid = 0x21,
    Report = 0x22,
}

/// USB HID Event (Keyboard / Mouse input)
#[derive(Debug, Clone)]
pub enum UsbInputEvent {
    KeyboardKey { key_code: u8, pressed: bool },
    MouseMove { dx: i8, dy: i8, buttons: u8 },
}

/// Emulated USB UHCI Controller
#[derive(Debug)]
pub struct UsbController {
    cmd: u16,
    status: u16,
    intr_enable: u16,
    frame_num: u16,
    fl_base_addr: u32,
    sof_mod: u8,
    port1_status: u16,
    port2_status: u16,
    /// Pending USB input events (keyboard keys, mouse moves)
    pub input_events: VecDeque<UsbInputEvent>,
    /// Connected USB device passthrough identifiers
    pub passthrough_devices: Vec<String>,
}

impl UsbController {
    pub fn new() -> Self {
        Self {
            cmd: 0x0000,
            status: 0x0000,
            intr_enable: 0x000F,
            frame_num: 0x0000,
            fl_base_addr: 0x0000_0000,
            sof_mod: 0x40,
            // Port 1: Connected (Keyboard/Mouse HID hub), Enabled, Low speed
            port1_status: PORTSC_CONNECTION_STATUS | PORTSC_CONNECT_CHANGE | PORTSC_PORT_ENABLE | PORTSC_LOW_SPEED,
            // Port 2: Connected (USB Passthrough device slot), Enabled
            port2_status: PORTSC_CONNECTION_STATUS | PORTSC_CONNECT_CHANGE | PORTSC_PORT_ENABLE,
            input_events: VecDeque::with_capacity(128),
            passthrough_devices: Vec::new(),
        }
    }

    /// Read UHCI I/O Register (relative to base port 0xC000)
    pub fn read_io(&mut self, offset: u16, size: u8) -> u32 {
        let val = match offset {
            0x00 => self.cmd as u32,
            0x02 => self.status as u32,
            0x04 => self.intr_enable as u32,
            0x06 => {
                // Increment frame counter on each read (1 ms USB frame ticker simulation)
                self.frame_num = (self.frame_num + 1) & 0x07FF;
                self.frame_num as u32
            }
            0x08 => self.fl_base_addr,
            0x0C => self.sof_mod as u32,
            0x10 => self.port1_status as u32,
            0x12 => self.port2_status as u32,
            _ => 0x0000_0000,
        };
        match size {
            1 => val & 0xFF,
            2 => val & 0xFFFF,
            _ => val,
        }
    }

    /// Write UHCI I/O Register
    pub fn write_io(&mut self, offset: u16, data: u32, size: u8) {
        match offset {
            0x00 => {
                let val = data as u16;
                self.cmd = val;
                // Bit 1 = Host Controller Reset
                if val & 0x0002 != 0 {
                    self.cmd = 0x0000;
                    self.status = 0x0000;
                }
            }
            0x02 => {
                // Status bits are write-1-to-clear
                self.status &= !(data as u16);
            }
            0x04 => self.intr_enable = data as u16,
            0x06 => self.frame_num = (data as u16) & 0x07FF,
            0x08 => {
                if size == 4 {
                    self.fl_base_addr = data & 0xFFFF_F000;
                } else if offset == 0x08 {
                    self.fl_base_addr = (self.fl_base_addr & 0xFFFF_0000) | (data & 0xFFFF);
                } else {
                    self.fl_base_addr = (self.fl_base_addr & 0x0000_FFFF) | ((data & 0xFFFF) << 16);
                }
            }
            0x0C => self.sof_mod = data as u8,
            0x10 => {
                let val = data as u16;
                // Port reset processing
                if val & PORTSC_PORT_RESET != 0 {
                    self.port1_status |= PORTSC_PORT_ENABLE;
                    self.port1_status &= !PORTSC_PORT_RESET;
                }
                // Write-1-to-clear change status bits
                if val & PORTSC_CONNECT_CHANGE != 0 { self.port1_status &= !PORTSC_CONNECT_CHANGE; }
                if val & PORTSC_ENABLE_CHANGE != 0 { self.port1_status &= !PORTSC_ENABLE_CHANGE; }
            }
            0x12 => {
                let val = data as u16;
                if val & PORTSC_PORT_RESET != 0 {
                    self.port2_status |= PORTSC_PORT_ENABLE;
                    self.port2_status &= !PORTSC_PORT_RESET;
                }
                if val & PORTSC_CONNECT_CHANGE != 0 { self.port2_status &= !PORTSC_CONNECT_CHANGE; }
                if val & PORTSC_ENABLE_CHANGE != 0 { self.port2_status &= !PORTSC_ENABLE_CHANGE; }
            }
            _ => {}
        }
    }

    /// Inject a keyboard key event into the virtual USB HID keyboard pipeline
    pub fn inject_keyboard_key(&mut self, key_code: u8, pressed: bool) {
        self.input_events.push_back(UsbInputEvent::KeyboardKey { key_code, pressed });
        // Set USB status interrupt flag
        self.status |= 0x0001; // USBINT
    }

    /// Inject a mouse move event into the virtual USB HID mouse pipeline
    pub fn inject_mouse_move(&mut self, dx: i8, dy: i8, buttons: u8) {
        self.input_events.push_back(UsbInputEvent::MouseMove { dx, dy, buttons });
        self.status |= 0x0001;
    }
}

impl Default for UsbController {
    fn default() -> Self {
        Self::new()
    }
}
