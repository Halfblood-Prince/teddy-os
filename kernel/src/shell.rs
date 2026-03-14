use core::fmt::{self, Write};

use spin::Mutex;
use teddy_boot_proto::FramebufferInfo;

use crate::{
    framebuffer::{Color, FramebufferSurface, Point, Rect},
    input::{KeyboardEvent, MouseEvent, mouse_snapshot},
    terminal,
    timer,
};

const MAX_WINDOWS: usize = 3;
const TASKBAR_HEIGHT: usize = 42;
const LAUNCHER_WIDTH: usize = 90;
const CLOCK_WIDTH: usize = 112;
const TITLE_BAR_HEIGHT: usize = 28;
const MENU_WIDTH: usize = 220;
const MENU_HEIGHT: usize = 162;

#[derive(Clone, Copy)]
enum WindowKind {
    Terminal,
    Info,
    Roadmap,
}

#[derive(Clone, Copy)]
struct Theme {
    wallpaper_top: Color,
    wallpaper_bottom: Color,
    wallpaper_accent: Color,
    taskbar: Color,
    taskbar_edge: Color,
    launcher: Color,
    launcher_active: Color,
    window_frame: Color,
    active_title: Color,
    inactive_title: Color,
    text: Color,
    muted_text: Color,
    panel: Color,
    clock: Color,
    cursor: Color,
}

impl Theme {
    const fn teddy() -> Self {
        Self {
            wallpaper_top: Color::rgb(0x10, 0x2D, 0x4A),
            wallpaper_bottom: Color::rgb(0x1A, 0x4E, 0x68),
            wallpaper_accent: Color::rgb(0xE3, 0xB5, 0x57),
            taskbar: Color::rgb(0x1A, 0x23, 0x31),
            taskbar_edge: Color::rgb(0x66, 0x7B, 0x8F),
            launcher: Color::rgb(0x2B, 0x63, 0x4F),
            launcher_active: Color::rgb(0x3D, 0x88, 0x69),
            window_frame: Color::rgb(0x3D, 0x4E, 0x62),
            active_title: Color::rgb(0x2B, 0x5A, 0x87),
            inactive_title: Color::rgb(0x5B, 0x6F, 0x83),
            text: Color::rgb(0x11, 0x18, 0x21),
            muted_text: Color::rgb(0x4E, 0x5C, 0x6B),
            panel: Color::rgb(0xF4, 0xF7, 0xFB),
            clock: Color::rgb(0xF5, 0xD8, 0x8B),
            cursor: Color::rgb(0xFF, 0xFA, 0xF2),
        }
    }
}

