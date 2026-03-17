#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

mod boot_info;
mod cpu;
mod interrupts;
mod port;
mod shell;
mod terminal;
mod vga;

const KERNEL_STACK_TOP: usize = 0x80000;
static mut DESKTOP_SHELL: shell::DesktopShell = shell::DesktopShell::empty();

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
    let mut last_seen_scancode = 0u8;
    let mut last_seen_second = 0u64;
    interrupts::init();
    let boot_info = boot_info::BootInfo::parse(boot_info_addr);
    let desktop = unsafe { &mut *core::ptr::addr_of_mut!(DESKTOP_SHELL) };
    desktop.init(boot_info);
    desktop.render();
    cpu::enable_interrupts();

    loop {
        let uptime_seconds = interrupts::uptime_seconds();
        if uptime_seconds != last_seen_second {
            last_seen_second = uptime_seconds;
            desktop.tick(uptime_seconds);
        }

        let scancode = interrupts::last_scancode();
        if scancode != last_seen_scancode {
            last_seen_scancode = scancode;
            if scancode & 0x80 == 0 {
                if let Some(action) = desktop.handle_key(scancode, interrupts::last_ascii()) {
                    match action {
                        shell::ShellAction::Reboot => reboot_system(),
                        shell::ShellAction::Shutdown => shutdown_system(),
                    }
                }
            }
        }
        cpu::halt();
    }
}

fn reboot_system() -> ! {
    port::outb(0x64, 0xFE);
    loop {
        cpu::halt();
    }
}

fn shutdown_system() -> ! {
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
