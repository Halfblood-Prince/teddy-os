#![no_std]

use core::slice;

pub const BOOTINFO_MAGIC: u64 = 0x5445_4444_5942_4F4F;
pub const MAX_MEMORY_REGIONS: usize = 256;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb = 0,
    Bgr = 1,
    Bitmask = 2,
    Unknown = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FramebufferInfo {
    pub base: u64,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
}

impl FramebufferInfo {
    pub const fn empty() -> Self {
        Self {
            base: 0,
            size: 0,
            width: 0,
            height: 0,
            stride: 0,
            format: PixelFormat::Unknown,
        }
    }

    pub const fn is_valid(&self) -> bool {
        self.base != 0 && self.width != 0 && self.height != 0 && self.stride != 0
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable = 0,
    Reserved = 1,
    AcpiReclaim = 2,
    AcpiNvs = 3,
    Mmio = 4,
    BadMemory = 5,
    Bootloader = 6,
    Kernel = 7,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryRegion {
    pub start: u64,
    pub len: u64,
    pub kind: MemoryRegionKind,
}

impl MemoryRegion {
    pub const EMPTY: Self = Self {
        start: 0,
        len: 0,
        kind: MemoryRegionKind::Reserved,
    };
}

#[repr(C)]
#[derive(Debug)]
pub struct BootInfo {
    pub magic: u64,
    pub framebuffer: FramebufferInfo,
    pub memory_regions_ptr: u64,
    pub memory_regions_len: u64,
    pub rsdp_addr: u64,
    pub kernel_start: u64,
    pub kernel_end: u64,
}

impl BootInfo {
    pub const fn new() -> Self {
        Self {
            magic: BOOTINFO_MAGIC,
            framebuffer: FramebufferInfo::empty(),
            memory_regions_ptr: 0,
            memory_regions_len: 0,
            rsdp_addr: 0,
            kernel_start: 0,
            kernel_end: 0,
        }
    }

    pub fn memory_regions(&self) -> &[MemoryRegion] {
        if self.memory_regions_ptr == 0 || self.memory_regions_len == 0 {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(
                    self.memory_regions_ptr as *const MemoryRegion,
                    self.memory_regions_len as usize,
                )
            }
        }
    }
}

