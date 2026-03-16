#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

mod cpu;
mod input;
mod interrupts;
mod port;
mod terminal;
mod vga;

const KERNEL_STACK_TOP: usize = 0x80000;

global_asm!(
    r#"
    .section .text.boot,"ax"
    .global _start
_start:
    cld
    mov rbx, 0xb8000
    mov ax, 0x2f4b
    mov [rbx], ax
    mov rsp, {stack_top}
    and rsp, -16
    call kernel_main
1:
    pause
    jmp 1b
"#,
    stack_top = const KERNEL_STACK_TOP
);

#[no_mangle]
extern "C" fn kernel_main(boot_info_addr: usize) -> ! {
    let _ = boot_info_addr;
    let mut terminal = terminal::Terminal::new();
    vga::clear_screen(0x1F);
    vga::write_line(2, 8, "TEDDY-OS KERNEL", 0x1F);
    vga::write_line(5, 8, "Rust x86_64 kernel loaded successfully", 0x1E);
    vga::write_line(8, 8, "Checkpoint: VGA console online", 0x17);
    vga::write_line(11, 8, "Boot contract: BIOS handoff stable", 0x1A);
    vga::write_line(12, 8, "Kernel core is stable again", 0x1F);
    vga::write_line(10, 8, "Kernel terminal consumes the IRQ input queue", 0x1A);

    interrupts::init();
    interrupts::render_status();
    terminal.init();
    cpu::enable_interrupts();

    loop {
        while let Some(event) = input::pop_key() {
            let _ = event.scancode;
            if let Some(ascii) = event.ascii {
                terminal.handle_byte(ascii);
            }
        }
        cpu::halt();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    vga::clear_screen(0x4F);
    vga::write_line(10, 8, "TEDDY-OS KERNEL PANIC", 0x4F);
    loop {
        core::hint::spin_loop();
    }
}
