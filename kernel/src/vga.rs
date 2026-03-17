const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

pub const fn width() -> usize {
    VGA_WIDTH
}

pub fn clear_screen(attribute: u8) {
    for row in 0..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            write_cell(row, col, b' ', attribute);
        }
    }
}

pub fn write_line(row: usize, col: usize, text: &str, attribute: u8) {
    if row >= VGA_HEIGHT || col >= VGA_WIDTH {
        return;
    }
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let x = col + index;
        if x >= VGA_WIDTH {
            break;
        }
        write_cell(row, x, bytes[index], attribute);
        index += 1;
    }
}

pub fn fill_rect(row: usize, col: usize, height: usize, width: usize, byte: u8, attribute: u8) {
    if row >= VGA_HEIGHT || col >= VGA_WIDTH {
        return;
    }
    let max_row = core::cmp::min(row.saturating_add(height), VGA_HEIGHT);
    let max_col = core::cmp::min(col.saturating_add(width), VGA_WIDTH);
    for y in row..max_row {
        for x in col..max_col {
            write_cell(y, x, byte, attribute);
        }
    }
}

pub fn draw_box(row: usize, col: usize, height: usize, width: usize, attribute: u8) {
    if height < 2 || width < 2 || row >= VGA_HEIGHT || col >= VGA_WIDTH {
        return;
    }

    let bottom = core::cmp::min(row.saturating_add(height - 1), VGA_HEIGHT - 1);
    let right = core::cmp::min(col.saturating_add(width - 1), VGA_WIDTH - 1);
    if bottom <= row || right <= col {
        return;
    }

    for x in col + 1..right {
        write_cell(row, x, b'-', attribute);
        write_cell(bottom, x, b'-', attribute);
    }
    for y in row + 1..bottom {
        write_cell(y, col, b'|', attribute);
        write_cell(y, right, b'|', attribute);
    }

    write_cell(row, col, b'+', attribute);
    write_cell(row, right, b'+', attribute);
    write_cell(bottom, col, b'+', attribute);
    write_cell(bottom, right, b'+', attribute);
}

pub fn write_hex_byte(row: usize, col: usize, label: &str, value: u8, attribute: u8) {
    write_line(row, col, label, attribute);
    let start = col + label.len();
    write_hex_digits(row, start, value as u64, 2, attribute);
}

pub fn write_hex_word(row: usize, col: usize, label: &str, value: u16, attribute: u8) {
    write_line(row, col, label, attribute);
    let start = col + label.len();
    write_hex_digits(row, start, value as u64, 4, attribute);
}

pub fn write_hex_dword(row: usize, col: usize, value: u32, attribute: u8) {
    write_hex_digits(row, col, value as u64, 8, attribute);
}

pub fn write_hex_qword(row: usize, col: usize, value: u64, attribute: u8) {
    write_hex_digits(row, col, value, 16, attribute);
}

pub fn write_ascii(row: usize, col: usize, value: u8, attribute: u8) {
    if row >= VGA_HEIGHT || col >= VGA_WIDTH {
        return;
    }
    let byte = match value {
        b'\n' => 0x14,
        0x20..=0x7E => value,
        _ => b'.',
    };
    write_cell(row, col, byte, attribute);
}

fn write_hex_digits(row: usize, col: usize, value: u64, digits: usize, attribute: u8) {
    if row >= VGA_HEIGHT || col >= VGA_WIDTH {
        return;
    }
    for index in 0..digits {
        let x = col + index;
        if x >= VGA_WIDTH {
            break;
        }
        let shift = (digits - 1 - index) * 4;
        let nibble = ((value >> shift) & 0x0F) as u8;
        let byte = match nibble {
            0..=9 => b'0' + nibble,
            _ => b'A' + (nibble - 10),
        };
        write_cell(row, x, byte, attribute);
    }
}

fn write_cell(row: usize, col: usize, byte: u8, attribute: u8) {
    if row >= VGA_HEIGHT || col >= VGA_WIDTH {
        return;
    }
    let index = (row * VGA_WIDTH + col) * 2;
    unsafe {
        VGA_BUFFER.add(index).write_volatile(byte);
        VGA_BUFFER.add(index + 1).write_volatile(attribute);
    }
}
