#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

mod boot_info;
mod vga;

use boot_info::BootInfo;

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

    if let Some(info) = BootInfo::from_addr(boot_info_addr) {
        vga::write_line(11, 8, "Boot contract: verified", 0x1A);
        vga::write_line(12, 8, "Checkpoint: boot info parsed", 0x1F);
        vga::write_hex_byte(13, 8, "Boot drive: 0x", info.boot_drive, 0x1F);
        vga::write_hex_word(14, 8, "Kernel segment: 0x", info.kernel_segment, 0x1F);
        vga::write_hex_word(15, 8, "Kernel sectors: 0x", info.kernel_sectors, 0x1F);
        vga::write_hex_word(16, 8, "Stage 2 sectors: 0x", info.stage2_sectors, 0x1F);
    } else {
        vga::write_line(11, 8, "Boot contract: invalid signature", 0x4F);
    }

    vga::write_line(22, 8, "Kernel idle loop active", 0x70);

    loop {
        core::hint::spin_loop();
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
