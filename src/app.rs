use crate::fs::navigator::Navigator;
use std::path::PathBuf;
pub enum Pane {
    Left,
    Right,
}

pub enum Operation{
    Delete{original:PathBuf,trash:PathBuf},
    Copy{from:PathBuf,to:PathBuf},
    Move{from:PathBuf,to:PathBuf},
}

pub struct App {
    pub left: Navigator,
    pub preview_content: Vec<String>,
    pub active_pane: Pane,
    pub should_quit: bool,
    pub preview_mode: bool,
    pub preview_scroll: usize,
    pub preview_cursor: usize,
    pub visible_height: usize,
    pub clipboard: Option<PathBuf>,
    pub cut_mode: bool,
    pub confirm_delete: bool,
    pub history: Vec<Operation>,
    pub status_msg: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap();

        Self {
            left: Navigator::new(cwd.clone()),
            active_pane: Pane::Left,
            should_quit: false,
            preview_mode: false,
            preview_scroll: 0,
            preview_content: Vec::new(),
            preview_cursor: 0,
            visible_height: 0,
            clipboard: None,
            cut_mode: false,
            confirm_delete: false,
            history: Vec::new(),
            status_msg: None,
        }
    }

    /// Enforce all scroll invariants in one place.
    /// Call this after any mutation of preview_cursor, preview_scroll,
    /// or visible_height (including after a terminal resize).
    pub fn clamp_scroll(&mut self, total_lines: usize, visible_height: usize) {
        if total_lines == 0 || visible_height == 0 {
            self.preview_cursor = 0;
            self.preview_scroll = 0;
            return;
        }

        // 1. Cursor must stay within the content.
        let max_cursor = total_lines.saturating_sub(1);
        if self.preview_cursor > max_cursor {
            self.preview_cursor = max_cursor;
        }

        // 2. Scroll down to reveal the cursor.
        if self.preview_cursor >= self.preview_scroll + visible_height {
            self.preview_scroll = self.preview_cursor - visible_height + 1;
        }

        // 3. Scroll up to reveal the cursor.
        if self.preview_cursor < self.preview_scroll {
            self.preview_scroll = self.preview_cursor;
        }

        // 4. Never leave empty rows at the bottom.
        let max_scroll = total_lines.saturating_sub(visible_height);
        if self.preview_scroll > max_scroll {
            self.preview_scroll = max_scroll;
        }
    }
}
