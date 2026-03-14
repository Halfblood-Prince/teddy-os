use core::fmt::{self, Write};

use spin::Mutex;

use crate::{
    framebuffer::{Color, FramebufferSurface, Rect},
    input::{KeyKind, KeyboardEvent},
    interrupts,
};

const LINE_CAPACITY: usize = 96;
const SCROLLBACK_LINES: usize = 160;
const INPUT_CAPACITY: usize = 96;
const MAX_NODES: usize = 64;
const MAX_NAME: usize = 24;
const MAX_FILE_BYTES: usize = 384;
const MAX_BODY_LINES: usize = 24;

#[derive(Clone, Copy)]
struct LineBuffer {
    bytes: [u8; LINE_CAPACITY],
    len: usize,
}

impl LineBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0; LINE_CAPACITY],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn push_str(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let write_len = bytes.len().min(self.bytes.len().saturating_sub(self.len));
        self.bytes[self.len..self.len + write_len].copy_from_slice(&bytes[..write_len]);
        self.len += write_len;
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("?")
    }
}

impl Write for LineBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum NodeKind {
    Directory,
    File,
}

#[derive(Clone, Copy)]
struct FsNode {
    used: bool,
    parent: usize,
    kind: NodeKind,
    name: [u8; MAX_NAME],
    name_len: usize,
    content: [u8; MAX_FILE_BYTES],
    content_len: usize,
}

impl FsNode {
    const fn empty() -> Self {
        Self {
            used: false,
            parent: 0,
            kind: NodeKind::Directory,
            name: [0; MAX_NAME],
            name_len: 0,
            content: [0; MAX_FILE_BYTES],
            content_len: 0,
        }
    }

    fn set_name(&mut self, name: &str) {
        self.name_len = 0;
        let bytes = name.as_bytes();
        let write_len = bytes.len().min(MAX_NAME);
        self.name[..write_len].copy_from_slice(&bytes[..write_len]);
        self.name_len = write_len;
    }

    fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("?")
    }

    fn set_content(&mut self, content: &str) {
        self.content_len = 0;
        let bytes = content.as_bytes();
        let write_len = bytes.len().min(MAX_FILE_BYTES);
        self.content[..write_len].copy_from_slice(&bytes[..write_len]);
        self.content_len = write_len;
    }

    fn content(&self) -> &str {
        core::str::from_utf8(&self.content[..self.content_len]).unwrap_or("?")
    }
}

struct TerminalFs {
    nodes: [FsNode; MAX_NODES],
    cwd: usize,
}

impl TerminalFs {
    fn new() -> Self {
        let mut fs = Self {
            nodes: [FsNode::empty(); MAX_NODES],
            cwd: 0,
        };

        fs.nodes[0].used = true;
        fs.nodes[0].parent = 0;
        fs.nodes[0].kind = NodeKind::Directory;
        fs.nodes[0].set_name("");

        let docs = fs.create_child(0, "docs", NodeKind::Directory).unwrap_or(0);
        let tmp = fs.create_child(0, "tmp", NodeKind::Directory).unwrap_or(0);
        let readme = fs.create_child(0, "readme.txt", NodeKind::File).unwrap_or(0);
        let notes = fs.create_child(docs, "notes.txt", NodeKind::File).unwrap_or(0);
        let todo = fs.create_child(tmp, "todo.txt", NodeKind::File).unwrap_or(0);

        fs.nodes[readme].set_content("Welcome to Teddy-OS terminal.\nPhase 5 will replace this in-memory filesystem.");
        fs.nodes[notes].set_content("Use help to list built-in commands.\nUse ls, cd, pwd, cat, mkdir, rm, and touch.");
        fs.nodes[todo].set_content("Build terminal MVP\nAdd persistent filesystem in Phase 5");
        fs
    }

    fn pwd(&self) -> LineBuffer {
        self.path_for(self.cwd)
    }

