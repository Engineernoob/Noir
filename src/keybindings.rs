use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::commands::CommandId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    Command(CommandId),
    OpenCommandPalette,
    FocusTerminal,
    FocusDiagnostics,
    NextEditorTab,
    PrevEditorTab,
    FileTreeMoveUp,
    FileTreeMoveDown,
    FileTreeExpand,
    FileTreeCollapse,
    FileTreeOpenSelected,
    EditorDismissHover,
    EditorCycleFocus,
    PaletteClose,
    PaletteMoveUp,
    PaletteMoveDown,
    PaletteBackspace,
    PaletteSubmit,
    SearchClose,
    SearchMoveUp,
    SearchMoveDown,
    SearchSubmit,
    SearchBackspace,
    TerminalScrollUp,
    TerminalScrollDown,
    TerminalFocusFileTree,
    TerminalSendEnter,
    TerminalSendBackspace,
    TerminalSendLeft,
    TerminalSendRight,
    TerminalSendHome,
    TerminalSendEnd,
    DiagnosticsMoveUp,
    DiagnosticsMoveDown,
    DiagnosticsSubmit,
    DiagnosticsClose,
    PromptClose,
    PromptBackspace,
    PromptSubmit,
    KeybindingHelpClose,
    KeybindingHelpMoveUp,
    KeybindingHelpMoveDown,
    KeybindingHelpPageUp,
    KeybindingHelpPageDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyContext {
    Global,
    FileTree,
    Editor,
    Palette,
    Search,
    Terminal,
    Diagnostics,
    Prompt,
    KeybindingHelp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyChord {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyChord {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn display(self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt".to_string());
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift".to_string());
        }

        parts.push(match self.code {
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::BackTab => "BackTab".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Insert => "Insert".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::F(n) => format!("F{n}"),
            KeyCode::Char(c) => c.to_ascii_uppercase().to_string(),
            other => format!("{other:?}"),
        });

        parts.join("+")
    }

    fn from_event(event: KeyEvent) -> Self {
        let code = match event.code {
            KeyCode::Char(c) if event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                KeyCode::Char(c.to_ascii_lowercase())
            }
            other => other,
        };

        Self {
            code,
            modifiers: event.modifiers,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Keybinding {
    context: KeyContext,
    chord: KeyChord,
    action: EditorAction,
}

impl Keybinding {
    const fn new(context: KeyContext, chord: KeyChord, action: EditorAction) -> Self {
        Self {
            context,
            chord,
            action,
        }
    }
}

pub struct KeybindingRegistry {
    bindings: Vec<Keybinding>,
}

#[derive(Debug, Clone)]
pub struct KeybindingHelpEntry {
    pub context: &'static str,
    pub shortcut: String,
    pub description: &'static str,
}

impl Default for KeybindingRegistry {
    fn default() -> Self {
        use EditorAction as Action;
        use KeyContext as Context;

        Self {
            bindings: vec![
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('1'), KeyModifiers::ALT), Action::Command(CommandId::FocusFileTree)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('2'), KeyModifiers::ALT), Action::Command(CommandId::FocusEditor)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('3'), KeyModifiers::ALT), Action::FocusTerminal),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('4'), KeyModifiers::ALT), Action::FocusDiagnostics),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('.'), KeyModifiers::ALT), Action::NextEditorTab),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char(','), KeyModifiers::ALT), Action::PrevEditorTab),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL), Action::Command(CommandId::Quit)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('s'), KeyModifiers::CONTROL), Action::Command(CommandId::Save)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('n'), KeyModifiers::CONTROL), Action::Command(CommandId::CreateFile)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('b'), KeyModifiers::CONTROL), Action::Command(CommandId::FocusFileTree)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('e'), KeyModifiers::CONTROL), Action::Command(CommandId::FocusEditor)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('k'), KeyModifiers::CONTROL), Action::Command(CommandId::Hover)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('g'), KeyModifiers::CONTROL), Action::Command(CommandId::GotoDefinition)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('l'), KeyModifiers::CONTROL), Action::Command(CommandId::GoToLine)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('p'), KeyModifiers::CONTROL), Action::Command(CommandId::OpenFile)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('o'), KeyModifiers::CONTROL), Action::OpenCommandPalette),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('f'), KeyModifiers::CONTROL), Action::Command(CommandId::SearchProject)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('t'), KeyModifiers::CONTROL), Action::Command(CommandId::ToggleTerminal)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('d'), KeyModifiers::CONTROL), Action::Command(CommandId::ToggleDiagnostics)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL), Action::Command(CommandId::CloseTab)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::F(1), KeyModifiers::NONE), Action::Command(CommandId::ShowKeybindings)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::F(12), KeyModifiers::NONE), Action::Command(CommandId::GotoDefinition)),
                Keybinding::new(Context::FileTree, KeyChord::new(KeyCode::Up, KeyModifiers::NONE), Action::FileTreeMoveUp),
                Keybinding::new(Context::FileTree, KeyChord::new(KeyCode::Down, KeyModifiers::NONE), Action::FileTreeMoveDown),
                Keybinding::new(Context::FileTree, KeyChord::new(KeyCode::Right, KeyModifiers::NONE), Action::FileTreeExpand),
                Keybinding::new(Context::FileTree, KeyChord::new(KeyCode::Left, KeyModifiers::NONE), Action::FileTreeCollapse),
                Keybinding::new(Context::FileTree, KeyChord::new(KeyCode::Enter, KeyModifiers::NONE), Action::FileTreeOpenSelected),
                Keybinding::new(Context::FileTree, KeyChord::new(KeyCode::Tab, KeyModifiers::NONE), Action::Command(CommandId::FocusEditor)),
                Keybinding::new(Context::Editor, KeyChord::new(KeyCode::Esc, KeyModifiers::NONE), Action::EditorDismissHover),
                Keybinding::new(Context::Editor, KeyChord::new(KeyCode::Tab, KeyModifiers::NONE), Action::EditorCycleFocus),
                Keybinding::new(Context::Palette, KeyChord::new(KeyCode::Esc, KeyModifiers::NONE), Action::PaletteClose),
                Keybinding::new(Context::Palette, KeyChord::new(KeyCode::Up, KeyModifiers::NONE), Action::PaletteMoveUp),
                Keybinding::new(Context::Palette, KeyChord::new(KeyCode::Down, KeyModifiers::NONE), Action::PaletteMoveDown),
                Keybinding::new(Context::Palette, KeyChord::new(KeyCode::Backspace, KeyModifiers::NONE), Action::PaletteBackspace),
                Keybinding::new(Context::Palette, KeyChord::new(KeyCode::Enter, KeyModifiers::NONE), Action::PaletteSubmit),
                Keybinding::new(Context::Search, KeyChord::new(KeyCode::Esc, KeyModifiers::NONE), Action::SearchClose),
                Keybinding::new(Context::Search, KeyChord::new(KeyCode::Up, KeyModifiers::NONE), Action::SearchMoveUp),
                Keybinding::new(Context::Search, KeyChord::new(KeyCode::Down, KeyModifiers::NONE), Action::SearchMoveDown),
                Keybinding::new(Context::Search, KeyChord::new(KeyCode::Enter, KeyModifiers::NONE), Action::SearchSubmit),
                Keybinding::new(Context::Search, KeyChord::new(KeyCode::Backspace, KeyModifiers::NONE), Action::SearchBackspace),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::Up, KeyModifiers::NONE), Action::TerminalScrollUp),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::Down, KeyModifiers::NONE), Action::TerminalScrollDown),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::Tab, KeyModifiers::NONE), Action::TerminalFocusFileTree),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::Enter, KeyModifiers::NONE), Action::TerminalSendEnter),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::Backspace, KeyModifiers::NONE), Action::TerminalSendBackspace),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::Left, KeyModifiers::NONE), Action::TerminalSendLeft),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::Right, KeyModifiers::NONE), Action::TerminalSendRight),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::Home, KeyModifiers::NONE), Action::TerminalSendHome),
                Keybinding::new(Context::Terminal, KeyChord::new(KeyCode::End, KeyModifiers::NONE), Action::TerminalSendEnd),
                Keybinding::new(Context::Diagnostics, KeyChord::new(KeyCode::Up, KeyModifiers::NONE), Action::DiagnosticsMoveUp),
                Keybinding::new(Context::Diagnostics, KeyChord::new(KeyCode::Down, KeyModifiers::NONE), Action::DiagnosticsMoveDown),
                Keybinding::new(Context::Diagnostics, KeyChord::new(KeyCode::Enter, KeyModifiers::NONE), Action::DiagnosticsSubmit),
                Keybinding::new(Context::Diagnostics, KeyChord::new(KeyCode::Esc, KeyModifiers::NONE), Action::DiagnosticsClose),
                Keybinding::new(Context::Diagnostics, KeyChord::new(KeyCode::Tab, KeyModifiers::NONE), Action::DiagnosticsClose),
                Keybinding::new(Context::Prompt, KeyChord::new(KeyCode::Esc, KeyModifiers::NONE), Action::PromptClose),
                Keybinding::new(Context::Prompt, KeyChord::new(KeyCode::Backspace, KeyModifiers::NONE), Action::PromptBackspace),
                Keybinding::new(Context::Prompt, KeyChord::new(KeyCode::Enter, KeyModifiers::NONE), Action::PromptSubmit),
                Keybinding::new(Context::KeybindingHelp, KeyChord::new(KeyCode::Esc, KeyModifiers::NONE), Action::KeybindingHelpClose),
                Keybinding::new(Context::KeybindingHelp, KeyChord::new(KeyCode::Enter, KeyModifiers::NONE), Action::KeybindingHelpClose),
                Keybinding::new(Context::KeybindingHelp, KeyChord::new(KeyCode::Up, KeyModifiers::NONE), Action::KeybindingHelpMoveUp),
                Keybinding::new(Context::KeybindingHelp, KeyChord::new(KeyCode::Down, KeyModifiers::NONE), Action::KeybindingHelpMoveDown),
                Keybinding::new(Context::KeybindingHelp, KeyChord::new(KeyCode::PageUp, KeyModifiers::NONE), Action::KeybindingHelpPageUp),
                Keybinding::new(Context::KeybindingHelp, KeyChord::new(KeyCode::PageDown, KeyModifiers::NONE), Action::KeybindingHelpPageDown),
            ],
        }
    }
}

