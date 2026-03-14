#![no_std]
#![no_main]

mod framebuffer;
mod logger;
mod serial;

use core::panic::PanicInfo;

use teddy_boot_proto::{BootInfo, BOOTINFO_MAGIC};

#[no_mangle]
pub extern "sysv64" fn kernel_main(boot_info: &'static BootInfo) -> ! {
    serial::init();

    if boot_info.magic != BOOTINFO_MAGIC {
        serial::write_str("Invalid BootInfo magic.\n");
        halt_forever();
    }

    logger::init(boot_info);

    logln!("Teddy-OS kernel entered.");
    logln!(
        "Kernel image: {:#018x} - {:#018x}",
        boot_info.kernel_start,
        boot_info.kernel_end
    );
    logln!(
        "Framebuffer: {}x{} stride {}",
        boot_info.framebuffer.width,
        boot_info.framebuffer.height,
        boot_info.framebuffer.stride
    );
    logln!("RSDP: {:#018x}", boot_info.rsdp_addr);
    logln!("Memory regions discovered: {}", boot_info.memory_regions().len());

    for (index, region) in boot_info.memory_regions().iter().take(12).enumerate() {
        logln!(
            "  [{}] start={:#018x} len={:#010x} kind={:?}",
            index,
            region.start,
            region.len,
            region.kind
        );
    }

    logln!("");
    logln!("Phase 1 foundation initialized.");
    logln!("System halted intentionally until Phase 2 adds interrupts and input.");

    halt_forever();
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    serial::write_str("\nKERNEL PANIC\n");
    logln!("");
    logln!("KERNEL PANIC: {}", info);
    halt_forever();
}

fn halt_forever() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

