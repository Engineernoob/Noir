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
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('b'), KeyModifiers::CONTROL), Action::Command(CommandId::FocusFileTree)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('e'), KeyModifiers::CONTROL), Action::Command(CommandId::FocusEditor)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('k'), KeyModifiers::CONTROL), Action::Command(CommandId::Hover)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('g'), KeyModifiers::CONTROL), Action::Command(CommandId::GotoDefinition)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('p'), KeyModifiers::CONTROL), Action::Command(CommandId::OpenFile)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('o'), KeyModifiers::CONTROL), Action::OpenCommandPalette),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('f'), KeyModifiers::CONTROL), Action::Command(CommandId::SearchProject)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('t'), KeyModifiers::CONTROL), Action::Command(CommandId::ToggleTerminal)),
                Keybinding::new(Context::Global, KeyChord::new(KeyCode::Char('d'), KeyModifiers::CONTROL), Action::Command(CommandId::ToggleDiagnostics)),
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
}
