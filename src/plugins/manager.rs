#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use super::manifest::Manifest;

// ── Plugin ────────────────────────────────────────────────────────────────────

/// A discovered plugin: its on-disk location plus its parsed manifest.
#[derive(Debug, Clone)]
pub struct Plugin {
    /// Absolute path to the plugin's own directory.
    pub dir: PathBuf,
    pub manifest: Manifest,
}

impl Plugin {
    /// Absolute path to the plugin entry point.
    pub fn entry_path(&self) -> PathBuf {
        self.dir.join(&self.manifest.entry)
    }

    pub fn is_enabled(&self) -> bool {
        self.manifest.is_enabled()
    }
}

// ── PluginManager ─────────────────────────────────────────────────────────────

/// Holds all discovered plugins.
///
/// Discovery is a separate step (`discover`) so the manager can be constructed
/// cheaply and populated later (or not at all when the directory is absent).
#[derive(Debug, Default)]
pub struct PluginManager {
    plugins: Vec<Plugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan `plugins_dir` for subdirectories that contain a `plugin.toml`.
    ///
    /// Each subdirectory is treated as one plugin.  Directories whose manifest
    /// cannot be parsed emit a warning to `stderr` and are skipped rather than
    /// failing the whole load.  Returns `Ok` even when `plugins_dir` does not
    /// exist (Noir simply has no plugins).
    pub fn discover(&mut self, plugins_dir: &Path) -> Result<()> {
        if !plugins_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(plugins_dir)
            .with_context(|| format!("cannot read plugins dir: {}", plugins_dir.display()))?;

        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }

            let manifest_path = dir.join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }

            match load_manifest(&manifest_path) {
                Ok(manifest) => self.plugins.push(Plugin { dir, manifest }),
                Err(e) => {
                    eprintln!(
                        "[noir] skipping plugin at {}: {e}",
                        dir.display()
                    );
                }
            }
        }

        // Stable order: sort by plugin name so the list is deterministic.
        self.plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));

        Ok(())
    }

    /// All discovered plugins, enabled or not.
    pub fn all(&self) -> &[Plugin] {
        &self.plugins
    }

    /// Only the enabled plugins.
    pub fn enabled(&self) -> impl Iterator<Item = &Plugin> {
        self.plugins.iter().filter(|p| p.is_enabled())
    }

    /// Look up a plugin by name (case-sensitive).
    pub fn find_by_name(&self, name: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.manifest.name == name)
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn load_manifest(path: &Path) -> Result<Manifest> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    Manifest::from_str(&source)
        .with_context(|| format!("invalid manifest at {}", path.display()))
}
