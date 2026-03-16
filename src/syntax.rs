use tree_sitter::{Parser, Query, QueryCursor};

pub struct SyntaxHighlighter {
    parser: Parser,
    query: Query,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();

        parser
            .set_language(&language)
            .expect("failed to load Rust grammar for tree-sitter");

        let query = Query::new(
            &language,
            r#"
            (string_literal) @string
            (char_literal) @string
            (line_comment) @comment
            (block_comment) @comment
            (identifier) @variable
            (type_identifier) @type
            (primitive_type) @type
            "#,
        )
        .expect("failed to build tree-sitter highlight query");

        Self { parser, query }
    }

    pub fn highlight(&mut self, source: &str) -> Vec<(usize, usize, &'static str)> {
        let Some(tree) = self.parser.parse(source, None) else {
            return Vec::new();
        };

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.query, tree.root_node(), source.as_bytes());

        let mut tokens = Vec::new();

        for m in matches {
            for capture in m.captures {
                let node = capture.node;
                let kind = match capture.index {
                    0 | 1 => "string",
                    2 | 3 => "comment",
                    4 => "variable",
                    5 | 6 => "type",
                    _ => "normal",
                };

                tokens.push((node.start_byte(), node.end_byte(), kind));
            }
        }

        tokens
    }
}
