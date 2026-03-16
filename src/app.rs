use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{editor::Editor, file_tree::FileTree, palette::CommandPalette, terminal::TerminalPane};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    FileTree,
    Editor,
    Palette,
    Terminal,
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
    pub terminal: TerminalPane,
    pub focus: FocusPane,
    pub should_quit: bool,
    pub status: String,
    pub editor_view_height: usize,
    pub editor_view_width: usize,
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
            terminal: TerminalPane::new(),
            focus: FocusPane::FileTree,
            should_quit: false,
            status: format!("Noir ready — {}", root_dir.display()),
            editor_view_height: 1,
            editor_view_width: 1,
        })
    }

    pub fn set_editor_viewport(&mut self, height: usize, width: usize) {
        self.editor_view_height = height.max(1);
        self.editor_view_width = width.max(1);
        self.editor
            .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<Action> {
        if self.palette.open {
            return self.handle_palette_key(key);
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char('1') => {
                    self.focus = FocusPane::FileTree;
                    self.status = "Focus: file tree".to_string();
                    return Ok(Action::None);
                }
                KeyCode::Char('2') => {
                    self.focus = FocusPane::Editor;
                    self.status = "Focus: editor".to_string();
                    return Ok(Action::None);
                }
                KeyCode::Char('3') => {
                    if self.terminal.visible {
                        self.focus = FocusPane::Terminal;
                        self.status = "Focus: terminal".to_string();
                    }
                    return Ok(Action::None);
                }
                KeyCode::Char('.') => {
                    self.editor.next_tab();
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    self.status = format!("Tab: {}", self.editor.title());
                    return Ok(Action::None);
                }
                KeyCode::Char(',') => {
                    self.editor.prev_tab();
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    self.status = format!("Tab: {}", self.editor.title());
                    return Ok(Action::None);
                }
                _ => {}
            }
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
                    self.terminal.push_line("Saved current file.");
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
                KeyCode::Char('t') => {
                    self.terminal.toggle();

                    if self.terminal.visible {
                        self.status = "Opened terminal pane".to_string();
                        self.terminal.push_line("Terminal pane opened.");
                    } else {
                        if self.focus == FocusPane::Terminal {
                            self.focus = FocusPane::Editor;
                        }
                        self.status = "Closed terminal pane".to_string();
                    }

                    return Ok(Action::None);
                }
                _ => {}
            }
        }

        match self.focus {
            FocusPane::FileTree => self.handle_file_tree_key(key),
            FocusPane::Editor => self.handle_editor_key(key),
            FocusPane::Palette => self.handle_palette_key(key),
            FocusPane::Terminal => self.handle_terminal_key(key),
        }
    }

    fn handle_file_tree_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Up => self.file_tree.move_up(),
            KeyCode::Down => self.file_tree.move_down(),
            KeyCode::Enter => {
                if let Some(path) = self.file_tree.selected_path() {
                    self.editor.open_file(path)?;
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    self.focus = FocusPane::Editor;
                    self.status = format!("Opened {}", path.display());
                    self.terminal
                        .push_line(format!("Opened file: {}", path.display()));
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
                if self.terminal.visible {
                    self.focus = FocusPane::Terminal;
                    self.status = "Focus: terminal".to_string();
                } else {
                    self.focus = FocusPane::FileTree;
                    self.status = "Focus: file tree".to_string();
                }
            }
            _ => {
                self.editor.handle_key(key);
                self.editor
                    .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
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
                let selected = self.palette.selected_result().map(str::to_string);

                if let Some(selected) = selected {
                    if let Some(path) = self.file_tree.find_full_path_by_display(&selected) {
                        let path = path.clone();
                        self.editor.open_file(path)?;
                        self.editor
                            .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                        self.status = format!("Opened {}", selected);
                        self.terminal
                            .push_line(format!("Palette opened file: {}", selected));
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

    fn handle_terminal_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Up => self.terminal.scroll_up(),
            KeyCode::Down => self.terminal.scroll_down(),
            KeyCode::Tab => {
                self.focus = FocusPane::FileTree;
                self.status = "Focus: file tree".to_string();
            }
            KeyCode::Enter => {
                self.terminal
                    .push_line("Terminal placeholder received Enter.");
                self.status = "Terminal input placeholder".to_string();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.terminal
                    .push_line(format!("Terminal placeholder input: {}", c));
            }
            _ => {}
        }

        Ok(Action::None)
    }
}
