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
use std::time::{Duration, Instant};

use super::{Screen, ScreenAction, ScreenType};

use crate::task_record::TaskStatus;
use crate::cli_manager::CliManager;

/// AI CLI status information
#[derive(Debug, Clone)]
struct AiCliStatus {
    name: String,
    installed: bool,
    version: Option<String>,
    default_provider: Option<String>,
}

/// Task summary for dashboard
#[derive(Debug, Clone)]
struct TaskSummary {
    task_id: u32,
    status: TaskStatus,
    prompt_preview: String,
    elapsed_secs: u64,
}

/// Authentication status
#[derive(Debug, Clone)]
struct AuthStatus {
    google_drive_authenticated: bool,
    token_expires_in_days: Option<i64>,
}

/// Dashboard screen
pub struct DashboardScreen {
    last_update: Instant,
    ai_cli_status: Vec<AiCliStatus>,
    running_tasks: Vec<TaskSummary>,
    auth_status: AuthStatus,
}

impl DashboardScreen {
    pub fn new() -> Result<Self> {
        let mut screen = Self {
            last_update: Instant::now(),
            ai_cli_status: Vec::new(),
            running_tasks: Vec::new(),
            auth_status: AuthStatus {
                google_drive_authenticated: false,
                token_expires_in_days: None,
            },
        };

        // Initial data load
        screen.refresh_data()?;

        Ok(screen)
    }

    fn detect_ai_cli_status() -> Vec<AiCliStatus> {
        // Try to use CliManager for comprehensive detection
        let _cli_manager = CliManager::new().ok();
        // Note: CliManager integration can be enhanced later to fully use its detection capabilities

        let cli_names = vec![
            ("codex", "Codex"),
            ("claude", "Claude"),
            ("gemini", "Gemini"),
        ];

        cli_names
            .into_iter()
            .map(|(cmd, name)| {
                let installed = which::which(cmd).is_ok();
                let version = if installed {
                    Self::get_cli_version(cmd)
                } else {
                    None
                };
                let default_provider = if installed {
                    Self::get_default_provider_for_cli(name)
                } else {
                    None
                };

                AiCliStatus {
                    name: name.to_string(),
                    installed,
                    version,
                    default_provider,
                }
            })
            .collect()
    }

    fn get_cli_version(cmd: &str) -> Option<String> {
        // Try to get version using --version flag
        if let Ok(output) = std::process::Command::new(cmd).arg("--version").output() {
            if output.status.success() {
                let version_str = String::from_utf8_lossy(&output.stdout);
                // Extract version number (usually first line)
                return version_str.lines().next().map(|s| s.trim().to_string());
            }
        }
        None
    }

    fn get_default_provider_for_cli(_cli_name: &str) -> Option<String> {
        // Try to read default provider from provider.json
        if let Ok(provider_manager) = crate::provider::ProviderManager::new() {
            if let Some((name, _)) = provider_manager.get_default_provider() {
                return Some(name.clone());
            }
        }
        None
    }

    fn get_running_tasks() -> Result<Vec<TaskSummary>> {
        let registry = crate::registry::TaskRegistry::connect()?;
        let all_entries = registry.entries()?;

        let tasks: Vec<TaskSummary> = all_entries
            .into_iter()
            .filter(|entry| matches!(entry.record.status, TaskStatus::Running))
            .take(5) // Show max 5 tasks
            .map(|entry| {
                let prompt_preview = Self::truncate_prompt(&entry.record.log_id, 50);
                let elapsed_secs =
                    (chrono::Utc::now().timestamp() - entry.record.started_at.timestamp()) as u64;

                TaskSummary {
                    task_id: entry.pid,
                    status: entry.record.status,
                    prompt_preview,
                    elapsed_secs,
                }
            })
            .collect();

        Ok(tasks)
    }

    fn truncate_prompt(prompt: &str, max_len: usize) -> String {
        if prompt.len() <= max_len {
            prompt.to_string()
        } else {
            format!("{}...", &prompt[..max_len.saturating_sub(3)])
        }
    }

    fn get_auth_status() -> Result<AuthStatus> {
        let auth_path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?
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
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        let token_expires_in_days =
            auth_state
                .get("expires_at")
                .and_then(|v| v.as_i64())
                .map(|expires_at| {
                    let now = chrono::Utc::now().timestamp();
                    let days = (expires_at - now) / 86400;
                    days
                });

        Ok(AuthStatus {
            google_drive_authenticated: authenticated,
            token_expires_in_days,
        })
    }

    fn refresh_data(&mut self) -> Result<()> {
        self.ai_cli_status = Self::detect_ai_cli_status();
        self.running_tasks = Self::get_running_tasks().unwrap_or_default();
        self.auth_status = Self::get_auth_status().unwrap_or(AuthStatus {
            google_drive_authenticated: false,
            token_expires_in_days: None,
        });
        Ok(())
    }

    fn format_elapsed_time(secs: u64) -> String {
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }
}

impl Screen for DashboardScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
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
                let status_icon = if cli.installed { "✓" } else { "✗" };
                let status_color = if cli.installed {
                    Color::Green
                } else {
                    Color::Red
                };

                let mut line_parts = vec![
                    Span::raw("  "),
                    Span::styled(status_icon, Style::default().fg(status_color)),
                    Span::raw(format!(" {} ", cli.name)),
                ];

                if let Some(version) = &cli.version {
                    line_parts.push(Span::styled(
                        format!("({})", version),
                        Style::default().fg(Color::Gray),
                    ));
                }

                if let Some(provider) = &cli.default_provider {
                    line_parts.push(Span::raw(format!(" → {}", provider)));
                }

                content.push(Line::from(line_parts));
            }
        }

        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "Running Tasks",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if self.running_tasks.is_empty() {
            content.push(Line::from("  No tasks running"));
        } else {
            for task in &self.running_tasks {
                let elapsed = Self::format_elapsed_time(task.elapsed_secs);
                content.push(Line::from(format!(
                    "  #{} [{}] {}",
                    task.task_id, elapsed, task.prompt_preview
                )));
            }
        }

        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "Authorization Status",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if self.auth_status.google_drive_authenticated {
            let mut auth_line = vec![
                Span::raw("  "),
                Span::styled("✓", Style::default().fg(Color::Green)),
                Span::raw(" Google Drive: Authenticated"),
            ];

            if let Some(days) = self.auth_status.token_expires_in_days {
                if days > 0 {
                    auth_line.push(Span::styled(
                        format!(" (expires in {} days)", days),
                        Style::default().fg(Color::Gray),
                    ));
                } else {
                    auth_line.push(Span::styled(
                        " (token expired)",
                        Style::default().fg(Color::Red),
                    ));
                }
            }

            content.push(Line::from(auth_line));
        } else {
            content.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("✗", Style::default().fg(Color::Red)),
                Span::raw(" Google Drive: Not authenticated"),
            ]));
        }

        let content_widget =
            Paragraph::new(content).block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(content_widget, chunks[1]);

        // Help
        let help = Paragraph::new("[P] Providers  [S] Status  [O] OAuth  [Q] Quit")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match key.code {
            KeyCode::Char('p') | KeyCode::Char('P') => {
                Ok(ScreenAction::SwitchTo(ScreenType::Provider))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Ok(ScreenAction::SwitchTo(ScreenType::Status))
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                Ok(ScreenAction::SwitchTo(ScreenType::OAuth))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Ok(ScreenAction::Quit),
            _ => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        // Auto-refresh every 2 seconds
        if self.last_update.elapsed() >= Duration::from_secs(2) {
            self.refresh_data()?;
            self.last_update = Instant::now();
        }
        Ok(())
    }
}
