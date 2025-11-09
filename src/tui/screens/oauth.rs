//! OAuth Device Flow authentication screen

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
use std::time::{Duration as StdDuration, Instant};
use tokio::runtime::Runtime;

use super::{Screen, ScreenAction};
use crate::sync::smart_device_flow::{AuthState, SmartDeviceFlowAuthenticator};
use crate::sync::device_flow_client::DeviceCodeResponse;
use crate::tui::app_state::AppState;

const PROVIDER_GOOGLE_DRIVE: &str = "google-drive";
const FLASH_DURATION: StdDuration = StdDuration::from_secs(3);

/// Visual states for the OAuth Device Flow screen.
enum OAuthMode {
    Intro,
    DisplayingDeviceCode,
    Polling,
    Success,
    Error(String),
}

/// OAuth Device Flow authentication screen
pub struct OAuthScreen {
    app_state: &'static AppState,
    provider: String,
    mode: OAuthMode,
    runtime: Option<Runtime>,
    authenticator: Option<SmartDeviceFlowAuthenticator>,
    flow_id: Option<String>,
    device_response: Option<DeviceCodeResponse>,
    polling_start: Option<Instant>,
    last_state: Option<AuthState>,
    status_flash: Option<(String, Color, Instant)>,
    spinner_index: usize,
}

impl OAuthScreen {
    pub fn new() -> Result<Self> {
        Ok(Self {
            app_state: AppState::global(),
            provider: PROVIDER_GOOGLE_DRIVE.to_string(),
            mode: OAuthMode::Intro,
            runtime: None,
            authenticator: None,
            flow_id: None,
            device_response: None,
            polling_start: None,
            last_state: None,
            status_flash: None,
            spinner_index: 0,
        })
    }

    /// Start a new OAuth Device Flow authorisation.
    fn begin_oauth_flow(&mut self) -> Result<()> {
        use crate::sync::device_flow_client::AuthConfig;
        use std::path::PathBuf;

        if self.runtime.is_none() {
            self.runtime = Some(Runtime::new().context("failed to create async runtime")?);
        }

        let runtime = self
            .runtime
            .as_mut()
            .expect("runtime must be available after initialisation");

        // TODO: Load client_id and client_secret from config
        // For now, creating a minimal authenticator
        let auth_config = AuthConfig {
            client_id: "your-client-id".to_string(),  // TODO: Load from config
            client_secret: "your-client-secret".to_string(),  // TODO: Load from config
            scopes: vec!["https://www.googleapis.com/auth/drive.file".to_string()],
            auth_timeout: 300,
            poll_interval: 5,
        };

        let auth_file_path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
            .join(".agentic-warden")
            .join("auth.json");

        let authenticator = SmartDeviceFlowAuthenticator::new(auth_config, auth_file_path);
        self.authenticator = Some(authenticator.clone());

        let flow_id = format!("device-flow-{}", Utc::now().timestamp_millis());

        // Start Device Flow
        let device_response = runtime
            .block_on(authenticator.start_device_flow())
            .context("failed to start Device Flow")?;

        let state = runtime.block_on(authenticator.get_state());
        self.last_state = Some(state.clone());

        self.device_response = Some(device_response.clone());
        self.flow_id = Some(flow_id);
        self.mode = OAuthMode::DisplayingDeviceCode;
        self.spinner_index = 0;
        self.polling_start = Some(Instant::now());

        // Attempt to open browser automatically
        match open::that(&device_response.verification_url) {
            Ok(_) => {
                self.flash_message("Opened browser to Google's authorization page", Color::Cyan)
            }
            Err(err) => self.flash_message(
                format!(
                    "Browser could not be opened automatically ({err}); visit the URL manually."
                ),
                Color::Yellow,
            ),
        }

        Ok(())
    }

    /// Start polling for Device Flow authorization.
    fn start_polling(&mut self) -> Result<()> {
        let runtime = match self.runtime.as_mut() {
            Some(rt) => rt,
            None => {
                self.flash_message("Runtime not initialised; restart the flow.", Color::Red);
                return Ok(());
            }
        };

        let authenticator = match &self.authenticator {
            Some(auth) => auth.clone(),
            None => {
                self.flash_message("Authenticator not ready; restart the flow.", Color::Red);
                return Ok(());
            }
        };

        let device_response = match &self.device_response {
            Some(resp) => resp.clone(),
            None => {
                self.flash_message("Device response not available; restart the flow.", Color::Red);
                return Ok(());
            }
        };

        self.mode = OAuthMode::Polling;
        self.spinner_index = 0;

        // Poll for authorization (blocking call)
        let poll_result = runtime.block_on(authenticator.poll_until_authorized(
            &device_response.device_code,
            device_response.interval,
            device_response.expires_in,
        ));

        match poll_result {
            Ok(_token_info) => {
                // TODO: Persist the token info using AppState
                // Tokens are already saved by SmartDeviceFlowAuthenticator

                let state = runtime.block_on(authenticator.get_state());
                self.last_state = Some(state);

                self.flash_message("Google Drive authenticated successfully!", Color::Green);
                self.mode = OAuthMode::Success;
            }
            Err(err) => {
                let message = format!("Device Flow authorization failed: {}", err);
                self.mode = OAuthMode::Error(message);
            }
        }

        Ok(())
    }