#[derive(Clone, Copy)]
struct DesktopWindow {
    title: &'static str,
    body_lines: [&'static str; 4],
    rect: Rect,
    accent: Color,
    kind: WindowKind,
}

#[derive(Clone, Copy)]
struct DragState {
    window_index: usize,
    offset_x: usize,
    offset_y: usize,
}

struct ShellState {
    surface: Option<FramebufferSurface>,
    theme: Theme,
    windows: [DesktopWindow; MAX_WINDOWS],
    active_window: usize,
    dragging: Option<DragState>,
    launcher_open: bool,
    last_mouse_x: usize,
    last_mouse_y: usize,
    last_left_down: bool,
    last_key_unicode: Option<char>,
    last_key_name: &'static str,
    last_render_tick: u64,
    last_status_text: FixedString,
}

impl ShellState {
    const fn new() -> Self {
        Self {
            surface: None,
            theme: Theme::teddy(),
            windows: [
                DesktopWindow {
                    title: "System",
                    body_lines: [
                        "Kernel MVP services online",
                        "Timer, keyboard, mouse active",
                        "Frame allocator ready",
                        "VMware is the primary target",
                    ],
                    rect: Rect {
                        x: 470,
                        y: 96,
                        width: 266,
                        height: 176,
                    },
                    accent: Color::rgb(0x69, 0x57, 0x8E),
                    kind: WindowKind::Info,
                },
                DesktopWindow {
                    title: "Roadmap",
                    body_lines: [
                        "Phase 4: terminal complete",
                        "Phase 5: persistent filesystem",
                        "Phase 6: file explorer",
                        "Phase 7: updater",
                    ],
                    rect: Rect {
                        x: 520,
                        y: 288,
                        width: 252,
                        height: 148,
                    },
                    accent: Color::rgb(0x7B, 0x5B, 0x26),
                    kind: WindowKind::Roadmap,
                },
                DesktopWindow {
                    title: "Teddy Terminal",
                    body_lines: ["", "", "", ""],
                    rect: Rect {
                        x: 56,
                        y: 70,
                        width: 566,
                        height: 324,
                    },
                    accent: Color::rgb(0x2F, 0x7C, 0x55),
                    kind: WindowKind::Terminal,
                },
            ],
            active_window: 2,
            dragging: None,
            launcher_open: false,
            last_mouse_x: 0,
            last_mouse_y: 0,
            last_left_down: false,
            last_key_unicode: None,
            last_key_name: "None",
            last_render_tick: u64::MAX,
            last_status_text: FixedString::new(),
        }
    }
}

static SHELL: Mutex<ShellState> = Mutex::new(ShellState::new());

pub fn init(framebuffer: FramebufferInfo) {
    let mut shell = SHELL.lock();
    shell.surface = FramebufferSurface::new(framebuffer);
    if let Some(surface) = shell.surface.as_ref() {
        let info = surface.info();
        shell.last_mouse_x = info.width as usize / 2;
        shell.last_mouse_y = info.height as usize / 2;
    }
}

pub fn handle_keyboard_event(event: KeyboardEvent) {
    let mut shell = SHELL.lock();
    shell.last_key_unicode = event.unicode;
    shell.last_key_name = event.key_name;

    if matches!(shell.windows[shell.active_window].kind, WindowKind::Terminal) {
        terminal::handle_keyboard_event(event);
    }
}

pub fn handle_mouse_event(event: MouseEvent) {
    let mut shell = SHELL.lock();
    let Some(surface_info) = shell.surface.as_ref().map(|surface| surface.info()) else {
        return;
    };

    let desktop_height = surface_info.height as usize;
    let taskbar_rect = Rect {
        x: 0,
        y: desktop_height.saturating_sub(TASKBAR_HEIGHT),
        width: surface_info.width as usize,
        height: TASKBAR_HEIGHT,
    };
    let launcher_rect = Rect {
        x: 10,
        y: taskbar_rect.y + 6,
        width: LAUNCHER_WIDTH,
        height: taskbar_rect.height.saturating_sub(12),
    };

    if event.left_button && !shell.last_left_down {
        if launcher_rect.contains(event.x, event.y) {
            shell.launcher_open = !shell.launcher_open;
        } else if let Some((index, offset_x, offset_y)) =
            title_hit_test(&shell.windows, event.x, event.y)
        {
            bring_to_front(&mut shell.windows, &mut shell.active_window, index);
            shell.dragging = Some(DragState {
                window_index: shell.active_window,
                offset_x,
                offset_y,
            });
        } else if shell.launcher_open && !launcher_menu_rect(surface_info).contains(event.x, event.y) {
            shell.launcher_open = false;
        } else if let Some(index) = window_hit_test(&shell.windows, event.x, event.y) {
            bring_to_front(&mut shell.windows, &mut shell.active_window, index);
        } else {
            shell.launcher_open = false;
        }
    }

    if !event.left_button {
        shell.dragging = None;
    }

    if let Some(drag) = shell.dragging {
        let max_x = surface_info.width as usize;
        let max_y = surface_info.height as usize - TASKBAR_HEIGHT;
        let window = &mut shell.windows[drag.window_index];
        window.rect.x = event
            .x
            .saturating_sub(drag.offset_x)
            .min(max_x.saturating_sub(window.rect.width));
        window.rect.y = event
            .y
            .saturating_sub(drag.offset_y)
            .min(max_y.saturating_sub(window.rect.height));
    }

    shell.last_mouse_x = event.x;
    shell.last_mouse_y = event.y;
    shell.last_left_down = event.left_button;
}

pub fn render(tick_count: u64) {
    let mut shell = SHELL.lock();
    if tick_count == shell.last_render_tick {
        return;
    }
    shell.last_render_tick = tick_count;

    let theme = shell.theme;
    let windows = shell.windows;
    let active_window = shell.active_window;
    let launcher_open = shell.launcher_open;
    let mouse_x = shell.last_mouse_x;
    let mouse_y = shell.last_mouse_y;
    let key_name = shell.last_key_name;
    let key_unicode = shell.last_key_unicode;
    let Some(mut surface) = shell.surface.take() else {
        return;
    };
    let info = surface.info();
    let width = info.width as usize;
    let height = info.height as usize;

    draw_wallpaper(&mut surface, width, height, theme);
    draw_windows(&mut surface, &windows, active_window, theme);
    draw_taskbar(&mut surface, width, height, theme, launcher_open, tick_count);
    if launcher_open {
        draw_launcher(&mut surface, width, height, theme);
    }
    draw_status_chip(
        &mut surface,
        width,
        theme,
        &mut shell.last_status_text,
        tick_count,
        key_name,
        key_unicode,
    );
    draw_cursor(&mut surface, mouse_x, mouse_y, theme);
    shell.surface = Some(surface);
}

fn draw_wallpaper(surface: &mut FramebufferSurface, width: usize, height: usize, theme: Theme) {
    for y in 0..height {
        let mix = y as u32 * 255 / height.max(1) as u32;
        let r = lerp(theme.wallpaper_top.r, theme.wallpaper_bottom.r, mix);
        let g = lerp(theme.wallpaper_top.g, theme.wallpaper_bottom.g, mix);
        let b = lerp(theme.wallpaper_top.b, theme.wallpaper_bottom.b, mix);
        surface.fill_rect(
            Rect {
                x: 0,
                y,
                width,
                height: 1,
            },
            Color::rgb(r, g, b),
        );
    }

    surface.draw_circle(
        Point {
            x: width.saturating_sub(180),
            y: 130,
        },
        54,
        theme.wallpaper_accent,
    );
    surface.draw_line(
        Point { x: 36, y: 64 },
        Point {
            x: width.saturating_sub(42),
            y: height.saturating_sub(90),
        },
        Color::rgb(0x9F, 0xC5, 0xD7),
    );
    surface.draw_line(
        Point {
            x: width / 3,
            y: 18,
        },
        Point {
            x: width / 2 + 120,
            y: height.saturating_sub(120),
        },
        Color::rgb(0x7D, 0xB1, 0xC4),
    );
}

fn draw_windows(
    surface: &mut FramebufferSurface,
    windows: &[DesktopWindow; MAX_WINDOWS],
    active_window: usize,
    theme: Theme,
) {
    for (index, window) in windows.iter().enumerate() {
        let frame = window.rect;
        surface.fill_rect(frame, theme.window_frame);
        surface.fill_rect(
            Rect {
                x: frame.x + 2,
                y: frame.y + 2,
                width: frame.width.saturating_sub(4),
                height: frame.height.saturating_sub(4),
            },
            theme.panel,
        );

        let title_color = if index == active_window {
            theme.active_title
        } else {
            theme.inactive_title
        };
        surface.fill_rect(
            Rect {
                x: frame.x + 2,
                y: frame.y + 2,
                width: frame.width.saturating_sub(4),
                height: TITLE_BAR_HEIGHT,
            },
            title_color,
        );
        surface.draw_text(
            window.title,
            frame.x + 12,
            frame.y + 8,
            Color::rgb(0xF8, 0xFB, 0xFF),
            title_color,
        );
        surface.fill_rect(
            Rect {
                x: frame.x + frame.width.saturating_sub(32),
                y: frame.y + 7,
                width: 16,
                height: 16,
            },
            window.accent,
        );

        let body = Rect {
            x: frame.x + 6,
            y: frame.y + TITLE_BAR_HEIGHT + 4,
            width: frame.width.saturating_sub(12),
            height: frame.height.saturating_sub(TITLE_BAR_HEIGHT + 10),
        };

        match window.kind {
            WindowKind::Terminal => {
                terminal::render(surface, body, index == active_window);
            }
            WindowKind::Info | WindowKind::Roadmap => {
                let mut line_y = body.y + 12;
                for line in window.body_lines {
                    surface.draw_text(line, body.x + 12, line_y, theme.text, theme.panel);
                    line_y += 20;
                }
            }
        }
    }
}

fn draw_taskbar(
    surface: &mut FramebufferSurface,
    width: usize,
    height: usize,
    theme: Theme,
    launcher_open: bool,
    tick_count: u64,
) {
    let taskbar_y = height.saturating_sub(TASKBAR_HEIGHT);
    surface.fill_rect(
        Rect {
            x: 0,
            y: taskbar_y,
            width,
            height: TASKBAR_HEIGHT,
        },
        theme.taskbar,
    );
    surface.fill_rect(
        Rect {
            x: 0,
            y: taskbar_y,
            width,
            height: 1,
        },
        theme.taskbar_edge,
    );

    let launcher_color = if launcher_open {
        theme.launcher_active
    } else {
        theme.launcher
    };
    surface.fill_rect(
        Rect {
            x: 10,
            y: taskbar_y + 6,
            width: LAUNCHER_WIDTH,
            height: TASKBAR_HEIGHT - 12,
        },
        launcher_color,
    );
    surface.draw_text(
        "Den",
        36,
        taskbar_y + 14,
        Color::rgb(0xF6, 0xFA, 0xFF),
        launcher_color,
    );

    surface.fill_rect(
        Rect {
            x: 116,
            y: taskbar_y + 6,
            width: 136,
            height: TASKBAR_HEIGHT - 12,
        },
        Color::rgb(0x2B, 0x36, 0x45),
    );
    surface.draw_text(
        "Teddy Terminal",
        126,
        taskbar_y + 14,
        Color::rgb(0xF0, 0xF5, 0xFA),
        Color::rgb(0x2B, 0x36, 0x45),
    );

    surface.fill_rect(
        Rect {
            x: width.saturating_sub(CLOCK_WIDTH),
            y: taskbar_y + 6,
            width: CLOCK_WIDTH - 10,
            height: TASKBAR_HEIGHT - 12,
        },
        Color::rgb(0x2B, 0x36, 0x45),
    );

    let mut clock_text = FixedString::new();
    let total_seconds = tick_count / timer::snapshot().frequency_hz as u64;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 3600) % 24;
    let seconds = total_seconds % 60;
    let _ = write!(clock_text, "{:02}:{:02}:{:02}", hours, minutes, seconds);
    surface.draw_text(
        clock_text.as_str(),
        width.saturating_sub(CLOCK_WIDTH) + 16,
        taskbar_y + 14,
        theme.clock,
        Color::rgb(0x2B, 0x36, 0x45),
    );
}

