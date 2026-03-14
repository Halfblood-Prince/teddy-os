use core::fmt::Write;

use spin::Mutex;

use crate::{
    framebuffer::{Color, FramebufferSurface, Rect},
    fs::{self, DirectoryEntry, FsTextBuffer, MAX_DIR_ENTRIES, MAX_OUTPUT_LINES},
    timer,
};

const HEADER_HEIGHT: usize = 30;
const TOOLBAR_HEIGHT: usize = 32;
const ROW_HEIGHT: usize = 18;
const DOUBLE_CLICK_TICKS: u64 = 30;

#[derive(Clone, Copy)]
enum ExplorerAction {
    Up,
    NewFolder,
    Rename,
    Delete,
}

struct ExplorerState {
    current_path: FsTextBuffer,
    entries: [DirectoryEntry; MAX_DIR_ENTRIES],
    entry_count: usize,
    selected: Option<usize>,
    last_clicked: Option<usize>,
    last_click_tick: u64,
    preview: [FsTextBuffer; MAX_OUTPUT_LINES],
    preview_count: usize,
    status: FsTextBuffer,
    new_folder_counter: u32,
    rename_counter: u32,
}

impl ExplorerState {
    fn new() -> Self {
        let mut state = Self {
            current_path: FsTextBuffer::new(),
            entries: [DirectoryEntry {
                name: FsTextBuffer::new(),
                is_dir: false,
                size: 0,
            }; MAX_DIR_ENTRIES],
            entry_count: 0,
            selected: None,
            last_clicked: None,
            last_click_tick: 0,
            preview: [FsTextBuffer::new(); MAX_OUTPUT_LINES],
            preview_count: 0,
            status: FsTextBuffer::new(),
            new_folder_counter: 1,
            rename_counter: 1,
        };
        state.refresh();
        state
    }

    fn refresh(&mut self) {
        self.current_path = fs::pwd().unwrap_or_else(|_| {
            let mut text = FsTextBuffer::new();
            text.push_str("/unmounted");
            text
        });
        self.entry_count = fs::list_dir_entries(self.current_path.as_str(), &mut self.entries)
            .unwrap_or(0);
        if let Some(selected) = self.selected {
            if selected >= self.entry_count {
                self.selected = None;
                self.preview_count = 0;
            }
        }
        if self.status.as_str().is_empty() {
            self.status.push_str("Explorer ready");
        }
    }
}

static EXPLORER: Mutex<Option<ExplorerState>> = Mutex::new(None);

pub fn init() {
    *EXPLORER.lock() = Some(ExplorerState::new());
}

pub fn render(surface: &mut FramebufferSurface, rect: Rect, focused: bool) {
    let mut guard = EXPLORER.lock();
    let Some(explorer) = guard.as_mut() else {
        return;
    };
    explorer.refresh();

    let bg = Color::rgb(0xF7, 0xFA, 0xFD);
    let panel = Color::rgb(0xE8, 0xEF, 0xF6);
    let text = Color::rgb(0x12, 0x19, 0x22);
    let muted = Color::rgb(0x4C, 0x5A, 0x6A);
    let accent = if focused {
        Color::rgb(0x2E, 0x67, 0x97)
    } else {
        Color::rgb(0x6B, 0x7C, 0x8D)
    };

    surface.fill_rect(rect, bg);
    surface.fill_rect(
        Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: HEADER_HEIGHT,
        },
        panel,
    );
    surface.fill_rect(
        Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: 2,
        },
        accent,
    );
    surface.draw_text(
        explorer.current_path.as_str(),
        rect.x + 12,
        rect.y + 10,
        text,
        panel,
    );

    draw_toolbar(surface, rect, focused);

    let list_rect = Rect {
        x: rect.x + 8,
        y: rect.y + HEADER_HEIGHT + TOOLBAR_HEIGHT + 4,
        width: rect.width / 2,
        height: rect.height.saturating_sub(HEADER_HEIGHT + TOOLBAR_HEIGHT + 40),
    };
    let preview_rect = Rect {
        x: rect.x + rect.width / 2 + 10,
        y: rect.y + HEADER_HEIGHT + TOOLBAR_HEIGHT + 4,
        width: rect.width / 2 - 18,
        height: rect.height.saturating_sub(HEADER_HEIGHT + TOOLBAR_HEIGHT + 40),
    };

    surface.fill_rect(list_rect, Color::rgb(0xFC, 0xFD, 0xFF));
    surface.stroke_rect(list_rect, Color::rgb(0xB5, 0xC3, 0xD3));
    surface.fill_rect(preview_rect, Color::rgb(0xFD, 0xFE, 0xFF));
    surface.stroke_rect(preview_rect, Color::rgb(0xB5, 0xC3, 0xD3));

    surface.draw_text("Files", list_rect.x + 8, list_rect.y + 8, text, Color::rgb(0xFC, 0xFD, 0xFF));
    surface.draw_text("Preview", preview_rect.x + 8, preview_rect.y + 8, text, Color::rgb(0xFD, 0xFE, 0xFF));

    let visible = ((list_rect.height.saturating_sub(22)) / ROW_HEIGHT).min(explorer.entry_count);
    let mut row_y = list_rect.y + 28;
    for index in 0..visible {
        let entry = explorer.entries[index];
        let row_bg = if explorer.selected == Some(index) {
            Color::rgb(0xD8, 0xE9, 0xF9)
        } else {
            Color::rgb(0xFC, 0xFD, 0xFF)
        };
        surface.fill_rect(
            Rect {
                x: list_rect.x + 4,
                y: row_y - 2,
                width: list_rect.width - 8,
                height: ROW_HEIGHT,
            },
            row_bg,
        );
        let icon = if entry.is_dir { "[D]" } else { "[F]" };
        surface.draw_text(icon, list_rect.x + 10, row_y, muted, row_bg);
        surface.draw_text(entry.name.as_str(), list_rect.x + 42, row_y, text, row_bg);

        let mut size = FsTextBuffer::new();
        let _ = write!(size, "{}b", entry.size);
        surface.draw_text(
            size.as_str(),
            list_rect.x + list_rect.width.saturating_sub(60),
            row_y,
            muted,
            row_bg,
        );
        row_y += ROW_HEIGHT;
    }

    let mut preview_y = preview_rect.y + 28;
    for index in 0..explorer.preview_count.min(MAX_OUTPUT_LINES) {
        surface.draw_text(
            explorer.preview[index].as_str(),
            preview_rect.x + 10,
            preview_y,
            text,
            Color::rgb(0xFD, 0xFE, 0xFF),
        );
        preview_y += 16;
    }

    surface.fill_rect(
        Rect {
            x: rect.x,
            y: rect.y + rect.height.saturating_sub(24),
            width: rect.width,
            height: 24,
        },
        panel,
    );
    surface.draw_text(
        explorer.status.as_str(),
        rect.x + 10,
        rect.y + rect.height.saturating_sub(16),
        muted,
        panel,
    );
}

