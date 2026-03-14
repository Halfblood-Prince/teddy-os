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
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat as GopPixelFormat};
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
        serial_write_str("UEFI helper init failed.\n");
        return Status::LOAD_ERROR;
    }

    serial_init();
    uefi::println!("Teddy-OS bootloader");
    serial_write_str("Teddy-OS bootloader\n");

    match boot() {
        Ok(()) => Status::SUCCESS,
        Err(status) => {
            uefi::println!("Boot failed: {:?}", status);
            serial_write_str("Boot failed.\n");
            status
        }
    }
}

fn boot() -> Result<(), Status> {
    let image_handle = boot::image_handle();

    let kernel_file = read_kernel_file(image_handle)?;
    serial_write_str("Kernel image loaded from EFI partition.\n");

    let framebuffer = init_framebuffer()?;
    uefi::println!(
        "Framebuffer: {}x{} stride {}",
        framebuffer.width,
        framebuffer.height,
        framebuffer.stride
    );
    serial_write_str("Framebuffer initialized.\n");

    let loaded_kernel = load_kernel_elf(&kernel_file)?;
    uefi::println!(
        "Kernel ELF loaded: start={:#x}, end={:#x}, entry={:#x}",
        loaded_kernel.start,
        loaded_kernel.end,
        loaded_kernel.entry
    );
    serial_write_str("Kernel ELF segments placed in memory.\n");

    let rsdp_addr = find_rsdp();
    let mut boot_info = Box::new(BootInfo::new());
    let mut memory_regions = Box::new([MemoryRegion::EMPTY; MAX_MEMORY_REGIONS]);

    boot_info.framebuffer = framebuffer;
    boot_info.rsdp_addr = rsdp_addr;
    boot_info.kernel_start = loaded_kernel.start;
    boot_info.kernel_end = loaded_kernel.end;

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

    let entry: extern "sysv64" fn(&'static BootInfo) -> ! =
        unsafe { mem::transmute(loaded_kernel.entry as usize) };

    serial_write_str("Jumping to kernel entry point.\n");
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
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()
        .map_err(|err| err.status())?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)
        .map_err(|err| err.status())?;
    let mode = gop.current_mode_info();
    let mut fb = gop.frame_buffer();

    Ok(FramebufferInfo {
        base: fb.as_mut_ptr() as u64,
        size: fb.size() as u64,
        width: mode.resolution().0 as u32,
        height: mode.resolution().1 as u32,
        stride: mode.stride() as u32,
        format: map_pixel_format(mode.pixel_format()),
    })
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
        serial_write_str("ELF segment has file_size larger than mem_size.\n");
        return Err(Status::LOAD_ERROR);
    }

    let file_end = offset.checked_add(file_size).ok_or(Status::LOAD_ERROR)?;
    if file_end > image.len() {
        serial_write_str("ELF segment exceeds kernel image bounds.\n");
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

const COM1: u16 = 0x3F8;
const SERIAL_READY_MASK: u8 = 0x20;
const SERIAL_WAIT_SPINS: usize = 100_000;

fn serial_init() {
    unsafe {
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x80);
        outb(COM1 + 0, 0x03);
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x03);
        outb(COM1 + 2, 0xC7);
        outb(COM1 + 4, 0x0B);
    }
}

fn serial_write_str(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            serial_write_byte(b'\r');
        }
        serial_write_byte(byte);
    }
}

fn serial_write_byte(byte: u8) {
    unsafe {
        if !serial_wait_for_transmit_ready() {
            return;
        }
        outb(COM1, byte);
    }
}

unsafe fn serial_wait_for_transmit_ready() -> bool {
    for _ in 0..SERIAL_WAIT_SPINS {
        if (inb(COM1 + 5) & SERIAL_READY_MASK) != 0 {
            return true;
        }
    }
    false
}

unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nomem, nostack, preserves_flags)
    );
    value
}
