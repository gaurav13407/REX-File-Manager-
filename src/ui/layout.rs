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

    // ── Status bar (✨ update badge shown when newer version detected) ───────────
    let default_hint = "rex | q:quit  Tab:pane  hjkl:nav  r:rename  i:info  d:delete  y:copy  p:paste  u:undo  /:search";

    if let Some(ref ver) = app.update_available {
        // Split the area: hint on left, badge on right
        let badge = format!(" ★ Update available: {}  Press U to install ", ver);

        // Render badge on top of the status bar in yellow
        let hint_block = Block::default()
            .style(if app.status_msg.is_some() {
                Style::default().bg(Color::DarkGray).fg(Color::Green)
            } else {
                Style::default().bg(Color::DarkGray)
            })
            .title(app.status_msg.as_deref().unwrap_or(default_hint));
        frame.render_widget(hint_block, vertical[1]);

        // Overlay the badge flush-right
        let badge_width = badge.len() as u16;
        if badge_width < vertical[1].width {
            let badge_rect = ratatui::layout::Rect {
                x: vertical[1].x + vertical[1].width - badge_width,
                y: vertical[1].y,
                width: badge_width,
                height: 1,
            };
            let badge_widget = Block::default()
                .style(Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD))
                .title(badge.as_str());
            frame.render_widget(badge_widget, badge_rect);
        }
    } else {
        let status_text = app.status_msg.as_deref().unwrap_or(default_hint);
        let status_style = if app.status_msg.is_some() {
            Style::default().bg(Color::DarkGray).fg(Color::Green)
        } else {
            Style::default().bg(Color::DarkGray)
        };
        let status = Block::default().style(status_style).title(status_text);
        frame.render_widget(status, vertical[1]);
    }

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

    // ── Update confirmation popup ────────────────────────────────────────────
    if app.show_update_popup {
        if let Some(ref ver) = app.update_available {
            let popup_area = centered_rect(54, 9, size);
            frame.render_widget(Clear, popup_area);

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("  A new version of "),
                    Span::styled("rex-fm", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(" is available!"),
                ]),
                Line::from(vec![
                    Span::raw("  Latest: "),
                    Span::styled(ver.as_str(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(format!("   (current: {})", env!("CARGO_PKG_VERSION"))),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  Update now?  "),
                    Span::styled("[Y]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" yes     "),
                    Span::styled("[N]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw(" later"),
                ]),
                Line::from(""),
                Line::from(Span::styled("  Tip: press U anytime to see this again.", Style::default().fg(Color::DarkGray))),
            ];

            let popup = Paragraph::new(text)
                .block(
                    Block::default()
                        .title(" 🚀 Update Available ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                )
                .wrap(Wrap { trim: false });

            frame.render_widget(popup, popup_area);
        }
    }

    // ── Rename popup ──────────────────────────────────────────────────────────
    if app.rename_mode {
        let popup_area = centered_rect(55, 7, size);
        frame.render_widget(Clear, popup_area);

        // Split buffer at caret to insert the block cursor glyph
        let before: String = app.input_buffer.chars().take(app.rename_cursor).collect();
        let after:  String = app.input_buffer.chars().skip(app.rename_cursor).collect();

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  New name: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(&before, Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(after.as_str(), Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ←/→", Style::default().fg(Color::Yellow)),
                Span::raw(" move  "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" confirm  "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(" cancel"),
            ]),
        ];

        let rename_popup = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" ✏  Rename ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(rename_popup, popup_area);
    }

    // ── Open-with popup ───────────────────────────────────────────────────────
    if app.open_with_mode {
        let height = (app.open_with_options.len() as u16 + 4).min(size.height - 4);
        let area = centered_rect(50, height, size);

        let items: Vec<ListItem> = app.open_with_options.iter().enumerate().map(|(i, opt)| {
            if i == app.open_with_cursor {
                ListItem::new(format!("  {}", opt))
                    .style(Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))
            } else {
                ListItem::new(format!("  {}", opt))
            }
        }).collect();

        let popup = List::new(items).block(
            Block::default()
                .title(" Open With — j/k:select  Enter:open  Esc:cancel ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

        frame.render_widget(Clear, area);
        frame.render_widget(popup, area);
    }

    // ── Help popup (?): all keybinds ─────────────────────────────────────────
    if app.show_help {
        let help_text = vec![
            Line::from(Span::styled("  Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from("  j / k        Move down / up"),
            Line::from("  h            Go to parent directory"),
            Line::from("  l / Enter    Enter directory"),
            Line::from("  Tab          Switch pane (file list ↔ preview)"),
            Line::from("  gg           Go to top   (in preview pane)"),
            Line::from(""),
            Line::from(Span::styled("  File Operations", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from("  o            Open file (uses config.json app)"),
            Line::from("  O            Open With popup (choose + save default)"),
            Line::from("  r            Rename file (pre-filled, Enter:confirm  Esc:cancel)"),
            Line::from("  i            File info popup (name/size/type/perms/modified/path)"),
            Line::from("  Space        Toggle select file"),
            Line::from("  A            Select all files"),
            Line::from("  Esc          Clear selection / cancel"),
            Line::from("  y            Copy  |  x  Cut  |  p  Paste"),
            Line::from("  d            Delete (trash)  — confirm with y"),
            Line::from("  u            Undo last operation"),
            Line::from(""),
            Line::from(Span::styled("  Search", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from("  /            Local search (current directory)"),
            Line::from("  g            Global search (from /)"),
            Line::from("  j / k        Navigate results"),
            Line::from("  Enter        Jump to result"),
            Line::from("  Esc / q      Exit search"),
            Line::from(""),
            Line::from(Span::styled("  General", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from("  ?            Toggle this help"),
            Line::from("  U            Install update (shown only when update is available)"),
            Line::from("  q            Quit"),
            Line::from(""),
            Line::from(Span::styled("  Press Esc or ? to close", Style::default().fg(Color::DarkGray))),
        ];

        let height = (help_text.len() as u16 + 2).min(size.height - 2);
        let help_area = centered_rect(62, height, size);

        let help_popup = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" ❓ rex — Help ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(Clear, help_area);
        frame.render_widget(help_popup, help_area);
    }

    // ── File Info popup (i) ──────────────────────────────────────────────────
    if app.show_info {
        use std::os::unix::fs::PermissionsExt;

        if let Some(path) = app.left.entries.get(app.left.cursor) {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            let (size_str, type_str, modified_str, perms_str) =
                if let Ok(meta) = std::fs::metadata(path) {
                    // Size
                    let bytes = meta.len();
                    let size_str = if meta.is_dir() {
                        "—".to_string()
                    } else if bytes < 1024 {
                        format!("{} B", bytes)
                    } else if bytes < 1024 * 1024 {
                        format!("{:.1} KB", bytes as f64 / 1024.0)
                    } else {
                        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
                    };
                    // Type
                    let type_str = if meta.is_dir() { "Directory" } else { "File" }.to_string();
                    // Modified
                    let modified_str = meta
                        .modified()
                        .ok()
                        .and_then(|t| {
                            t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| {
                                let secs = d.as_secs();
                                let s = secs % 60;
                                let m = (secs / 60) % 60;
                                let h = (secs / 3600) % 24;
                                let days = secs / 86400;
                                // Rough date: days since epoch
                                let y = 1970 + days / 365;
                                let remaining = days % 365;
                                let mo = remaining / 30 + 1;
                                let d2 = remaining % 30 + 1;
                                format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d2, h, m, s)
                            })
                        })
                        .unwrap_or_else(|| "Unknown".to_string());
                    // Permissions (Unix)
                    let mode = meta.permissions().mode();
                    let perms_str = format!(
                        "{}{}{}{}{}{}{}{}{}{}",
                        if meta.is_dir() { 'd' } else { '-' },
                        if mode & 0o400 != 0 { 'r' } else { '-' },
                        if mode & 0o200 != 0 { 'w' } else { '-' },
                        if mode & 0o100 != 0 { 'x' } else { '-' },
                        if mode & 0o040 != 0 { 'r' } else { '-' },
                        if mode & 0o020 != 0 { 'w' } else { '-' },
                        if mode & 0o010 != 0 { 'x' } else { '-' },
                        if mode & 0o004 != 0 { 'r' } else { '-' },
                        if mode & 0o002 != 0 { 'w' } else { '-' },
                        if mode & 0o001 != 0 { 'x' } else { '-' },
                    );
                    (size_str, type_str, modified_str, perms_str)
                } else {
                    ("?".into(), "?".into(), "?".into(), "?".into())
                };

            let full_path = path.to_string_lossy().to_string();
            let parent_path = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let label_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
            let info_text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Name     : ", label_style),
                    Span::raw(name.clone()),
                ]),
                Line::from(vec![
                    Span::styled("  Size     : ", label_style),
                    Span::raw(size_str),
                ]),
                Line::from(vec![
                    Span::styled("  Type     : ", label_style),
                    Span::raw(type_str),
                ]),
                Line::from(vec![
                    Span::styled("  Perms    : ", label_style),
                    Span::raw(perms_str),
                ]),
                Line::from(vec![
                    Span::styled("  Modified : ", label_style),
                    Span::raw(modified_str),
                ]),
                Line::from(vec![
                    Span::styled("  Dir      : ", label_style),
                    Span::raw(parent_path),
                ]),
                Line::from(vec![
                    Span::styled("  Path     : ", label_style),
                    Span::raw(full_path),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press i or Esc to close",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let height = (info_text.len() as u16 + 2).min(size.height - 2);
            let info_area = centered_rect(60, height, size);

            let info_popup = Paragraph::new(info_text)
                .block(
                    Block::default()
                        .title(" ℹ  File Info ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Magenta)),
                )
                .wrap(Wrap { trim: false });

            frame.render_widget(Clear, info_area);
            frame.render_widget(info_popup, info_area);
        }
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
