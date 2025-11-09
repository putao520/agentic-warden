//! Push progress screen

use super::{Screen, ScreenAction, ScreenType};
use crate::error::{errors, AgenticWardenError, ErrorCategory, SyncOperation, UserFacingError};
use crate::sync::config_sync_manager::{ConfigSyncManager, PushProgressEvent, SyncOperationResult};
use crate::sync::smart_device_flow::AuthState;
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

/// Push screen mode
enum PushMode {
    CheckingAuth,
    NeedAuth(DialogState),
    Ready,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug)]
enum PushWorkerEvent {
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
    UploadingFile {
        name: Option<String>,
        size: Option<u64>,
    },
    DirectoryCompleted {
        result: SyncOperationResult,
        index: usize,
        total: usize,
    },
    Error {
        error: Arc<AgenticWardenError>,
    },
    Cancelled,
}

enum PushWorkerResult {
    Completed(Vec<SyncOperationResult>),
    Cancelled,
    Failed(Arc<AgenticWardenError>),
}

/// Push progress screen
pub struct PushScreen {
    app_state: &'static AppState,
    directories: Vec<String>,
    progress_widget: ProgressState,
    mode: PushMode,
    progress: TransferProgress,
    runtime: Runtime,
    worker_handle: Option<JoinHandle<PushWorkerResult>>,
    progress_rx: Option<UnboundedReceiver<PushWorkerEvent>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    cancel_requested: bool,
    current_directory: Option<String>,
    current_file: Option<String>,
    current_file_size: Option<u64>,
    total_uploaded_bytes: u64,
    completed_dirs: usize,
    total_dirs: usize,
    summary: Vec<SyncOperationResult>,
    error_detail: Option<UserFacingError>,
    auth_checked: bool,
    started: bool,
    auto_start_pending: bool,
}

