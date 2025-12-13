//! Task status screen
//!
//! Displays running tasks grouped by their parent process and supports
//! keyboard navigation per SPEC/API.md §3.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::{Screen, ScreenAction};
use crate::platform;
use crate::registry_factory::{create_cli_registry, CliRegistry};
use crate::task_record::{TaskRecord, TaskStatus};
use crate::tui::app_state::{AppState, TaskSnapshot};

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

pub struct StatusScreen {
    registry: CliRegistry,
    app_state: &'static AppState,
    groups: Vec<TaskGroup>,
    flat_entries: Vec<FlatEntry>,
    selected_index: usize,
    last_refresh: Instant,
    last_loaded_at: Option<DateTime<Utc>>,
    message: Option<String>,
}

#[derive(Clone)]
struct TaskItem {
    pid: u32,
    record: TaskRecord,
}

#[derive(Clone)]
struct TaskGroup {
    label: String,
    tasks: Vec<TaskItem>,
}

#[derive(Clone, Copy)]
struct FlatEntry {
    group_idx: usize,
    task_idx: usize,
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum GroupKey {
    Manager(u32),
    Root(u32),
    Standalone,
}

impl StatusScreen {
    pub fn new() -> Result<Self> {
        let registry = create_cli_registry()?;
        let mut screen = Self {
            registry,
            app_state: AppState::global(),
            groups: Vec::new(),
            flat_entries: Vec::new(),
            selected_index: 0,
            last_refresh: Instant::now()
                .checked_sub(REFRESH_INTERVAL)
                .unwrap_or_else(Instant::now),
            last_loaded_at: None,
            message: None,
        };

        screen.sync_from_registry()?;
        screen.refresh_tasks()?;
        Ok(screen)
    }

    fn refresh_tasks(&mut self) -> Result<()> {
        let snapshots = self.app_state.tasks_snapshot();
        let tasks = Self::convert_snapshots(snapshots);
        self.groups = Self::group_tasks(tasks);
        self.flat_entries = Self::build_flat_index(&self.groups);
        if self.selected_index >= self.flat_entries.len() && !self.flat_entries.is_empty() {
            self.selected_index = self.flat_entries.len() - 1;
        }
        self.last_loaded_at = Some(Utc::now());
        Ok(())
    }

    fn convert_snapshots(snapshots: Vec<TaskSnapshot>) -> Vec<TaskItem> {
        snapshots
            .into_iter()
            .map(|snapshot| TaskItem {
                pid: snapshot.pid,
                record: snapshot.record,
            })
            .collect()
    }

    fn sync_from_registry(&mut self) -> Result<()> {
        let entries = self.registry.entries()?;
        self.app_state.replace_tasks_from_registry(entries);
        Ok(())
    }

    fn group_tasks(tasks: Vec<TaskItem>) -> Vec<TaskGroup> {
        let mut groups: BTreeMap<GroupKey, Vec<TaskItem>> = BTreeMap::new();

        for task in tasks {
            let key = if let Some(pid) = task.record.manager_pid {
                GroupKey::Manager(pid)
            } else if let Some(pid) = task.record.resolved_root_parent_pid() {
                GroupKey::Root(pid)
            } else {
                GroupKey::Standalone
            };

            groups.entry(key).or_default().push(task);
        }

        groups
            .into_iter()
            .map(|(key, mut tasks)| {
                tasks.sort_by(|a, b| b.record.started_at.cmp(&a.record.started_at));
                let label = match key {
                    GroupKey::Manager(pid) => format!("Manager PID {pid}"),
                    GroupKey::Root(pid) => format!("Parent PID {pid}"),
                    GroupKey::Standalone => "Standalone Tasks".to_string(),
                };
                TaskGroup { label, tasks }
            })
            .collect()
    }

    fn build_flat_index(groups: &[TaskGroup]) -> Vec<FlatEntry> {
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

    fn counts(&self) -> (usize, usize, usize) {
        let total = self
            .groups
            .iter()
            .map(|group| group.tasks.len())
            .sum::<usize>();
        let running = self
            .groups
            .iter()
            .flat_map(|group| group.tasks.iter())
            .filter(|task| task.record.status == TaskStatus::Running)
            .count();
        let completed = total.saturating_sub(running);
        (total, running, completed)
    }

    fn selected_task(&self) -> Option<&TaskItem> {
        let entry = self.flat_entries.get(self.selected_index)?;
        self.groups.get(entry.group_idx)?.tasks.get(entry.task_idx)
    }

    fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        if self.selected_index + 1 < self.flat_entries.len() {
            self.selected_index += 1;
        }
    }

