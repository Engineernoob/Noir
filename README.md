# Noir

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/built%20with-Rust-orange)

**Noir** is a keyboard-first **terminal-based IDE written in Rust**.

Built with `ratatui` and `crossterm`, Noir aims to provide a fast, minimal, and focused development environment directly inside the terminal.

Instead of replicating full GUI IDE complexity, Noir focuses on the core developer workflow:

- navigate projects
- edit files
- run commands
- stay in the terminal

---

## ✨ Features (Current)

- Terminal UI powered by **ratatui**
- Project **file explorer**
- Basic **text editor**
- **Rope-based text buffer** using `ropey`
- **Pane focus system**
- **Open and save files**
- Status bar with cursor position and file state

---

## 🚧 Planned Features

The roadmap for Noir includes:

- Multiple editor **tabs**
- **Syntax highlighting** (tree-sitter)
- **Command palette**
- **Project-wide search**
- **Embedded terminal pane**
- **LSP support**
- **Git integration**
- **Configurable themes**
- **Plugin system**

---

## 🖥 Layout

Noir uses a simple IDE layout:

```text
┌────────────────────────────────────────────────────────────────────────┐
│ Noir — Terminal IDE                                                    │
├───────────────┬────────────────────────────────────────────────────────┤
│               │                                                        │
│  Explorer     │  src/main.rs                                          │
│               │                                                        │
│ ▸ src/        │  1  fn main() {                                        │
│   ├ main.rs   │  2      println!("Hello, Noir");                       │
│   ├ app.rs    │  3  }                                                  │
│   ├ ui.rs     │                                                        │
│               │                                                        │
│               │                                                        │
│               │                                                        │
├───────────────┴────────────────────────────────────────────────────────┤
│ EDITOR | root:noir | src/main.rs | Ln 2, Col 12 | OK                   │
└────────────────────────────────────────────────────────────────────────┘
```

---

## ⌨️ Controls

| Key         | Action           |
| ----------- | ---------------- |
| `Ctrl+Q`    | Quit Noir        |
| `Ctrl+S`    | Save file        |
| `Ctrl+B`    | Focus file tree  |
| `Ctrl+E`    | Focus editor     |
| `Tab`       | Switch pane      |
| `↑ ↓`       | Move selection   |
| `Enter`     | Open file        |
| `Backspace` | Delete character |

---

## ⚙️ Installation

### Requirements

- Rust 1.70+
- Cargo

### Clone the repo

```bash
git clone https://github.com/Engineernoob/noir.git
cd noir
```

Run Noir
cargo run -- .

You can also open another project:

```bash
cargo run -- /path/to/project
```

## Philosophy

Noir is built around a few principles:

- Keyboard-first workflow

- Minimal UI

- Fast startup

- Terminal-native experience

The goal is not to replicate VS Code in the terminal, but to create a focused environment for developers who prefer working in the shell.

🛠 Built With

Rust

ratatui

crossterm

ropey

walkdir

## License

MIT License

## Contributing

Contributions are welcome.

If you want to help build Noir:

- Fork the repository

- Create a feature branch

- Submit a pull request

## Noir

Code in the dark.
