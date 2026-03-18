use crate::{
    boot_info::{BootInfo, FramebufferInfo},
    input::{self, InputEvent, InputManager, MouseState},
    interrupts,
};

const TITLE_BAR_HEIGHT: i32 = 14;
const CURSOR_SIZE: usize = 11;
const STATUS_X: i32 = 18;
const STATUS_Y: i32 = 140;
const STATUS_WIDTH: i32 = 284;
const STATUS_HEIGHT: i32 = 30;

pub struct GraphicsShell {
    fb: FramebufferInfo,
    input: InputManager,
    uptime_seconds: u64,
    accent_phase: u8,
    demo_window: WindowRect,
    notes_window: WindowRect,
    drag_state: DragState,
    cursor_backing: [u8; CURSOR_SIZE * CURSOR_SIZE],
    cursor_saved_x: i32,
    cursor_saved_y: i32,
}

#[derive(Clone, Copy)]
struct WindowRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy)]
struct DragState {
    active: bool,
    offset_x: i32,
    offset_y: i32,
}

enum MouseRedraw {
    None,
    Overlay,
    Full,
}

impl GraphicsShell {
    pub fn new(boot_info: BootInfo) -> Option<Self> {
        let fb = boot_info.framebuffer()?;
        if fb.bpp() != 8 {
            return None;
        }

        let max_x = fb.width() as i32 - 1;
        let max_y = fb.height() as i32 - 1;
        Some(Self {
            fb,
            input: InputManager::new(max_x, max_y),
            uptime_seconds: 0,
            accent_phase: 0,
            demo_window: WindowRect {
                x: 18,
                y: 24,
                width: 172,
                height: 104,
            },
            notes_window: WindowRect {
                x: 202,
                y: 36,
                width: 102,
                height: 98,
            },
            drag_state: DragState {
                active: false,
                offset_x: 0,
                offset_y: 0,
            },
            cursor_backing: [0; CURSOR_SIZE * CURSOR_SIZE],
            cursor_saved_x: 0,
            cursor_saved_y: 0,
        })
    }

