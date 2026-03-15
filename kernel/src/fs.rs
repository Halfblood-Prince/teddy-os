use spin::Mutex;
use core::fmt;

use crate::storage;

pub const LINE_CAPACITY: usize = 96;
pub const MAX_OUTPUT_LINES: usize = 24;
pub const MAX_DIR_ENTRIES: usize = 24;

const MAGIC: &[u8; 8] = b"TEDDYFS1";
const SUPERBLOCK_LBA: u32 = 0;
const ENTRY_SECTORS: u32 = 8;
const ENTRY_COUNT: usize = 64;
const ENTRY_SIZE: usize = 64;
const DATA_START_LBA: u32 = 1 + ENTRY_SECTORS;
const FILE_SECTORS: u32 = 8;
const MAX_NAME: usize = 24;
const MAX_FILE_BYTES: usize = (FILE_SECTORS as usize) * 512;
const STORAGE_BOOT_ENABLED: bool = false;

#[derive(Clone, Copy)]
pub struct FsTextBuffer {
    bytes: [u8; LINE_CAPACITY],
    len: usize,
}

impl FsTextBuffer {
    pub const fn new() -> Self {
        Self {
            bytes: [0; LINE_CAPACITY],
            len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn push_str(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let write_len = bytes.len().min(self.bytes.len().saturating_sub(self.len));
        self.bytes[self.len..self.len + write_len].copy_from_slice(&bytes[..write_len]);
        self.len += write_len;
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("?")
    }
}

impl fmt::Write for FsTextBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum EntryKind {
    Directory = 1,
    File = 2,
}

#[derive(Clone, Copy)]
struct FsEntry {
    used: bool,
    kind: EntryKind,
    parent: u16,
    name: [u8; MAX_NAME],
    name_len: usize,
    size: usize,
    created_tick: u64,
    modified_tick: u64,
}

impl FsEntry {
    const fn empty() -> Self {
        Self {
            used: false,
            kind: EntryKind::Directory,
            parent: 0,
            name: [0; MAX_NAME],
            name_len: 0,
            size: 0,
            created_tick: 0,
            modified_tick: 0,
        }
    }

    fn set_name(&mut self, name: &str) {
        self.name_len = 0;
        let bytes = name.as_bytes();
        let write_len = bytes.len().min(MAX_NAME);
        self.name[..write_len].copy_from_slice(&bytes[..write_len]);
        self.name_len = write_len;
    }

    fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("?")
    }
}

#[derive(Clone, Copy)]
pub struct Metadata {
    pub is_dir: bool,
    pub size: usize,
    pub created_tick: u64,
    pub modified_tick: u64,
}

#[derive(Clone, Copy)]
pub struct DirectoryEntry {
    pub name: FsTextBuffer,
    pub is_dir: bool,
    pub size: usize,
}

#[derive(Clone, Copy)]
pub struct MountStatus {
    pub mounted: bool,
    pub formatted: bool,
    pub persistent: bool,
}

#[derive(Clone, Copy)]
pub struct FsStats {
    pub mounted: bool,
    pub total_entries: usize,
    pub used_entries: usize,
    pub free_entries: usize,
    pub file_count: usize,
    pub directory_count: usize,
    pub bytes_used: usize,
    pub capacity_bytes: usize,
}

#[derive(Clone, Copy)]
pub struct FsCheckReport {
    pub mounted: bool,
    pub ok: bool,
    pub errors_found: usize,
    pub checked_entries: usize,
}

struct TeddyFs {
    entries: [FsEntry; ENTRY_COUNT],
    data: [[u8; MAX_FILE_BYTES]; ENTRY_COUNT],
    cwd: usize,
    mounted: bool,
    persistent: bool,
}

impl TeddyFs {
    const fn new() -> Self {
        Self {
            entries: [FsEntry::empty(); ENTRY_COUNT],
            data: [[0; MAX_FILE_BYTES]; ENTRY_COUNT],
            cwd: 0,
            mounted: false,
            persistent: false,
        }
    }

    fn reset_in_place(&mut self) {
        self.entries.fill(FsEntry::empty());
        self.data.fill([0; MAX_FILE_BYTES]);
        self.cwd = 0;
        self.mounted = false;
        self.persistent = false;
    }

    fn format(&mut self) -> Result<(), &'static str> {
        self.reset_in_place();
        self.entries[0].used = true;
        self.entries[0].kind = EntryKind::Directory;
        self.entries[0].parent = 0;
        self.entries[0].set_name("");

