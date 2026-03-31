use tree_sitter::{Parser, Query, QueryCursor};

use crate::languages::HighlightFn;

// ── Active highlighter ────────────────────────────────────────────────────────

struct Active {
    parser: Parser,
    query: Query,
}

// ── SyntaxHighlighter ─────────────────────────────────────────────────────────

/// Language-agnostic syntax highlighter backed by tree-sitter.
///
/// Start with no language (`new()`), then call [`set_language`](Self::set_language)
/// whenever the active file changes. Returns an empty token list for languages
/// with no grammar registered.
#[derive(Default)]
pub struct SyntaxHighlighter {
    active: Option<Active>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Switch to the grammar returned by `highlight_fn`, or clear highlighting
    /// if `None` is passed (e.g. for unknown file types).
    pub fn set_language(&mut self, highlight_fn: Option<HighlightFn>) {
        self.active = highlight_fn.and_then(|f| {
            let (lang, query_src) = f();
            let mut parser = Parser::new();
            parser.set_language(&lang).ok()?;
            let query = Query::new(&lang, query_src).ok()?;
            Some(Active { parser, query })
        });
    }

    /// Return byte-range token spans for `source`.
    ///
    /// Each entry is `(start_byte, end_byte, kind)` where `kind` is one of
    /// `"string"`, `"comment"`, `"type"`, `"keyword"`, `"variable"`, or
    /// `"normal"`.  Returns an empty `Vec` when no grammar is active.
    pub fn highlight(&mut self, source: &str) -> Vec<(usize, usize, &'static str)> {
        let Some(active) = &mut self.active else {
            return Vec::new();
        };

        let Some(tree) = active.parser.parse(source, None) else {
            return Vec::new();
        };

        // Collect names upfront so we can borrow `active.query` again below.
        let capture_names: Vec<&str> = active.query.capture_names().to_vec();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&active.query, tree.root_node(), source.as_bytes());

        let mut tokens = Vec::new();
        for m in matches {
            for cap in m.captures {
                let name = capture_names
                    .get(cap.index as usize)
                    .copied()
                    .unwrap_or("");
                tokens.push((
                    cap.node.start_byte(),
                    cap.node.end_byte(),
                    capture_name_to_kind(name),
                ));
            }
        }
        tokens
    }
}

/// Map a tree-sitter capture name to the static token-kind string used by
/// the renderer.  Unknown capture names fall through to `"normal"`.
fn capture_name_to_kind(name: &str) -> &'static str {
    match name {
        "string" => "string",
        "comment" => "comment",
        "type" => "type",
        "keyword" => "keyword",
        "variable" => "variable",
        _ => "normal",
    }
}