    pub fn render(&mut self) {
        self.draw_background();
        self.draw_top_bar();
        self.draw_taskbar();
        self.draw_demo_window();
        self.draw_notes_window();
        self.draw_status();
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    pub fn tick(&mut self, uptime_seconds: u64) {
        if self.uptime_seconds != uptime_seconds {
            self.restore_cursor_backing();
            self.uptime_seconds = uptime_seconds;
            self.accent_phase = self.accent_phase.wrapping_add(1) % 3;
            self.render();
        }
    }

    pub fn handle_key(&mut self, ascii: u8) {
        if ascii != b'?' {
            self.restore_cursor_backing();
            self.accent_phase = self.accent_phase.wrapping_add(1) % 3;
            self.render();
        }
    }

    pub fn poll_input(&mut self) {
        let previous_mouse = self.input.mouse_state();
        if !self.input.pump_hardware() {
            return;
        }

        let current_mouse = self.input.mouse_state();
        let mut redraw = if mouse_changed(previous_mouse, current_mouse) {
            MouseRedraw::Overlay
        } else {
            MouseRedraw::None
        };

        while let Some(event) = self.input.next_event() {
            let event_redraw = match event {
                InputEvent::MouseMove(state) => self.handle_mouse_move(state),
                InputEvent::MouseDown(state, button) => self.handle_mouse_down(state, button),
                InputEvent::MouseUp(state, button) => self.handle_mouse_up(state, button),
            };
            redraw = combine_redraw(redraw, event_redraw);
        }

        match redraw {
            MouseRedraw::None => {}
            MouseRedraw::Overlay => self.refresh_cursor_overlay(previous_mouse),
            MouseRedraw::Full => {
                self.restore_cursor_backing();
                self.render();
            }
        }
    }

    fn handle_mouse_move(&mut self, state: MouseState) -> MouseRedraw {
        if self.drag_state.active && state.buttons & input::MOUSE_BUTTON_LEFT != 0 {
            let max_x = self.fb.width() as i32 - self.demo_window.width - 1;
            let max_y = self.fb.height() as i32 - 20 - self.demo_window.height;
            self.demo_window.x = clamp(state.x - self.drag_state.offset_x, 0, max_x);
            self.demo_window.y = clamp(state.y - self.drag_state.offset_y, 20, max_y);
            MouseRedraw::Full
        } else {
            MouseRedraw::Overlay
        }
    }

    fn handle_mouse_down(&mut self, state: MouseState, button: u8) -> MouseRedraw {
        if button != input::MOUSE_BUTTON_LEFT {
            return MouseRedraw::Overlay;
        }

        if self.point_in_title_bar(&self.demo_window, state.x, state.y) {
            self.drag_state.active = true;
            self.drag_state.offset_x = state.x - self.demo_window.x;
            self.drag_state.offset_y = state.y - self.demo_window.y;
            return MouseRedraw::Full;
        }

        MouseRedraw::Overlay
    }

    fn handle_mouse_up(&mut self, _state: MouseState, button: u8) -> MouseRedraw {
        if button == input::MOUSE_BUTTON_LEFT && self.drag_state.active {
            self.drag_state.active = false;
            return MouseRedraw::Full;
        }

        MouseRedraw::Overlay
    }

    fn refresh_cursor_overlay(&mut self, previous_mouse: MouseState) {
        self.restore_cursor_backing();
        if mouse_changed(previous_mouse, self.input.mouse_state()) {
            self.draw_status();
        }
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    fn draw_background(&self) {
        let height = self.fb.height() as usize;
        let width = self.fb.width() as usize;
        let mut y = 0usize;
        while y < height {
            let color = if y < 64 {
                1
            } else if y < 132 {
                9
            } else {
                3
            };
            self.fill_rect(0, y as i32, width as i32, 1, color);
            y += 1;
        }

        self.fill_rect(0, 0, width as i32, 20, 8);
        self.fill_rect(0, height as i32 - 18, width as i32, 18, 8);
    }

    fn draw_top_bar(&self) {
        self.draw_text(10, 6, 15, "TEDDY-OS GRAPHICS DESKTOP");
        self.draw_text(208, 6, 14, "GUI PHASE 2");
    }

    fn draw_taskbar(&self) {
        let accent = self.accent_color();
        self.fill_rect(0, 182, self.fb.width() as i32, 18, 8);
        self.fill_rect(6, 185, 58, 10, accent);
        self.draw_text(14, 187, 15, "TEDDY");
        self.draw_text(78, 187, 15, "MOUSE");
        self.draw_text(126, 187, 15, "DRAG TITLE BAR");
    }

    fn draw_demo_window(&self) {
        let title_color = if self.drag_state.active { 12 } else { 8 };
        self.draw_window_frame(self.demo_window, 1, title_color, "DESKTOP DEMO");

        self.draw_text(self.demo_window.x + 12, self.demo_window.y + 24, 15, "PS/2 MOUSE ONLINE");
        self.draw_text(self.demo_window.x + 12, self.demo_window.y + 36, 15, "MOVE THE CURSOR");
        self.draw_text(self.demo_window.x + 12, self.demo_window.y + 48, 15, "CLICK THIS TITLE BAR");
        self.draw_text(self.demo_window.x + 12, self.demo_window.y + 60, 15, "DRAG THE WINDOW");
        self.draw_text(self.demo_window.x + 12, self.demo_window.y + 78, 14, "LEFT");
        self.draw_text(self.demo_window.x + 50, self.demo_window.y + 78, 15, "DRAG");
        self.draw_text(self.demo_window.x + 92, self.demo_window.y + 78, 14, "RIGHT");
        self.draw_text(self.demo_window.x + 136, self.demo_window.y + 78, 15, "TRACK");

        self.fill_rect(self.demo_window.x + 12, self.demo_window.y + 90, 144, 4, 0);
        self.fill_rect(
            self.demo_window.x + 12,
            self.demo_window.y + 90,
            36 + (self.accent_phase as i32 * 26),
            4,
            self.accent_color(),
        );
    }

    fn draw_notes_window(&self) {
        self.draw_window_frame(self.notes_window, 3, 8, "EVENTS");
        self.draw_text(self.notes_window.x + 10, self.notes_window.y + 24, 15, "IRQ12");
        self.draw_text(self.notes_window.x + 10, self.notes_window.y + 36, 15, "PACKETS");
        self.draw_text(self.notes_window.x + 10, self.notes_window.y + 52, 15, "MOVE");
        self.draw_text(self.notes_window.x + 10, self.notes_window.y + 64, 15, "DOWN");
        self.draw_text(self.notes_window.x + 10, self.notes_window.y + 76, 15, "UP");
        self.draw_text(self.notes_window.x + 10, self.notes_window.y + 92, 15, "NEXT");
        self.draw_text(self.notes_window.x + 10, self.notes_window.y + 104, 15, "CLICKABLE APPS");
    }

    fn draw_status(&self) {
        let mouse = self.input.mouse_state();
        self.fill_rect(STATUS_X, STATUS_Y, STATUS_WIDTH, STATUS_HEIGHT, 1);
        self.draw_rect(STATUS_X, STATUS_Y, STATUS_WIDTH, STATUS_HEIGHT, 15);
        self.draw_text(28, 148, 15, "UP");
        self.draw_number(48, 148, self.uptime_seconds as u32, 14);
        self.draw_text(80, 148, 15, "KEY");
        self.draw_ascii(106, 148, interrupts::last_ascii(), 14);
        self.draw_text(122, 148, 15, "SC");
        self.draw_hex_byte(140, 148, interrupts::last_scancode(), 14);
        self.draw_text(164, 148, 15, "X");
        self.draw_number(176, 148, mouse.x as u32, 14);
        self.draw_text(210, 148, 15, "Y");
        self.draw_number(222, 148, mouse.y as u32, 14);
        self.draw_text(256, 148, 15, "B");
        self.draw_hex_byte(268, 148, mouse.buttons, 14);
    }

    fn draw_cursor(&self) {
        let mouse = self.input.mouse_state();
        let color = if mouse.buttons & input::MOUSE_BUTTON_LEFT != 0 {
            12
        } else if mouse.buttons & input::MOUSE_BUTTON_RIGHT != 0 {
            14
        } else if mouse.buttons & input::MOUSE_BUTTON_MIDDLE != 0 {
            10
        } else {
            15
        };

        let mut step = 0;
        while step < CURSOR_SIZE as i32 {
            self.put_pixel(mouse.x, mouse.y + step, color);
            if step < 6 {
                self.put_pixel(mouse.x + step, mouse.y + step, color);
            }
            if step < 4 {
                self.put_pixel(mouse.x + 1, mouse.y + step, color);
            }
            step += 1;
        }
    }

    fn save_cursor_backing(&mut self, mouse: MouseState) {
        self.cursor_saved_x = mouse.x;
        self.cursor_saved_y = mouse.y;

        let mut row = 0usize;
        while row < CURSOR_SIZE {
            let mut col = 0usize;
            while col < CURSOR_SIZE {
                self.cursor_backing[row * CURSOR_SIZE + col] =
                    self.read_pixel(mouse.x + col as i32, mouse.y + row as i32);
                col += 1;
            }
            row += 1;
        }
    }

    fn restore_cursor_backing(&self) {
        let mut row = 0usize;
        while row < CURSOR_SIZE {
            let mut col = 0usize;
            while col < CURSOR_SIZE {
                self.put_pixel(
                    self.cursor_saved_x + col as i32,
                    self.cursor_saved_y + row as i32,
                    self.cursor_backing[row * CURSOR_SIZE + col],
                );
                col += 1;
            }
            row += 1;
        }
    }

    fn draw_window_frame(&self, rect: WindowRect, body: u8, title: u8, label: &str) {
        self.fill_rect(rect.x, rect.y, rect.width, rect.height, body);
        self.draw_rect(rect.x, rect.y, rect.width, rect.height, 15);
        self.fill_rect(rect.x + 1, rect.y + 1, rect.width - 2, TITLE_BAR_HEIGHT, title);
        self.draw_text(rect.x + 6, rect.y + 4, 15, label);
        self.fill_rect(rect.x + rect.width - 18, rect.y + 4, 5, 5, 4);
        self.fill_rect(rect.x + rect.width - 10, rect.y + 4, 5, 5, 14);
    }

    fn point_in_title_bar(&self, rect: &WindowRect, x: i32, y: i32) -> bool {
        x >= rect.x
            && x < rect.x + rect.width
            && y >= rect.y
            && y < rect.y + TITLE_BAR_HEIGHT + 1
    }

    fn accent_color(&self) -> u8 {
        match self.accent_phase {
            0 => 12,
            1 => 13,
            _ => 10,
        }
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

    fn read_pixel(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 {
            return 0;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.fb.width() as usize || y >= self.fb.height() as usize {
            return 0;
        }

        let offset = y * self.fb.pitch() as usize + x;
        let ptr = self.fb.addr() as usize as *const u8;
        unsafe { ptr.add(offset).read_volatile() }
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

fn mouse_changed(previous: MouseState, current: MouseState) -> bool {
    previous.x != current.x || previous.y != current.y || previous.buttons != current.buttons
}

fn combine_redraw(current: MouseRedraw, next: MouseRedraw) -> MouseRedraw {
    match (current, next) {
        (MouseRedraw::Full, _) | (_, MouseRedraw::Full) => MouseRedraw::Full,
        (MouseRedraw::Overlay, _) | (_, MouseRedraw::Overlay) => MouseRedraw::Overlay,
        _ => MouseRedraw::None,
    }
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
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
