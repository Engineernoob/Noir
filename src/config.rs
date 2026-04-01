use std::{
    collections::HashMap,
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{keybindings::KeybindingRegistry, theme::Theme};

const MAX_TAB_WIDTH: usize = 16;
const MAX_SCROLLBACK: usize = 100_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeConfig,
    pub editor: EditorConfig,
    pub terminal: TerminalConfig,
    pub plugins: PluginConfig,
    pub keymap: KeymapConfig,
}

#[derive(Debug, Clone)]
pub struct ConfigLoadReport {
    pub path: PathBuf,
    pub config: Config,
    pub issues: Vec<ConfigIssue>,
}

#[derive(Debug, Clone)]
pub struct ConfigIssue {
    pub level: ConfigIssueLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigIssueLevel {
    Warning,
    Error,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            editor: EditorConfig::default(),
            terminal: TerminalConfig::default(),
            plugins: PluginConfig::default(),
            keymap: KeymapConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> ConfigLoadReport {
        Self::load_from_path(&default_config_path())
    }

    pub fn load_from_path(path: &Path) -> ConfigLoadReport {
        let report = match fs::read_to_string(path) {
            Ok(source) => match toml::from_str::<Config>(&source) {
                Ok(mut config) => {
                    let issues = config.validate();
                    ConfigLoadReport {
                        path: path.to_path_buf(),
                        config,
                        issues,
                    }
                }
                Err(err) => ConfigLoadReport {
                    path: path.to_path_buf(),
                    config: Config::default(),
                    issues: vec![ConfigIssue::error(format!(
                        "Invalid config syntax at {}: {err}. Using defaults.",
                        path.display()
                    ))],
                },
            },
            Err(err) if err.kind() == ErrorKind::NotFound => ConfigLoadReport {
                path: path.to_path_buf(),
                config: Config::default(),
                issues: Vec::new(),
            },
            Err(err) => ConfigLoadReport {
                path: path.to_path_buf(),
                config: Config::default(),
                issues: vec![ConfigIssue::error(format!(
                    "Failed to read config at {}: {err}. Using defaults.",
                    path.display()
                ))],
            },
        };

        report.log_to_stderr();
        report
    }

    fn validate(&mut self) -> Vec<ConfigIssue> {
        let mut issues = Vec::new();

        if !Theme::supports_name(&self.theme.name) {
            issues.push(ConfigIssue::warning(format!(
                "Unknown theme '{}' in config. Falling back to '{}'.",
                self.theme.name,
                Theme::default_name()
            )));
            self.theme.name = Theme::default_name().to_string();
        }

        if self.editor.tab_width == 0 {
            issues.push(ConfigIssue::warning(format!(
                "editor.tab_width cannot be 0. Falling back to {}.",
                EditorConfig::default().tab_width
            )));
            self.editor.tab_width = EditorConfig::default().tab_width;
        } else if self.editor.tab_width > MAX_TAB_WIDTH {
            issues.push(ConfigIssue::warning(format!(
                "editor.tab_width={} is too large. Clamping to {MAX_TAB_WIDTH}.",
                self.editor.tab_width
            )));
            self.editor.tab_width = MAX_TAB_WIDTH;
        }

        if let Some(shell) = &self.terminal.shell {
            if shell.trim().is_empty() {
                issues.push(ConfigIssue::warning(
                    "terminal.shell is empty. Falling back to the environment shell."
                        .to_string(),
                ));
                self.terminal.shell = None;
            }
        }

        if self.terminal.scrollback == 0 {
            issues.push(ConfigIssue::warning(format!(
                "terminal.scrollback cannot be 0. Falling back to {}.",
                TerminalConfig::default().scrollback
            )));
            self.terminal.scrollback = TerminalConfig::default().scrollback;
        } else if self.terminal.scrollback > MAX_SCROLLBACK {
            issues.push(ConfigIssue::warning(format!(
                "terminal.scrollback={} is too large. Clamping to {MAX_SCROLLBACK}.",
                self.terminal.scrollback
            )));
            self.terminal.scrollback = MAX_SCROLLBACK;
        }

        if !KeybindingRegistry::supports_preset(&self.keymap.preset) {
            issues.push(ConfigIssue::warning(format!(
                "Unknown keymap preset '{}'. Falling back to '{}'.",
                self.keymap.preset,
                KeymapConfig::default().preset
            )));
            self.keymap.preset = KeymapConfig::default().preset;
        }

        let mut seen_bindings: HashMap<String, String> = HashMap::new();
        let mut has_custom_bindings = false;

        for binding in &self.keymap.bindings {
            match KeybindingRegistry::validate_binding_spec(&binding.key, &binding.action) {
                Ok((shortcut, action)) => {
                    has_custom_bindings = true;
                    if let Some(previous) = seen_bindings.insert(shortcut.clone(), action.clone()) {
                        issues.push(ConfigIssue::warning(format!(
                            "Duplicate keybinding '{}' in config: '{}' and '{}'.",
                            shortcut, previous, action
                        )));
                    }
                }
                Err(err) => issues.push(ConfigIssue::error(format!(
                    "Invalid keybinding '{} -> {}': {}.",
                    binding.key, binding.action, err
                ))),
            }
        }

        if has_custom_bindings {
            issues.push(ConfigIssue::warning(
                "Custom keybindings are validated but not applied yet."
                    .to_string(),
            ));
        }

        issues
    }
}

impl ConfigLoadReport {
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.level == ConfigIssueLevel::Warning)
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.level == ConfigIssueLevel::Error)
            .count()
    }

    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }

    pub fn summary(&self) -> Option<String> {
        if !self.has_issues() {
            return None;
        }

        let warnings = self.warning_count();
        let errors = self.error_count();

        Some(match (errors, warnings) {
            (0, w) => format!("config: {w} warning{}", if w == 1 { "" } else { "s" }),
            (e, 0) => format!("config: {e} error{}", if e == 1 { "" } else { "s" }),
            (e, w) => format!(
                "config: {e} error{}, {w} warning{}",
                if e == 1 { "" } else { "s" },
                if w == 1 { "" } else { "s" }
            ),
        })
    }

    pub fn log_to_stderr(&self) {
        for issue in &self.issues {
            eprintln!(
                "[noir][config][{}] {}",
                issue.level.label(),
                issue.message
            );
        }
    }
}

