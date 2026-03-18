use crate::{
    boot_info::BootInfo,
    explorer::ExplorerApp,
    fs::{EntryKind, FileSystem, NameText, MAX_FS_NODES},
    interrupts,
    terminal::{TerminalAction, TerminalApp},
    trace,
    vga,
};

const MAX_WINDOWS: usize = 5;
const TASKBAR_ROW: usize = 24;
const DESKTOP_HEIGHT: usize = 24;
const TERMINAL_VIEW_LINES: usize = 8;
const EXPLORER_VIEW_LINES: usize = 7;

pub enum ShellAction {
    Reboot,
    Shutdown,
}

pub struct DesktopShell {
    boot_info: Option<BootInfo>,
    fs: FileSystem,
    terminal: TerminalApp,
    explorer: ExplorerApp,
    windows: [Window; MAX_WINDOWS],
    focus_index: usize,
    launcher_open: bool,
    move_mode: bool,
    uptime_seconds: u64,
}

impl DesktopShell {
    pub const fn empty() -> Self {
        Self {
            boot_info: None,
            fs: FileSystem::empty(),
            terminal: TerminalApp::empty(),
            explorer: ExplorerApp::empty(),
            windows: [
                Window::hidden(WindowKind::Terminal),
                Window::hidden(WindowKind::Explorer),
                Window::hidden(WindowKind::Welcome),
                Window::hidden(WindowKind::System),
                Window::hidden(WindowKind::Roadmap),
            ],
            focus_index: 0,
            launcher_open: false,
            move_mode: false,
            uptime_seconds: 0,
        }
    }

    pub fn init(&mut self, boot_info: Option<BootInfo>) {
        trace::set_boot_stage(0x31);
        self.boot_info = boot_info;
        trace::set_boot_stage(0x32);
        self.fs.init();
        trace::set_boot_stage(0x36);
        self.terminal.init();
        trace::set_boot_stage(0x33);
        self.explorer.init();
        trace::set_boot_stage(0x37);
        self.focus_index = 0;
        self.launcher_open = false;
        self.move_mode = false;
        self.uptime_seconds = 0;
        trace::set_boot_stage(0x34);
        self.reset_layout();
        trace::set_boot_stage(0x35);
    }

    pub fn render(&self) {
        self.render_background();
        self.render_windows();
        if self.launcher_open {
            self.render_launcher();
        }
        self.render_taskbar();
    }

    pub fn tick(&mut self, uptime_seconds: u64) {
        if self.uptime_seconds != uptime_seconds {
            self.uptime_seconds = uptime_seconds;
            self.render();
        }
    }

    pub fn handle_key(&mut self, scancode: u8, ascii: u8) -> Option<ShellAction> {
        if self.launcher_open {
            if self.handle_launcher_key(ascii) {
                self.render();
            }
            return None;
        }

        if self.handle_global_key(scancode, ascii) {
            self.render();
            return None;
        }

        if self.focused_kind() == WindowKind::Terminal {
            let action = self.terminal.handle_key(ascii, &mut self.fs);
            self.render();
            return match action {
                TerminalAction::None => None,
                TerminalAction::Reboot => Some(ShellAction::Reboot),
                TerminalAction::Shutdown => Some(ShellAction::Shutdown),
            };
        }

        if self.focused_kind() == WindowKind::Explorer {
            if self.explorer.handle_key(ascii, &mut self.fs) {
                self.render();
            }
        }

        None
    }

    fn handle_launcher_key(&mut self, ascii: u8) -> bool {
        match ascii {
            b'1' => {
                self.open_window(WindowKind::Terminal);
                self.launcher_open = false;
                true
            }
            b'2' => {
                self.open_window(WindowKind::Explorer);
                self.launcher_open = false;
                true
            }
            b'3' => {
                self.open_window(WindowKind::Welcome);
                self.launcher_open = false;
                true
            }
            b'4' => {
                self.open_window(WindowKind::System);
                self.launcher_open = false;
                true
            }
            b'5' => {
                self.open_window(WindowKind::Roadmap);
                self.launcher_open = false;
                true
            }
            27 => {
                self.launcher_open = false;
                true
            }
            _ => false,
        }
    }