    fn ls(&self, path: Option<&str>, out: &mut [LineBuffer; MAX_BODY_LINES]) -> Result<usize, &'static str> {
        let index = self.resolve(path.unwrap_or("."))?;
        match self.nodes[index].kind {
            NodeKind::File => {
                out[0].clear();
                out[0].push_str(self.nodes[index].name());
                Ok(1)
            }
            NodeKind::Directory => {
                let mut count = 0usize;
                for node in self.nodes.iter() {
                    if node.used && node.parent == index && node.name_len > 0 && count < out.len() {
                        out[count].clear();
                        out[count].push_str(node.name());
                        if matches!(node.kind, NodeKind::Directory) {
                            out[count].push_str("/");
                        }
                        count += 1;
                    }
                }
                if count == 0 {
                    out[0].clear();
                    out[0].push_str("<empty>");
                    Ok(1)
                } else {
                    Ok(count)
                }
            }
        }
    }

    fn cd(&mut self, path: &str) -> Result<(), &'static str> {
        let index = self.resolve(path)?;
        match self.nodes[index].kind {
            NodeKind::Directory => {
                self.cwd = index;
                Ok(())
            }
            NodeKind::File => Err("cd: not a directory"),
        }
    }

    fn cat(&self, path: &str, out: &mut [LineBuffer; MAX_BODY_LINES]) -> Result<usize, &'static str> {
        let index = self.resolve(path)?;
        match self.nodes[index].kind {
            NodeKind::File => {
                let mut count = 0usize;
                for segment in self.nodes[index].content().split('\n') {
                    if count >= out.len() {
                        break;
                    }
                    out[count].clear();
                    out[count].push_str(segment);
                    count += 1;
                }
                Ok(count.max(1))
            }
            NodeKind::Directory => Err("cat: path is a directory"),
        }
    }

    fn mkdir(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent(path)?;
        if self.find_child(parent, name).is_some() {
            return Err("mkdir: entry already exists");
        }
        self.create_child(parent, name, NodeKind::Directory)
            .map(|_| ())
            .ok_or("mkdir: no free nodes")
    }

    fn touch(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent(path)?;
        if let Some(index) = self.find_child(parent, name) {
            match self.nodes[index].kind {
                NodeKind::File => return Ok(()),
                NodeKind::Directory => return Err("touch: path is a directory"),
            }
        }

        self.create_child(parent, name, NodeKind::File)
            .map(|_| ())
            .ok_or("touch: no free nodes")
    }

    fn rm(&mut self, path: &str) -> Result<(), &'static str> {
        let index = self.resolve(path)?;
        if index == 0 {
            return Err("rm: refusing to remove root");
        }

        if matches!(self.nodes[index].kind, NodeKind::Directory) {
            for node in self.nodes.iter() {
                if node.used && node.parent == index {
                    return Err("rm: directory not empty");
                }
            }
        }

        self.nodes[index] = FsNode::empty();
        Ok(())
    }

    fn resolve(&self, path: &str) -> Result<usize, &'static str> {
        if path.is_empty() || path == "." {
            return Ok(self.cwd);
        }

        let mut current = if path.starts_with('/') { 0 } else { self.cwd };
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            match segment {
                "." => {}
                ".." => current = self.nodes[current].parent,
                name => {
                    current = self
                        .find_child(current, name)
                        .ok_or("path not found")?;
                }
            }
        }

        Ok(current)
    }

    fn resolve_parent(&self, path: &str) -> Result<(usize, &str), &'static str> {
        let trimmed = path.trim_end_matches('/');
        if trimmed.is_empty() {
            return Err("invalid path");
        }

        if let Some((parent_path, name)) = trimmed.rsplit_once('/') {
            let parent = if parent_path.is_empty() {
                0
            } else {
                self.resolve(parent_path)?
            };
            if name.is_empty() {
                Err("invalid path")
            } else {
                Ok((parent, name))
            }
        } else {
            Ok((self.cwd, trimmed))
        }
    }

    fn find_child(&self, parent: usize, name: &str) -> Option<usize> {
        self.nodes.iter().enumerate().find_map(|(index, node)| {
            if node.used && node.parent == parent && node.name() == name {
                Some(index)
            } else {
                None
            }
        })
    }

    fn create_child(&mut self, parent: usize, name: &str, kind: NodeKind) -> Option<usize> {
        let slot = self.nodes.iter().position(|node| !node.used)?;
        self.nodes[slot].used = true;
        self.nodes[slot].parent = parent;
        self.nodes[slot].kind = kind;
        self.nodes[slot].set_name(name);
        self.nodes[slot].content_len = 0;
        Some(slot)
    }

    fn path_for(&self, index: usize) -> LineBuffer {
        if index == 0 {
            let mut root = LineBuffer::new();
            root.push_str("/");
            return root;
        }

        let mut segments = [[0u8; MAX_NAME]; 8];
        let mut lengths = [0usize; 8];
        let mut count = 0usize;
        let mut current = index;

        while current != 0 && count < segments.len() {
            let node = self.nodes[current];
            segments[count][..node.name_len].copy_from_slice(&node.name[..node.name_len]);
            lengths[count] = node.name_len;
            count += 1;
            current = node.parent;
        }

        let mut path = LineBuffer::new();
        path.push_str("/");
        for segment in (0..count).rev() {
            let text = core::str::from_utf8(&segments[segment][..lengths[segment]]).unwrap_or("?");
            path.push_str(text);
            if segment != 0 {
                path.push_str("/");
            }
        }
        path
    }
}

