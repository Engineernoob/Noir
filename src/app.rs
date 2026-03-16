use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    editor::Editor,
    file_tree::FileTree,
    lsp::{LspClient, LspDiagnostic, LspEvent, LspLocation},
    palette::CommandPalette,
    terminal::TerminalPane,
};

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
    pub lsp: Option<LspClient>,
    pub diagnostics: Vec<LspDiagnostic>,
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
        let mut terminal = TerminalPane::new();

        if let Some(first_file) = file_tree.selected_path() {
            editor.open_file(first_file)?;
        }

        terminal.init_shell()?;

        let mut lsp = LspClient::start(&root_dir).ok();
        if let Some(client) = lsp.as_mut() {
            let _ = client.initialize();
        }

        let mut app = Self {
            root_dir: root_dir.clone(),
            file_tree,
            editor,
            palette: CommandPalette::default(),
            terminal,
            lsp,
            diagnostics: Vec::new(),
            focus: FocusPane::FileTree,
            should_quit: false,
            status: format!("Noir ready — {}", root_dir.display()),
            editor_view_height: 1,
            editor_view_width: 1,
        };

        let _ = app.sync_current_buffer_with_lsp();

        Ok(app)
    }

    pub fn tick(&mut self) {
        self.terminal.poll_output();

        let events = if let Some(lsp) = &mut self.lsp {
            lsp.drain_events()
        } else {
            Vec::new()
        };

        for event in events {
            self.apply_lsp_event(event);
        }
    }

    pub fn set_editor_viewport(&mut self, height: usize, width: usize) {
        self.editor_view_height = height.max(1);
        self.editor_view_width = width.max(1);
        self.editor
            .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
    }

    pub fn resize_terminal_viewport(&mut self, rows: u16, cols: u16) {
        self.terminal.resize(rows.max(1), cols.max(1));
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
                    let _ = self.sync_current_buffer_with_lsp();
                    self.status = format!("Tab: {}", self.editor.title());
                    return Ok(Action::None);
                }
                KeyCode::Char(',') => {
                    self.editor.prev_tab();
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    let _ = self.sync_current_buffer_with_lsp();
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
                KeyCode::Char('k') => {
                    self.request_hover()?;
                    return Ok(Action::None);
                }
                KeyCode::Char('g') => {
                    self.request_definition()?;
                    return Ok(Action::None);
                }
                KeyCode::Char('t') => {
                    self.terminal.toggle();

                    if self.terminal.visible {
                        self.status = "Opened terminal pane".to_string();
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
                    let path = path.clone();

                    self.editor.open_file(&path)?;
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    let _ = self.sync_current_buffer_with_lsp();

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
                if self.terminal.visible {
                    self.focus = FocusPane::Terminal;
                    self.status = "Focus: terminal".to_string();
                } else {
                    self.focus = FocusPane::FileTree;
                    self.status = "Focus: file tree".to_string();
                }
            }
            _ => {
                let changed = self.editor.handle_key(key);
                self.editor
                    .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);

                if changed {
                    self.sync_current_buffer_change()?;
                }
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
            KeyCode::Up => self.palette.move_up(),
            KeyCode::Down => self.palette.move_down(),
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

                        self.editor.open_file(&path)?;
                        self.editor
                            .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                        let _ = self.sync_current_buffer_with_lsp();

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

    fn handle_terminal_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Up => self.terminal.scroll_up(),
            KeyCode::Down => self.terminal.scroll_down(),
            KeyCode::Tab => {
                self.focus = FocusPane::FileTree;
                self.status = "Focus: file tree".to_string();
            }
            KeyCode::Enter => {
                self.terminal.send_enter();
            }
            KeyCode::Backspace => {
                self.terminal.send_backspace();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.terminal.send_key_char(c);
            }
            KeyCode::Left => self.terminal.send_input("\u{1b}[D"),
            KeyCode::Right => self.terminal.send_input("\u{1b}[C"),
            KeyCode::Home => self.terminal.send_input("\u{1b}[H"),
            KeyCode::End => self.terminal.send_input("\u{1b}[F"),
            _ => {}
        }

        Ok(Action::None)
    }

    pub fn diagnostic_lines(&self) -> Vec<String> {
        let current_path = self.editor.current_buffer().file_path.as_ref();

        self.diagnostics
            .iter()
            .filter(|diagnostic| current_path == Some(&diagnostic.path))
            .map(LspDiagnostic::summary)
            .collect()
    }

    fn apply_lsp_event(&mut self, event: LspEvent) {
        match event {
            LspEvent::Diagnostics { uri, diagnostics } => {
                self.diagnostics.retain(|existing| existing.uri != uri);
                self.diagnostics.extend(diagnostics);
            }
            LspEvent::Hover { contents } => {
                self.status = match contents {
                    Some(contents) => format!("Hover: {}", truncate(&contents, 120)),
                    None => "Hover: no information".to_string(),
                };
            }
            LspEvent::Definition { location } => {
                if let Some(location) = location {
                    if let Err(err) = self.open_definition(location) {
                        self.status = format!("Definition failed: {err}");
                    }
                } else {
                    self.status = "Definition: no result".to_string();
                }
            }
            LspEvent::Status(message) => {
                self.status = message;
            }
        }
    }

    fn sync_current_buffer_with_lsp(&mut self) -> Result<()> {
        let Some((path, text, version)) = self.current_document_snapshot() else {
            return Ok(());
        };

        if let Some(lsp) = &mut self.lsp {
            lsp.open_document(&path, &text, version)?;
        }

        Ok(())
    }

    fn sync_current_buffer_change(&mut self) -> Result<()> {
        let Some((path, text, version)) = self.current_document_snapshot() else {
            return Ok(());
        };

        if let Some(lsp) = &mut self.lsp {
            lsp.change_document(&path, &text, version)?;
        }

        Ok(())
    }

    fn request_hover(&mut self) -> Result<()> {
        let Some((path, text, version)) = self.current_document_snapshot() else {
            self.status = "Hover: no file open".to_string();
            return Ok(());
        };

        let (line, character) = self.editor.current_lsp_position();

        if let Some(lsp) = &mut self.lsp {
            lsp.open_document(&path, &text, version)?;
            lsp.request_hover(&path, line, character)?;
            self.status = "Hover requested".to_string();
        } else {
            self.status = "LSP unavailable".to_string();
        }

        Ok(())
    }

    fn request_definition(&mut self) -> Result<()> {
        let Some((path, text, version)) = self.current_document_snapshot() else {
            self.status = "Definition: no file open".to_string();
            return Ok(());
        };

        let (line, character) = self.editor.current_lsp_position();

        if let Some(lsp) = &mut self.lsp {
            lsp.open_document(&path, &text, version)?;
            lsp.request_definition(&path, line, character)?;
            self.status = "Definition requested".to_string();
        } else {
            self.status = "LSP unavailable".to_string();
        }

        Ok(())
    }

    fn open_definition(&mut self, location: LspLocation) -> Result<()> {
        self.editor.open_file(&location.path)?;
        self.editor
            .set_cursor_from_lsp(location.line as usize, location.character as usize);
        self.editor
            .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
        self.sync_current_buffer_with_lsp()?;
        self.focus = FocusPane::Editor;
        self.status = format!(
            "Definition: {}:{}",
            location.path.display(),
            location.line + 1
        );
        Ok(())
    }

    fn current_document_snapshot(&self) -> Option<(PathBuf, String, i32)> {
        let buffer = self.editor.current_buffer();
        let path = buffer.file_path.clone()?;
        Some((path, self.editor.current_buffer_text(), buffer.version))
    }
}

fn truncate(text: &str, max_len: usize) -> String {
    let mut truncated = text.chars().take(max_len).collect::<String>();

    if text.chars().count() > max_len {
        truncated.push_str("...");
    }

    truncated
}