fn draw_launcher(surface: &mut FramebufferSurface, _width: usize, height: usize, theme: Theme) {
    let menu = Rect {
        x: 12,
        y: height.saturating_sub(TASKBAR_HEIGHT + MENU_HEIGHT + 8),
        width: MENU_WIDTH,
        height: MENU_HEIGHT,
    };
    surface.fill_rect(menu, Color::rgb(0xF0, 0xF4, 0xF9));
    surface.stroke_rect(menu, theme.window_frame);
    surface.fill_rect(
        Rect {
            x: menu.x,
            y: menu.y,
            width: menu.width,
            height: 34,
        },
        theme.active_title,
    );
    surface.draw_text(
        "Teddy-OS",
        menu.x + 14,
        menu.y + 10,
        Color::rgb(0xFA, 0xFD, 0xFF),
        theme.active_title,
    );

    let items = ["Terminal  Ready", "Files  Phase 6", "Updater  Phase 7", "Settings  Later"];
    let mut y = menu.y + 48;
    for item in items {
        surface.fill_rect(
            Rect {
                x: menu.x + 10,
                y: y - 4,
                width: menu.width - 20,
                height: 26,
            },
            Color::rgb(0xE3, 0xEB, 0xF2),
        );
        surface.draw_text(item, menu.x + 18, y, theme.text, Color::rgb(0xE3, 0xEB, 0xF2));
        y += 28;
    }
}

