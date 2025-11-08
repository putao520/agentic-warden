//! Pull progress screen

use super::{Screen, ScreenAction, ScreenType};
use crate::error::AgenticWardenError;
use crate::sync::config_sync_manager::{ConfigSyncManager, PullProgressEvent, SyncOperationResult};
use crate::sync::smart_oauth::AuthState;
use crate::tui::app_state::{AppState, SyncPhase, TransferKind, TransferProgress};
use super::render_helpers::{DialogResult, DialogState, ProgressState};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{error::TryRecvError, unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

const PROVIDER_GOOGLE_DRIVE: &str = "google-drive";

/// pull screen mode
enum PullMode {
    CheckingAuth,
    NeedAuth(DialogState),
    Ready,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug)]
enum PullWorkerEvent {
    Status {
        phase: SyncPhase,
        percent: u8,
        message: Option<String>,
    },
    DirectoryStarted {
        label: String,
        index: usize,
        total: usize,
    },
    DownloadingFile {
        name: Option<String>,
        size: Option<u64>,
    },
    RestoringFiles {
        restored: Option<usize>,
        total: Option<usize>,
    },
    DirectoryCompleted {
        result: SyncOperationResult,
        index: usize,
        total: usize,
    },
    Error {
        message: String,
        needs_auth: bool,
    },
    Cancelled,
}

enum PullWorkerResult {
    Completed(Vec<SyncOperationResult>),
    Cancelled,
    Failed(String),
}

/// pull progress screen
pub struct PullScreen {
    app_state: &'static AppState,
    directories: Vec<String>,
    progress_widget: ProgressState,
    mode: PullMode,
    progress: TransferProgress,
    runtime: Runtime,
    worker_handle: Option<JoinHandle<PullWorkerResult>>,
    progress_rx: Option<UnboundedReceiver<PullWorkerEvent>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    cancel_requested: bool,
    current_directory: Option<String>,
    current_file: Option<String>,
    current_file_size: Option<u64>,
    restored_files: Option<(usize, Option<usize>)>,
    total_downloaded_bytes: u64,
    completed_dirs: usize,
    total_dirs: usize,
    summary: Vec<SyncOperationResult>,
    error_message: Option<String>,
    auth_checked: bool,
    started: bool,
    auto_start_pending: bool,
}

impl PullScreen {
    pub fn new() -> Result<Self> {
        let manager = ConfigSyncManager::new().context("failed to initialise sync manager")?;
        let resolved_directories = manager
            .config_manager
            .get_sync_directories()
            .context("failed to load sync directories")?;

        let runtime = Runtime::new().context("failed to create async runtime")?;
        let progress_widget = ProgressState::new("Pulling from Google Drive".to_string());
        let app_state = AppState::global();
        app_state.clear_sync_progress(TransferKind::Pull);

        let total_dirs = resolved_directories.len();

        let mut screen = Self {
            app_state,
            directories: resolved_directories,
            progress_widget,
            mode: PullMode::CheckingAuth,
            progress: TransferProgress::for_kind(TransferKind::Pull),
            runtime,
            worker_handle: None,
            progress_rx: None,
            cancel_flag: None,
            cancel_requested: false,
            current_directory: None,
            current_file: None,
            current_file_size: None,
            restored_files: None,
            total_downloaded_bytes: 0,
            completed_dirs: 0,
            total_dirs,
            summary: Vec::new(),
            error_message: None,
            auth_checked: false,
            started: false,
            auto_start_pending: true,
        };

        screen.update_progress(
            SyncPhase::Idle,
            0,
            Some("Checking authentication state".to_string()),
        );

        Ok(screen)
    }

