use std::{fs, path::PathBuf};

const MAX_RESULTS: usize = 200;

#[derive(Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    /// 0-based line index (matches editor cursor_row).
    pub line: u32,
    pub snippet: String,
}

#[derive(Default)]
pub struct SearchPanel {
    pub open: bool,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected: usize,
}

impl SearchPanel {
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.results.clear();
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected)
    }

    /// Re-run the search against `all_files` using the current query.
    /// Each element is `(display_path, absolute_path)`.
    pub fn run_search(&mut self, all_files: &[(String, PathBuf)]) {
        self.results.clear();
        self.selected = 0;

        if self.query.is_empty() {
            return;
        }

        let query_lower = self.query.to_lowercase();

        'outer: for (_, path) in all_files {
            let bytes = match fs::read(path) {
                Ok(b) => b,
                Err(_) => continue,
            };

            // Skip binary files (null byte heuristic).
            if bytes.contains(&0u8) {
                continue;
            }

            let text = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            for (idx, line) in text.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    self.results.push(SearchResult {
                        path: path.clone(),
                        line: idx as u32,
                        snippet: line.trim().to_string(),
                    });
                    if self.results.len() >= MAX_RESULTS {
                        break 'outer;
                    }
                }
            }
        }
    }
}
