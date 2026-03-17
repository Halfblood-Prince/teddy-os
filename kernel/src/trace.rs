use core::sync::atomic::{AtomicU8, Ordering};

static BOOT_STAGE: AtomicU8 = AtomicU8::new(0);

pub fn set_boot_stage(stage: u8) {
    BOOT_STAGE.store(stage, Ordering::Relaxed);
}

pub fn boot_stage() -> u8 {
    BOOT_STAGE.load(Ordering::Relaxed)
}