    fn format_elapsed(record: &TaskRecord) -> String {
        let end = record.completed_at.unwrap_or_else(Utc::now);
        let mut seconds = (end - record.started_at).num_seconds();
        if seconds < 0 {
            seconds = 0;
        }
        let minutes = seconds / 60;
        let hours = minutes / 60;
        if hours > 0 {
            format!("{hours}h {}m", minutes % 60)
        } else if minutes > 0 {
            format!("{minutes}m {}s", seconds % 60)
        } else {
            format!("{seconds}s")
        }
    }

    fn summary_line(&self) -> String {
        let (total, running, completed) = self.counts();
        let refreshed = match self.last_loaded_at {
            Some(ts) => {
                let local: DateTime<Local> = DateTime::from(ts);
                format!("Last refresh: {}", local.format("%Y-%m-%d %H:%M:%S"))
            }
            None => "Last refresh: pending".to_string(),
        };
        format!(
            "Tasks: {} (Running: {}, Completed: {})    {}",
            total, running, completed, refreshed
        )
    }

    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = Vec::new();
        for (group_idx, group) in self.groups.iter().enumerate() {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                group.label.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )])));

            for (task_idx, task) in group.tasks.iter().enumerate() {
                let is_selected = self
                    .flat_entries
                    .get(self.selected_index)
                    .map(|entry| entry.group_idx == group_idx && entry.task_idx == task_idx)
                    .unwrap_or(false);

                let (status_label, status_color) = match task.record.status {
                    TaskStatus::Running => ("RUN", Color::Green),
                    TaskStatus::CompletedButUnread => ("DONE", Color::Blue),
                };

                let prefix = if is_selected { "> " } else { "  " };
                let elapsed = Self::format_elapsed(&task.record);
                let log = truncate(&task.record.log_id, 40);

                let content = Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(status_label, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::raw(format!("PID {} ", task.pid)),
                    Span::styled(format!("[{}]", elapsed), Style::default().fg(Color::Gray)),
                    Span::raw(" "),
                    Span::raw(log),
                ]);

                let style = if is_selected {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                items.push(ListItem::new(content).style(style));
            }
        }

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Tasks"));
        frame.render_widget(list, area);
    }

    fn render_details(&self, frame: &mut Frame, area: Rect) {
        if let Some(task) = self.selected_task() {
            let record = &task.record;
            let started_local: DateTime<Local> = DateTime::from(record.started_at);
            let completed_local = record.completed_at.map(DateTime::<Local>::from);

            let mut lines = Vec::new();
            lines.push(detail_line("PID", task.pid.to_string()));
            lines.push(detail_line(
                "Status",
                match record.status {
                    TaskStatus::Running => "Running",
                    TaskStatus::CompletedButUnread => "Completed",
                }
                .to_string(),
            ));
            lines.push(detail_line(
                "Started",
                started_local.format("%Y-%m-%d %H:%M:%S").to_string(),
            ));
            lines.push(detail_line("Elapsed", Self::format_elapsed(record)));
            if let Some(completed) = completed_local {
                lines.push(detail_line(
                    "Completed",
                    completed.format("%Y-%m-%d %H:%M:%S").to_string(),
                ));
            }
            if let Some(manager) = record.manager_pid {
                lines.push(detail_line("Manager PID", manager.to_string()));
            }
            if let Some(root) = record.resolved_root_parent_pid() {
                lines.push(detail_line("Parent PID", root.to_string()));
            }
            if let Some(ai_info) = record.ai_cli_process.as_ref() {
                let desc = ai_info.get_description();
                lines.push(detail_line("AI CLI Root", desc));
            }
            if !record.process_chain.is_empty() {
                let chain = record
                    .process_chain
                    .iter()
                    .map(|pid| pid.to_string())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                lines.push(detail_line("Process Chain", chain));
            }
            lines.push(detail_line("Log ID", record.log_id.clone()));
            lines.push(detail_line("Log Path", record.log_path.clone()));
            if let Some(reason) = &record.cleanup_reason {
                lines.push(detail_line("Cleanup", reason.clone()));
            }

            let paragraph = Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Details"));
            frame.render_widget(paragraph, area);
        } else {
            let paragraph = Paragraph::new("No task selected.")
                .block(Block::default().borders(Borders::ALL).title("Details"));
            frame.render_widget(paragraph, area);
        }
    }
}