    fn handle_global_key(&mut self, scancode: u8, ascii: u8) -> bool {
        match scancode {
            0x3B => {
                self.launcher_open = !self.launcher_open;
                true
            }
            0x3C => {
                self.focus_next_window();
                true
            }
            0x3D => {
                if self.has_visible_window() {
                    self.move_mode = !self.move_mode;
                    true
                } else {
                    false
                }
            }
            0x3E => {
                self.close_focused_window();
                true
            }
            0x3F => {
                self.reset_layout();
                true
            }
            _ if self.move_mode && matches!(ascii, b'w' | b'a' | b's' | b'd') => {
                self.move_focused_window(ascii);
                true
            }
            _ => false,
        }
    }

    fn render_background(&self) {
        vga::clear_screen(0x1F);
        for row in 0..DESKTOP_HEIGHT {
            let stripe = if row % 2 == 0 { b'.' } else { b' ' };
            let attribute = if row % 2 == 0 { 0x1B } else { 0x13 };
            vga::fill_rect(row, 0, 1, vga::width(), stripe, attribute);
        }

        vga::fill_rect(0, 0, 2, vga::width(), b' ', 0x30);
        vga::write_line(0, 2, "Teddy-OS Desktop Shell", 0x3F);
        vga::write_line(1, 2, "Terminal phase MVP in text mode", 0x3E);
        vga::write_line(1, 46, "Original Teddy shell theme", 0x3E);
    }

    fn render_windows(&self) {
        for index in 0..MAX_WINDOWS {
            let window = &self.windows[index];
            if !window.visible {
                continue;
            }
            self.render_window(window, index == self.focus_index);
        }
    }

    fn render_window(&self, window: &Window, focused: bool) {
        let border_attr = if focused { 0x1F } else { 0x17 };
        let title_attr = if focused { 0x70 } else { 0x30 };
        let body_attr = if window.kind == WindowKind::Terminal { 0x07 } else { 0x1E };

        vga::fill_rect(window.y, window.x, window.height, window.width, b' ', body_attr);
        vga::draw_box(window.y, window.x, window.height, window.width, border_attr);
        vga::fill_rect(window.y, window.x + 1, 1, window.width - 2, b' ', title_attr);
        vga::write_line(window.y, window.x + 2, window.kind.title(), title_attr);
        vga::write_line(window.y, window.x + window.width - 7, "[x][ ]", title_attr);
        self.render_window_body(window);
    }

    fn render_window_body(&self, window: &Window) {
        match window.kind {
            WindowKind::Terminal => self.render_terminal(window),
            WindowKind::Explorer => self.render_explorer(window),
            WindowKind::Welcome => self.render_welcome(window),
            WindowKind::System => self.render_system(window),
            WindowKind::Roadmap => self.render_roadmap(window),
        }
    }

    fn render_terminal(&self, window: &Window) {
        let available_lines = core::cmp::min(window.height.saturating_sub(4), TERMINAL_VIEW_LINES);
        let start = self.terminal.history_len().saturating_sub(available_lines);
        for line_index in 0..available_lines {
            let history_index = start + line_index;
            if history_index < self.terminal.history_len() {
                vga::write_line(
                    window.y + 2 + line_index,
                    window.x + 2,
                    self.terminal.history_line(history_index),
                    0x07,
                );
            }
        }

        let prompt_row = window.y + window.height - 2;
        let cwd = self.terminal.cwd(&self.fs);
        vga::write_line(prompt_row, window.x + 2, cwd, 0x0F);
        vga::write_line(prompt_row, window.x + 2 + cwd.len(), " $ ", 0x0F);
        vga::write_line(prompt_row, window.x + 5 + cwd.len(), self.terminal.input(), 0x07);
    }

