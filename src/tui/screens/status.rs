//! Status monitoring screen

use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::{Screen, ScreenAction};
use crate::registry::TaskRegistry;
use crate::task_record::TaskStatus;
use crate::tui::app_state::{AppState, TaskSnapshot, TaskUiState};
use crate::tui::widgets::{DialogResult, DialogWidget};

/// Origin of a task grouping.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum GroupSource {
    Manager,
    RootParent,
    Unknown,
}

/// Key used to group tasks by their parent process.
#[derive(Debug, Clone, Eq, PartialEq)]
struct GroupKey {
    parent_pid: Option<u32>,
    parent_name: Option<String>,
    source: GroupSource,
}

impl Ord for GroupKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.source
            .cmp(&other.source)
            .then_with(|| self.parent_name.cmp(&other.parent_name))
            .then_with(|| self.parent_pid.cmp(&other.parent_pid))
    }
}

impl PartialOrd for GroupKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Group of tasks sharing the same parent process.
#[derive(Debug, Clone)]
struct TaskGroup {
    key: GroupKey,
    label: String,
    tasks: Vec<TaskSnapshot>,
}

/// Flat entry used for navigation within grouped tasks.
#[derive(Debug, Clone, Copy)]
struct FlatEntry {
    group_idx: usize,
    task_idx: usize,
}

/// Active mode for the status screen.
enum StatusMode {
    List,
    Dialog(DialogContext),
}

/// Context for an active dialog widget.
struct DialogContext {
    widget: DialogWidget,
    action: Option<DialogAction>,
}

impl DialogContext {
    fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            widget: DialogWidget::info(title.into(), message.into()),
            action: None,
        }
    }

    fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            widget: DialogWidget::error(title.into(), message.into()),
            action: None,
        }
    }

    fn confirm_kill(pid: u32, process_name: Option<&str>) -> Self {
        let display_name = process_name.unwrap_or("<unknown>");
        let message = format!("Terminate task {} (PID {})?", display_name, pid);
        Self {
            widget: DialogWidget::confirm("Confirm Kill".to_string(), message),
            action: Some(DialogAction::Kill(pid)),
        }
    }
}

/// Supported dialog actions.
#[derive(Debug, Clone, Copy)]
enum DialogAction {
    Kill(u32),
}

/// Status monitoring screen.
pub struct StatusScreen {
    registry: TaskRegistry,
    app_state: &'static AppState,
    last_update: Instant,
    groups: Vec<TaskGroup>,
    flat_entries: Vec<FlatEntry>,
    selected_index: usize,
    last_refresh_at: Option<DateTime<Utc>>,
    mode: StatusMode,
}

impl StatusScreen {
    pub fn new() -> Result<Self> {
        let registry = TaskRegistry::connect()?;
        let app_state = AppState::global();

        let mut screen = Self {
            registry,
            app_state,
            last_update: Instant::now(),
            groups: Vec::new(),
            flat_entries: Vec::new(),
            selected_index: 0,
            last_refresh_at: None,
            mode: StatusMode::List,
        };

        screen.refresh_tasks()?;

        Ok(screen)
    }

    fn refresh_tasks(&mut self) -> Result<()> {
        self.app_state.refresh_tasks_from_registry(&self.registry)?;
        let state = self.app_state.task_state();
        self.apply_task_state(state);
        Ok(())
    }

    fn apply_task_state(&mut self, state: TaskUiState) {
        let previously_selected_pid = self.selected_task().map(|task| task.pid);

        self.groups = Self::group_snapshots(state.tasks);
        self.flat_entries = Self::flatten_indices(&self.groups);
        self.last_refresh_at = state.last_refresh;

        if let Some(pid) = previously_selected_pid {
            if let Some(idx) = self
                .flat_entries
                .iter()
                .position(|entry| self.groups[entry.group_idx].tasks[entry.task_idx].pid == pid)
            {
                self.selected_index = idx;
            } else {
                self.selected_index = 0;
            }
        } else {
            self.selected_index = 0;
        }

        if !self.flat_entries.is_empty() && self.selected_index >= self.flat_entries.len() {
            self.selected_index = self.flat_entries.len() - 1;
        }
    }

