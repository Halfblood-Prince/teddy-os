use core::arch::global_asm;
use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use crate::{cpu, port, vga};

const IDT_ENTRIES: usize = 256;
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;
const PIC_EOI: u8 = 0x20;
const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL0: u16 = 0x40;
const PIT_BASE_FREQUENCY: u32 = 1_193_182;
const PIT_TICKS_PER_SECOND: u32 = 100;

const TIMER_VECTOR: u8 = 32;
const KEYBOARD_VECTOR: u8 = 33;

static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);
static LAST_SCANCODE: AtomicU8 = AtomicU8::new(0);
static LAST_ASCII: AtomicU8 = AtomicU8::new(b'-');

#[repr(C)]
struct InterruptStackFrame {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    options: u16,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            options: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    fn set_handler(&mut self, handler: usize) {
        self.offset_low = handler as u16;
        self.selector = 0x08;
        self.options = 0x8E00;
        self.offset_mid = (handler >> 16) as u16;
        self.offset_high = (handler >> 32) as u32;
        self.reserved = 0;
    }
}

static mut IDT: [IdtEntry; IDT_ENTRIES] = [IdtEntry::missing(); IDT_ENTRIES];

unsafe extern "C" {
    static isr_stub_table: [usize; 48];
}

global_asm!(
    r#"
    .intel_syntax noprefix

    .macro PUSH_REGS
        push r15
        push r14
        push r13
        push r12
        push r11
        push r10
        push r9
        push r8
        push rsi
        push rdi
        push rbp
        push rdx
        push rcx
        push rbx
        push rax
    .endm

    .macro POP_REGS
        pop rax
        pop rbx
        pop rcx
        pop rdx
        pop rbp
        pop rdi
        pop rsi
        pop r8
        pop r9
        pop r10
        pop r11
        pop r12
        pop r13
        pop r14
        pop r15
    .endm

    .macro ISR_NOERR vector
        .global isr_stub_\vector
        isr_stub_\vector:
            push 0
            push \vector
            jmp interrupt_common_stub
    .endm

    .macro ISR_ERR vector
        .global isr_stub_\vector
        isr_stub_\vector:
            push \vector
            jmp interrupt_common_stub
    .endm

    .global interrupt_common_stub
    interrupt_common_stub:
        cld
        PUSH_REGS
        mov rdi, [rsp + 120]
        mov rsi, [rsp + 128]
        lea rdx, [rsp + 136]
        sub rsp, 8
        call interrupt_dispatch
        add rsp, 8
        POP_REGS
        add rsp, 16
        iretq

    ISR_NOERR 0
    ISR_NOERR 1
    ISR_NOERR 2
    ISR_NOERR 3
    ISR_NOERR 4
    ISR_NOERR 5
    ISR_NOERR 6
    ISR_NOERR 7
    ISR_ERR 8
    ISR_NOERR 9
    ISR_ERR 10
    ISR_ERR 11
    ISR_ERR 12
    ISR_ERR 13
    ISR_ERR 14
    ISR_NOERR 15
    ISR_NOERR 16
    ISR_ERR 17
    ISR_NOERR 18
    ISR_NOERR 19
    ISR_NOERR 20
    ISR_ERR 21
    ISR_NOERR 22
    ISR_NOERR 23
    ISR_NOERR 24
    ISR_NOERR 25
    ISR_NOERR 26
    ISR_NOERR 27
    ISR_NOERR 28
    ISR_NOERR 29
    ISR_ERR 30
    ISR_NOERR 31
    ISR_NOERR 32
    ISR_NOERR 33
    ISR_NOERR 34
    ISR_NOERR 35
    ISR_NOERR 36
    ISR_NOERR 37
    ISR_NOERR 38
    ISR_NOERR 39
    ISR_NOERR 40
    ISR_NOERR 41
    ISR_NOERR 42
    ISR_NOERR 43
    ISR_NOERR 44
    ISR_NOERR 45
    ISR_NOERR 46
    ISR_NOERR 47

    .global isr_stub_table
    isr_stub_table:
        .quad isr_stub_0
        .quad isr_stub_1
        .quad isr_stub_2
        .quad isr_stub_3
        .quad isr_stub_4
        .quad isr_stub_5
        .quad isr_stub_6
        .quad isr_stub_7
        .quad isr_stub_8
        .quad isr_stub_9
        .quad isr_stub_10
        .quad isr_stub_11
        .quad isr_stub_12
        .quad isr_stub_13
        .quad isr_stub_14
        .quad isr_stub_15
        .quad isr_stub_16
        .quad isr_stub_17
        .quad isr_stub_18
        .quad isr_stub_19
        .quad isr_stub_20
        .quad isr_stub_21
        .quad isr_stub_22
        .quad isr_stub_23
        .quad isr_stub_24
        .quad isr_stub_25
        .quad isr_stub_26
        .quad isr_stub_27
        .quad isr_stub_28
        .quad isr_stub_29
        .quad isr_stub_30
        .quad isr_stub_31
        .quad isr_stub_32
        .quad isr_stub_33
        .quad isr_stub_34
        .quad isr_stub_35
        .quad isr_stub_36
        .quad isr_stub_37
        .quad isr_stub_38
        .quad isr_stub_39
        .quad isr_stub_40
        .quad isr_stub_41
        .quad isr_stub_42
        .quad isr_stub_43
        .quad isr_stub_44
        .quad isr_stub_45
        .quad isr_stub_46
        .quad isr_stub_47
    "#
);

