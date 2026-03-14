use core::sync::atomic::{AtomicBool, Ordering};

use pic8259::ChainedPics;
use spin::{Mutex, Once};
use x86_64::instructions::{interrupts as cpu_interrupts, port::Port};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

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
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
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
    idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
    idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
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

fn notify_end_of_interrupt(index: InterruptIndex) {
    let mut pics = PICS.lock();
    if let Some(pics) = pics.as_mut() {
        unsafe {
            pics.notify_end_of_interrupt(index.as_u8());
        }
    }
}