pub fn handle_click(local_x: usize, local_y: usize, body_rect: Rect, tick: u64) {
    let mut guard = EXPLORER.lock();
    let Some(explorer) = guard.as_mut() else {
        return;
    };

    if local_y < HEADER_HEIGHT + TOOLBAR_HEIGHT {
        if let Some(action) = toolbar_hit(local_x, local_y) {
            run_action(explorer, action, tick);
        }
        return;
    }

    let list_x = 8;
    let list_y = HEADER_HEIGHT + TOOLBAR_HEIGHT + 4;
    let list_width = body_rect.width / 2;
    let list_height = body_rect.height.saturating_sub(HEADER_HEIGHT + TOOLBAR_HEIGHT + 40);
    if local_x >= list_x
        && local_x < list_x + list_width
        && local_y >= list_y + 24
        && local_y < list_y + list_height
    {
        let row = (local_y - (list_y + 24)) / ROW_HEIGHT;
        if row < explorer.entry_count {
            explorer.selected = Some(row);
            preview_entry(explorer, row);
            if explorer.last_clicked == Some(row)
                && tick.saturating_sub(explorer.last_click_tick) <= DOUBLE_CLICK_TICKS
            {
                open_selected(explorer);
            }
            explorer.last_clicked = Some(row);
            explorer.last_click_tick = tick;
        }
    }
}

fn draw_toolbar(surface: &mut FramebufferSurface, rect: Rect, focused: bool) {
    let y = rect.y + HEADER_HEIGHT;
    let background = Color::rgb(0xF0, 0xF5, 0xFA);
    surface.fill_rect(
        Rect {
            x: rect.x,
            y,
            width: rect.width,
            height: TOOLBAR_HEIGHT,
        },
        background,
    );

    let buttons = ["Up", "New", "Rename", "Delete"];
    let mut x = rect.x + 8;
    for label in buttons {
        let button_bg = if focused {
            Color::rgb(0xD9, 0xE7, 0xF3)
        } else {
            Color::rgb(0xE3, 0xEA, 0xF2)
        };
        surface.fill_rect(
            Rect {
                x,
                y: y + 5,
                width: 62,
                height: 22,
            },
            button_bg,
        );
        surface.stroke_rect(
            Rect {
                x,
                y: y + 5,
                width: 62,
                height: 22,
            },
            Color::rgb(0x91, 0xA7, 0xBC),
        );
        surface.draw_text(label, x + 12, y + 11, Color::rgb(0x13, 0x1B, 0x24), button_bg);
        x += 70;
    }
}

fn toolbar_hit(local_x: usize, local_y: usize) -> Option<ExplorerAction> {
    if local_y < HEADER_HEIGHT || local_y >= HEADER_HEIGHT + TOOLBAR_HEIGHT {
        return None;
    }
    let button_x = [8usize, 78, 148, 218];
    for (index, x) in button_x.into_iter().enumerate() {
        if local_x >= x && local_x < x + 62 {
            return Some(match index {
                0 => ExplorerAction::Up,
                1 => ExplorerAction::NewFolder,
                2 => ExplorerAction::Rename,
                3 => ExplorerAction::Delete,
                _ => return None,
            });
        }
    }
    None
}

