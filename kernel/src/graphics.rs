use crate::{
    boot_info::{BootInfo, FramebufferInfo},
    explorer::{ExplorerAction, ExplorerApp},
    fs::FileSystem,
    input::{self, InputEvent, InputManager, MouseState},
    interrupts,
    terminal::{TerminalAction, TerminalApp},
    trace,
    writer::WriterApp,
};

mod generated_icons {
    include!(concat!(env!("OUT_DIR"), "/generated_icons.rs"));
}

const BASE_WIDTH: i32 = 320;
const BASE_HEIGHT: i32 = 200;
const TITLE_BAR_HEIGHT: i32 = 14;
const CURSOR_SIZE: usize = 16;
const DOUBLE_CLICK_TICKS: u64 = 40;
const TERMINAL_VIEW_LINES: usize = 5;
const EXPLORER_ROWS_VISIBLE: usize = 4;
const TOP_BAR_HEIGHT: i32 = 18;
const TASKBAR_HEIGHT: i32 = 18;

struct IconAsset {
    width: usize,
    height: usize,
    pixels: &'static [u8],
}

pub struct GraphicsShell {
    fb: FramebufferInfo,
    fs: FileSystem,
    terminal: TerminalApp,
    explorer: ExplorerApp,
    writer: WriterApp,
    input: InputManager,
    uptime_seconds: u64,
    accent_phase: u8,
    terminal_window: WindowRect,
    explorer_window: WindowRect,
    writer_window: WindowRect,
    settings_window: WindowRect,
    terminal_open: bool,
    explorer_open: bool,
    writer_open: bool,
    settings_open: bool,
    focused_window: Option<WindowKind>,
    selected_icon: Option<DesktopIcon>,
    drag_state: DragState,
    last_desktop_icon_click: Option<IconClickState>,
    last_explorer_click_tick: Option<u64>,
    cursor_backing: [u32; CURSOR_SIZE * CURSOR_SIZE],
    cursor_saved_x: i32,
    cursor_saved_y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DesktopIcon {
    Terminal,
    Explorer,
    Writer,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowKind {
    Terminal,
    Explorer,
    Writer,
    Settings,
}

#[derive(Clone, Copy)]
struct WindowRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy)]
struct DragState {
    active: bool,
    window: Option<WindowKind>,
    offset_x: i32,
    offset_y: i32,
}

#[derive(Clone, Copy)]
struct IconClickState {
    icon: DesktopIcon,
    tick: u64,
}

enum MouseRedraw {
    None,
    Overlay,
    Panels,
    Hud,
    Full,
}

pub enum GraphicsAction {
    Reboot,
    Shutdown,
}

impl GraphicsShell {
    pub const fn empty() -> Self {
        Self {
            fb: FramebufferInfo::empty(),
            fs: FileSystem::empty(),
            terminal: TerminalApp::empty(),
            explorer: ExplorerApp::empty(),
            writer: WriterApp::empty(),
            input: InputManager::new(0, 0),
            uptime_seconds: 0,
            accent_phase: 0,
            terminal_window: WindowRect {
                x: 70,
                y: 32,
                width: 168,
                height: 96,
            },
            explorer_window: WindowRect {
                x: 126,
                y: 46,
                width: 168,
                height: 104,
            },
            writer_window: WindowRect {
                x: 94,
                y: 38,
                width: 176,
                height: 108,
            },
            settings_window: WindowRect {
                x: 92,
                y: 58,
                width: 164,
                height: 96,
            },
            terminal_open: false,
            explorer_open: false,
            writer_open: false,
            settings_open: false,
            focused_window: None,
            selected_icon: None,
            drag_state: DragState {
                active: false,
                window: None,
                offset_x: 0,
                offset_y: 0,
            },
            last_desktop_icon_click: None,
            last_explorer_click_tick: None,
            cursor_backing: [0; CURSOR_SIZE * CURSOR_SIZE],
            cursor_saved_x: 0,
            cursor_saved_y: 0,
        }
    }

    pub fn init(&mut self, boot_info: BootInfo) -> bool {
        trace::set_boot_stage(0x90);
        let fb = match boot_info.framebuffer() {
            Some(fb) => fb,
            None => return false,
        };
        trace::set_boot_stage(0x91);
        if fb.bpp() != 8 && fb.bpp() != 24 && fb.bpp() != 32 {
            return false;
        }

        trace::set_boot_stage(0x92);
        let max_x = fb.width() as i32 - 1;
        let max_y = fb.height() as i32 - 1;
        self.fb = fb;
        trace::set_boot_stage(0x93);
        self.input.reset(max_x, max_y);
        trace::set_boot_stage(0x94);
        self.uptime_seconds = 0;
        self.accent_phase = 0;
        self.terminal_open = false;
        self.explorer_open = false;
        self.writer_open = false;
        self.settings_open = false;
        self.focused_window = None;
        self.selected_icon = None;
        self.drag_state = DragState {
            active: false,
            window: None,
            offset_x: 0,
            offset_y: 0,
        };
        self.last_desktop_icon_click = None;
        self.last_explorer_click_tick = None;
        trace::set_boot_stage(0x95);
        trace::set_boot_stage(0x96);
        self.reset_layout();
        self.fs.init_ram_only();
        trace::set_boot_stage(0x97);
        self.terminal.init();
        trace::set_boot_stage(0x98);
        trace::set_boot_stage(0x9A);
        self.explorer.init();
        trace::set_boot_stage(0x9B);
        self.writer.init();
        trace::set_boot_stage(0x9C);
        true
    }

    fn reset_layout(&mut self) {
        let s = self.ui_scale();
        let width = self.fb.width() as i32;
        let _height = self.fb.height() as i32;
        let taskbar_y = self.taskbar_y();

        self.terminal_window = WindowRect {
            x: clamp(self.sx(70), 8, width - self.sx(168) - 8),
            y: clamp(self.sy(32), self.top_bar_height() + 6, taskbar_y - self.sy(96) - 8),
            width: self.sx(168).max(220),
            height: (self.sy(96) + s * 8).max(140),
        };
        self.explorer_window = WindowRect {
            x: clamp(self.sx(126), 12, width - self.sx(168) - 12),
            y: clamp(self.sy(46), self.top_bar_height() + 12, taskbar_y - self.sy(104) - 8),
            width: (self.sx(168) + s * 24).max(260),
            height: (self.sy(104) + s * 16).max(170),
        };
        self.writer_window = WindowRect {
            x: clamp(self.sx(90), 10, width - self.sx(184) - 10),
            y: clamp(self.sy(34), self.top_bar_height() + 10, taskbar_y - self.sy(110) - 8),
            width: (self.sx(184) + s * 28).max(280),
            height: (self.sy(110) + s * 18).max(180),
        };
        self.settings_window = WindowRect {
            x: clamp(self.sx(92), 12, width - self.sx(164) - 12),
            y: clamp(self.sy(58), self.top_bar_height() + 12, taskbar_y - self.sy(96) - 8),
            width: (self.sx(164) + s * 12).max(240),
            height: (self.sy(96) + s * 10).max(150),
        };
    }

