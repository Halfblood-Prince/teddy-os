#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
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
    clear_screen(0x1F);
    write_line(2, 8, "TEDDY-OS KERNEL", 0x1F);
    write_line(5, 8, "Rust x86_64 kernel loaded successfully", 0x1E);
    write_line(8, 8, "Stage 2 entered long mode and jumped into Rust code", 0x17);
    write_line(10, 8, "Checkpoint A: entered kernel_main", 0x1F);
    if let Some(boot_info) = BootInfo::from_addr(boot_info_addr) {
        write_line(11, 8, "Boot contract: stage 2 handoff verified", 0x1A);
        write_line(12, 8, "Checkpoint B: boot info validated", 0x1F);
        let _ = boot_info.boot_drive;
        let _ = boot_info.kernel_segment;
        let _ = boot_info.kernel_sectors;
    } else {
        write_line(11, 8, "Boot contract: invalid handoff signature", 0x4F);
    }
    write_line(14, 8, "Checkpoint C: entering idle loop", 0x1F);
    write_line(22, 8, "Kernel idle loop active", 0x70);

    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
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

#[repr(C)]
struct BootInfo {
    signature: [u8; 8],
    version: u8,
    boot_drive: u8,
    kernel_segment: u16,
    kernel_sectors: u16,
    stage2_sectors: u16,
}

impl BootInfo {
    fn from_addr(addr: usize) -> Option<&'static Self> {
        let info = unsafe { &*(addr as *const Self) };
        if &info.signature == b"TEDDYOS\0" && info.version == 1 {
            Some(info)
        } else {
            None
        }
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
