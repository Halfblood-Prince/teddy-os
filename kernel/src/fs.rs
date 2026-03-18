use crate::{storage::{self, PersistenceState}, trace};

pub const MAX_FS_NODES: usize = 16;
pub const MAX_NAME_LEN: usize = 12;
pub const MAX_FILE_LEN: usize = 96;
pub const MAX_PATH_LEN: usize = 58;
const FS_SECTOR_SIZE: usize = 512;
const FS_DISK_LBA_START: u32 = 1;
const FS_SIGNATURE: [u8; 8] = *b"TEDDYFS1";
const FS_VERSION: u8 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
}

#[derive(Clone, Copy)]
pub struct NameText {
    bytes: [u8; MAX_NAME_LEN],
    len: usize,
}

impl NameText {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; MAX_NAME_LEN],
            len: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

#[derive(Clone, Copy)]
pub struct PathText {
    bytes: [u8; MAX_PATH_LEN],
    len: usize,
}

impl PathText {
    pub const fn empty() -> Self {
        Self {
            bytes: [b'/'; MAX_PATH_LEN],
            len: 1,
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("/")
    }
}

#[derive(Clone, Copy)]
struct FsNode {
    used: bool,
    kind: EntryKind,
    parent: usize,
    name: [u8; MAX_NAME_LEN],
    name_len: usize,
    data: [u8; MAX_FILE_LEN],
    data_len: usize,
}

impl FsNode {
    const fn empty() -> Self {
        Self {
            used: false,
            kind: EntryKind::File,
            parent: 0,
            name: [0; MAX_NAME_LEN],
            name_len: 0,
            data: [0; MAX_FILE_LEN],
            data_len: 0,
        }
    }

    fn init_dir(&mut self, parent: usize, name: &str) {
        self.used = true;
        self.kind = EntryKind::Dir;
        self.parent = parent;
        self.set_name(name);
        self.data_len = 0;
    }

    fn init_file(&mut self, parent: usize, name: &str, contents: &str) {
        self.used = true;
        self.kind = EntryKind::File;
        self.parent = parent;
        self.set_name(name);
        self.set_data(contents);
    }

    fn set_name(&mut self, name: &str) {
        let mut clear_index = 0usize;
        while clear_index < MAX_NAME_LEN {
            self.name[clear_index] = 0;
            clear_index += 1;
        }
        self.name_len = 0;
        let bytes = name.as_bytes();
        let limit = core::cmp::min(bytes.len(), MAX_NAME_LEN);
        let mut index = 0usize;
        while index < limit {
            self.name[self.name_len] = sanitize(bytes[index]);
            self.name_len += 1;
            index += 1;
        }
    }

    fn set_data(&mut self, contents: &str) {
        let mut clear_index = 0usize;
        while clear_index < MAX_FILE_LEN {
            self.data[clear_index] = 0;
            clear_index += 1;
        }
        self.data_len = 0;
        let bytes = contents.as_bytes();
        let limit = core::cmp::min(bytes.len(), MAX_FILE_LEN);
        let mut index = 0usize;
        while index < limit {
            self.data[self.data_len] = sanitize(bytes[index]);
            self.data_len += 1;
            index += 1;
        }
    }

    fn name_eq(&self, name: &str) -> bool {
        self.name() == name
    }

    fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }
}

pub struct FileSystem {
    nodes: [FsNode; MAX_FS_NODES],
    cwd: usize,
    cwd_path: [u8; MAX_PATH_LEN],
    cwd_path_len: usize,
    persistence: PersistenceState,
}

impl FileSystem {
    pub const fn empty() -> Self {
        Self {
            nodes: [FsNode::empty(); MAX_FS_NODES],
            cwd: 0,
            cwd_path: [b'/'; MAX_PATH_LEN],
            cwd_path_len: 1,
            persistence: PersistenceState::Unknown,
        }
    }

    pub fn init(&mut self) {
        self.init_seed_data();
        trace::set_boot_stage(0x54);
        self.load_or_seed();
    }

