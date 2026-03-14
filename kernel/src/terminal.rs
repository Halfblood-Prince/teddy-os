use spin::Mutex;

use crate::{
    framebuffer::{Color, FramebufferSurface, Rect},
    fs::{self, FsTextBuffer, MAX_OUTPUT_LINES},
    input::{KeyKind, KeyboardEvent},
    interrupts,
    network,
    storage,
    timer,
};

const SCROLLBACK_LINES: usize = 160;
const INPUT_CAPACITY: usize = 96;

pub struct TerminalApp {
    lines: [FsTextBuffer; SCROLLBACK_LINES],
    line_count: usize,
    next_index: usize,
    input: [u8; INPUT_CAPACITY],
    input_len: usize,
    history: [FsTextBuffer; 24],
    history_len: usize,
    history_index: usize,
    scratch: [FsTextBuffer; MAX_OUTPUT_LINES],
}

impl TerminalApp {
    fn new() -> Self {
        let mut terminal = Self {
            lines: [FsTextBuffer::new(); SCROLLBACK_LINES],
            line_count: 0,
            next_index: 0,
            input: [0; INPUT_CAPACITY],
            input_len: 0,
            history: [FsTextBuffer::new(); 24],
            history_len: 0,
            history_index: 0,
            scratch: [FsTextBuffer::new(); MAX_OUTPUT_LINES],
        };

        terminal.println("Teddy Terminal");
        if fs::is_ready() {
            terminal.println("Persistent TeddyFS volume mounted.");
        } else {
            terminal.println("No writable TeddyFS volume detected.");
        }
        terminal.println("Type 'help' for commands.");
        terminal.prompt();
        terminal
    }

    fn clear(&mut self) {
        self.lines = [FsTextBuffer::new(); SCROLLBACK_LINES];
        self.line_count = 0;
        self.next_index = 0;
    }

    fn println(&mut self, text: &str) {
        let mut line = FsTextBuffer::new();
        line.push_str(text);
        self.push_line(line);
    }

    fn push_line(&mut self, line: FsTextBuffer) {
        self.lines[self.next_index] = line;
        self.next_index = (self.next_index + 1) % SCROLLBACK_LINES;
        if self.line_count < SCROLLBACK_LINES {
            self.line_count += 1;
        }
    }

    fn prompt(&mut self) {
        let path = fs::pwd().unwrap_or_else(|_| {
            let mut unavailable = FsTextBuffer::new();
            unavailable.push_str("/unmounted");
            unavailable
        });
        let mut line = FsTextBuffer::new();
        line.push_str(path.as_str());
        line.push_str("> ");
        line.push_str(self.current_input());
        self.push_line(line);
    }

    fn replace_prompt(&mut self) {
        if self.line_count == 0 {
            self.prompt();
            return;
        }

        let index = (self.next_index + SCROLLBACK_LINES - 1) % SCROLLBACK_LINES;
        let path = fs::pwd().unwrap_or_else(|_| {
            let mut unavailable = FsTextBuffer::new();
            unavailable.push_str("/unmounted");
            unavailable
        });
        let mut line = FsTextBuffer::new();
        line.push_str(path.as_str());
        line.push_str("> ");
        line.push_str(self.current_input());
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
            KeyKind::Enter => self.execute_current_line(),
            KeyKind::ArrowUp => self.history_up(),
            KeyKind::ArrowDown => self.history_down(),
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
        self.input_len = entry.as_str().len().min(INPUT_CAPACITY);
        self.input[..self.input_len].copy_from_slice(&entry.as_str().as_bytes()[..self.input_len]);
        self.replace_prompt();
    }

    fn history_down(&mut self) {
        if self.history_len == 0 {
            return;
        }
        self.history_index = (self.history_index + 1) % self.history_len;
        let entry = self.history[self.history_index];
        self.input_len = entry.as_str().len().min(INPUT_CAPACITY);
        self.input[..self.input_len].copy_from_slice(&entry.as_str().as_bytes()[..self.input_len]);
        self.replace_prompt();
    }

