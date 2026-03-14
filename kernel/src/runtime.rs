use core::fmt::{self, Write};

use spin::Mutex;
use teddy_boot_proto::BootInfo;

use crate::{
    framebuffer::{Color, FramebufferSurface, Point, Rect},
    input,
    memory,
    timer,
};

const MAX_TASKS: usize = 4;

#[derive(Clone, Copy)]
pub struct TaskContext {
    pub tick_count: u64,
}

#[derive(Clone, Copy)]
struct Task {
    callback: fn(TaskContext),
}

struct Scheduler {
    tasks: [Option<Task>; MAX_TASKS],
    task_count: usize,
    next_index: usize,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            tasks: [None, None, None, None],
            task_count: 0,
            next_index: 0,
        }
    }

    fn register(&mut self, task: Task) {
        if self.task_count < MAX_TASKS {
            self.tasks[self.task_count] = Some(task);
            self.task_count += 1;
        }
    }

    fn run_next(&mut self, tick_count: u64) {
        if self.task_count == 0 {
            return;
        }

        let task = self.tasks[self.next_index % self.task_count]
            .expect("scheduler slot should be populated");
        self.next_index = (self.next_index + 1) % self.task_count;
        (task.callback)(TaskContext { tick_count });
    }
}

struct RuntimeState {
    surface: Option<FramebufferSurface>,
    last_status_tick: u64,
    last_key_unicode: Option<char>,
    last_key_name: &'static str,
    last_key_scancode: u8,
    last_key_pressed: bool,
}

impl RuntimeState {
    const fn new() -> Self {
        Self {
            surface: None,
            last_status_tick: u64::MAX,
            last_key_unicode: None,
            last_key_name: "None",
            last_key_scancode: 0,
            last_key_pressed: false,
        }
    }
}

static RUNTIME: Mutex<RuntimeState> = Mutex::new(RuntimeState::new());
static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

pub fn init(boot_info: &BootInfo) {
    {
        let mut runtime = RUNTIME.lock();
        runtime.surface = FramebufferSurface::new(boot_info.framebuffer);
    }

    draw_boot_scene();

    let mut scheduler = SCHEDULER.lock();
    scheduler.register(Task {
        callback: status_task,
    });
    scheduler.register(Task {
        callback: input_task,
    });
}

pub fn run_next_task() {
    let tick_count = timer::ticks();
    SCHEDULER.lock().run_next(tick_count);
}

fn draw_boot_scene() {
    let mut runtime = RUNTIME.lock();
    let Some(surface) = runtime.surface.as_mut() else {
        return;
    };
    let info = surface.info();

    surface.clear(Color::rgb(0x11, 0x1B, 0x29));
    surface.fill_rect(
        Rect {
            x: 24,
            y: 24,
            width: info.width as usize.saturating_sub(48),
            height: info.height as usize.saturating_sub(96),
        },
        Color::rgb(0x16, 0x24, 0x33),
    );
    surface.stroke_rect(
        Rect {
            x: 24,
            y: 24,
            width: info.width as usize.saturating_sub(48),
            height: info.height as usize.saturating_sub(96),
        },
        Color::rgb(0x6B, 0x8A, 0xA6),
    );
    surface.fill_rect(
        Rect {
            x: 0,
            y: info.height as usize.saturating_sub(52),
            width: info.width as usize,
            height: 52,
        },
        Color::rgb(0x1B, 0x2A, 0x3B),
    );
    surface.draw_line(
        Point { x: 24, y: 24 },
        Point {
            x: info.width as usize.saturating_sub(24),
            y: info.height as usize.saturating_sub(72),
        },
        Color::rgb(0x56, 0x7B, 0x93),
    );
    surface.draw_line(
        Point {
            x: info.width as usize.saturating_sub(24),
            y: 24,
        },
        Point {
            x: 24,
            y: info.height as usize.saturating_sub(72),
        },
        Color::rgb(0x56, 0x7B, 0x93),
    );
    surface.draw_text(
        "Teddy-OS Phase 2 Kernel MVP",
        40,
        42,
        Color::rgb(0xEF, 0xF6, 0xFF),
        Color::rgb(0x16, 0x24, 0x33),
    );
    surface.draw_text(
        "Interrupts, timer, keyboard, memory, and primitive graphics online.",
        40,
        66,
        Color::rgb(0xCF, 0xDA, 0xE5),
        Color::rgb(0x16, 0x24, 0x33),
    );
}

