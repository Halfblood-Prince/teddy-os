use crate::{interrupts, vga};

const TERM_LEFT: usize = 2;
const TERM_TOP: usize = 20;
const TERM_WIDTH: usize = 76;
const TERM_HEIGHT: usize = 5;
const HISTORY_LINES: usize = TERM_HEIGHT - 2;
const INPUT_LIMIT: usize = TERM_WIDTH - 4;

pub struct Terminal {
    history: [[u8; TERM_WIDTH]; HISTORY_LINES],
    history_len: [usize; HISTORY_LINES],
    next_line: usize,
    input: [u8; INPUT_LIMIT],
    input_len: usize,
}

impl Terminal {
    pub const fn new() -> Self {
        Self {
            history: [[b' '; TERM_WIDTH]; HISTORY_LINES],
            history_len: [0; HISTORY_LINES],
            next_line: 0,
            input: [0; INPUT_LIMIT],
            input_len: 0,
        }
    }

    pub fn init(&mut self) {
        self.clear();
        self.push_line("Kernel terminal online");
        self.push_line("Commands: help, clear, ticks, key, about");
        self.render();
    }

    pub fn handle_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.submit_command(),
            8 => self.backspace(),
            0x20..=0x7E => self.push_input(byte),
            _ => {}
        }
        self.render();
    }

    fn submit_command(&mut self) {
        self.push_prompt_line();

        let command = core::str::from_utf8(&self.input[..self.input_len]).unwrap_or("");
        match command {
            "" => {}
            "help" => self.push_line("help  clear  ticks  key  about"),
            "clear" => self.clear(),
            "ticks" => {
                let mut buffer = [0u8; 16];
                let len = format_hex_u32(&mut buffer, interrupts::timer_ticks() as u32);
                self.push_prefixed("ticks=0x", &buffer[..len]);
            }
            "key" => {
                let mut scan = [0u8; 2];
                let scan_len = format_hex_u8(&mut scan, interrupts::last_scancode());
                self.push_prefixed("last key=0x", &scan[..scan_len]);
                self.push_prefixed("last ascii=", &[interrupts::last_ascii()]);
            }
            "about" => self.push_line("Teddy-OS kernel terminal MVP"),
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
        self.history = [[b' '; TERM_WIDTH]; HISTORY_LINES];
        self.history_len = [0; HISTORY_LINES];
        self.next_line = 0;
        self.input_len = 0;
    }

    fn push_prompt_line(&mut self) {
        let mut line = [b' '; TERM_WIDTH];
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
        let mut line = [b' '; TERM_WIDTH];
        let mut len = 0;

        for &byte in prefix.as_bytes() {
            if len >= TERM_WIDTH {
                break;
            }
            line[len] = byte;
            len += 1;
        }
        for &byte in suffix {
            if len >= TERM_WIDTH {
                break;
            }
            line[len] = byte;
            len += 1;
        }

        self.push_line_bytes(&line, len);
    }

    fn push_line_bytes(&mut self, bytes: &[u8], len: usize) {
        let row = self.next_line % HISTORY_LINES;
        self.history[row] = [b' '; TERM_WIDTH];
        let copy_len = core::cmp::min(len, TERM_WIDTH);
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
            let row = TERM_TOP + 1 + visible;
            vga::clear_row(row, 0x07);
            let source = start + visible;
            if source >= self.next_line {
                continue;
            }
            let index = source % HISTORY_LINES;
            let len = self.history_len[index];
            for col in 0..len {
                vga::write_ascii(row, TERM_LEFT + 1 + col, self.history[index][col], 0x07);
            }
        }

        let input_row = TERM_TOP + TERM_HEIGHT - 1;
        vga::clear_row(input_row, 0x70);
        vga::write_ascii(input_row, TERM_LEFT + 1, b'>', 0x70);
        vga::write_ascii(input_row, TERM_LEFT + 2, b' ', 0x70);
        for index in 0..self.input_len {
            vga::write_ascii(input_row, TERM_LEFT + 3 + index, self.input[index], 0x70);
        }
    }
}

fn draw_frame() {
    vga::write_line(TERM_TOP, TERM_LEFT, "Teddy-OS Kernel Terminal", 0x1F);
    for row in TERM_TOP + 1..(TERM_TOP + TERM_HEIGHT) {
        vga::clear_row(row, 0x07);
    }
}

fn format_hex_u8(buffer: &mut [u8; 2], value: u8) -> usize {
    buffer[0] = hex_digit((value >> 4) & 0x0F);
    buffer[1] = hex_digit(value & 0x0F);
    2
}

fn format_hex_u32(buffer: &mut [u8; 16], value: u32) -> usize {
    let mut started = false;
    let mut len = 0;
    for shift in (0..8).rev() {
        let nibble = ((value >> (shift * 4)) & 0x0F) as u8;
        if nibble != 0 || started || shift == 0 {
            buffer[len] = hex_digit(nibble);
            len += 1;
            started = true;
        }
    }
    len
}

fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'A' + (value - 10),
    }
}
