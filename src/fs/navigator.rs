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

        self.entries.sort_by(|a, b| {
            //keep parent at top
            if let Some(parent) = self.path.parent() {
                if a == parent {
                    return std::cmp::Ordering::Less;
                }
                if b == parent {
                    return std::cmp::Ordering::Greater;
                }
            }
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();

            match (a_is_dir, b_is_dir) {
                //Folder first
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,

                //same type->sort by name
                _ => a
                    .file_name()
                    .unwrap_or_else(|| a.as_os_str())
                    .to_string_lossy()
                    .to_lowercase()
                    .cmp(
                        &b.file_name()
                            .unwrap_or_else(|| b.as_os_str())
                            .to_string_lossy()
                            .to_lowercase(),
                    ),
            }
        });

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
