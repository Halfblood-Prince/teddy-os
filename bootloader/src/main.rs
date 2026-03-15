#![no_std]
#![no_main]

extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec};
use core::{mem, ptr};
use teddy_boot_proto::{
    BootInfo, FramebufferInfo, MemoryRegion, MemoryRegionKind, PixelFormat, MAX_MEMORY_REGIONS,
};
use uefi::boot::{self, AllocateType, MemoryType};
use uefi::cstr16;
use uefi::mem::memory_map::MemoryMap;
use uefi::prelude::*;
use uefi::proto::console::gop::{
    GraphicsOutput,
    Mode,
    PixelFormat as GopPixelFormat,
    PixelFormat as UefiPixelFormat,
};
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode, FileType, RegularFile};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::cfg::{ACPI2_GUID, ACPI_GUID};
use xmas_elf::{
    program::{ProgramHeader, Type},
    ElfFile,
};

const KERNEL_PATH: &uefi::CStr16 = cstr16!(r"\EFI\BOOT\KERNEL.ELF");

#[entry]
fn efi_main() -> Status {
    if uefi::helpers::init().is_err() {
        return Status::LOAD_ERROR;
    }

    uefi::println!("Teddy-OS bootloader");

    match boot() {
        Ok(()) => Status::SUCCESS,
        Err(status) => {
            uefi::println!("Boot failed: {:?}", status);
            status
        }
    }
}

fn boot() -> Result<(), Status> {
    let image_handle = boot::image_handle();

    uefi::println!("Loading kernel file...");
    let kernel_file = read_kernel_file(image_handle)?;
    uefi::println!("Kernel file loaded ({} bytes).", kernel_file.len());

    uefi::println!("Initializing framebuffer...");
    let framebuffer = init_framebuffer()?;
    uefi::println!(
        "Framebuffer: {}x{} stride {}",
        framebuffer.width,
        framebuffer.height,
        framebuffer.stride
    );

    uefi::println!("Loading kernel ELF...");
    let loaded_kernel = load_kernel_elf(&kernel_file)?;
    uefi::println!(
        "Kernel ELF loaded: start={:#x}, end={:#x}, entry={:#x}",
        loaded_kernel.start,
        loaded_kernel.end,
        loaded_kernel.entry
    );

    let rsdp_addr = find_rsdp();
    let mut boot_info = Box::new(BootInfo::new());
    let mut memory_regions = Box::new([MemoryRegion::EMPTY; MAX_MEMORY_REGIONS]);

    boot_info.framebuffer = framebuffer;
    boot_info.rsdp_addr = rsdp_addr;
    boot_info.kernel_start = loaded_kernel.start;
    boot_info.kernel_end = loaded_kernel.end;

    uefi::println!("Exiting UEFI boot services...");
    let memory_map = unsafe { boot::exit_boot_services(MemoryType::LOADER_DATA) };

    let mut count = 0usize;
    for descriptor in memory_map.entries() {
        if count >= MAX_MEMORY_REGIONS {
            break;
        }

        memory_regions[count] = MemoryRegion {
            start: descriptor.phys_start,
            len: descriptor.page_count * 4096,
            kind: map_memory_kind(descriptor.ty),
        };
        count += 1;
    }

    boot_info.memory_regions_ptr = memory_regions.as_mut_ptr() as u64;
    boot_info.memory_regions_len = count as u64;

    let boot_info_ptr = Box::into_raw(boot_info);
    let _memory_regions_ptr = Box::into_raw(memory_regions);
    mem::forget(memory_map);
    paint_debug_marker(framebuffer, 0x0022_8844);

    let entry: extern "sysv64" fn(&'static BootInfo) -> ! =
        unsafe { mem::transmute(loaded_kernel.entry as usize) };

    entry(unsafe { &*boot_info_ptr });
}

fn read_kernel_file(
    image_handle: Handle,
) -> Result<Vec<u8>, Status> {
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(image_handle)
        .map_err(|err| err.status())?;
    let device_handle = loaded_image.device().ok_or(Status::LOAD_ERROR)?;
    let mut fs = boot::open_protocol_exclusive::<SimpleFileSystem>(device_handle)
        .map_err(|err| err.status())?;
    let mut root = fs.open_volume().map_err(|err| err.status())?;
    let handle = root
        .open(KERNEL_PATH, FileMode::Read, FileAttribute::empty())
        .map_err(|err| err.status())?;

    let mut file = match handle.into_type().map_err(|err| err.status())? {
        FileType::Regular(file) => file,
        _ => return Err(Status::LOAD_ERROR),
    };

    read_regular_file(&mut file)
}

fn read_regular_file(file: &mut RegularFile) -> Result<Vec<u8>, Status> {
    let mut info_buffer = [0u8; 512];
    let info = file
        .get_info::<FileInfo>(&mut info_buffer)
        .map_err(|err| err.status())?;
    let mut data = vec![0u8; info.file_size() as usize];
    let read = file.read(&mut data).map_err(|err| err.status())?;
    data.truncate(read);
    Ok(data)
}

fn init_framebuffer() -> Result<FramebufferInfo, Status> {
    uefi::println!("  framebuffer: locating GOP handle...");
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()
        .map_err(|err| err.status())?;
    uefi::println!("  framebuffer: GOP handle located.");

    uefi::println!("  framebuffer: opening GOP protocol...");
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)
        .map_err(|err| err.status())?;
    uefi::println!("  framebuffer: GOP protocol opened.");

    uefi::println!("  framebuffer: scanning GOP modes...");
    let mode = select_preferred_mode(&gop).ok_or(Status::NOT_FOUND)?;
    let mode_info = *mode.info();
    let (width, height) = mode_info.resolution();
    uefi::println!(
        "  framebuffer: selected mode {}x{} stride {}.",
        width,
        height,
        mode_info.stride()
    );

    uefi::println!("  framebuffer: setting GOP mode...");
    gop.set_mode(&mode).map_err(|err| err.status())?;
    uefi::println!("  framebuffer: GOP mode set.");

    uefi::println!("  framebuffer: acquiring framebuffer view...");
    let mut fb = gop.frame_buffer();
    uefi::println!("  framebuffer: framebuffer view ready.");

    Ok(FramebufferInfo {
        base: fb.as_mut_ptr() as u64,
        size: fb.size() as u64,
        width: width as u32,
        height: height as u32,
        stride: mode_info.stride() as u32,
        format: map_pixel_format(mode_info.pixel_format()),
    })
}

