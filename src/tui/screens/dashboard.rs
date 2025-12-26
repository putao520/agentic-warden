//! Dashboard screen implementation
//!
//! Displays AI CLI status, default provider, and a summary of running tasks.

use anyhow::Result;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::cli_manager::{CliToolDetector, InstallType};
use crate::mcp_routing::config::McpConfigManager;
use crate::roles::{builtin, RoleManager};
use crate::tui::app_state::{AppState, TaskUiState};

use super::{Screen, ScreenAction, ScreenType};

const DASHBOARD_REFRESH_INTERVAL_SECS: i64 = 5;
const MAX_TASKS_DISPLAYED: usize = 5;

#[derive(Debug, Clone)]
struct AiCliStatus {
    name: String,
    command: String,
    installed: bool,
    version: Option<String>,
    install_type: Option<InstallType>,
}

#[derive(Debug, Clone)]
struct TaskSummary {
    pid: u32,
    manager_pid: Option<u32>,
    log_id: String,
    started_at: DateTime<Utc>,
    status: TaskUiState,
}

#[derive(Debug, Default)]
struct DashboardState {
    cli_status: Vec<AiCliStatus>,
    default_provider: Option<String>,
    running_tasks: Vec<TaskSummary>,
    total_running_tasks: usize,
    system_overview: SystemOverview,
}

#[derive(Debug, Clone, Default)]
struct SystemOverview {
    mcp_enabled: usize,
    mcp_total: usize,
    roles_builtin: usize,
    roles_custom: usize,
}

pub struct DashboardScreen {
    state: DashboardState,
    last_refresh: Option<DateTime<Utc>>,
    last_error: Option<String>,
    app_state: &'static AppState,
}

impl DashboardScreen {
    pub fn new() -> Result<Self> {
        let mut screen = Self {
            state: DashboardState::default(),
            last_refresh: None,
            last_error: None,
            app_state: AppState::global(),
        };
        screen.refresh_cli_state();
        screen.refresh_dynamic_state();
        Ok(screen)
    }

    /// Create a new dashboard screen for testing without CLI detection
    #[cfg(test)]
    fn new_for_test() -> Result<Self> {
        Ok(Self {
            state: DashboardState::default(),
            last_refresh: None,
            last_error: None,
            app_state: AppState::global(),
        })
    }

    fn refresh_cli_state(&mut self) {
        match Self::fetch_cli_status() {
            Ok(cli_status) => {
                self.state.cli_status = cli_status;
                self.last_refresh = Some(Utc::now());
                self.last_error = None;
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
            }
        }
    }

    fn refresh_dynamic_state(&mut self) {
        self.state.default_provider = self.resolve_default_provider();
        let (running_tasks, total_running_tasks) = self.collect_running_tasks();
        self.state.running_tasks = running_tasks;
        self.state.total_running_tasks = total_running_tasks;
        self.state.system_overview = self.collect_system_overview();
    }

    fn resolve_default_provider(&self) -> Option<String> {
        if let Some(provider) = self.app_state.default_provider() {
            return Some(provider);
        }
        // Use ProviderManager to load or create default config
        use crate::provider::manager::ProviderManager;
        ProviderManager::new()
            .ok()
            .map(|manager| manager.get_providers_config().default_provider.clone())
    }

    fn collect_running_tasks(&self) -> (Vec<TaskSummary>, usize) {
        let mut snapshots = self.app_state.tasks_snapshot();
        snapshots.sort_by(|a, b| b.record.started_at.cmp(&a.record.started_at));
        let total_running = snapshots
            .iter()
            .filter(|snapshot| matches!(snapshot.status, TaskUiState::Running))
            .count();
        let running = snapshots
            .into_iter()
            .filter(|snapshot| matches!(snapshot.status, TaskUiState::Running))
            .take(MAX_TASKS_DISPLAYED)
            .map(|snapshot| TaskSummary {
                pid: snapshot.pid,
                manager_pid: snapshot.record.manager_pid,
                log_id: snapshot.record.log_id.clone(),
                started_at: snapshot.record.started_at,
                status: snapshot.status,
            })
            .collect();

        (running, total_running)
    }

    fn collect_system_overview(&self) -> SystemOverview {
        // Collect MCP server stats
        let (mcp_enabled, mcp_total) = match McpConfigManager::load() {
            Ok(manager) => {
                let total = manager.config().mcp_servers.len();
                let enabled = manager.enabled_servers().len();
                (enabled, total)
            }
            Err(_) => (0, 0),
        };

        // Collect role stats
        let roles_builtin = builtin::list_builtin_roles().len();
        let roles_custom = RoleManager::new()
            .ok()
            .and_then(|manager| manager.list_all_roles().ok())
            .map(|roles| roles.len())
            .unwrap_or(0);

        SystemOverview {
            mcp_enabled,
            mcp_total,
            roles_builtin,
            roles_custom,
        }
    }

