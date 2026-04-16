# Changelog

All notable changes to this project will be documented in this file.

---

## v0.3.1 — Smart System Files & File Creation
**Release Date:** April 16, 2026

### 🚀 Added
- **Hide system files by default** — Hidden files/folders (starting with `.`) are now hidden in normal browsing
  - Press **F3** to toggle view system files only
  - Keep file manager clean and focused on regular files
- **Create new files** — Press `n` to create a new file with custom name
- **Create new folders** — Press `N` to create a new folder with custom name
  - Input popup with visual feedback
  - Prevents accidental overwrites (checks if file already exists)
  - Shows success/error messages
- **Fuzzy search with Nucleo** — Smart filename matching for search results
  - Ranks results by relevance, not just alphabetical
  - Better search experience for finding files

### 🎯 Filter System (Search)
- **F1** → Folders only (no hidden)
- **F2** → Files only (no hidden)
- **F3** → System/hidden files only
- **F4** → All files (including hidden - legacy mode)

### 🔧 Improvements
- Smart file/folder filtering applies to both normal browsing and search
- Hidden files no longer clutter the main view
- Better UX for file operations workflow
- Updated help section (?) with new keybinds

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