impl Screen for StatusScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(area);

        let header = Paragraph::new(self.summary_line())
            .alignment(ratatui::layout::Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, layout[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(layout[1]);

        if self.flat_entries.is_empty() {
            let empty = Paragraph::new("No active tasks detected.")
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Tasks"));
            frame.render_widget(empty, body[0]);
        } else {
            self.render_list(frame, body[0]);
        }

        self.render_details(frame, body[1]);

        let help = Paragraph::new("[↑/↓] Navigate  [R] Refresh  [K] Kill  [ESC/Q] Back")
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, layout[2]);

        let status_text = self.message.as_deref().unwrap_or("Ready");
        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, layout[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match key.code {
            KeyCode::Up => {
                self.move_selection_up();
                Ok(ScreenAction::None)
            }
            KeyCode::Down => {
                self.move_selection_down();
                Ok(ScreenAction::None)
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.sync_from_registry()?;
                self.refresh_tasks()?;
                self.last_refresh = Instant::now();
                self.message = Some("Tasks refreshed.".to_string());
                Ok(ScreenAction::None)
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                if let Some(task) = self.selected_task() {
                    match Self::terminate_task(task.pid) {
                        Ok(()) => {
                            self.message =
                                Some(format!("Sent terminate signal to PID {}", task.pid));
                            self.sync_from_registry()?;
                            self.refresh_tasks()?;
                            self.last_refresh = Instant::now();
                        }
                        Err(err) => {
                            self.message =
                                Some(format!("Failed to terminate {}: {}", task.pid, err));
                        }
                    }
                }
                Ok(ScreenAction::None)
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Ok(ScreenAction::Back),
            _ => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        if self.last_refresh.elapsed() >= REFRESH_INTERVAL {
            self.sync_from_registry()?;
            self.refresh_tasks()?;
            self.last_refresh = Instant::now();
        }
        Ok(())
    }
}

impl StatusScreen {
    fn terminate_task(pid: u32) -> Result<()> {
        platform::terminate_process(pid);
        Ok(())
    }
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

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        format!("{}...", &text[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};

    fn sample_task(pid: u32, manager: Option<u32>, root: Option<u32>) -> TaskItem {
        use crate::core::models::ProcessTreeInfo;

        let base = TaskRecord::new(
            Utc::now(),
            format!("task-{pid}"),
            format!("/tmp/{pid}.log"),
            manager,
        );

        let record = if let Some(root_pid) = root {
            base.with_process_tree_info(ProcessTreeInfo::new(vec![pid, root_pid]))
                .expect("process tree injection")
        } else {
            base
        };

        TaskItem { pid, record }
    }

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer.get(x, y).symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn groups_tasks_by_manager_and_root_pid() {
        let tasks = vec![
            sample_task(10, Some(1), None),
            sample_task(11, Some(1), None),
            sample_task(20, None, Some(9)),
            sample_task(30, None, None),
        ];

        let groups = StatusScreen::group_tasks(tasks);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].label, "Manager PID 1");
        assert_eq!(groups[1].label, "Parent PID 9");
        assert_eq!(groups[2].label, "Standalone Tasks");
    }

    #[test]
    fn status_screen_handle_key_updates_selection_and_refreshes() {
        let mut screen = StatusScreen::new().expect("screen should initialise");
        screen.groups = vec![TaskGroup {
            label: "Manager PID 1".into(),
            tasks: vec![sample_task(10, Some(1), None)],
        }];
        screen.flat_entries = vec![FlatEntry {
            group_idx: 0,
            task_idx: 0,
        }];
        screen.selected_index = 0;

        let refresh = screen
            .handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(refresh, ScreenAction::None));
        assert!(screen.message.is_some());

        let back = screen
            .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(back, ScreenAction::Back));
    }

    #[test]
    fn status_screen_render_includes_task_details() {
        let mut screen = StatusScreen::new().expect("screen should initialise");
        screen.groups = vec![TaskGroup {
            label: "Manager PID 7".into(),
            tasks: vec![sample_task(100, Some(7), None)],
        }];
        screen.flat_entries = vec![FlatEntry {
            group_idx: 0,
            task_idx: 0,
        }];
        screen.selected_index = 0;
        screen.last_loaded_at = Some(Utc::now());

        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| screen.render(frame, frame.size()))
            .unwrap();

        let rendered = buffer_to_string(terminal.backend().buffer());
        assert!(
            rendered.contains("Manager PID 7") && rendered.contains("task-100"),
            "render output missing details:\n{rendered}"
        );
    }
}
