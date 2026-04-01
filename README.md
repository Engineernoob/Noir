# Noir

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/built%20with-Rust-orange)

Noir is a keyboard-first, terminal-native editor written in Rust.

It is aimed at the space between Neovim and heavier GUI editors: fast to start, easier to understand, easier to configure, and still capable enough to grow into a general-purpose editor.

## Goals

- Stay in the terminal
- Keep the core simple and maintainable
- Prefer good defaults over deep setup
- Make features discoverable instead of hidden behind editor folklore

## Current Capabilities

- Terminal UI built with `ratatui` and `crossterm`
- Project file tree with nested directories
- Multi-tab text editing with save support
- Command palette for built-in and plugin commands
- Project-wide text search
- Embedded terminal pane
- Status bar, cursor position, and line numbers
- Theme selection from config
- Config loading and validation with fallback behavior
- Keybinding registry foundation plus keybinding help
- Lightweight process-based plugin system
- JSON stdio plugin registration and command execution
- Basic LSP-backed hover, go-to-definition, and diagnostics

## Current File Workflow

Noir currently supports:

- Open file
- Create file
- Edit file by path
- Go to line or `line:column`
- Close tab

These are available through the command palette, and several are also bound to shortcuts.

## Controls

Global:

- `Ctrl+O` open command palette
- `Ctrl+P` open file search
- `Ctrl+N` create file
- `Ctrl+L` go to line
- `Ctrl+F` search project text
- `Ctrl+S` save file
- `Ctrl+W` close tab
- `Ctrl+T` toggle terminal
- `Ctrl+D` toggle diagnostics
- `Ctrl+B` focus file tree
- `Ctrl+E` focus editor
- `Ctrl+K` hover
- `Ctrl+G` go to definition
- `F1` show keybindings
- `F12` go to definition
- `Ctrl+Q` quit

Focus and navigation:

- `Alt+1` focus file tree
- `Alt+2` focus editor
- `Alt+3` focus terminal
- `Alt+4` focus diagnostics
- `Alt+,` previous tab
- `Alt+.` next tab

File tree:

- `Up` / `Down` move
- `Right` expand directory
- `Left` collapse directory
- `Enter` open selected file
- `Tab` move focus to editor

Palette and prompts:

- `Esc` close
- `Enter` submit
- `Backspace` delete input
- `Up` / `Down` move selection
- In file search, type `>` first to switch into command mode

Terminal:

- `Up` / `Down` scroll terminal output
- `Enter`, `Backspace`, `Left`, `Right`, `Home`, `End` forward to the PTY
- `Tab` move focus back to file tree

Diagnostics:

- `Up` / `Down` move
- `Enter` jump to issue
- `Esc` or `Tab` close

## Configuration

Noir loads config from:

- `$XDG_CONFIG_HOME/noir/config.toml`
- or `~/.config/noir/config.toml`

Missing config falls back to defaults.

Invalid config does not unnecessarily abort startup. Noir falls back where reasonable, logs issues to stderr, surfaces a startup summary in the status bar, and writes config warnings/errors into the terminal log area.

Example:

```toml
[theme]
name = "daylight"

[editor]
line_numbers = true
tab_width = 4
soft_tabs = true
soft_wrap = false
show_status_bar = true

[terminal]
visible = true
shell = "/bin/zsh"
scrollback = 8000

[plugins]
enabled = true

[keymap]
preset = "default"

[[keymap.bindings]]
key = "Ctrl+S"
action = "save"
```

Notes:

- Supported built-in themes are `noir` and `daylight`
- Custom keybindings are validated today, but remapping is not applied yet

## Plugins

Noir's plugin system is intentionally simple:

- Plugins are external processes, not dynamic libraries
- Each plugin is discovered from a manifest
- Plugins communicate with Noir over stdin/stdout using line-delimited JSON
- Plugins can register commands
- Registered plugin commands appear in the command palette
- Noir can send command execution requests with editor context
- Plugin output is shown in the terminal pane

The protocol is intentionally minimal and debuggable rather than full JSON-RPC.

## Running Noir

Requirements:

- Rust
- Cargo

Run the editor from the crate directory:

```bash
cargo run -- .
```

Open another project:

```bash
cargo run -- /path/to/project
```

## Project Direction

The current architecture is aiming for:

- better defaults than Neovim
- less indirection than traditional plugin-heavy editors
- enough structure to grow without turning the core into a framework

Near-term work still includes deeper LSP support, more editor actions, and eventually user-applied key remapping on top of the registry that now exists.