fn draw_status_chip(
    surface: &mut FramebufferSurface,
    width: usize,
    theme: Theme,
    scratch: &mut FixedString,
    tick_count: u64,
    key_name: &'static str,
    key_unicode: Option<char>,
) {
    let panel = Rect {
        x: width.saturating_sub(248),
        y: 18,
        width: 220,
        height: 104,
    };
    surface.fill_rect(panel, Color::rgb(0xEC, 0xF1, 0xF6));
    surface.stroke_rect(panel, theme.window_frame);
    surface.draw_text(
        "Session",
        panel.x + 14,
        panel.y + 12,
        theme.text,
        Color::rgb(0xEC, 0xF1, 0xF6),
    );

    scratch.clear();
    let _ = write!(scratch, "Ticks {}", tick_count);
    surface.draw_text(
        scratch.as_str(),
        panel.x + 14,
        panel.y + 34,
        theme.muted_text,
        Color::rgb(0xEC, 0xF1, 0xF6),
    );

    let mouse = mouse_snapshot();
    scratch.clear();
    let _ = write!(scratch, "Pointer {}, {}", mouse.x, mouse.y);
    surface.draw_text(
        scratch.as_str(),
        panel.x + 14,
        panel.y + 54,
        theme.muted_text,
        Color::rgb(0xEC, 0xF1, 0xF6),
    );

    scratch.clear();
    let glyph = key_unicode.unwrap_or('-');
    let _ = write!(scratch, "Last key {} {}", key_name, glyph);
    surface.draw_text(
        scratch.as_str(),
        panel.x + 14,
        panel.y + 74,
        theme.muted_text,
        Color::rgb(0xEC, 0xF1, 0xF6),
    );

    scratch.clear();
    let _ = write!(
        scratch,
        "Mouse {}",
        if mouse.left_button { "dragging" } else { "ready" }
    );
    surface.draw_text(
        scratch.as_str(),
        panel.x + 14,
        panel.y + 94,
        theme.muted_text,
        Color::rgb(0xEC, 0xF1, 0xF6),
    );
}