        let docs = self.create_entry(0, "docs", EntryKind::Directory, 0)?;
        let data = self.create_entry(0, "data", EntryKind::Directory, 0)?;
        let readme = self.create_entry(0, "readme.txt", EntryKind::File, 0)?;
        let notes = self.create_entry(docs, "notes.txt", EntryKind::File, 0)?;
        let welcome = self.create_entry(data, "welcome.txt", EntryKind::File, 0)?;

        // Seed a tiny starter tree so the terminal has a meaningful mounted volume immediately.
        self.write_bytes(readme, b"Welcome to TeddyFS.\nThis volume persists on the VMware data disk.", 0)?;
        self.write_bytes(notes, b"Phase 5 mounted a simple persistent filesystem.\nTerminal commands now use this volume.", 0)?;
        self.write_bytes(welcome, b"Use echo text > file.txt to write files.", 0)?;
        self.persist_all()?;
        self.mounted = true;
        Ok(())
    }

    fn mount(&mut self) -> Result<bool, &'static str> {
        self.persistent = true;

        let mut sector = [0u8; 512];
        storage::read_sector(SUPERBLOCK_LBA, &mut sector)?;
        if &sector[..8] != MAGIC {
            self.format()?;
            return Ok(true);
        }

        self.entries = [FsEntry::empty(); ENTRY_COUNT];
        for sector_offset in 0..ENTRY_SECTORS {
            storage::read_sector(1 + sector_offset, &mut sector)?;
            for slot in 0..(512 / ENTRY_SIZE) {
                let index = sector_offset as usize * (512 / ENTRY_SIZE) + slot;
                if index >= ENTRY_COUNT {
                    break;
                }
                self.entries[index] = decode_entry(&sector[slot * ENTRY_SIZE..(slot + 1) * ENTRY_SIZE]);
            }
        }
        self.cwd = 0;
        self.mounted = true;
        Ok(false)
    }

    fn mount_ephemeral(&mut self) {
        self.reset_in_place();
        self.entries[0].used = true;
        self.entries[0].kind = EntryKind::Directory;
        self.entries[0].parent = 0;
        self.entries[0].set_name("");
        self.mounted = true;
        self.persistent = false;

        let _ = self.create_entry(0, "docs", EntryKind::Directory, 0);
        let _ = self.create_entry(0, "data", EntryKind::Directory, 0);
        if let Ok(readme) = self.create_entry(0, "readme.txt", EntryKind::File, 0) {
            let _ = self.write_bytes(
                readme,
                b"Welcome to TeddyFS.\nNo writable VMware disk was detected, so Teddy-OS booted with an in-memory volume.",
                0,
            );
        }
    }

    fn pwd(&self) -> FsTextBuffer {
        path_for(&self.entries, self.cwd)
    }

    fn ls(&self, path: Option<&str>, out: &mut [FsTextBuffer; MAX_OUTPUT_LINES]) -> Result<usize, &'static str> {
        let index = self.resolve(path.unwrap_or("."))?;
        match self.entries[index].kind {
            EntryKind::File => {
                out[0].clear();
                out[0].push_str(self.entries[index].name());
                Ok(1)
            }
            EntryKind::Directory => {
                let mut count = 0usize;
                for entry in self.entries.iter() {
                    if entry.used && entry.parent as usize == index && entry.name_len > 0 && count < out.len() {
                        out[count].clear();
                        out[count].push_str(entry.name());
                        if matches!(entry.kind, EntryKind::Directory) {
                            out[count].push_str("/");
                        }
                        count += 1;
                    }
                }
                if count == 0 {
                    out[0].clear();
                    out[0].push_str("<empty>");
                    Ok(1)
                } else {
                    Ok(count)
                }
            }
        }
    }

    fn cd(&mut self, path: &str) -> Result<(), &'static str> {
        let index = self.resolve(path)?;
        if matches!(self.entries[index].kind, EntryKind::Directory) {
            self.cwd = index;
            Ok(())
        } else {
            Err("cd: not a directory")
        }
    }

    fn cat(&self, path: &str, out: &mut [FsTextBuffer; MAX_OUTPUT_LINES]) -> Result<usize, &'static str> {
        let index = self.resolve(path)?;
        if !matches!(self.entries[index].kind, EntryKind::File) {
            return Err("cat: path is a directory");
        }

        let size = self.entries[index].size.min(MAX_FILE_BYTES);
        let mut bytes = [0u8; MAX_FILE_BYTES];
        self.read_bytes(index, &mut bytes)?;
        let content = core::str::from_utf8(&bytes[..size]).unwrap_or("?");
        let mut count = 0usize;
        for segment in content.split('\n') {
            if count >= out.len() {
                break;
            }
            out[count].clear();
            out[count].push_str(segment);
            count += 1;
        }
        Ok(count.max(1))
    }

    fn mkdir(&mut self, path: &str, tick: u64) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent(path)?;
        if self.find_child(parent, name).is_some() {
            return Err("mkdir: entry already exists");
        }
        self.create_entry(parent, name, EntryKind::Directory, tick)?;
        self.persist_all()
    }

    fn touch(&mut self, path: &str, tick: u64) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent(path)?;
        if let Some(index) = self.find_child(parent, name) {
            if matches!(self.entries[index].kind, EntryKind::Directory) {
                return Err("touch: path is a directory");
            }
            self.entries[index].modified_tick = tick;
            return self.persist_entry(index);
        }
        self.create_entry(parent, name, EntryKind::File, tick)?;
        self.persist_all()
    }

    fn rm(&mut self, path: &str) -> Result<(), &'static str> {
        let index = self.resolve(path)?;
        if index == 0 {
            return Err("rm: refusing to remove root");
        }
        if matches!(self.entries[index].kind, EntryKind::Directory) {
            for entry in self.entries.iter() {
                if entry.used && entry.parent as usize == index {
                    return Err("rm: directory not empty");
                }
            }
        }

        self.entries[index] = FsEntry::empty();
        self.clear_data(index)?;
        self.persist_all()
    }

    fn write_text(&mut self, path: &str, text: &str, tick: u64) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent(path)?;
        let index = if let Some(index) = self.find_child(parent, name) {
            if matches!(self.entries[index].kind, EntryKind::Directory) {
                return Err("write: path is a directory");
            }
            index
        } else {
            self.create_entry(parent, name, EntryKind::File, tick)?
        };

        self.write_bytes(index, text.as_bytes(), tick)?;
        self.persist_entry(index)
    }

    fn resolve(&self, path: &str) -> Result<usize, &'static str> {
        if path.is_empty() || path == "." {
            return Ok(self.cwd);
        }

        let mut current = if path.starts_with('/') { 0 } else { self.cwd };
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            match segment {
                "." => {}
                ".." => current = self.entries[current].parent as usize,
                name => current = self.find_child(current, name).ok_or("path not found")?,
            }
        }
        Ok(current)
    }

    fn resolve_parent<'a>(&self, path: &'a str) -> Result<(usize, &'a str), &'static str> {
        let trimmed = path.trim_end_matches('/');
        if trimmed.is_empty() {
            return Err("invalid path");
        }

        if let Some((parent_path, name)) = trimmed.rsplit_once('/') {
            let parent = if parent_path.is_empty() { 0 } else { self.resolve(parent_path)? };
            if name.is_empty() {
                Err("invalid path")
            } else {
                Ok((parent, name))
            }
        } else {
            Ok((self.cwd, trimmed))
        }
    }

    fn metadata(&self, path: &str) -> Result<Metadata, &'static str> {
        let index = self.resolve(path)?;
        let entry = self.entries[index];
        Ok(Metadata {
            is_dir: matches!(entry.kind, EntryKind::Directory),
            size: entry.size,
            created_tick: entry.created_tick,
            modified_tick: entry.modified_tick,
        })
    }

    fn list_dir_entries(
        &self,
        path: &str,
        out: &mut [DirectoryEntry; MAX_DIR_ENTRIES],
    ) -> Result<usize, &'static str> {
        let index = self.resolve(path)?;
        if !matches!(self.entries[index].kind, EntryKind::Directory) {
            return Err("list: not a directory");
        }

        let mut count = 0usize;
        for entry in self.entries.iter() {
            if entry.used && entry.parent as usize == index && entry.name_len > 0 && count < out.len() {
                let mut name = FsTextBuffer::new();
                name.push_str(entry.name());
                out[count] = DirectoryEntry {
                    name,
                    is_dir: matches!(entry.kind, EntryKind::Directory),
                    size: entry.size,
                };
                count += 1;
            }
        }
        Ok(count)
    }

    fn rename(&mut self, path: &str, new_name: &str, tick: u64) -> Result<(), &'static str> {
        if new_name.is_empty() || new_name.contains('/') {
            return Err("rename: invalid name");
        }
        let index = self.resolve(path)?;
        if index == 0 {
            return Err("rename: refusing to rename root");
        }
        let parent = self.entries[index].parent as usize;
        if self.find_child(parent, new_name).is_some() {
            return Err("rename: target already exists");
        }
        self.entries[index].set_name(new_name);
        self.entries[index].modified_tick = tick;
        self.persist_entry(index)
    }

    fn stats(&self) -> FsStats {
        let mut used_entries = 0usize;
        let mut file_count = 0usize;
        let mut directory_count = 0usize;
        let mut bytes_used = 0usize;

        for entry in self.entries.iter() {
            if !entry.used {
                continue;
            }
            used_entries += 1;
            match entry.kind {
                EntryKind::Directory => directory_count += 1,
                EntryKind::File => {
                    file_count += 1;
                    bytes_used += entry.size.min(MAX_FILE_BYTES);
                }
            }
        }

        FsStats {
            mounted: self.mounted,
            total_entries: ENTRY_COUNT,
            used_entries,
            free_entries: ENTRY_COUNT.saturating_sub(used_entries),
            file_count,
            directory_count,
            bytes_used,
            capacity_bytes: ENTRY_COUNT.saturating_sub(1) * MAX_FILE_BYTES,
        }
    }

    fn check(&self) -> FsCheckReport {
        if !self.mounted {
            return FsCheckReport {
                mounted: false,
                ok: false,
                errors_found: 1,
                checked_entries: 0,
            };
        }

        let mut errors_found = 0usize;
        let mut checked_entries = 0usize;

        if !self.entries[0].used || !matches!(self.entries[0].kind, EntryKind::Directory) {
            errors_found += 1;
        }

        for (index, entry) in self.entries.iter().enumerate() {
            if !entry.used {
                continue;
            }

            checked_entries += 1;

            if entry.name_len > MAX_NAME {
                errors_found += 1;
            }
            if entry.parent as usize >= ENTRY_COUNT {
                errors_found += 1;
            }
            if matches!(entry.kind, EntryKind::File) && entry.size > MAX_FILE_BYTES {
                errors_found += 1;
            }
            if index != 0 && entry.parent as usize == index {
                errors_found += 1;
            }
        }

        FsCheckReport {
            mounted: true,
            ok: errors_found == 0,
            errors_found,
            checked_entries,
        }
    }

    fn find_child(&self, parent: usize, name: &str) -> Option<usize> {
        self.entries.iter().enumerate().find_map(|(index, entry)| {
            if entry.used && entry.parent as usize == parent && entry.name() == name {
                Some(index)
            } else {
                None
            }
        })
    }

    fn create_entry(
        &mut self,
        parent: usize,
        name: &str,
        kind: EntryKind,
        tick: u64,
    ) -> Result<usize, &'static str> {
        let slot = self.entries.iter().position(|entry| !entry.used).ok_or("fs: no free entries")?;
        self.entries[slot].used = true;
        self.entries[slot].kind = kind;
        self.entries[slot].parent = parent as u16;
        self.entries[slot].set_name(name);
        self.entries[slot].size = 0;
        self.entries[slot].created_tick = tick;
        self.entries[slot].modified_tick = tick;
        Ok(slot)
    }

    fn read_bytes(&self, index: usize, output: &mut [u8; MAX_FILE_BYTES]) -> Result<(), &'static str> {
        if !self.persistent {
            output.copy_from_slice(&self.data[index]);
            return Ok(());
        }
        let mut sector = [0u8; 512];
        for sector_offset in 0..FILE_SECTORS {
            storage::read_sector(data_lba_for(index, sector_offset), &mut sector)?;
            let dest = sector_offset as usize * 512;
            output[dest..dest + 512].copy_from_slice(&sector);
        }
        Ok(())
    }

    fn write_bytes(&mut self, index: usize, bytes: &[u8], tick: u64) -> Result<(), &'static str> {
        let write_len = bytes.len().min(MAX_FILE_BYTES);
        if self.persistent {
            let mut sector = [0u8; 512];
            for sector_offset in 0..FILE_SECTORS {
                sector.fill(0);
                let start = sector_offset as usize * 512;
                let end = (start + 512).min(write_len);
                if start < end {
                    sector[..end - start].copy_from_slice(&bytes[start..end]);
                }
                storage::write_sector(data_lba_for(index, sector_offset), &sector)?;
            }
        } else {
            self.data[index].fill(0);
            self.data[index][..write_len].copy_from_slice(&bytes[..write_len]);
        }

        self.entries[index].size = write_len;
        self.entries[index].modified_tick = tick;
        Ok(())
    }

    fn clear_data(&mut self, index: usize) -> Result<(), &'static str> {
        if !self.persistent {
            self.data[index].fill(0);
            return Ok(());
        }
        let zero = [0u8; 512];
        for sector_offset in 0..FILE_SECTORS {
            storage::write_sector(data_lba_for(index, sector_offset), &zero)?;
        }
        Ok(())
    }

    fn persist_entry(&self, index: usize) -> Result<(), &'static str> {
        if !self.persistent {
            return Ok(());
        }
        let sector_lba = 1 + (index as u32 / (512 / ENTRY_SIZE) as u32);
        let entry_slot = index % (512 / ENTRY_SIZE);
        let mut sector = [0u8; 512];
        storage::read_sector(sector_lba, &mut sector)?;
        encode_entry(&self.entries[index], &mut sector[entry_slot * ENTRY_SIZE..(entry_slot + 1) * ENTRY_SIZE]);
        storage::write_sector(sector_lba, &sector)
    }

    fn persist_all(&self) -> Result<(), &'static str> {
        if !self.persistent {
            return Ok(());
        }
        let mut sector = [0u8; 512];
        sector[..8].copy_from_slice(MAGIC);
        sector[8..12].copy_from_slice(&(ENTRY_COUNT as u32).to_le_bytes());
        sector[12..16].copy_from_slice(&FILE_SECTORS.to_le_bytes());
        storage::write_sector(SUPERBLOCK_LBA, &sector)?;

        // The entry table is fixed-size on disk to keep mount/format logic straightforward.
        for sector_offset in 0..ENTRY_SECTORS {
            sector.fill(0);
            for slot in 0..(512 / ENTRY_SIZE) {
                let index = sector_offset as usize * (512 / ENTRY_SIZE) + slot;
                if index >= ENTRY_COUNT {
                    break;
                }
                encode_entry(&self.entries[index], &mut sector[slot * ENTRY_SIZE..(slot + 1) * ENTRY_SIZE]);
            }
            storage::write_sector(1 + sector_offset, &sector)?;
        }
        Ok(())
    }
}

