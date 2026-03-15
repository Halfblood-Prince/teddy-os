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
use core::ptr;

use teddy_boot_proto::{BootInfo, BOOTINFO_MAGIC};

#[no_mangle]
pub extern "sysv64" fn kernel_main(boot_info: &'static BootInfo) -> ! {
    paint_early_boot_marker(boot_info, 0x0088_2222);
    serial::init();
    paint_early_boot_marker(boot_info, 0x0088_6622);

    if boot_info.magic != BOOTINFO_MAGIC {
        paint_early_boot_marker(boot_info, 0x00aa_00aa);
        serial::write_str("Invalid BootInfo magic.\n");
        halt_forever();
    }

    paint_early_boot_marker(boot_info, 0x0088_8844);
    logger::init(boot_info);
    paint_early_boot_marker(boot_info, 0x0022_66aa);
    logln!("[boot] logger online");

    logln!("[boot] exception setup deferred for VMware compatibility");

    logln!("[boot] initializing memory manager");
    memory::init(boot_info);
    logln!("[boot] memory online");

    logln!("[boot] initializing timer");
    timer::init();
    logln!("[boot] timer online");

    logln!("[boot] preparing storage subsystem");
    storage::init();
    let storage_info = storage::stats();
    logln!("[boot] storage probe complete: present={}", storage_info.present);

    logln!("[boot] mounting filesystem");
    let mount_status = fs::init();
    logln!(
        "[boot] filesystem ready: mounted={} persistent={} formatted={}",
        mount_status.mounted,
        mount_status.persistent,
        mount_status.formatted
    );

    logln!("[boot] initializing input subsystem");
    let input_status = input::init(
        boot_info.framebuffer.width as usize,
        boot_info.framebuffer.height as usize,
    );
    logln!(
        "[boot] input online: mouse_ready={}",
        input_status.mouse_ready
    );

    logln!("[boot] initializing file explorer");
    file_explorer::init();
    logln!("[boot] file explorer online");

    logln!("[boot] initializing terminal");
    terminal::init();
    logln!("[boot] terminal online");

    logln!("[boot] initializing runtime scheduler");
    runtime::init(boot_info);
    logln!("[boot] runtime online");

    logln!("[boot] interrupt controller setup deferred");

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

    loop {
        timer::advance_polled_tick();
        runtime::run_next_task();
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

fn paint_early_boot_marker(boot_info: &BootInfo, color: u32) {
    let framebuffer = boot_info.framebuffer;
    if !framebuffer.is_valid() {
        return;
    }

    let width = framebuffer.width as usize;
    let height = framebuffer.height as usize;
    let stride = framebuffer.stride as usize;
    let pixels = framebuffer.base as *mut u32;
    let marker_width = width.min(160);
    let marker_height = height.min(48);

    for y in 0..marker_height {
        for x in 0..marker_width {
            unsafe {
                ptr::write_volatile(pixels.add(y * stride + x), color);
            }
        }
    }
}