impl ConfigIssue {
    fn warning(message: String) -> Self {
        Self {
            level: ConfigIssueLevel::Warning,
            message,
        }
    }

    fn error(message: String) -> Self {
        Self {
            level: ConfigIssueLevel::Error,
            message,
        }
    }
}

impl ConfigIssueLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub name: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: Theme::default_name().to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub line_numbers: bool,
    pub tab_width: usize,
    pub soft_tabs: bool,
    pub soft_wrap: bool,
    pub show_status_bar: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            line_numbers: true,
            tab_width: 4,
            soft_tabs: true,
            soft_wrap: false,
            show_status_bar: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    pub visible: bool,
    pub shell: Option<String>,
    pub scrollback: usize,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            visible: true,
            shell: None,
            scrollback: 5_000,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    pub enabled: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct KeymapConfig {
    pub preset: String,
    pub bindings: Vec<UserKeybindingConfig>,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            preset: "default".to_string(),
            bindings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UserKeybindingConfig {
    pub key: String,
    pub action: String,
}

impl Default for UserKeybindingConfig {
    fn default() -> Self {
        Self {
            key: String::new(),
            action: String::new(),
        }
    }
}

pub fn default_config_path() -> PathBuf {
    if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home).join("noir").join("config.toml");
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("noir")
            .join("config.toml");
    }

    PathBuf::from(".config").join("noir").join("config.toml")
}
