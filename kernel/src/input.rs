use pc_keyboard::{
    DecodedKey, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1, layouts,
};
use spin::Mutex;
use x86_64::instructions::{interrupts, port::Port};

const INPUT_QUEUE_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    Character,
    Enter,
    Backspace,
    Tab,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Unknown,
}

#[derive(Clone, Copy)]
pub struct KeyboardEvent {
    pub scancode: u8,
    pub pressed: bool,
    pub unicode: Option<char>,
    pub key_name: &'static str,
    pub key_kind: KeyKind,
}

impl KeyboardEvent {
    const EMPTY: Self = Self {
        scancode: 0,
        pressed: false,
        unicode: None,
        key_name: "None",
        key_kind: KeyKind::Unknown,
    };
}

#[derive(Clone, Copy)]
pub struct MouseEvent {
    pub x: usize,
    pub y: usize,
    pub delta_x: isize,
    pub delta_y: isize,
    pub left_button: bool,
    pub right_button: bool,
}

impl MouseEvent {
    const EMPTY: Self = Self {
        x: 0,
        y: 0,
        delta_x: 0,
        delta_y: 0,
        left_button: false,
        right_button: false,
    };
}

#[derive(Clone, Copy)]
pub struct KeyboardSnapshot {
    pub total_events: u64,
    pub pending_events: usize,
    pub last_event: Option<KeyboardEvent>,
}

#[derive(Clone, Copy)]
pub struct MouseSnapshot {
    pub total_events: u64,
    pub pending_events: usize,
    pub last_event: Option<MouseEvent>,
    pub x: usize,
    pub y: usize,
    pub left_button: bool,
    pub right_button: bool,
}

struct EventQueue<T: Copy, const N: usize> {
    entries: [T; N],
    head: usize,
    tail: usize,
    len: usize,
}

impl<T: Copy, const N: usize> EventQueue<T, N> {
    const fn new(empty: T) -> Self {
        Self {
            entries: [empty; N],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    fn push(&mut self, event: T) {
        self.entries[self.tail] = event;
        self.tail = (self.tail + 1) % N;
        if self.len == N {
            self.head = (self.head + 1) % N;
        } else {
            self.len += 1;
        }
    }

    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        let event = self.entries[self.head];
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(event)
    }
}

struct MouseDecoder {
    packet: [u8; 3],
    index: usize,
    x: usize,
    y: usize,
    left_button: bool,
    right_button: bool,
}

impl MouseDecoder {
    const fn new() -> Self {
        Self {
            packet: [0; 3],
            index: 0,
            x: 320,
            y: 200,
            left_button: false,
            right_button: false,
        }
    }

    fn feed(&mut self, byte: u8, screen_width: usize, screen_height: usize) -> Option<MouseEvent> {
        if self.index == 0 && (byte & 0x08) == 0 {
            return None;
        }

        self.packet[self.index] = byte;
        self.index += 1;
        if self.index < 3 {
            return None;
        }
        self.index = 0;

        let flags = self.packet[0];
        let delta_x = signed_delta(self.packet[1], flags & 0x10 != 0);
        let delta_y = signed_delta(self.packet[2], flags & 0x20 != 0);

        self.x = clamp_position(self.x as isize + delta_x, screen_width);
        self.y = clamp_position(self.y as isize - delta_y, screen_height);
        self.left_button = flags & 0x01 != 0;
        self.right_button = flags & 0x02 != 0;

        Some(MouseEvent {
            x: self.x,
            y: self.y,
            delta_x,
            delta_y,
            left_button: self.left_button,
            right_button: self.right_button,
        })
    }
}

struct InputState {
    keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
    key_queue: EventQueue<KeyboardEvent, INPUT_QUEUE_CAPACITY>,
    mouse_queue: EventQueue<MouseEvent, INPUT_QUEUE_CAPACITY>,
    mouse: MouseDecoder,
    key_total_events: u64,
    mouse_total_events: u64,
    last_key_event: Option<KeyboardEvent>,
    last_mouse_event: Option<MouseEvent>,
    screen_width: usize,
    screen_height: usize,
}

impl InputState {
    fn new(screen_width: usize, screen_height: usize) -> Self {
        Self {
            keyboard: Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore),
            key_queue: EventQueue::new(KeyboardEvent::EMPTY),
            mouse_queue: EventQueue::new(MouseEvent::EMPTY),
            mouse: MouseDecoder {
                x: screen_width / 2,
                y: screen_height / 2,
                ..MouseDecoder::new()
            },
            key_total_events: 0,
            mouse_total_events: 0,
            last_key_event: None,
            last_mouse_event: None,
            screen_width,
            screen_height,
        }
    }
}

static INPUT: Mutex<Option<InputState>> = Mutex::new(None);

pub fn init(screen_width: usize, screen_height: usize) {
    interrupts::without_interrupts(|| {
        *INPUT.lock() = Some(InputState::new(screen_width, screen_height));
    });

    initialize_ps2_mouse();
}

pub fn handle_scancode(scancode: u8) {
    let mut guard = INPUT.lock();
    let Some(state) = guard.as_mut() else {
        return;
    };

    if let Ok(Some(key_event)) = state.keyboard.add_byte(scancode) {
        if let Some(decoded) = state.keyboard.process_keyevent(key_event) {
            let unicode = decoded_key_to_char(decoded);
            let (key_kind, key_name) = decoded_key_meta(decoded, unicode);
            let event = KeyboardEvent {
                scancode,
                pressed: !matches!(key_event.state, KeyState::Up),
                unicode,
                key_name,
                key_kind,
            };
            state.key_queue.push(event);
            state.key_total_events += 1;
            state.last_key_event = Some(event);
        }
    }
}

