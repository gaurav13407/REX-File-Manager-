use crate::fs::navigator::Navigator;
pub enum Pane {
    Left,
    Right,
}

pub struct App {
    pub left: Navigator,
    pub preview_content: Vec<String>,
    pub active_pane: Pane,
    pub should_quit: bool,
    pub preview_mode: bool,
    pub preview_scroll: usize,
    pub preview_cursor: usize,
    pub visible_height:usize,
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
            visible_height:0,
        }
    }
}
