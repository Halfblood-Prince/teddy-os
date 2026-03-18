use crate::port;

const ATA_DATA: u16 = 0x1F0;
const ATA_SECTOR_COUNT: u16 = 0x1F2;
const ATA_LBA_LOW: u16 = 0x1F3;
const ATA_LBA_MID: u16 = 0x1F4;
const ATA_LBA_HIGH: u16 = 0x1F5;
const ATA_DRIVE_HEAD: u16 = 0x1F6;
const ATA_STATUS: u16 = 0x1F7;
const ATA_COMMAND: u16 = 0x1F7;

const STATUS_ERR: u8 = 0x01;
const STATUS_DRQ: u8 = 0x08;
const STATUS_DF: u8 = 0x20;
const STATUS_BSY: u8 = 0x80;

const COMMAND_READ_SECTORS: u8 = 0x20;
const COMMAND_WRITE_SECTORS: u8 = 0x30;
const COMMAND_CACHE_FLUSH: u8 = 0xE7;
const COMMAND_IDENTIFY: u8 = 0xEC;

const POLL_LIMIT: usize = 100_000;
const DRIVE_MASTER_LBA: u8 = 0xE0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PersistenceState {
    Unknown,
    Ready,
    Seeded,
    NoDisk,
    ReadError,
    WriteError,
    InvalidFormat,
}

impl PersistenceState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Ready => "disk loaded",
            Self::Seeded => "disk seeded",
            Self::NoDisk => "no disk",
            Self::ReadError => "read error",
            Self::WriteError => "write error",
            Self::InvalidFormat => "invalid format",
        }
    }
}

pub fn detect_primary_master() -> bool {
    select_drive(0);
    port::outb(ATA_SECTOR_COUNT, 0);
    port::outb(ATA_LBA_LOW, 0);
    port::outb(ATA_LBA_MID, 0);
    port::outb(ATA_LBA_HIGH, 0);
    port::outb(ATA_COMMAND, COMMAND_IDENTIFY);

    let status = port::inb(ATA_STATUS);
    if status == 0 {
        return false;
    }

    let signature_mid = port::inb(ATA_LBA_MID);
    let signature_high = port::inb(ATA_LBA_HIGH);
    if is_atapi_signature(signature_mid, signature_high) {
        return false;
    }

    let mut spins = 0usize;
    while spins < POLL_LIMIT {
        let current = port::inb(ATA_STATUS);
        if current & STATUS_BSY == 0 {
            if current & STATUS_ERR != 0 {
                return false;
            }
            if current & STATUS_DRQ != 0 {
                let mut word_index = 0usize;
                while word_index < 256 {
                    let _ = port::inw(ATA_DATA);
                    word_index += 1;
                }
                return true;
            }
        }
        spins += 1;
    }

    false
}

pub fn read_sector(lba: u32, buffer: &mut [u8; 512]) -> bool {
    if !prepare_lba(lba, COMMAND_READ_SECTORS) {
        return false;
    }

    let mut word_index = 0usize;
    while word_index < 256 {
        let word = port::inw(ATA_DATA);
        buffer[word_index * 2] = (word & 0x00FF) as u8;
        buffer[word_index * 2 + 1] = (word >> 8) as u8;
        word_index += 1;
    }
    true
}

pub fn write_sector(lba: u32, buffer: &[u8; 512]) -> bool {
    if !prepare_lba(lba, COMMAND_WRITE_SECTORS) {
        return false;
    }

    let mut word_index = 0usize;
    while word_index < 256 {
        let low = buffer[word_index * 2] as u16;
        let high = (buffer[word_index * 2 + 1] as u16) << 8;
        port::outw(ATA_DATA, low | high);
        word_index += 1;
    }

    port::outb(ATA_COMMAND, COMMAND_CACHE_FLUSH);
    wait_not_busy()
}

fn prepare_lba(lba: u32, command: u8) -> bool {
    select_drive(((lba >> 24) & 0x0F) as u8);
    port::outb(ATA_SECTOR_COUNT, 1);
    port::outb(ATA_LBA_LOW, (lba & 0xFF) as u8);
    port::outb(ATA_LBA_MID, ((lba >> 8) & 0xFF) as u8);
    port::outb(ATA_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);
    port::outb(ATA_COMMAND, command);
    wait_drq()
}

fn select_drive(high_lba_nibble: u8) {
    port::outb(ATA_DRIVE_HEAD, DRIVE_MASTER_LBA | (high_lba_nibble & 0x0F));
    port::io_wait();
}

fn wait_drq() -> bool {
    let mut spins = 0usize;
    while spins < POLL_LIMIT {
        let status = port::inb(ATA_STATUS);
        if status & STATUS_BSY == 0 {
            if status & (STATUS_ERR | STATUS_DF) != 0 {
                return false;
            }
            if status & STATUS_DRQ != 0 {
                return true;
            }
        }
        spins += 1;
    }
    false
}

fn wait_not_busy() -> bool {
    let mut spins = 0usize;
    while spins < POLL_LIMIT {
        let status = port::inb(ATA_STATUS);
        if status & STATUS_BSY == 0 {
            return status & (STATUS_ERR | STATUS_DF) == 0;
        }
        spins += 1;
    }
    false
}

fn is_atapi_signature(mid: u8, high: u8) -> bool {
    (mid == 0x14 && high == 0xEB) || (mid == 0x69 && high == 0x96)
}
