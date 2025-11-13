//! OAuth authentication screen

use anyhow::{Context, Result};
use chrono::Utc;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::time::Instant;

use super::{Screen, ScreenAction};
use crate::common::constants::{duration::FLASH_DURATION, providers::GOOGLE_DRIVE};
use crate::sync::smart_oauth::{AuthState, SmartOAuthAuthenticator};
use crate::tui::app_state::AppState;

/// Visual states for the OAuth screen.
enum OAuthMode {
    Intro,
    AwaitingCode,
    #[allow(dead_code)]
    Processing,
    Success,
    Error(String),
}

/// OAuth authentication screen
pub struct OAuthScreen {
    app_state: &'static AppState,
    provider: String,
    mode: OAuthMode,
    authenticator: Option<SmartOAuthAuthenticator>,
    flow_id: Option<String>,
    last_state: Option<AuthState>,
    status_flash: Option<(String, Color, Instant)>,
    spinner_index: usize,
}

impl OAuthScreen {
    pub fn new() -> Result<Self> {
        Ok(Self {
            app_state: AppState::global(),
            provider: GOOGLE_DRIVE.to_string(),
            mode: OAuthMode::Intro,
            authenticator: None,
            flow_id: None,
            last_state: None,
            status_flash: None,
            spinner_index: 0,
        })
    }

