# Noir

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/built%20with-Rust-orange)

> A terminal-native code editor built for people who want the power of Neovim
> without the configuration rabbit hole.

---

## 🧠 What is Noir?

**Noir is a keyboard-first terminal editor written in Rust.**

It’s designed to sit between:

* ⚡ **Neovim** → powerful but complex
* 🧠 **VS Code** → easy but heavy

Noir aims to be:

* fast
* minimal
* discoverable
* and actually usable out of the box

No plugins required. No Lua configs. No 2-hour setup.

---

## ✨ Current Features

* 🖥 Terminal UI powered by **ratatui**
* 📁 Project **file explorer**
* ✍️ Rope-based **text editor** (`ropey`)
* 🔄 **Pane system** (editor / explorer)
* 💾 Open and save files
* 📍 Cursor tracking + status bar
* ⚡ Fast startup, low resource usage

---

## 🚧 What’s Coming Next

Noir is actively evolving toward a full terminal IDE:

* 🧠 **LSP support** (multi-language, not just Rust)
* 🎨 **Tree-sitter syntax highlighting**
* 🔍 **Project-wide search**
* 🧾 **Command palette**
* 🖥 **Embedded terminal pane**
* 🌳 **Nested file tree**
* 🔀 **Git integration**
* 🎛 Config + themes
* 🔌 Lightweight plugin system

---

## 🖥 Layout

```text
┌────────────────────────────────────────────────────────────────────────┐
│ Noir                                                                   │
├───────────────┬────────────────────────────────────────────────────────┤
│               │                                                        │
│  Explorer     │  src/main.rs                                          │
│               │                                                        │
│ ▸ src/        │  1  fn main() {                                        │
│   ├ main.rs   │  2      println!("Hello, Noir");                       │
│   ├ app.rs    │  3  }                                                  │
│   ├ ui.rs     │                                                        │
│               │                                                        │
├───────────────┴────────────────────────────────────────────────────────┤
│ EDITOR | root:noir | Ln 2, Col 12 | OK                                 │
└────────────────────────────────────────────────────────────────────────┘
```

---

## ⌨️ Controls

| Key         | Action           |
| ----------- | ---------------- |
| `Ctrl+Q`    | Quit             |
| `Ctrl+S`    | Save             |
| `Ctrl+B`    | Focus explorer   |
| `Ctrl+E`    | Focus editor     |
| `Tab`       | Switch pane      |
| `↑ ↓`       | Navigate         |
| `Enter`     | Open file        |
| `Backspace` | Delete character |

---

## ⚙️ Getting Started

### Requirements

* Rust 1.70+
* Cargo

### Run Noir

```bash
git clone https://github.com/Engineernoob/noir.git
cd noir
cargo run -- .
```

Open another project:

```bash
cargo run -- /path/to/project
```

---

## 🎯 Philosophy

Noir is built around a few ideas:

* **Stay in the terminal**
* **Keyboard-first everything**
* **Minimal but powerful**
* **Good defaults over heavy configuration**

You shouldn’t need to fight your editor to use it.

---

## 🛠 Built With

* Rust
* ratatui
* crossterm
* ropey
* walkdir

---

## 🤝 Contributing

If Noir sounds like something you’d use, help build it.

* Fork the repo
* Create a feature branch
* Open a PR

---

## 🖤 Noir

> Code in the dark.
