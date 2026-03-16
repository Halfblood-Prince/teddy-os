use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use crate::port;

#[derive(Clone, Copy)]
pub struct KeyEvent {
    pub scancode: u8,
    pub ascii: Option<u8>,
}

const QUEUE_CAPACITY: usize = 64;

static WRITE_INDEX: AtomicUsize = AtomicUsize::new(0);
static READ_INDEX: AtomicUsize = AtomicUsize::new(0);
static DROPPED_EVENTS: AtomicUsize = AtomicUsize::new(0);
static LAST_SCANCODE: AtomicU8 = AtomicU8::new(0);
static LAST_ASCII: AtomicU8 = AtomicU8::new(0);
static mut QUEUE: [MaybeUninit<KeyEvent>; QUEUE_CAPACITY] = [MaybeUninit::uninit(); QUEUE_CAPACITY];

pub fn push_key(scancode: u8, ascii: Option<u8>) {
    LAST_SCANCODE.store(scancode, Ordering::Relaxed);
    LAST_ASCII.store(ascii.unwrap_or(0), Ordering::Relaxed);

    let write = WRITE_INDEX.load(Ordering::Relaxed);
    let next = (write + 1) % QUEUE_CAPACITY;
    let read = READ_INDEX.load(Ordering::Acquire);
    if next == read {
        DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
        return;
    }

    unsafe {
        QUEUE[write].write(KeyEvent { scancode, ascii });
    }
    WRITE_INDEX.store(next, Ordering::Release);
}

pub fn pop_key() -> Option<KeyEvent> {
    let read = READ_INDEX.load(Ordering::Relaxed);
    let write = WRITE_INDEX.load(Ordering::Acquire);
    if read == write {
        return None;
    }

    let event = unsafe { QUEUE[read].assume_init_read() };
    READ_INDEX.store((read + 1) % QUEUE_CAPACITY, Ordering::Release);
    Some(event)
}

pub fn last_scancode() -> u8 {
    LAST_SCANCODE.load(Ordering::Relaxed)
}

pub fn last_ascii() -> u8 {
    match LAST_ASCII.load(Ordering::Relaxed) {
        0 => b'-',
        value => value,
    }
}

pub fn dropped_events() -> usize {
    DROPPED_EVENTS.load(Ordering::Relaxed)
}

pub fn pending_events() -> usize {
    let write = WRITE_INDEX.load(Ordering::Acquire);
    let read = READ_INDEX.load(Ordering::Acquire);
    if write >= read {
        write - read
    } else {
        QUEUE_CAPACITY - (read - write)
    }
}

pub fn poll_key() -> Option<KeyEvent> {
    if port::inb(0x64) & 0x01 == 0 {
        return None;
    }

    let scancode = port::inb(0x60);
    let ascii = if scancode & 0x80 == 0 {
        Some(decode_scancode(scancode))
    } else {
        None
    };

    LAST_SCANCODE.store(scancode, Ordering::Relaxed);
    LAST_ASCII.store(ascii.unwrap_or(0), Ordering::Relaxed);

    Some(KeyEvent { scancode, ascii })
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