impl KeybindingRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn action_for(&self, context: KeyContext, event: KeyEvent) -> Option<EditorAction> {
        let chord = KeyChord::from_event(event);

        self.bindings
            .iter()
            .find(|binding| binding.context == context && binding.chord == chord)
            .map(|binding| binding.action)
    }

    pub fn help_entries(&self) -> Vec<KeybindingHelpEntry> {
        self.bindings
            .iter()
            .map(|binding| KeybindingHelpEntry {
                context: binding.context.label(),
                shortcut: binding.chord.display(),
                description: binding.action.label(),
            })
            .collect()
    }

    pub fn supports_preset(name: &str) -> bool {
        normalize_name(name) == "default"
    }

    pub fn validate_binding_spec(key: &str, action: &str) -> Result<(String, String), String> {
        let chord = parse_key_chord(key)?;
        let normalized_action = normalize_name(action);

        if normalized_action.is_empty() {
            return Err("missing action name".to_string());
        }
        if !supported_action_names().contains(&normalized_action.as_str()) {
            return Err(format!("unsupported action '{action}'"));
        }

        Ok((chord.display(), normalized_action))
    }
}

impl KeyContext {
    pub fn label(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::FileTree => "Files",
            Self::Editor => "Editor",
            Self::Palette => "Palette",
            Self::Search => "Search",
            Self::Terminal => "Terminal",
            Self::Diagnostics => "Diagnostics",
            Self::Prompt => "Prompt",
            Self::KeybindingHelp => "Help",
        }
    }
}