    pub fn render(&mut self) {
        self.draw_background();
        self.draw_top_bar();
        self.draw_desktop_icons();
        self.draw_windows();
        self.draw_taskbar();
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    pub fn tick(&mut self, uptime_seconds: u64) {
        if self.uptime_seconds != uptime_seconds {
            self.uptime_seconds = uptime_seconds;
            self.accent_phase = self.accent_phase.wrapping_add(1) % 3;
            self.redraw_hud();
        }
    }

    pub fn handle_key(&mut self, ascii: u8) -> Option<GraphicsAction> {
        if self.terminal_open && self.focused_window == Some(WindowKind::Terminal) {
            let action = self.terminal.handle_key(ascii, &mut self.fs);
            if matches!(ascii, 8 | 0x20..=0x7E) {
                self.redraw_terminal_input_strip();
            } else {
                self.redraw_window(WindowKind::Terminal);
            }
            return match action {
                TerminalAction::None => None,
                TerminalAction::Reboot => Some(GraphicsAction::Reboot),
                TerminalAction::Shutdown => Some(GraphicsAction::Shutdown),
            };
        }

        if self.explorer_open && self.focused_window == Some(WindowKind::Explorer) {
            let action = self.explorer.handle_key(ascii, &mut self.fs);
            self.handle_explorer_action(action);
            return None;
        }

        if self.writer_open && self.focused_window == Some(WindowKind::Writer) {
            if self.writer.handle_key(ascii) {
                self.redraw_window(WindowKind::Writer);
            }
            return None;
        }

        if ascii != b'?' {
            self.accent_phase = self.accent_phase.wrapping_add(1) % 3;
            self.redraw_hud();
        }
        None
    }

    pub fn poll_input(&mut self) {
        let previous_mouse = self.input.mouse_state();
        if !self.input.pump_hardware() {
            return;
        }

        let current_mouse = self.input.mouse_state();
        let mut redraw = if mouse_changed(previous_mouse, current_mouse) {
            MouseRedraw::Overlay
        } else {
            MouseRedraw::None
        };

        while let Some(event) = self.input.next_event() {
            let event_redraw = match event {
                InputEvent::MouseMove(state) => self.handle_mouse_move(state),
                InputEvent::MouseDown(state, button) => self.handle_mouse_down(state, button),
                InputEvent::MouseUp(state, button) => self.handle_mouse_up(state, button),
            };
            redraw = combine_redraw(redraw, event_redraw);
        }

        match redraw {
            MouseRedraw::None => {}
            MouseRedraw::Overlay => self.refresh_cursor_overlay(previous_mouse),
            MouseRedraw::Hud => self.redraw_hud(),
            MouseRedraw::Panels => self.redraw_panels(),
            MouseRedraw::Full => {
                self.restore_cursor_backing();
                self.render();
            }
        }
    }

    fn handle_mouse_move(&mut self, state: MouseState) -> MouseRedraw {
        if !self.drag_state.active || state.buttons & input::MOUSE_BUTTON_LEFT == 0 {
            return MouseRedraw::Overlay;
        }

        let window = match self.drag_state.window {
            Some(window) => window,
            None => return MouseRedraw::Overlay,
        };

        let bounds = self.window_bounds(window);
        let max_x = self.fb.width() as i32 - bounds.width - 1;
        let max_y = self.taskbar_y() - bounds.height - 2;
        let next_x = clamp(state.x - self.drag_state.offset_x, 0, max_x);
        let next_y = clamp(state.y - self.drag_state.offset_y, 20, max_y);

        let old_rect = self.window_bounds(window);
        let rect = self.window_rect_mut(window);
        rect.x = next_x;
        rect.y = next_y;
        let new_rect = *rect;
        self.redraw_window_move(old_rect, new_rect);
        MouseRedraw::None
    }

    fn handle_mouse_down(&mut self, state: MouseState, button: u8) -> MouseRedraw {
        if button != input::MOUSE_BUTTON_LEFT {
            return MouseRedraw::Overlay;
        }

        if let Some(window) = self.hit_close_button(state.x, state.y) {
            let closed_rect = self.window_bounds(window);
            self.close_window(window);
            self.redraw_region(window_to_region(closed_rect));
            return MouseRedraw::None;
        }

        if let Some(window) = self.hit_title_bar(state.x, state.y) {
            let old_focus = self.focused_window;
            self.focus_window(window);
            let rect = self.window_bounds(window);
            self.drag_state.active = true;
            self.drag_state.window = Some(window);
            self.drag_state.offset_x = state.x - rect.x;
            self.drag_state.offset_y = state.y - rect.y;
            self.redraw_focus_change(old_focus, Some(window));
            return MouseRedraw::None;
        }

        if let Some(redraw) = self.handle_window_client_click(state.x, state.y) {
            return redraw;
        }

        if let Some(window) = self.hit_window(state.x, state.y) {
            let old_focus = self.focused_window;
            self.focus_window(window);
            self.selected_icon = None;
            self.redraw_focus_change(old_focus, Some(window));
            return MouseRedraw::None;
        }

        if let Some(icon) = self.hit_taskbar_button(state.x, state.y) {
            let old_focus = self.focused_window;
            let opened = self.icon_is_open(icon);
            self.toggle_or_focus(icon);
            if opened != self.icon_is_open(icon) {
                self.redraw_panels();
                return MouseRedraw::None;
            }
            self.redraw_focus_change(old_focus, self.focused_window);
            self.redraw_hud();
            return MouseRedraw::None;
        }

        if let Some(icon) = self.hit_icon(state.x, state.y) {
            let now = interrupts::timer_ticks();
            let mut should_open = false;
            if let Some(last_click) = self.last_desktop_icon_click {
                if last_click.icon == icon && now.saturating_sub(last_click.tick) <= DOUBLE_CLICK_TICKS {
                    should_open = true;
                }
            }
            self.last_desktop_icon_click = Some(IconClickState { icon, tick: now });
            self.selected_icon = Some(icon);
            if should_open {
                self.open_window_for_icon(icon);
                self.redraw_panels();
                return MouseRedraw::None;
            }
            self.redraw_icon_strip();
            return MouseRedraw::None;
        }

        self.selected_icon = None;
        self.focused_window = None;
        self.redraw_panels();
        MouseRedraw::None
    }

    fn handle_mouse_up(&mut self, _state: MouseState, button: u8) -> MouseRedraw {
        if button == input::MOUSE_BUTTON_LEFT {
            let was_dragging = self.drag_state.active;
            self.drag_state.active = false;
            self.drag_state.window = None;
            if was_dragging {
                self.redraw_hud();
                return MouseRedraw::None;
            }
        }
        MouseRedraw::Overlay
    }

    fn refresh_cursor_overlay(&mut self, _previous_mouse: MouseState) {
        self.restore_cursor_backing();
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    fn redraw_hud(&mut self) {
        self.redraw_region(Rect {
            x: 0,
            y: self.taskbar_y() - self.sy(6),
            width: self.fb.width() as i32,
            height: self.taskbar_height() + self.sy(6),
        });
    }

    fn redraw_panels(&mut self) {
        let mut rects = [Rect { x: 0, y: 0, width: 0, height: 0 }; 6];
        let mut count = 0usize;
        rects[count] = Rect {
            x: 0,
            y: self.top_bar_height(),
            width: self.sidebar_width(),
            height: self.taskbar_y() - self.top_bar_height(),
        };
        count += 1;
        let order = self.window_order();
        let mut index = 0usize;
        while index < order.len() && count < rects.len() - 1 {
            if let Some(window) = order[index] {
                if self.window_is_open(window) {
                    rects[count] = window_to_region(self.window_bounds(window));
                    count += 1;
                }
            }
            index += 1;
        }
        rects[count] = Rect {
            x: 0,
            y: self.taskbar_y() - self.sy(6),
            width: self.fb.width() as i32,
            height: self.taskbar_height() + self.sy(6),
        };
        count += 1;
        self.redraw_regions(&rects[..count]);
    }

    fn draw_background(&self) {
        let height = self.fb.height() as usize;
        let width = self.fb.width() as usize;
        let top_bar_height = self.top_bar_height();
        let taskbar_y = self.taskbar_y();
        let taskbar_height = self.taskbar_height();
        let sidebar_width = self.sidebar_width();
        let mut y = 0usize;
        while y < height {
            let color = self.background_color_for_y(y as i32);
            self.fill_rect(0, y as i32, width as i32, 1, color);
            y += 1;
        }

        self.fill_rect(0, 0, width as i32, top_bar_height, 1);
        self.fill_rect(0, top_bar_height - self.sy(2), width as i32, self.sy(2), 8);
        self.fill_rect(0, taskbar_y - self.sy(6), width as i32, self.sy(6), 8);
        self.fill_rect(0, taskbar_y, width as i32, taskbar_height, 0);
        self.fill_rect(0, taskbar_y, width as i32, 1, 15);
        self.fill_rect(0, taskbar_y + 1, width as i32, 1, 8);
        self.fill_rect(self.sx(4), top_bar_height + self.sy(8), sidebar_width - self.sx(10), taskbar_y - top_bar_height - self.sy(14), 1);
        self.draw_rect(self.sx(4), top_bar_height + self.sy(8), sidebar_width - self.sx(10), taskbar_y - top_bar_height - self.sy(14), 8);
    }

    fn draw_background_region(&self, rect: Rect) {
        let clipped = self.clip_rect(rect);
        if clipped.width <= 0 || clipped.height <= 0 {
            return;
        }
        let top_bar_height = self.top_bar_height();
        let taskbar_y = self.taskbar_y();
        let taskbar_height = self.taskbar_height();
        let sidebar_rect = Rect {
            x: self.sx(4),
            y: top_bar_height + self.sy(8),
            width: self.sidebar_width() - self.sx(10),
            height: taskbar_y - top_bar_height - self.sy(14),
        };

        let mut y = clipped.y;
        while y < clipped.y + clipped.height {
            let color = self.background_color_for_y(y);
            self.fill_rect(clipped.x, y, clipped.width, 1, color);
            y += 1;
        }

        self.fill_if_intersects(clipped, Rect { x: 0, y: 0, width: self.fb.width() as i32, height: top_bar_height }, 1);
        self.fill_if_intersects(clipped, Rect { x: 0, y: top_bar_height - self.sy(2), width: self.fb.width() as i32, height: self.sy(2) }, 8);
        self.fill_if_intersects(clipped, Rect { x: 0, y: taskbar_y - self.sy(6), width: self.fb.width() as i32, height: self.sy(6) }, 8);
        self.fill_if_intersects(clipped, Rect { x: 0, y: taskbar_y, width: self.fb.width() as i32, height: taskbar_height }, 0);
        self.fill_if_intersects(clipped, Rect { x: 0, y: taskbar_y, width: self.fb.width() as i32, height: 1 }, 15);
        self.fill_if_intersects(clipped, Rect { x: 0, y: taskbar_y + 1, width: self.fb.width() as i32, height: 1 }, 8);
        self.fill_if_intersects(clipped, sidebar_rect, 1);
        if rects_intersect(clipped, sidebar_rect) {
            self.draw_rect(sidebar_rect.x, sidebar_rect.y, sidebar_rect.width, sidebar_rect.height, 8);
        }
    }

    fn draw_top_bar(&self) {
        self.draw_text(self.sx(10), self.sy(4), 15, "TEDDY-OS");
        self.draw_text(self.sx(64), self.sy(4), 7, "DESKTOP EDITION");
        self.draw_text(self.fb.width() as i32 - self.sx(130), self.sy(4), 14, "GRAPHICS");
        self.draw_text(self.fb.width() as i32 - self.sx(70), self.sy(4), 15, "BUILD");
        self.draw_text(self.sx(10), self.sy(10), 8, "Original Teddy shell theme");
        self.draw_text(self.fb.width() as i32 - self.sx(114), self.sy(10), 7, "VMWARE");
    }

    fn draw_desktop_icons(&self) {
        self.fill_background_rect(0, self.top_bar_height(), self.sidebar_width(), self.taskbar_y() - self.top_bar_height());
        self.draw_icon(self.sx(14), self.top_bar_height() + self.sy(10), DesktopIcon::Terminal, "TERMINAL");
        self.draw_icon(self.sx(14), self.top_bar_height() + self.sy(64), DesktopIcon::Explorer, "EXPLORER");
        self.draw_icon(self.sx(14), self.top_bar_height() + self.sy(118), DesktopIcon::Settings, "SETTINGS");
    }

    fn draw_icon(&self, x: i32, y: i32, icon: DesktopIcon, label: &str) {
        let s = self.ui_scale();
        let card = self.sx(44);
        let label_w = self.sx(60);
        let icon_w = self.sx(32);
        let icon_h = self.sy(30);
        let selected = self.selected_icon == Some(icon);
        let frame = if selected { 15 } else { 7 };
        let fill = if selected { 3 } else { 1 };
        self.fill_rect(x - s * 3, y - s * 3, card, card, 0);
        self.fill_rect(x - s * 4, y - s * 4, card, card, fill);
        self.draw_rect(x - s * 4, y - s * 4, card, card, frame);
        self.fill_rect(x - s * 4, y - s * 4, card, s, 8);

        let asset = icon_asset(icon);
        if asset.width != 0 && asset.height != 0 {
            let draw_x = x + ((icon_w - asset.width as i32 * s) / 2);
            let draw_y = y + ((icon_h - asset.height as i32 * s) / 2);
            self.draw_icon_asset(draw_x, draw_y, asset);
        } else {
            match icon {
                DesktopIcon::Terminal => {
                    self.fill_rect(x + self.sx(2), y + self.sy(6), self.sx(28), self.sy(18), 0);
                    self.fill_rect(x + self.sx(2), y + self.sy(6), self.sx(28), self.sy(4), 8);
                    self.draw_rect(x + self.sx(2), y + self.sy(6), self.sx(28), self.sy(18), 15);
                    self.draw_text(x + self.sx(6), y + self.sy(11), 10, ">");
                    self.draw_text(x + self.sx(12), y + self.sy(11), 15, "_");
                    self.draw_text(x + self.sx(6), y + self.sy(19), 7, "cmd");
                }
                DesktopIcon::Explorer => {
                    self.fill_rect(x + self.sx(4), y + self.sy(10), self.sx(24), self.sy(16), 14);
                    self.fill_rect(x + self.sx(6), y + self.sy(6), self.sx(10), self.sy(6), 6);
                    self.fill_rect(x + self.sx(7), y + self.sy(13), self.sx(18), self.sy(2), 12);
                    self.draw_rect(x + self.sx(4), y + self.sy(10), self.sx(24), self.sy(16), 6);
                }
                DesktopIcon::Writer => {
                    self.fill_rect(x + self.sx(7), y + self.sy(6), self.sx(18), self.sy(20), 15);
                    self.draw_rect(x + self.sx(7), y + self.sy(6), self.sx(18), self.sy(20), 8);
                    self.fill_rect(x + self.sx(18), y + self.sy(18), self.sx(8), self.sy(3), 6);
                    self.fill_rect(x + self.sx(11), y + self.sy(20), self.sx(12), self.sy(2), 9);
                }
                DesktopIcon::Settings => {
                    self.fill_rect(x + self.sx(8), y + self.sy(8), self.sx(16), self.sy(16), 8);
                    self.draw_rect(x + self.sx(8), y + self.sy(8), self.sx(16), self.sy(16), 15);
                    self.fill_rect(x + self.sx(13), y + self.sy(13), self.sx(6), self.sy(6), 1);
                }
            }
        }

        if selected {
            self.fill_rect(x - s * 2, y + self.sy(42), label_w, self.sy(12), 3);
            self.draw_rect(x - s * 2, y + self.sy(42), label_w, self.sy(12), 15);
        }
        self.draw_text(x, y + self.sy(45), 15, label);
    }

    fn draw_windows(&self) {
        let order = self.window_order();
        let mut index = order.len();
        while index > 0 {
            index -= 1;
            if let Some(window) = order[index] {
                if !self.window_is_open(window) {
                    continue;
                }
                let focused = self.focused_window == Some(window);
                match window {
                    WindowKind::Terminal => self.draw_terminal_window(focused),
                    WindowKind::Explorer => self.draw_explorer_window(focused),
                    WindowKind::Writer => self.draw_writer_window(focused),
                    WindowKind::Settings => self.draw_settings_window(focused),
                }
            }
        }
    }

    fn draw_terminal_window(&self, focused: bool) {
        let rect = self.terminal_window;
        let title = if focused { 3 } else { 8 };
        let pad = self.sx(8);
        let line_step = self.sy(10);
        self.draw_window_frame(rect, 1, title, "TERMINAL");
        self.fill_rect(rect.x + pad, rect.y + self.sy(20), rect.width - pad * 2, rect.height - self.sy(28), 0);
        self.fill_rect(rect.x + pad, rect.y + self.sy(20), rect.width - pad * 2, self.sy(8), 1);
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(24), 10, "TEDDY COMMAND LINE");

        let start = self.terminal.history_len().saturating_sub(TERMINAL_VIEW_LINES);
        let mut line = 0usize;
        while line < TERMINAL_VIEW_LINES {
            let history_index = start + line;
            if history_index < self.terminal.history_len() {
                self.draw_text(
                    rect.x + self.sx(12),
                    rect.y + self.sy(36) + (line as i32 * line_step),
                    15,
                    self.terminal.history_line(history_index),
                );
            }
            line += 1;
        }

        let cwd = self.terminal.cwd(&self.fs);
        let text_step = self.text_step();
        self.draw_text(rect.x + self.sx(12), rect.y + rect.height - self.sy(16), 15, cwd);
        self.draw_text(rect.x + self.sx(12) + (cwd.len() as i32 * text_step), rect.y + rect.height - self.sy(16), 15, " $ ");
        self.draw_text(
            rect.x + self.sx(30) + (cwd.len() as i32 * text_step),
            rect.y + rect.height - self.sy(16),
            15,
            self.terminal.input(),
        );
        self.draw_text(
            rect.x + self.sx(30) + (cwd.len() as i32 * text_step) + (self.terminal.input().len() as i32 * text_step),
            rect.y + rect.height - self.sy(16),
            10,
            "_",
        );
    }

    fn draw_explorer_window(&self, focused: bool) {
        let rect = self.explorer_window;
        let title = if focused { 3 } else { 8 };
        self.draw_window_frame(rect, 3, title, "FILE EXPLORER");
        self.fill_rect(rect.x + self.sx(8), rect.y + self.sy(20), rect.width - self.sx(16), self.sy(12), 1);
        self.draw_rect(rect.x + self.sx(8), rect.y + self.sy(20), rect.width - self.sx(16), self.sy(12), 8);
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(24), 15, self.fs.cwd_path());

        self.draw_explorer_toolbar(rect);

        self.fill_rect(rect.x + self.sx(8), rect.y + self.sy(36), self.sx(42), rect.height - self.sy(46), 0);
        self.draw_rect(rect.x + self.sx(8), rect.y + self.sy(36), self.sx(42), rect.height - self.sy(46), 8);
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(42), 15, "HOME");
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(54), 15, "DOCS");
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(66), 7, "SPACE");

        self.fill_rect(rect.x + self.sx(56), rect.y + self.sy(36), rect.width - self.sx(64), rect.height - self.sy(46), 0);
        self.draw_rect(rect.x + self.sx(56), rect.y + self.sy(36), rect.width - self.sx(64), rect.height - self.sy(46), 8);
        self.draw_explorer_entries(rect);
        self.fill_rect(rect.x + self.sx(8), rect.y + rect.height - self.sy(16), rect.width - self.sx(16), self.sy(10), 1);
        self.draw_text(rect.x + self.sx(12), rect.y + rect.height - self.sy(12), 15, self.explorer.status());
    }

    fn draw_settings_window(&self, focused: bool) {
        let rect = self.settings_window;
        let title = if focused { 3 } else { 8 };
        self.draw_window_frame(rect, 1, title, "SETTINGS");
        self.fill_rect(rect.x + self.sx(8), rect.y + self.sy(20), rect.width - self.sx(16), rect.height - self.sy(28), 1);
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(24), 15, "DISPLAY");
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(38), 7, "Current mode");
        self.draw_number(rect.x + self.sx(92), rect.y + self.sy(38), self.fb.width() as u32, 15);
        self.draw_text(rect.x + self.sx(110), rect.y + self.sy(38), 15, "x");
        self.draw_number(rect.x + self.sx(120), rect.y + self.sy(38), self.fb.height() as u32, 15);
        self.draw_text(rect.x + self.sx(140), rect.y + self.sy(38), 15, "x");
        self.draw_number(rect.x + self.sx(150), rect.y + self.sy(38), self.fb.bpp() as u32, 15);

        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(52), 7, "Resolution");
        self.fill_rect(rect.x + self.sx(84), rect.y + self.sy(48), self.sx(62), self.sy(12), 8);
        self.draw_rect(rect.x + self.sx(84), rect.y + self.sy(48), self.sx(62), self.sy(12), 15);
        self.draw_number(rect.x + self.sx(92), rect.y + self.sy(51), self.fb.width() as u32, 15);
        self.draw_text(rect.x + self.sx(116), rect.y + self.sy(51), 15, "X");
        self.draw_number(rect.x + self.sx(126), rect.y + self.sy(51), self.fb.height() as u32, 15);

        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(68), 7, "Status");
        self.draw_text(rect.x + self.sx(68), rect.y + self.sy(68), 14, "APPLY AT BOOT");
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(82), 7, "Modes");
        self.draw_text(rect.x + self.sx(48), rect.y + self.sy(82), 15, "kernelgfx");
        self.draw_text(rect.x + self.sx(48), rect.y + self.sy(90), 15, "kfg800");
        self.draw_text(rect.x + self.sx(48), rect.y + self.sy(98), 15, "kfg1024");
    }

    fn draw_writer_window(&self, focused: bool) {
        let rect = self.writer_window;
        let title = if focused { 3 } else { 8 };
        self.draw_window_frame(rect, 1, title, "TEDDY WRITE");
        self.fill_rect(rect.x + self.sx(8), rect.y + self.sy(20), rect.width - self.sx(16), rect.height - self.sy(28), 0);

        self.fill_rect(rect.x + self.sx(8), rect.y + self.sy(20), rect.width - self.sx(16), self.sy(12), 1);
        self.draw_text(rect.x + self.sx(12), rect.y + self.sy(24), 15, self.writer.path());
        self.draw_text(
            rect.x + rect.width - self.sx(70),
            rect.y + self.sy(24),
            if self.writer.is_dirty() { 14 } else { 10 },
            if self.writer.is_dirty() { "DIRTY" } else { "SAVED" },
        );

        self.draw_toolbar_button(rect.x + self.sx(8), rect.y + self.sy(36), self.sx(26), "SAVE");
        self.draw_toolbar_button(rect.x + self.sx(38), rect.y + self.sy(36), self.sx(34), "REVERT");

        self.fill_rect(rect.x + self.sx(8), rect.y + self.sy(52), rect.width - self.sx(16), rect.height - self.sy(72), 15);
        self.draw_rect(rect.x + self.sx(8), rect.y + self.sy(52), rect.width - self.sx(16), rect.height - self.sy(72), 8);
        self.draw_writer_text(rect);

        self.fill_rect(rect.x + self.sx(8), rect.y + rect.height - self.sy(16), rect.width - self.sx(16), self.sy(10), 1);
        self.draw_text(rect.x + self.sx(12), rect.y + rect.height - self.sy(12), 15, self.writer.status());
    }

    fn draw_explorer_toolbar(&self, rect: WindowRect) {
        self.draw_toolbar_button(rect.x + self.sx(56), rect.y + self.sy(20), self.sx(18), "UP");
        self.draw_toolbar_button(rect.x + self.sx(78), rect.y + self.sy(20), self.sx(26), "DIR");
        self.draw_toolbar_button(rect.x + self.sx(108), rect.y + self.sy(20), self.sx(30), "FILE");
        self.draw_toolbar_button(rect.x + self.sx(142), rect.y + self.sy(20), self.sx(24), "DEL");
    }

    fn draw_toolbar_button(&self, x: i32, y: i32, width: i32, label: &str) {
        self.fill_rect(x, y, width, self.sy(12), 8);
        self.draw_rect(x, y, width, self.sy(12), 15);
        self.draw_text(x + self.sx(4), y + self.sy(3), 15, label);
    }

    fn redraw_region(&mut self, rect: Rect) {
        let rects = [rect];
        self.redraw_regions(&rects);
    }

    fn redraw_regions(&mut self, rects: &[Rect]) {
        let mut clipped_rects = [Rect { x: 0, y: 0, width: 0, height: 0 }; 6];
        let mut clipped_len = 0usize;
        let mut has_top = false;
        let mut has_icons = false;
        let mut has_taskbar = false;

        let mut index = 0usize;
        while index < rects.len() && index < clipped_rects.len() {
            let clipped = self.clip_rect(rects[index]);
            if clipped.width > 0 && clipped.height > 0 {
                clipped_rects[clipped_len] = clipped;
                clipped_len += 1;
                has_top |= rects_intersect(clipped, Rect { x: 0, y: 0, width: self.fb.width() as i32, height: self.top_bar_height() });
                has_icons |= rects_intersect(clipped, Rect { x: 0, y: self.top_bar_height(), width: self.sidebar_width(), height: self.taskbar_y() - self.top_bar_height() });
                has_taskbar |= rects_intersect(clipped, Rect { x: 0, y: self.taskbar_y() - self.sy(6), width: self.fb.width() as i32, height: self.taskbar_height() + self.sy(6) });
            }
            index += 1;
        }

        if clipped_len == 0 {
            return;
        }

        self.restore_cursor_backing();
        let mut redraw_index = 0usize;
        while redraw_index < clipped_len {
            self.draw_background_region(clipped_rects[redraw_index]);
            redraw_index += 1;
        }
        if has_top {
            self.draw_top_bar();
        }
        if has_icons {
            self.draw_desktop_icons();
        }

        let order = self.window_order();
        let mut order_index = order.len();
        while order_index > 0 {
            order_index -= 1;
            if let Some(window) = order[order_index] {
                if !self.window_is_open(window) {
                    continue;
                }
                let window_region = window_to_region(self.window_bounds(window));
                let mut intersects = false;
                let mut region_index = 0usize;
                while region_index < clipped_len {
                    if rects_intersect(clipped_rects[region_index], window_region) {
                        intersects = true;
                        break;
                    }
                    region_index += 1;
                }
                if !intersects {
                    continue;
                }
                let focused = self.focused_window == Some(window);
                match window {
                    WindowKind::Terminal => self.draw_terminal_window(focused),
                    WindowKind::Explorer => self.draw_explorer_window(focused),
                    WindowKind::Writer => self.draw_writer_window(focused),
                    WindowKind::Settings => self.draw_settings_window(focused),
                }
            }
        }

        if has_taskbar {
            self.draw_taskbar();
        }
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    fn redraw_window(&mut self, window: WindowKind) {
        if self.window_is_open(window) {
            self.redraw_region(window_to_region(self.window_bounds(window)));
        }
    }

    fn redraw_terminal_input_strip(&mut self) {
        if !self.terminal_open {
            return;
        }
        let rect = self.terminal_window;
        let pad = self.sx(8);
        let baseline = rect.y + rect.height - self.sy(16);
        let text_step = self.text_step();
        self.restore_cursor_backing();
        self.fill_rect(rect.x + pad, rect.y + rect.height - self.sy(22), rect.width - pad * 2, self.sy(16), 0);

        let cwd = self.terminal.cwd(&self.fs);
        self.draw_text(rect.x + self.sx(12), baseline, 15, cwd);
        self.draw_text(rect.x + self.sx(12) + (cwd.len() as i32 * text_step), baseline, 15, " $ ");
        self.draw_text(
            rect.x + self.sx(30) + (cwd.len() as i32 * text_step),
            baseline,
            15,
            self.terminal.input(),
        );
        self.draw_text(
            rect.x + self.sx(30) + (cwd.len() as i32 * text_step) + (self.terminal.input().len() as i32 * text_step),
            baseline,
            10,
            "_",
        );
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    fn redraw_icon_strip(&mut self) {
        self.redraw_region(Rect {
            x: 0,
            y: self.top_bar_height(),
            width: self.sidebar_width(),
            height: self.taskbar_y() - self.top_bar_height(),
        });
    }

    fn redraw_focus_change(&mut self, old_focus: Option<WindowKind>, new_focus: Option<WindowKind>) {
        let mut rects = [Rect { x: 0, y: 0, width: 0, height: 0 }; 2];
        let mut count = 0usize;
        if let Some(window) = old_focus {
            rects[count] = window_to_region(self.window_bounds(window));
            count += 1;
        }
        if let Some(window) = new_focus {
            rects[count] = window_to_region(self.window_bounds(window));
            count += 1;
        }
        if count > 0 {
            self.redraw_regions(&rects[..count]);
        }
    }

    fn redraw_window_move(&mut self, old_rect: WindowRect, new_rect: WindowRect) {
        let rects = [window_to_region(old_rect), window_to_region(new_rect)];
        self.redraw_regions(&rects);
    }

    fn draw_icon_asset(&self, x: i32, y: i32, asset: IconAsset) {
        let scale = self.ui_scale();
        let mut row = 0usize;
        while row < asset.height {
            let mut col = 0usize;
            while col < asset.width {
                let pixel = asset.pixels[row * asset.width + col];
                if pixel != 255 {
                    self.fill_rect(
                        x + col as i32 * scale,
                        y + row as i32 * scale,
                        scale,
                        scale,
                        pixel,
                    );
                }
                col += 1;
            }
            row += 1;
        }
    }

    fn draw_explorer_entry(&self, x: i32, y: i32, folder: bool, name: &str) {
        let s = self.ui_scale();
        if folder {
            self.fill_rect(x, y + s, self.sx(10), self.sy(7), 14);
            self.fill_rect(x + s, y - s, self.sx(4), self.sy(3), 12);
            self.draw_rect(x, y + s, self.sx(10), self.sy(7), 6);
        } else {
            self.fill_rect(x, y, self.sx(9), self.sy(10), 15);
            self.draw_rect(x, y, self.sx(9), self.sy(10), 8);
            self.fill_rect(x + self.sx(5), y, self.sx(4), self.sy(3), 7);
        }
        self.draw_text(x + self.sx(14), y + s, 15, name);
    }

    fn draw_explorer_entries(&self, rect: WindowRect) {
        let mut kinds = [crate::fs::EntryKind::File; crate::fs::MAX_FS_NODES];
        let mut names = [crate::fs::NameText::empty(); crate::fs::MAX_FS_NODES];
        let mut sizes = [0usize; crate::fs::MAX_FS_NODES];
        let len = self.fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes);
        if len == 0 {
            self.draw_text(rect.x + self.sx(68), rect.y + self.sy(48), 15, "(EMPTY)");
            return;
        }

        let visible = core::cmp::min(len, 4);
        let start = if self.explorer.selected_index() >= visible {
            self.explorer.selected_index() + 1 - visible
        } else {
            0
        };

        let mut row = 0usize;
        while row < visible {
            let index = start + row;
            let y = rect.y + self.sy(42) + (row as i32 * self.sy(14));
            let selected = index == self.explorer.selected_index();
            if selected {
                self.fill_rect(rect.x + self.sx(58), y - self.sy(2), rect.width - self.sx(68), self.sy(12), 3);
                self.draw_rect(rect.x + self.sx(58), y - self.sy(2), rect.width - self.sx(68), self.sy(12), 15);
            }
            self.draw_explorer_entry(
                rect.x + self.sx(62),
                y,
                kinds[index] == crate::fs::EntryKind::Dir,
                names[index].as_str(),
            );
            if kinds[index] == crate::fs::EntryKind::File {
                let mut buffer = [b' '; 10];
                let len = format_small_decimal(sizes[index], &mut buffer);
                let rendered = core::str::from_utf8(&buffer[..len]).unwrap_or("");
                self.draw_text(rect.x + rect.width - self.sx(32), y + self.ui_scale(), 15, rendered);
            }
            row += 1;
        }
    }

    fn handle_window_client_click(&mut self, x: i32, y: i32) -> Option<MouseRedraw> {
        let window = self.hit_window(x, y)?;
        match window {
            WindowKind::Explorer => self.handle_explorer_click(x, y),
            WindowKind::Writer => self.handle_writer_click(x, y),
            WindowKind::Terminal => {
                self.focus_window(WindowKind::Terminal);
                self.selected_icon = None;
                Some(MouseRedraw::Panels)
            }
            WindowKind::Settings => {
                self.focus_window(WindowKind::Settings);
                self.selected_icon = None;
                Some(MouseRedraw::Panels)
            }
        }
    }

    fn handle_writer_click(&mut self, x: i32, y: i32) -> Option<MouseRedraw> {
        if !self.writer_open || !point_in_window(self.writer_window, x, y) {
            return None;
        }

        self.focus_window(WindowKind::Writer);
        self.selected_icon = None;

        let rect = self.writer_window;
        if point_in_rect(x, y, rect.x + self.sx(8), rect.y + self.sy(36), self.sx(26), self.sy(12)) {
            self.writer.save(&mut self.fs);
        } else if point_in_rect(x, y, rect.x + self.sx(38), rect.y + self.sy(36), self.sx(34), self.sy(12)) {
            self.writer.revert(&self.fs);
        }

        Some(MouseRedraw::Panels)
    }

    fn handle_explorer_click(&mut self, x: i32, y: i32) -> Option<MouseRedraw> {
        if !self.explorer_open || !point_in_window(self.explorer_window, x, y) {
            return None;
        }

        self.focus_window(WindowKind::Explorer);
        self.selected_icon = None;

        if self.handle_explorer_toolbar_click(x, y) {
            return Some(MouseRedraw::Panels);
        }

        if self.handle_explorer_sidebar_click(x, y) {
            return Some(MouseRedraw::Panels);
        }

        if self.handle_explorer_entry_click(x, y) {
            return Some(MouseRedraw::Panels);
        }

        Some(MouseRedraw::Panels)
    }

    fn handle_explorer_toolbar_click(&mut self, x: i32, y: i32) -> bool {
        let rect = self.explorer_window;
        if point_in_rect(x, y, rect.x + self.sx(56), rect.y + self.sy(20), self.sx(18), self.sy(12)) {
            return self.explorer.go_parent(&mut self.fs);
        }
        if point_in_rect(x, y, rect.x + self.sx(78), rect.y + self.sy(20), self.sx(26), self.sy(12)) {
            return self.explorer.create_folder(&mut self.fs);
        }
        if point_in_rect(x, y, rect.x + self.sx(108), rect.y + self.sy(20), self.sx(30), self.sy(12)) {
            return self.explorer.create_file(&mut self.fs);
        }
        if point_in_rect(x, y, rect.x + self.sx(142), rect.y + self.sy(20), self.sx(24), self.sy(12)) {
            return self.explorer.delete_selected(&mut self.fs);
        }
        false
    }

    fn handle_explorer_sidebar_click(&mut self, x: i32, y: i32) -> bool {
        let rect = self.explorer_window;
        if point_in_rect(x, y, rect.x + self.sx(8), rect.y + self.sy(36), self.sx(42), self.sy(12)) {
            return self.explorer.go_home(&mut self.fs);
        }
        if point_in_rect(x, y, rect.x + self.sx(8), rect.y + self.sy(48), self.sx(42), self.sy(12)) {
            return self.explorer.go_docs(&mut self.fs);
        }
        if point_in_rect(x, y, rect.x + self.sx(8), rect.y + self.sy(60), self.sx(42), self.sy(12)) {
            return self.explorer.go_home(&mut self.fs);
        }
        false
    }

    fn handle_explorer_entry_click(&mut self, x: i32, y: i32) -> bool {
        let rect = self.explorer_window;
        if !point_in_rect(x, y, rect.x + self.sx(56), rect.y + self.sy(36), rect.width - self.sx(64), rect.height - self.sy(46)) {
            return false;
        }

        let mut kinds = [crate::fs::EntryKind::File; crate::fs::MAX_FS_NODES];
        let mut names = [crate::fs::NameText::empty(); crate::fs::MAX_FS_NODES];
        let mut sizes = [0usize; crate::fs::MAX_FS_NODES];
        let len = self.fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes);
        if len == 0 {
            return false;
        }

        let visible = core::cmp::min(len, EXPLORER_ROWS_VISIBLE);
        let start = if self.explorer.selected_index() >= visible {
            self.explorer.selected_index() + 1 - visible
        } else {
            0
        };

        let mut row = 0usize;
        while row < visible {
            let index = start + row;
            let row_y = rect.y + self.sy(42) + (row as i32 * self.sy(14));
            if point_in_rect(x, y, rect.x + self.sx(58), row_y - self.sy(2), rect.width - self.sx(68), self.sy(12)) {
                let was_selected = index == self.explorer.selected_index();
                self.explorer.select_index(index, &self.fs);

                let now = interrupts::timer_ticks();
                let mut should_open = false;
                if let Some(last_tick) = self.last_explorer_click_tick {
                    if now.saturating_sub(last_tick) <= DOUBLE_CLICK_TICKS && was_selected {
                        should_open = true;
                    }
                }
                self.last_explorer_click_tick = Some(now);
                if should_open {
                    let action = self.explorer.open_selected(&mut self.fs);
                    self.handle_explorer_action(action);
                }
                return true;
            }
            row += 1;
        }

        false
    }

    fn draw_taskbar(&self) {
        let accent = self.accent_color();
        let y = self.taskbar_y() + self.sy(2);
        let button_h = self.sy(12);
        self.fill_rect(self.sx(6), y, self.sx(50), button_h, accent);
        self.draw_rect(self.sx(6), y, self.sx(50), button_h, 15);
        self.draw_text(self.sx(14), y + self.sy(3), 15, "TEDDY");

        self.draw_taskbar_button(self.sx(64), DesktopIcon::Terminal, self.terminal_open);
        self.draw_taskbar_button(self.sx(112), DesktopIcon::Explorer, self.explorer_open);
        self.draw_taskbar_button(self.sx(160), DesktopIcon::Writer, self.writer_open);
        self.draw_taskbar_button(self.sx(208), DesktopIcon::Settings, self.settings_open);

        self.fill_rect(self.fb.width() as i32 - self.sx(60), y, self.sx(54), button_h, 1);
        self.draw_rect(self.fb.width() as i32 - self.sx(60), y, self.sx(54), button_h, 8);
        self.draw_text(self.fb.width() as i32 - self.sx(54), y + self.sy(3), 15, "UP");
        self.draw_number(self.fb.width() as i32 - self.sx(36), y + self.sy(3), self.uptime_seconds as u32, 14);
    }

    fn draw_taskbar_button(&self, x: i32, icon: DesktopIcon, active: bool) {
        let fill = if active { 3 } else { 1 };
        let edge = if active { 15 } else { 8 };
        let label = match icon {
            DesktopIcon::Terminal => "TERM",
            DesktopIcon::Explorer => "FILES",
            DesktopIcon::Writer => "WRITE",
            DesktopIcon::Settings => "SET",
        };
        let y = self.taskbar_y() + self.sy(2);
        self.fill_rect(x, y, self.sx(42), self.sy(12), fill);
        self.draw_rect(x, y, self.sx(42), self.sy(12), edge);
        self.draw_text(x + self.sx(5), y + self.sy(3), 15, label);
    }

    fn fill_background_rect(&self, x: i32, y: i32, width: i32, height: i32) {
        let mut row = 0;
        while row < height {
            let color = self.background_color_for_y(y + row);
            self.fill_rect(x, y + row, width, 1, color);
            row += 1;
        }
    }

    fn background_color_for_y(&self, y: i32) -> u8 {
        if y < 40 {
            1
        } else if y < 92 {
            9
        } else if y < 150 {
            3
        } else {
            1
        }
    }

    fn draw_cursor(&self) {
        let mouse = self.input.mouse_state();
        let fill = if mouse.buttons & input::MOUSE_BUTTON_LEFT != 0 {
            14
        } else if mouse.buttons & input::MOUSE_BUTTON_RIGHT != 0 {
            11
        } else if mouse.buttons & input::MOUSE_BUTTON_MIDDLE != 0 {
            10
        } else {
            15
        };

        let mut row = 0usize;
        while row < CURSOR_BITMAP.len() {
            let bits = CURSOR_BITMAP[row];
            let mut col = 0usize;
            while col < CURSOR_SIZE {
                let mask = 1u16 << (CURSOR_SIZE - 1 - col);
                if bits & mask != 0 {
                    let color = if CURSOR_OUTLINE[row] & mask != 0 { 0 } else { fill };
                    self.put_pixel(mouse.x + col as i32, mouse.y + row as i32, color);
                }
                col += 1;
            }
            row += 1;
        }
    }

    fn save_cursor_backing(&mut self, mouse: MouseState) {
        self.cursor_saved_x = mouse.x;
        self.cursor_saved_y = mouse.y;

        let mut row = 0usize;
        while row < CURSOR_SIZE {
            let mut col = 0usize;
            while col < CURSOR_SIZE {
                self.cursor_backing[row * CURSOR_SIZE + col] =
                    self.read_native_pixel(mouse.x + col as i32, mouse.y + row as i32);
                col += 1;
            }
            row += 1;
        }
    }

    fn restore_cursor_backing(&self) {
        let mut row = 0usize;
        while row < CURSOR_SIZE {
            let mut col = 0usize;
            while col < CURSOR_SIZE {
                self.write_native_pixel(
                    self.cursor_saved_x + col as i32,
                    self.cursor_saved_y + row as i32,
                    self.cursor_backing[row * CURSOR_SIZE + col],
                );
                col += 1;
            }
            row += 1;
        }
    }

    fn hit_icon(&self, x: i32, y: i32) -> Option<DesktopIcon> {
        if point_in_rect(x, y, self.sx(10), self.top_bar_height() + self.sy(6), self.sx(44), self.sy(54)) {
            return Some(DesktopIcon::Terminal);
        }
        if point_in_rect(x, y, self.sx(10), self.top_bar_height() + self.sy(60), self.sx(44), self.sy(54)) {
            return Some(DesktopIcon::Explorer);
        }
        if point_in_rect(x, y, self.sx(10), self.top_bar_height() + self.sy(114), self.sx(44), self.sy(54)) {
            return Some(DesktopIcon::Settings);
        }
        None
    }

    fn hit_taskbar_button(&self, x: i32, y: i32) -> Option<DesktopIcon> {
        let ty = self.taskbar_y() + self.sy(2);
        if point_in_rect(x, y, self.sx(64), ty, self.sx(42), self.sy(12)) {
            return Some(DesktopIcon::Terminal);
        }
        if point_in_rect(x, y, self.sx(112), ty, self.sx(42), self.sy(12)) {
            return Some(DesktopIcon::Explorer);
        }
        if point_in_rect(x, y, self.sx(160), ty, self.sx(42), self.sy(12)) {
            return Some(DesktopIcon::Writer);
        }
        if point_in_rect(x, y, self.sx(208), ty, self.sx(42), self.sy(12)) {
            return Some(DesktopIcon::Settings);
        }
        None
    }

    fn hit_window(&self, x: i32, y: i32) -> Option<WindowKind> {
        let order = self.window_order();
        let mut index = 0usize;
        while index < order.len() {
            if let Some(window) = order[index] {
                if self.window_is_open(window) && point_in_window(self.window_bounds(window), x, y) {
                    return Some(window);
                }
            }
            index += 1;
        }
        None
    }

    fn hit_title_bar(&self, x: i32, y: i32) -> Option<WindowKind> {
        let window = self.hit_window(x, y)?;
        let rect = self.window_bounds(window);
        if point_in_rect(x, y, rect.x, rect.y, rect.width, self.title_bar_height() + 2) {
            Some(window)
        } else {
            None
        }
    }

    fn hit_close_button(&self, x: i32, y: i32) -> Option<WindowKind> {
        let window = self.hit_window(x, y)?;
        let rect = self.window_bounds(window);
        if point_in_rect(x, y, rect.x + rect.width - 18, rect.y + 4, 5, 5) {
            Some(window)
        } else {
            None
        }
    }

    fn open_window_for_icon(&mut self, icon: DesktopIcon) {
        match icon {
            DesktopIcon::Terminal => {
                self.terminal_open = true;
                self.focus_window(WindowKind::Terminal);
            }
            DesktopIcon::Explorer => {
                self.explorer_open = true;
                self.focus_window(WindowKind::Explorer);
            }
            DesktopIcon::Writer => {
                self.writer_open = true;
                self.focus_window(WindowKind::Writer);
            }
            DesktopIcon::Settings => {
                self.settings_open = true;
                self.focus_window(WindowKind::Settings);
            }
        }
    }

    fn toggle_or_focus(&mut self, icon: DesktopIcon) {
        match icon {
            DesktopIcon::Terminal => {
                if self.terminal_open && self.focused_window == Some(WindowKind::Terminal) {
                    self.terminal_open = false;
                    self.focused_window = self.next_visible_window(WindowKind::Terminal);
                } else {
                    self.terminal_open = true;
                    self.focus_window(WindowKind::Terminal);
                }
            }
            DesktopIcon::Explorer => {
                if self.explorer_open && self.focused_window == Some(WindowKind::Explorer) {
                    self.explorer_open = false;
                    self.focused_window = self.next_visible_window(WindowKind::Explorer);
                } else {
                    self.explorer_open = true;
                    self.focus_window(WindowKind::Explorer);
                }
            }
            DesktopIcon::Writer => {
                if self.writer_open && self.focused_window == Some(WindowKind::Writer) {
                    self.writer_open = false;
                    self.focused_window = self.next_visible_window(WindowKind::Writer);
                } else {
                    self.writer_open = true;
                    self.focus_window(WindowKind::Writer);
                }
            }
            DesktopIcon::Settings => {
                if self.settings_open && self.focused_window == Some(WindowKind::Settings) {
                    self.settings_open = false;
                    self.focused_window = self.next_visible_window(WindowKind::Settings);
                } else {
                    self.settings_open = true;
                    self.focus_window(WindowKind::Settings);
                }
            }
        }
    }

    fn close_window(&mut self, window: WindowKind) {
        match window {
            WindowKind::Terminal => self.terminal_open = false,
            WindowKind::Explorer => self.explorer_open = false,
            WindowKind::Writer => self.writer_open = false,
            WindowKind::Settings => self.settings_open = false,
        }
        if self.focused_window == Some(window) {
            self.focused_window = self.next_visible_window(window);
        }
    }

    fn icon_is_open(&self, icon: DesktopIcon) -> bool {
        match icon {
            DesktopIcon::Terminal => self.terminal_open,
            DesktopIcon::Explorer => self.explorer_open,
            DesktopIcon::Writer => self.writer_open,
            DesktopIcon::Settings => self.settings_open,
        }
    }

    fn window_bounds(&self, window: WindowKind) -> WindowRect {
        match window {
            WindowKind::Terminal => self.terminal_window,
            WindowKind::Explorer => self.explorer_window,
            WindowKind::Writer => self.writer_window,
            WindowKind::Settings => self.settings_window,
        }
    }

    fn window_rect_mut(&mut self, window: WindowKind) -> &mut WindowRect {
        match window {
            WindowKind::Terminal => &mut self.terminal_window,
            WindowKind::Explorer => &mut self.explorer_window,
            WindowKind::Writer => &mut self.writer_window,
            WindowKind::Settings => &mut self.settings_window,
        }
    }

    fn focus_window(&mut self, window: WindowKind) {
        self.focused_window = Some(window);
    }

    fn next_visible_window(&self, closed: WindowKind) -> Option<WindowKind> {
        let order = self.window_order();
        let mut index = 0usize;
        while index < order.len() {
            if let Some(window) = order[index] {
                if window != closed && self.window_is_open(window) {
                    return Some(window);
                }
            }
            index += 1;
        }
        None
    }

    fn window_is_open(&self, window: WindowKind) -> bool {
        match window {
            WindowKind::Terminal => self.terminal_open,
            WindowKind::Explorer => self.explorer_open,
            WindowKind::Writer => self.writer_open,
            WindowKind::Settings => self.settings_open,
        }
    }

    fn window_order(&self) -> [Option<WindowKind>; 4] {
        match self.focused_window {
            Some(WindowKind::Terminal) => [
                Some(WindowKind::Terminal),
                Some(WindowKind::Explorer),
                Some(WindowKind::Writer),
                Some(WindowKind::Settings),
            ],
            Some(WindowKind::Explorer) => [
                Some(WindowKind::Explorer),
                Some(WindowKind::Writer),
                Some(WindowKind::Settings),
                Some(WindowKind::Terminal),
            ],
            Some(WindowKind::Writer) => [
                Some(WindowKind::Writer),
                Some(WindowKind::Explorer),
                Some(WindowKind::Settings),
                Some(WindowKind::Terminal),
            ],
            Some(WindowKind::Settings) => [
                Some(WindowKind::Settings),
                Some(WindowKind::Writer),
                Some(WindowKind::Explorer),
                Some(WindowKind::Terminal),
            ],
            None => [
                Some(WindowKind::Settings),
                Some(WindowKind::Writer),
                Some(WindowKind::Explorer),
                Some(WindowKind::Terminal),
            ],
        }
    }

    fn draw_window_frame(&self, rect: WindowRect, body: u8, title: u8, label: &str) {
        self.fill_rect(rect.x + 2, rect.y + 2, rect.width, rect.height, 0);
        self.fill_rect(rect.x, rect.y, rect.width, rect.height, body);
        self.draw_rect(rect.x, rect.y, rect.width, rect.height, 15);
        self.fill_rect(rect.x + 1, rect.y + 1, rect.width - 2, self.title_bar_height(), title);
        self.fill_rect(rect.x + 1, rect.y + self.title_bar_height() + 1, rect.width - 2, 1, 8);
        self.draw_text(rect.x + self.sx(6), rect.y + self.sy(4), 15, label);
        self.fill_rect(rect.x + rect.width - self.sx(18), rect.y + self.sy(4), self.sx(5), self.sy(5), 4);
        self.fill_rect(rect.x + rect.width - self.sx(10), rect.y + self.sy(4), self.sx(5), self.sy(5), 8);
    }

    fn handle_explorer_action(&mut self, action: ExplorerAction) {
        match action {
            ExplorerAction::None => {}
            ExplorerAction::Changed => self.redraw_window(WindowKind::Explorer),
            ExplorerAction::OpenTextFile(name) => {
                self.open_writer_for_name(name.as_str());
                self.redraw_panels();
            }
        }
    }

    fn open_writer_for_name(&mut self, name: &str) {
        let mut path = [0u8; 72];
        let cwd = self.fs.cwd_path().as_bytes();
        let file = name.as_bytes();
        let mut len = 0usize;

        if cwd == b"/" {
            path[len] = b'/';
            len += 1;
        } else {
            let mut index = 0usize;
            while index < cwd.len() && len < path.len() {
                path[len] = cwd[index];
                len += 1;
                index += 1;
            }
            if len < path.len() && path[len - 1] != b'/' {
                path[len] = b'/';
                len += 1;
            }
        }

        let mut index = 0usize;
        while index < file.len() && len < path.len() {
            path[len] = file[index];
            len += 1;
            index += 1;
        }

        let full_path = core::str::from_utf8(&path[..len]).unwrap_or("");
        if self.writer.open(full_path, &self.fs) {
            self.writer_open = true;
            self.focus_window(WindowKind::Writer);
        }
    }

    fn draw_writer_text(&self, rect: WindowRect) {
        let start_x = rect.x + self.sx(12);
        let start_y = rect.y + self.sy(58);
        let step_x = self.text_step();
        let step_y = self.sy(10);
        let max_cols = ((rect.width - self.sx(28)) / step_x).max(1) as usize;
        let max_rows = ((rect.height - self.sy(82)) / step_y).max(1) as usize;

        let mut row = 0usize;
        let mut col = 0usize;
        let mut index = 0usize;
        while index < self.writer.text_len() && row < max_rows {
            let byte = self.writer.text_byte(index);
            if byte == b'\n' {
                row += 1;
                col = 0;
                index += 1;
                continue;
            }

            self.draw_char(start_x + (col as i32 * step_x), start_y + (row as i32 * step_y), byte, 0);
            col += 1;
            if col >= max_cols {
                row += 1;
                col = 0;
            }
            index += 1;
        }

        if row < max_rows {
            self.draw_text(start_x + (col as i32 * step_x), start_y + (row as i32 * step_y), 10, "_");
        }
    }

    fn accent_color(&self) -> u8 {
        match self.accent_phase {
            0 => 3,
            1 => 11,
            _ => 8,
        }
    }

    fn ui_scale(&self) -> i32 {
        let scale_x = (self.fb.width() as i32 / BASE_WIDTH).max(1);
        let scale_y = (self.fb.height() as i32 / BASE_HEIGHT).max(1);
        scale_x.min(scale_y).clamp(1, 3)
    }

    fn text_scale(&self) -> i32 {
        self.ui_scale()
    }

    fn text_step(&self) -> i32 {
        6 * self.text_scale()
    }

    fn sx(&self, value: i32) -> i32 {
        value * self.ui_scale()
    }

    fn sy(&self, value: i32) -> i32 {
        value * self.ui_scale()
    }

    fn top_bar_height(&self) -> i32 {
        TOP_BAR_HEIGHT * self.ui_scale()
    }

    fn title_bar_height(&self) -> i32 {
        TITLE_BAR_HEIGHT * self.ui_scale()
    }

    fn taskbar_height(&self) -> i32 {
        TASKBAR_HEIGHT * self.ui_scale()
    }

    fn taskbar_y(&self) -> i32 {
        self.fb.height() as i32 - self.taskbar_height()
    }

    fn sidebar_width(&self) -> i32 {
        self.sx(74)
    }

    fn draw_text(&self, x: i32, y: i32, color: u8, text: &str) {
        let bytes = text.as_bytes();
        let mut index = 0usize;
        let step = self.text_step();
        while index < bytes.len() {
            self.draw_char(x + (index as i32 * step), y, bytes[index], color);
            index += 1;
        }
    }

    fn draw_char(&self, x: i32, y: i32, byte: u8, color: u8) {
        let glyph = glyph_for(byte);
        let scale = self.text_scale();
        let mut row = 0usize;
        while row < glyph.len() {
            let bits = glyph[row];
            let mut col = 0usize;
            while col < 5 {
                if bits & (1 << (4 - col)) != 0 {
                    self.fill_rect(x + col as i32 * scale, y + row as i32 * scale, scale, scale, color);
                }
                col += 1;
            }
            row += 1;
        }
    }

    fn draw_number(&self, x: i32, y: i32, mut value: u32, color: u8) {
        if value == 0 {
            self.draw_char(x, y, b'0', color);
            return;
        }

        let mut scratch = [0u8; 10];
        let mut len = 0usize;
        while value > 0 {
            scratch[len] = b'0' + (value % 10) as u8;
            value /= 10;
            len += 1;
        }

        let mut index = 0usize;
        let step = self.text_step();
        while index < len {
            self.draw_char(x + (index as i32 * step), y, scratch[len - 1 - index], color);
            index += 1;
        }
    }

    fn fill_rect(&self, x: i32, y: i32, width: i32, height: i32, color: u8) {
        let mut yy = 0;
        while yy < height {
            let mut xx = 0;
            while xx < width {
                self.put_pixel(x + xx, y + yy, color);
                xx += 1;
            }
            yy += 1;
        }
    }

    fn draw_rect(&self, x: i32, y: i32, width: i32, height: i32, color: u8) {
        let mut xx = 0;
        while xx < width {
            self.put_pixel(x + xx, y, color);
            self.put_pixel(x + xx, y + height - 1, color);
            xx += 1;
        }
        let mut yy = 0;
        while yy < height {
            self.put_pixel(x, y + yy, color);
            self.put_pixel(x + width - 1, y + yy, color);
            yy += 1;
        }
    }

    fn clip_rect(&self, rect: Rect) -> Rect {
        let left = clamp(rect.x, 0, self.fb.width() as i32);
        let top = clamp(rect.y, 0, self.fb.height() as i32);
        let right = clamp(rect.x + rect.width, 0, self.fb.width() as i32);
        let bottom = clamp(rect.y + rect.height, 0, self.fb.height() as i32);
        Rect {
            x: left,
            y: top,
            width: (right - left).max(0),
            height: (bottom - top).max(0),
        }
    }

    fn fill_if_intersects(&self, clip: Rect, target: Rect, color: u8) {
        if let Some(intersection) = intersect_rects(clip, target) {
            self.fill_rect(intersection.x, intersection.y, intersection.width, intersection.height, color);
        }
    }

    fn read_native_pixel(&self, x: i32, y: i32) -> u32 {
        if x < 0 || y < 0 {
            return 0;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.fb.width() as usize || y >= self.fb.height() as usize {
            return 0;
        }

        let offset = y * self.fb.pitch() as usize + x * self.bytes_per_pixel();
        let ptr = self.fb.addr() as usize as *const u8;
        unsafe {
            match self.fb.bpp() {
                8 => ptr.add(offset).read_volatile() as u32,
                24 => {
                    let b = ptr.add(offset).read_volatile() as u32;
                    let g = ptr.add(offset + 1).read_volatile() as u32;
                    let r = ptr.add(offset + 2).read_volatile() as u32;
                    b | (g << 8) | (r << 16)
                }
                32 => (ptr.add(offset) as *const u32).read_volatile(),
                _ => 0,
            }
        }
    }

    fn put_pixel(&self, x: i32, y: i32, color: u8) {
        self.write_native_pixel(x, y, self.theme_color(color));
    }

    fn write_native_pixel(&self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.fb.width() as usize || y >= self.fb.height() as usize {
            return;
        }

        let offset = y * self.fb.pitch() as usize + x * self.bytes_per_pixel();
        let ptr = self.fb.addr() as usize as *mut u8;
        unsafe {
            match self.fb.bpp() {
                8 => ptr.add(offset).write_volatile(color as u8),
                24 => {
                    ptr.add(offset).write_volatile((color & 0xFF) as u8);
                    ptr.add(offset + 1).write_volatile(((color >> 8) & 0xFF) as u8);
                    ptr.add(offset + 2).write_volatile(((color >> 16) & 0xFF) as u8);
                }
                32 => (ptr.add(offset) as *mut u32).write_volatile(color),
                _ => {}
            }
        }
    }

    fn bytes_per_pixel(&self) -> usize {
        match self.fb.bpp() {
            24 => 3,
            32 => 4,
            _ => 1,
        }
    }

    fn theme_color(&self, color: u8) -> u32 {
        let rgb = match color & 0x0F {
            0 => 0x000000,
            1 => 0x0000AA,
            2 => 0x00AA00,
            3 => 0x00AAAA,
            4 => 0xAA0000,
            5 => 0xAA00AA,
            6 => 0xAA5500,
            7 => 0xAAAAAA,
            8 => 0x555555,
            9 => 0x5555FF,
            10 => 0x55FF55,
            11 => 0xFFFF55,
            12 => 0xFF5555,
            13 => 0xFF55FF,
            14 => 0x55FFFF,
            _ => 0xFFFFFF,
        };

        match self.fb.bpp() {
            8 => (color & 0x0F) as u32,
            24 => rgb_to_bgr(rgb),
            32 => rgb_to_bgr(rgb),
            _ => 0,
        }
    }
}

