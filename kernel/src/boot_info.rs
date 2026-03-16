use crate::vga;

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

    pub fn render(&self) {
        vga::write_line(14, 48, "Boot info", 0x1E);
        vga::write_line(15, 48, "Ver:", 0x1F);
        vga::write_hex_byte(15, 53, "", self.version, 0x1F);
        vga::write_line(15, 58, "Drv:", 0x1F);
        vga::write_hex_byte(15, 63, "", self.boot_drive, 0x1F);

        vga::write_line(16, 48, "KSeg:", 0x17);
        vga::write_hex_word(16, 54, "", self.kernel_segment, 0x17);
        vga::write_line(16, 60, "KSec:", 0x17);
        vga::write_hex_word(16, 66, "", self.kernel_sectors, 0x17);

        vga::write_line(17, 48, "S2Sec:", 0x1A);
        vga::write_hex_word(17, 55, "", self.stage2_sectors, 0x1A);
    }
}
