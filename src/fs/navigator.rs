use std::fs;
use std::path::PathBuf;

pub struct Navigator {
    pub path: PathBuf,
    pub entries: Vec<PathBuf>,
    pub entry_is_dir: Vec<bool>,   // cached from refresh — zero stat calls during render
    pub cursor: usize,
}

impl Navigator {
    pub fn new(path: PathBuf) -> Self {
        let mut nav = Self {
            path,
            entries: Vec::new(),
            entry_is_dir: Vec::new(),
            cursor: 0,
        };
        nav.refresh();
        nav
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.entry_is_dir.clear();

        // Parent entry
        if let Some(parent) = self.path.parent() {
            self.entries.push(parent.to_path_buf());
            self.entry_is_dir.push(true); // parent is always a dir
        }

        if let Ok(read) = fs::read_dir(&self.path) {
            // Use DirEntry::file_type() — cheaper than stat on most OSes
            for entry in read.flatten() {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                self.entries.push(entry.path());
                self.entry_is_dir.push(is_dir);
            }
        }

        // Sort using cached is_dir — no extra stat calls
        // Build combined vec, sort, then split back
        let mut combined: Vec<(PathBuf, bool)> = self.entries
            .drain(..)
            .zip(self.entry_is_dir.drain(..))
            .collect();

        let parent = self.path.parent().map(|p| p.to_path_buf());
        combined.sort_by(|(a, a_is_dir), (b, b_is_dir)| {
            if let Some(ref parent) = parent {
                if a == parent { return std::cmp::Ordering::Less; }
                if b == parent { return std::cmp::Ordering::Greater; }
            }
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
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

        for (path, is_dir) in combined {
            self.entries.push(path);
            self.entry_is_dir.push(is_dir);
        }

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
        if let Some(is_dir) = self.entry_is_dir.get(self.cursor) {
            if *is_dir {
                self.path = self.entries[self.cursor].clone();
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
