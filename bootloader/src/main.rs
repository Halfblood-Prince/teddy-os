#![no_main]
#![no_std]

use uefi::boot;
use uefi::prelude::*;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};

#[entry]
fn efi_main() -> Status {
    if uefi::helpers::init().is_err() {
        return Status::LOAD_ERROR;
    }

    match BootApp::new().and_then(|mut app| app.run()) {
        Ok(()) => Status::SUCCESS,
        Err(status) => status,
    }
}

struct BootApp {
    framebuffer: Framebuffer,
}

impl BootApp {
    fn new() -> Result<Self, Status> {
        let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()
            .map_err(|err| err.status())?;
        let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)
            .map_err(|err| err.status())?;
        let framebuffer = Framebuffer::from_gop(&mut gop)?;
        Ok(Self { framebuffer })
    }

    fn run(&mut self) -> Result<(), Status> {
        self.draw_desktop();
        loop {
            boot::stall(200_000);
        }
    }

    fn draw_desktop(&mut self) {
        let surface = &mut self.framebuffer;
        surface.clear(Color::rgb(24, 48, 78));

        let width = surface.width;
        let height = surface.height;
        let taskbar_height = 56usize.min(height);

        surface.fill_rect(Rect::new(0, 0, width, height), Color::rgb(28, 56, 92));
        surface.fill_rect(
            Rect::new(0, height.saturating_sub(taskbar_height), width, taskbar_height),
            Color::rgb(214, 221, 229),
        );
        surface.fill_rect(
            Rect::new(18, height.saturating_sub(taskbar_height) + 10, 132, 36),
            Color::rgb(34, 92, 68),
        );
        surface.fill_rect(
            Rect::new(width.saturating_sub(260), 60, 220, 140),
            Color::rgb(243, 246, 250),
        );
        surface.stroke_rect(
            Rect::new(width.saturating_sub(260), 60, 220, 140),
            Color::rgb(70, 92, 118),
        );
        surface.fill_rect(
            Rect::new(width.saturating_sub(260), 60, 220, 28),
            Color::rgb(64, 116, 174),
        );
        surface.fill_rect(Rect::new(40, 56, 96, 96), Color::rgb(246, 196, 72));
        surface.fill_rect(Rect::new(40, 176, 96, 96), Color::rgb(109, 176, 140));
        surface.fill_rect(Rect::new(160, 56, 96, 96), Color::rgb(236, 140, 108));

        surface.draw_text("TEDDY-OS", 24, 18, 4, Color::rgb(247, 249, 252));
        surface.draw_text(
            "RESET BUILD",
            width.saturating_sub(236),
            68,
            3,
            Color::rgb(255, 255, 255),
        );
        surface.draw_text(
            "BOOT OK",
            width.saturating_sub(228),
            104,
            3,
            Color::rgb(37, 57, 84),
        );
        surface.draw_text(
            "VMWARE UEFI",
            width.saturating_sub(228),
            136,
            2,
            Color::rgb(37, 57, 84),
        );
        surface.draw_text(
            "START",
            42,
            height.saturating_sub(taskbar_height) + 18,
            3,
            Color::rgb(255, 255, 255),
        );
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Rect {
    const fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self { x, y, width, height }
    }
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

struct Framebuffer {
    base: *mut u8,
    size: usize,
    width: usize,
    height: usize,
    stride: usize,
    format: PixelFormat,
}

impl Framebuffer {
    fn from_gop(gop: &mut GraphicsOutput) -> Result<Self, Status> {
        let mode = select_mode(gop).ok_or(Status::NOT_FOUND)?;
        gop.set_mode(&mode).map_err(|err| err.status())?;
        let info = gop.current_mode_info();
        let (width, height) = info.resolution();
        let mut fb = gop.frame_buffer();
        Ok(Self {
            base: fb.as_mut_ptr(),
            size: fb.size(),
            width,
            height,
            stride: info.stride(),
            format: info.pixel_format(),
        })
    }

    fn clear(&mut self, color: Color) {
        self.fill_rect(Rect::new(0, 0, self.width, self.height), color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let max_x = rect.x.saturating_add(rect.width).min(self.width);
        let max_y = rect.y.saturating_add(rect.height).min(self.height);
        for y in rect.y..max_y {
            for x in rect.x..max_x {
                self.put_pixel(x, y, color);
            }
        }
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        self.fill_rect(Rect::new(rect.x, rect.y, rect.width, 1), color);
        self.fill_rect(
            Rect::new(rect.x, rect.y + rect.height.saturating_sub(1), rect.width, 1),
            color,
        );
        self.fill_rect(Rect::new(rect.x, rect.y, 1, rect.height), color);
        self.fill_rect(
            Rect::new(rect.x + rect.width.saturating_sub(1), rect.y, 1, rect.height),
            color,
        );
    }

    fn draw_text(&mut self, text: &str, x: usize, y: usize, scale: usize, color: Color) {
        let mut cursor_x = x;
        for ch in text.bytes() {
            if ch == b' ' {
                cursor_x += 6 * scale;
                continue;
            }

            let glyph = glyph(ch);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) == 0 {
                        continue;
                    }
                    self.fill_rect(
                        Rect::new(
                            cursor_x + col * scale,
                            y + row * scale,
                            scale,
                            scale,
                        ),
                        color,
                    );
                }
            }
            cursor_x += 6 * scale;
        }
    }

    fn put_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }

        let offset = (y * self.stride + x) * 4;
        if offset + 3 >= self.size {
            return;
        }

        unsafe {
            let pixel = self.base.add(offset);
            match self.format {
                PixelFormat::Rgb => {
                    *pixel = color.r;
                    *pixel.add(1) = color.g;
                    *pixel.add(2) = color.b;
                    *pixel.add(3) = 0;
                }
                PixelFormat::Bgr | _ => {
                    *pixel = color.b;
                    *pixel.add(1) = color.g;
                    *pixel.add(2) = color.r;
                    *pixel.add(3) = 0;
                }
            }
        }
    }
}

fn select_mode(gop: &GraphicsOutput) -> Option<uefi::proto::console::gop::Mode> {
    let mut best = None;
    let mut best_area = 0usize;
    for mode in gop.modes() {
        let info = mode.info();
        if info.pixel_format() == PixelFormat::BltOnly {
            continue;
        }
        let (width, height) = info.resolution();
        let area = width.saturating_mul(height);
        if area >= best_area {
            best_area = area;
            best = Some(mode);
        }
    }
    best
}

fn glyph(ch: u8) -> [u8; 7] {
    match ch {
        b'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        b'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        b'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        b'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        b'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        b'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        b'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        b'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        b'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        b'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        b'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        b'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        b'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        b'-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        _ => [0, 0, 0, 0, 0, 0, 0],
    }
}