impl PushScreen {
    pub fn new(directories: Vec<String>) -> Result<Self> {
        let resolved_directories = if directories.is_empty() {
            let manager = ConfigSyncManager::new().context("failed to initialise sync manager")?;
            manager
                .config_manager
                .get_sync_directories()
                .context("failed to load sync directories")?
        } else {
            directories
        };

        let runtime = Runtime::new().context("failed to create async runtime")?;
        let progress_widget = ProgressState::new("Pushing to Google Drive".to_string());
        let app_state = AppState::global();
        app_state.clear_sync_progress(TransferKind::Push);

        let total_dirs = resolved_directories.len();

        let mut screen = Self {
            app_state,
            directories: resolved_directories,
            progress_widget,
            mode: PushMode::CheckingAuth,
            progress: TransferProgress::for_kind(TransferKind::Push),
            runtime,
            worker_handle: None,
            progress_rx: None,
            cancel_flag: None,
            cancel_requested: false,
            current_directory: None,
            current_file: None,
            current_file_size: None,
            total_uploaded_bytes: 0,
            completed_dirs: 0,
            total_dirs,
            summary: Vec::new(),
            error_detail: None,
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
                        self.mode = PushMode::Ready;
                        self.update_progress(
                            SyncPhase::Idle,
                            self.progress.percent,
                            Some("Ready to push configurations".to_string()),
                        );
                    }
                    _ => {
                        let dialog = DialogState::confirm(
                            "Authentication Required".to_string(),
                            "Google Drive authentication is required to push configurations.\n\nOpen OAuth screen now?"
                                .to_string(),
                        );
                        self.mode = PushMode::NeedAuth(dialog);
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
                self.mode = PushMode::NeedAuth(dialog);
            }
        }
    }

    /// Start the push operation.
    fn start_push(&mut self) -> Result<()> {
        if matches!(self.mode, PushMode::Running) {
            return Ok(());
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (tx, rx) = unbounded_channel();

        let directories = self.directories.clone();
        let flag_clone = cancel_flag.clone();
        let handle = self
            .runtime
            .spawn(async move { run_push_worker(directories, tx, flag_clone).await });

        self.worker_handle = Some(handle);
        self.progress_rx = Some(rx);
        self.cancel_flag = Some(cancel_flag);
        self.cancel_requested = false;
        self.started = true;
        self.auto_start_pending = false;
        self.summary.clear();
        self.error_detail = None;
        self.total_uploaded_bytes = 0;
        self.completed_dirs = 0;
        self.total_dirs = self.directories.len();
        self.current_directory = None;
        self.current_file = None;
        self.current_file_size = None;
        self.mode = PushMode::Running;

        self.update_progress(
            SyncPhase::Preparing,
            1,
            Some("Initialising push operation".to_string()),
        );

        Ok(())
    }

    /// Request cancellation of the running push.
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
                Some("Cancelling push...".to_string()),
            );
        }
    }

    /// Reset the state after encountering a failure so the user can retry.
    fn reset_after_failure(&mut self) {
        self.mode = PushMode::Ready;
        self.error_detail = None;
        self.cancel_requested = false;
        self.worker_handle = None;
        self.progress_rx = None;
        self.cancel_flag = None;
        self.started = false;
        self.auto_start_pending = false;
        self.auth_checked = false;
        self.update_progress(SyncPhase::Idle, 0, Some("Ready to retry push".to_string()));
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
    fn handle_worker_event(&mut self, event: PushWorkerEvent) {
        match event {
            PushWorkerEvent::Status {
                phase,
                percent,
                message,
            } => {
                self.update_progress(phase, percent, message);
            }
            PushWorkerEvent::DirectoryStarted {
                label,
                index,
                total,
            } => {
                self.total_dirs = total;
                self.current_directory = Some(label);
                if self.completed_dirs < index {
                    self.completed_dirs = index;
                }
            }
            PushWorkerEvent::UploadingFile { name, size } => {
                self.current_file = name;
                self.current_file_size = size;
            }
            PushWorkerEvent::DirectoryCompleted {
                result,
                index,
                total,
            } => {
                self.total_dirs = total;
                self.completed_dirs = index + 1;
                self.current_directory = Some(result.directory_name.clone());
                self.current_file = None;
                if result.uploaded {
                    if let Some(size) = result.file_size {
                        self.total_uploaded_bytes += size;
                        self.current_file_size = Some(size);
                    }
                }
                self.summary.push(result);
            }
            PushWorkerEvent::Error { error } => {
                let detail = error.as_ref().to_user_facing();
                let needs_auth = error.category() == ErrorCategory::Auth;
                self.error_detail = Some(detail.clone());
                if needs_auth {
                    self.auth_checked = false;
                    self.mode = PushMode::CheckingAuth;
                } else {
                    self.mode = PushMode::Failed;
                }
                self.update_progress(
                    SyncPhase::Failed,
                    self.progress.percent.max(1),
                    Some(detail.message.clone()),
                );
            }
            PushWorkerEvent::Cancelled => {
                self.mode = PushMode::Cancelled;
                self.update_progress(
                    SyncPhase::Failed,
                    self.progress.percent,
                    Some("Push cancelled by user".to_string()),
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
            .set_sync_progress(TransferKind::Push, self.progress.clone());
    }

    fn title_text(&self) -> &'static str {
        match self.mode {
            PushMode::CheckingAuth => "Checking Authentication",
            PushMode::NeedAuth(_) => "Authentication Required",
            PushMode::Ready => "Ready to Push to Google Drive",
            PushMode::Running => {
                if self.cancel_requested {
                    "Cancelling Push"
                } else {
                    "Pushing to Google Drive"
                }
            }
            PushMode::Completed => "Push Completed",
            PushMode::Cancelled => "Push Cancelled",
            PushMode::Failed => "Push Failed",
        }
    }

    fn help_text(&self) -> String {
        match self.mode {
            PushMode::Ready => "[Enter] Start Push  [ESC] Back".to_string(),
            PushMode::Running => {
                if self.cancel_requested {
                    "Cancelling push...".to_string()
                } else {
                    "[ESC] Cancel".to_string()
                }
            }
            PushMode::Completed | PushMode::Cancelled => "[Enter] Continue  [ESC] Back".to_string(),
            PushMode::Failed => "[Enter] Retry  [ESC] Back".to_string(),
            PushMode::NeedAuth(_) => "Use dialog to continue".to_string(),
            PushMode::CheckingAuth => "[ESC] Back".to_string(),
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
                    "  Uploading: {} ({})",
                    file,
                    format_bytes(size)
                )));
            } else {
                lines.push(Line::from(format!("  Uploading: {}", file)));
            }
        }

        if self.total_uploaded_bytes > 0 {
            lines.push(Line::from(format!(
                "  Uploaded total: {}",
                format_bytes(self.total_uploaded_bytes)
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
            PushMode::Completed | PushMode::Failed | PushMode::Cancelled
        ) && !self.summary.is_empty()
        {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Summary:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for result in &self.summary {
                let disposition = if result.changed {
                    if result.uploaded {
                        "uploaded"
                    } else {
                        "changed"
                    }
                } else {
                    "skipped"
                };
                lines.push(Line::from(format!(
                    "  {} - {} ({})",
                    result.directory_name, result.message, disposition
                )));
            }
        }

        if let Some(error) = &self.error_detail {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Error: {}", error.title),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(format!("  {}", error.message)));
            if let Some(hint) = &error.hint {
                lines.push(Line::from(format!("  Hint: {}", hint)));
            }
        }

        lines
    }
}

impl Screen for PushScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if let PushMode::NeedAuth(dialog) = &self.mode {
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
        if matches!(self.mode, PushMode::NeedAuth(_)) {
            let result = match &mut self.mode {
                PushMode::NeedAuth(dialog) => dialog.handle_key(key),
                _ => unreachable!(),
            };
            return match result {
                DialogResult::Confirmed => {
                    self.auth_checked = false;
                    self.auto_start_pending = true;
                    self.mode = PushMode::CheckingAuth;
                    Ok(ScreenAction::SwitchTo(ScreenType::OAuth))
                }
                DialogResult::Cancelled | DialogResult::Closed => Ok(ScreenAction::Back),
                DialogResult::None => Ok(ScreenAction::None),
            };
        }

        match self.mode {
            PushMode::CheckingAuth => match key.code {
                KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            PushMode::Ready => match key.code {
                KeyCode::Enter => {
                    self.start_push()?;
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            PushMode::Running => match key.code {
                KeyCode::Esc => {
                    self.request_cancel();
                    Ok(ScreenAction::None)
                }
                _ => Ok(ScreenAction::None),
            },
            PushMode::Completed | PushMode::Cancelled => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.app_state.clear_sync_progress(TransferKind::Push);
                    Ok(ScreenAction::Back)
                }
                _ => Ok(ScreenAction::None),
            },
            PushMode::Failed => match key.code {
                KeyCode::Enter => {
                    self.reset_after_failure();
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => Ok(ScreenAction::Back),
                _ => Ok(ScreenAction::None),
            },
            PushMode::NeedAuth(_) => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        if matches!(self.mode, PushMode::CheckingAuth) {
            self.check_authentication();
        }

        if matches!(self.mode, PushMode::Ready) && self.auto_start_pending && !self.started {
            self.start_push()?;
        }

        self.poll_worker_events();

        if let Some(handle) = self.worker_handle.as_ref() {
            if handle.is_finished() {
                let handle = self.worker_handle.take().unwrap();
                match self.runtime.block_on(handle) {
                    Ok(PushWorkerResult::Completed(results)) => {
                        self.summary = results;
                        self.mode = PushMode::Completed;
                        self.update_progress(
                            SyncPhase::Completed,
                            100,
                            Some("Push completed successfully".to_string()),
                        );
                    }
                    Ok(PushWorkerResult::Cancelled) => {
                        self.mode = PushMode::Cancelled;
                        self.update_progress(
                            SyncPhase::Failed,
                            self.progress.percent,
                            Some("Push cancelled by user".to_string()),
                        );
                    }
                    Ok(PushWorkerResult::Failed(error)) => {
                        let detail = error.as_ref().to_user_facing();
                        self.error_detail = Some(detail.clone());
                        self.mode = PushMode::Failed;
                        self.update_progress(
                            SyncPhase::Failed,
                            self.progress.percent.max(1),
                            Some(detail.message),
                        );
                    }
                    Err(err) => {
                        let raw_error = errors::sync_error(
                            SyncOperation::Unknown,
                            format!("Push worker error: {}", err),
                        );
                        let detail = raw_error.to_user_facing();
                        self.error_detail = Some(detail.clone());
                        self.mode = PushMode::Failed;
                        self.update_progress(
                            SyncPhase::Failed,
                            self.progress.percent.max(1),
                            Some(detail.message),
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

impl Drop for PushScreen {
    fn drop(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::SeqCst);
        }
        self.app_state.clear_sync_progress(TransferKind::Push);
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

async fn run_push_worker(
    directories: Vec<String>,
    tx: UnboundedSender<PushWorkerEvent>,
    cancel_flag: Arc<AtomicBool>,
) -> PushWorkerResult {
    let mut manager = match ConfigSyncManager::new() {
        Ok(mgr) => mgr,
        Err(err) => {
            let msg = format!("Failed to initialise sync manager: {}", err);
            let error = emit_worker_error(
                &tx,
                errors::sync_error(SyncOperation::StateVerification, msg),
            );
            return PushWorkerResult::Failed(error);
        }
    };

    let mut dirs = if directories.is_empty() {
        match manager.config_manager.get_sync_directories() {
            Ok(list) => list,
            Err(err) => {
                let msg = format!("Failed to load sync directories: {}", err);
                let error =
                    emit_worker_error(&tx, errors::sync_error(SyncOperation::Discovery, msg));
                return PushWorkerResult::Failed(error);
            }
        }
    } else {
        directories
    };

    if dirs.is_empty() {
        let _ = tx.send(PushWorkerEvent::Status {
            phase: SyncPhase::Completed,
            percent: 100,
            message: Some("No directories configured for sync".to_string()),
        });
        return PushWorkerResult::Completed(Vec::new());
    }

    if let Err(err) = manager.authenticate_google_drive().await {
        let error = emit_worker_error(&tx, err);
        return PushWorkerResult::Failed(error);
    }

    let total = dirs.len();
    let mut results = Vec::with_capacity(total);

    for (index, directory) in dirs.drain(..).enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = tx.send(PushWorkerEvent::Cancelled);
            return PushWorkerResult::Cancelled;
        }

        let label = display_name(&directory);
        let _ = tx.send(PushWorkerEvent::DirectoryStarted {
            label: label.clone(),
            index,
            total,
        });
        let _ = tx.send(PushWorkerEvent::Status {
            phase: SyncPhase::Preparing,
            percent: compute_percent(index, total, 0.05),
            message: Some(format!("Preparing {}", label)),
        });

        let tx_inner = tx.clone();
        let cancel_inner = cancel_flag.clone();
        let label_clone = label.clone();

        let result = manager
            .push_directory_with_observer(&directory, |event| {
                if cancel_inner.load(Ordering::SeqCst) {
                    return;
                }
                match event {
                    PushProgressEvent::Compressing { .. } => {
                        let _ = tx_inner.send(PushWorkerEvent::Status {
                            phase: SyncPhase::Compressing,
                            percent: compute_percent(index, total, 0.35),
                            message: Some(format!("Compressing {}", label_clone)),
                        });
                    }
                    PushProgressEvent::Uploading {
                        file_name, size, ..
                    } => {
                        let _ = tx_inner.send(PushWorkerEvent::Status {
                            phase: SyncPhase::Uploading,
                            percent: compute_percent(index, total, 0.7),
                            message: Some(format!("Uploading {}", file_name)),
                        });
                        let _ = tx_inner.send(PushWorkerEvent::UploadingFile {
                            name: Some(file_name),
                            size,
                        });
                    }
                    PushProgressEvent::Verifying { .. } => {
                        let _ = tx_inner.send(PushWorkerEvent::Status {
                            phase: SyncPhase::Verifying,
                            percent: compute_percent(index, total, 0.9),
                            message: Some(format!("Verifying {}", label_clone)),
                        });
                    }
                    PushProgressEvent::Skipped { reason, .. } => {
                        let _ = tx_inner.send(PushWorkerEvent::Status {
                            phase: SyncPhase::Completed,
                            percent: compute_percent(index, total, 1.0),
                            message: Some(reason),
                        });
                    }
                    PushProgressEvent::Completed { .. } => {
                        let _ = tx_inner.send(PushWorkerEvent::Status {
                            phase: SyncPhase::Completed,
                            percent: compute_percent(index, total, 1.0),
                            message: Some(format!("Completed {}", label_clone)),
                        });
                    }
                    PushProgressEvent::StartingDirectory { .. } => {}
                }
            })
            .await;

        match result {
            Ok(sync_result) => {
                let _ = tx.send(PushWorkerEvent::DirectoryCompleted {
                    result: sync_result.clone(),
                    index,
                    total,
                });
                results.push(sync_result);
            }
            Err(err) => {
                let mut msg = err.user_message();
                if !msg.contains(&label) {
                    msg = format!("{label}: {msg}");
                }
                let operation = err.sync_operation().unwrap_or(SyncOperation::Upload);
                let wrapped = errors::sync_error_with_source(operation, msg, err);
                let error = emit_worker_error(&tx, wrapped);
                return PushWorkerResult::Failed(error);
            }
        }
    }

    let _ = tx.send(PushWorkerEvent::Status {
        phase: SyncPhase::Completed,
        percent: 100,
        message: Some("Push completed successfully".to_string()),
    });
    PushWorkerResult::Completed(results)
}

fn emit_worker_error(
    tx: &UnboundedSender<PushWorkerEvent>,
    error: AgenticWardenError,
) -> Arc<AgenticWardenError> {
    let shared = Arc::new(error);
    let _ = tx.send(PushWorkerEvent::Error {
        error: shared.clone(),
    });
    shared
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

    fn test_screen_with_mode(mode: PushMode) -> PushScreen {
        PushScreen {
            app_state: AppState::global(),
            directories: vec!["/tmp/project".into()],
            progress_widget: ProgressState::new("Push Test".to_string()),
            mode,
            progress: TransferProgress::for_kind(TransferKind::Push),
            runtime: tokio::runtime::Runtime::new().expect("runtime"),
            worker_handle: None,
            progress_rx: None,
            cancel_flag: None,
            cancel_requested: false,
            current_directory: Some("/tmp/project".into()),
            current_file: None,
            current_file_size: None,
            total_uploaded_bytes: 0,
            completed_dirs: 0,
            total_dirs: 1,
            summary: Vec::new(),
            error_detail: None,
            auth_checked: true,
            started: false,
            auto_start_pending: false,
        }
    }

    #[test]
    fn push_screen_renders_progress_information() {
        let mut screen = test_screen_with_mode(PushMode::Running);
        screen.progress = TransferProgress::for_kind(TransferKind::Push)
            .with_phase(SyncPhase::Uploading)
            .with_percent(55)
            .with_message(Some("Uploading archive".into()));
        screen.current_file = Some("archive.tar.gz".into());
        screen.total_uploaded_bytes = 1024 * 1024;

        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| screen.render(frame, frame.size()))
            .unwrap();

        let rendered = buffer_to_string(terminal.backend().buffer());
        assert!(
            rendered.contains("Push Test") || rendered.contains("Pushing to Google Drive"),
            "rendered output missing push context:\n{rendered}"
        );
    }

    #[test]
    fn push_screen_handle_key_respects_mode() {
        let mut ready = test_screen_with_mode(PushMode::Ready);
        let back = ready
            .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(back, ScreenAction::Back));

        let mut completed = test_screen_with_mode(PushMode::Completed);
        let back = completed
            .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(back, ScreenAction::Back));
    }
}
