# Changelog

All notable changes to this project will be documented in this file.

---

## v0.4.0 — Disk Analyzer & System Awareness
**Release Date:** April 19, 2026

### 🚀 Added
- **💾 Disk usage bar** — Always-visible gauge at the bottom showing used/total/free space with color-coded fill (green < 70%, yellow < 90%, red ≥ 90%)
- **📊 Disk Analyzer mode (`z` / `Z`)** — ncdu-style directory size viewer in the right pane
  - Uses native `du` for instant results even on huge directories
  - Proportional bar charts with percentage for each entry
  - Color-coded bars (green/yellow/red) based on relative size
  - Navigate with `h/j/k/Enter`, exit with `Esc/q/z`
  - Title shows total directory size
- **Scrollable help popup (`?`)** — Help panel now supports `j/k` scrolling with position indicator
- **`Z` (capital) keybind** — Alias for `z` to toggle disk analyzer
- **Search filter keys in help** — `F1-F4` filter documentation now visible in help popup
- **`g` for global search** — Clearly documented in help popup

### 🔧 Improvements
- Disk stats auto-refresh when navigating to a different partition
- Help popup uses a fixed-height scrollable list instead of overflowing
- `format_size()` helper now supports TB-scale values
- Size analyzer picks the best (longest) mount-point match for accuracy

### 🧩 Keybinds (Disk Analyzer)
| Key | Action |
|-----|--------|
| `z` / `Z` | Toggle analyzer on/off |
| `j` / `k` | Navigate entries |
| `h` | Go to parent directory |
| `Enter` | Open directory (rescans) |
| `Esc` / `q` | Exit analyzer |

---

## v0.3.2 — Nucleo Fuzzy Search Upgrade
**Release Date:** April 17, 2026

### 🚀 Added
- **Nucleo-powered fuzzy search** — Search results are now ranked with `nucleo` instead of the old manual scorer
- **Filename-first matching** — Global search ranks by file or folder name, then returns the original full path
- **Search result refresh on every filter key** — `F1`/`F2`/`F3`/`F4`, typing, and backspace all rerun the active search with the current filter

### 🎯 Search Filters
- **F1** → Folders only
- **F2** → Files only
- **F3** → System-wide search from `/` with hidden entries included
- **F4** → All results from home search

### 🔧 Improvements
- Global search now starts from `$HOME` for `All`, `Files`, and `Folders`
- `System` search is the only mode that searches from `/`
- Home-scoped search now uses `--one-file-system` and skips noisy directories like `.cargo`, `.git`, `target`, `.wine`, `.rex_trash`, and `node_modules`
- Raw `fd` candidate lists are capped before fuzzy ranking to keep search responsive

### ⚠️ Compatibility
- Still works without `fd` by falling back to `find`

---

## [0.1.0] - Initial Release

### 🚀 Added
- Basic file navigation (vim-style)
- Two-pane layout (explorer + preview)
- Keyboard-driven UI
- Basic file preview
- Initial project structure
