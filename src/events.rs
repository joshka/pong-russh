use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

pub fn poll() -> io::Result<Option<KeyEvent>> {
    if event::poll(Duration::from_millis(16))? {
        if let Event::Key(key) = event::read()? {
            return Ok(Some(key));
        }
    }
    Ok(None)
}

pub fn is_quit_key(key: KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('q'), KeyModifiers::CONTROL)
            | (KeyCode::Esc, KeyModifiers::NONE)
            | (KeyCode::Char('c'), KeyModifiers::CONTROL)
    )
}
