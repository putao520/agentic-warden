//! Dashboard main screen

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::cmp::Reverse;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::{Screen, ScreenAction, ScreenType};

use crate::provider::config::ProvidersConfig;
use crate::provider::{AiType, ProviderManager};
use crate::task_record::TaskStatus;

/// Maximum number of status messages to retain in the footer.
const MAX_STATUS_MESSAGES: usize = 5;

/// Recommended installation hints for supported CLI tools.
const INSTALL_HINTS: &[(&str, &str)] = &[
    ("codex", "npm install -g @openai/codex-cli"),
    ("claude", "npm install -g @anthropic-ai/claude-cli"),
    ("gemini", "npm install -g @google-ai/gemini-cli"),
];

/// AI CLI status information.
#[derive(Debug, Clone)]
struct AiCliStatus {
    name: String,
    command: String,
    installed: bool,
    version: Option<String>,
    provider_hint: Option<String>,
}

/// Task summary for dashboard display.
#[derive(Debug, Clone)]
struct TaskSummary {
    task_id: u32,
    status: TaskStatus,
    prompt_preview: String,
    elapsed_secs: u64,
    started_at: i64,
}

/// Provider overview for the dashboard.
#[derive(Debug, Clone, Default)]
struct ProviderOverview {
    total: usize,
    with_tokens: usize,
    without_tokens: usize,
    default_provider: Option<String>,
    missing_tokens: Vec<String>,
}

/// Authentication status snapshot.
#[derive(Debug, Clone)]
struct AuthStatus {
    google_drive_authenticated: bool,
    token_expires_in_days: Option<i64>,
}

/// Dashboard screen state.
pub struct DashboardScreen {
    last_update: Instant,
    ai_cli_status: Vec<AiCliStatus>,
    recent_tasks: Vec<TaskSummary>,
    auth_status: AuthStatus,
    provider_overview: ProviderOverview,
    status_messages: VecDeque<String>,
    install_messages: VecDeque<String>,
}

impl DashboardScreen {
    pub fn new() -> Result<Self> {
        let mut screen = Self {
            last_update: Instant::now(),
            ai_cli_status: Vec::new(),
            recent_tasks: Vec::new(),
            auth_status: AuthStatus {
                google_drive_authenticated: false,
                token_expires_in_days: None,
            },
            provider_overview: ProviderOverview::default(),
            status_messages: VecDeque::new(),
            install_messages: VecDeque::new(),
        };

        screen.refresh_data()?;

        Ok(screen)
    }

    fn refresh_data(&mut self) -> Result<()> {
        self.status_messages.clear();

        let provider_config = match Self::load_provider_config() {
            Ok(cfg) => {
                self.provider_overview = Self::summarize_providers(&cfg);
                Some(cfg)
            }
            Err(err) => {
                self.provider_overview = ProviderOverview::default();
                self.push_status_message(format!("Provider info unavailable: {err}"));
                None
            }
        };

        self.ai_cli_status = Self::detect_ai_cli_status(provider_config.as_ref());

        match Self::get_recent_tasks() {
            Ok(tasks) => self.recent_tasks = tasks,
            Err(err) => {
                self.recent_tasks.clear();
                self.push_status_message(format!("Task registry unavailable: {err}"));
            }
        }

        self.auth_status = match Self::get_auth_status() {
            Ok(status) => status,
            Err(err) => {
                self.push_status_message(format!("Auth status unavailable: {err}"));
                AuthStatus {
                    google_drive_authenticated: false,
                    token_expires_in_days: None,
                }
            }
        };

        Ok(())
    }

    fn detect_ai_cli_status(provider_config: Option<&ProvidersConfig>) -> Vec<AiCliStatus> {
        let cli_definitions = [
            ("codex", "Codex CLI", AiType::Codex),
            ("claude", "Claude CLI", AiType::Claude),
            ("gemini", "Gemini CLI", AiType::Gemini),
        ];

        cli_definitions
            .iter()
            .map(|(command, name, ai_type)| {
                let installed = which::which(command).is_ok();
                let version = installed.then(|| Self::get_cli_version(command)).flatten();
                let provider_hint = provider_config
                    .and_then(|cfg| Self::select_provider_for_cli(cfg, ai_type.clone()));

                AiCliStatus {
                    name: (*name).to_string(),
                    command: (*command).to_string(),
                    installed,
                    version,
                    provider_hint,
                }
            })
            .collect()
    }

