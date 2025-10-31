//! Push progress screen

use super::{Screen, ScreenAction, ScreenType};
use crate::tui::widgets::{DialogResult, DialogWidget, ProgressWidget};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Push step
#[derive(Debug, Clone, PartialEq)]
enum PushStep {
    Initializing,
    Compressing,
    Uploading,
    Verifying,
    Completed,
    Failed(String),
}

/// Push screen mode
enum PushMode {
    CheckingAuth,
    Ready,
    Running,
    NeedAuth(DialogWidget),
}

/// Push progress screen
pub struct PushScreen {
    directories: Vec<String>,
    progress_widget: ProgressWidget,
    step: PushStep,
    current_file: Option<String>,
    uploaded_bytes: u64,
    total_bytes: u64,
    mode: PushMode,
    started: bool,
}

impl PushScreen {
    pub fn new(directories: Vec<String>) -> Result<Self> {
        let progress_widget = ProgressWidget::new("Pushing to Google Drive".to_string());

        Ok(Self {
            directories,
            progress_widget,
            step: PushStep::Initializing,
            current_file: None,
            uploaded_bytes: 0,
            total_bytes: 0,
            mode: PushMode::CheckingAuth,
            started: false,
        })
    }

    /// Check if authenticated
    fn check_auth(&self) -> bool {
        let auth_path = dirs::home_dir().map(|h| h.join(".agentic-warden").join("auth.json"));

        if let Some(path) = auth_path {
            path.exists()
        } else {
            false
        }
    }

    /// Start the push operation
    fn start_push(&mut self) -> Result<()> {
        self.started = true;
        self.mode = PushMode::Running;
        self.step = PushStep::Initializing;
        // TODO: Spawn async task to perform actual push
        // For now, just simulate completion
        self.step = PushStep::Completed;
        Ok(())
    }

    fn get_step_description(&self) -> String {
        match &self.step {
            PushStep::Initializing => "Initializing...".to_string(),
            PushStep::Compressing => "Compressing configuration files...".to_string(),
            PushStep::Uploading => {
                if let Some(file) = &self.current_file {
                    format!("Uploading: {}", file)
                } else {
                    "Uploading...".to_string()
                }
            }
            PushStep::Verifying => "Verifying upload...".to_string(),
            PushStep::Completed => "Push completed successfully!".to_string(),
            PushStep::Failed(err) => format!("Failed: {}", err),
        }
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }

    fn calculate_progress(&self) -> u16 {
        match self.step {
            PushStep::Initializing => 5,
            PushStep::Compressing => 25,
            PushStep::Uploading => {
                if self.total_bytes > 0 {
                    let upload_progress =
                        (self.uploaded_bytes as f64 / self.total_bytes as f64 * 50.0) as u16;
                    25 + upload_progress
                } else {
                    50
                }
            }
            PushStep::Verifying => 90,
            PushStep::Completed => 100,
            PushStep::Failed(_) => 0,
        }
    }
}

impl Screen for PushScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Handle auth dialog
        if let PushMode::NeedAuth(ref dialog) = self.mode {
            dialog.render(frame, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title_text = match self.mode {
            PushMode::CheckingAuth => "Checking Authentication...",
            PushMode::Ready => "Ready to Push to Google Drive",
            PushMode::Running => "Pushing to Google Drive",
            PushMode::NeedAuth(_) => "Authentication Required",
        };

        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Progress bar
        let progress = self.calculate_progress();
        self.progress_widget.set_progress(progress);
        self.progress_widget.render(frame, chunks[1]);

        // Status details
        let mut content = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Status:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("  {}", self.get_step_description())),
            Line::from(""),
        ];

        if !self.directories.is_empty() {
            content.push(Line::from(Span::styled(
                "Directories:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for dir in &self.directories {
                content.push(Line::from(format!("  • {}", dir)));
            }
            content.push(Line::from(""));
        }

        if self.total_bytes > 0 {
            content.push(Line::from(Span::styled(
                "Transfer:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            content.push(Line::from(format!(
                "  {} / {}",
                Self::format_bytes(self.uploaded_bytes),
                Self::format_bytes(self.total_bytes)
            )));
        }

        let status_widget =
            Paragraph::new(content).block(Block::default().borders(Borders::ALL).title("Details"));
        frame.render_widget(status_widget, chunks[2]);

        // Help
        let help = match (&self.mode, &self.step) {
            (PushMode::Ready, _) => "[Enter] Start Push  [ESC] Back",
            (PushMode::Running, PushStep::Completed) => "[Enter] Continue  [ESC] Back",
            (PushMode::Running, _) => "Pushing... [ESC] Cancel",
            _ => "[ESC] Back",
        };
        let help_widget = Paragraph::new(help)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help_widget, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        // Handle auth dialog
        if let PushMode::NeedAuth(ref mut dialog) = self.mode {
            match dialog.handle_key(key) {
                DialogResult::Confirmed => {
                    // User wants to authenticate
                    return Ok(ScreenAction::SwitchTo(ScreenType::OAuth));
                }
                DialogResult::Cancelled | DialogResult::Closed => {
                    // User cancelled, go back
                    return Ok(ScreenAction::Back);
                }
                DialogResult::None => return Ok(ScreenAction::None),
            }
        }

        match key.code {
            KeyCode::Enter => {
                match &self.mode {
                    PushMode::Ready => {
                        // Start push
                        if let Err(e) = self.start_push() {
                            self.step = PushStep::Failed(e.to_string());
                        }
                        Ok(ScreenAction::None)
                    }
                    PushMode::Running if self.step == PushStep::Completed => Ok(ScreenAction::Back),
                    _ => Ok(ScreenAction::None),
                }
            }
            KeyCode::Esc => Ok(ScreenAction::Back),
            _ => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        // Check authentication on first update
        if matches!(self.mode, PushMode::CheckingAuth) {
            if self.check_auth() {
                self.mode = PushMode::Ready;
            } else {
                let dialog = DialogWidget::confirm(
                    "Authentication Required".to_string(),
                    "You need to authenticate with Google Drive first.\n\nAuthenticate now?"
                        .to_string(),
                );
                self.mode = PushMode::NeedAuth(dialog);
            }
        }

        // TODO: Update progress from async task
        // For now, progress is updated in start_push()

        Ok(())
    }
}
