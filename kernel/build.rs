use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const TRANSPARENT: u8 = 255;
const VGA_PALETTE: [(u8, u8, u8); 16] = [
    (0x00, 0x00, 0x00),
    (0x00, 0x00, 0xAA),
    (0x00, 0xAA, 0x00),
    (0x00, 0xAA, 0xAA),
    (0xAA, 0x00, 0x00),
    (0xAA, 0x00, 0xAA),
    (0xAA, 0x55, 0x00),
    (0xAA, 0xAA, 0xAA),
    (0x55, 0x55, 0x55),
    (0x55, 0x55, 0xFF),
    (0x55, 0xFF, 0x55),
    (0x55, 0xFF, 0xFF),
    (0xFF, 0x55, 0x55),
    (0xFF, 0x55, 0xFF),
    (0xFF, 0xFF, 0x55),
    (0xFF, 0xFF, 0xFF),
];

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().expect("kernel crate should live under repo root");
    let assets_dir = repo_root.join("assets").join("icons");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let output = out_dir.join("generated_icons.rs");

    println!("cargo:rerun-if-changed={}", assets_dir.display());

    let terminal = load_icon(&assets_dir.join("terminal.bmp"));
    let explorer = load_icon(&assets_dir.join("explorer.bmp"));
    let settings = load_icon(&assets_dir.join("settings.bmp"));

    let source = format!(
        "pub const TERMINAL_ICON_WIDTH: usize = {tw};\n\
         pub const TERMINAL_ICON_HEIGHT: usize = {th};\n\
         pub static TERMINAL_ICON_PIXELS: [u8; {tlen}] = {tp};\n\
         pub const EXPLORER_ICON_WIDTH: usize = {ew};\n\
         pub const EXPLORER_ICON_HEIGHT: usize = {eh};\n\
         pub static EXPLORER_ICON_PIXELS: [u8; {elen}] = {ep};\n\
         pub const SETTINGS_ICON_WIDTH: usize = {sw};\n\
         pub const SETTINGS_ICON_HEIGHT: usize = {sh};\n\
         pub static SETTINGS_ICON_PIXELS: [u8; {slen}] = {sp};\n",
        tw = terminal.width,
        th = terminal.height,
        tlen = terminal.pixels.len(),
        tp = format_u8_array(&terminal.pixels),
        ew = explorer.width,
        eh = explorer.height,
        elen = explorer.pixels.len(),
        ep = format_u8_array(&explorer.pixels),
        sw = settings.width,
        sh = settings.height,
        slen = settings.pixels.len(),
        sp = format_u8_array(&settings.pixels),
    );

    fs::write(output, source).expect("write generated_icons.rs");
}

struct IconData {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

fn load_icon(path: &Path) -> IconData {
    if !path.exists() {
        return IconData {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        };
    }

    let bytes = fs::read(path).unwrap_or_else(|err| panic!("failed to read {}: {}", path.display(), err));
    parse_bmp(&bytes, path)
}

fn parse_bmp(bytes: &[u8], path: &Path) -> IconData {
    if bytes.len() < 54 || &bytes[0..2] != b"BM" {
        panic!("{} must be an uncompressed BMP file", path.display());
    }

    let data_offset = read_u32(bytes, 10) as usize;
    let dib_size = read_u32(bytes, 14);
    if dib_size < 40 {
        panic!("{} uses an unsupported BMP header", path.display());
    }

    let width = read_i32(bytes, 18);
    let height = read_i32(bytes, 22);
    let planes = read_u16(bytes, 26);
    let bpp = read_u16(bytes, 28);
    let compression = read_u32(bytes, 30);

    if width <= 0 || height == 0 {
        panic!("{} has invalid dimensions", path.display());
    }
    if planes != 1 || compression != 0 || (bpp != 24 && bpp != 32) {
        panic!("{} must be 24-bit or 32-bit uncompressed BMP", path.display());
    }

    let width = width as usize;
    let top_down = height < 0;
    let height = height.unsigned_abs() as usize;
    let bytes_per_pixel = (bpp / 8) as usize;
    let row_stride = (width * bytes_per_pixel + 3) & !3;
    let required = data_offset + row_stride * height;
    if bytes.len() < required {
        panic!("{} is truncated", path.display());
    }

    let mut pixels = Vec::with_capacity(width * height);
    let mut y = 0usize;
    while y < height {
        let source_y = if top_down { y } else { height - 1 - y };
        let row_start = data_offset + source_y * row_stride;
        let mut x = 0usize;
        while x < width {
            let offset = row_start + x * bytes_per_pixel;
            let b = bytes[offset];
            let g = bytes[offset + 1];
            let r = bytes[offset + 2];
            let a = if bytes_per_pixel == 4 { bytes[offset + 3] } else { 0xFF };
            if a == 0 || (r == 0xFF && g == 0x00 && b == 0xFF) {
                pixels.push(TRANSPARENT);
            } else {
                pixels.push(nearest_vga_palette_index(r, g, b));
            }
            x += 1;
        }
        y += 1;
    }

    IconData {
        width,
        height,
        pixels,
    }
}

fn nearest_vga_palette_index(r: u8, g: u8, b: u8) -> u8 {
    let mut best_index = 0usize;
    let mut best_distance = u32::MAX;
    let mut index = 0usize;
    while index < VGA_PALETTE.len() {
        let (pr, pg, pb) = VGA_PALETTE[index];
        let dr = r as i32 - pr as i32;
        let dg = g as i32 - pg as i32;
        let db = b as i32 - pb as i32;
        let distance = (dr * dr + dg * dg + db * db) as u32;
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
        index += 1;
    }
    best_index as u8
}

fn format_u8_array(values: &[u8]) -> String {
    let mut rendered = String::from("[");
    let mut index = 0usize;
    while index < values.len() {
        if index != 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&values[index].to_string());
        index += 1;
    }
    rendered.push(']');
    rendered
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
