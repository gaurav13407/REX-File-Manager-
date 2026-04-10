# REX — Terminal File Manager

> ⚡ A Vim-inspired, blazing-fast TUI file manager built in Rust.

![Rust](https://img.shields.io/badge/built_with-Rust-orange?logo=rust)
![TUI](https://img.shields.io/badge/interface-TUI-blue)
![Platform](https://img.shields.io/badge/platform-Linux-informational?logo=linux)
![License](https://img.shields.io/badge/license-MIT-green)

---

## 🚀 Version 2 is Coming

> **REX v2 is currently in active development and will be launching soon.**

Version 2 is a major leap forward — the goal is to make REX a **true daily-driver file manager** that you can replace Nautilus, Thunar, or any other GUI file manager with, right from your terminal.

**What's planned for v2:**
- 🔥 Major performance overhaul — even faster rendering and directory scans
- 🎨 Redesigned UI with better visual hierarchy and themes
- 📂 Dual-pane mode
- 🔖 Bookmarks / pinned directories
- 🖥️ macOS and Windows support *(Linux-only as of v1 — cross-platform is on the roadmap)*
- And much more...

If you find REX useful, star the repo and watch for the v2 release!

---

## What is REX?

REX is a keyboard-driven terminal file manager **inspired by Vim**. If you've used Vim, you'll feel right at home — `hjkl` to navigate, modal-style interactions, and zero mouse dependency. It's lightweight, fast, and stays out of your way.

It's built with [Rust](https://www.rust-lang.org/) and [ratatui](https://github.com/ratatui-org/ratatui), which means it starts instantly, uses almost no memory, and never lags.

> **Linux only as of now.** macOS and Windows support is planned for a future release.

---

## Features

| | Feature |
|---|---|
| ⚡ | **Zero-lag navigation** — dirty-flag rendering, async previews, cached `stat` calls |
| 🔍 | **Fast search** — `/` local, `g` global (via `fd`), fully async with per-keystroke cancellation |
| ✏️ | **Inline rename** — press `r`, edit with arrow keys, confirm with Enter |
| ℹ️ | **File info popup** — press `i` for name, size, type, permissions, modified date, full path |
| 📋 | **Multi-select** — `Space` to toggle, `A` to select all, then batch copy/cut/delete |
| 🗂️ | **Open With** — `o` opens with configured app, `O` shows a picker popup |
| ⚙️ | **JSON config** — map any extension to any app in `~/.config/rex/config.json` |
| 🗑️ | **Trash support** — delete sends to trash, `u` to undo |
| 📖 | **Built-in help** — press `?` anytime |

---

## Install

```bash
# Clone the repo
git clone https://github.com/gaurav13407/REX-File-Manager
cd REX-File-Manager

# Run directly
cargo run

# Or install the binary globally
cargo install --path .
```

> Requires **Rust ≥ 1.70**

```bash
# Install fd for faster search (optional but recommended)
sudo apt install fd-find       # Ubuntu / Debian
sudo pacman -S fd              # Arch
brew install fd                # macOS (future)
# Falls back to system 'find' automatically if fd is not installed
```

---

## Keybinds

### Navigation
| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `h` | Go to parent directory |
| `l` / `Enter` | Enter directory |
| `Tab` | Switch between file list and preview pane |

### File Operations
| Key | Action |
|-----|--------|
| `r` | **Rename** — opens popup with arrow-key cursor, pre-filled name |
| `i` | **File info** — popup with size, type, permissions, modified, path |
| `o` | Open file with configured app |
| `O` | Open With popup — choose app, saves as new default |
| `Space` | Toggle select file |
| `A` | Select all files |
| `y` | Copy selected / cursor file |
| `x` | Cut selected / cursor file |
| `p` | Paste |
| `d` | Delete → trash (confirm with `y`) |
| `u` | Undo last operation |
| `Esc` | Clear selection / close popups |

### Search
| Key | Action |
|-----|--------|
| `/` | Local search (current directory, depth 5) |
| `g` | Global search (from `/`, depth 8) |
| `j` / `k` | Navigate results |
| `Enter` | Jump to result |
| `Esc` / `q` | Exit search |

### Rename Popup Keys
| Key | Action |
|-----|--------|
| `←` / `→` | Move cursor left / right |
| `Home` / `End` | Jump to start / end |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Enter` | Confirm rename |
| `Esc` | Cancel |

### General
| Key | Action |
|-----|--------|
| `?` | Toggle help popup |
| `q` | Quit |

---

## Configuration

REX looks for `config.json` in the following order:
1. Next to the binary (for installed builds)
2. Current working directory
3. `~/.config/rex/config.json`

```json
{
  "open_with": {
    "rs": "nvim",
    "md": "nvim",
    "pdf": "evince",
    "png": "eog",
    "mp4": "vlc",
    "mp3": "vlc"
  }
}
```

- **`o`** opens with the mapped app, or falls back to `xdg-open`
- **`O`** lets you pick from a list and saves the choice back automatically

---

## Architecture

```
src/
├── main.rs          # Event loop, key handling, async search
├── app.rs           # App state (App struct, config, history)
├── fs/
│   └── navigator.rs # Directory listing with cached is_dir flags
├── ui/
│   └── layout.rs    # Rendering: widgets, popups, icons
└── utils/
    └── trash.rs     # Trash-safe deletion and undo
```

**Performance design:**
- Async preview via `mpsc::channel` — never blocks the event loop
- Search runs `fd` in a background thread with `AtomicBool` cancellation
- `needs_draw` dirty flag — zero redundant redraws
- Zero `stat()` syscalls during render — `is_dir` cached at directory load time

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI rendering |
| `crossterm` | Terminal backend & input |
| `notify` | Filesystem watcher (live refresh) |
| `serde` + `serde_json` | Config file serialization |
| `dirs` | Home directory resolution |

---

## Platform Support

| Platform | Status |
|----------|--------|
| 🐧 Linux | ✅ Fully supported |
| 🍎 macOS | 🔜 Planned (v2) |
| 🪟 Windows | 🔜 Planned (v2) |

---

## License

MIT — do whatever you want with it.

---

<p align="center">
  Built with ❤️ and Rust &nbsp;|&nbsp; Vim-inspired &nbsp;|&nbsp; Stay in the terminal
</p>
