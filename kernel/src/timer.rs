use core::sync::atomic::{AtomicU64, Ordering};

use crate::interrupts;

#[derive(Clone, Copy)]
pub struct TimerSnapshot {
    pub ticks: u64,
    pub frequency_hz: u32,
}

static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    TICKS.store(0, Ordering::SeqCst);
}

pub fn on_tick() {
    TICKS.fetch_add(1, Ordering::SeqCst);
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

pub fn snapshot() -> TimerSnapshot {
    TimerSnapshot {
        ticks: ticks(),
        frequency_hz: interrupts::timer_frequency_hz(),
    }
}