fn select_preferred_mode(gop: &GraphicsOutput) -> Option<Mode> {
    let mut best_mode = None;
    let mut best_area = 0usize;

    for mode in gop.modes() {
        let info = mode.info();
        let pixel_format = info.pixel_format();
        if pixel_format == UefiPixelFormat::BltOnly {
            continue;
        }

        let (width, height) = info.resolution();
        let area = width.saturating_mul(height);
        if area >= best_area {
            best_area = area;
            best_mode = Some(mode);
        }
    }

    best_mode
}

fn map_pixel_format(format: GopPixelFormat) -> PixelFormat {
    match format {
        GopPixelFormat::Rgb => PixelFormat::Rgb,
        GopPixelFormat::Bgr => PixelFormat::Bgr,
        GopPixelFormat::Bitmask => PixelFormat::Bitmask,
        _ => PixelFormat::Unknown,
    }
}

fn load_kernel_elf(image: &[u8]) -> Result<LoadedKernel, Status> {
    let elf = ElfFile::new(image).map_err(|_| Status::LOAD_ERROR)?;

    let mut kernel_start = u64::MAX;
    let mut kernel_end = 0u64;

    for header in elf.program_iter() {
        if header.get_type().map_err(|_| Status::LOAD_ERROR)? != Type::Load {
            continue;
        }

        load_segment(image, &header)?;

        kernel_start = kernel_start.min(header.physical_addr());
        kernel_end = kernel_end.max(header.physical_addr() + header.mem_size());
    }

    if kernel_start == u64::MAX || kernel_end == 0 {
        return Err(Status::LOAD_ERROR);
    }

    Ok(LoadedKernel {
        entry: elf.header.pt2.entry_point(),
        start: kernel_start,
        end: kernel_end,
    })
}

fn load_segment(image: &[u8], header: &ProgramHeader<'_>) -> Result<(), Status> {
    let target = header.physical_addr() as usize;
    let mem_size = header.mem_size() as usize;
    let file_size = header.file_size() as usize;
    let offset = header.offset() as usize;

    if mem_size == 0 {
        return Ok(());
    }

    if file_size > mem_size {
        uefi::println!("ELF segment has file_size larger than mem_size.");
        return Err(Status::LOAD_ERROR);
    }

    let file_end = offset.checked_add(file_size).ok_or(Status::LOAD_ERROR)?;
    if file_end > image.len() {
        uefi::println!("ELF segment exceeds kernel image bounds.");
        return Err(Status::LOAD_ERROR);
    }

    let pages = ((mem_size + 0xfff) / 0x1000) as usize;
    let memory_type = if header.flags().is_execute() {
        MemoryType::LOADER_CODE
    } else {
        MemoryType::LOADER_DATA
    };

    unsafe {
        boot::allocate_pages(
            AllocateType::Address(target as u64),
            memory_type,
            pages,
        )
        .map_err(|err| err.status())?;

        ptr::copy_nonoverlapping(image.as_ptr().add(offset), target as *mut u8, file_size);
        if mem_size > file_size {
            ptr::write_bytes((target + file_size) as *mut u8, 0, mem_size - file_size);
        }
    }

    Ok(())
}

fn map_memory_kind(kind: MemoryType) -> MemoryRegionKind {
    match kind {
        MemoryType::CONVENTIONAL => MemoryRegionKind::Usable,
        MemoryType::BOOT_SERVICES_CODE
        | MemoryType::BOOT_SERVICES_DATA
        | MemoryType::LOADER_CODE
        | MemoryType::LOADER_DATA => MemoryRegionKind::Bootloader,
        MemoryType::ACPI_RECLAIM => MemoryRegionKind::AcpiReclaim,
        MemoryType::ACPI_NON_VOLATILE => MemoryRegionKind::AcpiNvs,
        MemoryType::MMIO | MemoryType::MMIO_PORT_SPACE => MemoryRegionKind::Mmio,
        MemoryType::UNUSABLE => MemoryRegionKind::BadMemory,
        _ => MemoryRegionKind::Reserved,
    }
}

fn find_rsdp() -> u64 {
    uefi::system::with_config_table(|config_table| {
        for entry in config_table {
            if entry.guid == ACPI2_GUID || entry.guid == ACPI_GUID {
                return entry.address as u64;
            }
        }

        0
    })
}

struct LoadedKernel {
    entry: u64,
    start: u64,
    end: u64,
}

fn paint_debug_marker(framebuffer: FramebufferInfo, color: u32) {
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
