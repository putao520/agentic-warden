//! Pull progress screen

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::{Screen, ScreenAction, ScreenType};
use crate::tui::widgets::{DialogResult, DialogWidget, ProgressWidget};

/// Pull step
#[derive(Debug, Clone, PartialEq)]
enum PullStep {
    Initializing,
    Downloading,
    Decompressing,
    Restoring,
    Completed,
    Failed(String),
}

/// Pull screen mode
enum PullMode {
    CheckingAuth,
    Ready,
    Running,
    NeedAuth(DialogWidget),
}

/// Pull progress screen
pub struct PullScreen {
    progress_widget: ProgressWidget,
    step: PullStep,
    current_file: Option<String>,
    downloaded_bytes: u64,
    total_bytes: u64,
    files_restored: usize,
    total_files: usize,
    mode: PullMode,
    started: bool,
}

impl PullScreen {
    pub fn new() -> Result<Self> {
        let progress_widget = ProgressWidget::new("Pulling from Google Drive".to_string());

        Ok(Self {
            progress_widget,
            step: PullStep::Initializing,
            current_file: None,
            downloaded_bytes: 0,
            total_bytes: 0,
            files_restored: 0,
            total_files: 0,
            mode: PullMode::CheckingAuth,
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

    /// Start the pull operation
    fn start_pull(&mut self) -> Result<()> {
        self.started = true;
        self.mode = PullMode::Running;
        self.step = PullStep::Initializing;
        // TODO: Spawn async task to perform actual pull
        // For now, just simulate completion
        self.step = PullStep::Completed;
        Ok(())
    }

    fn get_step_description(&self) -> String {
        match &self.step {
            PullStep::Initializing => "Initializing...".to_string(),
            PullStep::Downloading => {
                if let Some(file) = &self.current_file {
                    format!("Downloading: {}", file)
                } else {
                    "Downloading...".to_string()
                }
            }
            PullStep::Decompressing => "Decompressing archives...".to_string(),
            PullStep::Restoring => {
                format!(
                    "Restoring files... ({}/{})",
                    self.files_restored, self.total_files
                )
            }
            PullStep::Completed => "Pull completed successfully!".to_string(),
            PullStep::Failed(err) => format!("Failed: {}", err),
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
            PullStep::Initializing => 5,
            PullStep::Downloading => {
                if self.total_bytes > 0 {
                    let download_progress =
                        (self.downloaded_bytes as f64 / self.total_bytes as f64 * 40.0) as u16;
                    5 + download_progress
                } else {
                    25
                }
            }
            PullStep::Decompressing => 60,
            PullStep::Restoring => {
                if self.total_files > 0 {
                    let restore_progress =
                        (self.files_restored as f64 / self.total_files as f64 * 30.0) as u16;
                    60 + restore_progress
                } else {
                    75
                }
            }
            PullStep::Completed => 100,
            PullStep::Failed(_) => 0,
        }
    }
}

impl Screen for PullScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Handle auth dialog
        if let PullMode::NeedAuth(ref dialog) = self.mode {
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
            PullMode::CheckingAuth => "Checking Authentication...",
            PullMode::Ready => "Ready to Pull from Google Drive",
            PullMode::Running => "Pulling from Google Drive",
            PullMode::NeedAuth(_) => "Authentication Required",
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

        if self.total_bytes > 0 {
            content.push(Line::from(Span::styled(
                "Download:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            content.push(Line::from(format!(
                "  {} / {}",
                Self::format_bytes(self.downloaded_bytes),
                Self::format_bytes(self.total_bytes)
            )));
            content.push(Line::from(""));
        }

        if self.total_files > 0 {
            content.push(Line::from(Span::styled(
                "Restore Progress:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            content.push(Line::from(format!(
                "  {} / {} files",
                self.files_restored, self.total_files
            )));
        }

        let status_widget =
            Paragraph::new(content).block(Block::default().borders(Borders::ALL).title("Details"));
        frame.render_widget(status_widget, chunks[2]);

        // Help
        let help = match (&self.mode, &self.step) {
            (PullMode::Ready, _) => "[Enter] Start Pull  [ESC] Back",
            (PullMode::Running, PullStep::Completed) => "[Enter] Continue  [ESC] Back",
            (PullMode::Running, _) => "Pulling... [ESC] Cancel",
            _ => "[ESC] Back",
        };
        let help_widget = Paragraph::new(help)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help_widget, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        // Handle auth dialog
        if let PullMode::NeedAuth(ref mut dialog) = self.mode {
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
                    PullMode::Ready => {
                        // Start pull
                        if let Err(e) = self.start_pull() {
                            self.step = PullStep::Failed(e.to_string());
                        }
                        Ok(ScreenAction::None)
                    }
                    PullMode::Running if self.step == PullStep::Completed => Ok(ScreenAction::Back),
                    _ => Ok(ScreenAction::None),
                }
            }
            KeyCode::Esc => Ok(ScreenAction::Back),
            _ => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        // Check authentication on first update
        if matches!(self.mode, PullMode::CheckingAuth) {
            if self.check_auth() {
                self.mode = PullMode::Ready;
            } else {
                let dialog = DialogWidget::confirm(
                    "Authentication Required".to_string(),
                    "You need to authenticate with Google Drive first.\n\nAuthenticate now?"
                        .to_string(),
                );
                self.mode = PullMode::NeedAuth(dialog);
            }
        }

        // TODO: Update progress from async task
        // For now, progress is updated in start_pull()

        Ok(())
    }
}
