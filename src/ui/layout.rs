use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem,ListState},
    style::{Style,Modifier,Color},
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
            let name = if let Some(parent)=app.left.path.parent(){
                if p==parent{
                    "..".into()
                }else{
                    p.file_name().unwrap().to_string_lossy()
                }
            }else{
                p.file_name().unwrap().to_string_lossy()
            };

            if p.is_dir(){
                ListItem::new(format!("📁 {}",name))
                    .style(Style::default().fg(Color::Cyan))
            }else{
                ListItem::new(format!("📄 {}",name))
                    .style(Style::default().fg(Color::White))
            }
        })
        .collect();

    let preview_items:Vec<ListItem>=if let Some(path)=app.left.entries.get(app.left.cursor){
        if path.is_dir(){
            match std::fs::read_dir(path){
                Ok(read)=>read
                    .flatten()
                    .map(|e|{
                        let name=e.file_name().to_string_lossy().to_string();
                        ListItem::new(name)
                    })
                .collect(),
                Err(_)=>vec![ListItem::new("Cannot read directroy")],
            }
        } else{
            match std::fs::read_to_string(path){
                Ok(content)=>content
                    .lines()
                    .take(20)
                    .map(|line| ListItem::new(line.to_string())
                        .style(Style::default().fg(Color::Gray)))
                    .collect(),

                    Err(_)=>vec![ListItem::new("Binary or unreadable file")],
            }
        }
    } else{
        vec![ListItem::new("No file selected")]
    };

    let left =
        List::new(left_items).block(Block::default().title(left_title).borders(Borders::ALL))
        .highlight_style(Style::default()
            .bg(Color::Blue)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD));

    let right =List::new(preview_items)
        .block(Block::default().title("Preview").borders(Borders::ALL));

    let mut left_state=ListState::default();
    left_state.select(Some(app.left.cursor));


    frame.render_stateful_widget(left, panes[0], &mut left_state);
    frame.render_widget(right, panes[1]);

    let status = Block::default()
        .style(Style::default().bg(Color::DarkGray))
        .title("rex | q = quit | Tab = switch pane");
    frame.render_widget(status, vertical[1]);
}
