use core::sync::atomic::{AtomicBool, Ordering};

use x86_64::instructions::interrupts as cpu_interrupts;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
const PIT_FREQUENCY_HZ: u32 = 100;

static EXCEPTIONS_READY: AtomicBool = AtomicBool::new(false);
static HARDWARE_READY: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    Mouse = PIC_2_OFFSET + 4,
}

pub fn init_exceptions() {
    EXCEPTIONS_READY.store(true, Ordering::SeqCst);
}

pub fn init_hardware() {
    HARDWARE_READY.store(true, Ordering::SeqCst);
}

pub fn enable() {
    cpu_interrupts::enable();
}

pub fn disable() {
    cpu_interrupts::disable();
}

pub fn timer_frequency_hz() -> u32 {
    PIT_FREQUENCY_HZ
}

pub fn exceptions_ready() -> bool {
    EXCEPTIONS_READY.load(Ordering::SeqCst)
}

pub fn is_initialized() -> bool {
    HARDWARE_READY.load(Ordering::SeqCst)
}
