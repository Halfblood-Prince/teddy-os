#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

global_asm!(
    r#"
    .section .text
    .global _start
_start:
    mov $0x9f000, %rsp
    and $-16, %rsp
    call kernel_main
1:
    pause
    jmp 1b
"#
);

#[no_mangle]
extern "C" fn kernel_main() -> ! {
    clear_screen(0x1F);
    write_line(2, 8, "TEDDY-OS KERNEL", 0x1F);
    write_line(5, 8, "Rust x86_64 kernel loaded successfully", 0x1E);
    write_line(8, 8, "Stage 2 entered long mode and jumped into Rust code", 0x17);
    write_line(11, 8, "Next: boot info, memory map, interrupts, filesystem", 0x1A);
    write_line(22, 8, "Rust kernel active - halt loop", 0x70);

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    clear_screen(0x4F);
    write_line(10, 8, "TEDDY-OS KERNEL PANIC", 0x4F);
    loop {
        core::hint::spin_loop();
    }
}

fn clear_screen(attribute: u8) {
    for row in 0..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            write_cell(row, col, b' ', attribute);
        }
    }
}

fn write_line(row: usize, col: usize, text: &str, attribute: u8) {
    for (index, byte) in text.bytes().enumerate() {
        let x = col + index;
        if x >= VGA_WIDTH {
            break;
        }
        write_cell(row, x, byte, attribute);
    }
}

fn write_cell(row: usize, col: usize, byte: u8, attribute: u8) {
    let index = (row * VGA_WIDTH + col) * 2;
    unsafe {
        VGA_BUFFER.add(index).write_volatile(byte);
        VGA_BUFFER.add(index + 1).write_volatile(attribute);
    }
}