pub struct TerminalApp {
    lines: [LineBuffer; SCROLLBACK_LINES],
    line_count: usize,
    next_index: usize,
    input: [u8; INPUT_CAPACITY],
    input_len: usize,
    history: [LineBuffer; 24],
    history_len: usize,
    history_index: usize,
    scratch: [LineBuffer; MAX_BODY_LINES],
    fs: TerminalFs,
}

impl TerminalApp {
    fn new() -> Self {
        let mut terminal = Self {
            lines: [LineBuffer::new(); SCROLLBACK_LINES],
            line_count: 0,
            next_index: 0,
            input: [0; INPUT_CAPACITY],
            input_len: 0,
            history: [LineBuffer::new(); 24],
            history_len: 0,
            history_index: 0,
            scratch: [LineBuffer::new(); MAX_BODY_LINES],
            fs: TerminalFs::new(),
        };

        terminal.println("Teddy Terminal");
        terminal.println("Type 'help' for commands.");
        terminal.prompt();
        terminal
    }

    fn clear(&mut self) {
        self.lines = [LineBuffer::new(); SCROLLBACK_LINES];
        self.line_count = 0;
        self.next_index = 0;
    }

    fn println(&mut self, text: &str) {
        let mut line = LineBuffer::new();
        line.push_str(text);
        self.push_line(line);
    }

    fn push_line(&mut self, line: LineBuffer) {
        self.lines[self.next_index] = line;
        self.next_index = (self.next_index + 1) % SCROLLBACK_LINES;
        if self.line_count < SCROLLBACK_LINES {
            self.line_count += 1;
        }
    }

    fn prompt(&mut self) {
        let path = self.fs.pwd();
        let mut line = LineBuffer::new();
        let _ = write!(line, "{}> {}", path.as_str(), self.current_input());
        self.push_line(line);
    }

    fn replace_prompt(&mut self) {
        if self.line_count == 0 {
            self.prompt();
            return;
        }

        let index = (self.next_index + SCROLLBACK_LINES - 1) % SCROLLBACK_LINES;
        let path = self.fs.pwd();
        let mut line = LineBuffer::new();
        let _ = write!(line, "{}> {}", path.as_str(), self.current_input());
        self.lines[index] = line;
    }

    fn current_input(&self) -> &str {
        core::str::from_utf8(&self.input[..self.input_len]).unwrap_or("")
    }

