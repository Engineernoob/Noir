# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**Noir** is a keyboard-first, terminal-based IDE written in Rust. It provides a TUI with a file tree, multi-tab editor, embedded PTY shell, fuzzy file search palette, and LSP integration (rust-analyzer).

## Commands

```bash
cargo build              # Build
cargo run -- .           # Run in current directory
cargo run -- /path       # Run in specific path
cargo check              # Type-check without building
cargo clippy             # Lint
cargo test               # Run tests (none exist yet)
```

## Architecture

The app follows an event loop pattern: `main.rs` initializes the terminal, creates `App`, then continuously polls for input events (50ms interval), calls `app.tick()` to drain async events, and renders via `ui::draw()`.

### Core Data Flow

```
main.rs (event loop)
  └─ App (src/app.rs) — central state coordinator
       ├─ Editor (src/editor.rs) — rope-based multi-tab buffers
       ├─ FileTree (src/file_tree.rs) — walkdir-based project navigation
       ├─ TerminalPane (src/terminal.rs) — PTY shell via portable-pty
       ├─ CommandPalette (src/palette.rs) — fuzzy file search (fuzzy-matcher)
       └─ LspClient (src/lsp/) — rust-analyzer via stdio JSON-RPC
```

### Concurrency Model

Two background threads communicate via `mpsc` channels back to the main thread:
- **LSP transport thread** (`src/lsp/transport.rs`): reads rust-analyzer stdout, parses Content-Length framed JSON-RPC, sends parsed messages to `LspClient`
- **PTY reader thread** (`src/terminal.rs`): reads PTY output, appends to scrollback buffer

`App::tick()` is called every frame to drain both channels and update app state (diagnostics, hover info, terminal output).

### LSP State Machine

`LspClient` has explicit states: `Created → Initializing → Ready → ShutdownRequested → Exited`. Messages sent before the server is `Ready` are queued and replayed once initialization completes. The client tracks open documents and versions to send correct `DidChange` notifications.

### Focus and Keybindings

`App` tracks `focus: FocusPane` (FileTree, Editor, Palette, Terminal). Key events are routed to the focused pane's handler. Global bindings in `App::handle_key_event()`:

| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit |
| `Ctrl+S` | Save current file |
| `Ctrl+B` / `Ctrl+E` | Focus file tree / editor |
| `Ctrl+P` | Toggle file palette |
| `Ctrl+T` | Toggle terminal pane |
| `Ctrl+K` | Request LSP hover |
| `Alt+,` / `Alt+.` | Previous / next editor tab |
| `Tab` | Cycle pane focus |

### UI Layout

Rendered with `ratatui`. Layout is computed dynamically each frame:
- Outer vertical split: tab bar → content area → status bar
- Content splits horizontally: file tree (30%) / editor (70%)
- When terminal is visible: content area splits vertically 70/30
- Focused pane gets a yellow border
- Hover info and palette render as centered modal overlays

### Syntax Highlighting

`src/syntax.rs` uses tree-sitter to parse Rust source and return byte-range spans with token types. `ui.rs` maps token types to `ratatui` `Style` colors via `token_style()`. Only Rust is supported currently.

### File Tree Exclusions

`src/file_tree.rs` skips: `.git`, `target`, `node_modules`, `.idea`, `.vscode`.

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | TUI rendering and terminal control |
| `ropey` | Efficient rope-based text buffers |
| `portable-pty` | Cross-platform PTY for the terminal pane |
| `tree-sitter` + `tree-sitter-rust` | Syntax highlighting |
| `lsp-types` + `serde_json` | LSP protocol types and JSON-RPC |
| `fuzzy-matcher` | Fuzzy search in the command palette |
| `walkdir` | Recursive file tree traversal |
