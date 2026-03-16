#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

mod boot_info;
mod cpu;
mod interrupts;
mod port;
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
    vga::clear_screen(0x1F);
    vga::write_line(2, 8, "TEDDY-OS KERNEL", 0x1F);
    vga::write_line(5, 8, "Rust x86_64 kernel loaded successfully", 0x1E);
    vga::write_line(8, 8, "Checkpoint: VGA console online", 0x17);
    vga::write_line(11, 8, "Boot contract: BIOS handoff stable", 0x1A);
    vga::write_line(12, 8, "Kernel core is stable again", 0x1F);
    vga::write_line(22, 8, "Timer + keyboard IRQs armed", 0x70);
    vga::write_line(23, 8, "Press keys in VMware to test PS/2 input", 0x70);

    match boot_info::BootInfo::parse(boot_info_addr) {
        Some(info) => info.render(),
        None => vga::write_line(14, 48, "Boot info parse failed", 0x4F),
    }

    interrupts::init();
    interrupts::render_status();
    cpu::enable_interrupts();

    loop {
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
