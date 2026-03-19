#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

mod boot_info;
mod boot_config;
mod cpu;
mod explorer;
mod font;
mod fs;
mod graphics;
mod image_viewer;
mod input;
mod interrupts;
mod port;
mod shell;
mod storage;
mod terminal;
mod trace;
mod vga;
mod writer;

const KERNEL_STACK_TOP: usize = 0x200000;
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
    let mut last_seen_second = 0u64;
    trace::set_boot_stage(1);
    interrupts::init();
    trace::set_boot_stage(2);
    if let Some(fb) = boot_info::framebuffer_hint(boot_info_addr) {
        trace::set_framebuffer(fb.addr(), fb.width(), fb.height(), fb.pitch(), fb.bpp());
    }
    let boot_info = boot_info::BootInfo::parse(boot_info_addr);
    if let Some(info) = boot_info {
        if info.graphics_mode_enabled() {
            run_graphics_shell(info);
        }
    } else if boot_info::framebuffer_hint(boot_info_addr).is_some() {
        trace::render_graphics_panic("TEDDY-OS KERNEL PANIC", "BOOT INFO INVALID", "CHECK STAGE2 HANDOFF");
        loop {
            cpu::halt();
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

        while let Some(event) = interrupts::consume_keyboard_event() {
            if let Some(action) = desktop.handle_key(event.scancode, event.ascii) {
                match action {
                    shell::ShellAction::Reboot => reboot_system(),
                    shell::ShellAction::Shutdown => shutdown_system(),
                }
            }
        }
        cpu::halt();
    }
}

fn run_graphics_shell(boot_info: boot_info::BootInfo) -> ! {
    let mut last_seen_second = 0u64;
    trace::set_boot_stage(0x70);
    trace::set_boot_stage(0x71);
    if let Some(fb) = boot_info.framebuffer() {
        trace::set_boot_stage(0x72);
        trace::set_framebuffer(fb.addr(), fb.width(), fb.height(), fb.pitch(), fb.bpp());
    }
    trace::set_boot_stage(0x73);
    let shell = unsafe { &mut *core::ptr::addr_of_mut!(GRAPHICS_SHELL) };
    trace::set_boot_stage(0x74);
    if !shell.init(boot_info) {
        trace::render_graphics_panic("TEDDY-OS KERNEL PANIC", "GRAPHICS INIT FAILED", "CHECK BOOT STAGE");
        loop {
            cpu::halt();
        }
    }
    trace::set_boot_stage(0x75);
    shell.render();
    trace::set_boot_stage(0x76);
    cpu::enable_interrupts();
    trace::set_boot_stage(0x77);

    loop {
        trace::set_boot_stage(0x80);
        let uptime_seconds = interrupts::uptime_seconds();
        trace::set_boot_stage(0x81);
        if uptime_seconds != last_seen_second {
            last_seen_second = uptime_seconds;
            trace::set_boot_stage(0x82);
            shell.tick(uptime_seconds);
        }

        trace::set_boot_stage(0x83);
        while let Some(event) = interrupts::consume_keyboard_event() {
            trace::set_boot_stage(0x84);
            if let Some(action) = shell.handle_key(event.scancode, event.ascii) {
                match action {
                    graphics::GraphicsAction::Reboot => reboot_system(),
                    graphics::GraphicsAction::Shutdown => shutdown_system(),
                }
            }
        }

        trace::set_boot_stage(0x85);
        shell.poll_input();

        trace::set_boot_stage(0x86);
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
    trace::render_graphics_panic("TEDDY-OS KERNEL PANIC", "GRAPHICS MODE ACTIVE", "CHECK BOOT STAGE");
    vga::clear_screen(0x4F);
    vga::write_line(10, 8, "TEDDY-OS KERNEL PANIC", 0x4F);
    loop {
        core::hint::spin_loop();
    }
}
