mod app;
mod fs;
mod ui;

use app::{App, Pane};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use std::io;

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    while !app.should_quit {
        terminal.draw(|frame| {
            ui::layout::draw(frame, &app);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => app.should_quit = true,

                KeyCode::Tab => {
                    app.active_pane = match app.active_pane {
                        Pane::Left => Pane::Right,
                        Pane::Right => Pane::Left,
                    }
                }
                KeyCode::Char('j') => match app.active_pane {
                    Pane::Left => app.left.move_down(),
                    Pane::Right => app.right.move_down(),
                },

                KeyCode::Char('k') => match app.active_pane {
                    Pane::Left => app.left.move_up(),
                    Pane::Right => app.right.move_up(),
                },

                KeyCode::Char('l') => match app.active_pane {
                    Pane::Left => app.left.enter(),
                    Pane::Right => app.right.enter(),
                },

                KeyCode::Char('h') => match app.active_pane {
                    Pane::Left => app.left.back(),
                    Pane::Right => app.right.back(),
                },

                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}
