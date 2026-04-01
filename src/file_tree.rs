use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

const IGNORED: &[&str] = &[".git", "target", "node_modules", ".idea", ".vscode"];

// ── Internal tree model ──────────────────────────────────────────────────────

struct TreeNode {
    name: String,
    path: PathBuf,
    kind: NodeKind,
}

enum NodeKind {
    File,
    Dir {
        children: Vec<TreeNode>,
        expanded: bool,
    },
}

// ── Public flat view (used by the UI) ────────────────────────────────────────

pub struct VisibleEntry {
    pub full_path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
}

// ── FileTree ─────────────────────────────────────────────────────────────────

pub struct FileTree {
    roots: Vec<TreeNode>,
    /// Current visible/navigable rows, rebuilt on every expand/collapse.
    flat: Vec<VisibleEntry>,
    selected: usize,
    /// All file paths (display, full) — used by the command palette, independent
    /// of which directories are currently expanded.
    all_files: Vec<(String, PathBuf)>,
}

impl FileTree {
    pub fn new(root: &Path) -> Result<Self> {
        let mut all_files = Vec::new();
        let roots = build_children(root, root, &mut all_files);
        all_files.sort_by(|a, b| a.0.cmp(&b.0));

        let mut flat = Vec::new();
        flatten_visible(&roots, 0, &mut flat);

        Ok(Self {
            roots,
            flat,
            selected: 0,
            all_files,
        })
    }

    pub fn reload(&mut self, root: &Path) -> Result<()> {
        let selected_path = self.flat.get(self.selected).map(|entry| entry.full_path.clone());
        let mut rebuilt = Self::new(root)?;

        if let Some(path) = selected_path {
            rebuilt.select_path(&path);
        }

        *self = rebuilt;
        Ok(())
    }

