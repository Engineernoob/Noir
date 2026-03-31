use std::{collections::HashMap, path::Path};

// ── LSP server configuration ──────────────────────────────────────────────────

/// Configuration for spawning an LSP server process.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub command: String,
    pub args: Vec<String>,
}

impl ServerConfig {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
        }
    }

    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }
}

// ── Syntax highlighting ───────────────────────────────────────────────────────

/// Function that returns a tree-sitter `Language` and its highlight query.
///
/// Using a function pointer (not a closure) keeps `LanguageDefinition: Clone`
/// without any heap allocation, and avoids storing `tree_sitter::Language`
/// values in a static table.
pub type HighlightFn = fn() -> (tree_sitter::Language, &'static str);

// ── Language definition ───────────────────────────────────────────────────────

/// Everything Noir needs to know about a language.
#[derive(Clone)]
pub struct LanguageDefinition {
    /// Sent as `languageId` in `textDocument/didOpen` (e.g. `"rust"`, `"python"`).
    pub id: &'static str,
    /// LSP server to launch. `None` means no LSP support for this language.
    pub lsp: Option<ServerConfig>,
    /// Tree-sitter grammar + highlight query. `None` means plain-text rendering.
    pub highlight: Option<HighlightFn>,
}

// ── Language registry ─────────────────────────────────────────────────────────

/// Maps file extensions (without a leading dot) to [`LanguageDefinition`]s.
///
/// # Example
/// ```
/// let registry = LanguageRegistry::empty()
///     .register(&["rs"], LanguageDefinition {
///         id: "rust",
///         lsp: Some(ServerConfig::new("rust-analyzer")),
///         highlight: Some(|| (tree_sitter_rust::language(), MY_RUST_QUERY)),
///     });
/// ```
pub struct LanguageRegistry {
    entries: HashMap<&'static str, LanguageDefinition>,
}

impl LanguageRegistry {
    pub fn empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register `def` for all of the listed `extensions`.
    pub fn register(mut self, extensions: &[&'static str], def: LanguageDefinition) -> Self {
        for &ext in extensions {
            self.entries.insert(ext, def.clone());
        }
        self
    }

    /// Language ID for the file at `path`, or `"plaintext"` if unrecognised.
    pub fn language_id_for_path(&self, path: &Path) -> &'static str {
        self.get(path).map(|d| d.id).unwrap_or("plaintext")
    }

    /// LSP server config for the file at `path`, if one is registered.
    pub fn lsp_for_path(&self, path: &Path) -> Option<&ServerConfig> {
        self.get(path)?.lsp.as_ref()
    }

    /// Highlight function for the file at `path`, if a grammar is available.
    pub fn highlight_for_path(&self, path: &Path) -> Option<HighlightFn> {
        self.get(path)?.highlight
    }

    fn get(&self, path: &Path) -> Option<&LanguageDefinition> {
        let ext = path.extension()?.to_str()?;
        self.entries.get(ext)
    }
}

// ── Built-in highlight queries ────────────────────────────────────────────────

// Capture names used here map directly to the token kinds in `syntax.rs`:
// @string, @comment, @type, @keyword, @variable.

const RUST_HIGHLIGHTS: &str = r#"
    (string_literal) @string
    (char_literal) @string
    (line_comment) @comment
    (block_comment) @comment
    (type_identifier) @type
    (primitive_type) @type
    (identifier) @variable
"#;

// ── Default registry ──────────────────────────────────────────────────────────

impl Default for LanguageRegistry {
    /// Pre-configured registry. Registers LSP servers for common languages and
    /// tree-sitter highlighting for languages whose grammars are compiled in.
    ///
    /// | Extension(s)         | Language ID  | LSP server                       | Highlighting  |
    /// |----------------------|--------------|----------------------------------|---------------|
    /// | `.rs`                | `rust`       | `rust-analyzer`                  | tree-sitter   |
    /// | `.ts` `.tsx`         | `typescript` | `typescript-language-server`     | —             |
    /// | `.js` `.jsx` `.mjs`  | `javascript` | `typescript-language-server`     | —             |
    /// | `.py`                | `python`     | `pyright-langserver`             | —             |
    ///
    /// Call [`register`](Self::register) to add or override entries.
    fn default() -> Self {
        Self::empty()
            .register(
                &["rs"],
                LanguageDefinition {
                    id: "rust",
                    lsp: Some(ServerConfig::new("rust-analyzer")),
                    highlight: Some(|| (tree_sitter_rust::language(), RUST_HIGHLIGHTS)),
                },
            )
            .register(
                &["ts", "tsx"],
                LanguageDefinition {
                    id: "typescript",
                    lsp: Some(
                        ServerConfig::new("typescript-language-server")
                            .with_args(["--stdio"]),
                    ),
                    highlight: None,
                },
            )
            .register(
                &["js", "jsx", "mjs"],
                LanguageDefinition {
                    id: "javascript",
                    lsp: Some(
                        ServerConfig::new("typescript-language-server")
                            .with_args(["--stdio"]),
                    ),
                    highlight: None,
                },
            )
            .register(
                &["py"],
                LanguageDefinition {
                    id: "python",
                    lsp: Some(
                        ServerConfig::new("pyright-langserver").with_args(["--stdio"]),
                    ),
                    highlight: None,
                },
            )
    }
}