fn status_task(context: TaskContext) {
    let mut runtime = RUNTIME.lock();
    if context.tick_count == runtime.last_status_tick {
        return;
    }
    runtime.last_status_tick = context.tick_count;

    let last_key_pressed = runtime.last_key_pressed;
    let last_key_scancode = runtime.last_key_scancode;
    let last_key_name = runtime.last_key_name;
    let last_key_unicode = runtime.last_key_unicode;

    let Some(surface) = runtime.surface.as_mut() else {
        return;
    };

    let timer_snapshot = timer::snapshot();
    let memory_stats = memory::stats();
    let input_snapshot = input::snapshot();

    let mut line = FixedString::new();
    let background = Color::rgb(0x1F, 0x31, 0x45);
    let foreground = Color::rgb(0xD0, 0xDE, 0xEA);
    let title = Color::rgb(0xF2, 0xF7, 0xFC);

    surface.fill_rect(
        Rect {
            x: 44,
            y: 110,
            width: 540,
            height: 124,
        },
        background,
    );
    surface.stroke_rect(
        Rect {
            x: 44,
            y: 110,
            width: 540,
            height: 124,
        },
        Color::rgb(0x76, 0x93, 0xA8),
    );
    surface.draw_text("Kernel status", 60, 126, title, background);

    let total_mib = memory_stats.total_bytes / 1024 / 1024;
    let usable_mib = memory_stats.usable_bytes / 1024 / 1024;
    let uptime_secs = timer_snapshot.ticks / timer_snapshot.frequency_hz as u64;

    line.write_line(format_args!("Ticks: {}", timer_snapshot.ticks));
    surface.draw_text(line.as_str(), 60, 150, foreground, background);
    line.clear();
    line.write_line(format_args!("Timer Hz: {}", timer_snapshot.frequency_hz));
    surface.draw_text(line.as_str(), 60, 170, foreground, background);
    line.clear();
    line.write_line(format_args!("Uptime s: {}", uptime_secs));
    surface.draw_text(line.as_str(), 60, 190, foreground, background);

    line.clear();
    line.write_line(format_args!("RAM MiB: {}", total_mib));
    surface.draw_text(line.as_str(), 280, 150, foreground, background);
    line.clear();
    line.write_line(format_args!("Usable MiB: {}", usable_mib));
    surface.draw_text(line.as_str(), 280, 170, foreground, background);
    line.clear();
    line.write_line(format_args!("Input total: {}", input_snapshot.total_events));
    surface.draw_text(line.as_str(), 280, 190, foreground, background);
    line.clear();
    line.write_line(format_args!("Input queued: {}", input_snapshot.pending_events));
    surface.draw_text(line.as_str(), 280, 210, foreground, background);

    let key_background = Color::rgb(0x1B, 0x2A, 0x3B);
    surface.fill_rect(
        Rect {
            x: 44,
            y: 248,
            width: 540,
            height: 76,
        },
        key_background,
    );
    surface.stroke_rect(
        Rect {
            x: 44,
            y: 248,
            width: 540,
            height: 76,
        },
        Color::rgb(0x76, 0x93, 0xA8),
    );
    surface.draw_text("Latest keyboard event", 60, 264, title, key_background);

    let key_action = if last_key_pressed { "down" } else { "up" };
    line.clear();
    line.write_line(format_args!(
        "Scan: {}  Key: {}  State: {}",
        last_key_scancode,
        last_key_name,
        key_action
    ));
    surface.draw_text(line.as_str(), 60, 288, foreground, key_background);

    line.clear();
    let glyph = last_key_unicode.unwrap_or('-');
    line.write_line(format_args!("Char: {}", glyph));
    surface.draw_text(
        line.as_str(),
        380,
        288,
        Color::rgb(0xFF, 0xD4, 0x7A),
        key_background,
    );
}

fn input_task(_context: TaskContext) {
    while let Some(event) = input::pop_event() {
        let mut runtime = RUNTIME.lock();
        runtime.last_key_unicode = event.unicode;
        runtime.last_key_name = event.key_name;
        runtime.last_key_scancode = event.scancode;
        runtime.last_key_pressed = event.pressed;
    }
}

struct FixedString {
    buffer: [u8; 96],
    len: usize,
}

impl FixedString {
    fn new() -> Self {
        Self {
            buffer: [0; 96],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn write_line(&mut self, args: fmt::Arguments<'_>) {
        let _ = self.write_fmt(args);
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.len]).unwrap_or("?")
    }
}

impl Write for FixedString {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let bytes = text.as_bytes();
        let remaining = self.buffer.len().saturating_sub(self.len);
        let write_len = bytes.len().min(remaining);
        self.buffer[self.len..self.len + write_len].copy_from_slice(&bytes[..write_len]);
        self.len += write_len;
        Ok(())
    }
}
