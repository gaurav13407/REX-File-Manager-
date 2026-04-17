mod app;
mod fs;
#[path = "utils/fuzzy.rs"]
mod fuzzy;
mod ui;
mod utils;
use app::SearchFilter;
use fuzzy::FuzzyFinder;
use utils::trash::move_to_trash;

use app::{App, Operation, Pane};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::sync::mpsc::{Sender, channel};
use std::{io};

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

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::sync::mpsc as smpsc;

fn spawn_search(
    query: String,
    search_root: std::path::PathBuf,
    global: bool,
    filter: SearchFilter,
    tx: smpsc::Sender<Vec<std::path::PathBuf>>,
    cancel: Arc<AtomicBool>,
    status_tx:Sender<String>,
) {
    std::thread::spawn(move || {
        let max_depth = if global { "8" } else { "5" };
        let exclusions = [".cargo", "target", ".git", ".wine", ".rex_trash", "node_modules"];

        let build_fd_command = |program: &str| {
            let mut command = std::process::Command::new(program);
            command.arg(&query).arg(&search_root);

            if program == "fdfind" {
                command.arg("-H");
            }

            command.arg("--max-depth").arg(max_depth);
            command.arg("--color").arg("never");

            if matches!(filter, SearchFilter::System) {
                command.arg("--hidden");
            } else {
                command.arg("--one-file-system");
                for exclusion in exclusions {
                    command.arg("--exclude").arg(exclusion);
                }
            }

            match filter {
                SearchFilter::Files => {
                    command.arg("--type").arg("f");
                }
                SearchFilter::Folders => {
                    command.arg("--type").arg("d");
                }
                _ => {}
            }

            command
        };

        // Try fd (fastest) → fdfind (Debian name) → fallback to built-in find

        let fd_output = build_fd_command("fd")
            .output()
            .or_else(|_| build_fd_command("fdfind").output());

        let mut used_fallback=false;

        if cancel.load(Ordering::Relaxed) { return; }

        let raw_output = match fd_output {
            Ok(out) if out.status.success() || !out.stdout.is_empty() => out.stdout,
            _ => {
                used_fallback=true;
                // fd not installed — fall back to system find
                let mut fallback = std::process::Command::new("find");
                fallback
                    .arg(&search_root)
                    .arg("-maxdepth").arg(max_depth);
                if matches!(filter, SearchFilter::Files) {
                    fallback.arg("-type").arg("f");
                } else if matches!(filter, SearchFilter::Folders) {
                    fallback.arg("-type").arg("d");
                }
                fallback.arg("-iname").arg(format!("*{}*", query));
                let fallback = fallback.output();
                match fallback {
                    Ok(out) => out.stdout,
                    Err(_) => return,
                }
            }
        };

        if cancel.load(Ordering::Relaxed) { return; }

        let result = String::from_utf8_lossy(&raw_output);
        let raw_paths: Vec<std::path::PathBuf> = result
            .lines()
            .filter(|l| !l.is_empty())
            .take(500)
            .map(std::path::PathBuf::from)
            .collect();

        let paths = if raw_paths.is_empty() {
            Vec::new()
        } else {
            let entries: Vec<(String, std::path::PathBuf)> = raw_paths
                .into_iter()
                .map(|path| {
                    let filename = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.to_string_lossy().into_owned());
                    (filename, path)
                })
                .collect();

            let mut finder = FuzzyFinder::new();
            finder.populate(entries.iter().map(|(filename, _)| filename.clone()).collect());

            let mut paths_by_name: std::collections::HashMap<
                String,
                std::collections::VecDeque<std::path::PathBuf>,
            > = std::collections::HashMap::new();
            for (filename, path) in entries {
                paths_by_name.entry(filename).or_default().push_back(path);
            }

            finder
                .query(&query)
                .into_iter()
                .filter_map(|filename| {
                    paths_by_name
                        .get_mut(&filename)
                        .and_then(|paths| paths.pop_front())
                })
                .collect()
        };

        let _ = tx.send(paths);

// 🔥 ADD THIS
if used_fallback {
    let _ = status_tx.send(
        "⚠ fd not installed — using slow fallback (install fd)".to_string()
    );
}
    });
}

