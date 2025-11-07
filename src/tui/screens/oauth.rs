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
use std::time::{Duration as StdDuration, Instant};
use tokio::runtime::Runtime;

use super::{Screen, ScreenAction};
use crate::sync::smart_oauth::{AuthState, SmartOAuthAuthenticator};
use crate::tui::app_state::AppState;

const PROVIDER_GOOGLE_DRIVE: &str = "google-drive";
const FLASH_DURATION: StdDuration = StdDuration::from_secs(3);

/// Visual states for the OAuth screen.
enum OAuthMode {
    Intro,
    AwaitingCode,
    Processing,
    Success,
    Error(String),
}

/// OAuth authentication screen
pub struct OAuthScreen {
    app_state: &'static AppState,
    provider: String,
    mode: OAuthMode,
    runtime: Option<Runtime>,
    authenticator: Option<SmartOAuthAuthenticator>,
    flow_id: Option<String>,
    auth_url: Option<String>,
    code_input: String,
    code_cursor: usize,
    code_focused: bool,
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
            auth_url: None,
            code_input: String::new(),
            code_cursor: 0,
            code_focused: false,
            last_state: None,
            status_flash: None,
            spinner_index: 0,
        })
    }

    /// Start a new OAuth authorisation flow.
    fn begin_oauth_flow(&mut self) -> Result<()> {
        if self.runtime.is_none() {
            self.runtime = Some(Runtime::new().context("failed to create async runtime")?);
        }

        let runtime = self
            .runtime
            .as_mut()
            .expect("runtime must be available after initialisation");

        let authenticator = self
            .app_state
            .ensure_authenticator(&self.provider)
            .context("failed to load OAuth credentials")?;
        self.authenticator = Some(authenticator.clone());

        let flow_id = format!("oauth-{}", Utc::now().timestamp_millis());
        self.app_state
            .create_oauth_flow(&self.provider, flow_id.clone(), None, None);

        let auth_url = runtime
            .block_on(authenticator.generate_auth_url_for_tui())
            .context("failed to generate authorization URL")?;

        self.app_state.set_oauth_url(&flow_id, &auth_url).ok();

        let state = runtime.block_on(authenticator.get_state());
        self.last_state = Some(state.clone());
        self.app_state.update_oauth_state(&flow_id, state).ok();

        self.auth_url = Some(auth_url.clone());
        self.flow_id = Some(flow_id);
        self.mode = OAuthMode::AwaitingCode;
        self.spinner_index = 0;
        self.code_input.clear();
        self.code_cursor = 0;
        self.code_focused = true;

        match open::that(&auth_url) {
            Ok(_) => {
                self.flash_message("Opened browser to Google's authorization page", Color::Cyan)
            }
            Err(err) => self.flash_message(
                format!(
                    "Browser could not be opened automatically ({err}); copy the URL manually."
                ),
                Color::Yellow,
            ),
        }

        Ok(())
    }

    /// Submit the user supplied authorisation code.
    fn submit_code(&mut self, code: String) -> Result<()> {
        if code.trim().is_empty() {
            self.flash_message("Authorization code cannot be empty", Color::Yellow);
            return Ok(());
        }

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

        self.mode = OAuthMode::Processing;
        self.spinner_index = 0;
        self.code_focused = false;

        let exchange_result =
            runtime.block_on(authenticator.set_auth_code(code.trim().to_string()));
        match exchange_result {
            Ok(response) => {
                if let Err(err) = self
                    .app_state
                    .persist_oauth_success(&self.provider, &response)
                {
                    let message = format!("Failed to persist credentials: {}", err);
                    if let Some(flow_id) = &self.flow_id {
                        self.app_state
                            .update_oauth_state(
                                flow_id,
                                AuthState::Error {
                                    message: message.clone(),
                                },
                            )
                            .ok();
                    }
                    self.mode = OAuthMode::Error(message);
                    return Ok(());
                }

                if let Some(flow_id) = &self.flow_id {
                    let state = runtime.block_on(authenticator.get_state());
                    self.last_state = Some(state.clone());
                    self.app_state.update_oauth_state(flow_id, state).ok();
                }

                self.flash_message("Google Drive authenticated.", Color::Green);
                self.mode = OAuthMode::Success;
            }
            Err(err) => {
                let message = format!("Failed to exchange authorization code: {}", err);
                if let Some(flow_id) = &self.flow_id {
                    self.app_state
                        .update_oauth_state(
                            flow_id,
                            AuthState::Error {
                                message: message.clone(),
                            },
                        )
                        .ok();
                }
                self.mode = OAuthMode::Error(message);
            }
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
        self.auth_url = None;
        self.flow_id = None;
        self.last_state = None;
        self.code_input.clear();
        self.code_cursor = 0;
        self.code_focused = false;
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

    /// Copy the authorisation URL to the system clipboard.
    fn copy_auth_url(&mut self) {
        if let Some(url) = &self.auth_url {
            match ClipboardContext::new().and_then(|mut ctx| ctx.set_contents(url.clone())) {
                Ok(_) => self.flash_message("Authorization URL copied to clipboard", Color::Green),
                Err(err) => {
                    self.flash_message(format!("Clipboard unavailable: {}", err), Color::Red)
                }
            }
        } else {
            self.flash_message("Authorization URL not available yet", Color::Yellow);
        }
    }

    /// Re-open the authorisation URL in the default browser.
    fn reopen_auth_url(&mut self) {
        if let Some(url) = &self.auth_url {
            match open::that(url) {
                Ok(_) => self.flash_message("Authorization URL opened in browser", Color::Cyan),
                Err(err) => self.flash_message(
                    format!("Failed to open browser automatically: {}", err),
                    Color::Yellow,
                ),
            }
        } else {
            self.flash_message("Authorization URL not available yet", Color::Yellow);
        }
    }

    /// Flash a short lived status message.
    fn flash_message<S: Into<String>>(&mut self, message: S, color: Color) {
        self.status_flash = Some((message.into(), color, Instant::now()));
    }

    /// Map the latest authenticator state to a human readable status line.
    fn status_text(&self) -> String {
        match &self.last_state {
            Some(AuthState::Initializing) => "Initialising authentication flow...".to_string(),
            Some(AuthState::WaitingForCode { .. }) => {
                "Waiting for approval in the browser...".to_string()
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("Authorize Agentic Warden")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

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
        status_lines.push(Line::from(
            "Authorise the request in your browser, then paste the code below.",
        ));
        status_lines.push(Line::from(
            "If nothing happened, press Alt+O to reopen the URL.",
        ));

        let status = Paragraph::new(status_lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, chunks[1]);

        let url_text = if let Some(url) = &self.auth_url {
            vec![Line::from(Span::styled(
                url.clone(),
                Style::default().fg(Color::Yellow),
            ))]
        } else {
            vec![Line::from("Authorisation URL is being generated...")]
        };

        let url_paragraph = Paragraph::new(url_text).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Authorization URL (Alt+C to copy)"),
        );
        frame.render_widget(url_paragraph, chunks[2]);

        self.render_code_input(frame, chunks[3]);

        let help = Paragraph::new(
            "[Enter] Submit  [Alt+C] Copy URL  [Alt+O] Open URL  [Alt+R] Restart  [ESC] Cancel",
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[4]);
    }

    fn render_code_input(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Authorization Code");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let line = if self.code_input.is_empty() {
            Line::from(Span::styled(
                "Paste the code from your browser prompt here",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Line::from(self.code_input.clone())
        };

        let paragraph = Paragraph::new(line).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, inner);

        if self.code_focused {
            let (cursor_x, cursor_y) = self.code_cursor_position(inner);
            frame.set_cursor(cursor_x, cursor_y);
        }
    }

    fn code_cursor_position(&self, inner: Rect) -> (u16, u16) {
        let width = inner.width.max(1) as usize;
        let offset_chars = self.code_input[..self.code_cursor].chars().count();
        let row = (offset_chars / width) as u16;
        let col = (offset_chars % width) as u16;
        (
            inner.x + col.min(inner.width.saturating_sub(1)),
            inner.y + row.min(inner.height.saturating_sub(1)),
        )
    }

    fn handle_code_input(&mut self, key: KeyEvent) -> bool {
        if !self.code_focused {
            return false;
        }

        match key.code {
            KeyCode::Char(c) => {
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    return false;
                }
                let insert_pos = self.code_cursor.min(self.code_input.len());
                self.code_input.insert(insert_pos, c);
                self.code_cursor = insert_pos + c.len_utf8();
                true
            }
            KeyCode::Backspace => {
                if self.code_cursor > 0 {
                    let new_cursor = self.code_input[..self.code_cursor]
                        .chars()
                        .next_back()
                        .map(|ch| self.code_cursor - ch.len_utf8())
                        .unwrap_or(0);
                    self.code_input
                        .replace_range(new_cursor..self.code_cursor, "");
                    self.code_cursor = new_cursor;
                }
                true
            }
            KeyCode::Delete => {
                if self.code_cursor < self.code_input.len() {
                    let delete_len = self.code_input[self.code_cursor..]
                        .chars()
                        .next()
                        .map(|ch| ch.len_utf8())
                        .unwrap_or(1);
                    self.code_input
                        .replace_range(self.code_cursor..self.code_cursor + delete_len, "");
                }
                true
            }
            KeyCode::Left => {
                if self.code_cursor > 0 {
                    let new_cursor = self.code_input[..self.code_cursor]
                        .chars()
                        .next_back()
                        .map(|ch| self.code_cursor - ch.len_utf8())
                        .unwrap_or(0);
                    self.code_cursor = new_cursor;
                }
                true
            }
            KeyCode::Right => {
                if self.code_cursor < self.code_input.len() {
                    let advance = self.code_input[self.code_cursor..]
                        .chars()
                        .next()
                        .map(|ch| ch.len_utf8())
                        .unwrap_or(1);
                    self.code_cursor += advance;
                }
                true
            }
            KeyCode::Home => {
                self.code_cursor = 0;
                true
            }
            KeyCode::End => {
                self.code_cursor = self.code_input.len();
                true
            }
            _ => false,
        }
    }

    fn paste_code_from_clipboard(&mut self) -> bool {
        match ClipboardContext::new().and_then(|mut ctx| ctx.get_contents()) {
            Ok(contents) => {
                let trimmed = contents.trim();
                if trimmed.is_empty() {
                    self.flash_message("Clipboard does not contain a code", Color::Yellow);
                } else {
                    self.code_input = trimmed.to_string();
                    self.code_cursor = self.code_input.len();
                    self.code_focused = true;
                    self.flash_message("Authorization code pasted from clipboard", Color::Cyan);
                }
                true
            }
            Err(err) => {
                self.flash_message(format!("Clipboard unavailable: {}", err), Color::Red);
                true
            }
        }
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

                if key.code == KeyCode::Enter {
                    let code = self.code_input.clone();
                    if let Err(err) = self.submit_code(code) {
                        self.mode = OAuthMode::Error(err.to_string());
                    }
                    return Ok(ScreenAction::None);
                }

                if self.handle_code_input(key) {
                    return Ok(ScreenAction::None);
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

        if let (Some(runtime), Some(authenticator), Some(flow_id)) = (
            self.runtime.as_mut(),
            self.authenticator.clone(),
            self.flow_id.clone(),
        ) {
            let state = runtime.block_on(authenticator.get_state());
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