    fn render_explorer(&self, window: &Window) {
        vga::write_line(window.y + 2, window.x + 2, "Path:", 0x1F);
        vga::write_line(window.y + 2, window.x + 8, self.fs.cwd_path(), 0x1E);
        vga::write_line(window.y + 3, window.x + 2, "Entries:", 0x1F);

        let mut kinds = [EntryKind::File; MAX_FS_NODES];
        let mut names = [NameText::empty(); MAX_FS_NODES];
        let mut sizes = [0usize; MAX_FS_NODES];
        let len = self.fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes);

        if len == 0 {
            vga::write_line(window.y + 5, window.x + 4, "(empty)", 0x1E);
        } else {
            let visible = core::cmp::min(len, EXPLORER_VIEW_LINES);
            let start = if self.explorer.selected_index() >= visible {
                self.explorer.selected_index() + 1 - visible
            } else {
                0
            };
            let mut row = 0usize;
            while row < visible {
                let index = start + row;
                let attr = if index == self.explorer.selected_index() { 0x70 } else { 0x1E };
                vga::fill_rect(window.y + 5 + row, window.x + 2, 1, window.width - 4, b' ', attr);
                match kinds[index] {
                    EntryKind::Dir => vga::write_line(window.y + 5 + row, window.x + 3, "[DIR]", attr),
                    EntryKind::File => vga::write_line(window.y + 5 + row, window.x + 3, "[FILE]", attr),
                }
                vga::write_line(window.y + 5 + row, window.x + 10, names[index].as_str(), attr);
                if kinds[index] == EntryKind::File {
                    let mut size_text = [b' '; 20];
                    let size_len = format_decimal(sizes[index] as u64, &mut size_text);
                    let mut size_index = 0usize;
                    while size_index < size_len {
                        vga::write_ascii(window.y + 5 + row, window.x + window.width - 8 + size_index, size_text[size_index], attr);
                        size_index += 1;
                    }
                }
                row += 1;
            }
        }

