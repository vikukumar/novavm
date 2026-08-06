//! VGA controller emulation.
//!
//! Handles the VGA I/O registers (0x3C0–0x3DF) and tracks:
//! - Text mode (mode 3): 80×25 characters at guest physical 0xB8000.
//!   Each cell is 2 bytes: character + colour attribute.
//! - Graphics mode (mode 13h): 320×200 in 256 colours at 0xA0000.
//!
//! The VGA *memory* (framebuffer) is mapped directly as guest RAM so the guest
//! can write without causing MMIO exits. A background display thread reads
//! `text_buffer` every ~50 ms and updates the native window.

pub const VGA_TEXT_COLS: usize = 80;
pub const VGA_TEXT_ROWS: usize = 25;
pub const VGA_TEXT_FB_SIZE: usize = VGA_TEXT_COLS * VGA_TEXT_ROWS * 2; // char + attr
pub const VGA_GRAPHICS_W: usize = 320;
pub const VGA_GRAPHICS_H: usize = 200;

/// VGA controller state.
#[derive(Debug)]
pub struct VgaDevice {
    // ── CRTC (Cathode Ray Tube Controller) registers ──────────────────────────
    pub crtc_index: u8,
    pub crtc_regs: [u8; 25],
    /// Cursor position in cells (row * 80 + col).
    pub cursor_pos: u16,

    // ── Attribute Controller ──────────────────────────────────────────────────
    attr_index: u8,
    attr_data_mode: bool, // toggle: false=index, true=data
    attr_regs: [u8; 21],

    // ── Sequencer ─────────────────────────────────────────────────────────────
    seq_index: u8,
    seq_regs: [u8; 5],

    // ── Graphics Controller ───────────────────────────────────────────────────
    gfx_index: u8,
    gfx_regs: [u8; 9],

    // ── Miscellaneous / DAC ───────────────────────────────────────────────────
    misc_output: u8,
    dac_index_w: u8,
    dac_index_r: u8,
    dac_sub: u8,
    pub palette: [u8; 768], // 256 × (R, G, B)

    // ── Framebuffer state (shared with display thread via Arc<Mutex<Self>>) ───
    /// True when the controller is in a planar/graphics mode (not text mode 3).
    pub graphics_mode: bool,
    /// 80×25 text mode character + attribute buffer (mirrored from guest RAM).
    /// Written by `update_text_byte()` called from the vCPU MMIO handler.
    pub text_buffer: [u8; VGA_TEXT_FB_SIZE],
    /// 320×200 graphics pixel buffer (256-colour planar, mode 13h).
    pub gfx_buffer: Vec<u8>,

    // ── Input Status 1 toggle for vsync polling ───────────────────────────────
    status1_toggle: bool,
}

impl VgaDevice {
    pub fn new() -> Self {
        let mut text = [0u8; VGA_TEXT_FB_SIZE];
        // Pre-fill with spaces + default attribute (white on black = 0x07)
        for i in 0..VGA_TEXT_COLS * VGA_TEXT_ROWS {
            text[i * 2] = b' ';
            text[i * 2 + 1] = 0x07;
        }
        let mut pal = [0u8; 768];
        // Minimal VGA 16-colour palette (EGA colours)
        let ega_rgb: [u8; 48] = [
            0x00,0x00,0x00, 0x00,0x00,0xAA, 0x00,0xAA,0x00, 0x00,0xAA,0xAA,
            0xAA,0x00,0x00, 0xAA,0x00,0xAA, 0xAA,0x55,0x00, 0xAA,0xAA,0xAA,
            0x55,0x55,0x55, 0x55,0x55,0xFF, 0x55,0xFF,0x55, 0x55,0xFF,0xFF,
            0xFF,0x55,0x55, 0xFF,0x55,0xFF, 0xFF,0xFF,0x55, 0xFF,0xFF,0xFF,
        ];
        pal[..48].copy_from_slice(&ega_rgb);
        Self {
            crtc_index: 0,
            crtc_regs: [0u8; 25],
            cursor_pos: 0,
            attr_index: 0,
            attr_data_mode: false,
            attr_regs: [0u8; 21],
            seq_index: 0,
            seq_regs: [0u8; 5],
            gfx_index: 0,
            gfx_regs: [0u8; 9],
            misc_output: 0x23,
            dac_index_w: 0,
            dac_index_r: 0,
            dac_sub: 0,
            palette: pal,
            graphics_mode: false,
            text_buffer: text,
            gfx_buffer: vec![0u8; VGA_GRAPHICS_W * VGA_GRAPHICS_H],
            status1_toggle: false,
        }
    }

    // ── I/O Port Handlers ──────────────────────────────────────────────────────

