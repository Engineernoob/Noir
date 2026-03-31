#![allow(dead_code)]

use serde::Deserialize;

/// Raw TOML representation of `plugin.toml`.
///
/// All fields that are optional in the spec use `Option<T>` so that a minimal
/// manifest (name + version + entry + capabilities) is valid.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    /// Relative path to the plugin entry point inside the plugin directory.
    pub entry: String,
    pub description: Option<String>,
    /// Declared capabilities, e.g. `["on_save", "on_key"]`.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// When absent the plugin is treated as enabled.
    pub enabled: Option<bool>,
}

impl Manifest {
    /// Parse a `plugin.toml` file from its raw text content.
    pub fn from_str(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }

    /// Returns `true` unless `enabled` is explicitly set to `false`.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let src = r#"
            name = "hello"
            version = "0.1.0"
            entry = "main.lua"
            capabilities = []
        "#;
        let m = Manifest::from_str(src).unwrap();
        assert_eq!(m.name, "hello");
        assert!(m.is_enabled());
    }

    #[test]
    fn respects_enabled_false() {
        let src = r#"
            name = "hello"
            version = "0.1.0"
            entry = "main.lua"
            capabilities = []
            enabled = false
        "#;
        let m = Manifest::from_str(src).unwrap();
        assert!(!m.is_enabled());
    }
}