    fn group_snapshots(snapshots: Vec<TaskSnapshot>) -> Vec<TaskGroup> {
        let mut groups: BTreeMap<GroupKey, Vec<TaskSnapshot>> = BTreeMap::new();

        for snapshot in snapshots {
            let (source, parent_pid, parent_name) = if let Some(pid) = snapshot.manager_pid {
                (
                    GroupSource::Manager,
                    Some(pid),
                    snapshot.manager_name.clone(),
                )
            } else if let Some(pid) = snapshot.root_parent_pid {
                (
                    GroupSource::RootParent,
                    Some(pid),
                    snapshot.root_parent_name.clone(),
                )
            } else {
                (GroupSource::Unknown, None, None)
            };

            let key = GroupKey {
                parent_pid,
                parent_name,
                source,
            };

            groups.entry(key).or_default().push(snapshot);
        }

        groups
            .into_iter()
            .map(|(key, mut tasks)| {
                tasks.sort_by(|a, b| b.started_at.cmp(&a.started_at));
                let label = Self::build_group_label(&key, tasks.len());

                TaskGroup { key, label, tasks }
            })
            .collect()
    }

    fn build_group_label(key: &GroupKey, count: usize) -> String {
        let role = match key.source {
            GroupSource::Manager => "Manager",
            GroupSource::RootParent => "Root Parent",
            GroupSource::Unknown => "Standalone",
        };
        let unit = if count == 1 { "task" } else { "tasks" };

        match (key.parent_pid, key.parent_name.as_deref()) {
            (Some(pid), Some(name)) if !name.is_empty() => {
                format!("{role}: {name} (PID {pid}) - {count} {unit}")
            }
            (Some(pid), _) => format!("{role}: PID {pid} - {count} {unit}"),
            (None, _) => format!("{role} tasks - {count} {unit}"),
        }
    }

    fn flatten_indices(groups: &[TaskGroup]) -> Vec<FlatEntry> {
        let mut entries = Vec::new();

        for (group_idx, group) in groups.iter().enumerate() {
            for (task_idx, _) in group.tasks.iter().enumerate() {
                entries.push(FlatEntry {
                    group_idx,
                    task_idx,
                });
            }
        }

        entries
    }

    fn selected_task(&self) -> Option<&TaskSnapshot> {
        let entry = self.flat_entries.get(self.selected_index)?;
        self.groups.get(entry.group_idx)?.tasks.get(entry.task_idx)
    }

    fn move_selection_up(&mut self) {
        if self.flat_entries.is_empty() || self.selected_index == 0 {
            return;
        }
        self.selected_index -= 1;
    }

    fn move_selection_down(&mut self) {
        if self.flat_entries.is_empty() || self.selected_index + 1 >= self.flat_entries.len() {
            return;
        }
        self.selected_index += 1;
    }

    fn counts(&self) -> (usize, usize, usize) {
        let mut total: usize = 0;
        let mut running: usize = 0;

        for group in &self.groups {
            for task in &group.tasks {
                total += 1;
                if task.status == TaskStatus::Running {
                    running += 1;
                }
            }
        }

        let completed = total.saturating_sub(running);
        (total, running, completed)
    }

    fn status_label(status: &TaskStatus) -> (&'static str, Color) {
        match status {
            TaskStatus::Running => ("[RUN]", Color::Green),
            TaskStatus::CompletedButUnread => ("[DONE]", Color::Blue),
        }
    }

