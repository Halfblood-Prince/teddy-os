pub fn init() {
    // Keep early kernel boot independent from legacy COM port state in VMware.
}

pub fn write_str(_text: &str) {
    // Framebuffer logging is the primary early-boot diagnostics path for now.
}

