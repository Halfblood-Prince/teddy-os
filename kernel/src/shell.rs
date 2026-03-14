use core::fmt::{self, Write};

use spin::Mutex;
use teddy_boot_proto::FramebufferInfo;

use crate::{
    file_explorer,
    framebuffer::{Color, FramebufferSurface, Point, Rect},
    input::{KeyKind, KeyboardEvent, MouseEvent, mouse_snapshot},
    terminal,
    timer,
};

const MAX_WINDOWS: usize = 4;
const TASKBAR_HEIGHT: usize = 42;
const LAUNCHER_WIDTH: usize = 90;
const CLOCK_WIDTH: usize = 112;
const TITLE_BAR_HEIGHT: usize = 28;
const MENU_WIDTH: usize = 220;
const MENU_HEIGHT: usize = 162;
const CONTROL_SIZE: usize = 16;
const CONTROL_GAP: usize = 6;
const RESIZE_GRIP: usize = 14;
const MIN_WINDOW_WIDTH: usize = 220;
const MIN_WINDOW_HEIGHT: usize = 140;
const TASKBAR_BUTTON_WIDTH: usize = 126;
const TASKBAR_BUTTON_START_X: usize = 116;
const TASKBAR_BUTTON_GAP: usize = 8;

#[derive(Clone, Copy)]
enum WindowKind {
    Terminal,
    Explorer,
    Info,
    Roadmap,
}

#[derive(Clone, Copy)]
enum WindowControl {
    Minimize,
    Maximize,
    Close,
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
    close_button: Color,
    maximize_button: Color,
    minimize_button: Color,
    resize_handle: Color,
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
            close_button: Color::rgb(0xC7, 0x52, 0x52),
            maximize_button: Color::rgb(0x4F, 0x8D, 0x62),
            minimize_button: Color::rgb(0xD1, 0x97, 0x42),
            resize_handle: Color::rgb(0x8F, 0xA2, 0xB7),
        }
    }
}

#[derive(Clone, Copy)]
struct DesktopWindow {
    title: &'static str,
    taskbar_label: &'static str,
    body_lines: [&'static str; 4],
    rect: Rect,
    restore_rect: Rect,
    accent: Color,
    kind: WindowKind,
    minimized: bool,
    maximized: bool,
    visible: bool,
}

#[derive(Clone, Copy)]
struct DragState {
    window_index: usize,
    offset_x: usize,
    offset_y: usize,
}

#[derive(Clone, Copy)]
struct ResizeState {
    window_index: usize,
    anchor_x: usize,
    anchor_y: usize,
    start_rect: Rect,
}

#[derive(Clone, Copy)]
enum InteractionState {
    Drag(DragState),
    Resize(ResizeState),
}

#[derive(Clone, Copy)]
enum DesktopEvent {
    ToggleLauncher,
    ClearLauncher,
    CycleFocus,
    FocusWindow(usize),
    BeginDrag(DragState),
    BeginResize(ResizeState),
    WindowControl { window_index: usize, control: WindowControl },
    BodyClick { window_index: usize, x: usize, y: usize },
    TaskbarToggle(usize),
    LauncherOpen(usize),
    None,
}

struct ShellState {
    surface: Option<FramebufferSurface>,
    theme: Theme,
    windows: [DesktopWindow; MAX_WINDOWS],
    active_window: Option<usize>,
    interaction: Option<InteractionState>,
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
                    taskbar_label: "System",
                    body_lines: [
                        "Kernel MVP services online",
                        "Input dispatch routed by shell",
                        "Window controls are active",
                        "VMware remains the primary target",
                    ],
                    rect: Rect { x: 548, y: 72, width: 244, height: 162 },
                    restore_rect: Rect { x: 548, y: 72, width: 244, height: 162 },
                    accent: Color::rgb(0x69, 0x57, 0x8E),
                    kind: WindowKind::Info,
                    minimized: false,
                    maximized: false,
                    visible: true,
                },
                DesktopWindow {
                    title: "Roadmap",
                    taskbar_label: "Roadmap",
                    body_lines: [
                        "Phase 11: windowing polish complete",
                        "Phase 12: app framework",
                        "Phase 13: package manager",
                        "Phase 14: storage tools",
                    ],
                    rect: Rect { x: 560, y: 252, width: 238, height: 148 },
                    restore_rect: Rect { x: 560, y: 252, width: 238, height: 148 },
                    accent: Color::rgb(0x7B, 0x5B, 0x26),
                    kind: WindowKind::Roadmap,
                    minimized: false,
                    maximized: false,
                    visible: true,
                },
                DesktopWindow {
                    title: "Teddy Explorer",
                    taskbar_label: "Explorer",
                    body_lines: ["", "", "", ""],
                    rect: Rect { x: 42, y: 54, width: 500, height: 342 },
                    restore_rect: Rect { x: 42, y: 54, width: 500, height: 342 },
                    accent: Color::rgb(0x5D, 0x79, 0xB2),
                    kind: WindowKind::Explorer,
                    minimized: false,
                    maximized: false,
                    visible: true,
                },
                DesktopWindow {
                    title: "Teddy Terminal",
                    taskbar_label: "Terminal",
                    body_lines: ["", "", "", ""],
                    rect: Rect { x: 98, y: 122, width: 470, height: 262 },
                    restore_rect: Rect { x: 98, y: 122, width: 470, height: 262 },
                    accent: Color::rgb(0x2F, 0x7C, 0x55),
                    kind: WindowKind::Terminal,
                    minimized: false,
                    maximized: false,
                    visible: true,
                },
            ],
            active_window: Some(3),
            interaction: None,
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
    if !event.pressed {
        return;
    }
    if handle_keyboard_shortcut(&mut shell, event) {
        return;
    }
    if let Some(index) = shell.active_window {
        dispatch_keyboard_to_window(shell.windows[index].kind, event);
    }
}