fn point_in_rect(x: i32, y: i32, left: i32, top: i32, width: i32, height: i32) -> bool {
    x >= left && x < left + width && y >= top && y < top + height
}

fn point_in_window(rect: WindowRect, x: i32, y: i32) -> bool {
    point_in_rect(x, y, rect.x, rect.y, rect.width, rect.height)
}

fn window_to_region(rect: WindowRect) -> Rect {
    Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width + 2,
        height: rect.height + 2,
    }
}

fn rects_intersect(a: Rect, b: Rect) -> bool {
    intersect_rects(a, b).is_some()
}

fn intersect_rects(a: Rect, b: Rect) -> Option<Rect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    if right <= left || bottom <= top {
        return None;
    }

    Some(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn mouse_changed(previous: MouseState, current: MouseState) -> bool {
    previous.x != current.x || previous.y != current.y || previous.buttons != current.buttons
}

fn combine_redraw(current: MouseRedraw, next: MouseRedraw) -> MouseRedraw {
    match (current, next) {
        (MouseRedraw::Full, _) | (_, MouseRedraw::Full) => MouseRedraw::Full,
        (MouseRedraw::Panels, _) | (_, MouseRedraw::Panels) => MouseRedraw::Panels,
        (MouseRedraw::Hud, _) | (_, MouseRedraw::Hud) => MouseRedraw::Hud,
        (MouseRedraw::Overlay, _) | (_, MouseRedraw::Overlay) => MouseRedraw::Overlay,
        _ => MouseRedraw::None,
    }
}

fn icon_asset(icon: DesktopIcon) -> IconAsset {
        match icon {
            DesktopIcon::Terminal => IconAsset {
                width: generated_icons::TERMINAL_ICON_WIDTH,
                height: generated_icons::TERMINAL_ICON_HEIGHT,
                pixels: &generated_icons::TERMINAL_ICON_PIXELS,
            },
            DesktopIcon::Explorer => IconAsset {
                width: generated_icons::EXPLORER_ICON_WIDTH,
                height: generated_icons::EXPLORER_ICON_HEIGHT,
                pixels: &generated_icons::EXPLORER_ICON_PIXELS,
            },
            DesktopIcon::Writer => IconAsset {
                width: generated_icons::WRITER_ICON_WIDTH,
                height: generated_icons::WRITER_ICON_HEIGHT,
                pixels: &generated_icons::WRITER_ICON_PIXELS,
            },
            DesktopIcon::Settings => IconAsset {
                width: generated_icons::SETTINGS_ICON_WIDTH,
                height: generated_icons::SETTINGS_ICON_HEIGHT,
                pixels: &generated_icons::SETTINGS_ICON_PIXELS,
        },
    }
}

const CURSOR_BITMAP: [u16; CURSOR_SIZE] = [
    0b1000000000000000,
    0b1100000000000000,
    0b1110000000000000,
    0b1111000000000000,
    0b1111100000000000,
    0b1111110000000000,
    0b1111111000000000,
    0b1111111100000000,
    0b1111111110000000,
    0b1111111111000000,
    0b1111111111100000,
    0b1110011000000000,
    0b1100001100000000,
    0b1000000110000000,
    0b0000000011000000,
    0b0000000000000000,
];

const CURSOR_OUTLINE: [u16; CURSOR_SIZE] = [
    0b1000000000000000,
    0b1100000000000000,
    0b1010000000000000,
    0b1001000000000000,
    0b1000100000000000,
    0b1000010000000000,
    0b1000001000000000,
    0b1000000100000000,
    0b1000000010000000,
    0b1000000001000000,
    0b1000000000100000,
    0b1000000000010000,
    0b1000001000000000,
    0b1100000100000000,
    0b1000000010000000,
    0b0000000000000000,
];

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn rgb_to_bgr(rgb: u32) -> u32 {
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    b | (g << 8) | (r << 16)
}

fn format_small_decimal(mut value: usize, buffer: &mut [u8; 10]) -> usize {
    if value == 0 {
        buffer[0] = b'0';
        return 1;
    }

    let mut scratch = [0u8; 10];
    let mut len = 0usize;
    while value > 0 && len < scratch.len() {
        scratch[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    let mut index = 0usize;
    while index < len {
        buffer[index] = scratch[len - 1 - index];
        index += 1;
    }
    len
}

fn glyph_for(byte: u8) -> [u8; 7] {
    match to_upper(byte) {
        b'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        b'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        b'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        b'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        b'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        b'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
        b'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        b'J' => [0x1F, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0C],
        b'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        b'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        b'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        b'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        b'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        b'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        b'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        b'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        b'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        b'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        b'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        b'0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        b'1' => [0x04, 0x0C, 0x14, 0x04, 0x04, 0x04, 0x1F],
        b'2' => [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
        b'3' => [0x1F, 0x01, 0x02, 0x06, 0x01, 0x11, 0x0E],
        b'4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        b'5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        b'6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        b'7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        b'9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        b'-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        b':' => [0x00, 0x04, 0x00, 0x00, 0x04, 0x00, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        b'/' => [0x01, 0x02, 0x04, 0x04, 0x08, 0x10, 0x00],
        b'>' => [0x10, 0x08, 0x04, 0x02, 0x04, 0x08, 0x10],
        b'?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        b'_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        b' ' => [0, 0, 0, 0, 0, 0, 0],
        _ => [0x1F, 0x11, 0x02, 0x04, 0x00, 0x04, 0x00],
    }
}

fn to_upper(byte: u8) -> u8 {
    if (b'a'..=b'z').contains(&byte) {
        byte - 32
    } else {
        byte
    }
}