impl EditorAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Command(CommandId::OpenFile) => "Open file",
            Self::Command(CommandId::CreateFile) => "Create file",
            Self::Command(CommandId::EditFile) => "Edit file",
            Self::Command(CommandId::SearchProject) => "Search project",
            Self::Command(CommandId::GoToLine) => "Go to line",
            Self::Command(CommandId::ToggleTerminal) => "Toggle terminal",
            Self::Command(CommandId::GotoDefinition) => "Go to definition",
            Self::Command(CommandId::Hover) => "Show hover",
            Self::Command(CommandId::ToggleDiagnostics) => "Toggle diagnostics",
            Self::Command(CommandId::FocusFileTree) => "Focus file tree",
            Self::Command(CommandId::FocusEditor) => "Focus editor",
            Self::Command(CommandId::Save) => "Save file",
            Self::Command(CommandId::CloseTab) => "Close tab",
            Self::Command(CommandId::ShowKeybindings) => "Show keybindings",
            Self::Command(CommandId::Quit) => "Quit Noir",
            Self::OpenCommandPalette => "Open command palette",
            Self::FocusTerminal => "Focus terminal",
            Self::FocusDiagnostics => "Focus diagnostics",
            Self::NextEditorTab => "Next tab",
            Self::PrevEditorTab => "Previous tab",
            Self::FileTreeMoveUp => "Move up",
            Self::FileTreeMoveDown => "Move down",
            Self::FileTreeExpand => "Expand directory",
            Self::FileTreeCollapse => "Collapse directory",
            Self::FileTreeOpenSelected => "Open selected entry",
            Self::EditorDismissHover => "Close hover",
            Self::EditorCycleFocus => "Cycle focus",
            Self::PaletteClose => "Close palette",
            Self::PaletteMoveUp => "Move selection up",
            Self::PaletteMoveDown => "Move selection down",
            Self::PaletteBackspace => "Delete input",
            Self::PaletteSubmit => "Submit selection",
            Self::SearchClose => "Close search",
            Self::SearchMoveUp => "Move result up",
            Self::SearchMoveDown => "Move result down",
            Self::SearchSubmit => "Open selected result",
            Self::SearchBackspace => "Delete search input",
            Self::TerminalScrollUp => "Scroll terminal up",
            Self::TerminalScrollDown => "Scroll terminal down",
            Self::TerminalFocusFileTree => "Focus file tree",
            Self::TerminalSendEnter => "Send Enter",
            Self::TerminalSendBackspace => "Send Backspace",
            Self::TerminalSendLeft => "Send Left",
            Self::TerminalSendRight => "Send Right",
            Self::TerminalSendHome => "Send Home",
            Self::TerminalSendEnd => "Send End",
            Self::DiagnosticsMoveUp => "Move issue up",
            Self::DiagnosticsMoveDown => "Move issue down",
            Self::DiagnosticsSubmit => "Jump to issue",
            Self::DiagnosticsClose => "Close diagnostics",
            Self::PromptClose => "Close prompt",
            Self::PromptBackspace => "Delete input",
            Self::PromptSubmit => "Submit prompt",
            Self::KeybindingHelpClose => "Close keybindings",
            Self::KeybindingHelpMoveUp => "Scroll up",
            Self::KeybindingHelpMoveDown => "Scroll down",
            Self::KeybindingHelpPageUp => "Page up",
            Self::KeybindingHelpPageDown => "Page down",
        }
    }
}