    /// Start Device Flow (RFC 8628) authorization
    fn begin_oauth_flow(&mut self) -> Result<()> {
        let authenticator = self
            .app_state
            .ensure_authenticator(&self.provider)
            .context("failed to load OAuth credentials")?;
        self.authenticator = Some(authenticator.clone());

        let flow_id = format!("oauth-{}", Utc::now().timestamp_millis());
        self.app_state
            .create_oauth_flow(&self.provider, flow_id.clone(), None, None);

        // Start Device Flow
        let device_response = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(authenticator.start_device_flow())
        })
        .context("failed to start device flow")?;

        let state = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(authenticator.get_state())
        });
        self.last_state = Some(state.clone());
        self.app_state.update_oauth_state(&flow_id, state).ok();

        self.flow_id = Some(flow_id);
        self.mode = OAuthMode::AwaitingCode;
        self.spinner_index = 0;

        // Try to open browser automatically
        match open::that(&device_response.verification_url) {
            Ok(_) => self.flash_message("Opened verification URL in browser", Color::Cyan),
            Err(err) => self.flash_message(
                format!("Browser could not be opened automatically ({err}); open URL manually."),
                Color::Yellow,
            ),
        }

        Ok(())
    }

    /// Cancel the current OAuth flow and return to the intro screen.
    fn cancel_flow(&mut self) {
        if let Some(flow_id) = &self.flow_id {
            self.app_state
                .update_oauth_state(
                    flow_id,
                    AuthState::Error {
                        message: "Authentication cancelled by user".to_string(),
                    },
                )
                .ok();
        }

        self.clear_flow_state();
        self.mode = OAuthMode::Intro;
    }

    /// Remove any per-flow cached data without altering stored credentials.
    fn clear_flow_state(&mut self) {
        self.flow_id = None;
        self.last_state = None;
        self.status_flash = None;
        self.spinner_index = 0;
    }

    /// Restart the OAuth flow from scratch.
    fn restart_flow(&mut self) -> Result<()> {
        self.clear_flow_state();
        self.mode = OAuthMode::Intro;
        self.begin_oauth_flow()
    }

    /// Return to the intro screen after a successful completion.
    fn reset_after_success(&mut self) {
        self.clear_flow_state();
        self.mode = OAuthMode::Intro;
    }

    /// Copy the user code to the system clipboard.
    fn copy_auth_url(&mut self) {
        if let Some(AuthState::WaitingForDeviceAuth { user_code, .. }) = &self.last_state {
            match ClipboardContext::new().and_then(|mut ctx| ctx.set_contents(user_code.clone())) {
                Ok(_) => self.flash_message("User code copied to clipboard", Color::Green),
                Err(err) => {
                    self.flash_message(format!("Clipboard unavailable: {}", err), Color::Red)
                }
            }
        } else {
            self.flash_message("User code not available yet", Color::Yellow);
        }
    }

    /// Re-open the verification URL in the default browser.
    fn reopen_auth_url(&mut self) {
        if let Some(AuthState::WaitingForDeviceAuth {
            verification_url, ..
        }) = &self.last_state
        {
            match open::that(verification_url) {
                Ok(_) => self.flash_message("Verification URL opened in browser", Color::Cyan),
                Err(err) => self.flash_message(
                    format!("Failed to open browser automatically: {}", err),
                    Color::Yellow,
                ),
            }
        } else {
            self.flash_message("Verification URL not available yet", Color::Yellow);
        }
    }

    /// Flash a short lived status message.
    fn flash_message<S: Into<String>>(&mut self, message: S, color: Color) {
        self.status_flash = Some((message.into(), color, Instant::now()));
    }

    /// Map the latest authenticator state to a human readable status line.
    #[allow(dead_code)]
    fn status_text(&self) -> String {
        match &self.last_state {
            Some(AuthState::Initializing) => "Initialising authentication flow...".to_string(),
            Some(AuthState::WaitingForDeviceAuth {
                user_code,
                verification_url,
                ..
            }) => {
                format!("Visit {} and enter code: {}", verification_url, user_code)
            }
            Some(AuthState::Authenticated { .. }) => {
                "Authentication completed successfully.".to_string()
            }
            Some(AuthState::Error { message }) => format!("Error: {}", message),
            None => "Ready to authenticate.".to_string(),
        }
    }

    fn spinner_char(&self) -> char {
        const FRAMES: [char; 4] = ['|', '/', '-', '\\'];
        FRAMES[self.spinner_index % FRAMES.len()]
    }

    fn render_intro(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("Google Drive Authentication")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        let lines = vec![
            Line::from(""),
            Line::from(
                "Agentic Warden needs permission to access your Google Drive for push/pull sync.",
            ),
            Line::from(""),
            Line::from("The authentication flow performs the following steps:"),
            Line::from("  1. Open Google's OAuth consent page in your browser;"),
            Line::from("  2. You approve the request and receive a one-time code;"),
            Line::from("  3. Paste the code back into this screen to finish."),
            Line::from(""),
            Line::from("Credentials are stored in ~/.agentic-warden/auth.json."),
            Line::from("You can revoke access at any time from Google's security settings."),
            Line::from(""),
            Line::from("Press Enter to start the OAuth flow."),
        ];

        let body = Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Overview"));
        frame.render_widget(body, chunks[1]);

        let help = Paragraph::new("[Enter] Start  [ESC] Back")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[2]);
    }

    fn render_awaiting(&self, frame: &mut Frame, area: Rect) {
        if let Some(AuthState::WaitingForDeviceAuth {
            user_code,
            verification_url,
            expires_at,
        }) = &self.last_state
        {
            self.render_device_flow(frame, area, user_code, verification_url, expires_at);
        } else {
            // Fallback if state is not Device Flow
            let text = Paragraph::new("Initializing Device Flow...").alignment(Alignment::Center);
            frame.render_widget(text, area);
        }
    }

    fn render_device_flow(
        &self,
        frame: &mut Frame,
        area: Rect,
        user_code: &str,
        verification_url: &str,
        expires_at: &chrono::DateTime<chrono::Utc>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("Device Authorization (RFC 8628)")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Status with spinner
        let mut status_lines = Vec::new();
        if let Some((message, color, _)) = &self.status_flash {
            status_lines.push(Line::from(Span::styled(
                message.clone(),
                Style::default().fg(*color).add_modifier(Modifier::BOLD),
            )));
            status_lines.push(Line::from(""));
        }

        status_lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", self.spinner_char()),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("Waiting for authorization..."),
        ]));
        status_lines.push(Line::from(""));

        let remaining = (*expires_at - chrono::Utc::now()).num_seconds();
        status_lines.push(Line::from(format!(
            "Code expires in {} seconds",
            remaining.max(0)
        )));

        let status = Paragraph::new(status_lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, chunks[1]);

        // User code display (large and prominent)
        let code_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("Visit: "),
                Span::styled(
                    verification_url,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            ]),
            Line::from(""),
            Line::from("And enter this code:"),
            Line::from(""),
            Line::from(Span::styled(
                format!("    {}    ", user_code),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Black),
            )),
        ];

        let code_paragraph = Paragraph::new(code_lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("User Code"));
        frame.render_widget(code_paragraph, chunks[2]);

        // Instructions
        let instructions = vec![
            Line::from(""),
            Line::from("Device Flow Instructions:"),
            Line::from("  1. Open the verification URL in any browser (phone/computer)"),
            Line::from("  2. Sign in to your Google account if needed"),
            Line::from("  3. Enter the user code shown above"),
            Line::from("  4. Grant permissions to Agentic Warden"),
            Line::from(""),
            Line::from("This window will automatically continue once you authorize."),
            Line::from("No need to copy/paste anything back here!"),
        ];

        let instructions_paragraph = Paragraph::new(instructions)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Instructions"));
        frame.render_widget(instructions_paragraph, chunks[3]);

        let help = Paragraph::new("[Alt+O] Open URL  [Alt+C] Copy Code  [ESC] Cancel")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[4]);
    }

    fn render_processing(&self, frame: &mut Frame, area: Rect) {
        let message = format!(
            "{} Exchanging authorization code for tokens...",
            self.spinner_char()
        );
        render_modal(
            frame,
            area,
            "Processing",
            &message,
            Style::default().fg(Color::Cyan),
        );
    }

    fn render_success(&self, frame: &mut Frame, area: Rect) {
        render_modal(
            frame,
            area,
            "Authentication Complete",
            "Google Drive OAuth succeeded.\nPress Enter to return to the previous screen.",
            Style::default().fg(Color::Green),
        );
    }

    fn render_error(&self, frame: &mut Frame, area: Rect, message: &str) {
        render_modal(
            frame,
            area,
            "Authentication Failed",
            &format!("{message}\nPress R to retry or ESC to cancel."),
            Style::default().fg(Color::Red),
        );
    }
}

