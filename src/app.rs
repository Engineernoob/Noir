use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    commands::{CommandId, CommandRegistry},
    editor::Editor,
    file_tree::FileTree,
    languages::LanguageRegistry,
    lsp::{DiagnosticSeverity, LspClient, LspDiagnostic, LspEvent},
    palette::{CommandPalette, PaletteMode},
    plugins::PluginManager,
    search::SearchPanel,
    terminal::TerminalPane,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    FileTree,
    Editor,
    Palette,
    Search,
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
    /// Maps file extensions to language IDs and LSP server commands.
    pub registry: LanguageRegistry,
    pub commands: CommandRegistry,
    pub plugins: PluginManager,
    pub file_tree: FileTree,
    pub editor: Editor,
    pub palette: CommandPalette,
    pub search: SearchPanel,
    pub terminal: TerminalPane,
    pub lsp: Option<LspClient>,
    /// Raw diagnostics per file path, as received from the LSP server.
    pub diagnostics: HashMap<PathBuf, Vec<LspDiagnostic>>,
    pub hover: Option<String>,
    pub hover_visible: bool,
    /// Whether the diagnostics pane is visible in the bottom slot.
    pub diagnostics_open: bool,
    /// Highlighted row in the diagnostics list.
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

        let initial_file = file_tree
            .selected_path()
            .or_else(|| file_tree.first_file_path())
            .cloned();
        if let Some(path) = &initial_file {
            editor.open_file(path)?;
        }

        terminal.init_shell()?;

        // Build the language registry. Additional servers can be registered here
        // as Noir gains support for more languages.
        let registry = LanguageRegistry::default();

        // Start an LSP server for the initial file's language, if a server is
        // registered for it. Falls back gracefully when no server is found.
        let server_config = initial_file
            .as_deref()
            .and_then(|p| registry.lsp_for_path(p))
            .cloned();

        let mut lsp = server_config
            .and_then(|cfg| LspClient::start(&root_dir, cfg).ok());

        if let Some(client) = lsp.as_mut() {
            let _ = client.initialize();
        }

        // Discover plugins from `<root>/.noir/plugins/`. Missing directory is fine.
        let mut plugins = PluginManager::new();
        let _ = plugins.discover(&root_dir.join(".noir").join("plugins"));

        let mut app = Self {
            root_dir: root_dir.clone(),
            registry,
            commands: CommandRegistry::default(),
            plugins,
            file_tree,
            editor,
            palette: CommandPalette::default(),
            search: SearchPanel::default(),
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
        if self.search.open {
            return self.handle_search_key(key);
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
                    self.status = "Saved".to_string();
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
                        self.refresh_palette_results();
                        self.focus = FocusPane::Palette;
                        self.status =
                            "File search  [type] filter  [>] commands  [Esc] close".to_string();
                    } else {
                        self.focus = FocusPane::Editor;
                        self.status = "Closed palette".to_string();
                    }
                    return Ok(Action::None);
                }
                KeyCode::Char('o') => {
                    // Ctrl+O opens command palette directly in command mode
                    self.palette.open_command_mode();
                    self.refresh_palette_results();
                    self.focus = FocusPane::Palette;
                    self.status =
                        "Commands  [type] filter  [Backspace] file search  [Esc] close".to_string();
                    return Ok(Action::None);
                }
                KeyCode::Char('f') => {
                    self.search.open();
                    self.focus = FocusPane::Search;
                    self.status = "Text search  [type to search]  [Enter] open  [Esc] close"
                        .to_string();
                    return Ok(Action::None);
                }
                KeyCode::Char('t') => {
                    self.terminal.toggle();
                    if self.terminal.visible {
                        self.status = "Opened terminal".to_string();
                    } else {
                        if self.focus == FocusPane::Terminal {
                            self.focus = FocusPane::Editor;
                        }
                        self.status = "Closed terminal".to_string();
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
            FocusPane::Search => self.handle_search_key(key),
            FocusPane::Terminal => self.handle_terminal_key(key),
            FocusPane::Diagnostics => self.handle_diagnostics_key(key),
        }
    }

    fn handle_file_tree_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Up => self.file_tree.move_up(),
            KeyCode::Down => self.file_tree.move_down(),
            KeyCode::Right => self.file_tree.expand_selected(),
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
                self.status = "Closed palette".to_string();
            }
            KeyCode::Up => self.palette.move_up(),
            KeyCode::Down => self.palette.move_down(),
            KeyCode::Backspace => {
                if self.palette.mode == PaletteMode::Command && self.palette.input.is_empty() {
                    // Backspace past command prompt → return to file mode.
                    self.palette.mode = PaletteMode::File;
                    self.status =
                        "File search  [type] filter  [>] commands  [Esc] close".to_string();
                } else {
                    self.palette.input.pop();
                }
                self.refresh_palette_results();
            }
            KeyCode::Enter => {
                match self.palette.mode {
                    PaletteMode::File => {
                        if let Some(selected) = self.palette.selected_result().map(str::to_string) {
                            if let Some(path) =
                                self.file_tree.find_full_path_by_display(&selected)
                            {
                                let path = path.clone();
                                self.editor.open_file(&path)?;
                                self.editor.ensure_cursor_visible(
                                    self.editor_view_height,
                                    self.editor_view_width,
                                );
                                let _ = self.sync_current_buffer_open();
                                self.status = format!("Opened {}", selected);
                            }
                        }
                        self.palette.close();
                        self.focus = FocusPane::Editor;
                    }
                    PaletteMode::Command => {
                        if let Some(name) = self.palette.selected_result().map(str::to_string) {
                            if let Some(cmd) = self.commands.find_by_name(&name) {
                                let id = cmd.id;
                                self.palette.close();
                                self.focus = FocusPane::Editor;
                                return self.execute_command(id);
                            }
                        }
                    }
                }
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // `>` as the first character in File mode switches to Command mode.
                if self.palette.mode == PaletteMode::File && self.palette.input.is_empty() && c == '>' {
                    self.palette.mode = PaletteMode::Command;
                    self.status =
                        "Commands  [type] filter  [Backspace] file search  [Esc] close".to_string();
                } else {
                    self.palette.input.push(c);
                }
                self.refresh_palette_results();
            }
            _ => {}
        }
        Ok(Action::None)
    }

    /// Populate palette results based on the current mode.
    fn refresh_palette_results(&mut self) {
        match self.palette.mode {
            PaletteMode::File => {
                self.palette
                    .update_results(self.file_tree.all_display_paths());
            }
            PaletteMode::Command => {
                let names: Vec<String> = self
                    .commands
                    .fuzzy_filter(&self.palette.input)
                    .into_iter()
                    .map(|c| c.name.to_string())
                    .collect();
                self.palette.results = names;
                if self.palette.selected >= self.palette.results.len() {
                    self.palette.selected = self.palette.results.len().saturating_sub(1);
                }
            }
        }
    }

    /// Execute a registered command by ID.
    fn execute_command(&mut self, id: CommandId) -> Result<Action> {
        match id {
            CommandId::OpenFile => {
                self.palette.open();
                self.refresh_palette_results();
                self.focus = FocusPane::Palette;
                self.status =
                    "File search  [type] filter  [>] commands  [Esc] close".to_string();
            }
            CommandId::SearchProject => {
                self.search.open();
                self.focus = FocusPane::Search;
                self.status =
                    "Text search  [type to search]  [Enter] open  [Esc] close".to_string();
            }
            CommandId::ToggleTerminal => {
                self.terminal.toggle();
                if self.terminal.visible {
                    self.status = "Opened terminal".to_string();
                } else {
                    if self.focus == FocusPane::Terminal {
                        self.focus = FocusPane::Editor;
                    }
                    self.status = "Closed terminal".to_string();
                }
            }
            CommandId::GotoDefinition => {
                self.request_definition()?;
            }
            CommandId::Hover => {
                self.request_hover()?;
            }
            CommandId::ToggleDiagnostics => {
                self.toggle_diagnostics();
            }
            CommandId::FocusFileTree => {
                self.focus = FocusPane::FileTree;
                self.status = "Focus: file tree".to_string();
            }
            CommandId::FocusEditor => {
                self.focus = FocusPane::Editor;
                self.status = "Focus: editor".to_string();
            }
            CommandId::Save => {
                self.editor.save()?;
                self.status = "Saved".to_string();
            }
            CommandId::Quit => {
                self.should_quit = true;
                return Ok(Action::Quit);
            }
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
            KeyCode::Enter => self.terminal.send_enter(),
            KeyCode::Backspace => self.terminal.send_backspace(),
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
            KeyCode::Enter => self.jump_to_diagnostic(),
            KeyCode::Esc | KeyCode::Tab => {
                self.diagnostics_open = false;
                self.focus = FocusPane::Editor;
                self.status = "Closed diagnostics".to_string();
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Esc => {
                self.search.close();
                self.focus = FocusPane::Editor;
                self.status = "Closed search".to_string();
            }
            KeyCode::Up => self.search.move_up(),
            KeyCode::Down => self.search.move_down(),
            KeyCode::Enter => {
                if let Some(result) = self.search.selected_result().cloned() {
                    let path = result.path.clone();
                    let line = result.line;
                    if let Err(e) = self.editor.open_file(&path) {
                        self.status = format!("Cannot open {}: {e}", path.display());
                        return Ok(Action::None);
                    }
                    self.editor.current_buffer_mut().cursor_row = line as usize;
                    self.editor.current_buffer_mut().cursor_col = 0;
                    self.editor
                        .ensure_cursor_visible(self.editor_view_height, self.editor_view_width);
                    let _ = self.sync_current_buffer_open();
                    self.search.close();
                    self.focus = FocusPane::Editor;
                    self.status =
                        format!("Jumped to {}:{}", path.display(), line + 1);
                }
            }
            KeyCode::Backspace => {
                self.search.query.pop();
                let all_files = self.file_tree.all_file_paths().to_vec();
                self.search.run_search(&all_files);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search.query.push(c);
                let all_files = self.file_tree.all_file_paths().to_vec();
                self.search.run_search(&all_files);
            }
            _ => {}
        }
        Ok(Action::None)
    }

    // ── LSP event handling ────────────────────────────────────────────────────

    fn apply_lsp_event(&mut self, event: LspEvent) {
        match event {
            LspEvent::Initialized => {
                self.status = "LSP server ready".to_string();
            }
            LspEvent::Shutdown => {
                self.status = "LSP server shut down".to_string();
            }
            LspEvent::Diagnostics { path, diagnostics } => {
                // An empty list means all diagnostics for this file are cleared.
                if diagnostics.is_empty() {
                    self.diagnostics.remove(&path);
                } else {
                    self.diagnostics.insert(path, diagnostics);
                }
                self.rebuild_diagnostic_entries();
                let errors = self.diagnostic_error_count();
                let warnings = self.diagnostic_warning_count();
                self.status =
                    format!("Diagnostics: {errors} error(s), {warnings} warning(s)");
            }
            LspEvent::Hover { contents } => {
                self.hover = contents;
                self.hover_visible = self.hover.is_some();
                self.status = if self.hover_visible {
                    "Hover info loaded".to_string()
                } else {
                    "No hover information".to_string()
                };
            }
            LspEvent::Definition { location } => {
                self.apply_definition(location);
            }
            LspEvent::LogMessage(msg) => {
                self.status = format!("LSP: {msg}");
            }
            LspEvent::TransportError(err) => {
                self.status = format!("LSP error: {err}");
            }
            LspEvent::ServerExited => {
                self.status = "LSP server exited".to_string();
                self.lsp = None;
            }
        }
    }

    // ── Diagnostics helpers ───────────────────────────────────────────────────

    /// Flatten `self.diagnostics` into `self.diagnostics_entries`, errors first.
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

        // Sort: errors → warnings → info → hints, then by file + line.
        entries.sort_by(|a, b| {
            severity_sort_key(a.severity)
                .cmp(&severity_sort_key(b.severity))
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.line.cmp(&b.line))
        });

        self.diagnostics_entries = entries;

        if self.diagnostics_entries.is_empty() {
            self.diagnostics_selected = 0;
        } else {
            self.diagnostics_selected =
                self.diagnostics_selected.min(self.diagnostics_entries.len() - 1);
        }
    }

    fn toggle_diagnostics(&mut self) {
        self.diagnostics_open = !self.diagnostics_open;
        if self.diagnostics_open {
            self.focus = FocusPane::Diagnostics;
            let n = self.diagnostics_entries.len();
            self.status = if n == 0 {
                "Diagnostics — no issues  [Esc/Tab] close".to_string()
            } else {
                format!("Diagnostics — {n} issue(s)  [↑↓] navigate  [Enter] jump  [Esc/Tab] close")
            };
        } else {
            if self.focus == FocusPane::Diagnostics {
                self.focus = FocusPane::Editor;
            }
            self.status = "Closed diagnostics".to_string();
        }
    }

    fn jump_to_diagnostic(&mut self) {
        let Some(entry) = self.diagnostics_entries.get(self.diagnostics_selected) else {
            return;
        };
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

    // ── LSP document sync ─────────────────────────────────────────────────────

    /// Update syntax highlighting and notify the LSP server that the current
    /// buffer has been opened. Call this whenever the active file changes.
    fn sync_current_buffer_open(&mut self) -> Result<()> {
        self.update_syntax_for_current_buffer();
        let Some((path, text, version, lang)) = self.current_document_snapshot() else {
            return Ok(());
        };
        if let Some(lsp) = &mut self.lsp {
            lsp.open_document(&path, text, version, &lang)?;
        }
        Ok(())
    }

    /// Notify the LSP server that the current buffer has changed.
    fn sync_current_buffer_change(&mut self) -> Result<()> {
        let Some((path, text, version, lang)) = self.current_document_snapshot() else {
            return Ok(());
        };
        if let Some(lsp) = &mut self.lsp {
            lsp.change_document(&path, text, version, &lang)?;
        }
        Ok(())
    }

    /// Returns `(path, text, version, language_id)` for the active buffer, or
    /// `None` if no file is open. Language ID is resolved once here so callers
    /// don't repeat the registry lookup.
    fn current_document_snapshot(&self) -> Option<(PathBuf, String, i32, String)> {
        let buf = self.editor.current_buffer();
        let path = buf.file_path.clone()?;
        let lang = self.registry.language_id_for_path(&path).to_string();
        Some((path, self.editor.current_buffer_text(), buf.version, lang))
    }

    /// Switch the syntax highlighter to match the current buffer's language.
    /// Call this whenever the active file changes (open, tab switch).
    fn update_syntax_for_current_buffer(&mut self) {
        let highlight_fn = self
            .editor
            .current_buffer()
            .file_path
            .as_deref()
            .and_then(|p| self.registry.highlight_for_path(p));
        self.editor.syntax.set_language(highlight_fn);
    }

    // ── LSP feature requests ──────────────────────────────────────────────────

    fn request_hover(&mut self) -> Result<()> {
        let Some((path, _, _, _)) = self.current_document_snapshot() else {
            self.status = "Hover unavailable: no file open".to_string();
            return Ok(());
        };
        let (line, character) = self.editor.current_lsp_position();
        if let Some(lsp) = &mut self.lsp {
            lsp.hover(&path, line, character)?;
            self.status = "Hover requested…".to_string();
        } else {
            self.status = "LSP unavailable".to_string();
        }
        Ok(())
    }

    fn request_definition(&mut self) -> Result<()> {
        let Some((path, _, _, _)) = self.current_document_snapshot() else {
            self.status = "Go to definition: no file open".to_string();
            return Ok(());
        };
        let (line, character) = self.editor.current_lsp_position();
        if let Some(lsp) = &mut self.lsp {
            lsp.definition(&path, line, character)?;
            self.status = "Go to definition requested…".to_string();
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

    // ── Shutdown ──────────────────────────────────────────────────────────────

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
