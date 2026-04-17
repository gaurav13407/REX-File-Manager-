# Changelog

All notable changes to this project will be documented in this file.

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
