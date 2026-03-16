use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Frame,
};

use crate::app::{App, Pane};

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.size();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(size);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(vertical[0]);

    let left_title = match app.active_pane {
        Pane::Left => "Left Pane *",
        _ => "Left Pane",
    };

    let right_title = match app.active_pane {
        Pane::Right => "Right Pane *",
        _ => "Right Pane",
    };

    let left = Block::default()
        .title(left_title)
        .borders(Borders::ALL);

    let right = Block::default()
        .title(right_title)
        .borders(Borders::ALL);

    frame.render_widget(left, panes[0]);
    frame.render_widget(right, panes[1]);

    let status = Block::default().title("rex | q = quit | Tab = switch pane");
    frame.render_widget(status, vertical[1]);
}
