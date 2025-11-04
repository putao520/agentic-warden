//! TUI Application State - Simplified placeholder implementation

use std::collections::HashMap;

// Main application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub providers: Vec<crate::core::models::Provider>,
    pub tasks: HashMap<String, TaskSnapshot>,
    pub current_sync_phase: SyncPhase,
    pub transfer_progress: TransferProgress,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            tasks: HashMap::new(),
            current_sync_phase: SyncPhase::Idle,
            transfer_progress: TransferProgress::default(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_provider(&mut self, provider: crate::core::models::Provider) {
        self.providers.push(provider);
    }

    pub fn remove_provider(&mut self, name: &str) {
        self.providers.retain(|p| p.name != name);
    }

    pub fn update_task(&mut self, id: String, snapshot: TaskSnapshot) {
        self.tasks.insert(id, snapshot);
    }
}

// Synchronization phases
#[derive(Debug, Clone, PartialEq)]
pub enum SyncPhase {
    Idle,
    Preparing,
    Authentication,
    Listing,
    Compressing,
    Uploading,
    Downloading,
    Verifying,
    Applying,
    Completed,
    Failed(String),
}

// Transfer types
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TransferKind {
    #[default]
    Push,
    Pull,
    Sync,
}

// Transfer progress
#[derive(Debug, Clone, Default)]
pub struct TransferProgress {
    pub kind: TransferKind,
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub speed: f64, // bytes per second
    pub eta: Option<std::time::Duration>,
}

impl TransferProgress {
    pub fn new(kind: TransferKind, total: usize) -> Self {
        Self {
            kind,
            total,
            ..Default::default()
        }
    }

    pub fn update(&mut self, current: usize, current_file: String) {
        self.current = current;
        self.current_file = current_file;
        // Update speed and ETA calculation could be added here
    }

    pub fn progress_percent(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.current as f32 / self.total as f32) * 100.0
    }
}

// Task snapshot for UI display
#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub id: String,
    pub name: String,
    pub status: TaskUiState,
    pub progress: f32,
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

// Task UI state
#[derive(Debug, Clone, PartialEq)]
pub enum TaskUiState {
    Running,
    Completed,
    Failed(String),
    Pending,
    Paused,
}