use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{editor::Editor, file_tree::FileTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    FileTree,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
}

pub struct App {
    pub root_dir: PathBuf,
    pub file_tree: FileTree,
    pub editor: Editor,
    pub focus: FocusPane,
    pub should_quit: bool,
    pub status: String,
}

impl App {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root_dir = root.as_ref().canonicalize()?;
        let file_tree = FileTree::new(&root_dir)?;
        let mut editor = Editor::default();

        let status = format!("Noir ready — {}", root_dir.display());

        if let Some(first_file) = file_tree.selected_path().filter(|p| p.is_file()) {
            editor.open_file(first_file)?;
        }

        Ok(Self {
            root_dir,
            file_tree,
            editor,
            focus: FocusPane::FileTree,
            should_quit: false,
            status,
        })
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    return Ok(Action::Quit);
                }
                KeyCode::Char('s') => {
                    self.editor.save()?;
                    self.status = "Saved file".to_string();
                    return Ok(Action::None);
                }
                KeyCode::Char('b') => {
                    self.focus = FocusPane::FileTree;
                    self.status = "Focus: file tree".to_string();
                    return Ok(Action::None);
                }
                KeyCode::Char('e') => {
                    self.focus = FocusPane::Editor;
                    self.status = "Focus: editor".to_string();
                    return Ok(Action::None);
                }
                _ => {}
            }
        }

        match self.focus {
            FocusPane::FileTree => self.handle_file_tree_key(key),
            FocusPane::Editor => self.handle_editor_key(key),
        }
    }

    fn handle_file_tree_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Up => self.file_tree.move_up(),
            KeyCode::Down => self.file_tree.move_down(),
            KeyCode::Enter => {
                if let Some(path) = self.file_tree.selected_path() {
                    if path.is_file() {
                        self.editor.open_file(path)?;
                        self.focus = FocusPane::Editor;
                        self.status = format!("Opened {}", path.display());
                    }
                }
            }
            KeyCode::Tab => {
                self.focus = FocusPane::Editor;
                self.status = "Focus: editor".to_string();
            }
            _ => {}
        }

        Ok(Action::None)
    }

    fn handle_editor_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Tab => {
                self.focus = FocusPane::FileTree;
                self.status = "Focus: file tree".to_string();
            }
            _ => self.editor.handle_key(key),
        }

        Ok(Action::None)
    }
}
