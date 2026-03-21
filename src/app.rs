use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    editor::Editor,
    file_tree::FileTree,
    lsp::{DiagnosticSeverity, LspClient, LspDiagnostic, LspEvent},
    palette::CommandPalette,
    terminal::TerminalPane,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    FileTree,
    Editor,
    Palette,
    Terminal,
    Diagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
}

/// A single flattened diagnostic entry used by the diagnostics pane.
#[derive(Debug, Clone)]
pub struct DiagnosticEntry {
    pub path: PathBuf,
    pub line: u32,
    pub character: u32,
    pub severity: Option<DiagnosticSeverity>,
    pub message: String,
}

pub struct App {
    pub root_dir: PathBuf,
    pub file_tree: FileTree,
    pub editor: Editor,
    pub palette: CommandPalette,
    pub terminal: TerminalPane,
    pub lsp: Option<LspClient>,
    pub diagnostics: HashMap<PathBuf, Vec<LspDiagnostic>>,
    pub hover: Option<String>,
    pub hover_visible: bool,
    /// Whether the diagnostics pane is shown in the bottom slot.
    pub diagnostics_open: bool,
    /// Currently highlighted row in the diagnostics list.
    pub diagnostics_selected: usize,
    /// Flattened, sorted list rebuilt whenever `self.diagnostics` changes.
    pub diagnostics_entries: Vec<DiagnosticEntry>,
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

        // selected_path() is None when the first visible entry is a directory,
        // which is the common case now that dirs are sorted before files.
        let initial_file = file_tree
            .selected_path()
            .or_else(|| file_tree.first_file_path())
            .cloned();
        if let Some(path) = initial_file {
            editor.open_file(&path)?;
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
            diagnostics: HashMap::new(),
            hover: None,
            hover_visible: false,
            diagnostics_open: false,
            diagnostics_selected: 0,
            diagnostics_entries: Vec::new(),
            focus: FocusPane::FileTree,
            should_quit: false,
            status: format!("Noir ready — {}", root_dir.display()),
            editor_view_height: 1,
            editor_view_width: 1,
        };