    fn get_cli_version(command: &str) -> Option<String> {
        std::process::Command::new(command)
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn select_provider_for_cli(config: &ProvidersConfig, ai_type: AiType) -> Option<String> {
        if let Some(provider) = config.get_provider(&config.default_provider) {
            if provider.compatible_with.contains(&ai_type) {
                return Some(config.default_provider.clone());
            }
        }

        config
            .get_compatible_providers(&ai_type)
            .into_iter()
            .map(|(name, _)| name.clone())
            .next()
    }

    fn get_recent_tasks() -> Result<Vec<TaskSummary>> {
        let registry = crate::registry::TaskRegistry::connect()?;
        let entries = registry.entries()?;
        let now = chrono::Utc::now().timestamp();

        let mut tasks: Vec<TaskSummary> = entries
            .into_iter()
            .map(|entry| {
                let started_at = entry.record.started_at.timestamp();
                let prompt_preview = Self::truncate_prompt(&entry.record.log_id, 48);
                let elapsed_secs = now.saturating_sub(started_at) as u64;

                TaskSummary {
                    task_id: entry.pid,
                    status: entry.record.status,
                    prompt_preview,
                    elapsed_secs,
                    started_at,
                }
            })
            .collect();

        tasks.sort_by_key(|task| Reverse(task.started_at));
        tasks.truncate(5);

        Ok(tasks)
    }

    fn truncate_prompt(prompt: &str, max_len: usize) -> String {
        if prompt.len() <= max_len {
            prompt.to_string()
        } else {
            let end = max_len.saturating_sub(3);
            format!("{}...", &prompt[..end])
        }
    }

    fn get_auth_status() -> Result<AuthStatus> {
        let auth_path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot locate home directory"))?
            .join(".agentic-warden")
            .join("auth.json");

        if !auth_path.exists() {
            return Ok(AuthStatus {
                google_drive_authenticated: false,
                token_expires_in_days: None,
            });
        }

        let auth_content = std::fs::read_to_string(&auth_path)?;
        let auth_state: serde_json::Value = serde_json::from_str(&auth_content)?;

        let authenticated = auth_state
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        let token_expires_in_days =
            auth_state
                .get("expires_at")
                .and_then(|v| v.as_i64())
                .map(|expires_at| {
                    let now = chrono::Utc::now().timestamp();
                    (expires_at - now) / 86_400
                });

        Ok(AuthStatus {
            google_drive_authenticated: authenticated,
            token_expires_in_days,
        })
    }

    fn format_elapsed_time(secs: u64) -> String {
        if secs < 60 {
            format!("{secs}s")
        } else if secs < 3_600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3_600, (secs % 3_600) / 60)
        }
    }

    fn load_provider_config() -> Result<ProvidersConfig> {
        let manager = ProviderManager::new()?;
        Ok(manager.get_providers_config().clone())
    }

    fn summarize_providers(config: &ProvidersConfig) -> ProviderOverview {
        let total = config.providers.len();

        let mut with_tokens = 0usize;
        let mut missing_tokens = Vec::new();

        for (id, provider) in &config.providers {
            let token_entry = config.user_tokens.get(id);
            let has_token = token_entry.map(Self::tokens_present).unwrap_or(false);

            if has_token {
                with_tokens += 1;
            } else {
                missing_tokens.push(provider.name.clone());
            }
        }

        ProviderOverview {
            total,
            with_tokens,
            without_tokens: total.saturating_sub(with_tokens),
            default_provider: (!config.default_provider.is_empty())
                .then(|| config.default_provider.clone()),
            missing_tokens,
        }
    }

    fn tokens_present(tokens: &crate::provider::config::RegionalTokens) -> bool {
        tokens
            .mainland_china
            .as_ref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
            || tokens
                .international
                .as_ref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
    }

    fn push_status_message(&mut self, message: String) {
        if self.status_messages.len() >= MAX_STATUS_MESSAGES {
            self.status_messages.pop_front();
        }
        self.status_messages.push_back(message);
    }

    fn task_status_label(status: &TaskStatus) -> &'static str {
        match status {
            TaskStatus::Running => "running",
            TaskStatus::CompletedButUnread => "completed",
        }
    }

    fn set_install_messages(&mut self, messages: Vec<String>) {
        self.install_messages.clear();
        for message in messages.into_iter().take(MAX_STATUS_MESSAGES) {
            self.install_messages.push_back(message);
        }
    }

    fn toggle_install_guidance(&mut self) {
        if !self.install_messages.is_empty() {
            self.install_messages.clear();
            return;
        }

        let mut instructions = Vec::new();
        let missing: Vec<_> = self
            .ai_cli_status
            .iter()
            .filter(|cli| !cli.installed)
            .collect();

        if missing.is_empty() {
            instructions.push(
                "All AI CLI tools detected. Run `npm update -g <package>` or `agentic-warden cli-manager` to update."
                    .to_string(),
            );
        } else {
            for cli in missing {
                let hint = Self::install_hint(&cli.command);
                instructions.push(format!("Install {}: {}", cli.name, hint));
            }
            instructions.push("Use `agentic-warden cli-manager` for guided setup.".to_string());
        }

        self.set_install_messages(instructions);
    }

    fn install_hint(command: &str) -> &'static str {
        INSTALL_HINTS
            .iter()
            .find(|(cmd, _)| cmd.eq_ignore_ascii_case(command))
            .map(|(_, hint)| *hint)
            .unwrap_or("Refer to provider documentation for installation steps.")
    }
}

