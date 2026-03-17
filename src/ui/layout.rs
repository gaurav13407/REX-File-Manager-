use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem,ListState},
    style::{Style,Modifier},
    Frame,
};

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

    let right_title = match app.active_pane {
        Pane::Right => "Right Pane *",
        _ => "Right Pane",
    };
    let left_items: Vec<ListItem> = app
        .left
        .entries
        .iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            ListItem::new(name)
        })
        .collect();

    let right_items: Vec<ListItem> = app
        .right
        .entries
        .iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            ListItem::new(name)
        })
        .collect();

    let left =
        List::new(left_items).block(Block::default().title(left_title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let right =
        List::new(right_items).block(Block::default().title(right_title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut left_state=ListState::default();
    left_state.select(Some(app.left.cursor));

    let mut right_state=ListState::default();
    right_state.select(Some(app.right.cursor));

    frame.render_stateful_widget(left, panes[0], &mut left_state);
    frame.render_stateful_widget(right, panes[1], &mut right_state);

    let status = Block::default().title("rex | q = quit | Tab = switch pane");
    frame.render_widget(status, vertical[1]);
}
