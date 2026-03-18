use core::arch::global_asm;
use core::sync::atomic::{AtomicI32, AtomicU8, AtomicU64, AtomicUsize, Ordering};

use crate::{cpu, input::MousePacket, port, trace, vga};

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
const MOUSE_VECTOR: u8 = 44;
const PS2_DATA: u16 = 0x60;
const PS2_STATUS: u16 = 0x64;
const PS2_COMMAND: u16 = 0x64;

static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);
static LAST_SCANCODE: AtomicU8 = AtomicU8::new(0);
static LAST_ASCII: AtomicU8 = AtomicU8::new(b'-');
static KEYBOARD_WRITE_INDEX: AtomicUsize = AtomicUsize::new(0);
static KEYBOARD_READ_INDEX: AtomicUsize = AtomicUsize::new(0);
static MOUSE_SEQ: AtomicU64 = AtomicU64::new(0);
static MOUSE_DELTA_X: AtomicI32 = AtomicI32::new(0);
static MOUSE_DELTA_Y: AtomicI32 = AtomicI32::new(0);
static MOUSE_BUTTONS: AtomicU8 = AtomicU8::new(0);
static MOUSE_PRESSED: AtomicU8 = AtomicU8::new(0);
static MOUSE_RELEASED: AtomicU8 = AtomicU8::new(0);

static mut MOUSE_PACKET_INDEX: u8 = 0;
static mut MOUSE_PACKET_BYTES: [u8; 3] = [0; 3];
static mut KEYBOARD_QUEUE: [KeyboardEvent; 32] = [KeyboardEvent::empty(); 32];

#[derive(Clone, Copy)]
pub struct KeyboardEvent {
    pub scancode: u8,
    pub ascii: u8,
}

impl KeyboardEvent {
    const fn empty() -> Self {
        Self {
            scancode: 0,
            ascii: 0,
        }
    }
}

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
        // The BIOS handoff jumps into long mode on GDT selector 0x18.
        // Interrupt gates must use the same 64-bit code segment.
        self.selector = 0x18;
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
        let mut index = 0usize;
        while index < 48 {
            let handler = isr_stub_table[index];
            IDT[index].set_handler(handler);
            index += 1;
        }
        cpu::load_idt(
            core::ptr::addr_of!(IDT) as *const _ as u64,
            (core::mem::size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
        );
    }

    remap_pic();
    init_pit(PIT_TICKS_PER_SECOND);
    init_ps2_mouse();
    // Unmask timer, keyboard, and IRQ2 so slave IRQ12 mouse events can reach the CPU.
    set_irq_masks(0b1111_1000, 0b1110_1111);
}

pub fn timer_ticks() -> u64 {
    TIMER_TICKS.load(Ordering::Relaxed)
}

pub fn last_scancode() -> u8 {
    LAST_SCANCODE.load(Ordering::Relaxed)
}

pub fn last_ascii() -> u8 {
    LAST_ASCII.load(Ordering::Relaxed)
}

pub fn uptime_seconds() -> u64 {
    timer_ticks() / PIT_TICKS_PER_SECOND as u64
}

pub fn consume_keyboard_event() -> Option<KeyboardEvent> {
    let read = KEYBOARD_READ_INDEX.load(Ordering::Acquire);
    let write = KEYBOARD_WRITE_INDEX.load(Ordering::Acquire);
    if read == write {
        return None;
    }

    let event = unsafe { KEYBOARD_QUEUE[read % 32] };
    KEYBOARD_READ_INDEX.store(read.wrapping_add(1), Ordering::Release);
    Some(event)
}

pub fn consume_mouse_packet(last_seq: u64) -> Option<MousePacket> {
    let seq = MOUSE_SEQ.load(Ordering::Acquire);
    if seq == last_seq {
        return None;
    }

    Some(MousePacket {
        seq,
        dx: MOUSE_DELTA_X.swap(0, Ordering::AcqRel),
        dy: MOUSE_DELTA_Y.swap(0, Ordering::AcqRel),
        buttons: MOUSE_BUTTONS.load(Ordering::Acquire),
        pressed: MOUSE_PRESSED.swap(0, Ordering::AcqRel),
        released: MOUSE_RELEASED.swap(0, Ordering::AcqRel),
    })
}

#[no_mangle]
extern "C" fn interrupt_dispatch(vector: u64, error_code: u64, stack_frame: *const InterruptStackFrame) {
    match vector as u8 {
        TIMER_VECTOR => handle_timer_irq(),
        KEYBOARD_VECTOR => handle_keyboard_irq(),
        MOUSE_VECTOR => handle_mouse_irq(),
        _ if vector < 32 => handle_exception(vector as u8, error_code, stack_frame),
        _ => end_of_interrupt(vector as u8),
    }
}

fn handle_timer_irq() {
    TIMER_TICKS.fetch_add(1, Ordering::Relaxed);
    end_of_interrupt(TIMER_VECTOR);
}