    fn execute_current_line(&mut self) {
        let mut command_line_storage = FsTextBuffer::new();
        command_line_storage.push_str(self.current_input());
        let command_line = command_line_storage.as_str();

        let index = (self.next_index + SCROLLBACK_LINES - 1) % SCROLLBACK_LINES;
        let path = fs::pwd().unwrap_or_else(|_| {
            let mut unavailable = FsTextBuffer::new();
            unavailable.push_str("/unmounted");
            unavailable
        });
        let mut line = FsTextBuffer::new();
        line.push_str(path.as_str());
        line.push_str("> ");
        line.push_str(command_line);
        self.lines[index] = line;

        if !command_line.is_empty() {
            let mut entry = FsTextBuffer::new();
            entry.push_str(command_line);
            let history_slot = self.history_len.min(self.history.len() - 1);
            if self.history_len < self.history.len() {
                self.history_len += 1;
            }
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
            "help" => self.println("help echo clear ls cd pwd cat mkdir rm touch uname netinfo netdiag netsend arp dhcp dns fetch df diskinfo fsck reboot shutdown"),
            "echo" => self.echo_command(parser.rest()),
            "clear" => self.clear(),
            "ls" => match fs::ls(parser.next(), &mut self.scratch) {
                Ok(count) => {
                    let lines = self.scratch;
                    for index in 0..count {
                        self.println(lines[index].as_str());
                    }
                }
                Err(error) => self.println(error),
            },
            "cd" => {
                if let Some(path) = parser.next() {
                    if let Err(error) = fs::cd(path) {
                        self.println(error);
                    }
                } else {
                    self.println("cd: missing path");
                }
            }
            "pwd" => match fs::pwd() {
                Ok(path) => self.println(path.as_str()),
                Err(error) => self.println(error),
            },
            "cat" => {
                if let Some(path) = parser.next() {
                    match fs::cat(path, &mut self.scratch) {
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
                    if let Err(error) = fs::mkdir(path, timer::ticks()) {
                        self.println(error);
                    }
                } else {
                    self.println("mkdir: missing path");
                }
            }
            "rm" => {
                if let Some(path) = parser.next() {
                    if let Err(error) = fs::rm(path) {
                        self.println(error);
                    }
                } else {
                    self.println("rm: missing path");
                }
            }
            "touch" => {
                if let Some(path) = parser.next() {
                    if let Err(error) = fs::touch(path, timer::ticks()) {
                        self.println(error);
                    }
                } else {
                    self.println("touch: missing path");
                }
            }
            "netinfo" => self.netinfo_command(),
            "netdiag" => self.netdiag_command(),
            "netsend" => self.netsend_command(),
            "arp" => self.arp_command(parser.next()),
            "dhcp" => self.dhcp_command(),
            "dns" => self.dns_command(parser.next()),
            "fetch" => self.fetch_command(parser.next()),
            "df" => self.disk_free_command(),
            "diskinfo" => self.disk_info_command(),
            "fsck" => self.fsck_command(),
            "uname" => self.println("Teddy-OS x86_64 phase10"),
            "reboot" => {
                self.println("Rebooting Teddy-OS...");
                reboot_system();
            }
            "shutdown" => {
                self.println("Shutting down Teddy-OS...");
                shutdown_system();
            }
            _ => self.println("unknown command"),
        }
    }

    fn echo_command(&mut self, rest: &str) {
        if let Some((text, path)) = rest.split_once('>') {
            let destination = path.trim();
            if destination.is_empty() {
                self.println("echo: missing redirection target");
                return;
            }
            match fs::write_text(destination, text.trim(), timer::ticks()) {
                Ok(()) => self.println("write ok"),
                Err(error) => self.println(error),
            }
        } else {
            self.println(rest);
        }
    }

    fn disk_free_command(&mut self) {
        match fs::stats() {
            Ok(stats) => {
                let mut line = FsTextBuffer::new();
                line.push_str("teddyfs ");
                push_usize(&mut line, stats.bytes_used);
                line.push_str("/");
                push_usize(&mut line, stats.capacity_bytes);
                line.push_str(" bytes used");
                self.push_line(line);

                let mut entries = FsTextBuffer::new();
                entries.push_str("entries ");
                push_usize(&mut entries, stats.used_entries);
                entries.push_str("/");
                push_usize(&mut entries, stats.total_entries);
                entries.push_str(" files ");
                push_usize(&mut entries, stats.file_count);
                entries.push_str(" dirs ");
                push_usize(&mut entries, stats.directory_count);
                self.push_line(entries);
            }
            Err(error) => self.println(error),
        }
    }

    fn disk_info_command(&mut self) {
        let storage_stats = storage::stats();
        if !storage_stats.present {
            self.println("disk: no ATA device detected");
            return;
        }

        let mut line = FsTextBuffer::new();
        line.push_str("drive ");
        line.push_str(match storage_stats.drive {
            storage::DriveSelect::Master => "master",
            storage::DriveSelect::Slave => "slave",
        });
        line.push_str(" model ");
        line.push_str(storage_stats.model.as_str());
        self.push_line(line);

        let mut capacity = FsTextBuffer::new();
        capacity.push_str("sectors ");
        push_u32(&mut capacity, storage_stats.total_sectors);
        capacity.push_str(" sector_size ");
        push_usize(&mut capacity, storage_stats.sector_size);
        capacity.push_str(" bytes capacity ");
        push_u64(&mut capacity, storage_stats.capacity_bytes);
        self.push_line(capacity);
    }

    fn fsck_command(&mut self) {
        match fs::check() {
            Ok(report) => {
                let mut line = FsTextBuffer::new();
                line.push_str("fsck ");
                line.push_str(if report.ok { "ok" } else { "failed" });
                line.push_str(" checked ");
                push_usize(&mut line, report.checked_entries);
                line.push_str(" errors ");
                push_usize(&mut line, report.errors_found);
                self.push_line(line);
            }
            Err(error) => self.println(error),
        }
    }

    fn netinfo_command(&mut self) {
        let info = network::info();
        if !info.detected {
            self.println("network: no supported VMware NIC detected");
            return;
        }

        let mut line = FsTextBuffer::new();
        line.push_str("nic ");
        line.push_str(info.name.as_str());
        line.push_str(" bus ");
        push_usize(&mut line, info.bus as usize);
        line.push_str(" slot ");
        push_usize(&mut line, info.slot as usize);
        line.push_str(" fn ");
        push_usize(&mut line, info.function as usize);
        self.push_line(line);

        let mut bars = FsTextBuffer::new();
        bars.push_str("prepared ");
        bars.push_str(if info.prepared { "yes" } else { "no" });
        bars.push_str(" driver ");
        bars.push_str(if info.driver_ready { "yes" } else { "no" });
        bars.push_str(" io ");
        push_u32(&mut bars, info.io_base);
        bars.push_str(" mmio ");
        push_u32(&mut bars, info.mmio_base);
        self.push_line(bars);

        let mut mac = FsTextBuffer::new();
        mac.push_str("mac ");
        push_mac(&mut mac, info.mac.bytes());
        self.push_line(mac);

        let mut state = FsTextBuffer::new();
        state.push_str("state ");
        state.push_str(info.driver_state.as_str());
        self.push_line(state);
    }

    fn netdiag_command(&mut self) {
        let info = network::info();
        if !info.detected {
            self.println("netdiag: no supported NIC");
            return;
        }

        let mut line = FsTextBuffer::new();
        line.push_str("irq ");
        push_usize(&mut line, info.irq_line as usize);
        line.push_str(" cmd ");
        push_hex_u8(&mut line, info.command_register);
        line.push_str(" isr ");
        push_hex_u16(&mut line, info.interrupt_status);
        self.push_line(line);

        let mut cfg = FsTextBuffer::new();
        cfg.push_str("rcr ");
        push_hex_u32(&mut cfg, info.rx_config);
        cfg.push_str(" tcr ");
        push_hex_u32(&mut cfg, info.tx_config);
        self.push_line(cfg);

        let mut dma = FsTextBuffer::new();
        dma.push_str("rxbuf ");
        push_hex_u32(&mut dma, info.rx_buffer_addr);
        dma.push_str(" cbr ");
        push_hex_u16(&mut dma, info.current_rx_read);
        dma.push_str(" rxok ");
        push_u64(&mut dma, info.rx_packets);
        self.push_line(dma);

        let mut tx = FsTextBuffer::new();
        tx.push_str("txok ");
        push_u64(&mut tx, info.tx_completions);
        tx.push_str(" txtry ");
        push_u64(&mut tx, info.tx_attempts);
        tx.push_str(" txlen ");
        push_usize(&mut tx, info.last_tx_length as usize);
        tx.push_str(" tsad0 ");
        push_hex_u32(&mut tx, info.tx_buffer_addr[0]);
        self.push_line(tx);

        let mut rx = FsTextBuffer::new();
        rx.push_str("rxlen ");
        push_usize(&mut rx, info.last_rx_length as usize);
        rx.push_str(" type ");
        push_hex_u16(&mut rx, info.last_rx_ethertype);
        self.push_line(rx);

        let mut macs = FsTextBuffer::new();
        macs.push_str("src ");
        push_mac(&mut macs, info.last_rx_source.bytes());
        macs.push_str(" dst ");
        push_mac(&mut macs, info.last_rx_destination.bytes());
        self.push_line(macs);

        let mut arp = FsTextBuffer::new();
        arp.push_str("arp ");
        push_u64(&mut arp, info.arp_packets);
        arp.push_str(" op ");
        push_hex_u16(&mut arp, info.last_arp_opcode);
        self.push_line(arp);

        let mut arp_addr = FsTextBuffer::new();
        arp_addr.push_str("arp src ");
        push_ipv4(&mut arp_addr, info.last_arp_sender_ip.octets());
        arp_addr.push_str(" tgt ");
        push_ipv4(&mut arp_addr, info.last_arp_target_ip.octets());
        self.push_line(arp_addr);

        let mut ipv4 = FsTextBuffer::new();
        ipv4.push_str("ip ");
        push_u64(&mut ipv4, info.ipv4_packets);
        ipv4.push_str(" proto ");
        push_hex_u8(&mut ipv4, info.last_ipv4_protocol);
        self.push_line(ipv4);

        let mut ipv4_addr = FsTextBuffer::new();
        ipv4_addr.push_str("ip src ");
        push_ipv4(&mut ipv4_addr, info.last_ipv4_source.octets());
        ipv4_addr.push_str(" dst ");
        push_ipv4(&mut ipv4_addr, info.last_ipv4_destination.octets());
        self.push_line(ipv4_addr);

        let mut udp = FsTextBuffer::new();
        udp.push_str("udp ");
        push_u64(&mut udp, info.udp_packets);
        udp.push_str(" sport ");
        push_usize(&mut udp, info.last_udp_source_port as usize);
        udp.push_str(" dport ");
        push_usize(&mut udp, info.last_udp_destination_port as usize);
        udp.push_str(" len ");
        push_usize(&mut udp, info.last_udp_length as usize);
        self.push_line(udp);

        let mut dhcp = FsTextBuffer::new();
        dhcp.push_str("dhcp rx ");
        push_u64(&mut dhcp, info.dhcp_packets);
        dhcp.push_str(" tx ");
        push_u64(&mut dhcp, info.dhcp_discover_attempts);
        dhcp.push_str(" type ");
        push_usize(&mut dhcp, info.last_dhcp_message_type as usize);
        self.push_line(dhcp);
    }

    fn netsend_command(&mut self) {
        match network::send_test_frame() {
            Ok(()) => self.println("netsend: broadcast test frame queued"),
            Err(error) => self.println(error),
        }
    }

    fn arp_command(&mut self, ip_text: Option<&str>) {
        let Some(ip_text) = ip_text else {
            self.println("arp: missing IPv4 address");
            return;
        };
        let Some(ip) = parse_ipv4(ip_text) else {
            self.println("arp: invalid IPv4 address");
            return;
        };
        match network::send_arp_request(network::Ipv4Address::from_octets(ip)) {
            Ok(()) => self.println("arp: request queued"),
            Err(error) => self.println(error),
        }
    }

    fn dhcp_command(&mut self) {
        match network::send_dhcp_discover() {
            Ok(()) => self.println("dhcp: discover queued"),
            Err(error) => self.println(error),
        }
    }

    fn dns_command(&mut self, host: Option<&str>) {
        let Some(host) = host else {
            self.println("dns: missing host");
            return;
        };
        let info = network::info();
        if !info.detected {
            self.println("dns: no NIC");
            return;
        }
        let mut line = FsTextBuffer::new();
        line.push_str("dns: resolver scaffolding present for ");
        line.push_str(host);
        self.push_line(line);
    }

    fn fetch_command(&mut self, url: Option<&str>) {
        let Some(url) = url else {
            self.println("fetch: missing url");
            return;
        };
        let info = network::info();
        if !info.detected {
            self.println("fetch: no NIC");
            return;
        }
        let mut line = FsTextBuffer::new();
        line.push_str("fetch: transport scaffolding present for ");
        line.push_str(url);
        self.push_line(line);
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

        let visible_lines = (rect.height / 16).saturating_sub(1).min(MAX_OUTPUT_LINES);
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

    fn scrollback_line(&self, logical_index: usize) -> Option<&FsTextBuffer> {
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

fn push_usize(buffer: &mut FsTextBuffer, value: usize) {
    let mut digits = [0u8; 20];
    let mut count = 0usize;
    let mut current = value;
    if current == 0 {
        buffer.push_str("0");
        return;
    }
    while current > 0 && count < digits.len() {
        digits[count] = b'0' + (current % 10) as u8;
        current /= 10;
        count += 1;
    }
    for index in (0..count).rev() {
        let text = [digits[index]];
        let digit = core::str::from_utf8(&text).unwrap_or("?");
        buffer.push_str(digit);
    }
}

fn push_u32(buffer: &mut FsTextBuffer, value: u32) {
    push_usize(buffer, value as usize);
}

fn push_u64(buffer: &mut FsTextBuffer, value: u64) {
    let mut digits = [0u8; 20];
    let mut count = 0usize;
    let mut current = value;
    if current == 0 {
        buffer.push_str("0");
        return;
    }
    while current > 0 && count < digits.len() {
        digits[count] = b'0' + (current % 10) as u8;
        current /= 10;
        count += 1;
    }
    for index in (0..count).rev() {
        let text = [digits[index]];
        let digit = core::str::from_utf8(&text).unwrap_or("?");
        buffer.push_str(digit);
    }
}

fn push_mac(buffer: &mut FsTextBuffer, mac: [u8; 6]) {
    for (index, byte) in mac.iter().enumerate() {
        push_hex_byte(buffer, *byte);
        if index + 1 != mac.len() {
            buffer.push_str(":");
        }
    }
}

fn push_hex_u8(buffer: &mut FsTextBuffer, value: u8) {
    buffer.push_str("0x");
    push_hex_byte(buffer, value);
}

fn push_hex_u16(buffer: &mut FsTextBuffer, value: u16) {
    buffer.push_str("0x");
    push_hex_byte(buffer, (value >> 8) as u8);
    push_hex_byte(buffer, (value & 0x00FF) as u8);
}

fn push_hex_u32(buffer: &mut FsTextBuffer, value: u32) {
    buffer.push_str("0x");
    push_hex_byte(buffer, (value >> 24) as u8);
    push_hex_byte(buffer, ((value >> 16) & 0xFF) as u8);
    push_hex_byte(buffer, ((value >> 8) & 0xFF) as u8);
    push_hex_byte(buffer, (value & 0xFF) as u8);
}

fn push_hex_byte(buffer: &mut FsTextBuffer, value: u8) {
    let high = nibble_to_hex((value >> 4) & 0x0F);
    let low = nibble_to_hex(value & 0x0F);
    let bytes = [high, low];
    let text = core::str::from_utf8(&bytes).unwrap_or("00");
    buffer.push_str(text);
}

fn nibble_to_hex(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    }
}

fn push_ipv4(buffer: &mut FsTextBuffer, octets: [u8; 4]) {
    for (index, octet) in octets.iter().enumerate() {
        push_usize(buffer, *octet as usize);
        if index + 1 != octets.len() {
            buffer.push_str(".");
        }
    }
}

fn parse_ipv4(text: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut count = 0usize;
    for part in text.split('.') {
        if count >= 4 || part.is_empty() {
            return None;
        }
        let value = parse_decimal(part)?;
        if value > 255 {
            return None;
        }
        octets[count] = value as u8;
        count += 1;
    }
    if count == 4 {
        Some(octets)
    } else {
        None
    }
}

fn parse_decimal(text: &str) -> Option<usize> {
    let mut value = 0usize;
    for byte in text.as_bytes() {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((byte - b'0') as usize)?;
    }
    Some(value)
}
