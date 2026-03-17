const BOOT_INFO_SIGNATURE: [u8; 8] = *b"TEDDYOS\0";

#[repr(C, packed)]
struct RawBootInfo {
    signature: [u8; 8],
    version: u8,
    boot_drive: u8,
    kernel_segment: u16,
    kernel_sectors: u16,
    stage2_sectors: u16,
}

#[derive(Clone, Copy)]
pub struct BootInfo {
    version: u8,
    boot_drive: u8,
    kernel_segment: u16,
    kernel_sectors: u16,
    stage2_sectors: u16,
}

impl BootInfo {
    pub fn parse(addr: usize) -> Option<Self> {
        let raw = unsafe { &*(addr as *const RawBootInfo) };
        if raw.signature != BOOT_INFO_SIGNATURE {
            return None;
        }

        Some(Self {
            version: raw.version,
            boot_drive: raw.boot_drive,
            kernel_segment: raw.kernel_segment,
            kernel_sectors: raw.kernel_sectors,
            stage2_sectors: raw.stage2_sectors,
        })
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn boot_drive(&self) -> u8 {
        self.boot_drive
    }

    pub fn kernel_segment(&self) -> u16 {
        self.kernel_segment
    }

    pub fn kernel_sectors(&self) -> u16 {
        self.kernel_sectors
    }

    pub fn stage2_sectors(&self) -> u16 {
        self.stage2_sectors
    }
}
