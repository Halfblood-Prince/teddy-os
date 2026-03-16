use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

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
