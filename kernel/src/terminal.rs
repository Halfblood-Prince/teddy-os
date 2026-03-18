use crate::{
    fs::{EntryKind, FileSystem, NameText, MAX_FILE_LEN, MAX_FS_NODES, MAX_PATH_LEN},
    trace,
};

const MAX_HISTORY_LINES: usize = 18;
const MAX_LINE_LEN: usize = 58;
const INPUT_BUFFER_LEN: usize = 58;

pub enum TerminalAction {
    None,
    Reboot,
    Shutdown,
}

pub struct TerminalApp {
    history: [HistoryLine; MAX_HISTORY_LINES],
    history_len: usize,
    input: [u8; INPUT_BUFFER_LEN],
    input_len: usize,
}

impl TerminalApp {
    pub const fn empty() -> Self {
        Self {
            history: [HistoryLine::empty(); MAX_HISTORY_LINES],
            history_len: 0,
            input: [0; INPUT_BUFFER_LEN],
            input_len: 0,
        }
    }

    pub fn init(&mut self) {
        trace::set_boot_stage(0x40);
        clear_history_lines(&mut self.history);
        self.history_len = 0;
        self.input = [0; INPUT_BUFFER_LEN];
        self.input_len = 0;
        trace::set_boot_stage(0x42);
        self.push_line("Teddy Terminal ready.");
        trace::set_boot_stage(0x43);
        self.push_line("Type 'help' for commands.");
        trace::set_boot_stage(0x44);
    }

    pub fn handle_key(&mut self, ascii: u8, fs: &mut FileSystem) -> TerminalAction {
        match ascii {
            8 => {
                if self.input_len > 0 {
                    self.input_len -= 1;
                }
                TerminalAction::None
            }
            b'\n' => self.submit_command(fs),
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

    pub fn cwd<'a>(&self, fs: &'a FileSystem) -> &'a str {
        fs.cwd_path()
    }

    fn submit_command(&mut self, fs: &mut FileSystem) -> TerminalAction {
        let command_buffer = self.input;
        let command_len = self.input_len;
        let command = core::str::from_utf8(&command_buffer[..command_len]).unwrap_or("");

        self.push_prompt_line(command, fs);

        let action = if command.is_empty() {
            TerminalAction::None
        } else {
            self.execute(command, fs)
        };

        self.input_len = 0;
        action
    }

    fn execute(&mut self, command: &str, fs: &mut FileSystem) -> TerminalAction {
        if command == "help" {
            self.push_line("help echo clear ls cd pwd cat mkdir rm");
            self.push_line("touch uname reboot shutdown");
            return TerminalAction::None;
        }
        if command == "clear" {
            self.clear_history();
            return TerminalAction::None;
        }
        if starts_with(command, "echo ") {
            self.push_line(slice_from(command, 5));
            return TerminalAction::None;
        }
        if command == "echo" {
            self.push_line("");
            return TerminalAction::None;
        }
        if command == "ls" {
            self.push_listing(fs);
            return TerminalAction::None;
        }
        if starts_with(command, "cd ") {
            let path = slice_from(command, 3);
            match fs.change_dir(path) {
                Ok(()) => {}
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if command == "pwd" {
            let cwd = fs.cwd_text();
            self.push_line(cwd.as_str());
            return TerminalAction::None;
        }
        if starts_with(command, "cat ") {
            let path = slice_from(command, 4);
            match self.push_file(path, fs) {
                Ok(()) => {}
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if starts_with(command, "mkdir ") {
            let path = slice_from(command, 6);
            match fs.create_dir(path) {
                Ok(()) => self.push_line("directory created"),
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if starts_with(command, "rm ") {
            let path = slice_from(command, 3);
            match fs.remove(path) {
                Ok(()) => self.push_line("removed"),
                Err(message) => self.push_line(message),
            }
            return TerminalAction::None;
        }
        if starts_with(command, "touch ") {
            let path = slice_from(command, 6);
            match fs.touch(path) {
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
        clear_history_lines(&mut self.history);
        self.history_len = 0;
    }

    fn push_prompt_line(&mut self, command: &str, fs: &FileSystem) {
        let cwd = fs.cwd_text();
        let mut line = HistoryLine::empty();
        line.push_str(cwd.as_str());
        line.push_str(" $ ");
        line.push_str(command);
        self.push_history(line);
    }

    fn push_line(&mut self, text: &str) {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            self.push_history(HistoryLine::empty());
            return;
        }

        let mut start = 0usize;
        while start < bytes.len() {
            let mut line = HistoryLine::empty();
            let take = core::cmp::min(bytes.len() - start, MAX_LINE_LEN);
            let mut index = 0usize;
            while index < take {
                line.push_byte(bytes[start + index]);
                index += 1;
            }
            self.push_history(line);
            start += take;
        }
    }

    fn push_file(&mut self, path: &str, fs: &FileSystem) -> Result<(), &'static str> {
        let mut buffer = [0u8; MAX_FILE_LEN];
        let len = fs.read_file_into(path, &mut buffer)?;
        let text = core::str::from_utf8(&buffer[..len]).unwrap_or("");
        self.push_line(text);
        Ok(())
    }

    fn push_listing(&mut self, fs: &FileSystem) {
        let mut kinds = [EntryKind::File; MAX_FS_NODES];
        let mut names = [NameText::empty(); MAX_FS_NODES];
        let mut sizes = [0usize; MAX_FS_NODES];
        let len = fs.list_current_dir_into(&mut kinds, &mut names, &mut sizes);
        if len == 0 {
            self.push_line("(empty)");
            return;
        }

        let mut index = 0usize;
        while index < len {
            let mut line = HistoryLine::empty();
            match kinds[index] {
                EntryKind::Dir => line.push_str("[dir] "),
                EntryKind::File => line.push_str("[file] "),
            }
            line.push_str(names[index].as_str());
            if kinds[index] == EntryKind::File {
                line.push_str(" ");
                append_decimal(&mut line, sizes[index]);
                line.push_str("b");
            }
            self.push_history(line);
            index += 1;
        }
    }

    fn push_history(&mut self, line: HistoryLine) {
        if self.history_len < MAX_HISTORY_LINES {
            self.history[self.history_len] = line;
            self.history_len += 1;
            return;
        }

        let mut index = 1usize;
        while index < MAX_HISTORY_LINES {
            self.history[index - 1] = self.history[index];
            index += 1;
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
        let bytes = text.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() {
            self.push_byte(bytes[index]);
            index += 1;
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

fn starts_with(text: &str, prefix: &str) -> bool {
    text.as_bytes().starts_with(prefix.as_bytes())
}

fn slice_from(text: &str, start: usize) -> &str {
    if start >= text.len() {
        ""
    } else {
        &text[start..]
    }
}

fn sanitize(byte: u8) -> u8 {
    match byte {
        0x20..=0x7E => byte,
        _ => b'?',
    }
}

fn clear_history_lines(lines: &mut [HistoryLine; MAX_HISTORY_LINES]) {
    let mut index = 0usize;
    while index < MAX_HISTORY_LINES {
        lines[index] = HistoryLine::empty();
        index += 1;
    }
}

fn append_decimal(line: &mut HistoryLine, mut value: usize) {
    if value == 0 {
        line.push_byte(b'0');
        return;
    }

    let mut scratch = [0u8; MAX_PATH_LEN];
    let mut len = 0usize;
    while value > 0 && len < scratch.len() {
        scratch[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    while len > 0 {
        len -= 1;
        line.push_byte(scratch[len]);
    }
}
