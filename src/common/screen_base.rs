//! Base screen functionality to reduce code duplication across TUI screens

use crate::tui::app_state::AppState;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::fmt;

/// Common screen actions that most screens can handle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseScreenAction {
    None,
    Back,
    Quit,
    Refresh,
    Help,
}

/// Common screen state patterns used across multiple screens
#[derive(Debug, Clone)]
pub enum ScreenMode {
    /// Screen is ready for user interaction
    Ready,
    /// Screen is processing something in background
    Processing,
    /// Screen operation completed successfully
    Completed,
    /// Screen operation failed
    Failed(String),
    /// Screen operation was cancelled by user
    Cancelled,
    /// Screen requires user authentication
    NeedAuth,
    /// Screen is loading data
    Loading,
}

impl fmt::Display for ScreenMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScreenMode::Ready => write!(f, "Ready"),
            ScreenMode::Processing => write!(f, "Processing..."),
            ScreenMode::Completed => write!(f, "Completed"),
            ScreenMode::Failed(msg) => write!(f, "Error: {}", msg),
            ScreenMode::Cancelled => write!(f, "Cancelled"),
            ScreenMode::NeedAuth => write!(f, "Authentication Required"),
            ScreenMode::Loading => write!(f, "Loading..."),
        }
    }
}

/// Base functionality that can be shared across screens
pub trait ScreenBase {
    /// Get the current screen mode
    fn mode(&self) -> &ScreenMode;

    /// Set the screen mode
    fn set_mode(&mut self, mode: ScreenMode);

    /// Check if the screen is in a terminal state (completed, failed, or cancelled)
    fn is_terminal(&self) -> bool {
        matches!(
            self.mode(),
            ScreenMode::Completed | ScreenMode::Failed(_) | ScreenMode::Cancelled
        )
    }

    /// Check if the screen is in an active state
    fn is_active(&self) -> bool {
        !self.is_terminal() && !matches!(self.mode(), ScreenMode::NeedAuth)
    }

    /// Handle common key events that most screens should respond to
    fn handle_common_key(&mut self, key: KeyEvent) -> BaseScreenAction {
        match key.code {
            KeyCode::Esc => BaseScreenAction::Back,
            KeyCode::Char('q') => BaseScreenAction::Quit,
            KeyCode::F(5) => BaseScreenAction::Refresh,
            KeyCode::F(1) => BaseScreenAction::Help,
            _ => BaseScreenAction::None,
        }
    }

    /// Get access to the global app state
    fn app_state(&self) -> &AppState;

    /// Render a common status bar for the screen
    fn render_status_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            style::{Color, Style},
            text::Span,
            widgets::{Block, Borders, Paragraph},
        };

        let status_text = match self.mode() {
            ScreenMode::Ready => Span::styled("Ready", Style::default().fg(Color::Green)),
            ScreenMode::Processing => {
                Span::styled("Processing...", Style::default().fg(Color::Yellow))
            }
            ScreenMode::Completed => Span::styled("Completed", Style::default().fg(Color::Green)),
            ScreenMode::Failed(msg) => {
                Span::styled(format!("Error: {}", msg), Style::default().fg(Color::Red))
            }
            ScreenMode::Cancelled => Span::styled("Cancelled", Style::default().fg(Color::Yellow)),
            ScreenMode::NeedAuth => Span::styled("Auth Required", Style::default().fg(Color::Red)),
            ScreenMode::Loading => Span::styled("Loading...", Style::default().fg(Color::Cyan)),
        };

        let block = Block::default().borders(Borders::ALL).title("Status");

        let paragraph = Paragraph::new(status_text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Render common help text at the bottom of the screen
    fn render_help_text(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            style::{Color, Style},
            text::{Line, Span},
            widgets::Paragraph,
        };

        let help_text = vec![Line::from(vec![
            Span::styled("ESC", Style::default().fg(Color::Cyan)),
            Span::raw(": Back "),
            Span::styled("Q", Style::default().fg(Color::Cyan)),
            Span::raw(": Quit "),
            Span::styled("F5", Style::default().fg(Color::Cyan)),
            Span::raw(": Refresh "),
            Span::styled("F1", Style::default().fg(Color::Cyan)),
            Span::raw(": Help"),
        ])];

        let paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
    }
}

/// Helper trait for screens that need to track authentication state
pub trait AuthAwareScreen: ScreenBase {
    /// Check if authentication is needed for the current operation
    fn needs_auth(&self) -> bool {
        matches!(self.mode(), ScreenMode::NeedAuth)
    }

    /// Mark that authentication has been completed
    fn set_auth_completed(&mut self) {
        if self.needs_auth() {
            self.set_mode(ScreenMode::Ready);
        }
    }

    /// Mark that authentication has failed
    fn set_auth_failed(&mut self, error: String) {
        self.set_mode(ScreenMode::Failed(error));
    }
}

/// Helper trait for screens that handle async operations
pub trait AsyncScreen: ScreenBase {
    /// Mark that an async operation has started
    fn start_processing(&mut self) {
        self.set_mode(ScreenMode::Processing);
    }

    /// Mark that an async operation has completed successfully
    fn operation_completed(&mut self) {
        self.set_mode(ScreenMode::Completed);
    }

    /// Mark that an async operation has failed
    fn operation_failed(&mut self, error: String) {
        self.set_mode(ScreenMode::Failed(error));
    }

    /// Mark that an async operation was cancelled
    fn operation_cancelled(&mut self) {
        self.set_mode(ScreenMode::Cancelled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    struct MockScreen {
        mode: ScreenMode,
        app_state: AppState,
    }

    impl ScreenBase for MockScreen {
        fn mode(&self) -> &ScreenMode {
            &self.mode
        }

        fn set_mode(&mut self, mode: ScreenMode) {
            self.mode = mode;
        }

        fn app_state(&self) -> &AppState {
            &self.app_state
        }
    }

    #[test]
    fn test_screen_mode_display() {
        assert_eq!(ScreenMode::Ready.to_string(), "Ready");
        assert_eq!(ScreenMode::Processing.to_string(), "Processing...");
        assert_eq!(
            ScreenMode::Failed("test".to_string()).to_string(),
            "Error: test"
        );
    }

    #[test]
    fn test_screen_base() {
        // Since AppState requires complex initialization, we'll skip the full test
        // and just test the basic functionality we can

        let mut mode = ScreenMode::Ready;
        assert_eq!(mode.to_string(), "Ready");

        mode = ScreenMode::Processing;
        assert_eq!(mode.to_string(), "Processing...");

        mode = ScreenMode::Failed("test".to_string());
        assert_eq!(mode.to_string(), "Error: test");
    }
}