static FS: Mutex<TeddyFs> = Mutex::new(TeddyFs::new());

pub fn init() -> MountStatus {
    let mut fs = FS.lock();
    *fs = TeddyFs::new();
    if !STORAGE_BOOT_ENABLED {
        fs.mount_ephemeral();
        return MountStatus {
            mounted: true,
            formatted: false,
            persistent: false,
        };
    }

    match fs.mount() {
        Ok(formatted) => MountStatus {
            mounted: true,
            formatted,
            persistent: true,
        },
        Err(_) => {
            fs.mount_ephemeral();
            MountStatus {
                mounted: true,
                formatted: false,
                persistent: false,
            }
        }
    }
}

pub fn is_ready() -> bool {
    FS.lock().mounted
}

pub fn pwd() -> Result<FsTextBuffer, &'static str> {
    let guard = FS.lock();
    let fs = &*guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    Ok(fs.pwd())
}

pub fn ls(path: Option<&str>, out: &mut [FsTextBuffer; MAX_OUTPUT_LINES]) -> Result<usize, &'static str> {
    let guard = FS.lock();
    let fs = &*guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.ls(path, out)
}

pub fn cd(path: &str) -> Result<(), &'static str> {
    let mut guard = FS.lock();
    let fs = &mut *guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.cd(path)
}

