use crate::{
    boot_info::{BootInfo, FramebufferInfo},
    input::{self, InputEvent, InputManager, MouseState},
    interrupts,
};

const TITLE_BAR_HEIGHT: i32 = 14;
const CURSOR_SIZE: usize = 11;
const STATUS_X: i32 = 12;
const STATUS_Y: i32 = 142;
const STATUS_WIDTH: i32 = 296;
const STATUS_HEIGHT: i32 = 22;
const TASKBAR_Y: i32 = 182;
const DOUBLE_CLICK_TICKS: u64 = 40;

pub struct GraphicsShell {
    fb: FramebufferInfo,
    input: InputManager,
    uptime_seconds: u64,
    accent_phase: u8,
    terminal_window: WindowRect,
    explorer_window: WindowRect,
    terminal_open: bool,
    explorer_open: bool,
    focused_window: Option<WindowKind>,
    selected_icon: Option<DesktopIcon>,
    drag_state: DragState,
    last_icon_click: Option<IconClickState>,
    cursor_backing: [u8; CURSOR_SIZE * CURSOR_SIZE],
    cursor_saved_x: i32,
    cursor_saved_y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DesktopIcon {
    Terminal,
    Explorer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowKind {
    Terminal,
    Explorer,
}

#[derive(Clone, Copy)]
struct WindowRect {
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

impl GraphicsShell {
    pub fn new(boot_info: BootInfo) -> Option<Self> {
        let fb = boot_info.framebuffer()?;
        if fb.bpp() != 8 {
            return None;
        }

        let max_x = fb.width() as i32 - 1;
        let max_y = fb.height() as i32 - 1;
        Some(Self {
            fb,
            input: InputManager::new(max_x, max_y),
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
            terminal_open: true,
            explorer_open: true,
            focused_window: Some(WindowKind::Explorer),
            selected_icon: None,
            drag_state: DragState {
                active: false,
                window: None,
                offset_x: 0,
                offset_y: 0,
            },
            last_icon_click: None,
            cursor_backing: [0; CURSOR_SIZE * CURSOR_SIZE],
            cursor_saved_x: 0,
            cursor_saved_y: 0,
        })
    }

    pub fn render(&mut self) {
        self.draw_background();
        self.draw_top_bar();
        self.draw_desktop_icons();
        self.draw_windows();
        self.draw_status();
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

    pub fn handle_key(&mut self, ascii: u8) {
        if ascii != b'?' {
            self.accent_phase = self.accent_phase.wrapping_add(1) % 3;
            self.redraw_hud();
        }
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
        let max_y = TASKBAR_Y - bounds.height - 2;
        let next_x = clamp(state.x - self.drag_state.offset_x, 0, max_x);
        let next_y = clamp(state.y - self.drag_state.offset_y, 20, max_y);

        let rect = self.window_rect_mut(window);
        rect.x = next_x;
        rect.y = next_y;
        MouseRedraw::Full
    }

    fn handle_mouse_down(&mut self, state: MouseState, button: u8) -> MouseRedraw {
        if button != input::MOUSE_BUTTON_LEFT {
            return MouseRedraw::Overlay;
        }

        if let Some(window) = self.hit_close_button(state.x, state.y) {
            self.close_window(window);
            return MouseRedraw::Full;
        }

        if let Some(window) = self.hit_title_bar(state.x, state.y) {
            self.focused_window = Some(window);
            let rect = self.window_bounds(window);
            self.drag_state.active = true;
            self.drag_state.window = Some(window);
            self.drag_state.offset_x = state.x - rect.x;
            self.drag_state.offset_y = state.y - rect.y;
            return MouseRedraw::Full;
        }

        if let Some(window) = self.hit_window(state.x, state.y) {
            self.focused_window = Some(window);
            self.selected_icon = None;
            return MouseRedraw::Panels;
        }

        if let Some(icon) = self.hit_taskbar_button(state.x, state.y) {
            let opened = self.icon_is_open(icon);
            self.toggle_or_focus(icon);
            if opened != self.icon_is_open(icon) {
                return MouseRedraw::Full;
            }
            return MouseRedraw::Panels;
        }

        if let Some(icon) = self.hit_icon(state.x, state.y) {
            let now = interrupts::timer_ticks();
            let mut should_open = false;
            if let Some(last_click) = self.last_icon_click {
                if last_click.icon == icon && now.saturating_sub(last_click.tick) <= DOUBLE_CLICK_TICKS {
                    should_open = true;
                }
            }
            self.last_icon_click = Some(IconClickState { icon, tick: now });
            self.selected_icon = Some(icon);
            if should_open {
                self.open_window_for_icon(icon);
                return MouseRedraw::Full;
            }
            return MouseRedraw::Panels;
        }

        self.selected_icon = None;
        self.focused_window = None;
        MouseRedraw::Panels
    }

    fn handle_mouse_up(&mut self, _state: MouseState, button: u8) -> MouseRedraw {
        if button == input::MOUSE_BUTTON_LEFT {
            let was_dragging = self.drag_state.active;
            self.drag_state.active = false;
            self.drag_state.window = None;
            if was_dragging {
                return MouseRedraw::Panels;
            }
        }
        MouseRedraw::Overlay
    }

    fn refresh_cursor_overlay(&mut self, previous_mouse: MouseState) {
        self.restore_cursor_backing();
        if mouse_changed(previous_mouse, self.input.mouse_state()) {
            self.draw_status();
        }
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    fn redraw_hud(&mut self) {
        self.restore_cursor_backing();
        self.draw_status();
        self.draw_taskbar();
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    fn redraw_panels(&mut self) {
        self.restore_cursor_backing();
        self.draw_desktop_icons();
        self.draw_windows();
        self.draw_status();
        self.draw_taskbar();
        self.save_cursor_backing(self.input.mouse_state());
        self.draw_cursor();
    }

    fn draw_background(&self) {
        let height = self.fb.height() as usize;
        let width = self.fb.width() as usize;
        let mut y = 0usize;
        while y < height {
            let color = self.background_color_for_y(y as i32);
            self.fill_rect(0, y as i32, width as i32, 1, color);
            y += 1;
        }

        self.fill_rect(0, 0, width as i32, 18, 8);
        self.fill_rect(0, TASKBAR_Y, width as i32, 18, 8);
        self.fill_rect(0, TASKBAR_Y - 1, width as i32, 1, 7);
    }

    fn draw_top_bar(&self) {
        self.draw_text(10, 5, 15, "TEDDY-OS DESKTOP");
        self.draw_text(160, 5, 14, "GRAPHICS SHELL");
        self.draw_text(252, 5, 15, "VMWARE");
    }

    fn draw_desktop_icons(&self) {
        self.fill_background_rect(0, 18, 74, 122);
        self.draw_icon(14, 28, DesktopIcon::Terminal, "TERMINAL");
        self.draw_icon(14, 82, DesktopIcon::Explorer, "EXPLORER");
    }

    fn draw_icon(&self, x: i32, y: i32, icon: DesktopIcon, label: &str) {
        let selected = self.selected_icon == Some(icon);
        let frame = if selected { 15 } else { 8 };
        let fill = if selected { 12 } else { 1 };
        self.fill_rect(x - 4, y - 4, 44, 44, fill);
        self.draw_rect(x - 4, y - 4, 44, 44, frame);

        match icon {
            DesktopIcon::Terminal => {
                self.fill_rect(x + 2, y + 6, 28, 18, 0);
                self.draw_rect(x + 2, y + 6, 28, 18, 15);
                self.draw_text(x + 6, y + 11, 10, "C>");
                self.draw_text(x + 6, y + 19, 15, "_");
            }
            DesktopIcon::Explorer => {
                self.fill_rect(x + 4, y + 10, 24, 16, 14);
                self.fill_rect(x + 6, y + 6, 10, 6, 12);
                self.draw_rect(x + 4, y + 10, 24, 16, 6);
            }
        }

        if selected {
            self.fill_rect(x - 2, y + 42, 60, 12, 1);
            self.draw_rect(x - 2, y + 42, 60, 12, 15);
        }
        self.draw_text(x, y + 45, 15, label);
    }

    fn draw_windows(&self) {
        match self.focused_window {
            Some(WindowKind::Terminal) => {
                if self.explorer_open {
                    self.draw_explorer_window(false);
                }
                if self.terminal_open {
                    self.draw_terminal_window(true);
                }
            }
            Some(WindowKind::Explorer) => {
                if self.terminal_open {
                    self.draw_terminal_window(false);
                }
                if self.explorer_open {
                    self.draw_explorer_window(true);
                }
            }
            None => {
                if self.terminal_open {
                    self.draw_terminal_window(false);
                }
                if self.explorer_open {
                    self.draw_explorer_window(false);
                }
            }
        }
    }

    fn draw_terminal_window(&self, focused: bool) {
        let rect = self.terminal_window;
        let title = if focused { 12 } else { 8 };
        self.draw_window_frame(rect, 1, title, "TERMINAL");
        self.fill_rect(rect.x + 8, rect.y + 20, rect.width - 16, rect.height - 28, 0);
        self.draw_text(rect.x + 14, rect.y + 26, 10, "TEDDY GRAPHICS TERMINAL");
        self.draw_text(rect.x + 14, rect.y + 38, 15, "guest@teddy:/ $ help");
        self.draw_text(rect.x + 14, rect.y + 50, 7, "Use 'kernel' for full CLI app.");
        self.draw_text(rect.x + 14, rect.y + 62, 15, "guest@teddy:/ $ uname");
        self.draw_text(rect.x + 14, rect.y + 74, 7, "Teddy-OS graphics desktop");
        self.draw_text(rect.x + 14, rect.y + 86, 15, "guest@teddy:/ $ _");
    }

    fn draw_explorer_window(&self, focused: bool) {
        let rect = self.explorer_window;
        let title = if focused { 12 } else { 8 };
        self.draw_window_frame(rect, 3, title, "FILE EXPLORER");
        self.fill_rect(rect.x + 8, rect.y + 20, rect.width - 16, 12, 8);
        self.draw_text(rect.x + 12, rect.y + 24, 15, "HOME > TEDDY DISK > /");

        self.fill_rect(rect.x + 8, rect.y + 36, 42, rect.height - 46, 1);
        self.draw_rect(rect.x + 8, rect.y + 36, 42, rect.height - 46, 8);
        self.draw_text(rect.x + 12, rect.y + 42, 15, "HOME");
        self.draw_text(rect.x + 12, rect.y + 54, 15, "DOCS");
        self.draw_text(rect.x + 12, rect.y + 66, 15, "APPS");

        self.fill_rect(rect.x + 56, rect.y + 36, rect.width - 64, rect.height - 46, 1);
        self.draw_rect(rect.x + 56, rect.y + 36, rect.width - 64, rect.height - 46, 8);
        self.draw_explorer_entry(rect.x + 62, rect.y + 42, true, "docs");
        self.draw_explorer_entry(rect.x + 62, rect.y + 56, false, "readme.txt");
        self.draw_explorer_entry(rect.x + 62, rect.y + 70, false, "notes.txt");
        self.draw_explorer_entry(rect.x + 62, rect.y + 84, true, "apps");
    }

    fn draw_explorer_entry(&self, x: i32, y: i32, folder: bool, name: &str) {
        if folder {
            self.fill_rect(x, y + 1, 10, 7, 14);
            self.fill_rect(x + 1, y - 1, 4, 3, 12);
            self.draw_rect(x, y + 1, 10, 7, 6);
        } else {
            self.fill_rect(x, y, 9, 10, 15);
            self.draw_rect(x, y, 9, 10, 8);
            self.fill_rect(x + 5, y, 4, 3, 7);
        }
        self.draw_text(x + 14, y + 1, 15, name);
    }

    fn draw_status(&self) {
        let mouse = self.input.mouse_state();
        self.fill_rect(STATUS_X, STATUS_Y, STATUS_WIDTH, STATUS_HEIGHT, 1);
        self.draw_rect(STATUS_X, STATUS_Y, STATUS_WIDTH, STATUS_HEIGHT, 15);
        self.draw_text(18, 148, 15, "UP");
        self.draw_number(36, 148, self.uptime_seconds as u32, 14);
        self.draw_text(64, 148, 15, "KEY");
        self.draw_ascii(88, 148, interrupts::last_ascii(), 14);
        self.draw_text(102, 148, 15, "X");
        self.draw_number(114, 148, mouse.x as u32, 14);
        self.draw_text(146, 148, 15, "Y");
        self.draw_number(158, 148, mouse.y as u32, 14);
        self.draw_text(190, 148, 15, "B");
        self.draw_hex_byte(202, 148, mouse.buttons, 14);
        self.draw_text(220, 148, 15, "TIP");
        self.draw_text(242, 148, 14, "DOUBLE-CLICK ICONS");
    }

    fn draw_taskbar(&self) {
        let accent = self.accent_color();
        self.fill_rect(6, 185, 48, 10, accent);
        self.draw_text(13, 187, 15, "TEDDY");

        self.draw_taskbar_button(64, DesktopIcon::Terminal, self.terminal_open);
        self.draw_taskbar_button(126, DesktopIcon::Explorer, self.explorer_open);

        self.draw_text(226, 187, 15, "UP");
        self.draw_number(244, 187, self.uptime_seconds as u32, 14);
    }

    fn draw_taskbar_button(&self, x: i32, icon: DesktopIcon, active: bool) {
        let fill = if active { 12 } else { 1 };
        let edge = if active { 15 } else { 8 };
        let label = match icon {
            DesktopIcon::Terminal => "TERM",
            DesktopIcon::Explorer => "FILES",
        };
        self.fill_rect(x, 184, 54, 12, fill);
        self.draw_rect(x, 184, 54, 12, edge);
        self.draw_text(x + 10, 187, 15, label);
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
        if y < 52 {
            1
        } else if y < 124 {
            9
        } else {
            3
        }
    }

    fn draw_cursor(&self) {
        let mouse = self.input.mouse_state();
        let color = if mouse.buttons & input::MOUSE_BUTTON_LEFT != 0 {
            12
        } else if mouse.buttons & input::MOUSE_BUTTON_RIGHT != 0 {
            14
        } else if mouse.buttons & input::MOUSE_BUTTON_MIDDLE != 0 {
            10
        } else {
            15
        };

        let mut step = 0;
        while step < CURSOR_SIZE as i32 {
            self.put_pixel(mouse.x, mouse.y + step, color);
            if step < 6 {
                self.put_pixel(mouse.x + step, mouse.y + step, color);
            }
            if step < 4 {
                self.put_pixel(mouse.x + 1, mouse.y + step, color);
            }
            step += 1;
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
                    self.read_pixel(mouse.x + col as i32, mouse.y + row as i32);
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
                self.put_pixel(
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
        if point_in_rect(x, y, 10, 24, 44, 54) {
            return Some(DesktopIcon::Terminal);
        }
        if point_in_rect(x, y, 10, 78, 44, 54) {
            return Some(DesktopIcon::Explorer);
        }
        None
    }

    fn hit_taskbar_button(&self, x: i32, y: i32) -> Option<DesktopIcon> {
        if point_in_rect(x, y, 64, 184, 54, 12) {
            return Some(DesktopIcon::Terminal);
        }
        if point_in_rect(x, y, 126, 184, 54, 12) {
            return Some(DesktopIcon::Explorer);
        }
        None
    }

    fn hit_window(&self, x: i32, y: i32) -> Option<WindowKind> {
        if self.focused_window == Some(WindowKind::Explorer) {
            if self.explorer_open && point_in_window(self.explorer_window, x, y) {
                return Some(WindowKind::Explorer);
            }
            if self.terminal_open && point_in_window(self.terminal_window, x, y) {
                return Some(WindowKind::Terminal);
            }
        } else {
            if self.terminal_open && point_in_window(self.terminal_window, x, y) {
                return Some(WindowKind::Terminal);
            }
            if self.explorer_open && point_in_window(self.explorer_window, x, y) {
                return Some(WindowKind::Explorer);
            }
        }
        None
    }

    fn hit_title_bar(&self, x: i32, y: i32) -> Option<WindowKind> {
        let window = self.hit_window(x, y)?;
        let rect = self.window_bounds(window);
        if point_in_rect(x, y, rect.x, rect.y, rect.width, TITLE_BAR_HEIGHT + 2) {
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
                self.focused_window = Some(WindowKind::Terminal);
            }
            DesktopIcon::Explorer => {
                self.explorer_open = true;
                self.focused_window = Some(WindowKind::Explorer);
            }
        }
    }

    fn toggle_or_focus(&mut self, icon: DesktopIcon) {
        match icon {
            DesktopIcon::Terminal => {
                if self.terminal_open && self.focused_window == Some(WindowKind::Terminal) {
                    self.terminal_open = false;
                    if self.focused_window == Some(WindowKind::Terminal) {
                        self.focused_window = if self.explorer_open {
                            Some(WindowKind::Explorer)
                        } else {
                            None
                        };
                    }
                } else {
                    self.terminal_open = true;
                    self.focused_window = Some(WindowKind::Terminal);
                }
            }
            DesktopIcon::Explorer => {
                if self.explorer_open && self.focused_window == Some(WindowKind::Explorer) {
                    self.explorer_open = false;
                    if self.focused_window == Some(WindowKind::Explorer) {
                        self.focused_window = if self.terminal_open {
                            Some(WindowKind::Terminal)
                        } else {
                            None
                        };
                    }
                } else {
                    self.explorer_open = true;
                    self.focused_window = Some(WindowKind::Explorer);
                }
            }
        }
    }

    fn close_window(&mut self, window: WindowKind) {
        match window {
            WindowKind::Terminal => self.terminal_open = false,
            WindowKind::Explorer => self.explorer_open = false,
        }
        if self.focused_window == Some(window) {
            self.focused_window = if window == WindowKind::Terminal && self.explorer_open {
                Some(WindowKind::Explorer)
            } else if window == WindowKind::Explorer && self.terminal_open {
                Some(WindowKind::Terminal)
            } else {
                None
            };
        }
    }

    fn icon_is_open(&self, icon: DesktopIcon) -> bool {
        match icon {
            DesktopIcon::Terminal => self.terminal_open,
            DesktopIcon::Explorer => self.explorer_open,
        }
    }

    fn window_bounds(&self, window: WindowKind) -> WindowRect {
        match window {
            WindowKind::Terminal => self.terminal_window,
            WindowKind::Explorer => self.explorer_window,
        }
    }

    fn window_rect_mut(&mut self, window: WindowKind) -> &mut WindowRect {
        match window {
            WindowKind::Terminal => &mut self.terminal_window,
            WindowKind::Explorer => &mut self.explorer_window,
        }
    }

    fn draw_window_frame(&self, rect: WindowRect, body: u8, title: u8, label: &str) {
        self.fill_rect(rect.x, rect.y, rect.width, rect.height, body);
        self.draw_rect(rect.x, rect.y, rect.width, rect.height, 15);
        self.fill_rect(rect.x + 1, rect.y + 1, rect.width - 2, TITLE_BAR_HEIGHT, title);
        self.draw_text(rect.x + 6, rect.y + 4, 15, label);
        self.fill_rect(rect.x + rect.width - 18, rect.y + 4, 5, 5, 4);
        self.fill_rect(rect.x + rect.width - 10, rect.y + 4, 5, 5, 14);
    }

    fn accent_color(&self) -> u8 {
        match self.accent_phase {
            0 => 12,
            1 => 13,
            _ => 10,
        }
    }

    fn draw_text(&self, x: i32, y: i32, color: u8, text: &str) {
        let bytes = text.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() {
            self.draw_char(x + (index as i32 * 6), y, bytes[index], color);
            index += 1;
        }
    }

    fn draw_char(&self, x: i32, y: i32, byte: u8, color: u8) {
        let glyph = glyph_for(byte);
        let mut row = 0usize;
        while row < glyph.len() {
            let bits = glyph[row];
            let mut col = 0usize;
            while col < 5 {
                if bits & (1 << (4 - col)) != 0 {
                    self.put_pixel(x + col as i32, y + row as i32, color);
                }
                col += 1;
            }
            row += 1;
        }
    }

    fn draw_ascii(&self, x: i32, y: i32, byte: u8, color: u8) {
        let rendered = match byte {
            0x20..=0x7E => byte,
            _ => b'?',
        };
        self.draw_char(x, y, rendered, color);
    }

    fn draw_hex_byte(&self, x: i32, y: i32, value: u8, color: u8) {
        let hi = nybble_to_hex((value >> 4) & 0x0F);
        let lo = nybble_to_hex(value & 0x0F);
        self.draw_char(x, y, hi, color);
        self.draw_char(x + 6, y, lo, color);
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
        while index < len {
            self.draw_char(x + (index as i32 * 6), y, scratch[len - 1 - index], color);
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

    fn read_pixel(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 {
            return 0;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.fb.width() as usize || y >= self.fb.height() as usize {
            return 0;
        }

        let offset = y * self.fb.pitch() as usize + x;
        let ptr = self.fb.addr() as usize as *const u8;
        unsafe { ptr.add(offset).read_volatile() }
    }

    fn put_pixel(&self, x: i32, y: i32, color: u8) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.fb.width() as usize || y >= self.fb.height() as usize {
            return;
        }

        let offset = y * self.fb.pitch() as usize + x;
        let ptr = self.fb.addr() as usize as *mut u8;
        unsafe {
            ptr.add(offset).write_volatile(color);
        }
    }
}

fn point_in_rect(x: i32, y: i32, left: i32, top: i32, width: i32, height: i32) -> bool {
    x >= left && x < left + width && y >= top && y < top + height
}

fn point_in_window(rect: WindowRect, x: i32, y: i32) -> bool {
    point_in_rect(x, y, rect.x, rect.y, rect.width, rect.height)
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

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn nybble_to_hex(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'A' + (value - 10),
    }
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
