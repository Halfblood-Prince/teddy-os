use crate::storage;

const BOOT_CONFIG_LBA: u32 = 8;
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

    const fn code(self) -> u8 {
        match self {
            Self::Mode640 => 1,
            Self::Mode800 => 2,
            Self::Mode1024 => 3,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Mode640),
            2 => Some(Self::Mode800),
            3 => Some(Self::Mode1024),
            _ => None,
        }
    }
}

pub fn load_boot_display_mode() -> BootDisplayMode {
    let mut sector = [0u8; 512];
    if !storage::detect_primary_master() {
        return BootDisplayMode::Mode1024;
    }
    if !storage::read_sector(BOOT_CONFIG_LBA, &mut sector) {
        return BootDisplayMode::Mode1024;
    }
    if sector[..BOOT_CONFIG_SIGNATURE.len()] != BOOT_CONFIG_SIGNATURE {
        return BootDisplayMode::Mode1024;
    }
    if sector[8] != BOOT_CONFIG_VERSION {
        return BootDisplayMode::Mode1024;
    }
    BootDisplayMode::from_code(sector[9]).unwrap_or(BootDisplayMode::Mode1024)
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
