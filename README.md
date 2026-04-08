# REX — Terminal File Manager

A fast, keyboard-driven TUI file manager written in Rust using [ratatui](https://github.com/ratatui-org/ratatui).

![Rust](https://img.shields.io/badge/built_with-Rust-orange?logo=rust)
![TUI](https://img.shields.io/badge/interface-TUI-blue)

---

## Features

- ⚡ **Zero-lag navigation** — dirty-flag rendering, async preview loading, cached stat calls
- 🔍 **Fast search** — `/` local search, `g` global search (via `fd`), non-blocking async results
- 📋 **Selection-first workflow** — select multiple files with `Space`, then batch copy/cut/delete
- 🗂️ **Open With** — `o` opens with configured app, `O` shows a selection popup
- ⚙️ **JSON config** — map file extensions to apps in `config.json`
- 🗑️ **Trash support** — delete moves to trash with undo
- 📖 **Inline help** — press `?` for keybind reference

---

## Install

```bash
# Clone
git clone https://github.com/yourusername/rex.git
cd rex

# Install fd (for search)
sudo apt install fd-find   # Ubuntu/Debian
# or: sudo pacman -S fd    # Arch

# Run
cargo run
```

> Requires Rust ≥ 1.70

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
| `o` | Open file with configured app |
| `O` | Open With popup — choose app, saves as new default |
| `Space` | Toggle select file |
| `A` | Select all files |
| `y` | Copy selected / cursor file |
| `x` | Cut selected / cursor file |
| `p` | Paste |
| `d` | Delete (moves to trash), prompts confirm |
| `u` | Undo last operation |
| `Esc` | Clear selection |

### Search
| Key | Action |
|-----|--------|
| `/` | Local search (current directory, depth 5) |
| `g` | Global search (from `/`, depth 8) |
| `j` / `k` | Navigate results |
| `Enter` | Jump to result directory |
| `Esc` / `q` | Exit search |

### General
| Key | Action |
|-----|--------|
| `?` | Toggle help popup |
| `q` | Quit |

---

## Configuration

Edit `config.json` in the project root to map file extensions to apps:

```json
{
  "rs": "nvim",
  "md": "nvim",
  "pdf": "libreoffice",
  "png": "eog",
  "mp4": "vlc",
  "mp3": "vlc"
}
```

- **`o`** opens with the mapped app, or `xdg-open` if no mapping exists
- **`O`** lets you pick from a list and saves the choice back to `config.json`

---

## Architecture

```
src/
├── main.rs          # Event loop, key handling, async search/preview
├── app.rs           # App state (App struct, config, operations)
├── fs/
│   └── navigator.rs # Directory listing with cached is_dir flags
└── ui/
    └── layout.rs    # Rendering (ratatui widgets, popups)
```

**Performance design:**
- Preview loaded in background thread via `mpsc::channel` — never blocks the event loop
- Search runs `fd` in a background thread with `AtomicBool` cancellation — each keystroke cancels the previous search
- Rendering uses a `needs_draw` dirty flag — no redundant redraws
- Zero `stat()` syscalls during render — `is_dir` cached in `Navigator`

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI rendering |
| `crossterm` | Terminal backend |
| `notify` | Filesystem watcher |
| `walkdir` | Directory traversal |
| `serde` + `serde_json` | Config file (de)serialization |
| `dirs` | Home directory resolution |

---

## License

MIT
