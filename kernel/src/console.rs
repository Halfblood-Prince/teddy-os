use crate::{interrupts, port, vga};

const CONSOLE_LEFT: usize = 2;
const CONSOLE_TOP: usize = 20;
const CONSOLE_WIDTH: usize = 76;
const CONSOLE_HEIGHT: usize = 5;
const HISTORY_LINES: usize = CONSOLE_HEIGHT - 2;
const INPUT_LIMIT: usize = CONSOLE_WIDTH - 4;

pub struct Console {
    history: [[u8; CONSOLE_WIDTH]; HISTORY_LINES],
    history_len: [usize; HISTORY_LINES],
    next_line: usize,
    input: [u8; INPUT_LIMIT],
    input_len: usize,
}

impl Console {
    pub const fn new() -> Self {
        Self {
            history: [[b' '; CONSOLE_WIDTH]; HISTORY_LINES],
            history_len: [0; HISTORY_LINES],
            next_line: 0,
            input: [0; INPUT_LIMIT],
            input_len: 0,
        }
    }

    pub fn init(&mut self) {
        self.push_line("Polling console online");
        self.push_line("Commands: help, clear, ticks, about");
        self.render();
    }

    pub fn poll_input(&mut self) -> bool {
        if port::inb(0x64) & 0x01 == 0 {
            return false;
        }

        let scancode = port::inb(0x60);
        if scancode & 0x80 != 0 {
            return true;
        }

        let ascii = decode_scancode(scancode);
        interrupts::record_polled_key(scancode, ascii);
        match ascii {
            8 => self.backspace(),
            b'\n' => self.submit_command(),
            0x20..=0x7E => self.push_input(ascii),
            _ => {}
        }
        self.render();
        true
    }

    fn submit_command(&mut self) {
        self.push_prompt_line();
        let command = core::str::from_utf8(&self.input[..self.input_len]).unwrap_or("");
        match command {
            "" => {}
            "help" => self.push_line("help  clear  ticks  about"),
            "clear" => self.clear(),
            "ticks" => {
                let mut buffer = [0u8; 16];
                let len = format_hex_u32(&mut buffer, interrupts::timer_ticks() as u32);
                self.push_prefixed("ticks=0x", &buffer[..len]);
            }
            "about" => self.push_line("Teddy-OS polling console MVP"),
            _ => self.push_line("Unknown command"),
        }
        self.input_len = 0;
    }

    fn push_input(&mut self, byte: u8) {
        if self.input_len < INPUT_LIMIT {
            self.input[self.input_len] = byte;
            self.input_len += 1;
        }
    }

    fn backspace(&mut self) {
        if self.input_len > 0 {
            self.input_len -= 1;
        }
    }

    fn clear(&mut self) {
        self.history = [[b' '; CONSOLE_WIDTH]; HISTORY_LINES];
        self.history_len = [0; HISTORY_LINES];
        self.next_line = 0;
        self.input_len = 0;
    }

    fn push_prompt_line(&mut self) {
        let mut line = [b' '; CONSOLE_WIDTH];
        line[0] = b'>';
        line[1] = b' ';
        for index in 0..self.input_len {
            line[index + 2] = self.input[index];
        }
        self.push_line_bytes(&line, 2 + self.input_len);
    }

    fn push_line(&mut self, text: &str) {
        self.push_line_bytes(text.as_bytes(), text.len());
    }

    fn push_prefixed(&mut self, prefix: &str, suffix: &[u8]) {
        let mut line = [b' '; CONSOLE_WIDTH];
        let mut len = 0;

        for &byte in prefix.as_bytes() {
            if len >= CONSOLE_WIDTH {
                break;
            }
            line[len] = byte;
            len += 1;
        }
        for &byte in suffix {
            if len >= CONSOLE_WIDTH {
                break;
            }
            line[len] = byte;
            len += 1;
        }

        self.push_line_bytes(&line, len);
    }

    fn push_line_bytes(&mut self, bytes: &[u8], len: usize) {
        let row = self.next_line % HISTORY_LINES;
        self.history[row] = [b' '; CONSOLE_WIDTH];
        let copy_len = core::cmp::min(len, CONSOLE_WIDTH);
        for (index, byte) in bytes.iter().take(copy_len).enumerate() {
            self.history[row][index] = *byte;
        }
        self.history_len[row] = copy_len;
        self.next_line += 1;
    }

    pub fn render(&self) {
        draw_frame();
        let start = self.next_line.saturating_sub(HISTORY_LINES);
        for visible in 0..HISTORY_LINES {
            let row = CONSOLE_TOP + 1 + visible;
            vga::clear_row(row, 0x07);
            let source = start + visible;
            if source >= self.next_line {
                continue;
            }
            let index = source % HISTORY_LINES;
            let len = self.history_len[index];
            for col in 0..len {
                vga::write_ascii(row, CONSOLE_LEFT + 1 + col, self.history[index][col], 0x07);
            }
        }

        let input_row = CONSOLE_TOP + CONSOLE_HEIGHT - 1;
        vga::clear_row(input_row, 0x70);
        vga::write_ascii(input_row, CONSOLE_LEFT + 1, b'>', 0x70);
        vga::write_ascii(input_row, CONSOLE_LEFT + 2, b' ', 0x70);
        for index in 0..self.input_len {
            vga::write_ascii(input_row, CONSOLE_LEFT + 3 + index, self.input[index], 0x70);
        }
    }
}

fn draw_frame() {
    vga::write_line(CONSOLE_TOP, CONSOLE_LEFT, "Teddy-OS Polling Console", 0x1F);
    for row in CONSOLE_TOP + 1..(CONSOLE_TOP + CONSOLE_HEIGHT) {
        vga::clear_row(row, 0x07);
    }
}

fn decode_scancode(scancode: u8) -> u8 {
    match scancode {
        0x02 => b'1',
        0x03 => b'2',
        0x04 => b'3',
        0x05 => b'4',
        0x06 => b'5',
        0x07 => b'6',
        0x08 => b'7',
        0x09 => b'8',
        0x0A => b'9',
        0x0B => b'0',
        0x0E => 8,
        0x10 => b'q',
        0x11 => b'w',
        0x12 => b'e',
        0x13 => b'r',
        0x14 => b't',
        0x15 => b'y',
        0x16 => b'u',
        0x17 => b'i',
        0x18 => b'o',
        0x19 => b'p',
        0x1C => b'\n',
        0x1E => b'a',
        0x1F => b's',
        0x20 => b'd',
        0x21 => b'f',
        0x22 => b'g',
        0x23 => b'h',
        0x24 => b'j',
        0x25 => b'k',
        0x26 => b'l',
        0x2C => b'z',
        0x2D => b'x',
        0x2E => b'c',
        0x2F => b'v',
        0x30 => b'b',
        0x31 => b'n',
        0x32 => b'm',
        0x39 => b' ',
        _ => b'?',
    }
}

fn format_hex_u32(buffer: &mut [u8; 16], value: u32) -> usize {
    let mut started = false;
    let mut len = 0;
    for shift in (0..8).rev() {
        let nibble = ((value >> (shift * 4)) & 0x0F) as u8;
        if nibble != 0 || started || shift == 0 {
            buffer[len] = match nibble {
                0..=9 => b'0' + nibble,
                _ => b'A' + (nibble - 10),
            };
            len += 1;
            started = true;
        }
    }
    len
}