    fn fetch_cli_status() -> Result<Vec<AiCliStatus>> {
        let mut detector = CliToolDetector::new();
        detector.detect_all_tools()?;

        let mut status: Vec<AiCliStatus> = detector
            .get_tools()
            .iter()
            .map(|tool| AiCliStatus {
                name: tool.name.clone(),
                command: tool.command.clone(),
                installed: tool.installed,
                version: tool.version.clone(),
                install_type: tool.install_type.clone(),
            })
            .collect();

        status.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(status)
    }

    fn render_cli_status(&self, frame: &mut Frame, area: Rect) {
        let default_provider = self.state.default_provider.as_deref().unwrap_or("unknown");

        if self.state.cli_status.is_empty() {
            let paragraph = Paragraph::new("No AI CLI tools detected.")
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "AI CLI Status · Default Provider: {default_provider}"
                )))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
            return;
        }

        let rows = self.state.cli_status.iter().map(|status| {
            let installed_text = if status.installed {
                "Installed"
            } else {
                "Missing"
            };
            let version_text = status.version.as_deref().unwrap_or("-");
            let install_type = status
                .install_type
                .as_ref()
                .map(|ty| match ty {
                    InstallType::Native => "native",
                    InstallType::Npm => "npm",
                    InstallType::Unknown => "unknown",
                })
                .unwrap_or("-");

            Row::new(vec![
                Cell::from(format!("{} ({})", status.name, status.command)),
                Cell::from(installed_text),
                Cell::from(version_text.to_string()),
                Cell::from(install_type.to_string()),
            ])
        });

        let widths = [
            Constraint::Percentage(45),
            Constraint::Length(10),
            Constraint::Percentage(25),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["CLI", "Status", "Version", "Install"])
                    .style(Style::default().fg(Color::Cyan)),
            )
            .column_spacing(1)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "AI CLI Status · Default Provider: {default_provider}"
            )));

        frame.render_widget(table, area);
    }

    fn render_task_summary(&self, frame: &mut Frame, area: Rect) {
        let mut lines = Vec::new();

        if self.state.running_tasks.is_empty() {
            lines.push("No running tasks detected.".to_string());
        } else {
            for (index, task) in self.state.running_tasks.iter().enumerate() {
                let elapsed = format_duration(Utc::now() - task.started_at);
                let manager = task
                    .manager_pid
                    .map(|pid| format!(" parent {}", pid))
                    .unwrap_or_default();
                let status_label = match &task.status {
                    TaskUiState::Running => "Running".to_string(),
                    TaskUiState::Completed => "Completed".to_string(),
                    TaskUiState::Failed(reason) => format!("Failed: {}", truncate(reason, 16)),
                    TaskUiState::Pending => "Pending".to_string(),
                    TaskUiState::Paused => "Paused".to_string(),
                };
                lines.push(format!(
                    "{:>2}. PID {:>6} · {} · Log {} · {}{}",
                    index + 1,
                    task.pid,
                    status_label,
                    truncate(&task.log_id, 12),
                    elapsed,
                    manager
                ));
            }

            if self.state.total_running_tasks > self.state.running_tasks.len() {
                let remaining = self.state.total_running_tasks - self.state.running_tasks.len();
                lines.push(format!("… {remaining} more task(s) not shown"));
            }
        }

        let title = format!(
            "Task Overview · showing {} of {}",
            self.state.running_tasks.len(),
            self.state.total_running_tasks
        );

        let paragraph = Paragraph::new(lines.join("\n"))
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_shortcuts(&self, frame: &mut Frame, area: Rect) {
        let last_update = self
            .last_refresh
            .map(|ts| ts.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "never".to_string());

        let text = format!(
            "Shortcuts: P · Provider Management   S · Task Status   Q/Esc · Exit   Last update: {}",
            last_update
        );

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_error(&self, frame: &mut Frame, area: Rect) {
        if let Some(error) = &self.last_error {
            let paragraph = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::LightRed))
                .block(Block::default().borders(Borders::ALL).title("Warning"))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
    }

    fn render_system_overview(&self, frame: &mut Frame, area: Rect) {
        let overview = &self.state.system_overview;
        let mut lines = Vec::new();

        // MCP Servers line
        if overview.mcp_total > 0 {
            lines.push(format!(
                "MCP Servers: {} enabled ({} total)",
                overview.mcp_enabled, overview.mcp_total
            ));
        } else {
            lines.push("MCP Servers: not configured".to_string());
        }

        // Roles line
        let total_roles = overview.roles_builtin + overview.roles_custom;
        lines.push(format!(
            "Roles: {} available ({} builtin, {} custom)",
            total_roles, overview.roles_builtin, overview.roles_custom
        ));

        let paragraph = Paragraph::new(lines.join("\n"))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("System Overview"),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }
}

