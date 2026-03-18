use std::fs;
use std::path::PathBuf;

pub struct Navigator {
    pub path: PathBuf,
    pub entries: Vec<PathBuf>,
    pub cursor: usize,
}

impl Navigator {
    pub fn new(path: PathBuf) -> Self {
        let mut nav = Self {
            path,
            entries: Vec::new(),
            cursor: 0,
        };

        nav.refresh();
        nav
    }

    pub fn refresh(&mut self) {
        self.entries.clear();

        if let Some(parent) = self.path.parent() {
            self.entries.push(parent.to_path_buf());
        }

        if let Ok(read) = fs::read_dir(&self.path) {
            for entry in read.flatten() {
                self.entries.push(entry.path());
            }
        }

        self.entries.sort();

        if self.cursor >= self.entries.len() {
            self.cursor = 0;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn enter(&mut self) {
        if let Some(path) = self.entries.get(self.cursor) {
            if path.is_dir() {
                self.path = path.clone();
                self.cursor = 0;
                self.refresh();
            }
        }
    }

    pub fn back(&mut self) {
        if let Some(parent) = self.path.parent() {
            self.path = parent.to_path_buf();
            self.cursor = 0;
            self.refresh();
        }
    }
}