/// Fetch the latest published version from crates.io in a background thread.
/// Sends Some(version_string) if the remote version is newer than the current build.
fn spawn_update_check(tx: smpsc::Sender<String>) {
    let current = env!("CARGO_PKG_VERSION").to_string();
    std::thread::spawn(move || {
        let url = "https://crates.io/api/v1/crates/rex-fm";
        let result = ureq::get(url)
            .set("User-Agent", &format!("rex-fm/{} update-checker", current))
            .call();
        if let Ok(resp) = result {
            if let Ok(json) = resp.into_json::<serde_json::Value>() {
                if let Some(latest) = json["crate"]["newest_version"].as_str() {
                    if latest != current {
                        let _ = tx.send(latest.to_string());
                    }
                }
            }
        }
    });
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    let (watcher_tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(watcher_tx, notify::Config::default()).unwrap();

    //start watching initial path
    watcher
        .watch(&app.left.path, RecursiveMode::NonRecursive)
        .unwrap();

    // Initial preview load
    app.refresh_preview();

    // Load changelog
    if let Ok(content) = std::fs::read_to_string(app::changelog_path()) {
        app.changelog_lines = content.lines().map(|s| s.to_string()).collect();
    } else {
        // Use embedded changelog as fallback (always available)
        app.changelog_lines = app::get_default_changelog();
    }

    // Show startup notification with version and changelog info
    let version = env!("CARGO_PKG_VERSION");
    app.set_status_timeout(format!("✨ rex-fm v{} — Press U for changelog, ? for help", version));

    let mut current_watch_path = app.left.path.clone();
    let mut needs_draw = true;

    // Async search channel
     let (status_tx, status_rx) = smpsc::channel::<String>();
    let (search_tx, search_rx) = smpsc::channel::<Vec<std::path::PathBuf>>();
    let mut search_cancel = Arc::new(AtomicBool::new(false));

    // Background update check — non-blocking, fires once at startup
    let (update_tx, update_rx) = smpsc::channel::<String>();
    spawn_update_check(update_tx);

    while !app.should_quit {
        // Update status message expiry (auto-clear old messages)
        app.update_status_expiry();

        if app.left.path != current_watch_path {
            watcher.unwatch(&current_watch_path).ok();

            //watch new
            watcher
                .watch(&app.left.path, RecursiveMode::NonRecursive)
                .ok();

            current_watch_path = app.left.path.clone();
        }

        // Receive async search results (non-blocking)
        if let Ok(results) = search_rx.try_recv() {
           // Helper: check if path or ANY parent component starts with .
           let is_hidden = |p: &std::path::PathBuf| {
               p.components().any(|c| {
                   if let std::path::Component::Normal(name) = c {
                       name.to_string_lossy().starts_with('.')
                   } else {
                       false
                   }
               })
           };
           
           app.search_results = results
    .into_iter()
    .filter(|p| match app.search_filter {
        SearchFilter::All => !is_hidden(p), // Hide system files by default
        SearchFilter::Folders => p.is_dir() && !is_hidden(p),
        SearchFilter::Files => p.is_file() && !is_hidden(p),
        SearchFilter::System => is_hidden(p),
    })
    .collect(); 
app.search_results.sort_by(|a, b| {
    match (a.is_dir(), b.is_dir()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.cmp(b),
    }
});
            app.search_cursor = 0;
            needs_draw = true;
        }

        // Receive update notification — auto-open the popup
        if let Ok(version) = update_rx.try_recv() {
            app.update_available = Some(version);
            app.show_update_popup = true; // show popup immediately
            needs_draw = true;
        }
        if let Ok(msg) = status_rx.try_recv() {
    // 🔥 only show fd warning once
    if msg.contains("fd not installed") {
        if !app.warned_no_fd {
            app.set_status_timeout(msg);
            app.warned_no_fd = true;
            needs_draw = true;
        }
    } else {
        // normal messages always show
        app.set_status_timeout(msg);
        needs_draw = true;
    }
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
            match event::read()? {
                Event::Resize(_, _) => {
                    needs_draw = true;
                }
                Event::Key(key) => {
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

                // ── Search mode: capture ALL keys before normal handling ──────
                if app.search_mode {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            search_cancel.store(true, Ordering::Relaxed);
                            app.search_mode = false;
                            app.search_query.clear();
                            app.search_results.clear();
                            app.search_filter=SearchFilter::All;
                        }
                        KeyCode::Enter => {
                            if let Some(path) = app.search_results.get(app.search_cursor).cloned() {
                                let target_dir = if path.is_dir() { path.clone() }
                                    else { path.parent().unwrap_or(&path).to_path_buf() };
                                app.left.path = target_dir;
                                app.left.refresh();
                                app.refresh_preview();
                                if let Some(idx) = app.left.entries.iter().position(|e| e == &path) {
                                    app.left.cursor = idx;
                                }
                                search_cancel.store(true, Ordering::Relaxed);
                                app.search_mode = false;
                                app.search_query.clear();
                                app.search_results.clear();
                            }
                        }
                        KeyCode::Down => {
                            if app.search_cursor + 1 < app.search_results.len() {
                                app.search_cursor += 1;
                            }
                        }
                        KeyCode::Up => {
                            if app.search_cursor > 0 { app.search_cursor -= 1; }
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.search_results.clear();
                            if !app.search_query.is_empty() {
                                search_cancel.store(true, Ordering::Relaxed);
                                search_cancel = Arc::new(AtomicBool::new(false));
                                let root = if app.global_search {
                                    match app.search_filter {
                                        SearchFilter::System => std::path::PathBuf::from("/"),
                                        _ => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
                                    }
                                } else {
                                    app.left.path.clone()
                                };
                                spawn_search(app.search_query.clone(), root, app.global_search, app.search_filter, search_tx.clone(), Arc::clone(&search_cancel),status_tx.clone(),);
                            }
                        }
                        

                         KeyCode::F(1) => {
    app.search_filter = SearchFilter::Folders;
      // 🔥 re-run search
    if !app.search_query.is_empty() {
        search_cancel.store(true, Ordering::Relaxed);
        search_cancel = Arc::new(AtomicBool::new(false));

        let root = if app.global_search {
            match app.search_filter {
                SearchFilter::System => std::path::PathBuf::from("/"),
                _ => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
            }
        } else {
            app.left.path.clone()
        };

        spawn_search(
            app.search_query.clone(),
            root,
            app.global_search,
            app.search_filter,
            search_tx.clone(),
            Arc::clone(&search_cancel),
            status_tx.clone(),
        );
    }
}

KeyCode::F(2) => {
    app.search_filter = SearchFilter::Files;
      // 🔥 re-run search
    if !app.search_query.is_empty() {
        search_cancel.store(true, Ordering::Relaxed);
        search_cancel = Arc::new(AtomicBool::new(false));

        let root = if app.global_search {
            match app.search_filter {
                SearchFilter::System => std::path::PathBuf::from("/"),
                _ => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
            }
        } else {
            app.left.path.clone()
        };

        spawn_search(
            app.search_query.clone(),
            root,
            app.global_search,
            app.search_filter,
            search_tx.clone(),
            Arc::clone(&search_cancel),
             status_tx.clone(), 
        );
    }
}

KeyCode::F(3) => {
    app.search_filter = SearchFilter::System;
      // 🔥 re-run search
    if !app.search_query.is_empty() {
        search_cancel.store(true, Ordering::Relaxed);
        search_cancel = Arc::new(AtomicBool::new(false));

        let root = if app.global_search {
            match app.search_filter {
                SearchFilter::System => std::path::PathBuf::from("/"),
                _ => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
            }
        } else {
            app.left.path.clone()
        };

        spawn_search(
            app.search_query.clone(),
            root,
            app.global_search,
            app.search_filter,
            search_tx.clone(),
            Arc::clone(&search_cancel),
             status_tx.clone(), 
        );
    }
}

KeyCode::F(4) => {
    app.search_filter = SearchFilter::All;
      // 🔥 re-run search
    if !app.search_query.is_empty() {
        search_cancel.store(true, Ordering::Relaxed);
        search_cancel = Arc::new(AtomicBool::new(false));

        let root = if app.global_search {
            match app.search_filter {
                SearchFilter::System => std::path::PathBuf::from("/"),
                _ => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
            }
        } else {
            app.left.path.clone()
        };

        spawn_search(
            app.search_query.clone(),
            root,
            app.global_search,
            app.search_filter,
            search_tx.clone(),
            Arc::clone(&search_cancel),
             status_tx.clone(), 
        );
    }
}


                       KeyCode::Char(c) => {

        app.search_query.push(c);

        search_cancel.store(true, Ordering::Relaxed);
        search_cancel = Arc::new(AtomicBool::new(false));

        let root = if app.global_search {
            match app.search_filter {
                SearchFilter::System => std::path::PathBuf::from("/"),
                _ => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
            }
        } else {
            app.left.path.clone()
        };

        spawn_search(
            app.search_query.clone(),
            root,
            app.global_search,
            app.search_filter,
            search_tx.clone(),
            Arc::clone(&search_cancel),
status_tx.clone(), 
        );
    }
 
                        _ => {}
                    }
                    needs_draw = true;
                    continue; // skip all normal key handling below
                }

                // ── Input mode: creating file/folder ────────────────────────
                if app.input_mode {
                    match key.code {
                        KeyCode::Esc => {
                            app.input_mode = false;
                            app.input_text.clear();
                        }
                        KeyCode::Backspace => {
                            app.input_text.pop();
                        }
                        KeyCode::Enter => {
                            if !app.input_text.is_empty() {
                                let mut path = app.left.path.clone();
                                path.push(&app.input_text);

                                // Check if already exists
                                if path.exists() {
                                    app.set_status_timeout(format!("❌ Already exists: {}", app.input_text));
                                } else {
                                    let result = if app.create_dir {
                                        std::fs::create_dir(&path)
                                    } else {
                                        std::fs::File::create(&path).map(|_| ())
                                    };

                                    match result {
                                        Ok(_) => {
                                            let item_type = if app.create_dir { "Folder" } else { "File" };
                                            app.set_status_timeout(format!("✅ Created {}: {}", item_type, app.input_text));
                                            app.left.refresh();
                                        }
                                        Err(e) => {
                                            app.set_status_timeout(format!("❌ Create failed: {}", e));
                                        }
                                    }
                                }
                            }
                            app.input_mode = false;
                            app.input_text.clear();
                        }
                        KeyCode::Char(c) => {
                            app.input_text.push(c);
                        }
                        _ => {}
                    }
                    needs_draw = true;
                    continue; // skip all normal key handling below
                }

                // ── Rename mode: capture input before normal handling ────────
                if app.rename_mode {
                    match key.code {
                        KeyCode::Esc => {
                            app.rename_mode = false;
                            app.input_buffer.clear();
                            app.rename_cursor = 0;
                        }
                        // Move caret left
                        KeyCode::Left => {
                            if app.rename_cursor > 0 {
                                app.rename_cursor -= 1;
                            }
                        }
                        // Move caret right
                        KeyCode::Right => {
                            if app.rename_cursor < app.input_buffer.chars().count() {
                                app.rename_cursor += 1;
                            }
                        }
                        // Jump to start / end
                        KeyCode::Home => { app.rename_cursor = 0; }
                        KeyCode::End  => { app.rename_cursor = app.input_buffer.chars().count(); }
                        // Delete char BEFORE caret
                        KeyCode::Backspace => {
                            if app.rename_cursor > 0 {
                                // find byte offset of the char before cursor
                                let byte_pos: usize = app.input_buffer
                                    .char_indices()
                                    .nth(app.rename_cursor - 1)
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                app.input_buffer.remove(byte_pos);
                                app.rename_cursor -= 1;
                            }
                        }
                        // Delete char AT caret
                        KeyCode::Delete => {
                            if app.rename_cursor < app.input_buffer.chars().count() {
                                let byte_pos: usize = app.input_buffer
                                    .char_indices()
                                    .nth(app.rename_cursor)
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                app.input_buffer.remove(byte_pos);
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(path) = app.left.entries.get(app.left.cursor).cloned() {
                                let is_parent = app.left.path.parent()
                                    .map_or(false, |p| path.as_path() == p);
                                if !is_parent && !app.input_buffer.is_empty() {
                                    let mut new_path = path.clone();
                                    new_path.set_file_name(&app.input_buffer);
                                    match std::fs::rename(&path, &new_path) {
                                        Ok(_) => {
                                            app.status_msg = Some(format!("Renamed → {}", app.input_buffer));
                                            app.left.refresh();
                                        }
                                        Err(e) => {
                                            app.status_msg = Some(format!("Rename failed: {}", e));
                                        }
                                    }
                                }
                            }
                            app.rename_mode = false;
                            app.input_buffer.clear();
                            app.rename_cursor = 0;
                        }
                        // Insert char at caret position
                        KeyCode::Char(c) => {
                            let byte_pos: usize = app.input_buffer
                                .char_indices()
                                .nth(app.rename_cursor)
                                .map(|(i, _)| i)
                                .unwrap_or_else(|| app.input_buffer.len());
                            app.input_buffer.insert(byte_pos, c);
                            app.rename_cursor += 1;
                        }
                        _ => {}
                    }
                    needs_draw = true;
                    continue;
                }

                // ── Update confirmation popup: Y / N ────────────────────────────
                if app.show_update_popup {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            app.show_update_popup = false;
                            disable_raw_mode().ok();
                            execute!(io::stdout(), LeaveAlternateScreen).ok();
                            eprintln!("Running: cargo install rex-fm --force");
                            let status = std::process::Command::new("cargo")
                                .args(["install", "rex-fm", "--force"])
                                .status();
                            execute!(io::stdout(), EnterAlternateScreen).ok();
                            enable_raw_mode().ok();
                            terminal.clear()?;
                            match status {
                                Ok(s) if s.success() => {
                                    app.status_msg = Some("✔ Updated! Restart rex to use the new version.".to_string());
                                    app.update_available = None;
                                }
                                _ => {
                                    app.status_msg = Some("✘ Update failed — run: cargo install rex-fm --force".to_string());
                                }
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            // Dismiss popup — badge stays in status bar, U reopens later
                            app.show_update_popup = false;
                        }
                        _ => {}
                    }
                    needs_draw = true;
                    continue;
                }

                // ── Help popup: Esc closes ───────────────────────────────────
                if app.show_help {
                    if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
                        app.show_help = false;
                    }
                    needs_draw = true;
                    continue;
                }

                // ── Changelog popup: Esc/q closes, j/k scrolls ──────────────
                if app.show_changelog {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.show_changelog = false;
                        }
                        KeyCode::Char('j') => {
                            let total_changelog_lines = app.changelog_lines.len();
                            if app.changelog_scroll + 1 < total_changelog_lines {
                                app.changelog_scroll += 1;
                            }
                        }
                        KeyCode::Char('k') => {
                            if app.changelog_scroll > 0 {
                                app.changelog_scroll -= 1;
                            }
                        }
                        _ => {}
                    }
                    needs_draw = true;
                    continue;
                }

                // ── Open-with popup: capture keys ───────────────────────────
                if app.open_with_mode {
                    let path = app.left.entries.get(app.left.cursor).cloned();
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.open_with_mode = false;
                        }
                        KeyCode::Char('j') => {
                            if app.open_with_cursor + 1 < app.open_with_options.len() {
                                app.open_with_cursor += 1;
                            }
                        }
                        KeyCode::Char('k') => {
                            if app.open_with_cursor > 0 { app.open_with_cursor -= 1; }
                        }
                        KeyCode::Enter => {
                            if let Some(path) = path {
                                let raw = app.open_with_options[app.open_with_cursor].clone();
                                // Strip "★ " prefix and " (configured)" suffix
                                let app_name = raw
                                    .trim_start_matches("★ ")
                                    .split(" (configured)")
                                    .next()
                                    .unwrap_or(&raw)
                                    .trim()
                                    .to_string();

                                // Save as new default for this extension
                                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
                                app.config.open_with.insert(ext.clone(), app_name.clone());
                                app::save_config(&app.config);

                                // Open the file
                                disable_raw_mode().ok();
                                execute!(io::stdout(), LeaveAlternateScreen).ok();
                                let _ = std::process::Command::new(&app_name).arg(&path).status();
                                execute!(io::stdout(), EnterAlternateScreen).ok();
                                enable_raw_mode().ok();
                                terminal.clear()?;

                                app.status_msg = Some(format!("Opened with {} (saved as default)", app_name));
                                app.open_with_mode = false;
                            }
                        }
                        _ => {}
                    }
                    needs_draw = true;
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char('?') => { app.show_help = true; }
                    KeyCode::Char('i') => { app.show_info = !app.show_info; }

                    // U — open changelog (or update popup if available)
                    KeyCode::Char('U') => {
                        if app.update_available.is_some() {
                            app.show_update_popup = true;
                        } else {
                            app.show_changelog = true;
                            app.changelog_scroll = 0;
                        }
                    }

                    // / — local search (current directory)
                    KeyCode::Char('/') => {
                        app.search_mode = true;
                        app.global_search = false;
                        app.search_query.clear();
                        app.search_results.clear();
                        app.search_cursor = 0;
                    }

                    // g — global search (entire filesystem from /)
                    KeyCode::Char('g') => {
                        app.search_mode = true;
                        app.global_search = true;
                        app.search_query.clear();
                        app.search_results.clear();
                        app.search_cursor = 0;
                    }

                    // o — open file with configured app or xdg-open
                    KeyCode::Char('o') => {
                        if let Some(path) = app.left.entries.get(app.left.cursor).cloned() {
                            if path.is_dir() {
                                // navigate into directory instead
                                app.left.enter();
                                app.left.refresh();
                                app.refresh_preview();
                            } else {
                                let ext = path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("")
                                    .to_string();
                                let app_name = app.config.open_with
                                    .get(&ext)
                                    .cloned()
                                    .unwrap_or_else(|| "xdg-open".to_string());

                                // Disable raw mode so terminal apps (nvim, etc.) get a proper tty
                                disable_raw_mode().ok();
                                execute!(io::stdout(), LeaveAlternateScreen).ok();

                                let _ = std::process::Command::new(&app_name)
                                    .arg(&path)
                                    .status();

                                execute!(io::stdout(), EnterAlternateScreen).ok();
                                enable_raw_mode().ok();
                                terminal.clear()?;

                                app.status_msg = Some(format!(
                                    "Opened {} with {}",
                                    path.file_name().unwrap_or_default().to_string_lossy(),
                                    app_name
                                ));
                            }
                        }
                    }

                    // O — show open-with popup
                    KeyCode::Char('O') => {
                        if let Some(path) = app.left.entries.get(app.left.cursor).cloned() {
                            if !path.is_dir() {
                                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

                                // Build option list — configured app first, then common apps
                                let mut opts: Vec<String> = Vec::new();
                                if let Some(configured) = app.config.open_with.get(&ext) {
                                    opts.push(format!("★ {} (configured)", configured));
                                }
                                // Common apps grouped by type
                                let ext_lc = ext.to_lowercase();
                                if ["rs","py","js","ts","c","cpp","h","toml","json","md","yaml","yml","sh","txt","html","css","lock"].contains(&ext_lc.as_str()) {
                                    for a in &["nvim", "vim", "nano", "code", "gedit"] { opts.push(a.to_string()); }
                                } else if ["png","jpg","jpeg","gif","bmp","webp","svg"].contains(&ext_lc.as_str()) {
                                    for a in &["eog", "feh", "gimp", "inkscape"] { opts.push(a.to_string()); }
                                } else if ["mp4","mkv","avi","mov","webm","mp3","wav","flac","ogg","aac"].contains(&ext_lc.as_str()) {
                                    for a in &["vlc", "mpv", "rhythmbox"] { opts.push(a.to_string()); }
                                } else if ["pdf","doc","docx","odt","xls","xlsx","ppt","pptx"].contains(&ext_lc.as_str()) {
                                    for a in &["libreoffice", "evince", "okular"] { opts.push(a.to_string()); }
                                }
                                // Always add fallbacks
                                for a in &["xdg-open", "nvim"] {
                                    if !opts.iter().any(|o| o == a) { opts.push(a.to_string()); }
                                }

                                app.open_with_options = opts;
                                app.open_with_cursor = 0;
                                app.open_with_mode = true;
                            }
                        }
                    }

                    KeyCode::Tab => {
                        app.active_pane = match app.active_pane {
                            Pane::Left => Pane::Right,
                            Pane::Right => Pane::Left,
                        }
                    }

                    KeyCode::Char('j') => {
                        if app.search_mode {
                            if app.search_cursor + 1 < app.search_results.len() {
                                app.search_cursor += 1;
                            }
                        } else {
                            match app.active_pane {
                                Pane::Left => { app.left.move_down(); }
                                Pane::Right => {
                                    if total_lines > 0 && app.preview_cursor < total_lines - 1 {
                                        app.preview_cursor += 1;
                                    }
                                    let vh = app.visible_height;
                                    app.clamp_scroll(total_lines, vh);
                                }
                            }
                        }
                    }

                    KeyCode::Char('k') => {
                        if app.search_mode {
                            if app.search_cursor > 0 { app.search_cursor -= 1; }
                        } else {
                            match app.active_pane {
                                Pane::Left => { app.left.move_up(); }
                                Pane::Right => {
                                    app.preview_cursor = app.preview_cursor.saturating_sub(1);
                                    let vh = app.visible_height;
                                    app.clamp_scroll(total_lines, vh);
                                }
                            }
                        }
                    }

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
                        if app.search_mode {
                            // Jump to result: navigate to its parent directory
                            if let Some(path) = app.search_results.get(app.search_cursor).cloned() {
                                let target_dir = if path.is_dir() {
                                    path.clone()
                                } else {
                                    path.parent().unwrap_or(&path).to_path_buf()
                                };
                                app.left.path = target_dir;
                                app.left.refresh();
                                app.refresh_preview();
                                // Position cursor on matched file
                                if let Some(idx) = app.left.entries.iter().position(|e| e == &path) {
                                    app.left.cursor = idx;
                                }
                                app.search_mode = false;
                                app.search_query.clear();
                                app.search_results.clear();
                            }
                        } else if !app.preview_mode {
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
                                                _from: src.clone(),
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
                                            _from: src.clone(),
                                            to: dest.clone(),
                                        });
                                    }
                                    Err(e) => eprintln!("Copy failed: {}", e),
                                }
                            }

                            app.left.refresh();
                        }
                    }

                    // r: rename file under cursor
                    KeyCode::Char('r') => {
                        if let Some(path) = app.left.entries.get(app.left.cursor) {
                            let is_parent = app.left.path.parent()
                                .map_or(false, |p| path.as_path() == p);
                            if !is_parent {
                                app.input_buffer = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                app.rename_cursor = app.input_buffer.chars().count(); // caret at end
                                app.rename_mode = true;
                                app.show_info = false; // close info if open
                            }
                        }
                    }

                    // n: create new file
                    KeyCode::Char('n') => {
                        app.input_mode = true;
                        app.input_text.clear();
                        app.create_dir = false; // file
                    }

                    // N: create new folder
                    KeyCode::Char('N') => {
                        app.input_mode = true;
                        app.input_text.clear();
                        app.create_dir = true; // folder
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
                        if app.show_info {
                            app.show_info = false;
                        } else if app.search_mode {
                            app.search_mode = false;
                            app.search_query.clear();
                            app.search_results.clear();
                        } else {
                            app.selected.clear();
                            app.clipboard = None;
                            app.cut_mode = false;
                        }
                    }

                    _ => {}
                }
                needs_draw = true;
                } // end Event::Key
                _ => {} // ignore mouse, focus, paste events
            } // end match event::read()
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}
