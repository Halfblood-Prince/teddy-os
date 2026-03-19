use crate::{
    fs::{EntryKind, FileSystem, NameText, MAX_FILE_LEN, MAX_FS_NODES},
    trace,
};

const STATUS_LEN: usize = 58;

pub enum ExplorerAction {
    None,
    Changed,
    OpenTextFile(NameText),
    OpenImageFile(NameText),
}

pub struct ExplorerApp {
    selection: usize,
    status: [u8; STATUS_LEN],
    status_len: usize,
    created_dirs: u8,
    created_files: u8,
    renamed_dirs: u8,
    renamed_files: u8,
}

impl ExplorerApp {
    pub const fn empty() -> Self {
        Self {
            selection: 0,
            status: [b' '; STATUS_LEN],
            status_len: 0,
            created_dirs: 0,
            created_files: 0,
            renamed_dirs: 0,
            renamed_files: 0,
        }
    }

    pub fn init(&mut self) {
        trace::set_boot_stage(0xA0);
        self.selection = 0;
        trace::set_boot_stage(0xA1);
        self.status = [b' '; STATUS_LEN];
        self.status_len = 0;
        self.created_dirs = 0;
        self.created_files = 0;
        self.renamed_dirs = 0;
        self.renamed_files = 0;
        trace::set_boot_stage(0xA2);
        self.set_status("J/K select  Enter open  B up  H home  N/T make  R rename");
        trace::set_boot_stage(0xA3);
    }

    pub fn handle_key(&mut self, ascii: u8, fs: &mut FileSystem) -> ExplorerAction {
        match ascii {
            b'j' => {
                let count = self.entry_count(fs);
                if count > 0 && self.selection + 1 < count {
                    self.selection += 1;
                }
                ExplorerAction::Changed
            }
            b'k' => {
                if self.selection > 0 {
                    self.selection -= 1;
                }
                ExplorerAction::Changed
            }
            b'b' => {
                if self.go_parent(fs) {
                    ExplorerAction::Changed
                } else {
                    ExplorerAction::None
                }
            }
            b'h' => {
                if self.go_home(fs) {
                    ExplorerAction::Changed
                } else {
                    ExplorerAction::None
                }
            }
            b'n' => {
                if self.create_folder(fs) {
                    ExplorerAction::Changed
                } else {
                    ExplorerAction::None
                }
            }
            b't' => {
                if self.create_file(fs) {
                    ExplorerAction::Changed
                } else {
                    ExplorerAction::None
                }
            }
            b'x' => {
                if self.delete_selected(fs) {
                    ExplorerAction::Changed
                } else {
                    ExplorerAction::None
                }
            }
            b'r' => {
                if self.rename_selected(fs) {
                    ExplorerAction::Changed
                } else {
                    ExplorerAction::None
                }
            }
            b'\n' => self.open_selected(fs),
            _ => ExplorerAction::None,
        }
    }

    pub fn select_index(&mut self, index: usize, fs: &FileSystem) -> bool {
        let count = self.entry_count(fs);
        if index >= count {
            return false;
        }
        self.selection = index;
        self.set_status("Entry selected");
        true
    }

    pub fn open_selected(&mut self, fs: &mut FileSystem) -> ExplorerAction {
        let mut kinds = [EntryKind::File; MAX_FS_NODES];
        let mut names = [NameText::empty(); MAX_FS_NODES];
        let mut sizes = [0usize; MAX_FS_NODES];
        let len = fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes);
        if self.selection >= len {
            self.set_status("No entry selected");
            return ExplorerAction::None;
        }

