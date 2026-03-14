use core::ptr;

use font8x8::{BASIC_FONTS, UnicodeFonts};
use teddy_boot_proto::{FramebufferInfo, PixelFormat};

const CHAR_WIDTH: usize = 8;
const CHAR_HEIGHT: usize = 16;

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub struct FramebufferConsole {
    info: FramebufferInfo,
    cursor_x: usize,
    cursor_y: usize,
    fg: Color,
    bg: Color,
}

impl FramebufferConsole {
    pub fn new(info: FramebufferInfo) -> Option<Self> {
        if !info.is_valid() {
            return None;
        }

        Some(Self {
            info,
            cursor_x: 0,
            cursor_y: 0,
            fg: Color::rgb(0xF1, 0xF5, 0xF9),
            bg: Color::rgb(0x0E, 0x1B, 0x2A),
        })
    }

    pub fn clear(&mut self, color: Color) {
        self.bg = color;
        for y in 0..self.info.height as usize {
            for x in 0..self.info.width as usize {
                self.write_pixel(x, y, color);
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn write_str(&mut self, text: &str) {
        for ch in text.chars() {
            self.write_char(ch);
        }
    }

    fn write_char(&mut self, ch: char) {
        match ch {
            '\n' => self.new_line(),
            '\r' => self.cursor_x = 0,
            _ => {
                self.draw_glyph(ch, self.cursor_x, self.cursor_y);
                self.cursor_x += CHAR_WIDTH;
                if self.cursor_x + CHAR_WIDTH >= self.info.width as usize {
                    self.new_line();
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += CHAR_HEIGHT;
        if self.cursor_y + CHAR_HEIGHT >= self.info.height as usize {
            self.clear(self.bg);
        }
    }

    fn draw_glyph(&mut self, ch: char, x: usize, y: usize) {
        let fallback = BASIC_FONTS.get('?').unwrap();
        let glyph = BASIC_FONTS.get(ch).unwrap_or(fallback);

        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..8 {
                let mask = 1u8 << col;
                let color = if bits & mask != 0 { self.fg } else { self.bg };
                let px = x + col;
                let py = y + row * 2;

                self.write_pixel(px, py, color);
                self.write_pixel(px, py + 1, color);
            }
        }
    }

    fn write_pixel(&self, x: usize, y: usize, color: Color) {
        if x >= self.info.width as usize || y >= self.info.height as usize {
            return;
        }

        let pixel_index = y * self.info.stride as usize + x;
        let pixel_ptr = self.info.base as *mut u32;

        let value = match self.info.format {
            PixelFormat::Rgb => {
                (color.r as u32) << 16 | (color.g as u32) << 8 | color.b as u32
            }
            PixelFormat::Bgr => {
                (color.b as u32) << 16 | (color.g as u32) << 8 | color.r as u32
            }
            _ => (color.r as u32) << 16 | (color.g as u32) << 8 | color.b as u32,
        };

        unsafe {
            ptr::write_volatile(pixel_ptr.add(pixel_index), value);
        }
    }
}