        let _ = app.sync_current_buffer_open();
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
                KeyCode::Char('4') => {
                    if self.diagnostics_open {
                        self.focus = FocusPane::Diagnostics;
                        self.status = "Focus: diagnostics".to_string();
                    }
                    return Ok(Action::None);
                }
                KeyCode::Char('.') => {
                    self.editor.next_tab();
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    let _ = self.sync_current_buffer_open();
                    self.status = format!("Tab: {}", self.editor.title());
                    return Ok(Action::None);
                }
                KeyCode::Char(',') => {
                    self.editor.prev_tab();
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    let _ = self.sync_current_buffer_open();
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
                KeyCode::Char('k') => {
                    self.request_hover()?;
                    return Ok(Action::None);
                }
                KeyCode::Char('g') => {
                    self.request_definition()?;
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
                    } else {
                        if self.focus == FocusPane::Terminal {
                            self.focus = FocusPane::Editor;
                        }
                        self.status = "Closed terminal pane".to_string();
                    }

                    return Ok(Action::None);
                }
                KeyCode::Char('d') => {
                    self.toggle_diagnostics();
                    return Ok(Action::None);
                }
                _ => {}
            }
        }

        if key.code == KeyCode::F(12) {
            self.request_definition()?;
            return Ok(Action::None);
        }

        match self.focus {
            FocusPane::FileTree => self.handle_file_tree_key(key),
            FocusPane::Editor => self.handle_editor_key(key),
            FocusPane::Palette => self.handle_palette_key(key),
            FocusPane::Terminal => self.handle_terminal_key(key),
            FocusPane::Diagnostics => self.handle_diagnostics_key(key),
        }
    }

    fn handle_file_tree_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Up => self.file_tree.move_up(),
            KeyCode::Down => self.file_tree.move_down(),
            // Right: expand a collapsed dir; no-op otherwise.
            KeyCode::Right => self.file_tree.expand_selected(),
            // Left: collapse an expanded dir, or jump to parent dir.
            KeyCode::Left => self.file_tree.collapse_selected(),
            KeyCode::Enter => {
                if self.file_tree.selected_is_dir() {
                    self.file_tree.toggle_expand();
                } else if let Some(path) = self.file_tree.selected_path() {
                    let path = path.clone();

                    self.editor.open_file(&path)?;
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    let _ = self.sync_current_buffer_open();

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
            KeyCode::Esc => {
                if self.hover_visible {
                    self.hover_visible = false;
                    self.status = "Closed hover".to_string();
                }
            }
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
                        let _ = self.sync_current_buffer_open();

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

    fn handle_diagnostics_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Up => {
                if self.diagnostics_selected > 0 {
                    self.diagnostics_selected -= 1;
                }
            }
            KeyCode::Down => {
                if !self.diagnostics_entries.is_empty()
                    && self.diagnostics_selected + 1 < self.diagnostics_entries.len()
                {
                    self.diagnostics_selected += 1;
                }
            }
            KeyCode::Enter => {
                self.jump_to_diagnostic();
            }
            KeyCode::Esc => {
                self.diagnostics_open = false;
                self.focus = FocusPane::Editor;
                self.status = "Closed diagnostics".to_string();
            }
            KeyCode::Tab => {
                self.focus = FocusPane::Editor;
                self.status = "Focus: editor".to_string();
            }
            _ => {}
        }

        Ok(Action::None)
    }

    fn apply_lsp_event(&mut self, event: LspEvent) {
        match event {
            LspEvent::Initialized => self.status = "rust-analyzer ready".to_string(),
            LspEvent::Shutdown => self.status = "rust-analyzer shutdown".to_string(),
            LspEvent::Diagnostics { path, diagnostics } => {
                if diagnostics.is_empty() {
                    self.diagnostics.remove(&path);
                } else {
                    self.diagnostics.insert(path, diagnostics);
                }
                self.rebuild_diagnostic_entries();
                let errors = self.diagnostic_error_count();
                let warnings = self.diagnostic_warning_count();
                self.status = format!("Diagnostics: {errors} error(s), {warnings} warning(s)");
            }
            LspEvent::Hover { contents } => {
                self.hover = contents;
                self.hover_visible = self.hover.is_some();
                self.status = if self.hover_visible {
                    "Hover loaded".to_string()
                } else {
                    "No hover information".to_string()
                };
            }
            LspEvent::Definition { location } => {
                self.apply_definition(location);
            }
            LspEvent::LogMessage(message) => self.status = format!("LSP: {message}"),
            LspEvent::TransportError(error) => self.status = format!("LSP error: {error}"),
            LspEvent::ServerExited => self.status = "rust-analyzer exited".to_string(),
        }
    }

    /// Flatten `self.diagnostics` into `self.diagnostics_entries`, sorted errors-first.
    fn rebuild_diagnostic_entries(&mut self) {
        let mut entries: Vec<DiagnosticEntry> = Vec::new();
        let mut paths: Vec<PathBuf> = self.diagnostics.keys().cloned().collect();
        paths.sort();

        for path in &paths {
            if let Some(diags) = self.diagnostics.get(path) {
                for d in diags {
                    entries.push(DiagnosticEntry {
                        path: path.clone(),
                        line: d.line,
                        character: d.character,
                        severity: d.severity,
                        message: d.message.clone(),
                    });
                }
            }
        }

        // Errors → warnings → info → hints, then by file + line within each bucket.
        entries.sort_by(|a, b| {
            severity_sort_key(a.severity)
                .cmp(&severity_sort_key(b.severity))
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.line.cmp(&b.line))
        });

        self.diagnostics_entries = entries;

        // Keep selection in bounds.
        if self.diagnostics_entries.is_empty() {
            self.diagnostics_selected = 0;
        } else if self.diagnostics_selected >= self.diagnostics_entries.len() {
            self.diagnostics_selected = self.diagnostics_entries.len() - 1;
        }
    }

    fn toggle_diagnostics(&mut self) {
        self.diagnostics_open = !self.diagnostics_open;
        if self.diagnostics_open {
            self.focus = FocusPane::Diagnostics;
            let n = self.diagnostics_entries.len();
            self.status = if n == 0 {
                "Diagnostics — no issues  [Esc] close".to_string()
            } else {
                format!("Diagnostics — {n} issue(s)  [↑↓] navigate  [Enter] jump  [Esc] close")
            };
        } else {
            if self.focus == FocusPane::Diagnostics {
                self.focus = FocusPane::Editor;
            }
            self.status = "Closed diagnostics".to_string();
        }
    }

    fn jump_to_diagnostic(&mut self) {
        if self.diagnostics_entries.is_empty() {
            return;
        }

        let entry = &self.diagnostics_entries[self.diagnostics_selected];
        let path = entry.path.clone();
        let line = entry.line;
        let character = entry.character;

        if let Err(e) = self.editor.open_file(&path) {
            self.status = format!("Cannot open {}: {e}", path.display());
            return;
        }

        {
            let buf = self.editor.current_buffer_mut();
            buf.cursor_row = line as usize;
            buf.cursor_col = character as usize;
        }

        self.editor
            .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
        let _ = self.sync_current_buffer_open();

        self.focus = FocusPane::Editor;
        self.status = format!("Jumped to {}:{}", path.display(), line + 1);
    }

    pub fn diagnostic_error_count(&self) -> usize {
        self.diagnostics_entries
            .iter()
            .filter(|e| e.severity == Some(DiagnosticSeverity::ERROR))
            .count()
    }

    pub fn diagnostic_warning_count(&self) -> usize {
        self.diagnostics_entries
            .iter()
            .filter(|e| e.severity == Some(DiagnosticSeverity::WARNING))
            .count()
    }

    fn sync_current_buffer_open(&mut self) -> Result<()> {
        let Some((path, text, version)) = self.current_document_snapshot() else {
            return Ok(());
        };

        if let Some(lsp) = &mut self.lsp {
            lsp.open_document(&path, text, version)?;
        }

        Ok(())
    }

    fn sync_current_buffer_change(&mut self) -> Result<()> {
        let Some((path, text, version)) = self.current_document_snapshot() else {
            return Ok(());
        };

        if let Some(lsp) = &mut self.lsp {
            lsp.change_document(&path, text, version)?;
        }

        Ok(())
    }

    fn current_document_snapshot(&self) -> Option<(PathBuf, String, i32)> {
        let buffer = self.editor.current_buffer();
        let path = buffer.file_path.clone()?;
        Some((path, self.editor.current_buffer_text(), buffer.version))
    }

    fn request_hover(&mut self) -> Result<()> {
        let Some((path, _, _)) = self.current_document_snapshot() else {
            self.status = "Hover unavailable: no file open".to_string();
            return Ok(());
        };

        let (line, character) = self.editor.current_lsp_position();

        if let Some(lsp) = &mut self.lsp {
            lsp.hover(&path, line, character)?;
            self.status = "Hover requested".to_string();
        } else {
            self.status = "LSP unavailable".to_string();
        }

        Ok(())
    }

    fn request_definition(&mut self) -> Result<()> {
        let Some((path, _, _)) = self.current_document_snapshot() else {
            self.status = "Go to definition: no file open".to_string();
            return Ok(());
        };

        let (line, character) = self.editor.current_lsp_position();

        if let Some(lsp) = &mut self.lsp {
            lsp.definition(&path, line, character)?;
            self.status = "Go to definition requested".to_string();
        } else {
            self.status = "LSP unavailable".to_string();
        }

        Ok(())
    }

    fn apply_definition(&mut self, location: Option<(PathBuf, u32, u32)>) {
        let Some((path, line, character)) = location else {
            self.status = "No definition found".to_string();
            return;
        };

        if let Err(e) = self.editor.open_file(&path) {
            self.status = format!("Definition: failed to open {}: {e}", path.display());
            return;
        }

        {
            let buf = self.editor.current_buffer_mut();
            buf.cursor_row = line as usize;
            buf.cursor_col = character as usize;
        }

        self.editor
            .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
        let _ = self.sync_current_buffer_open();

        self.focus = FocusPane::Editor;
        self.status = format!("Definition: {}:{}", path.display(), line + 1);
    }

    pub fn shutdown(&mut self) {
        if let Some(lsp) = &mut self.lsp {
            let _ = lsp.shutdown();
            let _ = lsp.exit();
        }
    }
}

fn severity_sort_key(severity: Option<DiagnosticSeverity>) -> u8 {
    match severity {
        Some(DiagnosticSeverity::ERROR) => 0,
        Some(DiagnosticSeverity::WARNING) => 1,
        Some(DiagnosticSeverity::INFORMATION) => 2,
        Some(DiagnosticSeverity::HINT) => 3,
        _ => 4,
    }
}
