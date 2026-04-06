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
            ui::layout::draw(frame, &mut app);
        })?;

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
                        }
                    }
                }

                KeyCode::Char('h') => {
                    if !app.preview_mode {
                        if let Pane::Left = app.active_pane {
                            app.left.back();
                        }
                    }
                }

                KeyCode::Enter => {
                    if !app.preview_mode {
                        if let Pane::Left = app.active_pane {
                            app.left.enter();
                        }
                    }
                }

                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}
