use spin::Mutex;
use teddy_boot_proto::{BootInfo, MemoryRegion, MemoryRegionKind};

const PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy)]
pub struct MemoryStats {
    pub total_bytes: u64,
    pub usable_bytes: u64,
    pub reserved_bytes: u64,
    pub bootloader_bytes: u64,
    pub kernel_bytes: u64,
}

#[derive(Clone, Copy)]
pub struct FrameAllocation {
    pub start: u64,
    pub frames: usize,
}

pub struct BootFrameAllocator {
    next_free: u64,
    usable_end: u64,
}

impl BootFrameAllocator {
    const fn empty() -> Self {
        Self {
            next_free: 0,
            usable_end: 0,
        }
    }

    fn initialize(&mut self, boot_info: &BootInfo) {
        let mut best_region: Option<MemoryRegion> = None;

        for region in boot_info.memory_regions() {
            if region.kind != MemoryRegionKind::Usable {
                continue;
            }

            if region.start + region.len <= boot_info.kernel_end {
                continue;
            }

            if region.len >= best_region.map(|existing| existing.len).unwrap_or(0) {
                best_region = Some(*region);
            }
        }

        if let Some(region) = best_region {
            let start = align_up(region.start.max(boot_info.kernel_end), PAGE_SIZE);
            self.next_free = start;
            self.usable_end = region.start + region.len;
        } else {
            self.next_free = 0;
            self.usable_end = 0;
        }
    }

    fn allocate_frames(&mut self, frame_count: usize) -> Option<FrameAllocation> {
        if frame_count == 0 || self.next_free == 0 {
            return None;
        }

        let bytes = (frame_count as u64) * PAGE_SIZE;
        let start = align_up(self.next_free, PAGE_SIZE);
        let end = start.checked_add(bytes)?;
        if end > self.usable_end {
            return None;
        }

        self.next_free = end;
        Some(FrameAllocation {
            start,
            frames: frame_count,
        })
    }
}

static FRAME_ALLOCATOR: Mutex<BootFrameAllocator> = Mutex::new(BootFrameAllocator::empty());
static MEMORY_STATS: Mutex<MemoryStats> = Mutex::new(MemoryStats {
    total_bytes: 0,
    usable_bytes: 0,
    reserved_bytes: 0,
    bootloader_bytes: 0,
    kernel_bytes: 0,
});

pub fn init(boot_info: &BootInfo) {
    let mut total_bytes = 0;
    let mut usable_bytes = 0;
    let mut reserved_bytes = 0;
    let mut bootloader_bytes = 0;

    for region in boot_info.memory_regions() {
        total_bytes += region.len;
        match region.kind {
            MemoryRegionKind::Usable => usable_bytes += region.len,
            MemoryRegionKind::Bootloader => bootloader_bytes += region.len,
            _ => reserved_bytes += region.len,
        }
    }

    *MEMORY_STATS.lock() = MemoryStats {
        total_bytes,
        usable_bytes,
        reserved_bytes,
        bootloader_bytes,
        kernel_bytes: boot_info.kernel_end.saturating_sub(boot_info.kernel_start),
    };

    FRAME_ALLOCATOR.lock().initialize(boot_info);
}

pub fn stats() -> MemoryStats {
    *MEMORY_STATS.lock()
}

pub fn allocate_frames(frame_count: usize) -> Option<FrameAllocation> {
    FRAME_ALLOCATOR.lock().allocate_frames(frame_count)
}

const fn align_up(value: u64, alignment: u64) -> u64 {
    let mask = alignment - 1;
    (value + mask) & !mask
}
