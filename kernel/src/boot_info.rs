const BOOT_INFO_SIGNATURE: [u8; 8] = *b"TEDDYOS\0";

#[repr(C, packed)]
struct RawBootInfo {
    signature: [u8; 8],
    version: u8,
    boot_drive: u8,
    kernel_segment: u16,
    kernel_sectors: u16,
    stage2_sectors: u16,
    video_mode: u8,
    framebuffer_bpp: u8,
    framebuffer_addr: u32,
    framebuffer_width: u16,
    framebuffer_height: u16,
    framebuffer_pitch: u16,
}

#[derive(Clone, Copy)]
pub struct BootInfo {
    version: u8,
    boot_drive: u8,
    kernel_segment: u16,
    kernel_sectors: u16,
    stage2_sectors: u16,
    video_mode: u8,
    framebuffer_bpp: u8,
    framebuffer_addr: u32,
    framebuffer_width: u16,
    framebuffer_height: u16,
    framebuffer_pitch: u16,
}

#[derive(Clone, Copy)]
pub struct FramebufferInfo {
    addr: u32,
    width: u16,
    height: u16,
    pitch: u16,
    bpp: u8,
}

impl BootInfo {
    pub fn parse(addr: usize) -> Option<Self> {
        let raw = unsafe { &*(addr as *const RawBootInfo) };
        if raw.signature != BOOT_INFO_SIGNATURE {
            return None;
        }

        Some(Self {
            version: raw.version,
            boot_drive: raw.boot_drive,
            kernel_segment: raw.kernel_segment,
            kernel_sectors: raw.kernel_sectors,
            stage2_sectors: raw.stage2_sectors,
            video_mode: raw.video_mode,
            framebuffer_bpp: raw.framebuffer_bpp,
            framebuffer_addr: raw.framebuffer_addr,
            framebuffer_width: raw.framebuffer_width,
            framebuffer_height: raw.framebuffer_height,
            framebuffer_pitch: raw.framebuffer_pitch,
        })
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn boot_drive(&self) -> u8 {
        self.boot_drive
    }

    pub fn kernel_segment(&self) -> u16 {
        self.kernel_segment
    }

    pub fn kernel_sectors(&self) -> u16 {
        self.kernel_sectors
    }

    pub fn stage2_sectors(&self) -> u16 {
        self.stage2_sectors
    }

    pub fn graphics_mode_enabled(&self) -> bool {
        self.video_mode != 0 && self.framebuffer_addr != 0
    }

    pub fn framebuffer(&self) -> Option<FramebufferInfo> {
        if !self.graphics_mode_enabled() {
            return None;
        }

        Some(FramebufferInfo {
            addr: self.framebuffer_addr,
            width: self.framebuffer_width,
            height: self.framebuffer_height,
            pitch: self.framebuffer_pitch,
            bpp: self.framebuffer_bpp,
        })
    }
}

impl FramebufferInfo {
    pub const fn empty() -> Self {
        Self {
            addr: 0,
            width: 0,
            height: 0,
            pitch: 0,
            bpp: 0,
        }
    }

    pub const fn addr(self) -> u32 {
        self.addr
    }

    pub const fn width(self) -> u16 {
        self.width
    }

    pub const fn height(self) -> u16 {
        self.height
    }

    pub const fn pitch(self) -> u16 {
        self.pitch
    }

    pub const fn bpp(self) -> u8 {
        self.bpp
    }
}