fn run_action(explorer: &mut ExplorerState, action: ExplorerAction, tick: u64) {
    match action {
        ExplorerAction::Up => {
            let current = explorer.current_path.as_str();
            if current != "/" {
                let parent = parent_path(current);
                if fs::cd(parent.as_str()).is_ok() {
                    explorer.status.clear();
                    explorer.status.push_str("Moved up");
                    explorer.selected = None;
                    explorer.preview_count = 0;
                    explorer.refresh();
                }
            }
        }
        ExplorerAction::NewFolder => {
            let mut name = FsTextBuffer::new();
            let _ = write!(name, "NewFolder{}", explorer.new_folder_counter);
            explorer.new_folder_counter += 1;
            let path = join_path(explorer.current_path.as_str(), name.as_str());
            match fs::mkdir(path.as_str(), tick) {
                Ok(()) => {
                    explorer.status.clear();
                    explorer.status.push_str("Folder created");
                    explorer.refresh();
                }
                Err(error) => {
                    explorer.status.clear();
                    explorer.status.push_str(error);
                }
            }
        }
        ExplorerAction::Rename => {
            if let Some(index) = explorer.selected {
                let entry = explorer.entries[index];
                let mut new_name = FsTextBuffer::new();
                let _ = write!(new_name, "{}-ren{}", entry.name.as_str(), explorer.rename_counter);
                explorer.rename_counter += 1;
                let path = join_path(explorer.current_path.as_str(), entry.name.as_str());
                match fs::rename(path.as_str(), new_name.as_str(), tick) {
                    Ok(()) => {
                        explorer.status.clear();
                        explorer.status.push_str("Renamed");
                        explorer.refresh();
                    }
                    Err(error) => {
                        explorer.status.clear();
                        explorer.status.push_str(error);
                    }
                }
            }
        }
        ExplorerAction::Delete => {
            if let Some(index) = explorer.selected {
                let entry = explorer.entries[index];
                let path = join_path(explorer.current_path.as_str(), entry.name.as_str());
                match fs::rm(path.as_str()) {
                    Ok(()) => {
                        explorer.status.clear();
                        explorer.status.push_str("Deleted");
                        explorer.selected = None;
                        explorer.preview_count = 0;
                        explorer.refresh();
                    }
                    Err(error) => {
                        explorer.status.clear();
                        explorer.status.push_str(error);
                    }
                }
            }
        }
    }
}

fn open_selected(explorer: &mut ExplorerState) {
    let Some(index) = explorer.selected else {
        return;
    };
    let entry = explorer.entries[index];
    let path = join_path(explorer.current_path.as_str(), entry.name.as_str());
    if entry.is_dir {
        if fs::cd(path.as_str()).is_ok() {
            explorer.status.clear();
            explorer.status.push_str("Opened folder");
            explorer.selected = None;
            explorer.preview_count = 0;
            explorer.refresh();
        }
    } else {
        preview_entry(explorer, index);
        explorer.status.clear();
        explorer.status.push_str("Opened file preview");
    }
}

fn preview_entry(explorer: &mut ExplorerState, index: usize) {
    explorer.preview_count = 0;
    if index >= explorer.entry_count {
        return;
    }

    let entry = explorer.entries[index];
    let path = join_path(explorer.current_path.as_str(), entry.name.as_str());
    if entry.is_dir {
        explorer.preview[0].clear();
        explorer.preview[0].push_str("Folder");
        explorer.preview[1].clear();
        explorer.preview[1].push_str(entry.name.as_str());
        explorer.preview_count = 2;
    } else {
        match fs::cat(path.as_str(), &mut explorer.preview) {
            Ok(count) => explorer.preview_count = count,
            Err(error) => {
                explorer.preview[0].clear();
                explorer.preview[0].push_str(error);
                explorer.preview_count = 1;
            }
        }
    }
}

fn join_path(base: &str, name: &str) -> FsTextBuffer {
    let mut path = FsTextBuffer::new();
    if base == "/" {
        path.push_str("/");
        path.push_str(name);
    } else {
        path.push_str(base);
        path.push_str("/");
        path.push_str(name);
    }
    path
}

fn parent_path(path: &str) -> FsTextBuffer {
    if path == "/" {
        let mut root = FsTextBuffer::new();
        root.push_str("/");
        return root;
    }

    let trimmed = path.trim_end_matches('/');
    if let Some((parent, _)) = trimmed.rsplit_once('/') {
        let mut result = FsTextBuffer::new();
        if parent.is_empty() {
            result.push_str("/");
        } else {
            result.push_str(parent);
        }
        result
    } else {
        let mut root = FsTextBuffer::new();
        root.push_str("/");
        root
    }
}
