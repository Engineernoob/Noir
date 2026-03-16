use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ropey::Rope;

#[derive(Default)]
pub struct Editor {
    pub file_path: Option<PathBuf>,
    pub buffer: Rope,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_y: usize,
    pub dirty: bool,
}

impl Editor {
    pub fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        let content = fs::read_to_string(&path).unwrap_or_default();
        self.buffer = Rope::from_str(&content);
        self.file_path = Some(path);
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_y = 0;
        self.dirty = false;
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(path) = &self.file_path {
            fs::write(path, self.buffer.to_string())?;
            self.dirty = false;
        }
        Ok(())
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
        match &self.file_path {
            Some(path) => {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("[unnamed]");
                if self.dirty {
                    format!("{} ●", name)
                } else {
                    name.to_string()
                }
            }
            None => "[No file]".to_string(),
        }
    }

    pub fn lines_for_render(&self, height: usize) -> Vec<String> {
        let total_lines = self.buffer.len_lines();
        let start = self.scroll_y.min(total_lines.saturating_sub(1));
        let end = (start + height).min(total_lines);

        (start..end)
            .map(|i| {
                self.buffer
                    .line(i)
                    .to_string()
                    .trim_end_matches('\n')
                    .to_string()
            })
            .collect()
    }

    fn line_len_chars(&self, row: usize) -> usize {
        if row >= self.buffer.len_lines() {
            return 0;
        }
        self.buffer
            .line(row)
            .to_string()
            .trim_end_matches('\n')
            .chars()
            .count()
    }

    fn char_index(&self, row: usize, col: usize) -> usize {
        let line_start = self
            .buffer
            .line_to_char(row.min(self.buffer.len_lines().saturating_sub(1)));
        line_start + col.min(self.line_len_chars(row))
    }

    fn insert_char(&mut self, ch: char) {
        if self.buffer.len_chars() == 0 && self.buffer.len_lines() == 0 {
            self.buffer = Rope::from_str("");
        }

        let idx = if self.buffer.len_lines() == 0 {
            0
        } else {
            self.char_index(self.cursor_row, self.cursor_col)
        };

        self.buffer.insert_char(idx, ch);
        self.dirty = true;

        if ch == '\n' {
            self.cursor_row += 1;
            self.cursor_col = 0;
        } else {
            self.cursor_col += 1;
        }
    }

    fn backspace(&mut self) {
        if self.buffer.len_chars() == 0 {
            return;
        }

        if self.cursor_row == 0 && self.cursor_col == 0 {
            return;
        }

        let idx = self.char_index(self.cursor_row, self.cursor_col);
        if idx == 0 {
            return;
        }

        self.buffer.remove((idx - 1)..idx);
        self.dirty = true;

        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.line_len_chars(self.cursor_row);
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.line_len_chars(self.cursor_row));
            if self.cursor_row < self.scroll_y {
                self.scroll_y = self.cursor_row;
            }
        }
    }

    fn move_down(&mut self) {
        let max_row = self.buffer.len_lines().saturating_sub(1);
        if self.cursor_row < max_row {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.line_len_chars(self.cursor_row));
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.line_len_chars(self.cursor_row);
        }
    }

    fn move_right(&mut self) {
        let len = self.line_len_chars(self.cursor_row);
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.buffer.len_lines() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }
}