    /// Perform authentication check once.
    fn check_authentication(&mut self) {
        if self.auth_checked {
            return;
        }

        self.auth_checked = true;

        match self.app_state.ensure_authenticator(PROVIDER_GOOGLE_DRIVE) {
            Ok(authenticator) => {
                let state = self.runtime.block_on(authenticator.get_state());
                match state {
                    AuthState::Authenticated { .. } => {
                        self.mode = PullMode::Ready;
                        self.update_progress(
                            SyncPhase::Idle,
                            self.progress.percent,
                            Some("Ready to pull configurations".to_string()),
                        );
                    }
                    _ => {
                        let dialog = DialogState::confirm(
                            "Authentication Required".to_string(),
                            "Google Drive authentication is required to pull configurations.\n\nOpen OAuth screen now?"
                                .to_string(),
                        );
                        self.mode = PullMode::NeedAuth(dialog);
                    }
                }
            }
            Err(err) => {
                let dialog = DialogState::confirm(
                    "Authentication Required".to_string(),
                    format!(
                        "Unable to load Google Drive credentials:\n{}\n\nOpen OAuth screen now?",
                        err
                    ),
                );
                self.mode = PullMode::NeedAuth(dialog);
            }
        }
    }

    /// Start the pull operation.
    fn start_pull(&mut self) -> Result<()> {
        if matches!(self.mode, PullMode::Running) {
            return Ok(());
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (tx, rx) = unbounded_channel();

        let directories = self.directories.clone();
        let flag_clone = cancel_flag.clone();
        let handle = self
            .runtime
            .spawn(async move { run_pull_worker(directories, tx, flag_clone).await });

        self.worker_handle = Some(handle);
        self.progress_rx = Some(rx);
        self.cancel_flag = Some(cancel_flag);
        self.cancel_requested = false;
        self.started = true;
        self.auto_start_pending = false;
        self.summary.clear();
        self.error_message = None;
        self.total_downloaded_bytes = 0;
        self.restored_files = None;
        self.completed_dirs = 0;
        self.total_dirs = self.directories.len();
        self.current_directory = None;
        self.current_file = None;
        self.current_file_size = None;
        self.mode = PullMode::Running;

        self.update_progress(
            SyncPhase::Preparing,
            1,
            Some("Initialising pull operation".to_string()),
        );

        Ok(())
    }

    /// Request cancellation of the running pull.
    fn request_cancel(&mut self) {
        if self.cancel_requested {
            return;
        }
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::SeqCst);
            self.cancel_requested = true;
            self.update_progress(
                SyncPhase::Failed,
                self.progress.percent,
                Some("Cancelling Pull...".to_string()),
            );
        }
    }

    /// Reset the state after encountering a failure so the user can retry.
    fn reset_after_failure(&mut self) {
        self.mode = PullMode::Ready;
        self.error_message = None;
        self.cancel_requested = false;
        self.worker_handle = None;
        self.progress_rx = None;
        self.cancel_flag = None;
        self.started = false;
        self.auto_start_pending = false;
        self.auth_checked = false;
        self.update_progress(SyncPhase::Idle, 0, Some("Ready to retry pull".to_string()));
    }

    /// Drain worker events.
    fn poll_worker_events(&mut self) {
        let mut disconnected = false;
        if let Some(mut rx) = self.progress_rx.take() {
            loop {
                match rx.try_recv() {
                    Ok(event) => self.handle_worker_event(event),
                    Err(TryRecvError::Empty) => {
                        self.progress_rx = Some(rx);
                        return;
                    }
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        if !disconnected {
            // Either there was no receiver or it has been drained.
            self.progress_rx = None;
        }
    }

    /// Handle a worker event.
    fn handle_worker_event(&mut self, event: PullWorkerEvent) {
        match event {
            PullWorkerEvent::Status {
                phase,
                percent,
                message,
            } => {
                self.update_progress(phase, percent, message);
            }
            PullWorkerEvent::DirectoryStarted {
                label,
                index,
                total,
            } => {
                self.total_dirs = total;
                self.current_directory = Some(label);
                if self.completed_dirs < index {
                    self.completed_dirs = index;
                }
                self.restored_files = None;
            }
            PullWorkerEvent::DownloadingFile { name, size } => {
                self.current_file = name;
                self.current_file_size = size;
            }
            PullWorkerEvent::RestoringFiles { restored, total } => {
                if let Some(restored) = restored {
                    self.restored_files = Some((restored, total));
                }
            }
            PullWorkerEvent::DirectoryCompleted {
                result,
                index,
                total,
            } => {
                self.total_dirs = total;
                self.completed_dirs = index + 1;
                self.current_directory = Some(result.directory_name.clone());
                self.current_file = None;
                if let Some(size) = result.file_size {
                    self.total_downloaded_bytes += size;
                    self.current_file_size = Some(size);
                }
                self.summary.push(result);
            }
            PullWorkerEvent::Error {
                message,
                needs_auth,
            } => {
                self.error_message = Some(message.clone());
                if needs_auth {
                    self.auth_checked = false;
                    self.mode = PullMode::CheckingAuth;
                } else {
                    self.mode = PullMode::Failed;
                }
                self.update_progress(
                    SyncPhase::Failed,
                    self.progress.percent.max(1),
                    Some(message),
                );
            }
            PullWorkerEvent::Cancelled => {
                self.mode = PullMode::Cancelled;
                self.update_progress(
                    SyncPhase::Failed,
                    self.progress.percent,
                    Some("Pull cancelled by user".to_string()),
                );
            }
        }
    }

    /// Update progress state and propagate to global app state.
    fn update_progress(&mut self, phase: SyncPhase, percent: u8, message: Option<String>) {
        let mut progress = self
            .progress
            .clone()
            .with_phase(phase)
            .with_percent(percent);
        progress = progress.with_message(message.clone());
        self.progress = progress;
        self.progress_widget.set_progress(percent as u16);
        if let Some(msg) = message {
            if msg.is_empty() {
                self.progress_widget.clear_message();
            } else {
                self.progress_widget.set_message(msg);
            }
        } else {
            self.progress_widget.clear_message();
        }
        self.app_state
            .set_sync_progress(TransferKind::Pull, self.progress.clone());
    }

    fn title_text(&self) -> &'static str {
        match self.mode {
            PullMode::CheckingAuth => "Checking Authentication",
            PullMode::NeedAuth(_) => "Authentication Required",
            PullMode::Ready => "Ready to Pull from Google Drive",
            PullMode::Running => {
                if self.cancel_requested {
                    "Cancelling Pull"
                } else {
                    "Pulling from Google Drive"
                }
            }
            PullMode::Completed => "Pull Completed",
            PullMode::Cancelled => "Pull Cancelled",
            PullMode::Failed => "Pull Failed",
        }
    }

    fn help_text(&self) -> String {
        match self.mode {
            PullMode::Ready => "[Enter] Start Pull  [ESC] Back".to_string(),
            PullMode::Running => {
                if self.cancel_requested {
                    "Cancelling Pull...".to_string()
                } else {
                    "[ESC] Cancel".to_string()
                }
            }
            PullMode::Completed | PullMode::Cancelled => "[Enter] Continue  [ESC] Back".to_string(),
            PullMode::Failed => "[Enter] Retry  [ESC] Back".to_string(),
            PullMode::NeedAuth(_) => "Use dialog to continue".to_string(),
            PullMode::CheckingAuth => "[ESC] Back".to_string(),
        }
    }

    fn build_details(&self) -> Vec<Line<'_>> {
        let mut lines = Vec::new();

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Status:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        let status = self
            .progress
            .message
            .clone()
            .unwrap_or_else(|| "Awaiting input".to_string());
        lines.push(Line::from(format!("  {}", status)));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Progress:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!(
            "  Directories {}/{}",
            self.completed_dirs, self.total_dirs
        )));

        if let Some(dir) = &self.current_directory {
            lines.push(Line::from(format!("  Current: {}", dir)));
        }

        if let Some(file) = &self.current_file {
            if let Some(size) = self.current_file_size {
                lines.push(Line::from(format!(
                    "  Downloading: {} ({})",
                    file,
                    format_bytes(size)
                )));
            } else {
                lines.push(Line::from(format!("  Downloading: {}", file)));
            }
        }

        if self.total_downloaded_bytes > 0 {
            lines.push(Line::from(format!(
                "  Downloaded total: {}",
                format_bytes(self.total_downloaded_bytes)
            )));
        }

        if let Some((restored, total)) = &self.restored_files {
            let total_text = total
                .map(|t| format!("/{}", t))
                .unwrap_or_else(|| String::new());
            lines.push(Line::from(format!(
                "  Restoring files: {}{}",
                restored, total_text
            )));
        }

        if !self.directories.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Target Directories:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for dir in &self.directories {
                lines.push(Line::from(format!("  - {}", dir)));
            }
        }

        if matches!(
            self.mode,
            PullMode::Completed | PullMode::Failed | PullMode::Cancelled
        ) && !self.summary.is_empty()
        {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Summary:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for result in &self.summary {
                let disposition = if result.changed {
                    "restored"
                } else {
                    "skipped"
                };
                lines.push(Line::from(format!(
                    "  {} - {} ({})",
                    result.directory_name, result.message, disposition
                )));
            }
        }

        if let Some(error) = &self.error_message {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Error:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(format!("  {}", error)));
        }

        lines
    }
}

