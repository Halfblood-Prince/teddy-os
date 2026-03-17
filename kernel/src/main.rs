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
const KEY_PREVIEW_LEN: usize = 32;

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
    let mut recent_keys = [b' '; KEY_PREVIEW_LEN];
    vga::clear_screen(0x1F);
    vga::write_line(2, 8, "TEDDY-OS KERNEL", 0x1F);
    vga::write_line(5, 8, "Rust x86_64 kernel loaded successfully", 0x1E);
    vga::write_line(8, 8, "Checkpoint: VGA console online", 0x17);
    vga::write_line(10, 8, "Kernel boot-info handoff is now visible", 0x1A);
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
    render_recent_keys(&recent_keys);
    cpu::enable_interrupts();

    loop {
        let scancode = interrupts::last_scancode();
        if scancode != last_seen_scancode {
            last_seen_scancode = scancode;
            if scancode & 0x80 == 0 {
                push_recent_key(&mut recent_keys, interrupts::last_ascii());
                render_recent_keys(&recent_keys);
            }
        }
        cpu::halt();
    }
}

fn push_recent_key(buffer: &mut [u8; KEY_PREVIEW_LEN], ascii: u8) {
    let byte = match ascii {
        8 => b'<',
        b'\n' => b'#',
        0x20..=0x7E => ascii,
        _ => b'.',
    };

    for index in 1..KEY_PREVIEW_LEN {
        buffer[index - 1] = buffer[index];
    }
    buffer[KEY_PREVIEW_LEN - 1] = byte;
}

fn render_recent_keys(buffer: &[u8; KEY_PREVIEW_LEN]) {
    vga::write_line(20, 8, "Recent keys:", 0x1F);
    for (index, byte) in buffer.iter().enumerate() {
        vga::write_ascii(20, 21 + index, *byte, 0x1F);
    }
    vga::write_line(21, 8, "< = backspace, # = enter", 0x17);
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    vga::clear_screen(0x4F);
    vga::write_line(10, 8, "TEDDY-OS KERNEL PANIC", 0x4F);
    loop {
        core::hint::spin_loop();
    }
}
