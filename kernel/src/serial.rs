const COM1: u16 = 0x3F8;
const SERIAL_READY_MASK: u8 = 0x20;
const SERIAL_WAIT_SPINS: usize = 100_000;

pub fn init() {
    unsafe {
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x80);
        outb(COM1 + 0, 0x03);
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x03);
        outb(COM1 + 2, 0xC7);
        outb(COM1 + 4, 0x0B);
    }
}

pub fn write_str(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            write_byte(b'\r');
        }
        write_byte(byte);
    }
}

fn write_byte(byte: u8) {
    unsafe {
        if !wait_for_transmit_ready() {
            return;
        }
        outb(COM1, byte);
    }
}

unsafe fn wait_for_transmit_ready() -> bool {
    for _ in 0..SERIAL_WAIT_SPINS {
        if (inb(COM1 + 5) & SERIAL_READY_MASK) != 0 {
            return true;
        }
    }
    false
}

unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nomem, nostack, preserves_flags)
    );
    value
}

