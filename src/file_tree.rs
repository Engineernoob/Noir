use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

pub struct FileTree {
    entries: Vec<PathBuf>,
    selected: usize,
}

impl FileTree {
    pub fn new(root: &Path) -> Result<Self> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !is_hidden(e.path()))
        {
            let path = entry.path().to_path_buf();
            if path.is_file() {
                entries.push(path);
            }
        }

        entries.sort();

        Ok(Self {
            entries,
            selected: 0,
        })
    }

    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.entries.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }
}

fn is_hidden(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
    })
}
