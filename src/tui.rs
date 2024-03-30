use std::io::{self, stdout, Stdout};

use crossterm::{execute, terminal::*};
use ratatui::prelude::*;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> io::Result<Tui> {
    let backend = CrosstermBackend::new(stdout());
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Terminal::new(backend)
}

pub fn restore() -> io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
