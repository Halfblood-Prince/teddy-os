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

#[derive(Clone, Copy)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

pub struct FramebufferSurface {
    info: FramebufferInfo,
}

impl FramebufferSurface {
    pub fn new(info: FramebufferInfo) -> Option<Self> {
        if !info.is_valid() {
            return None;
        }

        Some(Self { info })
    }

    pub fn info(&self) -> FramebufferInfo {
        self.info
    }

    pub fn clear(&mut self, color: Color) {
        self.fill_rect(
            Rect {
                x: 0,
                y: 0,
                width: self.info.width as usize,
                height: self.info.height as usize,
            },
            color,
        );
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let max_x = rect.x.saturating_add(rect.width).min(self.info.width as usize);
        let max_y = rect.y.saturating_add(rect.height).min(self.info.height as usize);

        for y in rect.y..max_y {
            for x in rect.x..max_x {
                self.write_pixel(x, y, color);
            }
        }
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Color) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }

        self.fill_rect(
            Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: 1,
            },
            color,
        );
        self.fill_rect(
            Rect {
                x: rect.x,
                y: rect.y + rect.height.saturating_sub(1),
                width: rect.width,
                height: 1,
            },
            color,
        );
        self.fill_rect(
            Rect {
                x: rect.x,
                y: rect.y,
                width: 1,
                height: rect.height,
            },
            color,
        );
        self.fill_rect(
            Rect {
                x: rect.x + rect.width.saturating_sub(1),
                y: rect.y,
                width: 1,
                height: rect.height,
            },
            color,
        );
    }

    pub fn draw_line(&mut self, start: Point, end: Point, color: Color) {
        let mut x0 = start.x as isize;
        let mut y0 = start.y as isize;
        let x1 = end.x as isize;
        let y1 = end.y as isize;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.write_pixel(x0 as usize, y0 as usize, color);
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let error_twice = err * 2;
            if error_twice >= dy {
                err += dy;
                x0 += sx;
            }
            if error_twice <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_char(&mut self, ch: char, x: usize, y: usize, fg: Color, bg: Color) {
        let fallback = BASIC_FONTS.get('?').unwrap();
        let glyph = BASIC_FONTS.get(ch).unwrap_or(fallback);

        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..8 {
                let mask = 1u8 << col;
                let color = if bits & mask != 0 { fg } else { bg };
                let px = x + col;
                let py = y + row * 2;

                self.write_pixel(px, py, color);
                self.write_pixel(px, py + 1, color);
            }
        }
    }

    pub fn draw_text(&mut self, text: &str, x: usize, y: usize, fg: Color, bg: Color) {
        let mut cursor_x = x;
        let mut cursor_y = y;

        for ch in text.chars() {
            match ch {
                '\n' => {
                    cursor_x = x;
                    cursor_y += CHAR_HEIGHT;
                }
                '\r' => cursor_x = x,
                _ => {
                    self.draw_char(ch, cursor_x, cursor_y, fg, bg);
                    cursor_x += CHAR_WIDTH;
                }
            }
        }
    }

    pub fn write_pixel(&self, x: usize, y: usize, color: Color) {
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

pub struct FramebufferConsole {
    surface: FramebufferSurface,
    cursor_x: usize,
    cursor_y: usize,
    fg: Color,
    bg: Color,
}

impl FramebufferConsole {
    pub fn new(info: FramebufferInfo) -> Option<Self> {
        Some(Self {
            surface: FramebufferSurface::new(info)?,
            cursor_x: 0,
            cursor_y: 0,
            fg: Color::rgb(0xF1, 0xF5, 0xF9),
            bg: Color::rgb(0x0E, 0x1B, 0x2A),
        })
    }

    pub fn clear(&mut self, color: Color) {
        self.bg = color;
        self.surface.clear(color);
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
                self.surface
                    .draw_char(ch, self.cursor_x, self.cursor_y, self.fg, self.bg);
                self.cursor_x += CHAR_WIDTH;
                if self.cursor_x + CHAR_WIDTH >= self.surface.info().width as usize {
                    self.new_line();
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += CHAR_HEIGHT;
        if self.cursor_y + CHAR_HEIGHT >= self.surface.info().height as usize {
            self.clear(self.bg);
        }
    }
}
