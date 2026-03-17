const MAX_HISTORY_LINES: usize = 18;
const MAX_LINE_LEN: usize = 58;
const INPUT_BUFFER_LEN: usize = 58;
const MAX_FS_NODES: usize = 16;
const MAX_NAME_LEN: usize = 12;
const MAX_FILE_LEN: usize = 96;

pub enum TerminalAction {
    None,
    Reboot,
    Shutdown,
}

pub struct TerminalApp {
    fs: MiniFs,
    history: [HistoryLine; MAX_HISTORY_LINES],
    history_len: usize,
    input: [u8; INPUT_BUFFER_LEN],
    input_len: usize,
}

impl TerminalApp {
    pub fn new() -> Self {
        let mut app = Self {
            fs: MiniFs::new(),
            history: [HistoryLine::empty(); MAX_HISTORY_LINES],
            history_len: 0,
            input: [0; INPUT_BUFFER_LEN],
            input_len: 0,
        };

        app.push_line("Teddy Terminal ready.");
        app.push_line("Type 'help' for commands.");
        app
    }

    pub fn handle_key(&mut self, ascii: u8) -> TerminalAction {
        match ascii {
            8 => {
                if self.input_len > 0 {
                    self.input_len -= 1;
                }
                TerminalAction::None
            }
            b'\n' => self.submit_command(),
            0x20..=0x7E => {
                if self.input_len < INPUT_BUFFER_LEN {
                    self.input[self.input_len] = ascii;
                    self.input_len += 1;
                }
                TerminalAction::None
            }
            _ => TerminalAction::None,
        }
    }

    pub fn history_len(&self) -> usize {
        self.history_len
    }

    pub fn history_line(&self, index: usize) -> &str {
        self.history[index].as_str()
    }

    pub fn input(&self) -> &str {
        core::str::from_utf8(&self.input[..self.input_len]).unwrap_or("")
    }

    pub fn cwd(&self) -> &str {
        self.fs.cwd_path()
    }

    fn submit_command(&mut self) -> TerminalAction {
        let command_buffer = self.input;
        let command_len = self.input_len;
        let command = core::str::from_utf8(&command_buffer[..command_len]).unwrap_or("");

        self.push_prompt_line(command);

        let action = if command.is_empty() {
            TerminalAction::None
        } else {
            self.execute(command)
        };

        self.input_len = 0;
        action
    }

