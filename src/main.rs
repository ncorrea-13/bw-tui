mod app;
mod bw;
mod clipboard;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::time::Duration;

const TICK: Duration = Duration::from_millis(250);

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    let mut app = App::new();
    app.start();

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(TICK)?
            && let Event::Key(key) = event::read()?
                && key.kind == event::KeyEventKind::Press {
                    app.handle_key(key);
                }
        app.on_tick();

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
