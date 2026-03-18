use crate::{boot_info::{BootInfo, FramebufferInfo}, interrupts};

pub struct GraphicsShell {
    fb: FramebufferInfo,
    uptime_seconds: u64,
    accent_phase: u8,
}

impl GraphicsShell {
    pub fn new(boot_info: BootInfo) -> Option<Self> {
        let fb = boot_info.framebuffer()?;
        if fb.bpp() != 8 {
            return None;
        }

        Some(Self {
            fb,
            uptime_seconds: 0,
            accent_phase: 0,
        })
    }

    pub fn render(&self) {
        self.draw_background();
        self.draw_top_bar();
        self.draw_taskbar();
        self.draw_window(18, 22, 284, 96, 1, "TEDDY-OS GUI");
        self.draw_window(34, 56, 120, 116, 3, "GRAPHICS READY");
        self.draw_window(168, 48, 120, 116, 3, "NEXT STEPS");

        self.draw_text(30, 42, 15, "PIXEL FRAMEBUFFER ONLINE");
        self.draw_text(30, 54, 15, "MODE 13H 320X200X8");
        self.draw_text(30, 66, 15, "THIS IS THE GUI SCAFFOLD");
        self.draw_text(30, 78, 15, "MOUSE INPUT COMES NEXT");

        self.draw_text(46, 76, 15, "WINDOW HIT TESTING");
        self.draw_text(46, 88, 15, "CURSOR COMPOSITING");
        self.draw_text(46, 100, 15, "DIRTY REGION REDRAWS");
        self.draw_text(180, 68, 15, "PS/2 MOUSE DRIVER");
        self.draw_text(180, 80, 15, "EVENT DISPATCH");
        self.draw_text(180, 92, 15, "CLICK + DRAG STATE");
        self.draw_text(180, 104, 15, "GUI APP PORTS");

        self.draw_status();
        self.draw_cursor_placeholder();
    }

    pub fn tick(&mut self, uptime_seconds: u64) {
        if self.uptime_seconds != uptime_seconds {
            self.uptime_seconds = uptime_seconds;
            self.accent_phase = self.accent_phase.wrapping_add(1) % 3;
            self.render();
        }
    }

    pub fn handle_key(&mut self, ascii: u8) {
        if ascii != b'?' {
            self.accent_phase = self.accent_phase.wrapping_add(1) % 3;
            self.draw_status();
            self.draw_cursor_placeholder();
        }
    }

    fn draw_background(&self) {
        let height = self.fb.height() as usize;
        let width = self.fb.width() as usize;
        let mut y = 0usize;
        while y < height {
            let color = if y < 70 {
                1
            } else if y < 140 {
                9
            } else {
                3
            };
            self.fill_rect(0, y as i32, width as i32, 1, color);
            y += 1;
        }

        self.fill_rect(0, 0, width as i32, 20, 8);
        self.fill_rect(0, (height as i32) - 18, width as i32, 18, 8);
    }

    fn draw_top_bar(&self) {
        self.draw_text(10, 6, 15, "TEDDY-OS GRAPHICS PREVIEW");
        self.draw_text(210, 6, 14, "GUI PHASE 1");
    }

    fn draw_taskbar(&self) {
        let accent = match self.accent_phase {
            0 => 12,
            1 => 13,
            _ => 10,
        };
        self.fill_rect(0, 182, self.fb.width() as i32, 18, 8);
        self.fill_rect(6, 185, 64, 10, accent);
        self.draw_text(12, 187, 15, "TEDDY");
        self.draw_text(88, 187, 15, "FRAMEBUFFER");
    }

    fn draw_window(&self, x: i32, y: i32, width: i32, height: i32, body: u8, title: &str) {
        self.fill_rect(x, y, width, height, body);
        self.draw_rect(x, y, width, height, 15);
        self.fill_rect(x + 1, y + 1, width - 2, 12, 8);
        self.draw_text(x + 6, y + 4, 15, title);
        self.fill_rect(x + width - 18, y + 3, 6, 6, 4);
        self.fill_rect(x + width - 10, y + 3, 6, 6, 14);
    }

    fn draw_status(&self) {
        self.fill_rect(18, 132, 284, 34, 1);
        self.draw_rect(18, 132, 284, 34, 15);
        self.draw_text(28, 140, 15, "UPTIME");
        self.draw_number(82, 140, self.uptime_seconds as u32, 14);
        self.draw_text(132, 140, 15, "LAST KEY");
        self.draw_ascii(198, 140, interrupts::last_ascii(), 14);
        self.draw_text(224, 140, 15, "SC");
        self.draw_hex_byte(244, 140, interrupts::last_scancode(), 14);
    }