pub fn init() {
    unsafe {
        for (index, handler) in isr_stub_table.iter().copied().enumerate() {
            IDT[index].set_handler(handler);
        }
        cpu::load_idt(
            core::ptr::addr_of!(IDT) as *const _ as u64,
            (core::mem::size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
        );
    }

    remap_pic();
    set_irq_masks(0b1111_1100, 0b1111_1111);
    init_pit(PIT_TICKS_PER_SECOND);
}

pub fn render_status() {
    let ticks = TIMER_TICKS.load(Ordering::Relaxed);
    let seconds = ticks / PIT_TICKS_PER_SECOND as u64;
    vga::write_line(14, 8, "Interrupts: IDT+PIC+PIT online", 0x1E);
    vga::write_line(16, 8, "Timer ticks:", 0x1F);
    vga::write_hex_dword(16, 21, ticks as u32, 0x1F);
    vga::write_line(17, 8, "Uptime seconds:", 0x17);
    vga::write_hex_dword(17, 24, seconds as u32, 0x17);
    vga::write_line(19, 8, "Last keyboard scancode:", 0x1F);
    vga::write_hex_byte(19, 31, "", LAST_SCANCODE.load(Ordering::Relaxed), 0x1F);
    vga::write_line(20, 8, "Last keyboard ascii:", 0x1A);
    vga::write_ascii(20, 28, LAST_ASCII.load(Ordering::Relaxed), 0x1A);
}

#[no_mangle]
extern "C" fn interrupt_dispatch(vector: u64, error_code: u64, stack_frame: *const InterruptStackFrame) {
    match vector as u8 {
        TIMER_VECTOR => handle_timer_irq(),
        KEYBOARD_VECTOR => handle_keyboard_irq(),
        _ if vector < 32 => handle_exception(vector as u8, error_code, stack_frame),
        _ => end_of_interrupt(vector as u8),
    }
}

fn handle_timer_irq() {
    let ticks = TIMER_TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    if ticks % 10 == 0 {
        render_status();
    }
    end_of_interrupt(TIMER_VECTOR);
}

fn handle_keyboard_irq() {
    let scancode = port::inb(0x60);
    LAST_SCANCODE.store(scancode, Ordering::Relaxed);

    if scancode & 0x80 == 0 {
        LAST_ASCII.store(decode_scancode(scancode).unwrap_or(b'?'), Ordering::Relaxed);
        render_status();
    }

    end_of_interrupt(KEYBOARD_VECTOR);
}

fn handle_exception(vector: u8, error_code: u64, stack_frame: *const InterruptStackFrame) {
    vga::clear_screen(0x4F);
    vga::write_line(4, 8, "TEDDY-OS EXCEPTION", 0x4F);
    vga::write_line(6, 8, "Vector:", 0x4F);
    vga::write_hex_byte(6, 16, "", vector, 0x4F);
    vga::write_line(7, 8, "Error code:", 0x4F);
    vga::write_hex_dword(7, 20, error_code as u32, 0x4F);
    render_exception_frame(stack_frame, 9, 0x4F);
    loop {
        cpu::halt();
    }
}

fn render_exception_frame(stack_frame: *const InterruptStackFrame, start_row: usize, attribute: u8) {
    let frame = unsafe { &*stack_frame };
    vga::write_line(start_row, 8, "RIP:", attribute);
    vga::write_hex_qword(start_row, 13, frame.instruction_pointer, attribute);
    vga::write_line(start_row + 1, 8, "CS:", attribute);
    vga::write_hex_qword(start_row + 1, 12, frame.code_segment, attribute);
    vga::write_line(start_row + 2, 8, "RFLAGS:", attribute);
    vga::write_hex_qword(start_row + 2, 16, frame.cpu_flags, attribute);
}

fn decode_scancode(scancode: u8) -> Option<u8> {
    match scancode {
        0x02 => Some(b'1'),
        0x03 => Some(b'2'),
        0x04 => Some(b'3'),
        0x05 => Some(b'4'),
        0x06 => Some(b'5'),
        0x07 => Some(b'6'),
        0x08 => Some(b'7'),
        0x09 => Some(b'8'),
        0x0A => Some(b'9'),
        0x0B => Some(b'0'),
        0x10 => Some(b'q'),
        0x11 => Some(b'w'),
        0x12 => Some(b'e'),
        0x13 => Some(b'r'),
        0x14 => Some(b't'),
        0x15 => Some(b'y'),
        0x16 => Some(b'u'),
        0x17 => Some(b'i'),
        0x18 => Some(b'o'),
        0x19 => Some(b'p'),
        0x1C => Some(b'\n'),
        0x1E => Some(b'a'),
        0x1F => Some(b's'),
        0x20 => Some(b'd'),
        0x21 => Some(b'f'),
        0x22 => Some(b'g'),
        0x23 => Some(b'h'),
        0x24 => Some(b'j'),
        0x25 => Some(b'k'),
        0x26 => Some(b'l'),
        0x2C => Some(b'z'),
        0x2D => Some(b'x'),
        0x2E => Some(b'c'),
        0x2F => Some(b'v'),
        0x30 => Some(b'b'),
        0x31 => Some(b'n'),
        0x32 => Some(b'm'),
        0x39 => Some(b' '),
        _ => None,
    }
}

fn remap_pic() {
    let master_mask = port::inb(PIC1_DATA);
    let slave_mask = port::inb(PIC2_DATA);

    port::outb(PIC1_COMMAND, 0x11);
    port::io_wait();
    port::outb(PIC2_COMMAND, 0x11);
    port::io_wait();

    port::outb(PIC1_DATA, TIMER_VECTOR);
    port::io_wait();
    port::outb(PIC2_DATA, 40);
    port::io_wait();

    port::outb(PIC1_DATA, 0x04);
    port::io_wait();
    port::outb(PIC2_DATA, 0x02);
    port::io_wait();

    port::outb(PIC1_DATA, 0x01);
    port::io_wait();
    port::outb(PIC2_DATA, 0x01);
    port::io_wait();

    port::outb(PIC1_DATA, master_mask);
    port::outb(PIC2_DATA, slave_mask);
}

fn set_irq_masks(master_mask: u8, slave_mask: u8) {
    port::outb(PIC1_DATA, master_mask);
    port::outb(PIC2_DATA, slave_mask);
}

fn init_pit(hz: u32) {
    let divisor = (PIT_BASE_FREQUENCY / hz) as u16;
    port::outb(PIT_COMMAND, 0x36);
    port::outb(PIT_CHANNEL0, (divisor & 0x00FF) as u8);
    port::outb(PIT_CHANNEL0, (divisor >> 8) as u8);
}

fn end_of_interrupt(vector: u8) {
    if vector >= 40 {
        port::outb(PIC2_COMMAND, PIC_EOI);
    }
    if vector >= 32 {
        port::outb(PIC1_COMMAND, PIC_EOI);
    }
}