pub fn cat(path: &str, out: &mut [FsTextBuffer; MAX_OUTPUT_LINES]) -> Result<usize, &'static str> {
    let guard = FS.lock();
    let fs = &*guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.cat(path, out)
}

pub fn mkdir(path: &str, tick: u64) -> Result<(), &'static str> {
    let mut guard = FS.lock();
    let fs = &mut *guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.mkdir(path, tick)
}

pub fn touch(path: &str, tick: u64) -> Result<(), &'static str> {
    let mut guard = FS.lock();
    let fs = &mut *guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.touch(path, tick)
}

pub fn rm(path: &str) -> Result<(), &'static str> {
    let mut guard = FS.lock();
    let fs = &mut *guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.rm(path)
}

pub fn write_text(path: &str, text: &str, tick: u64) -> Result<(), &'static str> {
    let mut guard = FS.lock();
    let fs = &mut *guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.write_text(path, text, tick)
}

pub fn metadata(path: &str) -> Result<Metadata, &'static str> {
    let guard = FS.lock();
    let fs = &*guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.metadata(path)
}

pub fn list_dir_entries(
    path: &str,
    out: &mut [DirectoryEntry; MAX_DIR_ENTRIES],
) -> Result<usize, &'static str> {
    let guard = FS.lock();
    let fs = &*guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.list_dir_entries(path, out)
}

