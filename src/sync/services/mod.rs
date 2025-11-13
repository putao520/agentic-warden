//! Sync Services - Unified service layer for synchronization operations
//!
//! This module provides a unified service layer to eliminate the 336+ repeated
//! sync/upload/download calls across the sync module, following DRY principles.

pub mod auth_manager;
pub mod google_drive_sync_service;
pub mod mock_sync_service;
pub mod progress_reporter;
pub mod sync_service;

pub use google_drive_sync_service::GoogleDriveSyncService;
pub use mock_sync_service::{MockAuthManager, MockSyncService};
pub use sync_service::ProgressReporter;
pub use sync_service::{SyncError, SyncOperation, SyncResult, SyncService};

/// Common synchronization events
#[derive(Debug, Clone)]
pub enum SyncEvent {
    Started { operation: SyncOperation },
    Progress { percent: u8, message: String },
    Warning { message: String },
    Error { error: String },
    Completed { result: SyncResult },
    Cancelled,
}

/// Authentication status
#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    NotConfigured,
    Configured,
    Authenticated,
    Expired,
    Error(String),
}

/// Sync provider types
#[derive(Debug, Clone)]
pub enum SyncProvider {
    GoogleDrive,
    Local,
    Custom(String),
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub provider: SyncProvider,
    pub base_folder: String,
    pub auto_retry: bool,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            provider: SyncProvider::GoogleDrive,
            base_folder: "agentic-warden".to_string(),
            auto_retry: true,
            max_retries: 3,
            timeout_seconds: 300,
        }
    }
}