    fn handle_event(&mut self, event: KeyboardEvent) {
        if !event.pressed {
            return;
        }

        match event.key_kind {
            KeyKind::Character => {
                if let Some(character) = event.unicode {
                    if character >= ' ' && self.input_len < INPUT_CAPACITY {
                        self.input[self.input_len] = character as u8;
                        self.input_len += 1;
                        self.replace_prompt();
                    }
                }
            }
            KeyKind::Backspace => {
                if self.input_len > 0 {
                    self.input_len -= 1;
                    self.replace_prompt();
                }
            }
            KeyKind::Enter => {
                self.execute_current_line();
            }
            KeyKind::ArrowUp => {
                self.history_up();
            }
            KeyKind::ArrowDown => {
                self.history_down();
            }
            _ => {}
        }
    }

    fn history_up(&mut self) {
        if self.history_len == 0 {
            return;
        }
        if self.history_index == 0 {
            self.history_index = self.history_len - 1;
        } else {
            self.history_index -= 1;
        }
        let entry = self.history[self.history_index];
        self.input_len = entry.len.min(INPUT_CAPACITY);
        self.input[..self.input_len].copy_from_slice(&entry.bytes[..self.input_len]);
        self.replace_prompt();
    }

    fn history_down(&mut self) {
        if self.history_len == 0 {
            return;
        }
        self.history_index = (self.history_index + 1) % self.history_len;
        let entry = self.history[self.history_index];
        self.input_len = entry.len.min(INPUT_CAPACITY);
        self.input[..self.input_len].copy_from_slice(&entry.bytes[..self.input_len]);
        self.replace_prompt();
    }

    fn execute_current_line(&mut self) {
        let mut command_line_storage = LineBuffer::new();
        command_line_storage.push_str(self.current_input());
        let command_line = command_line_storage.as_str();
        let mut line = LineBuffer::new();
        let path = self.fs.pwd();
        let _ = write!(line, "{}> {}", path.as_str(), command_line);
        let index = (self.next_index + SCROLLBACK_LINES - 1) % SCROLLBACK_LINES;
        self.lines[index] = line;

        if !command_line.is_empty() {
            let history_slot = self.history_len.min(self.history.len() - 1);
            if self.history_len < self.history.len() {
                self.history_len += 1;
            }
            let mut entry = LineBuffer::new();
            entry.push_str(command_line);
            self.history[history_slot] = entry;
            self.history_index = self.history_len.saturating_sub(1);
        }

        let mut parser = CommandParser::new(command_line);
        if let Some(command) = parser.next() {
            self.run_command(command, &mut parser);
        }

        self.input_len = 0;
        self.prompt();
    }

    fn run_command(&mut self, command: &str, parser: &mut CommandParser<'_>) {
        match command {
            "help" => {
                self.println("help echo clear ls cd pwd cat mkdir rm touch uname reboot shutdown");
            }
            "echo" => {
                self.println(parser.rest());
            }
            "clear" => {
                self.clear();
            }
            "ls" => {
                match self.fs.ls(parser.next(), &mut self.scratch) {
                    Ok(count) => {
                        let lines = self.scratch;
                        for index in 0..count {
                            self.println(lines[index].as_str());
                        }
                    }
                    Err(error) => self.println(error),
                }
            }
            "cd" => {
                if let Some(path) = parser.next() {
                    if let Err(error) = self.fs.cd(path) {
                        self.println(error);
                    }
                } else {
                    self.println("cd: missing path");
                }
            }
            "pwd" => {
                let path = self.fs.pwd();
                self.println(path.as_str());
            }
            "cat" => {
                if let Some(path) = parser.next() {
                    match self.fs.cat(path, &mut self.scratch) {
                        Ok(count) => {
                            let lines = self.scratch;
                            for index in 0..count {
                                self.println(lines[index].as_str());
                            }
                        }
                        Err(error) => self.println(error),
                    }
                } else {
                    self.println("cat: missing path");
                }
            }
            "mkdir" => {
                if let Some(path) = parser.next() {
                    if let Err(error) = self.fs.mkdir(path) {
                        self.println(error);
                    }
                } else {
                    self.println("mkdir: missing path");
                }
            }
            "rm" => {
                if let Some(path) = parser.next() {
                    if let Err(error) = self.fs.rm(path) {
                        self.println(error);
                    }
                } else {
                    self.println("rm: missing path");
                }
            }
            "touch" => {
                if let Some(path) = parser.next() {
                    if let Err(error) = self.fs.touch(path) {
                        self.println(error);
                    }
                } else {
                    self.println("touch: missing path");
                }
            }
            "uname" => {
                self.println("Teddy-OS x86_64 phase4");
            }
            "reboot" => {
                self.println("Rebooting Teddy-OS...");
                reboot_system();
            }
            "shutdown" => {
                self.println("Shutting down Teddy-OS...");
                shutdown_system();
            }
            _ => {
                self.println("unknown command");
            }
        }
    }