pub fn rename(path: &str, new_name: &str, tick: u64) -> Result<(), &'static str> {
    let mut guard = FS.lock();
    let fs = &mut *guard;
    if !fs.mounted {
        return Err("fs: volume not mounted");
    }
    fs.rename(path, new_name, tick)
}

pub fn stats() -> Result<FsStats, &'static str> {
    let guard = FS.lock();
    let fs = &*guard;
    Ok(fs.stats())
}

pub fn check() -> Result<FsCheckReport, &'static str> {
    let guard = FS.lock();
    let fs = &*guard;
    Ok(fs.check())
}

fn path_for(entries: &[FsEntry; ENTRY_COUNT], index: usize) -> FsTextBuffer {
    if index == 0 {
        let mut root = FsTextBuffer::new();
        root.push_str("/");
        return root;
    }

    let mut names = [[0u8; MAX_NAME]; 8];
    let mut lengths = [0usize; 8];
    let mut count = 0usize;
    let mut current = index;
    while current != 0 && count < names.len() {
        let entry = entries[current];
        names[count][..entry.name_len].copy_from_slice(&entry.name[..entry.name_len]);
        lengths[count] = entry.name_len;
        count += 1;
        current = entry.parent as usize;
    }

    let mut path = FsTextBuffer::new();
    path.push_str("/");
    for segment in (0..count).rev() {
        let text = core::str::from_utf8(&names[segment][..lengths[segment]]).unwrap_or("?");
        path.push_str(text);
        if segment != 0 {
            path.push_str("/");
        }
    }
    path
}