impl Screen for DashboardScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),
                Constraint::Length(9),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let mid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(layout[1]);

        self.render_cli_status(frame, layout[0]);
        self.render_task_summary(frame, mid[0]);
        self.render_system_overview(frame, mid[1]);
        self.render_shortcuts(frame, layout[2]);
        self.render_error(frame, layout[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('p') | KeyCode::Char('P') => {
                Ok(ScreenAction::SwitchTo(ScreenType::Provider))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Ok(ScreenAction::SwitchTo(ScreenType::Status))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Ok(ScreenAction::Quit),
            _ => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        self.refresh_dynamic_state();

        let should_refresh_cli = self
            .last_refresh
            .map(|ts| Utc::now() - ts > ChronoDuration::seconds(DASHBOARD_REFRESH_INTERVAL_SECS))
            .unwrap_or(true);

        if should_refresh_cli {
            self.refresh_cli_state();
        }

        Ok(())
    }
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else if max_len > 3 {
        format!("{}...", &text[..max_len - 3])
    } else {
        text[..max_len].to_string()
    }
}

fn format_duration(duration: ChronoDuration) -> String {
    let total_seconds = duration.num_seconds().max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_manager::InstallType;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};
    use std::env;
    use tempfile::TempDir;

    struct TempHome {
        #[allow(dead_code)]
        dir: TempDir,
        old_home: Option<std::ffi::OsString>,
        old_user: Option<std::ffi::OsString>,
    }

    impl TempHome {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("temp dir");
            let path = dir.path().to_path_buf();
            let old_home = env::var_os("HOME");
            env::set_var("HOME", &path);
            let old_user = env::var_os("USERPROFILE");
            if cfg!(windows) {
                env::set_var("USERPROFILE", &path);
            }
            Self {
                dir,
                old_home,
                old_user,
            }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            if let Some(old) = self.old_home.take() {
                env::set_var("HOME", old);
            } else {
                env::remove_var("HOME");
            }
            if cfg!(windows) {
                if let Some(old) = self.old_user.take() {
                    env::set_var("USERPROFILE", old);
                } else {
                    env::remove_var("USERPROFILE");
                }
            }
        }
    }

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.get(x, y);
                text.push_str(cell.symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn dashboard_screen_renders_sections() {
        let _home = TempHome::new();
        let mut screen = DashboardScreen::new_for_test().expect("screen should initialise");

        screen.state = DashboardState {
            cli_status: vec![AiCliStatus {
                name: "Claude CLI".into(),
                command: "claude".into(),
                installed: true,
                version: Some("1.2.3".into()),
                install_type: Some(InstallType::Native),
            }],
            default_provider: Some("openrouter".into()),
            running_tasks: vec![TaskSummary {
                pid: 42,
                manager_pid: Some(1),
                log_id: "log-1".into(),
                started_at: Utc::now(),
                status: TaskUiState::Running,
            }],
            total_running_tasks: 1,
            system_overview: SystemOverview::default(),
        };
        screen.last_refresh = Some(Utc::now());

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| screen.render(frame, frame.size()))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = buffer_to_string(buffer);
        assert!(
            rendered.contains("AI CLI Status")
                && rendered.contains("Task Overview")
                && rendered.contains("Claude CLI"),
            "unexpected render output:\n{rendered}"
        );
    }

    #[test]
    fn dashboard_key_handling_switches_screens_and_quits() {
        let _home = TempHome::new();
        let mut screen = DashboardScreen::new_for_test().expect("screen should initialise");

        let provider = screen
            .handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE))
            .expect("key handling should succeed");
        assert!(matches!(
            provider,
            ScreenAction::SwitchTo(ScreenType::Provider)
        ));

        let status = screen
            .handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE))
            .expect("key handling should succeed");
        assert!(matches!(status, ScreenAction::SwitchTo(ScreenType::Status)));

        let quit = screen
            .handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
            .expect("key handling should succeed");
        assert!(matches!(quit, ScreenAction::Quit));
    }
}
