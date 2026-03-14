use core::fmt::{self, Write};

use spin::Mutex;
use teddy_boot_proto::BootInfo;

use crate::framebuffer::{Color, FramebufferConsole};
use crate::serial;

static LOGGER: Mutex<Logger> = Mutex::new(Logger::new());

pub fn init(boot_info: &BootInfo) {
    let mut logger = LOGGER.lock();
    logger.framebuffer = FramebufferConsole::new(boot_info.framebuffer);

    if let Some(framebuffer) = logger.framebuffer.as_mut() {
        framebuffer.clear(Color::rgb(0x10, 0x18, 0x24));
        framebuffer.write_str("Teddy-OS kernel boot log\n");
        framebuffer.write_str("========================\n");
    }
}

pub fn print(args: fmt::Arguments<'_>) {
    let _ = LOGGER.lock().write_fmt(args);
}

struct Logger {
    framebuffer: Option<FramebufferConsole>,
}

impl Logger {
    const fn new() -> Self {
        Self { framebuffer: None }
    }
}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial::write_str(s);
        if let Some(framebuffer) = self.framebuffer.as_mut() {
            framebuffer.write_str(s);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::logger::print(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! logln {
    () => {
        $crate::log!("\n")
    };
    ($fmt:expr) => {
        $crate::log!(concat!($fmt, "\n"))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::log!(concat!($fmt, "\n"), $($arg)*)
    };
}

