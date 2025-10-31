//! OAuth authentication screen

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use tokio::runtime::Runtime;

use super::{Screen, ScreenAction};
use crate::sync::oauth_client::{OAuthClient, OAuthConfig};
use crate::tui::widgets::{DialogWidget, InputWidget};

/// OAuth screen mode
enum OAuthMode {
    Instructions,
    ManualInput,
    Processing,
    Success,
    Error(String),
}

/// OAuth authentication result
#[derive(Debug)]
pub enum OAuthResult {
    Success,
    Cancelled,
    Error(String),
}

/// OAuth authentication screen
pub struct OAuthScreen {
    mode: OAuthMode,
    auth_url: String,
    input_widget: InputWidget,
    runtime: Option<Runtime>,
    auth_code: Option<String>,
}

impl OAuthScreen {
    pub fn new() -> Result<Self> {
        Ok(Self {
            mode: OAuthMode::Instructions,
            auth_url: String::new(),
            input_widget: InputWidget::new("Authorization Code".to_string()),
            runtime: None,
            auth_code: None,
        })
    }

    /// Start OAuth flow
    fn start_oauth(&mut self) -> Result<()> {
        // Create runtime if not exists
        if self.runtime.is_none() {
            self.runtime = Some(Runtime::new()?);
        }

        // Generate OAuth URL
        let config = OAuthConfig::default();
        let client = OAuthClient::new(config.client_id, config.client_secret, None);

        let auth_url = client.generate_auth_url()?;
        self.auth_url = auth_url.clone();

        // Try to open browser
        let _ = open::that(&auth_url);

        // Switch to manual input mode
        self.mode = OAuthMode::ManualInput;
        self.input_widget.set_focused(true);

        Ok(())
    }

    /// Exchange code for token
    fn exchange_code(&mut self, code: String) -> Result<()> {
        if let Some(runtime) = &self.runtime {
            self.mode = OAuthMode::Processing;
            self.auth_code = Some(code.clone());

            let config = OAuthConfig::default();
            let mut client = OAuthClient::new(config.client_id, config.client_secret, None);

            // Exchange code for tokens
            match runtime.block_on(client.exchange_code_for_tokens(&code)) {
                Ok(token_response) => {
                    // Save tokens
                    let auth_dir = dirs::home_dir()
                        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?
                        .join(".agentic-warden");

                    std::fs::create_dir_all(&auth_dir)?;
                    let auth_file = auth_dir.join("auth.json");

                    // Create config with tokens
                    let mut save_config = OAuthConfig::default();
                    save_config.access_token = Some(token_response.access_token);
                    save_config.refresh_token = token_response.refresh_token;
                    save_config.expires_in = token_response.expires_in;
                    save_config.token_type = token_response.token_type;

                    // Save to file
                    let config_json = serde_json::to_string_pretty(&save_config)?;
                    std::fs::write(&auth_file, config_json)?;

                    self.mode = OAuthMode::Success;
                }
                Err(e) => {
                    self.mode = OAuthMode::Error(format!("Failed to exchange code: {}", e));
                }
            }
        }

        Ok(())
    }
}

impl Screen for OAuthScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            OAuthMode::Instructions => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                // Title
                let title = Paragraph::new("Google Drive Authentication")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                // Content
                let content = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "⚠️  Authentication Required",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(
                        "You need to authenticate with Google Drive to use push/pull features.",
                    ),
                    Line::from(""),
                    Line::from(Span::styled(
                        "This will:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from("  • Store credentials in ~/.agentic-warden/auth.json"),
                    Line::from("  • Allow backup/sync of your configuration files"),
                    Line::from("  • Open your browser for Google authentication"),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Note: Full TUI OAuth is under development.",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(Span::styled(
                        "Please use the CLI command instead:",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(Span::styled(
                        "  agentic-warden oauth",
                        Style::default().fg(Color::Cyan),
                    )),
                ];

                let content_widget = Paragraph::new(content)
                    .wrap(Wrap { trim: true })
                    .block(Block::default().borders(Borders::ALL).title("Instructions"));
                frame.render_widget(content_widget, chunks[1]);

                // Help
                let help = Paragraph::new("[S/Enter] Start OAuth  [ESC] Back")
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
            }
            OAuthMode::ManualInput => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(5),
                        Constraint::Min(5),
                        Constraint::Length(3),
                    ])
                    .split(area);

                let title = Paragraph::new("Enter Authorization Code")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                let url_display = Paragraph::new(format!("URL: {}", self.auth_url))
                    .wrap(Wrap { trim: true })
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Authorization URL"),
                    );
                frame.render_widget(url_display, chunks[1]);

                self.input_widget.render(frame, chunks[2]);

                let help = Paragraph::new("[Enter] Submit  [ESC] Cancel")
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[3]);
            }
            OAuthMode::Processing => {
                let dialog = DialogWidget::info(
                    "Processing".to_string(),
                    "Exchanging authorization code for tokens...".to_string(),
                );
                dialog.render(frame, area);
            }
            OAuthMode::Success => {
                let dialog = DialogWidget::info(
                    "Success".to_string(),
                    "Authentication successful! You can now use push/pull features.".to_string(),
                );
                dialog.render(frame, area);
            }
            OAuthMode::Error(msg) => {
                let dialog = DialogWidget::error("Error".to_string(), msg.clone());
                dialog.render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match &mut self.mode {
            OAuthMode::Instructions => match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Enter => {
                    // Start OAuth flow
                    if let Err(e) = self.start_oauth() {
                        self.mode = OAuthMode::Error(format!("Failed to start OAuth: {}", e));
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            OAuthMode::ManualInput => {
                if self.input_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Enter => {
                        let code = self.input_widget.value().to_string();
                        if !code.is_empty() {
                            if let Err(e) = self.exchange_code(code) {
                                self.mode =
                                    OAuthMode::Error(format!("Failed to exchange code: {}", e));
                            }
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => Ok(ScreenAction::Back),
                    _ => Ok(ScreenAction::None),
                }
            }
            OAuthMode::Processing => {
                // Cannot cancel during processing
                Ok(ScreenAction::None)
            }
            OAuthMode::Success => match key.code {
                KeyCode::Enter | KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            OAuthMode::Error(_) => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.mode = OAuthMode::Instructions;
                    Ok(ScreenAction::None)
                }
                _ => Ok(ScreenAction::None),
            },
        }
    }

    fn update(&mut self) -> Result<()> {
        Ok(())
    }
}
