//! Status monitoring screen

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::time::{Duration, Instant};

use super::{Screen, ScreenAction};
use crate::registry::TaskRegistry;
use crate::task_record::TaskStatus;
use crate::tui::widgets::{DialogResult, DialogWidget};

/// Task display entry
#[derive(Debug, Clone)]
struct TaskDisplay {
    task_id: u32,
    pid: u32,
    status: TaskStatus,
    prompt: String,
    elapsed_secs: u64,
}

/// Status screen mode
enum StatusMode {
    List,
    KillConfirm(u32), // PID to kill
    Dialog(DialogWidget),
}

/// Status monitoring screen
pub struct StatusScreen {
    last_update: Instant,
    tasks: Vec<TaskDisplay>,
    selected_idx: usize,
    mode: StatusMode,
}

impl StatusScreen {
    pub fn new() -> Result<Self> {
        let mut screen = Self {
            last_update: Instant::now(),
            tasks: Vec::new(),
            selected_idx: 0,
            mode: StatusMode::List,
        };

        screen.load_tasks()?;

        Ok(screen)
    }

    fn load_tasks(&mut self) -> Result<()> {
        let registry = TaskRegistry::connect()?;
        let entries = registry.entries()?;

        self.tasks = entries
            .into_iter()
            .map(|entry| {
                let elapsed_secs =
                    (chrono::Utc::now().timestamp() - entry.record.started_at.timestamp()) as u64;

                TaskDisplay {
                    task_id: entry.pid,
                    pid: entry.pid,
                    status: entry.record.status,
                    prompt: entry.record.log_id.clone(),
                    elapsed_secs,
                }
            })
            .collect();

        // Reset selection if out of bounds
        if self.selected_idx >= self.tasks.len() && !self.tasks.is_empty() {
            self.selected_idx = self.tasks.len() - 1;
        }

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

    fn truncate_prompt(prompt: &str, max_len: usize) -> String {
        if prompt.len() <= max_len {
            prompt.to_string()
        } else {
            format!("{}...", &prompt[..max_len.saturating_sub(3)])
        }
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
            // Windows process termination
            use std::process::Command;
            Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output()?;
        }

        Ok(())
    }
}

impl Screen for StatusScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            StatusMode::List => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                // Title
                let title = Paragraph::new(format!(
                    "Running Tasks [Auto-refresh: 2s] - {} task(s)",
                    self.tasks.len()
                ))
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                // Task list
                if self.tasks.is_empty() {
                    let content = vec![Line::from(""), Line::from("No tasks running")];
                    let content_widget = Paragraph::new(content)
                        .block(Block::default().borders(Borders::ALL).title("Tasks"));
                    frame.render_widget(content_widget, chunks[1]);
                } else {
                    let items: Vec<ListItem> = self
                        .tasks
                        .iter()
                        .enumerate()
                        .map(|(idx, task)| {
                            let status_icon = match task.status {
                                TaskStatus::Running => "▶",
                                TaskStatus::CompletedButUnread => "✓",
                            };

                            let status_color = match task.status {
                                TaskStatus::Running => Color::Green,
                                TaskStatus::CompletedButUnread => Color::Blue,
                            };

                            let elapsed = Self::format_elapsed_time(task.elapsed_secs);
                            let prompt = Self::truncate_prompt(&task.prompt, 50);
                            let prefix = if idx == self.selected_idx { "> " } else { "  " };

                            let content = vec![
                                Span::raw(prefix),
                                Span::styled(status_icon, Style::default().fg(status_color)),
                                Span::raw(format!(
                                    " #{} [PID:{}] [{}] {}",
                                    task.task_id, task.pid, elapsed, prompt
                                )),
                            ];

                            let style = if idx == self.selected_idx {
                                Style::default().add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            };

                            ListItem::new(Line::from(content)).style(style)
                        })
                        .collect();

                    let list = List::new(items)
                        .block(Block::default().borders(Borders::ALL).title("Tasks"));
                    frame.render_widget(list, chunks[1]);
                }

                // Help
                let help = Paragraph::new("[↑/↓] Navigate  [R] Refresh  [K] Kill  [ESC] Back")
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
            }
            StatusMode::KillConfirm(pid) => {
                let dialog = DialogWidget::confirm(
                    "Confirm Kill".to_string(),
                    format!("Are you sure you want to kill task (PID: {})?", pid),
                );
                dialog.render(frame, area);
            }
            StatusMode::Dialog(dialog) => {
                dialog.render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match &mut self.mode {
            StatusMode::List => match key.code {
                KeyCode::Up => {
                    if self.selected_idx > 0 {
                        self.selected_idx -= 1;
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Down => {
                    if !self.tasks.is_empty() && self.selected_idx < self.tasks.len() - 1 {
                        self.selected_idx += 1;
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.load_tasks()?;
                    self.last_update = Instant::now();
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('k') | KeyCode::Char('K') => {
                    if !self.tasks.is_empty() && self.selected_idx < self.tasks.len() {
                        let pid = self.tasks[self.selected_idx].pid;
                        self.mode = StatusMode::KillConfirm(pid);
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            StatusMode::KillConfirm(pid) => {
                let mut dialog = DialogWidget::confirm("".to_string(), "".to_string());
                let result = dialog.handle_key(key);

                match result {
                    DialogResult::Confirmed => {
                        let pid_to_kill = *pid;
                        if let Err(e) = Self::kill_process(pid_to_kill) {
                            let error_dialog = DialogWidget::error(
                                "Error".to_string(),
                                format!("Failed to kill process: {}", e),
                            );
                            self.mode = StatusMode::Dialog(error_dialog);
                        } else {
                            self.load_tasks()?;
                            let success_dialog = DialogWidget::info(
                                "Success".to_string(),
                                format!("Process {} terminated", pid_to_kill),
                            );
                            self.mode = StatusMode::Dialog(success_dialog);
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
            StatusMode::Dialog(dialog) => {
                let result = dialog.handle_key(key);

                match result {
                    DialogResult::Closed | DialogResult::Confirmed | DialogResult::Cancelled => {
                        self.mode = StatusMode::List;
                        Ok(ScreenAction::None)
                    }
                    DialogResult::None => Ok(ScreenAction::None),
                }
            }
        }
    }

    fn update(&mut self) -> Result<()> {
        // Auto-refresh every 2 seconds
        if self.last_update.elapsed() >= Duration::from_secs(2) {
            self.load_tasks()?;
            self.last_update = Instant::now();
        }
        Ok(())
    }
}