fn data_lba_for(index: usize, sector_offset: u32) -> u32 {
    DATA_START_LBA + (index as u32 * FILE_SECTORS) + sector_offset
}

fn encode_entry(entry: &FsEntry, output: &mut [u8]) {
    output.fill(0);
    output[0] = if entry.used { 1 } else { 0 };
    output[1] = entry.kind as u8;
    output[2..4].copy_from_slice(&entry.parent.to_le_bytes());
    output[4] = entry.name_len as u8;
    output[8..12].copy_from_slice(&(entry.size as u32).to_le_bytes());
    output[12..20].copy_from_slice(&entry.created_tick.to_le_bytes());
    output[20..28].copy_from_slice(&entry.modified_tick.to_le_bytes());
    output[28..28 + entry.name_len].copy_from_slice(&entry.name[..entry.name_len]);
}

fn decode_entry(input: &[u8]) -> FsEntry {
    let used = input[0] != 0;
    let kind = if input[1] == EntryKind::File as u8 {
        EntryKind::File
    } else {
        EntryKind::Directory
    };
    let parent = u16::from_le_bytes([input[2], input[3]]);
    let name_len = usize::from(input[4]).min(MAX_NAME);
    let mut name = [0u8; MAX_NAME];
    name[..name_len].copy_from_slice(&input[28..28 + name_len]);
    let size = u32::from_le_bytes([input[8], input[9], input[10], input[11]]) as usize;
    let created_tick = u64::from_le_bytes([
        input[12], input[13], input[14], input[15], input[16], input[17], input[18], input[19],
    ]);
    let modified_tick = u64::from_le_bytes([
        input[20], input[21], input[22], input[23], input[24], input[25], input[26], input[27],
    ]);

    FsEntry {
        used,
        kind,
        parent,
        name,
        name_len,
        size,
        created_tick,
        modified_tick,
    }
}
