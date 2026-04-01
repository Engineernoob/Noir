use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeConfig,
    pub editor: EditorConfig,
    pub terminal: TerminalConfig,
    pub plugins: PluginConfig,
    pub keymap: KeymapConfig,
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
    pub fn load() -> Result<Self> {
        Self::load_from_path(&default_config_path())
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(source) => toml::from_str(&source)
                .with_context(|| format!("invalid config at {}", path.display())),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err)
                .with_context(|| format!("failed to read config at {}", path.display())),
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
            name: "noir".to_string(),
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
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            preset: "default".to_string(),
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
