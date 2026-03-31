use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

// ── CommandId ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    OpenFile,
    SearchProject,
    ToggleTerminal,
    GotoDefinition,
    Hover,
    ToggleDiagnostics,
    FocusFileTree,
    FocusEditor,
    Save,
    Quit,
}

// ── Command ───────────────────────────────────────────────────────────────────

pub struct Command {
    pub id: CommandId,
    pub name: &'static str,
    pub description: &'static str,
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
                    id: CommandId::SearchProject,
                    name: "Search Project",
                    description: "Search for text across all project files",
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
                    id: CommandId::Quit,
                    name: "Quit",
                    description: "Quit Noir",
                },
            ],
        }
    }

    /// Fuzzy-filter commands by `query`, returning up to 20 ranked results.
    pub fn fuzzy_filter(&self, query: &str) -> Vec<&Command> {
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, &Command)> = self
            .commands
            .iter()
            .filter_map(|cmd| {
                if query.is_empty() {
                    Some((0, cmd))
                } else {
                    matcher.fuzzy_match(cmd.name, query).map(|s| (s, cmd))
                }
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, cmd)| cmd).take(20).collect()
    }

    pub fn find_by_name(&self, name: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.name == name)
    }
}