fn normalize_name(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

fn supported_action_names() -> &'static [&'static str] {
    &[
        "open_file",
        "create_file",
        "edit_file",
        "search_project",
        "go_to_line",
        "toggle_terminal",
        "goto_definition",
        "hover",
        "toggle_diagnostics",
        "focus_file_tree",
        "focus_editor",
        "save",
        "close_tab",
        "show_keybindings",
        "quit",
    ]
}

fn parse_key_chord(input: &str) -> Result<KeyChord, String> {
    let parts: Vec<&str> = input
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();

    if parts.is_empty() {
        return Err("missing key".to_string());
    }

    let mut modifiers = KeyModifiers::NONE;
    for modifier in &parts[..parts.len() - 1] {
        match normalize_name(modifier).as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "option" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            other => return Err(format!("unknown modifier '{other}'")),
        }
    }

    let key_name = normalize_name(parts[parts.len() - 1]);
    let code = match key_name.as_str() {
        "backspace" => KeyCode::Backspace,
        "enter" | "return" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "esc" | "escape" => KeyCode::Esc,
        _ if key_name.len() == 1 => {
            KeyCode::Char(key_name.chars().next().expect("single char key"))
        }
        _ if key_name.starts_with('f') => {
            let number = key_name[1..]
                .parse::<u8>()
                .map_err(|_| format!("unknown key '{}'", parts[parts.len() - 1]))?;
            KeyCode::F(number)
        }
        _ => return Err(format!("unknown key '{}'", parts[parts.len() - 1])),
    };

    Ok(KeyChord::new(code, modifiers))
}
