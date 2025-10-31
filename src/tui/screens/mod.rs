//! TUI screens module

pub mod dashboard;
pub mod oauth;
pub mod provider;
pub mod provider_add_wizard;
pub mod provider_edit;
pub mod pull;
pub mod push;
pub mod status;

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub use dashboard::DashboardScreen;
pub use oauth::OAuthScreen;
pub use provider::ProviderScreen;
pub use provider_add_wizard::ProviderAddWizard;
pub use provider_edit::ProviderEditScreen;
pub use pull::PullScreen;
pub use push::PushScreen;
pub use status::StatusScreen;

/// Screen navigation action
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

/// Available screen types
#[derive(Debug, Clone)]
pub enum ScreenType {
    Dashboard,
    Provider,
    ProviderAddWizard,    // New provider setup wizard
    ProviderEdit(String), // Edit specific provider
    Status,
    OAuth,
    Push(Vec<String>), // Directories to push
    Pull,
}

impl ScreenType {
    /// Create screen instance from type
    pub fn create(&self) -> Result<Box<dyn Screen>> {
        match self {
            ScreenType::Dashboard => Ok(Box::new(DashboardScreen::new()?)),
            ScreenType::Provider => Ok(Box::new(ProviderScreen::new()?)),
            ScreenType::ProviderAddWizard => Ok(Box::new(ProviderAddWizard::new()?)),
            ScreenType::ProviderEdit(name) => Ok(Box::new(ProviderEditScreen::new(name.clone())?)),
            ScreenType::Status => Ok(Box::new(StatusScreen::new()?)),
            ScreenType::OAuth => Ok(Box::new(OAuthScreen::new()?)),
            ScreenType::Push(dirs) => Ok(Box::new(PushScreen::new(dirs.clone())?)),
            ScreenType::Pull => Ok(Box::new(PullScreen::new()?)),
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
    fn update(&mut self) -> Result<()>;
}
