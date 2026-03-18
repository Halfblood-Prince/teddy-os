use crate::fs::{FileSystem, MAX_FILE_LEN};

const STATUS_LEN: usize = 58;
const PATH_LEN: usize = 72;

pub struct WriterApp {
    path: [u8; PATH_LEN],
    path_len: usize,
    buffer: [u8; MAX_FILE_LEN],
    len: usize,
    dirty: bool,
    status: [u8; STATUS_LEN],
    status_len: usize,
}

impl WriterApp {
    pub const fn empty() -> Self {
        Self {
            path: [0; PATH_LEN],
            path_len: 0,
            buffer: [0; MAX_FILE_LEN],
            len: 0,
            dirty: false,
            status: [b' '; STATUS_LEN],
            status_len: 0,
        }
    }

    pub fn init(&mut self) {
        self.path = [0; PATH_LEN];
        self.path_len = 0;
        self.buffer = [0; MAX_FILE_LEN];
        self.len = 0;
        self.dirty = false;
        self.status = [b' '; STATUS_LEN];
        self.status_len = 0;
        self.set_status("Open a .txt file from Explorer to edit it");
    }

    pub fn open(&mut self, path: &str, fs: &FileSystem) -> bool {
        let mut data = [0u8; MAX_FILE_LEN];
        match fs.read_file_into(path, &mut data) {
            Ok(len) => {
                self.path = [0; PATH_LEN];
                self.path_len = 0;
                let path_bytes = path.as_bytes();
                let path_limit = core::cmp::min(path_bytes.len(), PATH_LEN);
                let mut path_index = 0usize;
                while path_index < path_limit {
                    self.path[path_index] = sanitize(path_bytes[path_index]);
                    self.path_len += 1;
                    path_index += 1;
                }

                self.buffer = [0; MAX_FILE_LEN];
                self.len = 0;
                let mut index = 0usize;
                while index < len {
                    self.buffer[index] = sanitize(data[index]);
                    self.len += 1;
                    index += 1;
                }
                self.dirty = false;
                self.set_status("Teddy Write ready  Click SAVE to store changes");
                true
            }
            Err(message) => {
                self.set_status(message);
                false
            }
        }
    }

    pub fn handle_key(&mut self, ascii: u8) -> bool {
        match ascii {
            8 => {
                if self.len > 0 {
                    self.len -= 1;
                    self.buffer[self.len] = 0;
                    self.dirty = true;
                    self.set_status("Deleted character");
                    return true;
                }
                false
            }
            b'\n' => {
                if self.len < MAX_FILE_LEN {
                    self.buffer[self.len] = b'\n';
                    self.len += 1;
                    self.dirty = true;
                    self.set_status("Inserted newline");
                    return true;
                }
                self.set_status("File is full");
                true
            }
            0x20..=0x7E => {
                if self.len < MAX_FILE_LEN {
                    self.buffer[self.len] = ascii;
                    self.len += 1;
                    self.dirty = true;
                    self.set_status("Editing");
                    return true;
                }
                self.set_status("File is full");
                true
            }
            _ => false,
        }
    }

    pub fn save(&mut self, fs: &mut FileSystem) -> bool {
        if self.path_len == 0 {
            self.set_status("No text file is open");
            return false;
        }

        let path = core::str::from_utf8(&self.path[..self.path_len]).unwrap_or("");
        match fs.write_file(path, &self.buffer[..self.len]) {
            Ok(()) => {
                self.dirty = false;
                self.set_status("Saved");
                true
            }
            Err(message) => {
                self.set_status(message);
                false
            }
        }
    }

    pub fn revert(&mut self, fs: &FileSystem) -> bool {
        if self.path_len == 0 {
            self.set_status("No text file is open");
            return false;
        }
        let mut path = [0u8; PATH_LEN];
        let mut index = 0usize;
        while index < self.path_len {
            path[index] = self.path[index];
            index += 1;
        }
        let text = core::str::from_utf8(&path[..self.path_len]).unwrap_or("");
        self.open(text, fs)
    }

    pub fn path(&self) -> &str {
        if self.path_len == 0 {
            "(no file)"
        } else {
            core::str::from_utf8(&self.path[..self.path_len]).unwrap_or("(bad path)")
        }
    }

    pub fn status(&self) -> &str {
        core::str::from_utf8(&self.status[..self.status_len]).unwrap_or("")
    }

    pub fn text_len(&self) -> usize {
        self.len
    }

    pub fn text_byte(&self, index: usize) -> u8 {
        self.buffer[index]
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn set_status(&mut self, text: &str) {
        self.status = [b' '; STATUS_LEN];
        self.status_len = 0;
        let bytes = text.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() && index < STATUS_LEN {
            self.status[index] = sanitize(bytes[index]);
            self.status_len += 1;
            index += 1;
        }
    }
}

fn sanitize(byte: u8) -> u8 {
    match byte {
        0x20..=0x7E | b'\n' => byte,
        _ => b'?',
    }
}