    /// Cancel the current OAuth flow and return to the intro screen.
    fn cancel_flow(&mut self) {
        self.clear_flow_state();
        self.mode = OAuthMode::Intro;
    }

    /// Remove any per-flow cached data without altering stored credentials.
    fn clear_flow_state(&mut self) {
        self.device_response = None;
        self.flow_id = None;
        self.last_state = None;
        self.polling_start = None;
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

    /// Copy the device code to the system clipboard.
    fn copy_device_code(&mut self) {
        if let Some(device_resp) = &self.device_response {
            match ClipboardContext::new().and_then(|mut ctx| ctx.set_contents(device_resp.user_code.clone())) {
                Ok(_) => self.flash_message("Device code copied to clipboard", Color::Green),
                Err(err) => {
                    self.flash_message(format!("Clipboard unavailable: {}", err), Color::Red)
                }
            }
        } else {
            self.flash_message("Device code not available yet", Color::Yellow);
        }
    }

    /// Copy the verification URL to the system clipboard.
    fn copy_verification_url(&mut self) {
        if let Some(device_resp) = &self.device_response {
            match ClipboardContext::new().and_then(|mut ctx| ctx.set_contents(device_resp.verification_url.clone())) {
                Ok(_) => self.flash_message("Verification URL copied to clipboard", Color::Green),
                Err(err) => {
                    self.flash_message(format!("Clipboard unavailable: {}", err), Color::Red)
                }
            }
        } else {
            self.flash_message("Verification URL not available yet", Color::Yellow);
        }
    }

    /// Re-open the verification URL in the default browser.
    fn reopen_verification_url(&mut self) {
        if let Some(device_resp) = &self.device_response {
            match open::that(&device_resp.verification_url) {
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
    fn status_text(&self) -> String {
        match &self.last_state {
            Some(AuthState::Initializing) => "Initialising Device Flow...".to_string(),
            Some(AuthState::WaitingForUser { .. }) => {
                "Waiting for you to authorize in the browser...".to_string()
            }
            Some(AuthState::Polling { .. }) => {
                "Polling for authorization...".to_string()
            }
            Some(AuthState::Authenticated { .. }) => {
                "Authentication completed successfully!".to_string()
            }
            Some(AuthState::Failed { message }) => format!("Error: {}", message),
            Some(AuthState::Expired) => "Device code expired. Please restart.".to_string(),
            Some(AuthState::AccessDenied) => "Access denied by user.".to_string(),
            None => "Ready to start Device Flow.".to_string(),
        }
    }

    /// Calculate remaining time for device code expiration.
    fn remaining_seconds(&self) -> Option<u64> {
        if let (Some(device_resp), Some(start_time)) = (&self.device_response, &self.polling_start) {
            let elapsed = start_time.elapsed().as_secs();
            if elapsed < device_resp.expires_in {
                Some(device_resp.expires_in - elapsed)
            } else {
                Some(0)
            }
        } else {
            None
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

        let title = Paragraph::new("Google Drive Authentication (Device Flow)")
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
                "Agentic Warden needs permission to access your Google Drive for config sync.",
            ),
            Line::from(""),
            Line::from("The Device Flow authentication performs these steps:"),
            Line::from("  1. Display a device code and verification URL;"),
            Line::from("  2. Open the URL in your browser (automatic);"),
            Line::from("  3. Enter the device code on Google's page;"),
            Line::from("  4. Authorize the request;"),
            Line::from("  5. Automatic background polling completes the flow."),
            Line::from(""),
            Line::from("Credentials are stored in ~/.agentic-warden/auth.json."),
            Line::from("You can revoke access anytime from Google's security settings."),
            Line::from(""),
            Line::from("Press Enter to start the Device Flow."),
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

    fn render_displaying_device_code(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("Device Flow Authorization")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Status section
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
            Span::raw(self.status_text()),
        ]));
        status_lines.push(Line::from(""));

        if let Some(remaining) = self.remaining_seconds() {
            let minutes = remaining / 60;
            let seconds = remaining % 60;
            status_lines.push(Line::from(vec![
                Span::raw("Time remaining: "),
                Span::styled(
                    format!("{}:{:02}", minutes, seconds),
                    Style::default().fg(if remaining < 60 { Color::Red } else { Color::Yellow }),
                ),
            ]));
        }

        status_lines.push(Line::from(""));
        status_lines.push(Line::from(
            "Visit the URL below and enter the device code to authorize.",
        ));
        status_lines.push(Line::from(
            "Press Enter to start polling after authorizing.",
        ));

        let status = Paragraph::new(status_lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, chunks[1]);

        // Device code section
        let device_code_text = if let Some(device_resp) = &self.device_response {
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter this code: "),
                    Span::styled(
                        device_resp.user_code.clone(),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
            ]
        } else {
            vec![Line::from("Generating device code...")]
        };

        let device_code_paragraph = Paragraph::new(device_code_text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Device Code (Alt+C to copy)"),
            );
        frame.render_widget(device_code_paragraph, chunks[2]);

        // Verification URL section
        let url_text = if let Some(device_resp) = &self.device_response {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    device_resp.verification_url.clone(),
                    Style::default().fg(Color::Yellow),
                )),
            ]
        } else {
            vec![Line::from("Generating verification URL...")]
        };

