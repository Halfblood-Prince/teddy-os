#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

mod boot_info;
mod cpu;
mod explorer;
mod fs;
mod graphics;
mod input;
mod interrupts;
mod port;
mod shell;
mod storage;
mod terminal;
mod trace;
mod vga;

const KERNEL_STACK_TOP: usize = 0x80000;
static mut DESKTOP_SHELL: shell::DesktopShell = shell::DesktopShell::empty();
static mut GRAPHICS_SHELL: graphics::GraphicsShell = graphics::GraphicsShell::empty();

global_asm!(
    r#"
    .section .text.boot,"ax"
    .global _start
_start:
    cld
    mov rbx, 0xb8000
    mov ax, 0x2f4b
    mov [rbx], ax
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
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
    trace::set_boot_stage(1);
    interrupts::init();
    trace::set_boot_stage(2);
    let boot_info = boot_info::BootInfo::parse(boot_info_addr);
    if let Some(info) = boot_info {
        if info.graphics_mode_enabled() {
            run_graphics_shell(info);
        }
    }
    trace::set_boot_stage(3);
    let desktop = unsafe { &mut *core::ptr::addr_of_mut!(DESKTOP_SHELL) };
    trace::set_boot_stage(0x30);
    desktop.init(boot_info);
    trace::set_boot_stage(4);
    desktop.render();
    trace::set_boot_stage(5);
    cpu::enable_interrupts();
    trace::set_boot_stage(6);

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

fn run_graphics_shell(boot_info: boot_info::BootInfo) -> ! {
    let mut last_seen_scancode = 0u8;
    let mut last_seen_second = 0u64;
    trace::set_boot_stage(0x70);
    let shell = unsafe { &mut *core::ptr::addr_of_mut!(GRAPHICS_SHELL) };
    if !shell.init(boot_info) {
        loop {
            cpu::halt();
        }
    }
    trace::set_boot_stage(0x71);
    shell.render();
    trace::set_boot_stage(0x72);
    cpu::enable_interrupts();
    trace::set_boot_stage(0x73);

    loop {
        let uptime_seconds = interrupts::uptime_seconds();
        if uptime_seconds != last_seen_second {
            last_seen_second = uptime_seconds;
            shell.tick(uptime_seconds);
        }

        let scancode = interrupts::last_scancode();
        if scancode != last_seen_scancode {
            last_seen_scancode = scancode;
            if scancode & 0x80 == 0 {
                if let Some(action) = shell.handle_key(interrupts::last_ascii()) {
                    match action {
                        graphics::GraphicsAction::Reboot => reboot_system(),
                        graphics::GraphicsAction::Shutdown => shutdown_system(),
                    }
                }
            }
        }

        shell.poll_input();

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
