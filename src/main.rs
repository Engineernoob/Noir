mod app;
mod editor;
mod file_tree;
mod ui;
mod util;

use std::{
    env,
    io::{self, Stdout},
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::{Action, App};

fn main() -> Result<()> {
    let root = env::args().nth(1).unwrap_or_else(|| ".".to_string());

    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, App::new(root)?);
    restore_terminal(&mut terminal)?;

    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let action = app.handle_key_event(key)?;
                    if matches!(action, Action::Quit) {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
