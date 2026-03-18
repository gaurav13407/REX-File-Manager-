use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use toml::to_string;

use crate::app::{App, Pane};

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical[0]);

    let left_title = match app.active_pane {
        Pane::Left => "Left Pane *",
        _ => "Left Pane",
    };

    let left_items: Vec<ListItem> = app
        .left
        .entries
        .iter()
        .map(|p| {
            let is_parent=app.left.path.parent().map_or(false, |parent| p==parent);

            let name=if is_parent{
                p.file_name()
                    .unwrap_or_else(|| p.as_os_str())
                    .to_string_lossy()
            }else{
                p.file_name().unwrap().to_string_lossy()
            };

            let icon=if is_parent{
                "⬆️"
            }else{
                get_icon(p)
            };

            let item=format!("{} {}",icon,name);

            if is_parent{
                ListItem::new(item)
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))

            } else if p.is_dir(){
                ListItem::new(item)
                    .style(Style::default().fg(Color::Cyan))
            }else{
                ListItem::new(item)
            }
        })
        .collect();

    let preview_items: Vec<ListItem> = if let Some(path) = app.left.entries.get(app.left.cursor) {
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
                Err(_) => vec![ListItem::new("Cannot read directroy")],
            }
        } else {
            match std::fs::read_to_string(path) {
                Ok(content) => content
                    .lines()
                    .take(20)
                    .map(|line| {
                        ListItem::new(line.to_string()).style(Style::default().fg(Color::Gray))
                    })
                    .collect(),

                Err(_) => vec![ListItem::new("Binary or unreadable file")],
            }
        }
    } else {
        vec![ListItem::new("No file selected")]
    };

    let left = List::new(left_items)
        .block(Block::default().title(left_title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

    let right =
        List::new(preview_items).block(Block::default().title("Preview").borders(Borders::ALL));

    let mut left_state = ListState::default();
    left_state.select(Some(app.left.cursor));

    frame.render_stateful_widget(left, panes[0], &mut left_state);
    frame.render_widget(right, panes[1]);

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
