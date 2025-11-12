//! Sync Service - Unified interface for synchronization operations
//!
//! Provides a common interface to eliminate repeated sync logic and consolidate
//! upload/download operations across different providers.

use async_trait::async_trait;
use std::path::Path;
use anyhow::Result;

use super::{SyncConfig, AuthStatus};

/// Synchronization operations
#[derive(Debug, Clone, PartialEq)]
pub enum SyncOperation {
    Upload,
    Download,
    List,
    Delete,
    Verify,
}

/// Synchronization result
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub operation: SyncOperation,
    pub success: bool,
    pub bytes_transferred: u64,
    pub files_processed: usize,
    pub duration_ms: u64,
    pub message: String,
}

/// Synchronization error
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Network error: {0}")]
    Network(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Timeout error: operation exceeded {0} seconds")]
    Timeout(u64),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl SyncError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SyncError::Network(_) | SyncError::Timeout(_) | SyncError::Provider(_)
        )
    }
}

/// Progress reporter trait for sync operations
#[async_trait]
pub trait ProgressReporter: Send + Sync {
    /// Report progress update
    async fn report(&mut self, percent: u8, message: String);

    /// Report error
    async fn error(&mut self, error: String);

    /// Report completion
    async fn complete(&mut self, result: SyncResult);

    /// Check if operation should continue
    async fn should_continue(&self) -> bool;
}

/// Authentication manager for sync services
#[async_trait]
pub trait AuthManager: Send + Sync {
    /// Get current authentication status
    async fn get_auth_status(&self) -> AuthStatus;

    /// Authenticate with the provider
    async fn authenticate(&mut self) -> Result<AuthStatus>;

    /// Refresh authentication if needed
    async fn refresh_auth(&mut self) -> Result<AuthStatus>;

    /// Check if authenticated
    async fn is_authenticated(&self) -> bool;

    /// Get access token for API calls
    async fn get_access_token(&self) -> Result<String>;
}

/// Unified synchronization service trait
#[async_trait]
pub trait SyncService: Send + Sync {
    /// Get service name
    fn name(&self) -> &str;

    /// Get authentication manager
    fn auth_manager(&self) -> &dyn AuthManager;

    /// Configure the service
    async fn configure(&mut self, config: SyncConfig) -> Result<()>;

    /// Upload a file
    async fn upload_file(
        &mut self,
        local_path: &Path,
        remote_path: &str,
        progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult>;

    /// Download a file
    async fn download_file(
        &mut self,
        remote_path: &str,
        local_path: &Path,
        progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult>;

    /// Upload directory contents
    async fn upload_directory(
        &mut self,
        local_dir: &Path,
        remote_dir: &str,
        progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult>;

    /// Download directory contents
    async fn download_directory(
        &mut self,
        remote_dir: &str,
        local_dir: &Path,
        progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult>;

    /// List remote files/directories
    async fn list_files(&mut self, remote_path: &str) -> Result<Vec<RemoteFile>>;

    /// Delete a remote file
    async fn delete_file(&mut self, remote_path: &str) -> Result<SyncResult>;

    /// Check if remote file exists
    async fn file_exists(&mut self, remote_path: &str) -> Result<bool>;

    /// Get file metadata
    async fn get_file_metadata(&mut self, remote_path: &str) -> Result<RemoteFile>;

    /// Create remote directory
    async fn create_directory(&mut self, remote_path: &str) -> Result<()>;

    /// Delete remote directory
    async fn delete_directory(&mut self, remote_path: &str) -> Result<()>;

    /// Sync directory (compare and update)
    async fn sync_directory(
        &mut self,
        local_dir: &Path,
        remote_dir: &str,
        progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult>;

    /// Verify synchronization
    async fn verify_sync(
        &mut self,
        local_dir: &Path,
        remote_dir: &str,
    ) -> Result<SyncResult>;
}

/// Remote file information
#[derive(Debug, Clone)]
pub struct RemoteFile {
    pub path: String,
    pub name: String,
    pub size: Option<u64>,
    pub modified_time: Option<chrono::DateTime<chrono::Utc>>,
    pub is_directory: bool,
    pub mime_type: Option<String>,
}

impl RemoteFile {
    /// Create new remote file
    pub fn new(path: String, name: String) -> Self {
        Self {
            path,
            name,
            size: None,
            modified_time: None,
            is_directory: false,
            mime_type: None,
        }
    }

    /// Create directory entry
    pub fn directory(path: String, name: String) -> Self {
        Self {
            path,
            name,
            size: None,
            modified_time: None,
            is_directory: true,
            mime_type: Some("application/vnd.google-apps.folder".to_string()),
        }
    }
}

/// Sync service factory
pub struct SyncServiceFactory;

impl SyncServiceFactory {
    /// Create sync service for provider
    pub async fn create_service(provider: super::SyncProvider) -> Result<Box<dyn SyncService>> {
        match provider {
            super::SyncProvider::GoogleDrive => {
                let service = super::GoogleDriveSyncService::new().await?;
                Ok(Box::new(service))
            }
            super::SyncProvider::Local => {
                Err(SyncError::Configuration("Local sync not implemented".to_string()).into())
            }
            super::SyncProvider::Custom(name) => {
                Err(SyncError::Configuration(format!("Custom provider '{}' not implemented", name)).into())
            }
        }
    }

    /// Create sync service with configuration
    pub async fn create_service_with_config(
        provider: super::SyncProvider,
        config: SyncConfig,
    ) -> Result<Box<dyn SyncService>> {
        let mut service = Self::create_service(provider).await?;
        service.configure(config).await?;
        Ok(service)
    }
}

/// Default progress reporter implementation
pub struct DefaultProgressReporter {
    start_time: std::time::Instant,
    cancelled: bool,
}

impl DefaultProgressReporter {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            cancelled: false,
        }
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

#[async_trait]
impl ProgressReporter for DefaultProgressReporter {
    async fn report(&mut self, percent: u8, message: String) {
        if !self.cancelled {
            tracing::info!("Progress: {}% - {}", percent, message);
        }
    }

    async fn error(&mut self, error: String) {
        tracing::error!("Sync error: {}", error);
    }

    async fn complete(&mut self, result: SyncResult) {
        let duration = self.start_time.elapsed().as_millis() as u64;
        tracing::info!(
            "Sync completed in {}ms: {} files, {} bytes",
            duration,
            result.files_processed,
            result.bytes_transferred
        );
    }

    async fn should_continue(&self) -> bool {
        !self.cancelled
    }
}

impl Default for DefaultProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}