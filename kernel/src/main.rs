#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

mod framebuffer;
mod file_explorer;
mod fs;
mod input;
mod interrupts;
mod logger;
mod memory;
mod memory_intrinsics;
mod runtime;
mod serial;
mod shell;
mod storage;
mod terminal;
mod timer;

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
    logln!("Boot step: logger online");
    memory::init(boot_info);
    logln!("Boot step: memory online");
    timer::init();
    logln!("Boot step: timer online");
    logln!("Boot step: entering storage init");
    storage::init();
    logln!("Boot step: storage probe complete");
    logln!("Boot step: entering filesystem init");
    let mount_status = fs::init();
    let storage_info = storage::stats();
    logln!(
        "Boot step: filesystem probe complete (mounted={} persistent={})",
        mount_status.mounted,
        mount_status.persistent
    );
    let input_status = input::init(
        boot_info.framebuffer.width as usize,
        boot_info.framebuffer.height as usize,
    );
    logln!(
        "Boot step: input online (mouse_ready={})",
        input_status.mouse_ready
    );
    file_explorer::init();
    logln!("Boot step: explorer online");
    terminal::init();
    logln!("Boot step: terminal online");
    runtime::init(boot_info);
    logln!("Boot step: runtime online");
    interrupts::init();
    logln!("Boot step: interrupts online");

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

    let stats = memory::stats();
    logln!("");
    logln!("Teddy-OS kernel services initialized.");
    logln!(
        "Memory: total={} bytes usable={} bytes reserved={} bytes bootloader={} bytes kernel={} bytes",
        stats.total_bytes,
        stats.usable_bytes,
        stats.reserved_bytes,
        stats.bootloader_bytes,
        stats.kernel_bytes
    );

    if let Some(allocation) = memory::allocate_frames(4) {
        logln!(
            "Boot frame allocator test: allocated {} frames at {:#018x}",
            allocation.frames,
            allocation.start
        );
    } else {
        logln!("Boot frame allocator test: no usable frames available.");
    }

    logln!(
        "Interrupts online: {}. PIT frequency {} Hz.",
        interrupts::is_initialized(),
        interrupts::timer_frequency_hz()
    );
    logln!(
        "Storage: present={} persistent_fs={} formatted={}",
        storage_info.present,
        mount_status.persistent,
        mount_status.formatted
    );
    if storage_info.present {
        logln!(
            "Storage model={} sectors={} sector_size={}",
            storage_info.model.as_str(),
            storage_info.total_sectors,
            storage_info.sector_size
        );
    }
    logln!("Keyboard and mouse IRQ handlers armed. Entering desktop shell runtime.");

    interrupts::enable();

    loop {
        runtime::run_next_task();
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    serial::write_str("\nKERNEL PANIC\n");
    interrupts::disable();
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
