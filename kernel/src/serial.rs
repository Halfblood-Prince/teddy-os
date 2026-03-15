use crate::port::{inb, outb};

const COM1_PORT: u16 = 0x3F8;

pub fn init() {
    unsafe {
        outb(COM1_PORT + 1, 0x00);
        outb(COM1_PORT + 3, 0x80);
        outb(COM1_PORT, 0x03);
        outb(COM1_PORT + 1, 0x00);
        outb(COM1_PORT + 3, 0x03);
        outb(COM1_PORT + 2, 0xC7);
        outb(COM1_PORT + 4, 0x0B);
    }
}

pub fn write_str(text: &str) {
    for byte in text.bytes() {
        write_byte(byte);
    }
}

pub fn write_line(text: &str) {
    write_str(text);
    write_str("\r\n");
}

pub fn write_hex_byte(label: &str, value: u8) {
    write_str(label);
    write_hex_u8(value);
    write_str("\r\n");
}

pub fn write_hex_word(label: &str, value: u16) {
    write_str(label);
    write_hex_u16(value);
    write_str("\r\n");
}

fn write_byte(byte: u8) {
    unsafe {
        while (inb(COM1_PORT + 5) & 0x20) == 0 {}
        outb(COM1_PORT, byte);
    }
}

fn write_hex_u8(value: u8) {
    write_nibble(value >> 4);
    write_nibble(value & 0x0F);
}

fn write_hex_u16(value: u16) {
    write_hex_u8((value >> 8) as u8);
    write_hex_u8(value as u8);
}

fn write_nibble(nibble: u8) {
    let byte = match nibble & 0x0F {
        0..=9 => b'0' + (nibble & 0x0F),
        _ => b'A' + ((nibble & 0x0F) - 10),
    };
    write_byte(byte);
}