    fn execute(&mut self, command: &str) -> TerminalAction {
        if command == "help" {
            self.push_line("help echo clear ls cd pwd cat mkdir rm");
            self.push_line("touch uname reboot shutdown");
            return TerminalAction::None;
        }
        if command == "clear" {
            self.clear_history();
            return TerminalAction::None;
        }
        if let Some(text) = command.strip_prefix("echo ") {
            self.push_line(text);
            return TerminalAction::None;
        }
        if command == "echo" {
            self.push_line("");
            return TerminalAction::None;
        }
        if command == "ls" {
            let (lines, count) = self.fs.listing(self.fs.cwd);
            for line in lines.iter().take(count) {
                self.push_history(*line);
            }
            return TerminalAction::None;
        }
        if let Some(path) = command.strip_prefix("cd ") {
            match self.fs.change_dir(path) {
                Ok(()) => {}
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if command == "pwd" {
            self.push_history(self.fs.cwd_line());
            return TerminalAction::None;
        }
        if let Some(path) = command.strip_prefix("cat ") {
            match self.fs.read_file(path) {
                Ok(content) => self.push_history(content),
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if let Some(path) = command.strip_prefix("mkdir ") {
            match self.fs.create_dir(path) {
                Ok(()) => self.push_line("directory created"),
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if let Some(path) = command.strip_prefix("rm ") {
            match self.fs.remove(path) {
                Ok(()) => self.push_line("removed"),
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if let Some(path) = command.strip_prefix("touch ") {
            match self.fs.touch(path) {
                Ok(()) => self.push_line("file updated"),
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if command == "uname" {
            self.push_line("Teddy-OS 0.1 text-shell x86_64");
            return TerminalAction::None;
        }
        if command == "reboot" {
            self.push_line("rebooting Teddy-OS...");
            return TerminalAction::Reboot;
        }
        if command == "shutdown" {
            self.push_line("system halted");
            return TerminalAction::Shutdown;
        }

        self.push_line("unknown command");
        TerminalAction::None
    }

    fn clear_history(&mut self) {
        self.history = [HistoryLine::empty(); MAX_HISTORY_LINES];
        self.history_len = 0;
    }

    fn push_prompt_line(&mut self, command: &str) {
        let mut line = HistoryLine::empty();
        line.push_str(self.fs.cwd_path());
        line.push_str(" $ ");
        line.push_str(command);
        self.push_history(line);
    }

    fn push_line(&mut self, text: &str) {
        let mut remaining = text.as_bytes();
        if remaining.is_empty() {
            self.push_history(HistoryLine::empty());
            return;
        }

        while !remaining.is_empty() {
            let mut line = HistoryLine::empty();
            let take = core::cmp::min(remaining.len(), MAX_LINE_LEN);
            for byte in &remaining[..take] {
                line.push_byte(*byte);
            }
            self.push_history(line);
            remaining = &remaining[take..];
        }
    }

    fn push_history(&mut self, line: HistoryLine) {
        if self.history_len < MAX_HISTORY_LINES {
            self.history[self.history_len] = line;
            self.history_len += 1;
            return;
        }

        for index in 1..MAX_HISTORY_LINES {
            self.history[index - 1] = self.history[index];
        }
        self.history[MAX_HISTORY_LINES - 1] = line;
    }
}

#[derive(Clone, Copy)]
struct HistoryLine {
    bytes: [u8; MAX_LINE_LEN],
    len: usize,
}

impl HistoryLine {
    const fn empty() -> Self {
        Self {
            bytes: [b' '; MAX_LINE_LEN],
            len: 0,
        }
    }

    fn push_str(&mut self, text: &str) {
        for byte in text.bytes() {
            self.push_byte(byte);
        }
    }

    fn push_byte(&mut self, byte: u8) {
        if self.len < MAX_LINE_LEN {
            self.bytes[self.len] = sanitize(byte);
            self.len += 1;
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    File,
    Dir,
}

#[derive(Clone, Copy)]
struct FsNode {
    used: bool,
    kind: NodeKind,
    parent: usize,
    name: [u8; MAX_NAME_LEN],
    name_len: usize,
    data: [u8; MAX_FILE_LEN],
    data_len: usize,
}

impl FsNode {
    const fn empty() -> Self {
        Self {
            used: false,
            kind: NodeKind::File,
            parent: 0,
            name: [0; MAX_NAME_LEN],
            name_len: 0,
            data: [0; MAX_FILE_LEN],
            data_len: 0,
        }
    }

    fn init_dir(&mut self, parent: usize, name: &str) {
        self.used = true;
        self.kind = NodeKind::Dir;
        self.parent = parent;
        self.set_name(name);
        self.data_len = 0;
    }

    fn init_file(&mut self, parent: usize, name: &str, contents: &str) {
        self.used = true;
        self.kind = NodeKind::File;
        self.parent = parent;
        self.set_name(name);
        self.set_data(contents);
    }

    fn set_name(&mut self, name: &str) {
        self.name = [0; MAX_NAME_LEN];
        self.name_len = 0;
        for byte in name.bytes().take(MAX_NAME_LEN) {
            self.name[self.name_len] = sanitize(byte);
            self.name_len += 1;
        }
    }

    fn set_data(&mut self, contents: &str) {
        self.data = [0; MAX_FILE_LEN];
        self.data_len = 0;
        for byte in contents.bytes().take(MAX_FILE_LEN) {
            self.data[self.data_len] = sanitize(byte);
            self.data_len += 1;
        }
    }

    fn name_eq(&self, name: &str) -> bool {
        self.name() == name
    }

    fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }

    fn data(&self) -> &str {
        core::str::from_utf8(&self.data[..self.data_len]).unwrap_or("")
    }
}

struct MiniFs {
    nodes: [FsNode; MAX_FS_NODES],
    cwd: usize,
    cwd_path: [u8; MAX_LINE_LEN],
    cwd_path_len: usize,
}

impl MiniFs {
    fn new() -> Self {
        let mut fs = Self {
            nodes: [FsNode::empty(); MAX_FS_NODES],
            cwd: 0,
            cwd_path: [b'/'; MAX_LINE_LEN],
            cwd_path_len: 1,
        };

        fs.nodes[0].init_dir(0, "");
        fs.nodes[1].init_dir(0, "docs");
        fs.nodes[2].init_file(0, "readme.txt", "Teddy Terminal demo filesystem.");
        fs.nodes[3].init_file(1, "plan.txt", "Next: persistent filesystem and real apps.");
        fs.nodes[4].init_file(0, "notes.txt", "Use F1 launcher, F2 focus, F3 move.");
        fs.refresh_cwd_path();
        fs
    }

    fn cwd_path(&self) -> &str {
        core::str::from_utf8(&self.cwd_path[..self.cwd_path_len]).unwrap_or("/")
    }

    fn change_dir(&mut self, path: &str) -> Result<(), &'static str> {
        let node = self.resolve_dir(path)?;
        self.cwd = node;
        self.refresh_cwd_path();
        Ok(())
    }

    fn read_file(&self, path: &str) -> Result<HistoryLine, &'static str> {
        let node = self.resolve_path(path)?;
        let entry = &self.nodes[node];
        if entry.kind != NodeKind::File {
            return Err("cat: not a file");
        }
        let mut line = HistoryLine::empty();
        line.push_str(entry.data());
        Ok(line)
    }

    fn create_dir(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent_and_name(path)?;
        self.create_node(parent, name, NodeKind::Dir)
    }

    fn touch(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent, name) = self.resolve_parent_and_name(path)?;
        if let Some(index) = self.find_child(parent, name) {
            if self.nodes[index].kind != NodeKind::File {
                return Err("touch: path is directory");
            }
            self.nodes[index].set_data("empty file");
            return Ok(());
        }

        self.create_node(parent, name, NodeKind::File)
    }

    fn remove(&mut self, path: &str) -> Result<(), &'static str> {
        let index = self.resolve_path(path)?;
        if index == 0 {
            return Err("rm: cannot remove root");
        }
        if self.nodes[index].kind == NodeKind::Dir && self.has_children(index) {
            return Err("rm: directory not empty");
        }
        if self.cwd == index {
            return Err("rm: cannot remove cwd");
        }
        self.nodes[index] = FsNode::empty();
        Ok(())
    }

    fn listing(&self, parent: usize) -> ([HistoryLine; MAX_FS_NODES], usize) {
        let mut lines = [HistoryLine::empty(); MAX_FS_NODES];
        let mut count = 0usize;
        let mut found = false;
        for index in 0..MAX_FS_NODES {
            let node = &self.nodes[index];
            if node.used && index != 0 && node.parent == parent {
                let mut line = HistoryLine::empty();
                if node.kind == NodeKind::Dir {
                    line.push_str("[dir] ");
                } else {
                    line.push_str("[file] ");
                }
                line.push_str(node.name());
                lines[count] = line;
                count += 1;
                found = true;
            }
        }

        if !found {
            lines[0].push_str("(empty)");
            count = 1;
        }
        (lines, count)
    }

    fn cwd_line(&self) -> HistoryLine {
        let mut line = HistoryLine::empty();
        line.push_str(self.cwd_path());
        line
    }

    fn resolve_dir(&self, path: &str) -> Result<usize, &'static str> {
        let node = self.resolve_path(path)?;
        if self.nodes[node].kind != NodeKind::Dir {
            return Err("cd: not a directory");
        }
        Ok(node)
    }

    fn resolve_path(&self, path: &str) -> Result<usize, &'static str> {
        if path.is_empty() {
            return Ok(self.cwd);
        }

        let mut current = if path.starts_with('/') { 0 } else { self.cwd };
        for component in path.split('/') {
            if component.is_empty() || component == "." {
                continue;
            }
            if component == ".." {
                current = self.nodes[current].parent;
                continue;
            }

            match self.find_child(current, component) {
                Some(index) => current = index,
                None => return Err("path not found"),
            }
        }
        Ok(current)
    }

    fn resolve_parent_and_name<'a>(&self, path: &'a str) -> Result<(usize, &'a str), &'static str> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err("missing path");
        }

        let mut parts = trimmed.rsplitn(2, '/');
        let name = parts.next().unwrap_or("");
        if name.is_empty() || name == "." || name == ".." {
            return Err("invalid name");
        }
        if !valid_name(name) {
            return Err("name too long or invalid");
        }

        let parent_path = parts.next().unwrap_or("");
        let parent = if trimmed.starts_with('/') && parent_path.is_empty() {
            0
        } else if parent_path.is_empty() {
            self.cwd
        } else {
            self.resolve_dir(parent_path)?
        };

        Ok((parent, name))
    }

    fn create_node(&mut self, parent: usize, name: &str, kind: NodeKind) -> Result<(), &'static str> {
        if self.find_child(parent, name).is_some() {
            return Err("path already exists");
        }

        for index in 1..MAX_FS_NODES {
            if !self.nodes[index].used {
                match kind {
                    NodeKind::Dir => self.nodes[index].init_dir(parent, name),
                    NodeKind::File => self.nodes[index].init_file(parent, name, "empty file"),
                }
                return Ok(());
            }
        }
        Err("filesystem full")
    }

    fn find_child(&self, parent: usize, name: &str) -> Option<usize> {
        for index in 1..MAX_FS_NODES {
            let node = &self.nodes[index];
            if node.used && node.parent == parent && node.name_eq(name) {
                return Some(index);
            }
        }
        None
    }

    fn has_children(&self, parent: usize) -> bool {
        for index in 1..MAX_FS_NODES {
            if self.nodes[index].used && self.nodes[index].parent == parent {
                return true;
            }
        }
        false
    }

    fn refresh_cwd_path(&mut self) {
        self.cwd_path = [b' '; MAX_LINE_LEN];
        if self.cwd == 0 {
            self.cwd_path[0] = b'/';
            self.cwd_path_len = 1;
            return;
        }

        let mut segments = [[0u8; MAX_NAME_LEN]; 8];
        let mut segment_lens = [0usize; 8];
        let mut segment_count = 0usize;
        let mut current = self.cwd;

        while current != 0 && segment_count < segments.len() {
            let node = &self.nodes[current];
            for (index, byte) in node.name[..node.name_len].iter().enumerate() {
                segments[segment_count][index] = *byte;
            }
            segment_lens[segment_count] = node.name_len;
            segment_count += 1;
            current = node.parent;
        }

        let mut len = 0usize;
        self.cwd_path[len] = b'/';
        len += 1;
        for segment_index in (0..segment_count).rev() {
            for byte in segments[segment_index].iter().take(segment_lens[segment_index]) {
                if len >= MAX_LINE_LEN {
                    break;
                }
                self.cwd_path[len] = *byte;
                len += 1;
            }
            if segment_index != 0 && len < MAX_LINE_LEN {
                self.cwd_path[len] = b'/';
                len += 1;
            }
        }
        self.cwd_path_len = len;
    }
}

fn valid_name(name: &str) -> bool {
    if name.len() > MAX_NAME_LEN {
        return false;
    }
    for byte in name.bytes() {
        if !matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-') {
            return false;
        }
    }
    true
}

fn sanitize(byte: u8) -> u8 {
    match byte {
        0x20..=0x7E => byte,
        _ => b'?',
    }
}