    fn draw_cursor_placeholder(&self) {
        let color = match self.accent_phase {
            0 => 15,
            1 => 14,
            _ => 12,
        };
        self.fill_rect(270, 34, 10, 10, color);
        self.draw_rect(270, 34, 10, 10, 0);
    }

    fn draw_text(&self, x: i32, y: i32, color: u8, text: &str) {
        let bytes = text.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() {
            self.draw_char(x + (index as i32 * 6), y, bytes[index], color);
            index += 1;
        }
    }

    fn draw_char(&self, x: i32, y: i32, byte: u8, color: u8) {
        let glyph = glyph_for(byte);
        let mut row = 0usize;
        while row < glyph.len() {
            let bits = glyph[row];
            let mut col = 0usize;
            while col < 5 {
                if bits & (1 << (4 - col)) != 0 {
                    self.put_pixel(x + col as i32, y + row as i32, color);
                }
                col += 1;
            }
            row += 1;
        }
    }

    fn draw_ascii(&self, x: i32, y: i32, byte: u8, color: u8) {
        let rendered = match byte {
            0x20..=0x7E => byte,
            _ => b'?',
        };
        self.draw_char(x, y, rendered, color);
    }

    fn draw_hex_byte(&self, x: i32, y: i32, value: u8, color: u8) {
        let hi = nybble_to_hex((value >> 4) & 0x0F);
        let lo = nybble_to_hex(value & 0x0F);
        self.draw_char(x, y, hi, color);
        self.draw_char(x + 6, y, lo, color);
    }

    fn draw_number(&self, x: i32, y: i32, mut value: u32, color: u8) {
        if value == 0 {
            self.draw_char(x, y, b'0', color);
            return;
        }

        let mut scratch = [0u8; 10];
        let mut len = 0usize;
        while value > 0 {
            scratch[len] = b'0' + (value % 10) as u8;
            value /= 10;
            len += 1;
        }

        let mut index = 0usize;
        while index < len {
            self.draw_char(x + (index as i32 * 6), y, scratch[len - 1 - index], color);
            index += 1;
        }
    }

    fn fill_rect(&self, x: i32, y: i32, width: i32, height: i32, color: u8) {
        let mut yy = 0;
        while yy < height {
            let mut xx = 0;
            while xx < width {
                self.put_pixel(x + xx, y + yy, color);
                xx += 1;
            }
            yy += 1;
        }
    }

    fn draw_rect(&self, x: i32, y: i32, width: i32, height: i32, color: u8) {
        let mut xx = 0;
        while xx < width {
            self.put_pixel(x + xx, y, color);
            self.put_pixel(x + xx, y + height - 1, color);
            xx += 1;
        }
        let mut yy = 0;
        while yy < height {
            self.put_pixel(x, y + yy, color);
            self.put_pixel(x + width - 1, y + yy, color);
            yy += 1;
        }
    }

    fn put_pixel(&self, x: i32, y: i32, color: u8) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.fb.width() as usize || y >= self.fb.height() as usize {
            return;
        }

        let offset = y * self.fb.pitch() as usize + x;
        let ptr = self.fb.addr() as usize as *mut u8;
        unsafe {
            ptr.add(offset).write_volatile(color);
        }
    }
}

fn nybble_to_hex(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'A' + (value - 10),
    }
}

fn glyph_for(byte: u8) -> [u8; 7] {
    match to_upper(byte) {
        b'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        b'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        b'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        b'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        b'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        b'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
        b'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        b'J' => [0x1F, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0C],
        b'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        b'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        b'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        b'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        b'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        b'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        b'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        b'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        b'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        b'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        b'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        b'0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        b'1' => [0x04, 0x0C, 0x14, 0x04, 0x04, 0x04, 0x1F],
        b'2' => [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
        b'3' => [0x1F, 0x01, 0x02, 0x06, 0x01, 0x11, 0x0E],
        b'4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        b'5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        b'6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        b'7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        b'9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        b'-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        b':' => [0x00, 0x04, 0x00, 0x00, 0x04, 0x00, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        b'/' => [0x01, 0x02, 0x04, 0x04, 0x08, 0x10, 0x00],
        b'?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        b' ' => [0, 0, 0, 0, 0, 0, 0],
        _ => [0x1F, 0x11, 0x02, 0x04, 0x00, 0x04, 0x00],
    }
}

fn to_upper(byte: u8) -> u8 {
    if (b'a'..=b'z').contains(&byte) {
        byte - 32
    } else {
        byte
    }
}
