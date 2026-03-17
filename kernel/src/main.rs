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
const SHELL_HISTORY_LINES: usize = 3;
const SHELL_LINE_WIDTH: usize = 48;

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
    let mut history = [[b' '; SHELL_LINE_WIDTH]; SHELL_HISTORY_LINES];
    let mut history_len = [0usize; SHELL_HISTORY_LINES];
    let mut history_count = 0usize;
    vga::clear_screen(0x1F);
    vga::write_line(2, 8, "TEDDY-OS KERNEL", 0x1F);
    vga::write_line(5, 8, "Rust x86_64 kernel loaded successfully", 0x1E);
    vga::write_line(8, 8, "Checkpoint: VGA console online", 0x17);
    vga::write_line(10, 8, "Kernel boot-info handoff is now visible", 0x1A);
    vga::write_line(11, 8, "Boot contract: BIOS handoff stable", 0x1A);
    vga::write_line(12, 8, "Kernel core is stable again", 0x1F);
    vga::write_line(24, 8, "Enter=submit Backspace=edit  Commands: help clear ticks about", 0x70);

    match boot_info::BootInfo::parse(boot_info_addr) {
        Some(info) => info.render(),
        None => vga::write_line(14, 48, "Boot info parse failed", 0x4F),
    }

    interrupts::init();
    interrupts::render_status();
    push_history_line(
        &mut history,
        &mut history_len,
        &mut history_count,
        b"Shell area online",
        17,
    );
    push_history_line(
        &mut history,
        &mut history_len,
        &mut history_count,
        b"Commands: help clear ticks about",
        32,
    );
    render_history(&history, &history_len, history_count);
    render_input_line(&input_buffer, input_len);
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
                    &mut history,
                    &mut history_len,
                    &mut history_count,
                );
            }
        }
        cpu::halt();
    }
}

fn handle_key(
    ascii: u8,
    input_buffer: &mut [u8; INPUT_BUFFER_LEN],
    input_len: &mut usize,
    history: &mut [[u8; SHELL_LINE_WIDTH]; SHELL_HISTORY_LINES],
    history_len: &mut [usize; SHELL_HISTORY_LINES],
    history_count: &mut usize,
) {
    match ascii {
        8 => {
            if *input_len > 0 {
                *input_len -= 1;
            }
        }
        b'\n' => {
            submit_command(input_buffer, input_len, history, history_len, history_count);
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

fn submit_command(
    input_buffer: &mut [u8; INPUT_BUFFER_LEN],
    input_len: &mut usize,
    history: &mut [[u8; SHELL_LINE_WIDTH]; SHELL_HISTORY_LINES],
    history_len: &mut [usize; SHELL_HISTORY_LINES],
    history_count: &mut usize,
) {
    let command = core::str::from_utf8(&input_buffer[..*input_len]).unwrap_or("");
    let mut command_line = [b' '; SHELL_LINE_WIDTH];
    command_line[0] = b'>';
    command_line[1] = b' ';
    for (index, byte) in input_buffer.iter().take(*input_len).enumerate() {
        if index + 2 >= SHELL_LINE_WIDTH {
            break;
        }
        command_line[index + 2] = *byte;
    }
    push_history_line(history, history_len, history_count, &command_line, *input_len + 2);

    match command {
        "" => push_history_line(history, history_len, history_count, b"", 0),
        "help" => push_history_line(history, history_len, history_count, b"help clear ticks about", 22),
        "clear" => clear_history(history, history_len, history_count),
        "ticks" => {
            let mut text = [b' '; 32];
            let len = format_ticks(&mut text, interrupts::timer_ticks() as u32);
            push_history_line(history, history_len, history_count, &text, len);
        }
        "about" => push_history_line(history, history_len, history_count, b"Teddy-OS tiny shell MVP", 24),
        _ => push_history_line(history, history_len, history_count, b"Unknown command", 15),
    }
    *input_len = 0;
    render_history(history, history_len, *history_count);
    render_input_line(input_buffer, *input_len);
}

fn render_input_line(buffer: &[u8; INPUT_BUFFER_LEN], len: usize) {
    vga::clear_row(23, 0x1F);
    vga::write_line(23, 8, "Input: ", 0x1F);
    for (index, byte) in buffer.iter().take(len).enumerate() {
        vga::write_ascii(23, 15 + index, *byte, 0x1F);
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

fn clear_history(
    history: &mut [[u8; SHELL_LINE_WIDTH]; SHELL_HISTORY_LINES],
    history_len: &mut [usize; SHELL_HISTORY_LINES],
    history_count: &mut usize,
) {
    *history = [[b' '; SHELL_LINE_WIDTH]; SHELL_HISTORY_LINES];
    *history_len = [0; SHELL_HISTORY_LINES];
    *history_count = 0;
}

fn push_history_line(
    history: &mut [[u8; SHELL_LINE_WIDTH]; SHELL_HISTORY_LINES],
    history_len: &mut [usize; SHELL_HISTORY_LINES],
    history_count: &mut usize,
    bytes: &[u8],
    len: usize,
) {
    let row = *history_count % SHELL_HISTORY_LINES;
    history[row] = [b' '; SHELL_LINE_WIDTH];
    let copy_len = core::cmp::min(len, SHELL_LINE_WIDTH);
    for (index, byte) in bytes.iter().take(copy_len).enumerate() {
        history[row][index] = *byte;
    }
    history_len[row] = copy_len;
    *history_count += 1;
}

fn render_history(
    history: &[[u8; SHELL_LINE_WIDTH]; SHELL_HISTORY_LINES],
    history_len: &[usize; SHELL_HISTORY_LINES],
    history_count: usize,
) {
    vga::write_line(20, 8, "Shell:", 0x1F);
    for visible in 0..SHELL_HISTORY_LINES {
        let row = 20 + visible;
        vga::clear_row(row, 0x17);
        if visible == 0 {
            vga::write_line(row, 8, "Shell:", 0x1F);
        }
        let start = history_count.saturating_sub(SHELL_HISTORY_LINES);
        let source = start + visible;
        if source >= history_count {
            continue;
        }
        let index = source % SHELL_HISTORY_LINES;
        let len = history_len[index];
        for col in 0..len {
            vga::write_ascii(row, 16 + col, history[index][col], 0x17);
        }
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