impl Screen for PullScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if let PullMode::NeedAuth(dialog) = &self.mode {
            dialog.render(frame, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(area);

        let title = Paragraph::new(self.title_text())
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        self.progress_widget.render(frame, chunks[1]);

        let details = Paragraph::new(self.build_details())
            .block(Block::default().borders(Borders::ALL).title("Details"));
        frame.render_widget(details, chunks[2]);

        let help = Paragraph::new(self.help_text())
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        if matches!(self.mode, PullMode::NeedAuth(_)) {
            let result = match &mut self.mode {
                PullMode::NeedAuth(dialog) => dialog.handle_key(key),
                _ => unreachable!(),
            };
            return match result {
                DialogResult::Confirmed => {
                    self.auth_checked = false;
                    self.auto_start_pending = true;
                    self.mode = PullMode::CheckingAuth;
                    Ok(ScreenAction::SwitchTo(ScreenType::OAuth))
                }
                DialogResult::Cancelled | DialogResult::Closed => Ok(ScreenAction::Back),
                DialogResult::None => Ok(ScreenAction::None),
            };
        }

        match self.mode {
            PullMode::CheckingAuth => match key.code {
                KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            PullMode::Ready => match key.code {
                KeyCode::Enter => {
                    self.start_pull()?;
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            PullMode::Running => match key.code {
                KeyCode::Esc => {
                    self.request_cancel();
                    Ok(ScreenAction::None)
                }
                _ => Ok(ScreenAction::None),
            },
            PullMode::Completed | PullMode::Cancelled => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.app_state.clear_sync_progress(TransferKind::Pull);
                    Ok(ScreenAction::Back)
                }
                _ => Ok(ScreenAction::None),
            },
            PullMode::Failed => match key.code {
                KeyCode::Enter => {
                    self.reset_after_failure();
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            PullMode::NeedAuth(_) => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        if matches!(self.mode, PullMode::CheckingAuth) {
            self.check_authentication();
        }

        if matches!(self.mode, PullMode::Ready) && self.auto_start_pending && !self.started {
            self.start_pull()?;
        }

        self.poll_worker_events();

        if let Some(handle) = self.worker_handle.as_ref() {
            if handle.is_finished() {
                let handle = self.worker_handle.take().unwrap();
                match self.runtime.block_on(handle) {
                    Ok(PullWorkerResult::Completed(results)) => {
                        self.summary = results;
                        self.mode = PullMode::Completed;
                        self.update_progress(
                            SyncPhase::Completed,
                            100,
                            Some("pull completed successfully".to_string()),
                        );
                    }
                    Ok(PullWorkerResult::Cancelled) => {
                        self.mode = PullMode::Cancelled;
                        self.update_progress(
                            SyncPhase::Failed,
                            self.progress.percent,
                            Some("Pull cancelled by user".to_string()),
                        );
                    }
                    Ok(PullWorkerResult::Failed(msg)) => {
                        self.error_message = Some(msg.clone());
                        self.mode = PullMode::Failed;
                        self.update_progress(
                            SyncPhase::Failed,
                            self.progress.percent.max(1),
                            Some(msg),
                        );
                    }
                    Err(err) => {
                        let msg = format!("pull worker error: {}", err);
                        self.error_message = Some(msg.clone());
                        self.mode = PullMode::Failed;
                        self.update_progress(
                            SyncPhase::Failed,
                            self.progress.percent.max(1),
                            Some(msg),
                        );
                    }
                }
                self.progress_rx = None;
                self.cancel_flag = None;
                self.cancel_requested = false;
            }
        }

        Ok(())
    }
}

impl Drop for PullScreen {
    fn drop(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::SeqCst);
        }
        self.app_state.clear_sync_progress(TransferKind::Pull);
    }
}

fn display_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|p| p.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string())
}