impl Screen for DashboardScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(4),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Agentic-Warden Dashboard v0.3.0")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Content
        let mut content = vec![
            Line::from(""),
            Line::from(Span::styled(
                "AI CLI Status",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
        ];

        if self.ai_cli_status.is_empty() {
            content.push(Line::from("  No AI CLIs detected"));
        } else {
            for cli in &self.ai_cli_status {
                let status_icon = if cli.installed { "[OK]" } else { "[--]" };
                let status_color = if cli.installed {
                    Color::Green
                } else {
                    Color::Red
                };

                let mut spans = vec![
                    Span::raw("  "),
                    Span::styled(status_icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{} ({})", cli.name, cli.command),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ];

                if let Some(version) = &cli.version {
                    spans.push(Span::raw(format!(" v{}", version)));
                }

                if let Some(provider) = &cli.provider_hint {
                    spans.push(Span::raw(format!(" · provider {}", provider)));
                }

                content.push(Line::from(spans));
            }
        }

        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "Provider Overview",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if self.provider_overview.total == 0 {
            content.push(Line::from("  No providers configured"));
        } else {
            content.push(Line::from(format!(
                "  Total: {}  With tokens: {}  Without tokens: {}",
                self.provider_overview.total,
                self.provider_overview.with_tokens,
                self.provider_overview.without_tokens
            )));

            if let Some(default) = &self.provider_overview.default_provider {
                content.push(Line::from(format!("  Default provider: {default}")));
            }

            if !self.provider_overview.missing_tokens.is_empty() {
                content.push(Line::from(vec![
                    Span::styled("  Missing tokens: ", Style::default().fg(Color::Yellow)),
                    Span::raw(self.provider_overview.missing_tokens.join(", ")),
                ]));
            }
        }

        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "Recent Tasks",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if self.recent_tasks.is_empty() {
            content.push(Line::from("  No tasks recorded"));
        } else {
            for task in &self.recent_tasks {
                let elapsed = Self::format_elapsed_time(task.elapsed_secs);
                content.push(Line::from(format!(
                    "  #{} [{} | {}] {}",
                    task.task_id,
                    Self::task_status_label(&task.status),
                    elapsed,
                    task.prompt_preview
                )));
            }
        }

        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "Google Drive Authentication",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if self.auth_status.google_drive_authenticated {
            let mut line = vec![
                Span::raw("  "),
                Span::styled("[OK]", Style::default().fg(Color::Green)),
                Span::raw(" Authenticated"),
            ];

            if let Some(days) = self.auth_status.token_expires_in_days {
                if days > 0 {
                    line.push(Span::styled(
                        format!(" (expires in {days} days)"),
                        Style::default().fg(Color::Gray),
                    ));
                } else {
                    line.push(Span::styled(
                        " (token expired)",
                        Style::default().fg(Color::Red),
                    ));
                }
            }

            content.push(Line::from(line));
        } else {
            content.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("[!!]", Style::default().fg(Color::Red)),
                Span::raw(" Not authenticated"),
            ]));
        }

        let content_widget =
            Paragraph::new(content).block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(content_widget, chunks[1]);

        // Footer (help + status messages)
        let mut footer_lines = vec![Line::from(vec![Span::styled(
            "[P] Providers  [S] Status  [I] Install  [Q] Quit",
            Style::default().fg(Color::Cyan),
        )])];

        if !self.status_messages.is_empty() {
            footer_lines.push(Line::from(""));
            for message in &self.status_messages {
                footer_lines.push(Line::from(vec![
                    Span::styled("! ", Style::default().fg(Color::Yellow)),
                    Span::raw(message),
                ]));
            }
        }

        if !self.install_messages.is_empty() {
            footer_lines.push(Line::from(""));
            for message in &self.install_messages {
                footer_lines.push(Line::from(vec![
                    Span::styled("→ ", Style::default().fg(Color::Green)),
                    Span::raw(message),
                ]));
            }
        }

        let footer = Paragraph::new(footer_lines)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match key.code {
            KeyCode::Char('p') | KeyCode::Char('P') => {
                Ok(ScreenAction::SwitchTo(ScreenType::Provider))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Ok(ScreenAction::SwitchTo(ScreenType::Status))
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.toggle_install_guidance();
                Ok(ScreenAction::None)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Ok(ScreenAction::Quit),
            _ => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        if self.last_update.elapsed() >= Duration::from_secs(2) {
            self.refresh_data()?;
            self.last_update = Instant::now();
        }
        Ok(())
    }
}