        vga::write_line(window.y + window.height - 3, window.x + 2, "Status:", 0x1F);
        vga::write_line(window.y + window.height - 3, window.x + 10, self.explorer.status(), 0x1E);
        vga::write_line(window.y + window.height - 2, window.x + 2, "J/K move  Enter open  B back  N dir  T file  X delete", 0x17);
    }

    fn render_welcome(&self, window: &Window) {
        let lines = [
            "Welcome to Teddy Terminal.",
            "Terminal and Explorer share one filesystem.",
            "F1 launcher  F2 focus  F3 move mode",
            "F4 close     F5 reset layout",
            "Use WASD while move mode is active.",
        ];
        self.write_lines(window, &lines);
    }

    fn render_system(&self, window: &Window) {
        let mut line = window.y + 2;
        self.write_kv(line, window.x + 2, "Kernel", "terminal phase");
        line += 1;
        self.write_u64(line, window.x + 2, "Ticks", interrupts::timer_ticks());
        line += 1;
        self.write_u64(line, window.x + 2, "Uptime", self.uptime_seconds);
        line += 1;
        self.write_hex_byte(line, window.x + 2, "Last key", interrupts::last_ascii());
        line += 1;
        self.write_hex_byte(line, window.x + 2, "Scancode", interrupts::last_scancode());
        line += 1;
        self.write_boot_info(line, window.x + 2);
    }

    fn render_roadmap(&self, window: &Window) {
        let lines = [
            "Terminal now has parsing, scrollback, and fs stubs.",
            "Filesystem is shared by Terminal and Explorer.",
            "Still memory-backed in this phase.",
            "Later: framebuffer windows and mouse input.",
        ];
        self.write_lines(window, &lines);
    }

    fn render_launcher(&self) {
        let row = 13;
        let col = 1;
        let height = 10;
        let width = 29;
        vga::fill_rect(row, col, height, width, b' ', 0x1E);
        vga::draw_box(row, col, height, width, 0x1F);
        vga::fill_rect(row, col + 1, 1, width - 2, b' ', 0x70);
        vga::write_line(row, col + 2, "Teddy Launcher", 0x70);
        vga::write_line(row + 2, col + 2, "[1] Terminal", 0x1F);
        vga::write_line(row + 3, col + 2, "[2] Explorer", 0x1F);
        vga::write_line(row + 4, col + 2, "[3] Welcome", 0x1F);
        vga::write_line(row + 5, col + 2, "[4] System Monitor", 0x1F);
        vga::write_line(row + 6, col + 2, "[5] Roadmap", 0x1F);
        vga::write_line(row + 8, col + 2, "Esc closes launcher", 0x17);
    }

    fn render_taskbar(&self) {
        vga::fill_rect(TASKBAR_ROW, 0, 1, vga::width(), b' ', 0x70);
        vga::write_line(TASKBAR_ROW, 1, "[Teddy]", 0x7F);
        vga::write_line(TASKBAR_ROW, 10, self.focused_window_title(), 0x70);
        if self.move_mode {
            vga::write_line(TASKBAR_ROW, 30, "MOVE", 0x4F);
        } else {
            vga::write_line(TASKBAR_ROW, 30, "DESK", 0x2F);
        }
        vga::write_line(TASKBAR_ROW, 36, "F1 launch F2 focus F3 move", 0x70);

        let mut clock = [b' '; 16];
        let len = format_clock(self.uptime_seconds, &mut clock);
        let mut index = 0usize;
        while index < len {
            vga::write_ascii(TASKBAR_ROW, vga::width() - len - 2 + index, clock[index], 0x7F);
            index += 1;
        }
    }

    fn write_lines(&self, window: &Window, lines: &[&str]) {
        let mut index = 0usize;
        while index < lines.len() {
            if index + 2 >= window.height - 1 {
                break;
            }
            vga::write_line(window.y + 2 + index, window.x + 2, lines[index], 0x1E);
            index += 1;
        }
    }

    fn write_kv(&self, row: usize, col: usize, label: &str, value: &str) {
        vga::write_line(row, col, label, 0x1F);
        vga::write_line(row, col + 11, value, 0x1E);
    }

    fn write_u64(&self, row: usize, col: usize, label: &str, value: u64) {
        let mut buffer = [b' '; 20];
        let len = format_decimal(value, &mut buffer);
        vga::write_line(row, col, label, 0x1F);
        let mut index = 0usize;
        while index < len {
            vga::write_ascii(row, col + 11 + index, buffer[index], 0x1E);
            index += 1;
        }
    }

    fn write_hex_byte(&self, row: usize, col: usize, label: &str, value: u8) {
        vga::write_line(row, col, label, 0x1F);
        vga::write_hex_byte(row, col + 11, "", value, 0x1E);
    }

    fn write_boot_info(&self, row: usize, col: usize) {
        match self.boot_info {
            Some(info) => {
                vga::write_line(row, col, "Boot ver", 0x1F);
                vga::write_hex_byte(row, col + 11, "", info.version(), 0x1E);
                vga::write_line(row + 1, col, "Boot drv", 0x1F);
                vga::write_hex_byte(row + 1, col + 11, "", info.boot_drive(), 0x1E);
                vga::write_line(row + 2, col, "Kernel seg", 0x1F);
                vga::write_hex_word(row + 2, col + 11, "", info.kernel_segment(), 0x1E);
                vga::write_line(row + 3, col, "Kernel sec", 0x1F);
                vga::write_hex_word(row + 3, col + 11, "", info.kernel_sectors(), 0x1E);
                vga::write_line(row + 4, col, "Stage2 sec", 0x1F);
                vga::write_hex_word(row + 4, col + 11, "", info.stage2_sectors(), 0x1E);
            }
            None => vga::write_line(row, col, "Boot info unavailable", 0x4F),
        }
    }

    fn focused_kind(&self) -> WindowKind {
        self.windows[self.focus_index].kind
    }

    fn focus_next_window(&mut self) {
        for _ in 0..MAX_WINDOWS {
            self.focus_index = (self.focus_index + 1) % MAX_WINDOWS;
            if self.windows[self.focus_index].visible {
                return;
            }
        }
    }

    fn open_window(&mut self, kind: WindowKind) {
        let index = kind as usize;
        self.windows[index].visible = true;
        self.focus_index = index;
    }

    fn close_focused_window(&mut self) {
        if !self.windows[self.focus_index].visible {
            return;
        }

        self.windows[self.focus_index].visible = false;
        self.move_mode = false;
        if !self.has_visible_window() {
            self.focus_index = 0;
            self.windows[0].visible = true;
            return;
        }
        self.focus_next_window();
    }

    fn move_focused_window(&mut self, ascii: u8) {
        let window = &mut self.windows[self.focus_index];
        if !window.visible {
            return;
        }

        match ascii {
            b'w' if window.y > 2 => window.y -= 1,
            b's' if window.y + window.height < TASKBAR_ROW => window.y += 1,
            b'a' if window.x > 1 => window.x -= 1,
            b'd' if window.x + window.width < vga::width() - 1 => window.x += 1,
            _ => {}
        }
    }

    fn reset_layout(&mut self) {
        self.windows[WindowKind::Terminal as usize] = Window::new(WindowKind::Terminal, 2, 2, 60, 12, true);
        self.windows[WindowKind::Explorer as usize] = Window::new(WindowKind::Explorer, 2, 14, 60, 10, true);
        self.windows[WindowKind::Welcome as usize] = Window::new(WindowKind::Welcome, 47, 2, 30, 9, true);
        self.windows[WindowKind::System as usize] = Window::new(WindowKind::System, 45, 11, 32, 12, false);
        self.windows[WindowKind::Roadmap as usize] = Window::new(WindowKind::Roadmap, 8, 15, 36, 8, false);
        self.focus_index = 0;
        self.launcher_open = false;
        self.move_mode = false;
    }

    fn has_visible_window(&self) -> bool {
        let mut index = 0usize;
        while index < self.windows.len() {
            if self.windows[index].visible {
                return true;
            }
            index += 1;
        }
        false
    }

    fn focused_window_title(&self) -> &str {
        if self.windows[self.focus_index].visible {
            self.windows[self.focus_index].kind.title()
        } else {
            "Desktop"
        }
    }
}