pub fn handle_mouse_event(event: MouseEvent) {
    let mut shell = SHELL.lock();
    let Some(surface_info) = shell.surface.as_ref().map(|surface| surface.info()) else {
        return;
    };
    let desktop = desktop_rect(surface_info);
    let taskbar = taskbar_rect(surface_info);
    let launcher_button = launcher_button_rect(taskbar);

    if event.left_button && !shell.last_left_down {
        let desktop_event = if launcher_button.contains(event.x, event.y) {
            DesktopEvent::ToggleLauncher
        } else if shell.launcher_open {
            launcher_hit_test(surface_info, event.x, event.y)
        } else if let Some(index) = taskbar_button_hit_test(event.x, event.y, taskbar) {
            DesktopEvent::TaskbarToggle(index)
        } else {
            hit_test_windows(&shell.windows, event.x, event.y, desktop)
        };
        apply_desktop_event(&mut shell, desktop_event, surface_info, timer::ticks());
    }

    if !event.left_button {
        shell.interaction = None;
    }

    if event.left_button {
        match shell.interaction {
            Some(InteractionState::Drag(state)) => {
                drag_window(&mut shell.windows[state.window_index], state, desktop, event.x, event.y);
            }
            Some(InteractionState::Resize(state)) => {
                resize_window(&mut shell.windows[state.window_index], state, event.x, event.y, desktop);
            }
            None => {}
        }
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
    draw_taskbar(&mut surface, width, height, theme, launcher_open, tick_count, &windows, active_window);
    if launcher_open {
        draw_launcher(&mut surface, height, theme);
    }
    draw_status_chip(
        &mut surface,
        width,
        theme,
        &mut shell.last_status_text,
        tick_count,
        key_name,
        key_unicode,
        active_window.and_then(|index| windows.get(index).map(|window| window.taskbar_label)),
    );
    draw_cursor(&mut surface, mouse_x, mouse_y, theme);
    shell.surface = Some(surface);
}

fn handle_keyboard_shortcut(shell: &mut ShellState, event: KeyboardEvent) -> bool {
    match event.key_kind {
        KeyKind::Escape => {
            shell.launcher_open = false;
            true
        }
        KeyKind::Tab => {
            focus_next_visible_window(shell);
            true
        }
        _ => false,
    }
}

fn dispatch_keyboard_to_window(kind: WindowKind, event: KeyboardEvent) {
    if matches!(kind, WindowKind::Terminal) {
        terminal::handle_keyboard_event(event);
    }
}

fn dispatch_mouse_to_window(kind: WindowKind, frame: Rect, x: usize, y: usize, tick: u64) {
    if matches!(kind, WindowKind::Explorer) {
        let body = window_body(frame);
        if body.contains(x, y) {
            file_explorer::handle_click(x.saturating_sub(body.x), y.saturating_sub(body.y), body, tick);
        }
    }
}

fn apply_desktop_event(shell: &mut ShellState, event: DesktopEvent, info: FramebufferInfo, tick: u64) {
    match event {
        DesktopEvent::ToggleLauncher => shell.launcher_open = !shell.launcher_open,
        DesktopEvent::ClearLauncher => shell.launcher_open = false,
        DesktopEvent::CycleFocus => focus_next_visible_window(shell),
        DesktopEvent::FocusWindow(index) => focus_window(shell, index),
        DesktopEvent::BeginDrag(state) => shell.interaction = Some(InteractionState::Drag(state)),
        DesktopEvent::BeginResize(state) => shell.interaction = Some(InteractionState::Resize(state)),
        DesktopEvent::WindowControl { window_index, control } => {
            apply_window_control(shell, window_index, control, info);
            shell.launcher_open = false;
        }
        DesktopEvent::BodyClick { window_index, x, y } => {
            focus_window(shell, window_index);
            let top = shell.active_window.unwrap_or(window_index);
            dispatch_mouse_to_window(shell.windows[top].kind, shell.windows[top].rect, x, y, tick);
            shell.launcher_open = false;
        }
        DesktopEvent::TaskbarToggle(index) => {
            toggle_taskbar_window(shell, index, info);
            shell.launcher_open = false;
        }
        DesktopEvent::LauncherOpen(index) => {
            restore_window(shell, index, info);
            focus_window(shell, index);
            shell.launcher_open = false;
        }
        DesktopEvent::None => shell.launcher_open = false,
    }
}

fn apply_window_control(shell: &mut ShellState, index: usize, control: WindowControl, info: FramebufferInfo) {
    match control {
        WindowControl::Minimize => {
            shell.windows[index].minimized = true;
            if shell.active_window == Some(index) {
                shell.active_window = next_visible_window(&shell.windows, index);
            }
        }
        WindowControl::Maximize => {
            let desktop = desktop_rect(info);
            let window = &mut shell.windows[index];
            if window.maximized {
                window.rect = window.restore_rect;
                window.maximized = false;
            } else {
                window.restore_rect = window.rect;
                window.rect = Rect {
                    x: desktop.x + 8,
                    y: desktop.y + 8,
                    width: desktop.width.saturating_sub(16),
                    height: desktop.height.saturating_sub(16),
                };
                window.maximized = true;
                window.minimized = false;
            }
            focus_window(shell, index);
        }
        WindowControl::Close => {
            shell.windows[index].visible = false;
            shell.windows[index].minimized = true;
            if shell.active_window == Some(index) {
                shell.active_window = next_any_visible_window(&shell.windows);
            }
        }
    }
}

fn toggle_taskbar_window(shell: &mut ShellState, index: usize, info: FramebufferInfo) {
    if !shell.windows[index].visible || shell.windows[index].minimized {
        restore_window(shell, index, info);
        focus_window(shell, index);
    } else if shell.active_window == Some(index) {
        shell.windows[index].minimized = true;
        shell.active_window = next_visible_window(&shell.windows, index);
    } else {
        focus_window(shell, index);
    }
}

fn restore_window(shell: &mut ShellState, index: usize, info: FramebufferInfo) {
    let desktop = desktop_rect(info);
    let window = &mut shell.windows[index];
    window.visible = true;
    window.minimized = false;
    if window.maximized {
        window.rect = Rect {
            x: desktop.x + 8,
            y: desktop.y + 8,
            width: desktop.width.saturating_sub(16),
            height: desktop.height.saturating_sub(16),
        };
    } else {
        window.rect = window.restore_rect;
    }
}

fn focus_window(shell: &mut ShellState, index: usize) {
    if index >= MAX_WINDOWS || !shell.windows[index].visible {
        return;
    }
    shell.windows[index].minimized = false;
    bring_to_front(&mut shell.windows, index);
    shell.active_window = Some(MAX_WINDOWS - 1);
}

fn focus_next_visible_window(shell: &mut ShellState) {
    let start = shell.active_window.unwrap_or(MAX_WINDOWS - 1);
    if let Some(index) = next_visible_window(&shell.windows, start) {
        focus_window(shell, index);
    }
}

fn next_visible_window(windows: &[DesktopWindow; MAX_WINDOWS], current: usize) -> Option<usize> {
    for offset in 1..=MAX_WINDOWS {
        let index = (current + MAX_WINDOWS - offset) % MAX_WINDOWS;
        let window = windows[index];
        if window.visible && !window.minimized {
            return Some(index);
        }
    }
    None
}

fn next_any_visible_window(windows: &[DesktopWindow; MAX_WINDOWS]) -> Option<usize> {
    for index in (0..MAX_WINDOWS).rev() {
        let window = windows[index];
        if window.visible && !window.minimized {
            return Some(index);
        }
    }
    None
}

fn drag_window(window: &mut DesktopWindow, state: DragState, desktop: Rect, x: usize, y: usize) {
    if window.maximized {
        return;
    }
    window.rect.x = x
        .saturating_sub(state.offset_x)
        .clamp(desktop.x, desktop.x + desktop.width.saturating_sub(window.rect.width));
    window.rect.y = y
        .saturating_sub(state.offset_y)
        .clamp(desktop.y, desktop.y + desktop.height.saturating_sub(window.rect.height));
    window.restore_rect = window.rect;
}

fn resize_window(window: &mut DesktopWindow, state: ResizeState, x: usize, y: usize, desktop: Rect) {
    if window.maximized {
        return;
    }
    let delta_x = x.saturating_sub(state.anchor_x);
    let delta_y = y.saturating_sub(state.anchor_y);
    let max_width = desktop.width.saturating_sub(state.start_rect.x.saturating_sub(desktop.x));
    let max_height = desktop.height.saturating_sub(state.start_rect.y.saturating_sub(desktop.y));
    window.rect.width = state.start_rect.width.saturating_add(delta_x).clamp(MIN_WINDOW_WIDTH, max_width);
    window.rect.height = state.start_rect.height.saturating_add(delta_y).clamp(MIN_WINDOW_HEIGHT, max_height);
    window.restore_rect = window.rect;
}

fn draw_wallpaper(surface: &mut FramebufferSurface, width: usize, height: usize, theme: Theme) {
    for y in 0..height {
        let mix = y as u32 * 255 / height.max(1) as u32;
        let r = lerp(theme.wallpaper_top.r, theme.wallpaper_bottom.r, mix);
        let g = lerp(theme.wallpaper_top.g, theme.wallpaper_bottom.g, mix);
        let b = lerp(theme.wallpaper_top.b, theme.wallpaper_bottom.b, mix);
        surface.fill_rect(Rect { x: 0, y, width, height: 1 }, Color::rgb(r, g, b));
    }
    surface.draw_circle(Point { x: width.saturating_sub(180), y: 130 }, 54, theme.wallpaper_accent);
    surface.draw_line(
        Point { x: 36, y: 64 },
        Point { x: width.saturating_sub(42), y: height.saturating_sub(90) },
        Color::rgb(0x9F, 0xC5, 0xD7),
    );
    surface.draw_line(
        Point { x: width / 3, y: 18 },
        Point { x: width / 2 + 120, y: height.saturating_sub(120) },
        Color::rgb(0x7D, 0xB1, 0xC4),
    );
}

fn draw_windows(surface: &mut FramebufferSurface, windows: &[DesktopWindow; MAX_WINDOWS], active: Option<usize>, theme: Theme) {
    for (index, window) in windows.iter().enumerate() {
        if !window.visible || window.minimized {
            continue;
        }
        let frame = window.rect;
        surface.fill_rect(frame, theme.window_frame);
        surface.fill_rect(
            Rect { x: frame.x + 2, y: frame.y + 2, width: frame.width.saturating_sub(4), height: frame.height.saturating_sub(4) },
            theme.panel,
        );
        let title_color = if active == Some(index) { theme.active_title } else { theme.inactive_title };
        let title_rect = title_bar_rect(frame);
        surface.fill_rect(title_rect, title_color);
        surface.draw_text(window.title, frame.x + 12, frame.y + 8, Color::rgb(0xF8, 0xFB, 0xFF), title_color);
        draw_window_controls(surface, frame, theme);
        let body = window_body(frame);
        match window.kind {
            WindowKind::Terminal => terminal::render(surface, body, active == Some(index)),
            WindowKind::Explorer => file_explorer::render(surface, body, active == Some(index)),
            WindowKind::Info | WindowKind::Roadmap => {
                let mut line_y = body.y + 12;
                for line in window.body_lines {
                    surface.draw_text(line, body.x + 12, line_y, theme.text, theme.panel);
                    line_y += 20;
                }
            }
        }
        draw_resize_handle(surface, frame, theme);
    }
}

fn draw_window_controls(surface: &mut FramebufferSurface, frame: Rect, theme: Theme) {
    for control in [WindowControl::Minimize, WindowControl::Maximize, WindowControl::Close] {
        let rect = control_button_rect(frame, control);
        let color = match control {
            WindowControl::Minimize => theme.minimize_button,
            WindowControl::Maximize => theme.maximize_button,
            WindowControl::Close => theme.close_button,
        };
        surface.fill_rect(rect, color);
    }
}

fn draw_resize_handle(surface: &mut FramebufferSurface, frame: Rect, theme: Theme) {
    let handle = resize_handle_rect(frame);
    let mut x = handle.x;
    let mut y = handle.y + handle.height.saturating_sub(1);
    for _ in 0..3 {
        surface.draw_line(Point { x, y }, Point { x: x + 4, y }, theme.resize_handle);
        x += 3;
        y = y.saturating_sub(3);
    }
}

fn draw_taskbar(
    surface: &mut FramebufferSurface,
    width: usize,
    height: usize,
    theme: Theme,
    launcher_open: bool,
    tick_count: u64,
    windows: &[DesktopWindow; MAX_WINDOWS],
    active: Option<usize>,
) {
    let taskbar = Rect { x: 0, y: height.saturating_sub(TASKBAR_HEIGHT), width, height: TASKBAR_HEIGHT };
    surface.fill_rect(taskbar, theme.taskbar);
    surface.fill_rect(Rect { x: 0, y: taskbar.y, width, height: 1 }, theme.taskbar_edge);
    let launcher_color = if launcher_open { theme.launcher_active } else { theme.launcher };
    let launcher = launcher_button_rect(taskbar);
    surface.fill_rect(launcher, launcher_color);
    surface.draw_text("Den", launcher.x + 26, launcher.y + 8, Color::rgb(0xF6, 0xFA, 0xFF), launcher_color);

    let mut x = TASKBAR_BUTTON_START_X;
    for (index, window) in windows.iter().enumerate() {
        let button_bg = if !window.visible {
            Color::rgb(0x3B, 0x2C, 0x2C)
        } else if window.minimized {
            Color::rgb(0x2A, 0x31, 0x3A)
        } else if active == Some(index) {
            Color::rgb(0x3A, 0x4B, 0x5F)
        } else {
            Color::rgb(0x2B, 0x36, 0x45)
        };
        surface.fill_rect(Rect { x, y: taskbar.y + 6, width: TASKBAR_BUTTON_WIDTH, height: TASKBAR_HEIGHT - 12 }, button_bg);
        surface.draw_text(window.taskbar_label, x + 12, taskbar.y + 14, Color::rgb(0xF0, 0xF5, 0xFA), button_bg);
        x += TASKBAR_BUTTON_WIDTH + TASKBAR_BUTTON_GAP;
    }

    surface.fill_rect(
        Rect { x: width.saturating_sub(CLOCK_WIDTH), y: taskbar.y + 6, width: CLOCK_WIDTH - 10, height: TASKBAR_HEIGHT - 12 },
        Color::rgb(0x2B, 0x36, 0x45),
    );
    let mut clock_text = FixedString::new();
    let total_seconds = tick_count / timer::snapshot().frequency_hz as u64;
    let _ = write!(
        clock_text,
        "{:02}:{:02}:{:02}",
        (total_seconds / 3600) % 24,
        (total_seconds / 60) % 60,
        total_seconds % 60
    );
    surface.draw_text(clock_text.as_str(), width.saturating_sub(CLOCK_WIDTH) + 16, taskbar.y + 14, theme.clock, Color::rgb(0x2B, 0x36, 0x45));
}

fn draw_launcher(surface: &mut FramebufferSurface, height: usize, theme: Theme) {
    let menu = Rect { x: 12, y: height.saturating_sub(TASKBAR_HEIGHT + MENU_HEIGHT + 8), width: MENU_WIDTH, height: MENU_HEIGHT };
    surface.fill_rect(menu, Color::rgb(0xF0, 0xF4, 0xF9));
    surface.stroke_rect(menu, theme.window_frame);
    surface.fill_rect(Rect { x: menu.x, y: menu.y, width: menu.width, height: 34 }, theme.active_title);
    surface.draw_text("Teddy-OS", menu.x + 14, menu.y + 10, Color::rgb(0xFA, 0xFD, 0xFF), theme.active_title);

    let items = ["Explorer", "Terminal", "System", "Roadmap"];
    let mut y = menu.y + 48;
    for item in items {
        surface.fill_rect(Rect { x: menu.x + 10, y: y - 4, width: menu.width - 20, height: 26 }, Color::rgb(0xE3, 0xEB, 0xF2));
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
    active_label: Option<&'static str>,
) {
    let panel = Rect { x: width.saturating_sub(248), y: 18, width: 220, height: 122 };
    surface.fill_rect(panel, Color::rgb(0xEC, 0xF1, 0xF6));
    surface.stroke_rect(panel, theme.window_frame);
    surface.draw_text("Session", panel.x + 14, panel.y + 12, theme.text, Color::rgb(0xEC, 0xF1, 0xF6));

    scratch.clear();
    let _ = write!(scratch, "Ticks {}", tick_count);
    surface.draw_text(scratch.as_str(), panel.x + 14, panel.y + 34, theme.muted_text, Color::rgb(0xEC, 0xF1, 0xF6));

    let mouse = mouse_snapshot();
    scratch.clear();
    let _ = write!(scratch, "Pointer {}, {}", mouse.x, mouse.y);
    surface.draw_text(scratch.as_str(), panel.x + 14, panel.y + 54, theme.muted_text, Color::rgb(0xEC, 0xF1, 0xF6));

    scratch.clear();
    let _ = write!(scratch, "Last key {} {}", key_name, key_unicode.unwrap_or('-'));
    surface.draw_text(scratch.as_str(), panel.x + 14, panel.y + 74, theme.muted_text, Color::rgb(0xEC, 0xF1, 0xF6));

    scratch.clear();
    let _ = write!(scratch, "Focus {}", active_label.unwrap_or("None"));
    surface.draw_text(scratch.as_str(), panel.x + 14, panel.y + 94, theme.muted_text, Color::rgb(0xEC, 0xF1, 0xF6));

    scratch.clear();
    let _ = write!(scratch, "Mouse {}", if mouse.left_button { "held" } else { "ready" });
    surface.draw_text(scratch.as_str(), panel.x + 14, panel.y + 112, theme.muted_text, Color::rgb(0xEC, 0xF1, 0xF6));
}

fn draw_cursor(surface: &mut FramebufferSurface, x: usize, y: usize, theme: Theme) {
    surface.draw_line(Point { x, y }, Point { x: x.saturating_add(10), y: y.saturating_add(18) }, theme.cursor);
    surface.draw_line(Point { x, y }, Point { x, y: y.saturating_add(18) }, theme.cursor);
    surface.draw_line(Point { x, y }, Point { x: x.saturating_add(12), y }, theme.cursor);
}

fn hit_test_windows(windows: &[DesktopWindow; MAX_WINDOWS], x: usize, y: usize, desktop: Rect) -> DesktopEvent {
    for index in (0..MAX_WINDOWS).rev() {
        let window = windows[index];
        if !window.visible || window.minimized || !window.rect.contains(x, y) {
            continue;
        }
        if let Some(control) = control_hit_test(window.rect, x, y) {
            return DesktopEvent::WindowControl { window_index: index, control };
        }
        if resize_handle_rect(window.rect).contains(x, y) && !window.maximized {
            return DesktopEvent::BeginResize(ResizeState { window_index: index, anchor_x: x, anchor_y: y, start_rect: window.rect });
        }
        if title_bar_rect(window.rect).contains(x, y) {
            return DesktopEvent::BeginDrag(DragState {
                window_index: index,
                offset_x: x.saturating_sub(window.rect.x),
                offset_y: y.saturating_sub(window.rect.y),
            });
        }
        if desktop.contains(x, y) {
            return DesktopEvent::BodyClick { window_index: index, x, y };
        }
    }
    DesktopEvent::ClearLauncher
}

fn launcher_hit_test(info: FramebufferInfo, x: usize, y: usize) -> DesktopEvent {
    let menu = launcher_menu_rect(info);
    if !menu.contains(x, y) {
        return DesktopEvent::ClearLauncher;
    }
    let row = (y.saturating_sub(menu.y + 44)) / 28;
    match row {
        0 => DesktopEvent::LauncherOpen(2),
        1 => DesktopEvent::LauncherOpen(3),
        2 => DesktopEvent::LauncherOpen(0),
        3 => DesktopEvent::LauncherOpen(1),
        _ => DesktopEvent::None,
    }
}

fn taskbar_button_hit_test(x: usize, y: usize, taskbar: Rect) -> Option<usize> {
    let mut button_x = TASKBAR_BUTTON_START_X;
    for index in 0..MAX_WINDOWS {
        let rect = Rect { x: button_x, y: taskbar.y + 6, width: TASKBAR_BUTTON_WIDTH, height: TASKBAR_HEIGHT - 12 };
        if rect.contains(x, y) {
            return Some(index);
        }
        button_x += TASKBAR_BUTTON_WIDTH + TASKBAR_BUTTON_GAP;
    }
    None
}

fn control_hit_test(frame: Rect, x: usize, y: usize) -> Option<WindowControl> {
    for control in [WindowControl::Minimize, WindowControl::Maximize, WindowControl::Close] {
        if control_button_rect(frame, control).contains(x, y) {
            return Some(control);
        }
    }
    None
}

fn title_bar_rect(frame: Rect) -> Rect {
    Rect { x: frame.x + 2, y: frame.y + 2, width: frame.width.saturating_sub(4), height: TITLE_BAR_HEIGHT }
}

fn control_button_rect(frame: Rect, control: WindowControl) -> Rect {
    let close_x = frame.x + frame.width.saturating_sub(20 + CONTROL_SIZE);
    let max_x = close_x.saturating_sub(CONTROL_SIZE + CONTROL_GAP);
    let min_x = max_x.saturating_sub(CONTROL_SIZE + CONTROL_GAP);
    let x = match control {
        WindowControl::Minimize => min_x,
        WindowControl::Maximize => max_x,
        WindowControl::Close => close_x,
    };
    Rect { x, y: frame.y + 7, width: CONTROL_SIZE, height: CONTROL_SIZE }
}

fn resize_handle_rect(frame: Rect) -> Rect {
    Rect {
        x: frame.x + frame.width.saturating_sub(RESIZE_GRIP + 4),
        y: frame.y + frame.height.saturating_sub(RESIZE_GRIP + 4),
        width: RESIZE_GRIP,
        height: RESIZE_GRIP,
    }
}

fn desktop_rect(info: FramebufferInfo) -> Rect {
    Rect { x: 0, y: 0, width: info.width as usize, height: info.height as usize - TASKBAR_HEIGHT }
}

fn taskbar_rect(info: FramebufferInfo) -> Rect {
    Rect { x: 0, y: info.height as usize - TASKBAR_HEIGHT, width: info.width as usize, height: TASKBAR_HEIGHT }
}

fn launcher_button_rect(taskbar: Rect) -> Rect {
    Rect { x: 10, y: taskbar.y + 6, width: LAUNCHER_WIDTH, height: taskbar.height.saturating_sub(12) }
}

fn launcher_menu_rect(info: FramebufferInfo) -> Rect {
    Rect { x: 12, y: info.height as usize - (TASKBAR_HEIGHT + MENU_HEIGHT + 8), width: MENU_WIDTH, height: MENU_HEIGHT }
}

fn window_body(frame: Rect) -> Rect {
    Rect { x: frame.x + 6, y: frame.y + TITLE_BAR_HEIGHT + 4, width: frame.width.saturating_sub(12), height: frame.height.saturating_sub(TITLE_BAR_HEIGHT + 10) }
}

fn bring_to_front(windows: &mut [DesktopWindow; MAX_WINDOWS], index: usize) {
    if index >= MAX_WINDOWS.saturating_sub(1) {
        return;
    }
    let selected = windows[index];
    let mut current = index;
    while current + 1 < MAX_WINDOWS {
        windows[current] = windows[current + 1];
        current += 1;
    }
    windows[MAX_WINDOWS - 1] = selected;
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
        Self { buffer: [0; 96], len: 0 }
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
