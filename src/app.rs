use crate::fs::navigator::Navigator;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub open_with: HashMap<String, String>, // ext -> app name
}

pub fn config_path() -> std::path::PathBuf {
    // 1. Next to the binary (when installed / cargo run)
    if let Ok(exe) = std::env::current_exe() {
        let p = exe.parent().unwrap_or(std::path::Path::new("/")).join("config.json");
        if p.exists() { return p; }
    }
    // 2. Current working directory (cargo run from project root)
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join("config.json");
        if p.exists() { return p; }
    }
    // 3. ~/.config/rex/config.json
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".config").join("rex").join("config.json");
        if p.exists() { return p; }
    }
    // Fallback: write to cwd
    std::env::current_dir().unwrap_or_default().join("config.json")
}

pub fn load_config() -> AppConfig {
    if let Ok(data) = std::fs::read_to_string(config_path()) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

pub fn save_config(config: &AppConfig) {
    if let Ok(data) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(config_path(), data);
    }
}

pub enum Pane {
    Left,
    Right,
}

pub enum Operation {
    DeleteBatch{items:Vec<(PathBuf,PathBuf)>},
    Copy { _from: PathBuf, to: PathBuf },
    Move { from: PathBuf, to: PathBuf },
}

pub struct App {
    pub left: Navigator,
    pub preview_content: Vec<String>,
    pub preview_cached_cursor: Option<usize>,   // cursor at last preview read
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
    pub selected: HashSet<PathBuf>,
    pub config: AppConfig,
    pub search_mode: bool,
    pub search_query: String,
    pub search_results: Vec<PathBuf>,
    pub search_cursor: usize,
    pub global_search: bool,
    pub open_with_mode: bool,
    pub open_with_options: Vec<String>,
    pub open_with_cursor: usize,
    pub show_help: bool,
    pub rename_mode: bool,
    pub input_buffer: String,
    pub rename_cursor: usize,   // caret position inside input_buffer
    pub show_info: bool,
    pub update_available: Option<String>, // Some("0.2.0") when an update exists
    pub show_update_popup: bool,           // show the update Y/N popup
    pub show_changelog: bool,
    pub changelog_lines: Vec<String>,
    pub changelog_scroll: usize,
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
            preview_cached_cursor: None,
            preview_cursor: 0,
            visible_height: 0,
            clipboard: None,
            cut_mode: false,
            confirm_delete: false,
            history: Vec::new(),
            status_msg: None,
            selected: HashSet::new(),
            config: load_config(),
            search_mode: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_cursor: 0,
            global_search: false,
            open_with_mode: false,
            open_with_options: Vec::new(),
            open_with_cursor: 0,
            show_help: false,
            rename_mode: false,
            input_buffer: String::new(),
            rename_cursor: 0,
            show_info: false,
            update_available: None,
            show_update_popup: false,
            show_changelog: false,
            changelog_lines: Vec::new(),
            changelog_scroll: 0,
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

    /// Re-read the file/directory at the cursor and cache lines into preview_content.
    /// Call this whenever the cursor position or directory changes.
    pub fn refresh_preview(&mut self) {
        self.preview_cached_cursor = Some(self.left.cursor);
        self.preview_scroll = 0;
        self.preview_cursor = 0;
        self.preview_content.clear();

        if let Some(path) = self.left.entries.get(self.left.cursor).cloned() {
            if path.is_dir() {
                if let Ok(read) = std::fs::read_dir(&path) {
                    use crate::ui::layout::get_icon;
                    self.preview_content = read
                        .flatten()
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            let icon = get_icon(&e.path());
                            format!("{} {}", icon, name)
                        })
                        .collect();
                }
            } else if let Ok(content) = std::fs::read_to_string(&path) {
                self.preview_content = content.lines().map(|l| l.to_string()).collect();
            } else {
                self.preview_content = vec!["Binary or unreadable file".to_string()];
            }
        }
    }
}