    fn render(&self, surface: &mut FramebufferSurface, rect: Rect, focused: bool) {
        let bg = Color::rgb(0x0C, 0x12, 0x18);
        let fg = Color::rgb(0xC7, 0xD5, 0xE3);
        let accent = if focused {
            Color::rgb(0x7F, 0xCF, 0x93)
        } else {
            Color::rgb(0x6F, 0x7D, 0x88)
        };
        surface.fill_rect(rect, bg);
        surface.fill_rect(
            Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: 2,
            },
            accent,
        );

        let visible_lines = (rect.height / 16).saturating_sub(1).min(MAX_BODY_LINES);
        let start = self.line_count.saturating_sub(visible_lines);

        let mut y = rect.y + 6;
        for offset in 0..visible_lines {
            let Some(line) = self.scrollback_line(start + offset) else {
                continue;
            };
            surface.draw_text(line.as_str(), rect.x + 8, y, fg, bg);
            y += 16;
        }
    }

    fn scrollback_line(&self, logical_index: usize) -> Option<&LineBuffer> {
        if logical_index >= self.line_count {
            return None;
        }

        let oldest = (self.next_index + SCROLLBACK_LINES - self.line_count) % SCROLLBACK_LINES;
        let index = (oldest + logical_index) % SCROLLBACK_LINES;
        Some(&self.lines[index])
    }
}

struct CommandParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> CommandParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    fn next(&mut self) -> Option<&'a str> {
        let bytes = self.input.as_bytes();
        while self.cursor < bytes.len() && bytes[self.cursor].is_ascii_whitespace() {
            self.cursor += 1;
        }
        if self.cursor >= bytes.len() {
            return None;
        }
        let start = self.cursor;
        while self.cursor < bytes.len() && !bytes[self.cursor].is_ascii_whitespace() {
            self.cursor += 1;
        }
        Some(&self.input[start..self.cursor])
    }

    fn rest(&mut self) -> &'a str {
        let bytes = self.input.as_bytes();
        while self.cursor < bytes.len() && bytes[self.cursor].is_ascii_whitespace() {
            self.cursor += 1;
        }
        &self.input[self.cursor..]
    }
}

static TERMINAL: Mutex<Option<TerminalApp>> = Mutex::new(None);

pub fn init() {
    *TERMINAL.lock() = Some(TerminalApp::new());
}

pub fn handle_keyboard_event(event: KeyboardEvent) {
    let mut guard = TERMINAL.lock();
    if let Some(terminal) = guard.as_mut() {
        terminal.handle_event(event);
    }
}

pub fn render(surface: &mut FramebufferSurface, rect: Rect, focused: bool) {
    let guard = TERMINAL.lock();
    if let Some(terminal) = guard.as_ref() {
        terminal.render(surface, rect, focused);
    }
}

fn reboot_system() -> ! {
    interrupts::disable();
    unsafe {
        use x86_64::instructions::port::Port;

        let mut command = Port::<u8>::new(0x64);
        command.write(0xFE);
    }
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

fn shutdown_system() -> ! {
    interrupts::disable();
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
