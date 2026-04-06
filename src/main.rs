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
            let total_lines = if let Some(path) = app.left.entries.get(app.left.cursor) {
                if path.is_file() {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        content.lines().count()
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            };
            match key.code {
                KeyCode::Char('q') => app.should_quit = true,

                KeyCode::Tab => {
                    app.active_pane = match app.active_pane {
                        Pane::Left => Pane::Right,
                        Pane::Right => Pane::Left,
                    }
                }

                KeyCode::Char('j') => {
                    match app.active_pane {
                        Pane::Left => {
                            app.left.move_down();
                            app.preview_scroll = 0;
                            app.preview_cursor = 0;
                        }

                        Pane::Right => {
                            if total_lines > 0 && app.preview_cursor < total_lines - 1 {
                                app.preview_cursor += 1;
                            }

                            let visible_height = app.visible_height; 
                            if app.preview_cursor >= app.preview_scroll + visible_height {
                                app.preview_scroll += 1;
                            }
                            if app.preview_scroll + visible_height > total_lines {
                                app.preview_scroll = total_lines.saturating_sub(visible_height);
                            }
                             let max_scroll = total_lines.saturating_sub(visible_height);

if app.preview_scroll > max_scroll {
    app.preview_scroll = max_scroll;
}
                        }
                    }
                }

                KeyCode::Char('k') => match app.active_pane {
                    Pane::Left => {
                        app.left.move_up();
                        app.preview_scroll = 0;
                        app.preview_cursor = 0;
                    }

                    Pane::Right => {
                        if app.preview_cursor > 0 {
                            app.preview_cursor -= 1;
                        }

                        if app.preview_cursor < app.preview_scroll {
                            if app.preview_scroll > 0 {
                                app.preview_scroll -= 1;
                            }
                        }
                    }
                },

                KeyCode::Char('l') => {
                    if !app.preview_mode {
                        match app.active_pane {
                            Pane::Left => app.left.enter(),
                            Pane::Right => {}
                        }
                    }
                }

                KeyCode::Char('h') => {
                    if !app.preview_mode {
                        match app.active_pane {
                            Pane::Left => app.left.back(),
                            Pane::Right => {}
                        }
                    }
                }

                KeyCode::Enter => {
                    if !app.preview_mode {
                        match app.active_pane {
                            Pane::Left => app.left.enter(),
                            Pane::Right => {}
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
