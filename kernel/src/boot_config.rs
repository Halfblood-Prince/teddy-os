use crate::storage;

const BOOT_CONFIG_LBA: u32 = 400;
const BOOT_CONFIG_SIGNATURE: [u8; 8] = *b"TDBOOT1\0";
const BOOT_CONFIG_VERSION: u8 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BootDisplayMode {
    Mode640,
    Mode800,
    Mode1024,
}

impl BootDisplayMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mode640 => "640 x 480",
            Self::Mode800 => "800 x 600",
            Self::Mode1024 => "1024 x 768",
        }
    }

    pub const fn from_dimensions(width: u16, height: u16) -> Self {
        match (width, height) {
            (640, 480) => Self::Mode640,
            (800, 600) => Self::Mode800,
            (1024, 768) => Self::Mode1024,
            _ => Self::Mode1024,
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Mode640 => 1,
            Self::Mode800 => 2,
            Self::Mode1024 => 3,
        }
    }
}

pub fn save_boot_display_mode(mode: BootDisplayMode) -> bool {
    let mut sector = [0u8; 512];
    if !storage::detect_primary_master() {
        return false;
    }
    let mut index = 0usize;
    while index < BOOT_CONFIG_SIGNATURE.len() {
        sector[index] = BOOT_CONFIG_SIGNATURE[index];
        index += 1;
    }
    sector[8] = BOOT_CONFIG_VERSION;
    sector[9] = mode.code();
    storage::write_sector(BOOT_CONFIG_LBA, &sector)
}