pub fn handle_mouse_byte(byte: u8) {
    let mut guard = INPUT.lock();
    let Some(state) = guard.as_mut() else {
        return;
    };

    if let Some(event) = state.mouse.feed(byte, state.screen_width, state.screen_height) {
        state.mouse_queue.push(event);
        state.mouse_total_events += 1;
        state.last_mouse_event = Some(event);
    }
}

pub fn pop_keyboard_event() -> Option<KeyboardEvent> {
    interrupts::without_interrupts(|| {
        let mut guard = INPUT.lock();
        guard.as_mut()?.key_queue.pop()
    })
}

pub fn pop_mouse_event() -> Option<MouseEvent> {
    interrupts::without_interrupts(|| {
        let mut guard = INPUT.lock();
        guard.as_mut()?.mouse_queue.pop()
    })
}

pub fn keyboard_snapshot() -> KeyboardSnapshot {
    interrupts::without_interrupts(|| {
        let guard = INPUT.lock();
        let Some(state) = guard.as_ref() else {
            return KeyboardSnapshot {
                total_events: 0,
                pending_events: 0,
                last_event: None,
            };
        };

        KeyboardSnapshot {
            total_events: state.key_total_events,
            pending_events: state.key_queue.len,
            last_event: state.last_key_event,
        }
    })
}

pub fn mouse_snapshot() -> MouseSnapshot {
    interrupts::without_interrupts(|| {
        let guard = INPUT.lock();
        let Some(state) = guard.as_ref() else {
            return MouseSnapshot {
                total_events: 0,
                pending_events: 0,
                last_event: None,
                x: 0,
                y: 0,
                left_button: false,
                right_button: false,
            };
        };

        MouseSnapshot {
            total_events: state.mouse_total_events,
            pending_events: state.mouse_queue.len,
            last_event: state.last_mouse_event,
            x: state.mouse.x,
            y: state.mouse.y,
            left_button: state.mouse.left_button,
            right_button: state.mouse.right_button,
        }
    })
}

fn decoded_key_to_char(key: DecodedKey) -> Option<char> {
    match key {
        DecodedKey::Unicode(character) => Some(character),
        DecodedKey::RawKey(_) => None,
    }
}

fn decoded_key_meta(key: DecodedKey, unicode: Option<char>) -> (KeyKind, &'static str) {
    match key {
        DecodedKey::Unicode('\n') => (KeyKind::Enter, "Enter"),
        DecodedKey::Unicode('\t') => (KeyKind::Tab, "Tab"),
        DecodedKey::Unicode(character) => {
            let _ = unicode;
            if character == '\u{8}' {
                (KeyKind::Backspace, "Backspace")
            } else {
                (KeyKind::Character, "Char")
            }
        }
        DecodedKey::RawKey(code) => match code {
            KeyCode::Enter => (KeyKind::Enter, "Enter"),
            KeyCode::Backspace => (KeyKind::Backspace, "Backspace"),
            KeyCode::Tab => (KeyKind::Tab, "Tab"),
            KeyCode::Escape => (KeyKind::Escape, "Escape"),
            KeyCode::ArrowUp => (KeyKind::ArrowUp, "Up"),
            KeyCode::ArrowDown => (KeyKind::ArrowDown, "Down"),
            KeyCode::ArrowLeft => (KeyKind::ArrowLeft, "Left"),
            KeyCode::ArrowRight => (KeyKind::ArrowRight, "Right"),
            _ => (KeyKind::Unknown, "Raw"),
        },
    }
}

fn initialize_ps2_mouse() {
    unsafe {
        wait_for_write();
        Port::<u8>::new(0x64).write(0xA8);

        wait_for_write();
        Port::<u8>::new(0x64).write(0x20);
        wait_for_read();
        let mut status: u8 = Port::<u8>::new(0x60).read();
        status |= 0x02;

        wait_for_write();
        Port::<u8>::new(0x64).write(0x60);
        wait_for_write();
        Port::<u8>::new(0x60).write(status);

        write_mouse_command(0xF6);
        let _ = read_mouse_ack();
        write_mouse_command(0xF4);
        let _ = read_mouse_ack();
    }
}

unsafe fn write_mouse_command(command: u8) {
    wait_for_write();
    Port::<u8>::new(0x64).write(0xD4);
    wait_for_write();
    Port::<u8>::new(0x60).write(command);
}

unsafe fn read_mouse_ack() -> u8 {
    wait_for_read();
    Port::<u8>::new(0x60).read()
}

unsafe fn wait_for_write() {
    let mut status = Port::<u8>::new(0x64);
    while status.read() & 0x02 != 0 {}
}

unsafe fn wait_for_read() {
    let mut status = Port::<u8>::new(0x64);
    while status.read() & 0x01 == 0 {}
}

fn signed_delta(value: u8, negative: bool) -> isize {
    if negative {
        (value as isize) - 256
    } else {
        value as isize
    }
}

fn clamp_position(value: isize, max: usize) -> usize {
    if max == 0 {
        return 0;
    }
    value.clamp(0, max.saturating_sub(1) as isize) as usize
}
