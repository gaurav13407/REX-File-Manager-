mod app;
mod fs;
mod ui;
mod utils;

use utils::trash::move_to_trash;

use app::{App, Operation, Pane};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::sync::mpsc::channel;
use std::{io, path::PathBuf};

fn get_unique_path(mut dest: std::path::PathBuf) -> std::path::PathBuf {
    let mut count = 1;

    while dest.exists() {
        let file_stem = dest.file_stem().unwrap().to_string_lossy().to_string();

        let extension = dest.extension().map(|e| e.to_string_lossy());

        let new_name = if let Some(ext) = extension {
            format!("{}_{}.{}", file_stem, count, ext)
        } else {
            format!("{}_{}", file_stem, count)
        };

        dest.set_file_name(new_name);
        count += 1;
    }

    dest
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    let (tx, rx) = channel::<notify::Result<notify::Event>>();

    //clone path to watch
    let watch_path = app.left.path.clone();

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, notify::Config::default()).unwrap();

    //start watching initial path
    watcher
        .watch(&app.left.path, RecursiveMode::NonRecursive)
        .unwrap();

    // Initial preview load
    app.refresh_preview();

    let mut current_watch_path = app.left.path.clone();
    let mut needs_draw = true;

    while !app.should_quit {
        if app.left.path != current_watch_path {
            watcher.unwatch(&current_watch_path).ok();

            //watch new
            watcher
                .watch(&app.left.path, RecursiveMode::NonRecursive)
                .ok();

            current_watch_path = app.left.path.clone();
        }
        // Draw FIRST — instant visual feedback regardless of preview state.
        if needs_draw {
            terminal.draw(|frame| {
                ui::layout::draw(frame, &mut app);
            })?;
            needs_draw = false;
        }

        // Debounced preview AFTER draw: only read disk when cursor changed
        // and no key is queued. Screen is already updated at this point.
        let cursor_changed = app.preview_cached_cursor != Some(app.left.cursor);
        if cursor_changed && !event::poll(std::time::Duration::ZERO)? {
            app.refresh_preview();
            needs_draw = true; // redraw to show fresh preview
        }

        if rx.try_recv().is_ok() {
            app.left.refresh();
            if !app.left.entries.is_empty() {
                app.left.cursor = app.left.cursor.min(app.left.entries.len() - 1);
            } else {
                app.left.cursor = 0;
            }
            needs_draw = true;
        }
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                // total_lines from cache — no disk read.
                let total_lines = app.preview_content.len();

                // Clear any previous status message on the next keypress.
                app.status_msg = None;

                // If waiting for delete confirmation, handle y/n and skip normal keys.
                if app.confirm_delete {
                    match key.code {
                        KeyCode::Char('y') => {
                            let mut batch: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

                            if !app.selected.is_empty() {
                                for path in app.selected.clone() {
                                    let trash_dest = {
                                        let trash_dir = utils::trash::get_trash_dir();
                                        utils::trash::unique_dest_pub(
                                            trash_dir.join(path.file_name().unwrap()),
                                        )
                                    };
                                    if move_to_trash(&path).is_ok() {
                                        batch.push((path, trash_dest));
                                    }
                                }
                                app.selected.clear();
                            } else if let Some(path) =
                                app.left.entries.get(app.left.cursor).cloned()
                            {
                                let is_parent = app.left.path.parent()
                                    .map_or(false, |parent| path.as_path() == parent);
                                if !is_parent {
                                    let trash_dest = {
                                        let trash_dir = utils::trash::get_trash_dir();
                                        utils::trash::unique_dest_pub(
                                            trash_dir.join(path.file_name().unwrap()),
                                        )
                                    };
                                    if move_to_trash(&path).is_ok() {
                                        batch.push((path, trash_dest));
                                    }
                                }
                            }

                            if !batch.is_empty() {
                                let count = batch.len();
                                app.history.push(app::Operation::DeleteBatch { items: batch });
                                app.status_msg = Some(format!("Trashed {} item(s)", count));
                                app.left.refresh();
                                if !app.left.entries.is_empty() {
                                    app.left.cursor =
                                        app.left.cursor.min(app.left.entries.len() - 1);
                                } else {
                                    app.left.cursor = 0;
                                }
                            }
                            app.confirm_delete = false;
                        }

                        KeyCode::Char('n') | KeyCode::Esc => {
                            app.confirm_delete = false;
                        }

                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,

                    KeyCode::Tab => {
                        app.active_pane = match app.active_pane {
                            Pane::Left => Pane::Right,
                            Pane::Right => Pane::Left,
                        }
                    }

                    KeyCode::Char('j') => match app.active_pane {
                        Pane::Left => {
                            app.left.move_down();
                        }
                        Pane::Right => {
                            if total_lines > 0 && app.preview_cursor < total_lines - 1 {
                                app.preview_cursor += 1;
                            }
                            let vh = app.visible_height;
                            app.clamp_scroll(total_lines, vh);
                        }
                    },

                    KeyCode::Char('k') => match app.active_pane {
                        Pane::Left => {
                            app.left.move_up();
                        }
                        Pane::Right => {
                            app.preview_cursor = app.preview_cursor.saturating_sub(1);
                            let vh = app.visible_height;
                            app.clamp_scroll(total_lines, vh);
                        }
                    },

                    KeyCode::Char('l') => {
                        if !app.preview_mode {
                            if let Pane::Left = app.active_pane {
                                app.left.enter();
                                app.left.refresh();
                                app.refresh_preview();
                            }
                        }
                    }

                    KeyCode::Char('h') => {
                        if !app.preview_mode {
                            if let Pane::Left = app.active_pane {
                                app.left.back();
                                app.left.refresh();
                                app.refresh_preview();
                            }
                        }
                    }

                    KeyCode::Enter => {
                        if !app.preview_mode {
                            if let Pane::Left = app.active_pane {
                                app.left.enter();
                                app.left.refresh();
                                app.refresh_preview();
                            }
                        }
                    }

                    // Space: toggle selection on current entry, advance cursor
                    KeyCode::Char(' ') => {
                        if let Pane::Left = app.active_pane {
                            if let Some(path) = app.left.entries.get(app.left.cursor).cloned() {
                                let is_parent = app.left.path.parent()
                                    .map_or(false, |par| path.as_path() == par);
                                if !is_parent {
                                    if app.selected.contains(&path) {
                                        app.selected.remove(&path);
                                    } else {
                                        app.selected.insert(path);
                                    }
                                }
                            }
                            app.left.move_down();
                        }
                    }

                    // y: copy — operates on selection if any, else cursor
                    KeyCode::Char('y') => {
                        if !app.selected.is_empty() {
                            app.clipboard = None;
                            app.cut_mode = false;
                            app.status_msg = Some(format!(
                                "Yanked {} item(s) — press p to paste",
                                app.selected.len()
                            ));
                        } else if let Some(path) = app.left.entries.get(app.left.cursor) {
                            let is_parent = app.left.path.parent()
                                .map_or(false, |par| path.as_path() == par);
                            if !is_parent {
                                app.clipboard = Some(path.clone());
                                app.cut_mode = false;
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                app.status_msg = Some(format!("Yanked: {}", name));
                            }
                        }
                    }

                    // x: cut — operates on selection if any, else cursor
                    KeyCode::Char('x') => {
                        if !app.selected.is_empty() {
                            app.clipboard = None;
                            app.cut_mode = true;
                            app.status_msg = Some(format!(
                                "Cut {} item(s) — press p to move",
                                app.selected.len()
                            ));
                        } else if let Some(path) = app.left.entries.get(app.left.cursor) {
                            let is_parent = app.left.path.parent()
                                .map_or(false, |par| path.as_path() == par);
                            if !is_parent {
                                app.clipboard = Some(path.clone());
                                app.cut_mode = true;
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                app.status_msg = Some(format!("Cut: {}", name));
                            }
                        }
                    }

                    KeyCode::Char('p') => {
                        // Multi-select: operate on all selected files
                        if !app.selected.is_empty() {
                            let count = app.selected.len();
                            for src in app.selected.clone() {
                                let file_name = src.file_name().unwrap();
                                let mut dest = app.left.path.clone();
                                dest.push(file_name);
                                let dest = get_unique_path(dest);

                                if app.cut_mode {
                                    match std::fs::rename(&src, &dest) {
                                        Ok(_) => {
                                            app.history.push(Operation::Move {
                                                from: src.clone(),
                                                to: dest.clone(),
                                            });
                                        }
                                        Err(e) => eprintln!("Move failed: {}", e),
                                    }
                                } else {
                                    match std::fs::copy(&src, &dest) {
                                        Ok(_) => {
                                            app.history.push(Operation::Copy {
                                                from: src.clone(),
                                                to: dest.clone(),
                                            });
                                        }
                                        Err(e) => eprintln!("Copy failed: {}", e),
                                    }
                                }
                            }

                            if app.cut_mode {
                                app.selected.clear();
                                app.cut_mode = false;
                            }

                            app.status_msg = Some(format!("Processed {} item(s)", count));
                            app.left.refresh();

                        // Fallback: single clipboard file
                        } else if let Some(src) = app.clipboard.clone() {
                            let file_name = src.file_name().unwrap();
                            let mut dest = app.left.path.clone();
                            dest.push(file_name);
                            let dest = get_unique_path(dest);

                            if app.cut_mode {
                                match std::fs::rename(&src, &dest) {
                                    Ok(_) => {
                                        app.history.push(Operation::Move {
                                            from: src.clone(),
                                            to: dest.clone(),
                                        });
                                        app.clipboard = None;
                                        app.cut_mode = false;
                                    }
                                    Err(e) => eprintln!("Move failed: {}", e),
                                }
                            } else {
                                match std::fs::copy(&src, &dest) {
                                    Ok(_) => {
                                        app.history.push(Operation::Copy {
                                            from: src.clone(),
                                            to: dest.clone(),
                                        });
                                    }
                                    Err(e) => eprintln!("Copy failed: {}", e),
                                }
                            }

                            app.left.refresh();
                        }
                    }

                    KeyCode::Char('d') => {
                        app.confirm_delete = true;
                    }

                    KeyCode::Char('u') => {
                        if let Some(op) = app.history.pop() {
                            match op {
                                app::Operation::DeleteBatch { items } => {
                                    let count = items.len();
                                    for (original, trash) in &items {
                                        let _ = std::fs::rename(trash, original);
                                    }
                                    if count == 1 {
                                        let name = items[0].0
                                            .file_name()
                                            .map(|n: &std::ffi::OsStr| n.to_string_lossy().into_owned())
                                            .unwrap_or_else(|| "file".into());
                                        app.status_msg = Some(format!("✔ Restored: {}", name));
                                    } else {
                                        app.status_msg = Some(format!("✔ Restored {} item(s)", count));
                                    }
                                }
                                app::Operation::Copy { to, .. } => {
                                    let result = if to.is_dir() {
                                        std::fs::remove_dir_all(&to)
                                    } else {
                                        std::fs::remove_file(&to)
                                    };
                                    if let Err(e) = result {
                                        eprintln!("Undo copy failed: {}", e);
                                    }
                                }

                                app::Operation::Move { from, to } => {
                                    if let Err(e) = std::fs::rename(&to, &from) {
                                        eprintln!("Undo move failed: {}", e);
                                    }
                                }
                            }
                            app.left.refresh();
                        }
                    }

                    // A: select all (skip parent)
                    KeyCode::Char('A') => {
                        app.selected.clear();
                        let parent = app.left.path.parent().map(|p| p.to_path_buf());
                        for path in &app.left.entries {
                            if parent.as_deref().map_or(false, |par| path.as_path() == par) {
                                continue;
                            }
                            app.selected.insert(path.clone());
                        }
                    }

                    KeyCode::Esc => {
                        app.selected.clear();
                        app.clipboard = None;
                        app.cut_mode = false;
                    }

                    _ => {}
                }
                needs_draw = true;
            }
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}