fn handle_keyboard_irq() {
    let scancode = port::inb(PS2_DATA);
    LAST_SCANCODE.store(scancode, Ordering::Relaxed);
    if scancode & 0x80 == 0 {
        let ascii = decode_scancode(scancode);
        LAST_ASCII.store(ascii, Ordering::Relaxed);
        push_keyboard_event(scancode, ascii);
    }
    end_of_interrupt(KEYBOARD_VECTOR);
}

fn handle_mouse_irq() {
    let byte = port::inb(PS2_DATA);

    unsafe {
        if MOUSE_PACKET_INDEX == 0 && byte & 0x08 == 0 {
            end_of_interrupt(MOUSE_VECTOR);
            return;
        }

        MOUSE_PACKET_BYTES[MOUSE_PACKET_INDEX as usize] = byte;
        MOUSE_PACKET_INDEX += 1;

        if MOUSE_PACKET_INDEX == 3 {
            MOUSE_PACKET_INDEX = 0;
            process_mouse_packet(MOUSE_PACKET_BYTES);
        }
    }

    end_of_interrupt(MOUSE_VECTOR);
}

fn handle_exception(vector: u8, error_code: u64, stack_frame: *const InterruptStackFrame) {
    let frame = unsafe { &*stack_frame };
    trace::render_graphics_exception(vector, error_code, frame.instruction_pointer);
    vga::clear_screen(0x4F);
    vga::write_line(4, 8, "TEDDY-OS EXCEPTION", 0x4F);
    vga::write_line(6, 8, "Vector:", 0x4F);
    vga::write_hex_byte(6, 16, "", vector, 0x4F);
    vga::write_line(7, 8, "Error code:", 0x4F);
    vga::write_hex_dword(7, 20, error_code as u32, 0x4F);
    vga::write_line(8, 8, "Boot stage:", 0x4F);
    vga::write_hex_byte(8, 20, "", trace::boot_stage(), 0x4F);
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
        0x0C => b'-',
        0x0F => b'\t',
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
        0x33 => b',',
        0x34 => b'.',
        0x35 => b'/',
        0x39 => b' ',
        0x01 => 27,
        _ => b'?',
    }
}

fn push_keyboard_event(scancode: u8, ascii: u8) {
    let write = KEYBOARD_WRITE_INDEX.load(Ordering::Relaxed);
    let read = KEYBOARD_READ_INDEX.load(Ordering::Acquire);
    let next = write.wrapping_add(1);
    if next.wrapping_sub(read) > 32 {
        KEYBOARD_READ_INDEX.store(read.wrapping_add(1), Ordering::Release);
    }

    unsafe {
        KEYBOARD_QUEUE[write % 32] = KeyboardEvent { scancode, ascii };
    }
    KEYBOARD_WRITE_INDEX.store(next, Ordering::Release);
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

fn init_ps2_mouse() {
    ps2_wait_write();
    port::outb(PS2_COMMAND, 0xA8);

    ps2_wait_write();
    port::outb(PS2_COMMAND, 0x20);
    ps2_wait_read();
    let mut config = port::inb(PS2_DATA);
    config |= 0x02;
    config &= !0x20;

    ps2_wait_write();
    port::outb(PS2_COMMAND, 0x60);
    ps2_wait_write();
    port::outb(PS2_DATA, config);

    mouse_write(0xF6);
    let _ = mouse_read();
    mouse_write(0xF4);
    let _ = mouse_read();
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

fn process_mouse_packet(packet: [u8; 3]) {
    let status = packet[0];
    if status & 0xC0 != 0 {
        return;
    }

    let dx = sign_extend(packet[1], status & 0x10 != 0);
    let dy = sign_extend(packet[2], status & 0x20 != 0);
    let buttons = status & 0x07;
    let previous = MOUSE_BUTTONS.swap(buttons, Ordering::AcqRel);
    let pressed = (!previous) & buttons;
    let released = previous & (!buttons);

    MOUSE_DELTA_X.fetch_add(dx, Ordering::AcqRel);
    MOUSE_DELTA_Y.fetch_add(dy, Ordering::AcqRel);
    if pressed != 0 {
        MOUSE_PRESSED.fetch_or(pressed, Ordering::AcqRel);
    }
    if released != 0 {
        MOUSE_RELEASED.fetch_or(released, Ordering::AcqRel);
    }
    MOUSE_SEQ.fetch_add(1, Ordering::Release);
}

fn sign_extend(byte: u8, negative: bool) -> i32 {
    if negative {
        (byte as i32) - 256
    } else {
        byte as i32
    }
}

fn ps2_wait_write() {
    let mut spins = 0usize;
    while spins < 100_000 {
        if port::inb(PS2_STATUS) & 0x02 == 0 {
            return;
        }
        spins += 1;
    }
}

fn ps2_wait_read() {
    let mut spins = 0usize;
    while spins < 100_000 {
        if port::inb(PS2_STATUS) & 0x01 != 0 {
            return;
        }
        spins += 1;
    }
}

fn mouse_write(value: u8) {
    ps2_wait_write();
    port::outb(PS2_COMMAND, 0xD4);
    ps2_wait_write();
    port::outb(PS2_DATA, value);
}

fn mouse_read() -> u8 {
    ps2_wait_read();
    port::inb(PS2_DATA)
}