    // ── Navigation ──────────────────────────────────────────────────────────

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.flat.len() {
            self.selected += 1;
        }
    }

    /// Expand the selected directory. No-op if already expanded or not a dir.
    pub fn expand_selected(&mut self) {
        let Some(entry) = self.flat.get(self.selected) else { return };
        if entry.is_dir && !entry.expanded {
            let path = entry.full_path.clone();
            set_expanded(&mut self.roots, &path, true);
            self.rebuild_flat();
        }
    }

    /// Collapse the selected directory if expanded; otherwise jump to the
    /// nearest parent directory entry in the flat list.
    pub fn collapse_selected(&mut self) {
        let Some(entry) = self.flat.get(self.selected) else { return };
        if entry.is_dir && entry.expanded {
            let path = entry.full_path.clone();
            set_expanded(&mut self.roots, &path, false);
            self.rebuild_flat();
            self.clamp_selected();
        } else {
            self.select_parent();
        }
    }

    /// Toggle expand/collapse on the selected directory. No-op on files.
    pub fn toggle_expand(&mut self) {
        let Some(entry) = self.flat.get(self.selected) else { return };
        if entry.is_dir {
            let (path, currently) = (entry.full_path.clone(), entry.expanded);
            set_expanded(&mut self.roots, &path, !currently);
            self.rebuild_flat();
            self.clamp_selected();
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    pub fn entries(&self) -> &[VisibleEntry] {
        &self.flat
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_is_dir(&self) -> bool {
        self.flat.get(self.selected).map_or(false, |e| e.is_dir)
    }

    /// Returns the full path of the selected entry only if it is a file.
    pub fn selected_path(&self) -> Option<&PathBuf> {
        let e = self.flat.get(self.selected)?;
        if e.is_dir { None } else { Some(&e.full_path) }
    }

    /// First file from the alphabetically-sorted all-files list. Used to
    /// pre-open a file on startup when the initial selection lands on a dir.
    pub fn first_file_path(&self) -> Option<&PathBuf> {
        self.all_files.first().map(|(_, p)| p)
    }

    // ── Palette support ──────────────────────────────────────────────────────

    pub fn all_display_paths(&self) -> Vec<String> {
        self.all_files.iter().map(|(d, _)| d.clone()).collect()
    }

    pub fn all_file_paths(&self) -> &[(String, PathBuf)] {
        &self.all_files
    }

    pub fn find_full_path_by_display(&self, display: &str) -> Option<&PathBuf> {
        self.all_files
            .iter()
            .find(|(d, _)| d == display)
            .map(|(_, p)| p)
    }

    // ── Private ──────────────────────────────────────────────────────────────

    /// Move selection to the nearest ancestor directory in the flat list.
    fn select_parent(&mut self) {
        let depth = match self.flat.get(self.selected) {
            Some(e) if e.depth > 0 => e.depth,
            _ => return,
        };
        let parent_depth = depth - 1;
        for i in (0..self.selected).rev() {
            if self.flat[i].is_dir && self.flat[i].depth == parent_depth {
                self.selected = i;
                return;
            }
        }
    }

    fn rebuild_flat(&mut self) {
        self.flat.clear();
        flatten_visible(&self.roots, 0, &mut self.flat);
    }

    fn clamp_selected(&mut self) {
        if !self.flat.is_empty() && self.selected >= self.flat.len() {
            self.selected = self.flat.len() - 1;
        }
    }

    fn select_path(&mut self, path: &Path) {
        if let Some(index) = self.flat.iter().position(|entry| entry.full_path == path) {
            self.selected = index;
        }
    }
}

// ── Free functions ───────────────────────────────────────────────────────────

/// Recursively read `dir`, skipping ignored names.
/// Fills `all_files` with every file found, regardless of tree depth.
/// Returns the sorted child `TreeNode`s for `dir`.
fn build_children(
    dir: &Path,
    root: &Path,
    all_files: &mut Vec<(String, PathBuf)>,
) -> Vec<TreeNode> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut raw: Vec<(String, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if should_ignore(&name) {
                return None;
            }
            Some((name, e.path()))
        })
        .collect();

    // Directories before files; each group sorted case-insensitively.
    raw.sort_by(|a, b| {
        let a_dir = a.1.is_dir();
        let b_dir = b.1.is_dir();
        match (a_dir, b_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
        }
    });

    let mut nodes = Vec::new();
    for (name, path) in raw {
        if path.is_dir() {
            let children = build_children(&path, root, all_files);
            nodes.push(TreeNode {
                name,
                path,
                kind: NodeKind::Dir {
                    children,
                    expanded: false,
                },
            });
        } else if path.is_file() {
            let display = path
                .strip_prefix(root)
                .ok()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| path.display().to_string());
            all_files.push((display, path.clone()));
            nodes.push(TreeNode {
                name,
                path,
                kind: NodeKind::File,
            });
        }
    }

    nodes
}

/// Append all currently-visible entries to `out`, honouring `expanded` state.
fn flatten_visible(nodes: &[TreeNode], depth: usize, out: &mut Vec<VisibleEntry>) {
    for node in nodes {
        match &node.kind {
            NodeKind::File => {
                out.push(VisibleEntry {
                    full_path: node.path.clone(),
                    name: node.name.clone(),
                    depth,
                    is_dir: false,
                    expanded: false,
                });
            }
            NodeKind::Dir { children, expanded } => {
                out.push(VisibleEntry {
                    full_path: node.path.clone(),
                    name: node.name.clone(),
                    depth,
                    is_dir: true,
                    expanded: *expanded,
                });
                if *expanded {
                    flatten_visible(children, depth + 1, out);
                }
            }
        }
    }
}

/// Set the `expanded` flag on the directory whose path equals `target`.
/// Searches through all nodes (not just currently-visible ones) so that
/// nodes in unexpanded subtrees can still be located.
fn set_expanded(nodes: &mut Vec<TreeNode>, target: &Path, new_expanded: bool) -> bool {
    for node in nodes.iter_mut() {
        if node.path == target {
            if let NodeKind::Dir { expanded, .. } = &mut node.kind {
                *expanded = new_expanded;
            }
            return true;
        }
        if let NodeKind::Dir { children, .. } = &mut node.kind {
            if set_expanded(children, target, new_expanded) {
                return true;
            }
        }
    }
    false
}

fn should_ignore(name: &str) -> bool {
    IGNORED.contains(&name)
}
