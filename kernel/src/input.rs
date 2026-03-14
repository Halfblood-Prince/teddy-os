use pc_keyboard::{
    DecodedKey, HandleControl, KeyState, Keyboard, ScancodeSet1, layouts,
};
use spin::Mutex;
use x86_64::instructions::interrupts;

const INPUT_QUEUE_CAPACITY: usize = 64;

#[derive(Clone, Copy)]
pub struct InputEvent {
    pub scancode: u8,
    pub pressed: bool,
    pub unicode: Option<char>,
    pub key_name: &'static str,
}

impl InputEvent {
    const EMPTY: Self = Self {
        scancode: 0,
        pressed: false,
        unicode: None,
        key_name: "None",
    };
}

#[derive(Clone, Copy)]
pub struct InputSnapshot {
    pub total_events: u64,
    pub pending_events: usize,
    pub last_event: Option<InputEvent>,
}

struct EventQueue {
    entries: [InputEvent; INPUT_QUEUE_CAPACITY],
    head: usize,
    tail: usize,
    len: usize,
}

impl EventQueue {
    const fn new() -> Self {
        Self {
            entries: [InputEvent::EMPTY; INPUT_QUEUE_CAPACITY],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    fn push(&mut self, event: InputEvent) {
        self.entries[self.tail] = event;
        self.tail = (self.tail + 1) % INPUT_QUEUE_CAPACITY;
        if self.len == INPUT_QUEUE_CAPACITY {
            self.head = (self.head + 1) % INPUT_QUEUE_CAPACITY;
        } else {
            self.len += 1;
        }
    }

    fn pop(&mut self) -> Option<InputEvent> {
        if self.len == 0 {
            return None;
        }

        let event = self.entries[self.head];
        self.head = (self.head + 1) % INPUT_QUEUE_CAPACITY;
        self.len -= 1;
        Some(event)
    }
}

struct InputState {
    keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
    queue: EventQueue,
    total_events: u64,
    last_event: Option<InputEvent>,
}

impl InputState {
    fn new() -> Self {
        Self {
            keyboard: Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore),
            queue: EventQueue::new(),
            total_events: 0,
            last_event: None,
        }
    }
}

static INPUT: Mutex<Option<InputState>> = Mutex::new(None);

pub fn init() {
    interrupts::without_interrupts(|| {
        *INPUT.lock() = Some(InputState::new());
    });
}

pub fn handle_scancode(scancode: u8) {
    let mut guard = INPUT.lock();
    let Some(state) = guard.as_mut() else {
        return;
    };

    if let Ok(Some(key_event)) = state.keyboard.add_byte(scancode) {
        if let Some(decoded) = state.keyboard.process_keyevent(key_event) {
            let unicode = decoded_key_to_char(decoded);
            let event = InputEvent {
                scancode,
                pressed: !matches!(key_event.state, KeyState::Up),
                unicode,
                key_name: if unicode.is_some() { "Char" } else { "Raw" },
            };
            state.queue.push(event);
            state.total_events += 1;
            state.last_event = Some(event);
        }
    }
}

pub fn pop_event() -> Option<InputEvent> {
    interrupts::without_interrupts(|| {
        let mut guard = INPUT.lock();
        guard.as_mut()?.queue.pop()
    })
}

pub fn snapshot() -> InputSnapshot {
    interrupts::without_interrupts(|| {
        let guard = INPUT.lock();
        let Some(state) = guard.as_ref() else {
            return InputSnapshot {
                total_events: 0,
                pending_events: 0,
                last_event: None,
            };
        };

        InputSnapshot {
            total_events: state.total_events,
            pending_events: state.queue.len,
            last_event: state.last_event,
        }
    })
}

fn decoded_key_to_char(key: DecodedKey) -> Option<char> {
    match key {
        DecodedKey::Unicode(character) => Some(character),
        DecodedKey::RawKey(_) => None,
    }
}
