use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::{App, Pane};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical[0]);

    // ── Sync visible_height from the real layout every frame ─────────────────
    // panes[1].height includes the two border rows, so subtract 2.
    let visible_height = (panes[1].height as usize).saturating_sub(2);
    app.visible_height = visible_height;

    let left_title = match app.active_pane {
        Pane::Left => "Left Pane *",
        _ => "Left Pane",
    };

    let left_items: Vec<ListItem> = app
        .left
        .entries
        .iter()
        .map(|p| {
            let is_parent = app.left.path.parent().map_or(false, |parent| p == parent);

            let name = if is_parent {
                p.file_name()
                    .unwrap_or_else(|| p.as_os_str())
                    .to_string_lossy()
            } else {
                p.file_name().unwrap().to_string_lossy()
            };

            let icon = if is_parent { "⬆️" } else { get_icon(p) };
            let item = format!("{} {}", icon, name);

            if is_parent {
                ListItem::new(item).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else if p.is_dir() {
                ListItem::new(item).style(Style::default().fg(Color::Cyan))
            } else {
                ListItem::new(item)
            }
        })
        .collect();

    // ── Build preview items ───────────────────────────────────────────────────
    let preview_items: Vec<ListItem> =
        if let Some(path) = app.left.entries.get(app.left.cursor) {
            if path.is_dir() {
                match std::fs::read_dir(path) {
                    Ok(read) => read
                        .flatten()
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            let icon = get_icon(&e.path());
                            ListItem::new(format!("{} {}", icon, name))
                        })
                        .collect(),
                    Err(_) => vec![ListItem::new("Cannot read directory")],
                }
            } else {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let all_lines: Vec<&str> = content.lines().collect();
                        let total = all_lines.len();

                        // Clamp scroll/cursor every frame so a resize is
                        // automatically corrected before we render.
                        app.clamp_scroll(total, visible_height);

                        all_lines
                            .iter()
                            .skip(app.preview_scroll)
                            .take(visible_height)
                            .map(|line| {
                                ListItem::new(line.to_string())
                                    .style(Style::default().fg(Color::Gray))
                            })
                            .collect()
                    }
                    Err(_) => vec![ListItem::new("Binary or unreadable file")],
                }
            }
        } else {
            vec![ListItem::new("No file selected")]
        };

    let right_title = match app.active_pane {
        Pane::Right => "Preview *",
        _ => "Preview",
    };

    let left_block = Block::default()
        .title(left_title)
        .borders(Borders::ALL)
        .border_style(match app.active_pane {
            Pane::Left => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        });

    let right_block = Block::default()
        .title(right_title)
        .borders(Borders::ALL)
        .border_style(match app.active_pane {
            Pane::Right => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        });

    let left = List::new(left_items).block(left_block).highlight_style(
        Style::default()
            .bg(Color::Blue)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let right = List::new(preview_items).block(right_block).highlight_style(
        Style::default()
            .bg(Color::Blue)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let mut left_state = ListState::default();
    if matches!(app.active_pane, Pane::Left) {
        left_state.select(Some(app.left.cursor));
    } else {
        left_state.select(None);
    }

    frame.render_stateful_widget(left, panes[0], &mut left_state);

    let mut right_state = ListState::default();
    if matches!(app.active_pane, Pane::Right) {
        // The preview list is built from .skip(preview_scroll), so item 0 in
        // the list corresponds to line preview_scroll in the file.
        // We must pass a RELATIVE offset, not the absolute cursor index.
        let relative = app.preview_cursor.saturating_sub(app.preview_scroll);
        right_state.select(Some(relative));
    } else {
        right_state.select(None);
    }

    frame.render_stateful_widget(right, panes[1], &mut right_state);

    let status = Block::default()
        .style(Style::default().bg(Color::DarkGray))
        .title("rex | q = quit | Tab = switch pane");
    frame.render_widget(status, vertical[1]);
}

fn get_icon(path: &std::path::Path) -> &'static str {
    if path.is_dir() {
        return "📁";
    }

    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    {
        Some(ext) => match ext.as_str() {
            "rs" => "🦀",

            "c" => "🔵",
            "cpp" | "cc" | "cxx" => "🟣",
            "h" | "hpp" => "📘",
            "java" => "☕",
            "py" => "🐍",
            "js" => "🟡",
            "ts" => "🔷",

            "ipynb" => "📓",

            "xls" | "xlsx" => "📊",
            "doc" | "docx" => "📄",
            "ppt" | "pptx" => "📽️",

            "json" => "🧾",
            "toml" => "⚙️",
            "yaml" | "yml" => "📋",

            "md" => "📘",
            "txt" => "📄",
            "pdf" => "📕",

            "jpg" | "jpeg" | "png" | "gif" => "🖼️",
            "mp4" | "mkv" => "🎬",
            "mp3" | "wav" => "🎵",

            "zip" | "tar" | "gz" | "7z" => "📦",

            "exe" | "bin" | "sh" => "⚡",

            "html" | "htm" => "🌐",
            "css" => "🎨",
            "scss" | "sass" => "💅",

            _ => "📄",
        },
        None => "📄",
    }
}