        let url_paragraph = Paragraph::new(url_text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Verification URL (Alt+U to copy)"),
            );
        frame.render_widget(url_paragraph, chunks[3]);

        // Instructions
        let instructions = Paragraph::new(vec![
            Line::from(""),
            Line::from("1. The browser should have opened automatically"),
            Line::from("2. If not, press Alt+O to open it manually"),
            Line::from("3. Enter the device code shown above"),
            Line::from("4. Approve the authorization request"),
            Line::from("5. Press Enter here to start polling"),
        ])
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Instructions"));
        frame.render_widget(instructions, chunks[4]);

        let help = Paragraph::new(
            "[Enter] Start Polling  [Alt+C] Copy Code  [Alt+U] Copy URL  [Alt+O] Open URL  [Alt+R] Restart  [ESC] Cancel",
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[5]);
    }

    fn render_polling(&self, frame: &mut Frame, area: Rect) {
        let mut message = format!(
            "{} Polling for authorization...\n\n",
            self.spinner_char()
        );

        if let Some(remaining) = self.remaining_seconds() {
            let minutes = remaining / 60;
            let seconds = remaining % 60;
            message.push_str(&format!("Time remaining: {}:{:02}\n\n", minutes, seconds));
        }

        message.push_str("Please complete the authorization in your browser.");

        render_modal(
            frame,
            area,
            "Polling for Authorization",
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
        // Increment spinner
        self.spinner_index = (self.spinner_index + 1) % 4;

        match &self.mode {
            OAuthMode::Intro => self.render_intro(frame, area),
            OAuthMode::DisplayingDeviceCode => self.render_displaying_device_code(frame, area),
            OAuthMode::Polling => self.render_polling(frame, area),
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
            OAuthMode::DisplayingDeviceCode => {
                if key.code == KeyCode::Esc {
                    self.cancel_flow();
                    return Ok(ScreenAction::None);
                }

                if key.modifiers.contains(KeyModifiers::ALT) {
                    match key.code {
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            self.copy_device_code();
                            return Ok(ScreenAction::None);
                        }
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            self.copy_verification_url();
                            return Ok(ScreenAction::None);
                        }
                        KeyCode::Char('o') | KeyCode::Char('O') => {
                            self.reopen_verification_url();
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

                // Start polling when user presses Enter
                if key.code == KeyCode::Enter {
                    if let Err(err) = self.start_polling() {
                        self.mode = OAuthMode::Error(err.to_string());
                    }
                    return Ok(ScreenAction::None);
                }

                Ok(ScreenAction::None)
            }
            OAuthMode::Polling => {
                // During polling, only allow cancellation
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
        // Clear expired flash messages
        if let Some((_, _, instant)) = &self.status_flash {
            if instant.elapsed() > FLASH_DURATION {
                self.status_flash = None;
            }
        }

        // Update authenticator state
        if let (Some(runtime), Some(authenticator)) = (
            self.runtime.as_mut(),
            self.authenticator.clone(),
        ) {
            let state = runtime.block_on(authenticator.get_state());
            if self.last_state.as_ref() != Some(&state) {
                self.last_state = Some(state.clone());

                // Check for completion states during polling
                if matches!(state, AuthState::Authenticated { .. })
                    && matches!(self.mode, OAuthMode::Polling | OAuthMode::DisplayingDeviceCode)
                {
                    self.mode = OAuthMode::Success;
                }

                if let AuthState::Failed { message } = state {
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
