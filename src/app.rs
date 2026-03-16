use std::path::{Path, PathBuf};

use anyhow::{Ok, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{editor::Editor, file_tree::FileTree, palette::CommandPalette};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    FileTree,
    Editor,
    Palette,
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
    pub palette: CommandPalette,
    pub focus: FocusPane,
    pub should_quit: bool,
    pub status: String,
}

impl App {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root_dir = root.as_ref().canonicalize()?;
        let file_tree = FileTree::new(&root_dir)?;
        let mut editor = Editor::default();

        if let Some(first_file) = file_tree.selected_path() {
            editor.open_file(first_file)?;
        }

        Ok(Self {
            root_dir: root_dir.clone(),
            file_tree,
            editor,
            palette: CommandPalette::default(),
            focus: FocusPane::FileTree,
            should_quit: false,
            status: format!("Noir ready — {}", root_dir.display()),
        })
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<Action> {
        if self.palette.open {
            return self.handle_palette_key(key);
        }

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
                KeyCode::Char('p') => {
                    self.palette.toggle();

                    if self.palette.open {
                        self.palette
                            .update_results(self.file_tree.all_display_paths());
                        self.focus = FocusPane::Palette;
                        self.status = "File search".to_string();
                    } else {
                        self.focus = FocusPane::Editor;
                        self.status = "Closed file search".to_string();
                    }

                    return Ok(Action::None);
                }
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char('.') => {
                    self.editor.next_tab();
                    self.status = format!("Tab: {}", self.editor.title());
                    return Ok(Action::None);
                }
                KeyCode::Char(',') => {
                    self.editor.prev_tab();
                    self.status = format!("Tab: {}", self.editor.title());
                    return Ok(Action::None);
                }
                _ => {}
            }
        }

        match self.focus {
            FocusPane::FileTree => self.handle_file_tree_key(key),
            FocusPane::Editor => self.handle_editor_key(key),
            FocusPane::Palette => self.handle_palette_key(key),
        }
    }

    fn handle_file_tree_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Up => self.file_tree.move_up(),
            KeyCode::Down => self.file_tree.move_down(),
            KeyCode::Enter => {
                if let Some(path) = self.file_tree.selected_path() {
                    self.editor.open_file(path)?;
                    self.focus = FocusPane::Editor;
                    self.status = format!("Opened {}", path.display());
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
            _ => {
                self.editor.handle_key(key);
                self.editor.ensure_cursor_visible(20, 80);
            }
        }

        Ok(Action::None)
    }

    fn handle_palette_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Esc => {
                self.palette.close();
                self.focus = FocusPane::Editor;
                self.status = "Closed file search".to_string();
            }
            KeyCode::Up => {
                self.palette.move_up();
            }
            KeyCode::Down => {
                self.palette.move_down();
            }
            KeyCode::Backspace => {
                self.palette.input.pop();
                self.palette
                    .update_results(self.file_tree.all_display_paths());
            }
            KeyCode::Enter => {
                if let Some(selected) = self.palette.selected_result() {
                    if let Some(path) = self.file_tree.find_full_path_by_display(selected) {
                        self.editor.open_file(path)?;
                        self.status = format!("Opened {}", selected);
                    }
                }
                self.palette.close();
                self.focus = FocusPane::Editor;
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.palette.input.push(c);
                self.palette
                    .update_results(self.file_tree.all_display_paths());
            }
            _ => {}
        }

        Ok(Action::None)
    }
}