        match kinds[self.selection] {
            EntryKind::Dir => match fs.change_dir(names[self.selection].as_str()) {
                Ok(()) => {
                    self.selection = 0;
                    self.set_status("Opened folder");
                    ExplorerAction::Changed
                }
                Err(message) => {
                    self.set_status(message);
                    ExplorerAction::None
                }
            },
            EntryKind::File => {
                let name = names[self.selection];
                if ends_with_txt(name.as_str()) {
                    self.set_status("Opening Teddy Write");
                    ExplorerAction::OpenTextFile(name)
                } else if is_timg_file(name.as_str(), fs) {
                    self.set_status("Opening Image Viewer");
                    ExplorerAction::OpenImageFile(name)
                } else {
                    let mut buffer = [0u8; MAX_FILE_LEN];
                    match fs.read_file_into(name.as_str(), &mut buffer) {
                        Ok(read_len) => {
                            self.set_preview(&buffer, read_len);
                            ExplorerAction::Changed
                        }
                        Err(message) => {
                            self.set_status(message);
                            ExplorerAction::None
                        }
                    }
                }
            }
        }
    }

    pub fn go_parent(&mut self, fs: &mut FileSystem) -> bool {
        match fs.change_dir("..") {
            Ok(()) => {
                self.selection = 0;
                self.set_status("Moved to parent directory");
                true
            }
            Err(message) => {
                self.set_status(message);
                false
            }
        }
    }

    pub fn go_home(&mut self, fs: &mut FileSystem) -> bool {
        match fs.change_dir("/") {
            Ok(()) => {
                self.selection = 0;
                self.set_status("Opened home");
                true
            }
            Err(message) => {
                self.set_status(message);
                false
            }
        }
    }

    pub fn go_docs(&mut self, fs: &mut FileSystem) -> bool {
        match fs.change_dir("/docs") {
            Ok(()) => {
                self.selection = 0;
                self.set_status("Opened docs");
                true
            }
            Err(message) => {
                self.set_status(message);
                false
            }
        }
    }

    pub fn create_folder(&mut self, fs: &mut FileSystem) -> bool {
        let name = next_name("dir", &mut self.created_dirs);
        match fs.create_dir(name) {
            Ok(()) => {
                self.set_status("Folder created");
                true
            }
            Err(message) => {
                self.set_status(message);
                false
            }
        }
    }

    pub fn create_file(&mut self, fs: &mut FileSystem) -> bool {
        let name = next_name("file", &mut self.created_files);
        match fs.touch(name) {
            Ok(()) => {
                self.set_status("File created");
                true
            }
            Err(message) => {
                self.set_status(message);
                false
            }
        }
    }

    pub fn delete_selected(&mut self, fs: &mut FileSystem) -> bool {
        let mut name = [0u8; crate::fs::MAX_NAME_LEN];
        if let Some(name_len) = self.selected_name_into(fs, &mut name) {
            let entry_name = core::str::from_utf8(&name[..name_len]).unwrap_or("");
            match fs.remove(entry_name) {
                Ok(()) => {
                    self.selection = self.selection.saturating_sub(1);
                    self.set_status("Entry removed");
                    true
                }
                Err(message) => {
                    self.set_status(message);
                    false
                }
            }
        } else {
            self.set_status("No entry selected");
            false
        }
    }

    pub fn rename_selected(&mut self, fs: &mut FileSystem) -> bool {
        let mut kinds = [EntryKind::File; MAX_FS_NODES];
        let mut names = [NameText::empty(); MAX_FS_NODES];
        let mut sizes = [0usize; MAX_FS_NODES];
        let len = fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes);
        if self.selection >= len {
            self.set_status("No entry selected");
            return false;
        }

        let old_name = names[self.selection].as_str();
        let new_name = match kinds[self.selection] {
            EntryKind::Dir => next_rename("folder", &mut self.renamed_dirs),
            EntryKind::File => next_rename("file", &mut self.renamed_files),
        };

        match fs.rename(old_name, new_name) {
            Ok(()) => {
                self.set_status("Entry renamed");
                true
            }
            Err(message) => {
                self.set_status(message);
                false
            }
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

    pub fn selected_name_into(&self, fs: &FileSystem, out: &mut [u8; crate::fs::MAX_NAME_LEN]) -> Option<usize> {
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

fn next_rename(prefix: &str, counter: &mut u8) -> &'static str {
    *counter = counter.wrapping_add(1);
    match (prefix, *counter % 4) {
        ("folder", 1) => "ren1",
        ("folder", 2) => "ren2",
        ("folder", 3) => "ren3",
        ("folder", _) => "ren4",
        ("file", 1) => "edit1.txt",
        ("file", 2) => "edit2.txt",
        ("file", 3) => "edit3.txt",
        _ => "edit4.txt",
    }
}

fn sanitize(byte: u8) -> u8 {
    match byte {
        0x20..=0x7E => byte,
        _ => b'?',
    }
}

fn ends_with_txt(name: &str) -> bool {
    name.as_bytes().ends_with(b".txt")
}

fn is_timg_file(name: &str, fs: &FileSystem) -> bool {
    let mut buffer = [0u8; MAX_FILE_LEN];
    match fs.read_file_into(name, &mut buffer) {
        Ok(len) => len >= 4 && buffer[..4] == *b"TIMG",
        Err(_) => false,
    }
}