#[derive(Clone, Copy)]
struct Window {
    kind: WindowKind,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    visible: bool,
}

impl Window {
    const fn new(kind: WindowKind, x: usize, y: usize, width: usize, height: usize, visible: bool) -> Self {
        Self {
            kind,
            x,
            y,
            width,
            height,
            visible,
        }
    }

    const fn hidden(kind: WindowKind) -> Self {
        Self::new(kind, 0, 0, 2, 2, false)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum WindowKind {
    Terminal = 0,
    Explorer = 1,
    Welcome = 2,
    System = 3,
    Roadmap = 4,
}

impl WindowKind {
    const fn title(self) -> &'static str {
        match self {
            Self::Terminal => "Terminal",
            Self::Explorer => "File Explorer",
            Self::Welcome => "Welcome",
            Self::System => "System Monitor",
            Self::Roadmap => "Roadmap",
        }
    }
}

fn format_decimal(mut value: u64, buffer: &mut [u8; 20]) -> usize {
    if value == 0 {
        buffer[0] = b'0';
        return 1;
    }

    let mut scratch = [0u8; 20];
    let mut len = 0;
    while value > 0 {
        scratch[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    for index in 0..len {
        buffer[index] = scratch[len - 1 - index];
    }
    len
}

fn format_clock(uptime_seconds: u64, buffer: &mut [u8; 16]) -> usize {
    let hours = (uptime_seconds / 3600) % 24;
    let minutes = (uptime_seconds / 60) % 60;
    let seconds = uptime_seconds % 60;

    buffer[0] = b'0' + (hours / 10) as u8;
    buffer[1] = b'0' + (hours % 10) as u8;
    buffer[2] = b':';
    buffer[3] = b'0' + (minutes / 10) as u8;
    buffer[4] = b'0' + (minutes % 10) as u8;
    buffer[5] = b':';
    buffer[6] = b'0' + (seconds / 10) as u8;
    buffer[7] = b'0' + (seconds % 10) as u8;
    8
}
