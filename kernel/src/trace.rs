use core::sync::atomic::{AtomicU16, AtomicU32, AtomicU8, Ordering};

use crate::font;

static BOOT_STAGE: AtomicU8 = AtomicU8::new(0);
static FB_ADDR: AtomicU32 = AtomicU32::new(0);
static FB_WIDTH: AtomicU16 = AtomicU16::new(0);
static FB_HEIGHT: AtomicU16 = AtomicU16::new(0);
static FB_PITCH: AtomicU16 = AtomicU16::new(0);
static FB_BPP: AtomicU8 = AtomicU8::new(0);

pub fn set_boot_stage(stage: u8) {
    BOOT_STAGE.store(stage, Ordering::Relaxed);
}

pub fn boot_stage() -> u8 {
    BOOT_STAGE.load(Ordering::Relaxed)
}

pub fn set_framebuffer(addr: u32, width: u16, height: u16, pitch: u16, bpp: u8) {
    FB_ADDR.store(addr, Ordering::Relaxed);
    FB_WIDTH.store(width, Ordering::Relaxed);
    FB_HEIGHT.store(height, Ordering::Relaxed);
    FB_PITCH.store(pitch, Ordering::Relaxed);
    FB_BPP.store(bpp, Ordering::Relaxed);
}

#[allow(dead_code)]
pub fn clear_framebuffer() {
    set_framebuffer(0, 0, 0, 0, 0);
}

pub fn render_graphics_panic(title: &str, detail_a: &str, detail_b: &str) {
    let fb = match framebuffer() {
        Some(fb) => fb,
        None => return,
    };

    fill_screen(fb, 0x0000AA);
    draw_text(fb, 14, 14, 0xFFFFFF, title);
    draw_text(fb, 14, 30, 0xFFFF55, detail_a);
    draw_text(fb, 14, 46, 0xFFFF55, detail_b);
    draw_text(fb, 14, 62, 0xFFFFFF, "BOOT STAGE");
    draw_hex_byte(fb, 92, 62, boot_stage(), 0x55FFFF);
}

pub fn render_graphics_exception(vector: u8, error_code: u64, rip: u64) {
    let fb = match framebuffer() {
        Some(fb) => fb,
        None => return,
    };

    fill_screen(fb, 0xAA0000);
    draw_text(fb, 14, 14, 0xFFFFFF, "TEDDY-OS EXCEPTION");
    draw_text(fb, 14, 30, 0xFFFF55, "VECTOR");
    draw_hex_byte(fb, 74, 30, vector, 0xFFFFFF);
    draw_text(fb, 14, 46, 0xFFFF55, "ERROR");
    draw_hex_dword(fb, 68, 46, error_code as u32, 0xFFFFFF);
    draw_text(fb, 14, 62, 0xFFFF55, "STAGE");
    draw_hex_byte(fb, 68, 62, boot_stage(), 0x55FFFF);
    draw_text(fb, 14, 78, 0xFFFF55, "RIP");
    draw_hex_qword(fb, 50, 78, rip, 0xFFFFFF);
}

#[derive(Clone, Copy)]
struct Framebuffer {
    addr: u32,
    width: u16,
    height: u16,
    pitch: u16,
    bpp: u8,
}

fn framebuffer() -> Option<Framebuffer> {
    let addr = FB_ADDR.load(Ordering::Relaxed);
    if addr == 0 {
        return None;
    }

    Some(Framebuffer {
        addr,
        width: FB_WIDTH.load(Ordering::Relaxed),
        height: FB_HEIGHT.load(Ordering::Relaxed),
        pitch: FB_PITCH.load(Ordering::Relaxed),
        bpp: FB_BPP.load(Ordering::Relaxed),
    })
}

fn fill_screen(fb: Framebuffer, color: u32) {
    let mut y = 0usize;
    while y < fb.height as usize {
        let mut x = 0usize;
        while x < fb.width as usize {
            put_pixel(fb, x as i32, y as i32, color);
            x += 1;
        }
        y += 1;
    }
}

fn draw_text(fb: Framebuffer, x: i32, y: i32, color: u32, text: &str) {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        draw_char(fb, x + (index as i32 * font::GLYPH_ADVANCE), y, bytes[index], color);
        index += 1;
    }
}

fn draw_char(fb: Framebuffer, x: i32, y: i32, byte: u8, color: u32) {
    let glyph = font::glyph_for(byte);
    let mut row = 0usize;
    while row < glyph.len() {
        let bits = glyph[row];
        let mut col = 0usize;
        while col < font::GLYPH_WIDTH {
            if bits & (1 << (font::GLYPH_WIDTH - 1 - col)) != 0 {
                put_pixel(fb, x + col as i32, y + row as i32, color);
            }
            col += 1;
        }
        row += 1;
    }
}

fn draw_hex_byte(fb: Framebuffer, x: i32, y: i32, value: u8, color: u32) {
    draw_char(fb, x, y, hex_nybble((value >> 4) & 0x0F), color);
    draw_char(fb, x + 6, y, hex_nybble(value & 0x0F), color);
}

fn draw_hex_dword(fb: Framebuffer, x: i32, y: i32, value: u32, color: u32) {
    let mut shift = 28i32;
    let mut col = 0i32;
    while shift >= 0 {
        let digit = ((value >> shift) & 0x0F) as u8;
        draw_char(fb, x + col, y, hex_nybble(digit), color);
        shift -= 4;
        col += 6;
    }
}

fn draw_hex_qword(fb: Framebuffer, x: i32, y: i32, value: u64, color: u32) {
    let mut shift = 60i32;
    let mut col = 0i32;
    while shift >= 0 {
        let digit = ((value >> shift) & 0x0F) as u8;
        draw_char(fb, x + col, y, hex_nybble(digit), color);
        shift -= 4;
        col += 6;
    }
}

fn hex_nybble(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'A' + (value - 10),
    }
}

fn put_pixel(fb: Framebuffer, x: i32, y: i32, color: u32) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    if x >= fb.width as usize || y >= fb.height as usize {
        return;
    }

    let bytes_per_pixel = match fb.bpp {
        24 => 3,
        32 => 4,
        _ => 1,
    };
    let offset = y * fb.pitch as usize + x * bytes_per_pixel;
        let ptr = fb.addr as usize as *mut u8;
    unsafe {
        match fb.bpp {
            8 => ptr.add(offset).write_volatile(vga_palette_index(color)),
            24 => {
                ptr.add(offset).write_volatile((color & 0xFF) as u8);
                ptr.add(offset + 1).write_volatile(((color >> 8) & 0xFF) as u8);
                ptr.add(offset + 2).write_volatile(((color >> 16) & 0xFF) as u8);
            }
            32 => (ptr.add(offset) as *mut u32).write_volatile(color),
            _ => {}
        }
    }
}

fn vga_palette_index(color: u32) -> u8 {
    match color {
        0x0000AA => 0x01,
        0xAA0000 => 0x04,
        0xFFFFFF => 0x0F,
        0xFFFF55 => 0x0E,
        0x55FFFF => 0x0B,
        _ => 0x07,
    }
}