    fn status_description(status: &TaskStatus) -> &'static str {
        match status {
            TaskStatus::Running => "Running",
            TaskStatus::CompletedButUnread => "Completed (unread)",
        }
    }

    fn format_elapsed(started: &DateTime<Utc>, completed: Option<&DateTime<Utc>>) -> String {
        let now = Utc::now();
        let end = completed.unwrap_or(&now);
        let mut seconds = end.timestamp() - started.timestamp();

        if seconds < 0 {
            seconds = 0;
        }

        if seconds < 60 {
            format!("{seconds}s")
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
        }
    }

    fn format_last_refresh(last_refresh: Option<DateTime<Utc>>) -> String {
        match last_refresh {
            Some(utc) => {
                let local: DateTime<Local> = DateTime::from(utc);
                format!("Last refresh: {}", local.format("%Y-%m-%d %H:%M:%S"))
            }
            None => "Last refresh: pending".to_string(),
        }
    }

    fn truncate(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else if max_len <= 3 {
            ".".repeat(max_len)
        } else {
            format!("{}...", &text[..max_len - 3])
        }
    }

    fn build_detail_lines(task: &TaskSnapshot) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        lines.push(Self::detail_line(
            "Process",
            task.process_name
                .as_deref()
                .unwrap_or("<unknown>")
                .to_string(),
        ));
        lines.push(Self::detail_line("PID", task.pid.to_string()));
        lines.push(Self::detail_line(
            "Status",
            Self::status_description(&task.status).to_string(),
        ));

        let started_local: DateTime<Local> = DateTime::from(task.started_at);
        lines.push(Self::detail_line(
            "Started",
            started_local.format("%Y-%m-%d %H:%M:%S").to_string(),
        ));
        lines.push(Self::detail_line(
            "Elapsed",
            Self::format_elapsed(&task.started_at, task.completed_at.as_ref()),
        ));

        if let Some(completed_at) = task.completed_at {
            let local_completed: DateTime<Local> = DateTime::from(completed_at);
            lines.push(Self::detail_line(
                "Completed",
                local_completed.format("%Y-%m-%d %H:%M:%S").to_string(),
            ));
        }

        if let Some(pid) = task.manager_pid {
            let name = task.manager_name.as_deref().unwrap_or("<unknown>");
            lines.push(Self::detail_line("Manager", format!("{name} (PID {pid})")));
        }

        if let Some(pid) = task.root_parent_pid {
            let name = task.root_parent_name.as_deref().unwrap_or("<unknown>");
            lines.push(Self::detail_line(
                "Root Parent",
                format!("{name} (PID {pid})"),
            ));
        }

        if !task.process_chain.is_empty() {
            let chain = task
                .process_chain
                .iter()
                .map(|pid| pid.to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            lines.push(Self::detail_line("Process Chain", chain));
        }

        lines.push(Self::detail_line("Log Path", task.log_path.clone()));

        let result_value = task
            .result
            .as_deref()
            .map(|r| r.to_string())
            .unwrap_or_else(|| "<pending>".to_string());
        lines.push(Self::detail_line("Result", result_value));

        let exit_code_value = task
            .exit_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "<pending>".to_string());
        lines.push(Self::detail_line("Exit Code", exit_code_value));

        if let Some(reason) = &task.cleanup_reason {
            lines.push(Self::detail_line("Cleanup", reason.clone()));
        }

        lines
    }

    fn detail_line(label: &str, value: String) -> Line<'static> {
        Line::from(vec![
            Span::styled(
                format!("{label}: "),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(value),
        ])
    }

    fn execute_dialog_action(&mut self, action: DialogAction) -> Result<()> {
        match action {
            DialogAction::Kill(pid) => {
                match Self::kill_process(pid) {
                    Ok(()) => {
                        // Attempt to refresh immediately so the UI reflects the change.
                        self.refresh_tasks()?;
                        self.last_update = Instant::now();
                        self.mode = StatusMode::Dialog(DialogContext::info(
                            "Task Terminated",
                            format!("Process {pid} terminated successfully."),
                        ));
                    }
                    Err(err) => {
                        self.mode = StatusMode::Dialog(DialogContext::error(
                            "Kill Failed",
                            format!("Failed to terminate {pid}: {err}"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn kill_process(pid: u32) -> Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;
            kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?;
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output()?;
        }

        Ok(())
    }
}

impl Screen for StatusScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            StatusMode::List => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                let (total, running, completed) = self.counts();
                let summary = format!(
                    "Tasks: {total} (Running: {running}, Completed: {completed})    {}",
                    Self::format_last_refresh(self.last_refresh_at)
                );

                let title = Paragraph::new(summary)
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, layout[0]);

                let body = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .split(layout[1]);

                if self.groups.is_empty() {
                    let content = Paragraph::new("No active tasks detected.")
                        .alignment(Alignment::Center)
                        .block(Block::default().borders(Borders::ALL).title("Tasks"));
                    frame.render_widget(content, body[0]);
                } else {
                    let mut items = Vec::new();

                    for (group_idx, group) in self.groups.iter().enumerate() {
                        let header = ListItem::new(Line::from(vec![Span::styled(
                            group.label.clone(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )]));
                        items.push(header);

                        for (task_idx, task) in group.tasks.iter().enumerate() {
                            let is_selected = self
                                .flat_entries
                                .get(self.selected_index)
                                .map(|entry| {
                                    entry.group_idx == group_idx && entry.task_idx == task_idx
                                })
                                .unwrap_or(false);

                            let (status_icon, status_color) = Self::status_label(&task.status);
                            let elapsed =
                                Self::format_elapsed(&task.started_at, task.completed_at.as_ref());
                            let name = task.process_name.as_deref().unwrap_or("<unknown process>");
                            let prompt = Self::truncate(&task.log_id, 40);
                            let prefix = if is_selected { "> " } else { "  " };

                            let content = vec![
                                Span::raw(prefix),
                                Span::styled(status_icon, Style::default().fg(status_color)),
                                Span::raw(" "),
                                Span::raw(format!("{name} (PID {}) ", task.pid)),
                                Span::styled(
                                    format!("[{}]", elapsed),
                                    Style::default().fg(Color::Gray),
                                ),
                                Span::raw(" "),
                                Span::raw(prompt),
                            ];

                            let style = if is_selected {
                                Style::default().add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            };

                            items.push(ListItem::new(Line::from(content)).style(style));
                        }
                    }

                    let list = List::new(items)
                        .block(Block::default().borders(Borders::ALL).title("Tasks"))
                        .highlight_style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        );

                    frame.render_widget(list, body[0]);
                }

                let details = if let Some(task) = self.selected_task() {
                    Paragraph::new(Self::build_detail_lines(task))
                        .wrap(Wrap { trim: true })
                        .block(Block::default().borders(Borders::ALL).title("Details"))
                } else {
                    Paragraph::new("No task selected.")
                        .block(Block::default().borders(Borders::ALL).title("Details"))
                };

                frame.render_widget(details, body[1]);

                let help = Paragraph::new("[Up/Down] Navigate  [R] Refresh  [K] Kill  [ESC] Back")
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, layout[2]);
            }
            StatusMode::Dialog(context) => {
                context.widget.render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match &mut self.mode {
            StatusMode::List => match key.code {
                KeyCode::Up => {
                    self.move_selection_up();
                    Ok(ScreenAction::None)
                }
                KeyCode::Down => {
                    self.move_selection_down();
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.refresh_tasks()?;
                    self.last_update = Instant::now();
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('k') | KeyCode::Char('K') => {
                    if let Some(task) = self.selected_task() {
                        let dialog =
                            DialogContext::confirm_kill(task.pid, task.process_name.as_deref());
                        self.mode = StatusMode::Dialog(dialog);
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            StatusMode::Dialog(context) => {
                let action = context.action;
                let result = context.widget.handle_key(key);

                match result {
                    DialogResult::Confirmed => {
                        self.mode = StatusMode::List;
                        if let Some(action) = action {
                            self.execute_dialog_action(action)?;
                        }
                        Ok(ScreenAction::None)
                    }
                    DialogResult::Cancelled | DialogResult::Closed => {
                        self.mode = StatusMode::List;
                        Ok(ScreenAction::None)
                    }
                    DialogResult::None => Ok(ScreenAction::None),
                }
            }
        }
    }

    fn update(&mut self) -> Result<()> {
        if self.last_update.elapsed() >= Duration::from_secs(2) {
            self.refresh_tasks()?;
            self.last_update = Instant::now();
        }
        Ok(())
    }
}
