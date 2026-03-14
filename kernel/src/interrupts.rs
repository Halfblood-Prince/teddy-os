use core::sync::atomic::{AtomicBool, Ordering};

use pic8259::ChainedPics;
use spin::{Mutex, Once};
use x86_64::registers::control::Cr2;
use x86_64::instructions::{interrupts as cpu_interrupts, port::Port};
use x86_64::structures::idt::{
    InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
};

use crate::{input, logln, timer};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
const PIT_FREQUENCY_HZ: u32 = 100;

static IDT: Once<InterruptDescriptorTable> = Once::new();
static PICS: Mutex<Option<ChainedPics>> = Mutex::new(None);
static INITIALIZED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    Mouse = PIC_2_OFFSET + 4,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

}

pub fn init() {
    let idt = IDT.call_once(build_idt);
    idt.load();

    {
        let mut pics = PICS.lock();
        *pics = Some(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
        unsafe {
            pics.as_mut().unwrap().initialize();
        }
    }

    configure_irq_masks(input::mouse_ready());
    initialize_pit(PIT_FREQUENCY_HZ);
    INITIALIZED.store(true, Ordering::SeqCst);
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

pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::SeqCst)
}

fn build_idt() -> InterruptDescriptorTable {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.double_fault.set_handler_fn(double_fault_handler);
    idt.divide_error.set_handler_fn(divide_error_handler);
    idt.general_protection_fault
        .set_handler_fn(general_protection_fault_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
    idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
    idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
    idt[InterruptIndex::Mouse.as_u8()].set_handler_fn(mouse_interrupt_handler);
    idt
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    logln!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT ({error_code})\n{stack_frame:#?}");
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: DIVIDE ERROR\n{stack_frame:#?}");
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT ({error_code:#x})\n{stack_frame:#?}"
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let fault_address = Cr2::read();
    panic!(
        "EXCEPTION: PAGE FAULT addr={fault_address:?} error={error_code:?}\n{stack_frame:#?}"
    );
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: INVALID OPCODE\n{stack_frame:#?}");
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    timer::on_tick();
    notify_end_of_interrupt(InterruptIndex::Timer);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::<u8>::new(0x60);
    let scancode = unsafe { port.read() };
    input::handle_scancode(scancode);
    notify_end_of_interrupt(InterruptIndex::Keyboard);
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::<u8>::new(0x60);
    let byte = unsafe { port.read() };
    input::handle_mouse_byte(byte);
    notify_end_of_interrupt(InterruptIndex::Mouse);
}

fn initialize_pit(frequency_hz: u32) {
    let divisor = (1_193_182 / frequency_hz.max(1)) as u16;
    let mut command = Port::<u8>::new(0x43);
    let mut data = Port::<u8>::new(0x40);

    unsafe {
        command.write(0x36);
        data.write((divisor & 0x00FF) as u8);
        data.write((divisor >> 8) as u8);
    }
}

fn configure_irq_masks(mouse_ready: bool) {
    let mut master_mask: u8 = 0xFF;
    let mut slave_mask: u8 = 0xFF;

    master_mask &= !(1 << 0);
    master_mask &= !(1 << 1);
    master_mask &= !(1 << 2);
    if mouse_ready {
        slave_mask &= !(1 << 4);
    }

    unsafe {
        Port::<u8>::new(0x21).write(master_mask);
        Port::<u8>::new(0xA1).write(slave_mask);
    }
}

fn notify_end_of_interrupt(index: InterruptIndex) {
    let mut pics = PICS.lock();
    if let Some(pics) = pics.as_mut() {
        unsafe {
            pics.notify_end_of_interrupt(index.as_u8());
        }
    }
}
