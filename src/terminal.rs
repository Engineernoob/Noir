use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver},
    thread,
};

use anyhow::Result;
use portable_pty::{CommandBuilder, PtyPair, PtySize, native_pty_system};

pub struct TerminalPane {
    pub visible: bool,
    pub lines: Vec<String>,
    pub scroll: usize,
    scrollback_limit: usize,
    shell_input: Option<Box<dyn Write + Send>>,
    output_rx: Option<Receiver<String>>,
    _pty_pair: Option<PtyPair>,
}

impl TerminalPane {
    pub fn new(scrollback_limit: usize) -> Self {
        Self {
            visible: true,
            lines: vec!["Starting Noir PTY shell...".to_string()],
            scroll: 0,
            scrollback_limit: scrollback_limit.max(1),
            shell_input: None,
            output_rx: None,
            _pty_pair: None,
        }
    }

    pub fn init_shell(&mut self, shell: Option<&str>) -> Result<()> {
        let pty_system = native_pty_system();
        let pty_pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = shell
            .map(ToOwned::to_owned)
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/zsh".to_string());
        let cmd = CommandBuilder::new(shell);
        let _child = pty_pair.slave.spawn_command(cmd)?;

        let mut reader = pty_pair.master.try_clone_reader()?;
        let writer = pty_pair.master.take_writer()?;

        let (tx, rx) = mpsc::channel::<String>();

        thread::spawn(move || {
            let mut buf = [0u8; 4096];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        if tx.send(text).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.shell_input = Some(writer);
        self.output_rx = Some(rx);
        self._pty_pair = Some(pty_pair);
        self.lines
            .push("PTY shell started successfully.".to_string());

        Ok(())
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn poll_output(&mut self) {
        let mut pending = Vec::new();

        if let Some(rx) = &self.output_rx {
            while let Ok(chunk) = rx.try_recv() {
                pending.push(chunk);
            }
        }

        for chunk in pending {
            self.push_output_chunk(&chunk);
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        if let Some(pair) = &mut self._pty_pair {
            let _ = pair.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn send_input(&mut self, input: &str) {
        if let Some(writer) = &mut self.shell_input {
            let _ = writer.write_all(input.as_bytes());
            let _ = writer.flush();
        }
    }

    pub fn send_key_char(&mut self, c: char) {
        let mut s = String::new();
        s.push(c);
        self.send_input(&s);
    }

    pub fn send_enter(&mut self) {
        self.send_input("\r");
    }

    pub fn send_backspace(&mut self) {
        self.send_input("\u{7f}");
    }

    pub fn push_system_message(&mut self, message: &str) {
        self.push_output_chunk(message);
    }

    pub fn visible_lines(&self, height: usize) -> Vec<String> {
        let total = self.lines.len();
        if total == 0 {
            return vec![];
        }

        let start = total.saturating_sub(height + self.scroll);
        let end = total.saturating_sub(self.scroll).min(total);
        self.lines[start..end].to_vec()
    }

    pub fn scroll_up(&mut self) {
        if self.scroll + 1 < self.lines.len() {
            self.scroll += 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    fn push_output_chunk(&mut self, chunk: &str) {
        let normalized = chunk.replace("\r\n", "\n").replace('\r', "\n");

        for part in normalized.split('\n') {
            if part.is_empty() {
                self.lines.push(String::new());
            } else {
                self.lines.push(strip_ansi_basic(part));
            }
        }

        if self.lines.len() > self.scrollback_limit {
            let drain_count = self.lines.len() - self.scrollback_limit;
            self.lines.drain(0..drain_count);
        }

        self.scroll = 0;
    }
}

fn strip_ansi_basic(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                while let Some(next) = chars.next() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
        } else {
            out.push(ch);
        }
    }

    out
}
