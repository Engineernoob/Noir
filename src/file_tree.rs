use std::path::{Component, Path, PathBuf};

use anyhow::Result;
use walkdir::{DirEntry, WalkDir};

pub struct FileEntry {
    pub full_path: PathBuf,
    pub display_path: String,
}

pub struct FileTree {
    entries: Vec<FileEntry>,
    selected: usize,
}

impl FileTree {
    pub fn new(root: &Path) -> Result<Self> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !should_skip(e))
            .filter_map(Result::ok)
        {
            let path = entry.path().to_path_buf();
            if path.is_file() {
                let display_path = path
                    .strip_prefix(root)
                    .ok()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| path.display().to_string());

                entries.push(FileEntry {
                    full_path: path,
                    display_path,
                });
            }
        }

        entries.sort_by(|a, b| a.display_path.cmp(&b.display_path));

        Ok(Self {
            entries,
            selected: 0,
        })
    }

    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.entries.get(self.selected).map(|e| &e.full_path)
    }

    pub fn all_display_paths(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|e| e.display_path.clone())
            .collect()
    }

    pub fn find_full_path_by_display(&self, display: &str) -> Option<&PathBuf> {
        self.entries
            .iter()
            .find(|e| e.display_path == display)
            .map(|e| &e.full_path)
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

fn should_skip(entry: &DirEntry) -> bool {
    let path = entry.path();

    for component in path.components() {
        if let Component::Normal(name) = component {
            if let Some(s) = name.to_str() {
                if matches!(s, ".git" | "target" | "node_modules" | ".idea" | ".vscode") {
                    return true;
                }
            }
        }
    }

    false
}
