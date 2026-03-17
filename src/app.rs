use crate::fs::navigator::Navigator;
use std::path::PathBuf;
pub enum Pane {
    Left,
    Right,
}

pub struct App {
    pub left: Navigator,
    pub right: Navigator,
    pub active_pane: Pane,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap();

        Self {
            left: Navigator::new(cwd.clone()),
            right: Navigator::new(cwd),
            active_pane: Pane::Left,
            should_quit: false,
        }
    }
}
