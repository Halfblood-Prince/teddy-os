#[repr(C)]
pub struct BootInfo {
    pub signature: [u8; 8],
    pub version: u8,
    pub boot_drive: u8,
    pub kernel_segment: u16,
    pub kernel_sectors: u16,
    pub stage2_sectors: u16,
}

impl BootInfo {
    pub fn from_addr(addr: usize) -> Option<&'static Self> {
        let info = unsafe { &*(addr as *const Self) };
        if info.signature == *b"TEDDYOS\0" && info.version == 1 {
            Some(info)
        } else {
            None
        }
    }
}
