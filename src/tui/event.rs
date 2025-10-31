//! Event handling for TUI

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// TUI events
#[derive(Debug, Clone)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Tick event for periodic updates
    Tick,
    /// Resize event
    Resize(u16, u16),
}

/// Event handler for TUI
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    /// Create new event handler
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Read next event with timeout
    pub fn next(&self) -> anyhow::Result<Event> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                CrosstermEvent::Key(key) => Ok(Event::Key(key)),
                CrosstermEvent::Resize(w, h) => Ok(Event::Resize(w, h)),
                _ => Ok(Event::Tick),
            }
        } else {
            Ok(Event::Tick)
        }
    }
}

/// Check if key matches a specific code
pub fn key_match(event: &KeyEvent, code: KeyCode) -> bool {
    event.code == code && event.modifiers == KeyModifiers::NONE
}

/// Check if key is Ctrl+C
pub fn is_ctrl_c(event: &KeyEvent) -> bool {
    event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL)
}

/// Check if key is Ctrl+S
pub fn is_ctrl_s(event: &KeyEvent) -> bool {
    event.code == KeyCode::Char('s') && event.modifiers.contains(KeyModifiers::CONTROL)
}
