use crate::fs::{EntryKind, FileSystem, NameText, MAX_FILE_LEN, MAX_FS_NODES};

const STATUS_LEN: usize = 58;

pub struct ExplorerApp {
    selection: usize,
    status: [u8; STATUS_LEN],
    status_len: usize,
    created_dirs: u8,
    created_files: u8,
}

impl ExplorerApp {
    pub const fn empty() -> Self {
        Self {
            selection: 0,
            status: [b' '; STATUS_LEN],
            status_len: 0,
            created_dirs: 0,
            created_files: 0,
        }
    }

    pub fn init(&mut self) {
        self.selection = 0;
        self.status = [b' '; STATUS_LEN];
        self.status_len = 0;
        self.created_dirs = 0;
        self.created_files = 0;
        self.set_status("J/K select  Enter open  B back  N dir  T file  X delete");
    }

    pub fn handle_key(&mut self, ascii: u8, fs: &mut FileSystem) -> bool {
        let count = self.entry_count(fs);
        match ascii {
            b'j' => {
                if count > 0 && self.selection + 1 < count {
                    self.selection += 1;
                }
                true
            }
            b'k' => {
                if self.selection > 0 {
                    self.selection -= 1;
                }
                true
            }
            b'b' => {
                match fs.change_dir("..") {
                    Ok(()) => {
                        self.selection = 0;
                        self.set_status("Moved to parent directory");
                    }
                    Err(message) => self.set_status(message),
                }
                true
            }
            b'n' => {
                let name = next_name("dir", &mut self.created_dirs);
                match fs.create_dir(name) {
                    Ok(()) => self.set_status("Folder created"),
                    Err(message) => self.set_status(message),
                }
                true
            }
            b't' => {
                let name = next_name("file", &mut self.created_files);
                match fs.touch(name) {
                    Ok(()) => self.set_status("File created"),
                    Err(message) => self.set_status(message),
                }
                true
            }
            b'x' => {
                let mut name = [0u8; 12];
                if let Some(name_len) = self.selected_name_into(fs, &mut name) {
                    let entry_name = core::str::from_utf8(&name[..name_len]).unwrap_or("");
                    match fs.remove(entry_name) {
                        Ok(()) => {
                            self.selection = self.selection.saturating_sub(1);
                            self.set_status("Entry removed");
                        }
                        Err(message) => self.set_status(message),
                    }
                } else {
                    self.set_status("No entry selected");
                }
                true
            }
            b'\n' => {
                let mut kinds = [EntryKind::File; MAX_FS_NODES];
                let mut names = [NameText::empty(); MAX_FS_NODES];
                let mut sizes = [0usize; MAX_FS_NODES];
                let len = fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes);
                if self.selection < len {
                    match kinds[self.selection] {
                        EntryKind::Dir => {
                            match fs.change_dir(names[self.selection].as_str()) {
                                Ok(()) => {
                                    self.selection = 0;
                                    self.set_status("Opened folder");
                                }
                                Err(message) => self.set_status(message),
                            }
                        }
                        EntryKind::File => {
                            let mut buffer = [0u8; MAX_FILE_LEN];
                            match fs.read_file_into(names[self.selection].as_str(), &mut buffer) {
                                Ok(read_len) => self.set_preview(&buffer, read_len),
                                Err(message) => self.set_status(message),
                            }
                        }
                    }
                } else {
                    self.set_status("No entry selected");
                }
                true
            }
            _ => false,
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selection
    }

    pub fn status(&self) -> &str {
        core::str::from_utf8(&self.status[..self.status_len]).unwrap_or("")
    }

    fn entry_count(&self, fs: &FileSystem) -> usize {
        let mut kinds = [EntryKind::File; MAX_FS_NODES];
        let mut names = [NameText::empty(); MAX_FS_NODES];
        let mut sizes = [0usize; MAX_FS_NODES];
        fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes)
    }

    pub fn selected_name_into(&self, fs: &FileSystem, out: &mut [u8; 12]) -> Option<usize> {
        let mut kinds = [EntryKind::File; MAX_FS_NODES];
        let mut names = [NameText::empty(); MAX_FS_NODES];
        let mut sizes = [0usize; MAX_FS_NODES];
        let len = fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes);
        if self.selection >= len {
            return None;
        }
        let bytes = names[self.selection].as_str().as_bytes();
        let mut index = 0usize;
        while index < bytes.len() && index < out.len() {
            out[index] = bytes[index];
            index += 1;
        }
        Some(index)
    }

    fn set_status(&mut self, text: &str) {
        self.status = [b' '; STATUS_LEN];
        self.status_len = 0;
        let bytes = text.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() && index < STATUS_LEN {
            self.status[index] = bytes[index];
            self.status_len += 1;
            index += 1;
        }
    }

    fn set_preview(&mut self, bytes: &[u8; MAX_FILE_LEN], len: usize) {
        self.status = [b' '; STATUS_LEN];
        self.status_len = 0;
        let limit = core::cmp::min(len, STATUS_LEN);
        let mut index = 0usize;
        while index < limit {
            self.status[index] = sanitize(bytes[index]);
            self.status_len += 1;
            index += 1;
        }
    }
}

fn next_name(prefix: &str, counter: &mut u8) -> &'static str {
    *counter = counter.wrapping_add(1);
    match (prefix, *counter % 4) {
        ("dir", 1) => "dir1",
        ("dir", 2) => "dir2",
        ("dir", 3) => "dir3",
        ("dir", _) => "dir4",
        ("file", 1) => "file1.txt",
        ("file", 2) => "file2.txt",
        ("file", 3) => "file3.txt",
        _ => "file4.txt",
    }
}

fn sanitize(byte: u8) -> u8 {
    match byte {
        0x20..=0x7E => byte,
        _ => b'?',
    }
}
