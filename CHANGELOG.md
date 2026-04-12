# Changelog

## v.3.0-Current
# Changelog

All notable changes to this project will be documented in this file.

---

### 🚀 Added
- Async search system (non-blocking UI)
- Global search (`g`) and local search (`/`)
- Search filtering:
  - F1 → Folders only
  - F2 → Files only
  - F3 → System/hidden files
  - F4 → All
- Multi-select system (batch operations)
- Copy / Cut / Paste support for multiple files
- Delete with confirmation (safe delete)
- Trash system (files moved instead of permanent delete)
- Undo system (`u`) for file operations
- File preview panel with scrolling
- Open with default app (`o`)
- Open with selection (`O`)
- JSON-based app configuration (`config.json`)
- Inline help popup (`?`)
- Filesystem auto-refresh using watcher
- Background preview loading (non-blocking)

---

### ⚡ Performance
- Fast search using `fd` / `fdfind` (if available)
- Fallback to `find` for compatibility
- Async operations using threads + channels
- Reduced UI redraws (dirty flag system)
- Cached directory metadata (no repeated syscalls)

---

### 🛠 Improvements
- Better scroll handling in preview pane
- Proper bounds checking (no infinite scroll bug)
- Cursor + scroll synchronization
- Cleaner key handling logic
- Improved search UI and navigation

---

### 🐛 Fixed
- Infinite scrolling bug in preview panel
- Cursor out-of-bounds after resize
- Incorrect highlight offset in preview list
- Layout issues when resizing terminal
- Search not refreshing on filter change

---

### ⚠️ Compatibility
- Works without `fd` (fallback to `find`)
- Supports Linux environments (tested on Ubuntu/Arch)

---

## [0.1.0] - Initial Release

### 🚀 Added
- Basic file navigation (vim-style)
- Two-pane layout (explorer + preview)
- Keyboard-driven UI
- Basic file preview
- Initial project structure
