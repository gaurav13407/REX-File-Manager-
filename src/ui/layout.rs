use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
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
        .zip(app.left.entry_is_dir.iter())
        .map(|(p, &is_dir_cached)| {
            let is_parent = app
                .left
                .path
                .parent()
                .map_or(false, |parent| p.as_path() == parent);

            let name = if is_parent {
                p.file_name()
                    .unwrap_or_else(|| p.as_os_str())
                    .to_string_lossy()
            } else {
                p.file_name().unwrap().to_string_lossy()
            };

            // Use cached is_dir — zero stat syscalls
            let icon = if is_parent {
                "⬆️"
            } else {
                get_icon_cached(p, is_dir_cached)
            };
            let is_selected = app.selected.contains(p);
            let is_clipboard = app.clipboard.as_ref().map_or(false, |c| c == p);

            let prefix = if is_parent {
                ""
            } else if is_selected || (is_clipboard && app.cut_mode) {
                "[x] "
            } else if is_clipboard {
                "[y] "
            } else {
                ""
            };
            let item = format!("{}{} {}", prefix, icon, name);

            if is_parent {
                ListItem::new(item).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_dir_cached {
                ListItem::new(item).style(Style::default().fg(Color::Cyan))
            } else {
                ListItem::new(item)
            }
        })
        .collect();

    // ── Right pane: search results OR preview ────────────────────────────────
    if app.search_mode {
        // Build search result items
        let result_items: Vec<ListItem> = if app.search_results.is_empty() {
            vec![ListItem::new(if app.search_query.is_empty() {
                "Type to search…".to_string()
            } else {
                "No results".to_string()
            })]
        } else {
            app.search_results
                .iter()
                .map(|p| {
                    // Show path relative to current dir for readability
                    let display = p.strip_prefix(&app.left.path)
                        .unwrap_or(p)
                        .to_string_lossy()
                        .to_string();
                    let is_dir = p.to_string_lossy().ends_with('/') || p.is_dir();
                    let icon = if is_dir { "/" } else { "~" };
                    ListItem::new(format!("{} {}", icon, display))
                })
                .collect()
        };

        let search_block = Block::default()
            .title(format!(" {} Search: {}█ ", if app.global_search { "🌐 Global" } else { "📁 Local" }, app.search_query))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let search_list = List::new(result_items)
            .block(search_block)
            .highlight_style(
                Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD),
            );

        let left_block = Block::default()
            .title(left_title)
            .borders(Borders::ALL)
            .border_style(Style::default()); // dim left when searching

        let left = List::new(left_items).block(left_block).highlight_style(
            Style::default().bg(Color::Blue).fg(Color::Black).add_modifier(Modifier::BOLD),
        );
        let mut left_state = ListState::default();
        left_state.select(Some(app.left.cursor));
        frame.render_stateful_widget(left, panes[0], &mut left_state);

        let mut search_state = ListState::default();
        if !app.search_results.is_empty() {
            search_state.select(Some(app.search_cursor));
        }
        frame.render_stateful_widget(search_list, panes[1], &mut search_state);

        let status_bar = Block::default()
            .style(Style::default().bg(Color::Cyan).fg(Color::Black))
            .title(format!(" {} {} | j/k:nav  Enter:goto  Esc/q:cancel ",
                if app.global_search { "g (global /):" } else { "/ (local):" },
                app.search_query
            ));
        frame.render_widget(status_bar, vertical[1]);
        return;
    }

    // ── Normal mode: preview ─────────────────────────────────────────────────
    let preview_items: Vec<ListItem> = if app.preview_content.is_empty() {
        vec![ListItem::new("No preview")]
    } else {
        let total = app.preview_content.len();
        app.clamp_scroll(total, visible_height);
        app.preview_content
            .iter()
            .skip(app.preview_scroll)
            .take(visible_height)
            .map(|line| ListItem::new(line.clone()).style(Style::default().fg(Color::Gray)))
            .collect()
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

    let status_text = app
        .status_msg
        .as_deref()
        .unwrap_or("rex | q:quit  Tab:pane  hjkl:nav  d:delete  y:copy  p:paste  u:undo  /:search");

    let status_style = if app.status_msg.is_some() {
        Style::default().bg(Color::DarkGray).fg(Color::Green)
    } else {
        Style::default().bg(Color::DarkGray)
    };

    let status = Block::default().style(status_style).title(status_text);
    frame.render_widget(status, vertical[1]);

    // ── Delete confirmation popup ─────────────────────────────────────────────
    if app.confirm_delete {
        // Show selected count or cursor file name
        let delete_label: String = if !app.selected.is_empty() {
            format!("{} selected item(s)", app.selected.len())
        } else {
            app.left
                .entries
                .get(app.left.cursor)
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "this item".into())
        };

        let popup_area = centered_rect(50, 7, size);
        frame.render_widget(Clear, popup_area);

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Delete  "),
                Span::styled(
                    &delete_label,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("?"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  [y] ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("confirm    "),
                Span::styled(
                    "[n / Esc] ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw("cancel"),
            ]),
            Line::from(""),
        ];

        let popup = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" ⚠ Confirm Delete ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(popup, popup_area);
    }
}

/// Returns a centered `Rect` of fixed height (rows) and percentage width.
fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let x = r.x + (r.width.saturating_sub(popup_width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: popup_width.min(r.width),
        height: height.min(r.height),
    }
}

/// Icon lookup using a pre-cached is_dir value — zero filesystem calls.
pub fn get_icon_cached(path: &std::path::Path, is_dir: bool) -> &'static str {
    if is_dir {
        return "📁";
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("rs") => "🦀",
        Some("toml") | Some("yaml") | Some("yml") | Some("json") => "⚙️",
        Some("md") | Some("txt") => "📄",
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") => "🖼️",
        Some("mp4") | Some("mkv") | Some("avi") | Some("mov") => "🎬",
        Some("mp3") | Some("wav") | Some("flac") => "🎵",
        Some("zip") | Some("tar") | Some("gz") | Some("xz") | Some("7z") => "📦",
        Some("sh") | Some("bash") | Some("zsh") => "🐚",
        Some("py") => "🐍",
        Some("js") | Some("ts") => "📜",
        Some("html") | Some("htm") => "🌐",
        Some("css") => "🎨",
        _ => "📄",
    }
}

pub fn get_icon(path: &std::path::Path) -> &'static str {
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
