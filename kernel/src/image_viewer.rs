use crate::{
    fs::{FileSystem, MAX_FILE_LEN},
    trace,
};

const STATUS_LEN: usize = 58;
const PATH_LEN: usize = 96;
const TIMG_SIGNATURE: [u8; 4] = *b"TIMG";
const HEADER_LEN: usize = 8;

pub struct ImageViewerApp {
    path: [u8; PATH_LEN],
    path_len: usize,
    pixels: [u8; MAX_FILE_LEN],
    width: usize,
    height: usize,
    pixel_bytes_len: usize,
    status: [u8; STATUS_LEN],
    status_len: usize,
}

impl ImageViewerApp {
    pub const fn empty() -> Self {
        Self {
            path: [0; PATH_LEN],
            path_len: 0,
            pixels: [0; MAX_FILE_LEN],
            width: 0,
            height: 0,
            pixel_bytes_len: 0,
            status: [b' '; STATUS_LEN],
            status_len: 0,
        }
    }

    pub fn init(&mut self) {
        trace::set_boot_stage(0xB8);
        self.path_len = 0;
        self.width = 0;
        self.height = 0;
        self.pixel_bytes_len = 0;
        self.status_len = 0;
        self.set_status("Open a Teddy image from Explorer");
    }

    pub fn open(&mut self, path: &str, fs: &FileSystem) -> bool {
        let mut buffer = [0u8; MAX_FILE_LEN];
        let len = match fs.read_file_into(path, &mut buffer) {
            Ok(len) => len,
            Err(message) => {
                self.set_status(message);
                return false;
            }
        };

        if len < HEADER_LEN || buffer[..4] != TIMG_SIGNATURE {
            self.set_status("Unsupported image file");
            return false;
        }

        let width = u16::from_le_bytes([buffer[4], buffer[5]]) as usize;
        let height = u16::from_le_bytes([buffer[6], buffer[7]]) as usize;
        if width == 0 || height == 0 {
            self.set_status("Invalid image size");
            return false;
        }

        let packed_len = (width * height).div_ceil(2);
        if HEADER_LEN + packed_len > len {
            self.set_status("Truncated image file");
            return false;
        }

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

        self.pixels = [0; MAX_FILE_LEN];
        let mut index = 0usize;
        while index < packed_len {
            self.pixels[index] = buffer[HEADER_LEN + index];
            index += 1;
        }
        self.width = width;
        self.height = height;
        self.pixel_bytes_len = packed_len;
        self.set_status("Teddy image ready");
        true
    }

    pub fn path(&self) -> &str {
        if self.path_len == 0 {
            "(no image)"
        } else {
            core::str::from_utf8(&self.path[..self.path_len]).unwrap_or("(bad path)")
        }
    }

    pub fn status(&self) -> &str {
        core::str::from_utf8(&self.status[..self.status_len]).unwrap_or("")
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub fn pixel(&self, x: usize, y: usize) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let index = y * self.width + x;
        let packed = self.pixels[index / 2];
        if index & 1 == 0 {
            packed >> 4
        } else {
            packed & 0x0F
        }
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
        0x20..=0x7E => byte,
        _ => b'?',
    }
}
