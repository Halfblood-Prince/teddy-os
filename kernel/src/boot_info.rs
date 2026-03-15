use core::ptr;

#[derive(Clone, Copy)]
pub struct BootInfo {
    pub signature: [u8; 8],
    pub version: u8,
    pub boot_drive: u8,
    pub kernel_segment: u16,
    pub kernel_sectors: u16,
    pub stage2_sectors: u16,
}

impl BootInfo {
    pub fn from_addr(addr: usize) -> Option<Self> {
        let base = addr as *const u8;
        let mut signature = [0u8; 8];
        let mut index = 0usize;
        while index < signature.len() {
            signature[index] = unsafe { ptr::read_volatile(base.add(index)) };
            index += 1;
        }

        if signature != *b"TEDDYOS\0" {
            return None;
        }

        let version = unsafe { ptr::read_volatile(base.add(8)) };
        if version != 1 {
            return None;
        }

        Some(Self {
            signature,
            version,
            boot_drive: unsafe { ptr::read_volatile(base.add(9)) },
            kernel_segment: read_u16(base, 10),
            kernel_sectors: read_u16(base, 12),
            stage2_sectors: read_u16(base, 14),
        })
    }
}

fn read_u16(base: *const u8, offset: usize) -> u16 {
    let low = unsafe { ptr::read_volatile(base.add(offset)) } as u16;
    let high = unsafe { ptr::read_volatile(base.add(offset + 1)) } as u16;
    low | (high << 8)
}
