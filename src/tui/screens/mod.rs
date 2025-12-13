//! TUI screens module - Simplified implementation

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};
use std::fmt;

// Screen-local rendering helpers using ONLY ratatui standard components
mod render_helpers;

// Include the available screens
pub mod dashboard;
pub mod provider;
pub mod status;

// Re-export key screens
pub use dashboard::DashboardScreen;
pub use provider::ProviderScreen;
pub use status::StatusScreen;

// Screen navigation action
#[derive(Debug, Clone)]
pub enum ScreenAction {
    /// No action
    None,
    /// Switch to another screen
    SwitchTo(ScreenType),
    /// Go back to previous screen
    Back,
    /// Quit the application
    Quit,
}

// Available screen types - simplified
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScreenType {
    Dashboard,
    Status,
    Provider,
}

impl ScreenType {
    /// Create screen instance from type
    pub fn create(&self) -> Result<Box<dyn Screen>> {
        match self {
            ScreenType::Dashboard => Ok(Box::new(DashboardScreen::new()?)),
            ScreenType::Status => Ok(Box::new(StatusScreen::new()?)),
            ScreenType::Provider => Ok(Box::new(ProviderScreen::new()?)),
        }
    }
}

impl fmt::Display for ScreenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScreenType::Dashboard => write!(f, "Dashboard"),
            ScreenType::Status => write!(f, "System Status"),
            ScreenType::Provider => write!(f, "Provider Management"),
        }
    }
}

/// Trait for all TUI screens
pub trait Screen {
    /// Render the screen
    fn render(&mut self, frame: &mut Frame, area: Rect);

    /// Handle key input
    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction>;

    /// Update screen state (called on Tick event)
    fn update(&mut self) -> Result<()> {
        Ok(()) // Default implementation does nothing
    }
}
