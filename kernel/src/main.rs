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
const INPUT_BUFFER_LEN: usize = 32;

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
    let mut input_buffer = [0u8; INPUT_BUFFER_LEN];
    let mut input_len = 0usize;
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
    render_input_line(&input_buffer, input_len);
    render_result_line("Commands: help, clear, ticks, about");
    cpu::enable_interrupts();

    loop {
        let scancode = interrupts::last_scancode();
        if scancode != last_seen_scancode {
            last_seen_scancode = scancode;
            if scancode & 0x80 == 0 {
                handle_key(
                    interrupts::last_ascii(),
                    &mut input_buffer,
                    &mut input_len,
                );
            }
        }
        cpu::halt();
    }
}

fn handle_key(ascii: u8, input_buffer: &mut [u8; INPUT_BUFFER_LEN], input_len: &mut usize) {
    match ascii {
        8 => {
            if *input_len > 0 {
                *input_len -= 1;
            }
        }
        b'\n' => {
            submit_command(input_buffer, input_len);
        }
        0x20..=0x7E => {
            if *input_len < INPUT_BUFFER_LEN {
                input_buffer[*input_len] = ascii;
                *input_len += 1;
            }
        }
        _ => {}
    }
    render_input_line(input_buffer, *input_len);
}

fn submit_command(input_buffer: &mut [u8; INPUT_BUFFER_LEN], input_len: &mut usize) {
    let command = core::str::from_utf8(&input_buffer[..*input_len]).unwrap_or("");
    match command {
        "" => render_result_line(""),
        "help" => render_result_line("help clear ticks about"),
        "clear" => render_result_line(""),
        "ticks" => {
            let mut text = [b' '; 32];
            let len = format_ticks(&mut text, interrupts::timer_ticks() as u32);
            render_result_bytes(&text, len);
        }
        "about" => render_result_line("Teddy-OS one-line input MVP"),
        _ => render_result_line("Unknown command"),
    }
    *input_len = 0;
    render_input_line(input_buffer, *input_len);
}

fn render_input_line(buffer: &[u8; INPUT_BUFFER_LEN], len: usize) {
    vga::clear_row(20, 0x1F);
    vga::write_line(20, 8, "Input: ", 0x1F);
    for (index, byte) in buffer.iter().take(len).enumerate() {
        vga::write_ascii(20, 15 + index, *byte, 0x1F);
    }
    vga::write_line(20, 50, "Enter=submit Backspace=edit", 0x17);
}

fn render_result_line(text: &str) {
    vga::clear_row(21, 0x17);
    vga::write_line(21, 8, "Result: ", 0x17);
    vga::write_line(21, 16, text, 0x17);
}

fn render_result_bytes(bytes: &[u8; 32], len: usize) {
    vga::clear_row(21, 0x17);
    vga::write_line(21, 8, "Result: ", 0x17);
    for (index, byte) in bytes.iter().take(len).enumerate() {
        vga::write_ascii(21, 16 + index, *byte, 0x17);
    }
}

fn format_ticks(buffer: &mut [u8; 32], value: u32) -> usize {
    let prefix = b"ticks=0x";
    let mut len = 0;
    for byte in prefix {
        buffer[len] = *byte;
        len += 1;
    }
    let mut started = false;
    for shift in (0..8).rev() {
        let nibble = ((value >> (shift * 4)) & 0x0F) as u8;
        if nibble != 0 || started || shift == 0 {
            buffer[len] = match nibble {
                0..=9 => b'0' + nibble,
                _ => b'A' + (nibble - 10),
            };
            len += 1;
            started = true;
        }
    }
    len
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    vga::clear_screen(0x4F);
    vga::write_line(10, 8, "TEDDY-OS KERNEL PANIC", 0x4F);
    loop {
        core::hint::spin_loop();
    }
}
