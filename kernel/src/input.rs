use crate::interrupts;

pub const MOUSE_BUTTON_LEFT: u8 = 0x01;
pub const MOUSE_BUTTON_RIGHT: u8 = 0x02;
pub const MOUSE_BUTTON_MIDDLE: u8 = 0x04;

#[derive(Clone, Copy)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub buttons: u8,
}

#[derive(Clone, Copy)]
pub enum InputEvent {
    MouseMove(MouseState),
    MouseDown(MouseState, u8),
    MouseUp(MouseState, u8),
}

pub struct InputManager {
    mouse: MouseState,
    last_mouse_seq: u64,
    pending_pressed: u8,
    pending_released: u8,
    pending_move: bool,
    max_x: i32,
    max_y: i32,
}

impl InputManager {
    pub const fn new(max_x: i32, max_y: i32) -> Self {
        Self {
            mouse: MouseState {
                x: max_x / 2,
                y: max_y / 2,
                buttons: 0,
            },
            last_mouse_seq: 0,
            pending_pressed: 0,
            pending_released: 0,
            pending_move: false,
            max_x,
            max_y,
        }
    }

    pub fn mouse_state(&self) -> MouseState {
        self.mouse
    }

    pub fn reset(&mut self, max_x: i32, max_y: i32) {
        self.mouse = MouseState {
            x: max_x / 2,
            y: max_y / 2,
            buttons: 0,
        };
        self.last_mouse_seq = 0;
        self.pending_pressed = 0;
        self.pending_released = 0;
        self.pending_move = false;
        self.max_x = max_x;
        self.max_y = max_y;
    }

    pub fn pump_hardware(&mut self) -> bool {
        let packet = match interrupts::consume_mouse_packet(self.last_mouse_seq) {
            Some(packet) => packet,
            None => return false,
        };

        self.last_mouse_seq = packet.seq;
        self.mouse.x = clamp(self.mouse.x + packet.dx, 0, self.max_x);
        self.mouse.y = clamp(self.mouse.y - packet.dy, 0, self.max_y);
        self.mouse.buttons = packet.buttons;
        self.pending_pressed |= packet.pressed;
        self.pending_released |= packet.released;
        self.pending_move |= packet.dx != 0 || packet.dy != 0;
        true
    }

    pub fn next_event(&mut self) -> Option<InputEvent> {
        let button = take_lowest_button(&mut self.pending_pressed);
        if button != 0 {
            return Some(InputEvent::MouseDown(self.mouse, button));
        }

        let button = take_lowest_button(&mut self.pending_released);
        if button != 0 {
            return Some(InputEvent::MouseUp(self.mouse, button));
        }

        if self.pending_move {
            self.pending_move = false;
            return Some(InputEvent::MouseMove(self.mouse));
        }

        None
    }
}

#[derive(Clone, Copy)]
pub struct MousePacket {
    pub seq: u64,
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
    pub pressed: u8,
    pub released: u8,
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

fn take_lowest_button(mask: &mut u8) -> u8 {
    if *mask == 0 {
        return 0;
    }

    let button = *mask & (!*mask).wrapping_add(1);
    *mask &= !button;
    button
}