fn compute_percent(index: usize, total: usize, stage: f32) -> u8 {
    if total == 0 {
        return 100;
    }
    let per_dir = 100.0 / total as f32;
    let clamped_stage = stage.clamp(0.0, 1.0);
    let value = index as f32 * per_dir + clamped_stage * per_dir;
    value.clamp(0.0, 100.0).round() as u8
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

async fn run_pull_worker(
    directories: Vec<String>,
    tx: UnboundedSender<PullWorkerEvent>,
    cancel_flag: Arc<AtomicBool>,
) -> PullWorkerResult {
    let mut manager = match ConfigSyncManager::new() {
        Ok(mgr) => mgr,
        Err(err) => {
            let msg = format!("Failed to initialise sync manager: {}", err);
            let _ = tx.send(PullWorkerEvent::Error {
                message: msg.clone(),
                needs_auth: false,
            });
            return PullWorkerResult::Failed(msg);
        }
    };

    let mut dirs = if directories.is_empty() {
        match manager.config_manager.get_sync_directories() {
            Ok(list) => list,
            Err(err) => {
                let msg = format!("Failed to load sync directories: {}", err);
                let _ = tx.send(PullWorkerEvent::Error {
                    message: msg.clone(),
                    needs_auth: false,
                });
                return PullWorkerResult::Failed(msg);
            }
        }
    } else {
        directories
    };

    if dirs.is_empty() {
        let _ = tx.send(PullWorkerEvent::Status {
            phase: SyncPhase::Completed,
            percent: 100,
            message: Some("No directories configured for sync".to_string()),
        });
        return PullWorkerResult::Completed(Vec::new());
    }

    if let Err(err) = manager.authenticate_google_drive().await {
        let needs_auth = matches!(
            &err,
            AgenticWardenError::Auth { provider, .. } if provider == "google_drive"
        );
        let message = err.user_message();
        let _ = tx.send(PullWorkerEvent::Error {
            message: message.clone(),
            needs_auth,
        });
        return PullWorkerResult::Failed(message);
    }

    let total = dirs.len();
    let mut results = Vec::with_capacity(total);

    for (index, directory) in dirs.drain(..).enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = tx.send(PullWorkerEvent::Cancelled);
            return PullWorkerResult::Cancelled;
        }

        let label = display_name(&directory);
        let _ = tx.send(PullWorkerEvent::DirectoryStarted {
            label: label.clone(),
            index,
            total,
        });
        let _ = tx.send(PullWorkerEvent::Status {
            phase: SyncPhase::Preparing,
            percent: compute_percent(index, total, 0.05),
            message: Some(format!("Preparing {}", label)),
        });

        let tx_inner = tx.clone();
        let cancel_inner = cancel_flag.clone();
        let label_clone = label.clone();

        let result = manager
            .pull_directory_with_observer(&directory, |event| {
                if cancel_inner.load(Ordering::SeqCst) {
                    return;
                }
                match event {
                    PullProgressEvent::Downloading {
                        file_name, size, ..
                    } => {
                        let name = file_name.clone().unwrap_or_else(|| label_clone.clone());
                        let _ = tx_inner.send(PullWorkerEvent::Status {
                            phase: SyncPhase::Downloading,
                            percent: compute_percent(index, total, 0.45),
                            message: Some(format!("Downloading {}", name)),
                        });
                        let _ = tx_inner.send(PullWorkerEvent::DownloadingFile {
                            name: file_name,
                            size,
                        });
                    }
                    PullProgressEvent::Decompressing { .. } => {
                        let _ = tx_inner.send(PullWorkerEvent::Status {
                            phase: SyncPhase::Verifying,
                            percent: compute_percent(index, total, 0.7),
                            message: Some(format!("Decompressing {}", label_clone)),
                        });
                    }
                    PullProgressEvent::Restoring {
                        files_restored,
                        total_files,
                        ..
                    } => {
                        let _ = tx_inner.send(PullWorkerEvent::Status {
                            phase: SyncPhase::Applying,
                            percent: compute_percent(index, total, 0.9),
                            message: Some(format!("Restoring {}", label_clone)),
                        });
                        let _ = tx_inner.send(PullWorkerEvent::RestoringFiles {
                            restored: files_restored,
                            total: total_files,
                        });
                    }
                    PullProgressEvent::Skipped { reason, .. } => {
                        let _ = tx_inner.send(PullWorkerEvent::Status {
                            phase: SyncPhase::Completed,
                            percent: compute_percent(index, total, 1.0),
                            message: Some(reason),
                        });
                    }
                    PullProgressEvent::Completed { .. } => {
                        let _ = tx_inner.send(PullWorkerEvent::Status {
                            phase: SyncPhase::Completed,
                            percent: compute_percent(index, total, 1.0),
                            message: Some(format!("Completed {}", label_clone)),
                        });
                    }
                    PullProgressEvent::StartingDirectory { .. } => {}
                }
            })
            .await;

        match result {
            Ok(sync_result) => {
                let _ = tx.send(PullWorkerEvent::DirectoryCompleted {
                    result: sync_result.clone(),
                    index,
                    total,
                });
                results.push(sync_result);
            }
            Err(err) => {
                let needs_auth = matches!(
                    &err,
                    AgenticWardenError::Auth { provider, .. } if provider == "google_drive"
                );
                let mut msg = err.user_message();
                if !msg.contains(&label) {
                    msg = format!("{label}: {msg}");
                }
                let _ = tx.send(PullWorkerEvent::Error {
                    message: msg.clone(),
                    needs_auth,
                });
                return PullWorkerResult::Failed(msg);
            }
        }
    }

    let _ = tx.send(PullWorkerEvent::Status {
        phase: SyncPhase::Completed,
        percent: 100,
        message: Some("pull completed successfully".to_string()),
    });
    PullWorkerResult::Completed(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};

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

    fn test_screen_with_mode(mode: PullMode) -> PullScreen {
        PullScreen {
            app_state: AppState::global(),
            directories: vec!["/tmp/project".into()],
            progress_widget: ProgressState::new("Pull Test".to_string()),
            mode,
            progress: TransferProgress::for_kind(TransferKind::Pull),
            runtime: tokio::runtime::Runtime::new().expect("runtime"),
            worker_handle: None,
            progress_rx: None,
            cancel_flag: None,
            cancel_requested: false,
            current_directory: Some("/tmp/project".into()),
            current_file: None,
            current_file_size: None,
            restored_files: None,
            total_downloaded_bytes: 0,
            completed_dirs: 0,
            total_dirs: 1,
            summary: Vec::new(),
            error_message: None,
            auth_checked: true,
            started: false,
            auto_start_pending: false,
        }
    }

    #[test]
    fn pull_screen_renders_progress_information() {
        let mut screen = test_screen_with_mode(PullMode::Running);
        screen.progress = TransferProgress::for_kind(TransferKind::Pull)
            .with_phase(SyncPhase::Downloading)
            .with_percent(40)
            .with_message(Some("Downloading archive".into()));
        screen.current_file = Some("archive.tar.gz".into());
        screen.total_downloaded_bytes = 512 * 1024;

        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| screen.render(frame, frame.size()))
            .unwrap();

        let rendered = buffer_to_string(terminal.backend().buffer());
        assert!(
            rendered.contains("Pull Test") || rendered.contains("Pulling from Google Drive"),
            "rendered output missing pull context:\n{rendered}"
        );
    }

    #[test]
    fn pull_screen_handle_key_respects_mode() {
        let mut ready = test_screen_with_mode(PullMode::Ready);
        let back = ready
            .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(back, ScreenAction::Back));

        let mut completed = test_screen_with_mode(PullMode::Completed);
        let back = completed
            .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(back, ScreenAction::Back));
    }
}