    pub fn io_read(&mut self, port: u16) -> u8 {
        match port {
            0x3C0 => self.attr_index & 0x1F,
            0x3C1 => {
                let i = (self.attr_index & 0x1F) as usize;
                if i < 21 { self.attr_regs[i] } else { 0 }
            }
            0x3C2 | 0x3CC => self.misc_output,
            0x3C4 => self.seq_index,
            0x3C5 => {
                let i = self.seq_index as usize;
                if i < 5 { self.seq_regs[i] } else { 0 }
            }
            0x3C7 => 0, // DAC state register
            0x3C9 => {
                // DAC colour read (3 sub-reads per palette entry)
                let v = self.palette[self.dac_index_r as usize * 3 + self.dac_sub as usize];
                self.dac_sub = (self.dac_sub + 1) % 3;
                if self.dac_sub == 0 { self.dac_index_r = self.dac_index_r.wrapping_add(1); }
                v >> 2 // VGA delivers 6-bit values (0–63)
            }
            0x3CE => self.gfx_index,
            0x3CF => {
                let i = self.gfx_index as usize;
                if i < 9 { self.gfx_regs[i] } else { 0 }
            }
            0x3D4 | 0x3B4 => self.crtc_index,
            0x3D5 | 0x3B5 => match self.crtc_index {
                0x0E => ((self.cursor_pos >> 8) & 0xFF) as u8,
                0x0F => (self.cursor_pos & 0xFF) as u8,
                i if (i as usize) < 25 => self.crtc_regs[i as usize],
                _ => 0,
            },
            // Input Status Register 1: bit 3 = vsync, bit 0 = display enable
            // Toggle bits so software waiting for vsync doesn't loop forever.
            0x3DA | 0x3BA => {
                self.status1_toggle = !self.status1_toggle;
                if self.status1_toggle { 0x09 } else { 0x00 }
            }
            _ => 0xFF,
        }
    }

    pub fn io_write(&mut self, port: u16, data: u8) {
        match port {
            0x3C0 => {
                // Attribute controller: alternating index / data writes
                if !self.attr_data_mode {
                    self.attr_index = data & 0x1F;
                    self.attr_data_mode = true;
                } else {
                    let i = (self.attr_index & 0x1F) as usize;
                    if i < 21 { self.attr_regs[i] = data; }
                    self.attr_data_mode = false;
                }
            }
            0x3C2 => self.misc_output = data,
            0x3C4 => self.seq_index = data,
            0x3C5 => {
                let i = self.seq_index as usize;
                if i < 5 { self.seq_regs[i] = data; }
            }
            0x3C8 => {
                self.dac_index_w = data;
                self.dac_sub = 0;
            }
            0x3C9 => {
                // DAC colour write (6-bit R/G/B, stored as 8-bit)
                let slot = (self.dac_index_w as usize) * 3 + self.dac_sub as usize;
                if slot < 768 { self.palette[slot] = (data & 0x3F) << 2; }
                self.dac_sub = (self.dac_sub + 1) % 3;
                if self.dac_sub == 0 { self.dac_index_w = self.dac_index_w.wrapping_add(1); }
            }
            0x3CE => self.gfx_index = data,
            0x3CF => {
                let i = self.gfx_index as usize;
                if i < 9 { self.gfx_regs[i] = data; }
            }
            0x3D4 | 0x3B4 => self.crtc_index = data,
            0x3D5 | 0x3B5 => match self.crtc_index {
                0x0E => {
                    self.cursor_pos = (self.cursor_pos & 0x00FF) | ((data as u16) << 8);
                }
                0x0F => {
                    self.cursor_pos = (self.cursor_pos & 0xFF00) | (data as u16);
                }
                i if (i as usize) < 25 => self.crtc_regs[i as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }

    // ── Framebuffer helpers called from vCPU MMIO / direct RAM read ───────────

    /// Update one byte in the VGA text framebuffer.
    /// `offset` is relative to 0xB8000.
    pub fn write_text_byte(&mut self, offset: usize, data: u8) {
        if offset < VGA_TEXT_FB_SIZE {
            self.text_buffer[offset] = data;
        }
    }

    /// Read one byte of the graphics pixel buffer (mode 13h, relative to 0xA0000).
    pub fn read_gfx_byte(&self, offset: usize) -> u8 {
        self.gfx_buffer.get(offset).copied().unwrap_or(0)
    }

    /// Write one byte to the graphics pixel buffer.
    pub fn write_gfx_byte(&mut self, offset: usize, data: u8) {
        if let Some(b) = self.gfx_buffer.get_mut(offset) {
            *b = data;
        }
    }

    // ── Display rendering helpers ─────────────────────────────────────────────

    /// Render the text framebuffer as a plain UTF-8 string (for the console tab).
    pub fn render_text(&self) -> String {
        let mut out = String::with_capacity((VGA_TEXT_COLS + 1) * VGA_TEXT_ROWS);
        for row in 0..VGA_TEXT_ROWS {
            for col in 0..VGA_TEXT_COLS {
                let ch = self.text_buffer[(row * VGA_TEXT_COLS + col) * 2];
                out.push(if ch >= 0x20 && ch < 0x7F { ch as char } else { ' ' });
            }
            out.push('\n');
        }
        out
    }
}

impl Default for VgaDevice {
    fn default() -> Self {
        Self::new()
    }
}
