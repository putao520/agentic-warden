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
use std::time::{Duration as StdDuration, Instant};

use crate::cli_manager::{CliToolDetector, InstallType};
use crate::tui::app_state::{
    AppState, AuthStatus, GoogleDriveAuthSnapshot, SyncPhase, TaskUiState, TransferKind,
    TransferProgress,
};

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
    sync_overview: SyncOverview,
}

#[derive(Debug, Clone, Default)]
struct SyncOverview {
    drive: GoogleDriveAuthSnapshot,
    push: Option<TransferProgress>,
    pull: Option<TransferProgress>,
    oauth: Option<OAuthFlowSummary>,
}

#[derive(Debug, Clone)]
struct OAuthFlowSummary {
    provider: String,
    status: AuthStatus,
    updated_at: Instant,
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
        self.state.sync_overview = self.collect_sync_overview();
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

    fn collect_sync_overview(&self) -> SyncOverview {
        let drive = self.app_state.google_drive_auth_snapshot();
        let push = self.app_state.get_sync_progress(&TransferKind::Push);
        let pull = self.app_state.get_sync_progress(&TransferKind::Pull);
        let oauth_flow = self
            .app_state
            .recent_oauth_flows(StdDuration::from_secs(600))
            .into_iter()
            .next()
            .map(|flow| OAuthFlowSummary {
                provider: flow.provider,
                status: flow.dialog.status.clone(),
                updated_at: flow.updated_at,
            });

        SyncOverview {
            drive,
            push,
            pull,
            oauth: oauth_flow,
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

    fn render_sync_overview(&self, frame: &mut Frame, area: Rect) {
        let overview = &self.state.sync_overview;
        let mut lines = Vec::new();
        lines.push(self.describe_drive_status(&overview.drive));
        lines.push(self.describe_transfer_status("Push", overview.push.as_ref()));
        lines.push(self.describe_transfer_status("Pull", overview.pull.as_ref()));
        lines.push(self.describe_oauth_status(overview.oauth.as_ref()));

        let paragraph = Paragraph::new(lines.join("\n"))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Sync Activity"),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    fn describe_drive_status(&self, snapshot: &GoogleDriveAuthSnapshot) -> String {
        if let Some(err) = &snapshot.error {
            return format!("✗ Google Drive error: {}", truncate(err, 48));
        }

        if !snapshot.configured {
            return "✗ Google Drive credentials missing – open OAuth to connect.".to_string();
        }

        if !snapshot.has_refresh_token {
            return "⚠ Google Drive connected without refresh token – re-run OAuth.".to_string();
        }

        if let Some(expires_at) = snapshot.expires_at {
            let remaining = (expires_at - Utc::now()).num_minutes();
            if remaining > 0 {
                return format!("✓ Google Drive connected (token refreshes in {remaining}m)");
            }
        }

        "✓ Google Drive connected".to_string()
    }

    fn describe_transfer_status(&self, label: &str, progress: Option<&TransferProgress>) -> String {
        match progress {
            Some(progress) => {
                let phase = sync_phase_label(progress.phase);
                let mut message = progress
                    .message
                    .clone()
                    .unwrap_or_else(|| phase.to_string());
                if message.contains('\n') {
                    message = message.replace('\n', " ");
                }
                let message = truncate(&message, 48);
                format!("{label}: {phase} · {:>3}% · {}", progress.percent, message)
            }
            None => format!("{label}: idle"),
        }
    }

    fn describe_oauth_status(&self, summary: Option<&OAuthFlowSummary>) -> String {
        match summary {
            Some(flow) => {
                let status = match &flow.status {
                    AuthStatus::Waiting => "waiting for approval".to_string(),
                    AuthStatus::CallbackStarted => "processing authorization".to_string(),
                    AuthStatus::Authorized => "completed successfully".to_string(),
                    AuthStatus::Failed(reason) => {
                        format!("failed: {}", truncate(reason, 32))
                    }
                };
                let age = human_duration(flow.updated_at.elapsed());
                format!("OAuth ({}) updated {} ago – {}", flow.provider, age, status)
            }
            None => "OAuth: no recent activity".to_string(),
        }
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
        self.render_sync_overview(frame, mid[1]);
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

fn human_duration(duration: StdDuration) -> String {
    let secs = duration.as_secs();
    if secs >= 3600 {
        format!("{:02}h{:02}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{:02}m{:02}s", secs / 60, secs % 60)
    } else {
        format!("{:02}s", secs)
    }
}

fn sync_phase_label(phase: SyncPhase) -> &'static str {
    match phase {
        SyncPhase::Idle => "Idle",
        SyncPhase::Preparing => "Preparing",
        SyncPhase::Authentication => "Authenticating",
        SyncPhase::Listing => "Listing",
        SyncPhase::Compressing => "Compressing",
        SyncPhase::Uploading => "Uploading",
        SyncPhase::Downloading => "Downloading",
        SyncPhase::Verifying => "Verifying",
        SyncPhase::Applying => "Restoring",
        SyncPhase::Completed => "Completed",
        SyncPhase::Failed => "Failed",
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
            sync_overview: SyncOverview::default(),
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