    pub fn init_ram_only(&mut self) {
        self.init_seed_data();
        trace::set_boot_stage(0x54);
        self.persistence = PersistenceState::NoDisk;
    }

    fn init_seed_data(&mut self) {
        trace::set_boot_stage(0x50);
        self.cwd = 0;
        let mut clear_index = 0usize;
        while clear_index < MAX_PATH_LEN {
            self.cwd_path[clear_index] = b' ';
            clear_index += 1;
        }
        self.cwd_path_len = 1;
        self.cwd_path[0] = b'/';
        self.persistence = PersistenceState::Unknown;
        trace::set_boot_stage(0x51);
        self.nodes[0].init_dir(0, "");
        self.nodes[1].init_dir(0, "docs");
        self.nodes[2].init_file(0, "readme.txt", "Teddy filesystem layer online.");
        self.nodes[3].init_file(1, "plan.txt", "Next: disk-backed persistence.");
        self.nodes[4].init_file(0, "notes.txt", "Terminal now uses kernel fs APIs.");
        trace::set_boot_stage(0x53);
        self.refresh_cwd_path();
    }

    pub fn cwd_path(&self) -> &str {
        core::str::from_utf8(&self.cwd_path[..self.cwd_path_len]).unwrap_or("/")
    }

    pub fn cwd_text(&self) -> PathText {
        let mut text = PathText::empty();
        let mut index = 0usize;
        while index < self.cwd_path_len {
            text.bytes[index] = self.cwd_path[index];
            index += 1;
        }
        text.len = self.cwd_path_len;
        text
    }

