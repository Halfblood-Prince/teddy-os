use spin::Mutex;
use x86_64::instructions::port::Port;

const PRIMARY_IO: u16 = 0x1F0;
const PRIMARY_CTRL: u16 = 0x3F6;
const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_READ_SECTORS: u8 = 0x20;
const ATA_CMD_WRITE_SECTORS: u8 = 0x30;
const ATA_CMD_CACHE_FLUSH: u8 = 0xE7;
const STATUS_BSY: u8 = 0x80;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;

#[derive(Clone, Copy)]
pub enum DriveSelect {
    Master,
    Slave,
}

#[derive(Clone, Copy)]
pub struct StorageInfo {
    pub present: bool,
    pub drive: DriveSelect,
}

#[derive(Clone, Copy)]
struct AtaDrive {
    io_base: u16,
    ctrl_base: u16,
    drive: DriveSelect,
}

static DRIVE: Mutex<Option<AtaDrive>> = Mutex::new(None);

pub fn init() -> StorageInfo {
    let drive = detect_drive(DriveSelect::Master).or_else(|| detect_drive(DriveSelect::Slave));
    *DRIVE.lock() = drive;

    if let Some(drive) = drive {
        StorageInfo {
            present: true,
            drive: drive.drive,
        }
    } else {
        StorageInfo {
            present: false,
            drive: DriveSelect::Master,
        }
    }
}

pub fn is_ready() -> bool {
    DRIVE.lock().is_some()
}

pub fn read_sector(lba: u32, buffer: &mut [u8; 512]) -> Result<(), &'static str> {
    let Some(drive) = *DRIVE.lock() else {
        return Err("storage: no ATA drive");
    };

    unsafe {
        // Phase 5 uses simple 28-bit PIO reads to keep VMware disk support minimal.
        setup_lba28(drive, lba, 1);
        Port::<u8>::new(drive.io_base + 7).write(ATA_CMD_READ_SECTORS);
        poll_for_data(drive)?;

        let mut data = Port::<u16>::new(drive.io_base);
        for index in 0..256 {
            let word = data.read();
            buffer[index * 2] = (word & 0x00FF) as u8;
            buffer[index * 2 + 1] = (word >> 8) as u8;
        }
    }

    Ok(())
}

pub fn write_sector(lba: u32, buffer: &[u8; 512]) -> Result<(), &'static str> {
    let Some(drive) = *DRIVE.lock() else {
        return Err("storage: no ATA drive");
    };

    unsafe {
        // Flush each sector write so filesystem updates survive guest resets.
        setup_lba28(drive, lba, 1);
        Port::<u8>::new(drive.io_base + 7).write(ATA_CMD_WRITE_SECTORS);
        poll_for_data(drive)?;

        let mut data = Port::<u16>::new(drive.io_base);
        for index in 0..256 {
            let lo = buffer[index * 2] as u16;
            let hi = (buffer[index * 2 + 1] as u16) << 8;
            data.write(lo | hi);
        }

        Port::<u8>::new(drive.io_base + 7).write(ATA_CMD_CACHE_FLUSH);
        poll_not_busy(drive)?;
    }

    Ok(())
}

fn detect_drive(select: DriveSelect) -> Option<AtaDrive> {
    let drive = AtaDrive {
        io_base: PRIMARY_IO,
        ctrl_base: PRIMARY_CTRL,
        drive: select,
    };

    unsafe {
        select_drive(drive);
        Port::<u8>::new(drive.io_base + 2).write(0);
        Port::<u8>::new(drive.io_base + 3).write(0);
        Port::<u8>::new(drive.io_base + 4).write(0);
        Port::<u8>::new(drive.io_base + 5).write(0);
        Port::<u8>::new(drive.io_base + 7).write(ATA_CMD_IDENTIFY);

        let status = Port::<u8>::new(drive.io_base + 7).read();
        if status == 0 {
            return None;
        }

        if poll_not_busy(drive).is_err() {
            return None;
        }

        let lba_mid = Port::<u8>::new(drive.io_base + 4).read();
        let lba_hi = Port::<u8>::new(drive.io_base + 5).read();
        if lba_mid != 0 || lba_hi != 0 {
            return None;
        }

        if poll_for_data(drive).is_err() {
            return None;
        }

        let mut data = Port::<u16>::new(drive.io_base);
        for _ in 0..256 {
            let _ = data.read();
        }
    }

    Some(drive)
}

unsafe fn setup_lba28(drive: AtaDrive, lba: u32, sectors: u8) {
    select_drive_with_lba(drive, lba);
    Port::<u8>::new(drive.io_base + 1).write(0);
    Port::<u8>::new(drive.io_base + 2).write(sectors);
    Port::<u8>::new(drive.io_base + 3).write((lba & 0xFF) as u8);
    Port::<u8>::new(drive.io_base + 4).write(((lba >> 8) & 0xFF) as u8);
    Port::<u8>::new(drive.io_base + 5).write(((lba >> 16) & 0xFF) as u8);
}

unsafe fn select_drive(drive: AtaDrive) {
    let selector = match drive.drive {
        DriveSelect::Master => 0xA0,
        DriveSelect::Slave => 0xB0,
    };
    Port::<u8>::new(drive.io_base + 6).write(selector);
    io_wait(drive);
}

unsafe fn select_drive_with_lba(drive: AtaDrive, lba: u32) {
    let base = match drive.drive {
        DriveSelect::Master => 0xE0,
        DriveSelect::Slave => 0xF0,
    };
    Port::<u8>::new(drive.io_base + 6).write(base | (((lba >> 24) & 0x0F) as u8));
    io_wait(drive);
}

fn poll_not_busy(drive: AtaDrive) -> Result<(), &'static str> {
    unsafe {
        let mut status_port = Port::<u8>::new(drive.io_base + 7);
        for _ in 0..100_000 {
            let status = status_port.read();
            if status & STATUS_BSY == 0 {
                return Ok(());
            }
        }
    }
    Err("storage: timed out waiting for BSY")
}

fn poll_for_data(drive: AtaDrive) -> Result<(), &'static str> {
    unsafe {
        let mut status_port = Port::<u8>::new(drive.io_base + 7);
        for _ in 0..100_000 {
            let status = status_port.read();
            if status & STATUS_ERR != 0 {
                return Err("storage: ATA error");
            }
            if status & STATUS_BSY == 0 && status & STATUS_DRQ != 0 {
                return Ok(());
            }
        }
    }
    Err("storage: timed out waiting for DRQ")
}

unsafe fn io_wait(drive: AtaDrive) {
    let mut control = Port::<u8>::new(drive.ctrl_base);
    for _ in 0..4 {
        let _ = control.read();
    }
}
