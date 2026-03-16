use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ropey::Rope;

pub struct Buffer {
    pub file_path: Option<PathBuf>,
    pub rope: Rope,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_y: usize,
    pub dirty: bool,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            file_path: None,
            rope: Rope::new(),
            cursor_row: 0,
            cursor_col: 0,
            scroll_y: 0,
            dirty: false,
        }
    }
}

pub struct Editor {
    pub buffers: Vec<Buffer>,
    pub active: usize,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            buffers: vec![Buffer::default()],
            active: 0,
        }
    }
}

impl Editor {
    pub fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        if let Some(idx) = self
            .buffers
            .iter()
            .position(|b| b.file_path.as_ref() == Some(&path))
        {
            self.active = idx;
            return Ok(());
        }

        let content = fs::read_to_string(&path).unwrap_or_default();
        let buffer = Buffer {
            file_path: Some(path),
            rope: Rope::from_str(&content),
            cursor_row: 0,
            cursor_col: 0,
            scroll_y: 0,
            dirty: false,
        };

        self.buffers.push(buffer);
        self.active = self.buffers.len() - 1;
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        let buf = self.current_buffer_mut();
        if let Some(path) = &buf.file_path {
            fs::write(path, buf.rope.to_string())?;
            buf.dirty = false;
        }
        Ok(())
    }

    pub fn next_tab(&mut self) {
        if !self.buffers.is_empty() {
            self.active = (self.active + 1) % self.buffers.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.buffers.is_empty() {
            self.active = if self.active == 0 {
                self.buffers.len() - 1
            } else {
                self.active - 1
            };
        }
    }

    pub fn current_buffer(&self) -> &Buffer {
        &self.buffers[self.active]
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.active]
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Enter => self.insert_char('\n'),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.insert_char(c);
            }
            _ => {}
        }
    }

    pub fn title(&self) -> String {
        let buf = self.current_buffer();
        match &buf.file_path {
            Some(path) => {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("[unnamed]");
                if buf.dirty {
                    format!("{} ●", name)
                } else {
                    name.to_string()
                }
            }
            None => "[No file]".to_string(),
        }
    }

    pub fn tab_titles(&self) -> Vec<String> {
        self.buffers
            .iter()
            .map(|buf| match &buf.file_path {
                Some(path) => {
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("[unnamed]");
                    if buf.dirty {
                        format!("{} ●", name)
                    } else {
                        name.to_string()
                    }
                }
                None => "[No file]".to_string(),
            })
            .collect()
    }

    pub fn lines_for_render(&self, height: usize) -> Vec<String> {
        let buf = self.current_buffer();
        let total_lines = buf.rope.len_lines();
        let start = buf.scroll_y.min(total_lines.saturating_sub(1));
        let end = (start + height).min(total_lines);

        (start..end)
            .map(|i| {
                buf.rope
                    .line(i)
                    .to_string()
                    .trim_end_matches('\n')
                    .to_string()
            })
            .collect()
    }

    fn line_len_chars(&self, row: usize) -> usize {
        let buf = self.current_buffer();
        if row >= buf.rope.len_lines() {
            return 0;
        }
        buf.rope
            .line(row)
            .to_string()
            .trim_end_matches('\n')
            .chars()
            .count()
    }

    fn char_index(&self, row: usize, col: usize) -> usize {
        let buf = self.current_buffer();
        let safe_row = row.min(buf.rope.len_lines().saturating_sub(1));
        let line_start = buf.rope.line_to_char(safe_row);
        line_start + col.min(self.line_len_chars(row))
    }

    fn insert_char(&mut self, ch: char) {
        let idx = {
            let buf = self.current_buffer();
            if buf.rope.len_lines() == 0 {
                0
            } else {
                self.char_index(buf.cursor_row, buf.cursor_col)
            }
        };

        let buf = self.current_buffer_mut();
        buf.rope.insert_char(idx, ch);
        buf.dirty = true;

        if ch == '\n' {
            buf.cursor_row += 1;
            buf.cursor_col = 0;
        } else {
            buf.cursor_col += 1;
        }
    }

    fn backspace(&mut self) {
        {
            let buf = self.current_buffer();
            if buf.rope.len_chars() == 0 {
                return;
            }
            if buf.cursor_row == 0 && buf.cursor_col == 0 {
                return;
            }
        }

        let idx = {
            let buf = self.current_buffer();
            self.char_index(buf.cursor_row, buf.cursor_col)
        };

        if idx == 0 {
            return;
        }

        {
            let buf = self.current_buffer_mut();
            buf.rope.remove((idx - 1)..idx);
            buf.dirty = true;
        }

        let cursor_col = self.current_buffer().cursor_col;
        if cursor_col > 0 {
            self.current_buffer_mut().cursor_col -= 1;
        } else {
            let prev_row = self.current_buffer().cursor_row.saturating_sub(1);
            let prev_len = self.line_len_chars(prev_row);
            let buf = self.current_buffer_mut();
            buf.cursor_row = prev_row;
            buf.cursor_col = prev_len;
        }
    }

    fn move_up(&mut self) {
        let row = self.current_buffer().cursor_row;
        if row > 0 {
            let new_row = row - 1;
            let new_col = self
                .current_buffer()
                .cursor_col
                .min(self.line_len_chars(new_row));
            let buf = self.current_buffer_mut();
            buf.cursor_row = new_row;
            buf.cursor_col = new_col;
            if buf.cursor_row < buf.scroll_y {
                buf.scroll_y = buf.cursor_row;
            }
        }
    }

    fn move_down(&mut self) {
        let max_row = self.current_buffer().rope.len_lines().saturating_sub(1);
        let row = self.current_buffer().cursor_row;
        if row < max_row {
            let new_row = row + 1;
            let new_col = self
                .current_buffer()
                .cursor_col
                .min(self.line_len_chars(new_row));
            let buf = self.current_buffer_mut();
            buf.cursor_row = new_row;
            buf.cursor_col = new_col;
        }
    }

    fn move_left(&mut self) {
        let row = self.current_buffer().cursor_row;
        let col = self.current_buffer().cursor_col;

        if col > 0 {
            self.current_buffer_mut().cursor_col -= 1;
        } else if row > 0 {
            let prev_row = row - 1;
            let prev_len = self.line_len_chars(prev_row);
            let buf = self.current_buffer_mut();
            buf.cursor_row = prev_row;
            buf.cursor_col = prev_len;
        }
    }

    fn move_right(&mut self) {
        let row = self.current_buffer().cursor_row;
        let col = self.current_buffer().cursor_col;
        let len = self.line_len_chars(row);
        let total_lines = self.current_buffer().rope.len_lines();

        if col < len {
            self.current_buffer_mut().cursor_col += 1;
        } else if row + 1 < total_lines {
            let buf = self.current_buffer_mut();
            buf.cursor_row += 1;
            buf.cursor_col = 0;
        }
    }
}