    pub fn change_dir(&mut self, path: &str) -> Result<(), &'static str> {
        let node = self.resolve_dir(path)?;
        self.cwd = node;
        self.refresh_cwd_path();
        Ok(())
    }

    pub fn read_file_into(&self, path: &str, out: &mut [u8; MAX_FILE_LEN]) -> Result<usize, &'static str> {
        let node = self.resolve_path(path)?;
        let entry = &self.nodes[node];
        if entry.kind != EntryKind::File {
            return Err("cat: not a file");
        }
        let mut index = 0usize;
        while index < entry.data_len {
            out[index] = entry.data[index];
            index += 1;
        }
        Ok(entry.data_len)
    }

    pub fn write_file(&mut self, path: &str, bytes: &[u8]) -> Result<(), &'static str> {
        let node = self.resolve_path(path)?;
        let entry = &mut self.nodes[node];
        if entry.kind != EntryKind::File {
            return Err("write: not a file");
        }

        let mut clear_index = 0usize;
        while clear_index < MAX_FILE_LEN {
            entry.data[clear_index] = 0;
            clear_index += 1;
        }

        entry.data_len = 0;
        let limit = core::cmp::min(bytes.len(), MAX_FILE_LEN);
        let mut index = 0usize;
        while index < limit {
            entry.data[index] = sanitize(bytes[index]);
            entry.data_len += 1;
            index += 1;
        }
        self.save_if_possible();
        Ok(())
    }

    pub fn create_dir(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent_and_name(path)?;
        let result = self.create_node(parent, name, EntryKind::Dir);
        if result.is_ok() {
            self.save_if_possible();
        }
        result
    }

    pub fn touch(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent_and_name(path)?;
        if let Some(index) = self.find_child(parent, name) {
            if self.nodes[index].kind != EntryKind::File {
                return Err("touch: path is directory");
            }
            self.nodes[index].set_data("empty file");
            self.save_if_possible();
            return Ok(());
        }
        let result = self.create_node(parent, name, EntryKind::File);
        if result.is_ok() {
            self.save_if_possible();
        }
        result
    }

    pub fn remove(&mut self, path: &str) -> Result<(), &'static str> {
        let index = self.resolve_path(path)?;
        if index == 0 {
            return Err("rm: cannot remove root");
        }
        if self.nodes[index].kind == EntryKind::Dir && self.has_children(index) {
            return Err("rm: directory not empty");
        }
        if self.cwd == index {
            return Err("rm: cannot remove cwd");
        }
        self.nodes[index] = FsNode::empty();
        self.save_if_possible();
        Ok(())
    }

    pub fn persistence_label(&self) -> &'static str {
        self.persistence.label()
    }

    pub fn list_current_dir_into(
        &self,
        kinds: &mut [EntryKind; MAX_FS_NODES],
        names: &mut [NameText; MAX_FS_NODES],
        sizes: &mut [usize; MAX_FS_NODES],
    ) -> usize {
        let mut len = 0usize;
        let mut index = 0usize;
        while index < MAX_FS_NODES {
            let node = &self.nodes[index];
            if node.used && index != 0 && node.parent == self.cwd {
                kinds[len] = node.kind;
                sizes[len] = node.data_len;
                names[len] = NameText::empty();
                let mut name_index = 0usize;
                while name_index < node.name_len {
                    names[len].bytes[name_index] = node.name[name_index];
                    name_index += 1;
                }
                names[len].len = node.name_len;
                len += 1;
            }
            index += 1;
        }
        len
    }

    fn resolve_dir(&self, path: &str) -> Result<usize, &'static str> {
        let node = self.resolve_path(path)?;
        if self.nodes[node].kind != EntryKind::Dir {
            return Err("cd: not a directory");
        }
        Ok(node)
    }

    fn resolve_path(&self, path: &str) -> Result<usize, &'static str> {
        if path.is_empty() {
            return Ok(self.cwd);
        }

        let mut current = if path.as_bytes().starts_with(b"/") { 0 } else { self.cwd };
        let mut start = 0usize;
        while start <= path.len() {
            let component_end = find_separator(path, start).unwrap_or(path.len());
            let component = &path[start..component_end];
            if component.is_empty() || component == "." {
            } else if component == ".." {
                current = self.nodes[current].parent;
            } else {
                match self.find_child(current, component) {
                    Some(index) => current = index,
                    None => return Err("path not found"),
                }
            }

            if component_end == path.len() {
                break;
            }
            start = component_end + 1;
        }
        Ok(current)
    }

    fn resolve_parent_and_name<'a>(&self, path: &'a str) -> Result<(usize, &'a str), &'static str> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err("missing path");
        }

        let split_index = find_last_separator(trimmed);
        let name = match split_index {
            Some(index) => &trimmed[index + 1..],
            None => trimmed,
        };
        if name.is_empty() || name == "." || name == ".." {
            return Err("invalid name");
        }
        if !valid_name(name) {
            return Err("name too long or invalid");
        }

        let parent_path = match split_index {
            Some(index) => &trimmed[..index],
            None => "",
        };
        let parent = if trimmed.as_bytes().starts_with(b"/") && parent_path.is_empty() {
            0
        } else if parent_path.is_empty() {
            self.cwd
        } else {
            self.resolve_dir(parent_path)?
        };

        Ok((parent, name))
    }

    fn create_node(&mut self, parent: usize, name: &str, kind: EntryKind) -> Result<(), &'static str> {
        if self.find_child(parent, name).is_some() {
            return Err("path already exists");
        }

        let mut index = 1usize;
        while index < MAX_FS_NODES {
            if !self.nodes[index].used {
                match kind {
                    EntryKind::Dir => self.nodes[index].init_dir(parent, name),
                    EntryKind::File => self.nodes[index].init_file(parent, name, "empty file"),
                }
                return Ok(());
            }
            index += 1;
        }
        Err("filesystem full")
    }

    fn find_child(&self, parent: usize, name: &str) -> Option<usize> {
        let mut index = 1usize;
        while index < MAX_FS_NODES {
            let node = &self.nodes[index];
            if node.used && node.parent == parent && node.name_eq(name) {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn has_children(&self, parent: usize) -> bool {
        let mut index = 1usize;
        while index < MAX_FS_NODES {
            if self.nodes[index].used && self.nodes[index].parent == parent {
                return true;
            }
            index += 1;
        }
        false
    }

    fn refresh_cwd_path(&mut self) {
        let mut clear_index = 0usize;
        while clear_index < MAX_PATH_LEN {
            self.cwd_path[clear_index] = b' ';
            clear_index += 1;
        }
        if self.cwd == 0 {
            self.cwd_path[0] = b'/';
            self.cwd_path_len = 1;
            return;
        }

        let mut segments = [[0u8; MAX_NAME_LEN]; 8];
        let mut segment_lens = [0usize; 8];
        let mut segment_count = 0usize;
        let mut current = self.cwd;

        while current != 0 && segment_count < segments.len() {
            let node = &self.nodes[current];
            let mut index = 0usize;
            while index < node.name_len {
                segments[segment_count][index] = node.name[index];
                index += 1;
            }
            segment_lens[segment_count] = node.name_len;
            segment_count += 1;
            current = node.parent;
        }

        let mut len = 0usize;
        self.cwd_path[len] = b'/';
        len += 1;
        let mut segment_index = segment_count;
        while segment_index > 0 {
            segment_index -= 1;
            let mut byte_index = 0usize;
            while byte_index < segment_lens[segment_index] {
                if len >= MAX_PATH_LEN {
                    break;
                }
                self.cwd_path[len] = segments[segment_index][byte_index];
                len += 1;
                byte_index += 1;
            }
            if segment_index != 0 && len < MAX_PATH_LEN {
                self.cwd_path[len] = b'/';
                len += 1;
            }
        }
        self.cwd_path_len = len;
    }

    fn load_or_seed(&mut self) {
        trace::set_boot_stage(0x55);
        if !storage::detect_primary_master() {
            self.persistence = PersistenceState::NoDisk;
            trace::set_boot_stage(0x56);
            return;
        }

        trace::set_boot_stage(0x57);
        match self.load_from_disk() {
            Ok(()) => self.persistence = PersistenceState::Ready,
            Err(PersistenceState::InvalidFormat) => {
                trace::set_boot_stage(0x58);
                if self.save_to_disk() {
                    self.persistence = PersistenceState::Seeded;
                } else {
                    self.persistence = PersistenceState::WriteError;
                }
            }
            Err(error) => self.persistence = error,
        }
        trace::set_boot_stage(0x59);
    }

    fn save_if_possible(&mut self) {
        match self.persistence {
            PersistenceState::NoDisk => {}
            _ => {
                if self.save_to_disk() {
                    self.persistence = PersistenceState::Ready;
                } else {
                    self.persistence = PersistenceState::WriteError;
                }
            }
        }
    }

    fn load_from_disk(&mut self) -> Result<(), PersistenceState> {
        trace::set_boot_stage(0x5A);
        let mut header = [0u8; FS_SECTOR_SIZE];
        if !storage::read_sector(FS_DISK_LBA_START, &mut header) {
            return Err(PersistenceState::ReadError);
        }
        let mut signature_index = 0usize;
        while signature_index < FS_SIGNATURE.len() {
            if header[signature_index] != FS_SIGNATURE[signature_index] {
                return Err(PersistenceState::InvalidFormat);
            }
            signature_index += 1;
        }
        if header[8] != FS_VERSION {
            return Err(PersistenceState::InvalidFormat);
        }

        self.cwd = header[9] as usize;
        if self.cwd >= MAX_FS_NODES {
            self.cwd = 0;
        }

        trace::set_boot_stage(0x5B);
        let mut sector = [0u8; FS_SECTOR_SIZE];
        let mut node_index = 0usize;
        while node_index < MAX_FS_NODES {
            let base = node_index * 128;
            let sector_index = 1 + (base / FS_SECTOR_SIZE);
            let offset = base % FS_SECTOR_SIZE;
            if !storage::read_sector(FS_DISK_LBA_START + sector_index as u32, &mut sector) {
                return Err(PersistenceState::ReadError);
            }

            self.nodes[node_index].used = sector[offset] != 0;
            self.nodes[node_index].kind = if sector[offset + 1] == 1 { EntryKind::Dir } else { EntryKind::File };
            self.nodes[node_index].parent = sector[offset + 2] as usize;
            self.nodes[node_index].name_len = sector[offset + 3] as usize;
            self.nodes[node_index].data_len = u16::from_le_bytes([sector[offset + 4], sector[offset + 5]]) as usize;

            let mut name_index = 0usize;
            while name_index < MAX_NAME_LEN {
                self.nodes[node_index].name[name_index] = sector[offset + 6 + name_index];
                name_index += 1;
            }

            let mut file_index = 0usize;
            while file_index < MAX_FILE_LEN {
                self.nodes[node_index].data[file_index] = sector[offset + 18 + file_index];
                file_index += 1;
            }

            if self.nodes[node_index].name_len > MAX_NAME_LEN || self.nodes[node_index].data_len > MAX_FILE_LEN {
                return Err(PersistenceState::InvalidFormat);
            }
            node_index += 1;
        }

        self.refresh_cwd_path();
        trace::set_boot_stage(0x5C);
        Ok(())
    }

    fn save_to_disk(&self) -> bool {
        trace::set_boot_stage(0x5D);
        let mut sector = [0u8; FS_SECTOR_SIZE];
        let mut signature_index = 0usize;
        while signature_index < FS_SIGNATURE.len() {
            sector[signature_index] = FS_SIGNATURE[signature_index];
            signature_index += 1;
        }
        sector[8] = FS_VERSION;
        sector[9] = self.cwd as u8;
        if !storage::write_sector(FS_DISK_LBA_START, &sector) {
            return false;
        }

        let mut node_index = 0usize;
        while node_index < MAX_FS_NODES {
            let base = node_index * 128;
            let sector_index = 1 + (base / FS_SECTOR_SIZE);
            let offset = base % FS_SECTOR_SIZE;
            let entry = &self.nodes[node_index];
            sector = [0u8; FS_SECTOR_SIZE];
            if !storage::read_sector(FS_DISK_LBA_START + sector_index as u32, &mut sector) {
                return false;
            }

            sector[offset] = if entry.used { 1 } else { 0 };
            sector[offset + 1] = if entry.kind == EntryKind::Dir { 1 } else { 0 };
            sector[offset + 2] = entry.parent as u8;
            sector[offset + 3] = entry.name_len as u8;
            let data_len = (entry.data_len as u16).to_le_bytes();
            sector[offset + 4] = data_len[0];
            sector[offset + 5] = data_len[1];

            let mut name_index = 0usize;
            while name_index < MAX_NAME_LEN {
                sector[offset + 6 + name_index] = entry.name[name_index];
                name_index += 1;
            }

            let mut data_index = 0usize;
            while data_index < MAX_FILE_LEN {
                sector[offset + 18 + data_index] = entry.data[data_index];
                data_index += 1;
            }

            if !storage::write_sector(FS_DISK_LBA_START + sector_index as u32, &sector) {
                return false;
            }
            node_index += 1;
        }
        true
    }
}

fn valid_name(name: &str) -> bool {
    if name.len() > MAX_NAME_LEN {
        return false;
    }
    let bytes = name.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if !matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-') {
            return false;
        }
        index += 1;
    }
    true
}

fn find_separator(path: &str, start: usize) -> Option<usize> {
    let bytes = path.as_bytes();
    let mut index = start;
    while index < bytes.len() {
        if bytes[index] == b'/' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn find_last_separator(path: &str) -> Option<usize> {
    let bytes = path.as_bytes();
    let mut index = bytes.len();
    while index > 0 {
        index -= 1;
        if bytes[index] == b'/' {
            return Some(index);
        }
    }
    None
}

fn sanitize(byte: u8) -> u8 {
    match byte {
        0x20..=0x7E | b'\n' => byte,
        _ => b'?',
    }
}