impl Screen for OAuthScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            OAuthMode::Intro => self.render_intro(frame, area),
            OAuthMode::AwaitingCode => self.render_awaiting(frame, area),
            OAuthMode::Processing => self.render_processing(frame, area),
            OAuthMode::Success => self.render_success(frame, area),
            OAuthMode::Error(message) => self.render_error(frame, area, message),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match &self.mode {
            OAuthMode::Intro => match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Enter => {
                    if let Err(err) = self.begin_oauth_flow() {
                        self.mode = OAuthMode::Error(err.to_string());
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            OAuthMode::AwaitingCode => {
                if key.code == KeyCode::Esc {
                    self.cancel_flow();
                    return Ok(ScreenAction::None);
                }

                if key.modifiers.contains(KeyModifiers::ALT) {
                    match key.code {
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            self.copy_auth_url();
                            return Ok(ScreenAction::None);
                        }
                        KeyCode::Char('o') | KeyCode::Char('O') => {
                            self.reopen_auth_url();
                            return Ok(ScreenAction::None);
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            if let Err(err) = self.restart_flow() {
                                self.mode = OAuthMode::Error(err.to_string());
                            }
                            return Ok(ScreenAction::None);
                        }
                        _ => {}
                    }
                }

                Ok(ScreenAction::None)
            }
            OAuthMode::Processing => {
                if key.code == KeyCode::Esc {
                    self.cancel_flow();
                }
                Ok(ScreenAction::None)
            }
            OAuthMode::Success => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.reset_after_success();
                    Ok(ScreenAction::Back)
                }
                _ => Ok(ScreenAction::None),
            },
            OAuthMode::Error(_) => match key.code {
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    if let Err(err) = self.restart_flow() {
                        self.mode = OAuthMode::Error(err.to_string());
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Enter => {
                    self.mode = OAuthMode::Intro;
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => {
                    self.cancel_flow();
                    Ok(ScreenAction::Back)
                }
                _ => Ok(ScreenAction::None),
            },
        }
    }

    fn update(&mut self) -> Result<()> {
        if matches!(self.mode, OAuthMode::AwaitingCode | OAuthMode::Processing) {
            self.spinner_index = self.spinner_index.wrapping_add(1);
        }

        if let Some((_, _, instant)) = &self.status_flash {
            if instant.elapsed() > FLASH_DURATION {
                self.status_flash = None;
            }
        }

        if let (Some(authenticator), Some(flow_id)) =
            (self.authenticator.clone(), self.flow_id.clone())
        {
            let state = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(authenticator.get_state())
            });
            if self.last_state.as_ref() != Some(&state) {
                self.last_state = Some(state.clone());
                self.app_state
                    .update_oauth_state(&flow_id, state.clone())
                    .ok();

                if matches!(state, AuthState::Authenticated { .. })
                    && matches!(self.mode, OAuthMode::Processing | OAuthMode::AwaitingCode)
                {
                    self.mode = OAuthMode::Success;
                }

                if let AuthState::Error { message } = state {
                    if !matches!(self.mode, OAuthMode::Error(_)) {
                        self.mode = OAuthMode::Error(message);
                    }
                }
            }
        }

        Ok(())
    }
}

fn render_modal(frame: &mut Frame, area: Rect, title: &str, message: &str, style: Style) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(35),
                Constraint::Length(7),
                Constraint::Percentage(58),
            ]
            .as_ref(),
        )
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(15),
                Constraint::Percentage(70),
                Constraint::Percentage(15),
            ]
            .as_ref(),
        )
        .split(vertical[1]);

    let modal_area = horizontal[1];
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style);
    frame.render_widget(block.clone(), modal_area);

    let inner = block.inner(modal_area);
    let content = Paragraph::new(message)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(style);
    frame.render_widget(content, inner);
}
