mod app;
mod fs;
mod ui;
mod utils;

use utils::trash::move_to_trash;

use app::{App, Pane,Operation};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::mpsc::channel;

fn get_unique_path(mut dest: std::path::PathBuf) -> std::path::PathBuf {
    let mut count = 1;

    while dest.exists() {
        let file_stem = dest
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

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

    let mut current_watch_path = app.left.path.clone();

    while !app.should_quit {
        if app.left.path != current_watch_path {
            watcher.unwatch(&current_watch_path).ok();

            //watch new
            watcher
                .watch(&app.left.path, RecursiveMode::NonRecursive)
                .ok();

            current_watch_path = app.left.path.clone();
        }
        terminal.draw(|frame| {
            ui::layout::draw(frame, &mut app);
        })?;

        if rx.try_recv().is_ok() {
            app.left.refresh();
            if !app.left.entries.is_empty() {
                app.left.cursor = app.left.cursor.min(app.left.entries.len() - 1);
            } else {
                app.left.cursor = 0;
            }
        }
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Compute total_lines once per event for the currently previewed file.
                let total_lines = app
                    .left
                    .entries
                    .get(app.left.cursor)
                    .filter(|p| p.is_file())
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .map(|c| c.lines().count())
                    .unwrap_or(0);

                // Clear any previous status message on the next keypress.
                app.status_msg = None;

                // If waiting for delete confirmation, handle y/n and skip normal keys.
                if app.confirm_delete {
                    match key.code {
                        KeyCode::Char('y') => {
                            if let Some(path) = app.left.entries.get(app.left.cursor).cloned() {
                                let is_parent = app
                                    .left
                                    .path
                                    .parent()
                                    .map_or(false, |parent| path == parent);

                                if is_parent {
                                    eprintln!("Cannot trash parent directory");
                                } else {
                                    // Compute where it will land before moving.
                                    let trash_dest = {
                                        let trash_dir = utils::trash::get_trash_dir();
                                        utils::trash::unique_dest_pub(trash_dir.join(
                                            path.file_name().unwrap(),
                                        ))
                                    };
                                    match move_to_trash(&path) {
                                        Ok(()) => {
                                            app.history.push(app::Operation::Delete {
                                                original: path.clone(),
                                                trash: trash_dest,
                                            });
                                            app.left.refresh();
                                            if !app.left.entries.is_empty() {
                                                app.left.cursor =
                                                    app.left.cursor.min(app.left.entries.len() - 1);
                                            } else {
                                                app.left.cursor = 0;
                                            }
                                        }
                                        Err(e) => eprintln!("Trash failed: {}", e),
                                    }
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
                            app.preview_scroll = 0;
                            app.preview_cursor = 0;
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
                            app.preview_scroll = 0;
                            app.preview_cursor = 0;
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
                            }
                        }
                    }

                    KeyCode::Char('h') => {
                        if !app.preview_mode {
                            if let Pane::Left = app.active_pane {
                                app.left.back();
                                app.left.refresh();
                            }
                        }
                    }

                    KeyCode::Enter => {
                        if !app.preview_mode {
                            if let Pane::Left = app.active_pane {
                                app.left.enter();
                                app.left.refresh();
                            }
                        }
                    }

                    KeyCode::Char('y') => {
                        if let Some(path) = app.left.entries.get(app.left.cursor) {
                            app.clipboard = Some(path.clone());
                            app.cut_mode = false;

                        

                        let name=path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                        app.status_msg=Some(format!("Copied:{}",name));
                    
                        }
                    }


                   KeyCode::Char('p') => {
    if let Some(src) = &app.clipboard {
        let file_name = src.file_name().unwrap();
        let mut dest = app.left.path.clone();
        dest.push(file_name);

        let dest = get_unique_path(dest);

        if app.cut_mode {
            // 🔥 MOVE
            match std::fs::rename(src, &dest) {
                Ok(_) => {
                    app.history.push(Operation::Move {
                        from: src.clone(),
                        to: dest.clone(),
                    });

                    app.clipboard = None;
                    app.cut_mode = false;
                }
                Err(e) => {
                    eprintln!("Move failed: {}", e);
                }
            }
        } else {
            // 🔥 COPY
            match std::fs::copy(src, &dest) {
                Ok(_) => {
                    app.history.push(Operation::Copy {
                        from: src.clone(),
                        to: dest.clone(),
                    });
                }
                Err(e) => {
                    eprintln!("Copy failed: {}", e);
                }
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
                                app::Operation::Delete { original, trash } => {
                                    match std::fs::rename(&trash, &original) {
                                        Ok(()) => {
                                            let name = original
                                                .file_name()
                                                .map(|n| n.to_string_lossy().into_owned())
                                                .unwrap_or_else(|| "file".into());
                                            app.status_msg =
                                                Some(format!("✔ Restored: {}", name));
                                        }
                                        Err(e) => {
                                            app.status_msg =
                                                Some(format!("✗ Restore failed: {}", e));
                                        }
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

                    KeyCode::Char('x')=>{
                        if let Some(path)=app.left.entries.get(app.left.cursor){
                            app.clipboard=Some(path.clone());
                            app.cut_mode=true;

                            let name=path.file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string();

                            app.status_msg=Some(format!("Cut:{}",name));
                        }

                    }

                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}