fn draw_cursor(surface: &mut FramebufferSurface, x: usize, y: usize, theme: Theme) {
    surface.draw_line(
        Point { x, y },
        Point {
            x: x.saturating_add(10),
            y: y.saturating_add(18),
        },
        theme.cursor,
    );
    surface.draw_line(
        Point { x, y },
        Point {
            x,
            y: y.saturating_add(18),
        },
        theme.cursor,
    );
    surface.draw_line(
        Point { x, y },
        Point {
            x: x.saturating_add(12),
            y,
        },
        theme.cursor,
    );
}

fn title_hit_test(
    windows: &[DesktopWindow; MAX_WINDOWS],
    x: usize,
    y: usize,
) -> Option<(usize, usize, usize)> {
    for index in (0..MAX_WINDOWS).rev() {
        let window = windows[index];
        let title = Rect {
            x: window.rect.x,
            y: window.rect.y,
            width: window.rect.width,
            height: TITLE_BAR_HEIGHT + 2,
        };
        if title.contains(x, y) {
            return Some((index, x.saturating_sub(window.rect.x), y.saturating_sub(window.rect.y)));
        }
    }
    None
}

fn window_hit_test(windows: &[DesktopWindow; MAX_WINDOWS], x: usize, y: usize) -> Option<usize> {
    for index in (0..MAX_WINDOWS).rev() {
        if windows[index].rect.contains(x, y) {
            return Some(index);
        }
    }
    None
}

fn bring_to_front(
    windows: &mut [DesktopWindow; MAX_WINDOWS],
    active_window: &mut usize,
    index: usize,
) {
    if index >= MAX_WINDOWS.saturating_sub(1) {
        *active_window = index;
        return;
    }

    let selected = windows[index];
    let mut current = index;
    while current + 1 < MAX_WINDOWS {
        windows[current] = windows[current + 1];
        current += 1;
    }
    windows[MAX_WINDOWS - 1] = selected;
    *active_window = MAX_WINDOWS - 1;
}

fn launcher_menu_rect(info: FramebufferInfo) -> Rect {
    Rect {
        x: 12,
        y: info.height as usize - (TASKBAR_HEIGHT + MENU_HEIGHT + 8),
        width: MENU_WIDTH,
        height: MENU_HEIGHT,
    }
}

fn lerp(a: u8, b: u8, mix: u32) -> u8 {
    let inv = 255 - mix;
    (((a as u32) * inv + (b as u32) * mix) / 255) as u8
}

struct FixedString {
    buffer: [u8; 96],
    len: usize,
}

impl FixedString {
    const fn new() -> Self {
        Self {
            buffer: [0; 96],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.len = 0;
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
