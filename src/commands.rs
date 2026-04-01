use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

// ── CommandId ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    OpenFile,
    CreateFile,
    EditFile,
    SearchProject,
    GoToLine,
    ToggleTerminal,
    GotoDefinition,
    Hover,
    ToggleDiagnostics,
    FocusFileTree,
    FocusEditor,
    Save,
    CloseTab,
    ShowKeybindings,
    Quit,
}

// ── Command ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Command {
    pub id: CommandId,
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteCommandTarget {
    BuiltIn(CommandId),
    Plugin {
        plugin_name: String,
        command_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteCommandEntry {
    pub title: String,
    pub description: String,
    pub target: PaletteCommandTarget,
}

// ── CommandRegistry ───────────────────────────────────────────────────────────

pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: vec![
                Command {
                    id: CommandId::OpenFile,
                    name: "Open File",
                    description: "Open a file from the project tree",
                },
                Command {
                    id: CommandId::CreateFile,
                    name: "Create File",
                    description: "Create a new file and open it in the editor",
                },
                Command {
                    id: CommandId::EditFile,
                    name: "Edit File",
                    description: "Open an existing file by path",
                },
                Command {
                    id: CommandId::SearchProject,
                    name: "Search Project",
                    description: "Search for text across all project files",
                },
                Command {
                    id: CommandId::GoToLine,
                    name: "Go to Line",
                    description: "Jump to a specific line or line:column",
                },
                Command {
                    id: CommandId::ToggleTerminal,
                    name: "Toggle Terminal",
                    description: "Show or hide the embedded terminal",
                },
                Command {
                    id: CommandId::GotoDefinition,
                    name: "Go to Definition",
                    description: "Jump to the definition of the symbol under the cursor",
                },
                Command {
                    id: CommandId::Hover,
                    name: "Hover",
                    description: "Show hover information for the symbol under the cursor",
                },
                Command {
                    id: CommandId::ToggleDiagnostics,
                    name: "Toggle Diagnostics",
                    description: "Show or hide the diagnostics pane",
                },
                Command {
                    id: CommandId::FocusFileTree,
                    name: "Focus File Tree",
                    description: "Move focus to the file tree",
                },
                Command {
                    id: CommandId::FocusEditor,
                    name: "Focus Editor",
                    description: "Move focus to the editor",
                },
                Command {
                    id: CommandId::Save,
                    name: "Save File",
                    description: "Save the current file",
                },
                Command {
                    id: CommandId::CloseTab,
                    name: "Close Tab",
                    description: "Close the current editor tab",
                },
                Command {
                    id: CommandId::ShowKeybindings,
                    name: "Show Keybindings",
                    description: "List available keyboard shortcuts",
                },
                Command {
                    id: CommandId::Quit,
                    name: "Quit",
                    description: "Quit Noir",
                },
            ],
        }
    }

    pub fn built_in_palette_commands(&self) -> Vec<PaletteCommandEntry> {
        self.commands
            .iter()
            .map(|cmd| PaletteCommandEntry {
                title: cmd.name.to_string(),
                description: cmd.description.to_string(),
                target: PaletteCommandTarget::BuiltIn(cmd.id),
            })
            .collect()
    }

    /// Fuzzy-filter command entries by `query`, returning up to 20 ranked results.
    pub fn fuzzy_filter(
        &self,
        query: &str,
        extra_commands: impl IntoIterator<Item = PaletteCommandEntry>,
    ) -> Vec<PaletteCommandEntry> {
        let matcher = SkimMatcherV2::default();

        let mut candidates = self.built_in_palette_commands();
        candidates.extend(extra_commands);

        let mut scored: Vec<(i64, PaletteCommandEntry)> = candidates
            .into_iter()
            .filter_map(|cmd| {
                if query.is_empty() {
                    Some((0, cmd))
                } else {
                    matcher.fuzzy_match(&cmd.title, query).map(|s| (s, cmd))
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.title.cmp(&b.1.title)));

        scored.into_iter().map(|(_, cmd)| cmd).take(20).collect()
    }
}
