//! Mock Sync Service - Testing implementation of SyncService
//!
//! Provides a mock implementation for testing purposes.

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

use super::sync_service::{
    AuthManager, ProgressReporter, RemoteFile, SyncError, SyncOperation, SyncResult, SyncService,
};
use super::AuthStatus;
use crate::sync::services::auth_manager::AuthToken;

/// Mock sync service for testing
pub struct MockSyncService {
    name: String,
    auth_manager: MockAuthManager,
}

impl MockSyncService {
    pub fn new(auth_manager: MockAuthManager) -> Self {
        Self {
            name: "Mock Service".to_string(),
            auth_manager,
        }
    }
}

#[async_trait]
impl SyncService for MockSyncService {
    fn name(&self) -> &str {
        &self.name
    }

    fn auth_manager(&self) -> &dyn AuthManager {
        &self.auth_manager
    }

    async fn configure(&mut self, config: super::SyncConfig) -> Result<()> {
        self.name = format!("Mock Service - {:?}", config.provider);
        Ok(())
    }

    async fn upload_file(
        &mut self,
        local_path: &Path,
        remote_path: &str,
        _progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult> {
        Ok(SyncResult {
            operation: SyncOperation::Upload,
            success: true,
            bytes_transferred: 1024,
            files_processed: 1,
            duration_ms: 100,
            message: format!("Mock uploaded {} to {}", local_path.display(), remote_path),
        })
    }

    async fn download_file(
        &mut self,
        remote_path: &str,
        local_path: &Path,
        _progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult> {
        Ok(SyncResult {
            operation: SyncOperation::Download,
            success: true,
            bytes_transferred: 2048,
            files_processed: 1,
            duration_ms: 150,
            message: format!(
                "Mock downloaded {} to {}",
                remote_path,
                local_path.display()
            ),
        })
    }

    async fn upload_directory(
        &mut self,
        local_dir: &Path,
        remote_dir: &str,
        _progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult> {
        Ok(SyncResult {
            operation: SyncOperation::Upload,
            success: true,
            bytes_transferred: 4096,
            files_processed: 5,
            duration_ms: 500,
            message: format!(
                "Mock uploaded directory {} to {}",
                local_dir.display(),
                remote_dir
            ),
        })
    }

    async fn download_directory(
        &mut self,
        remote_dir: &str,
        local_dir: &Path,
        _progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult> {
        Ok(SyncResult {
            operation: SyncOperation::Download,
            success: true,
            bytes_transferred: 8192,
            files_processed: 8,
            duration_ms: 750,
            message: format!(
                "Mock downloaded directory {} to {}",
                remote_dir,
                local_dir.display()
            ),
        })
    }

    async fn list_files(&mut self, _remote_path: &str) -> Result<Vec<RemoteFile>> {
        Ok(vec![
            RemoteFile::new("test1.txt".to_string(), "test1.txt".to_string()),
            RemoteFile::directory("subdir".to_string(), "subdir".to_string()),
            RemoteFile::new("test2.txt".to_string(), "test2.txt".to_string()),
        ])
    }

    async fn delete_file(&mut self, remote_path: &str) -> Result<SyncResult> {
        Ok(SyncResult {
            operation: SyncOperation::Delete,
            success: true,
            bytes_transferred: 0,
            files_processed: 1,
            duration_ms: 50,
            message: format!("Mock deleted {}", remote_path),
        })
    }

    async fn file_exists(&mut self, remote_path: &str) -> Result<bool> {
        Ok(!remote_path.contains("missing"))
    }

    async fn get_file_metadata(&mut self, remote_path: &str) -> Result<RemoteFile> {
        Ok(RemoteFile::new(
            remote_path.to_string(),
            remote_path.to_string(),
        ))
    }

    async fn create_directory(&mut self, remote_path: &str) -> Result<()> {
        tracing::info!("Mock created directory: {}", remote_path);
        Ok(())
    }

    async fn delete_directory(&mut self, remote_path: &str) -> Result<()> {
        tracing::info!("Mock deleted directory: {}", remote_path);
        Ok(())
    }

    async fn sync_directory(
        &mut self,
        local_dir: &Path,
        remote_dir: &str,
        _progress: Option<&mut dyn ProgressReporter>,
    ) -> Result<SyncResult> {
        Ok(SyncResult {
            operation: SyncOperation::Upload,
            success: true,
            bytes_transferred: 16384,
            files_processed: 12,
            duration_ms: 1000,
            message: format!("Mock synced {} with {}", local_dir.display(), remote_dir),
        })
    }

    async fn verify_sync(&mut self, local_dir: &Path, remote_dir: &str) -> Result<SyncResult> {
        Ok(SyncResult {
            operation: SyncOperation::Verify,
            success: true,
            bytes_transferred: 0,
            files_processed: 10,
            duration_ms: 25,
            message: format!("Mock verified {} with {}", local_dir.display(), remote_dir),
        })
    }
}

/// Mock authentication manager
pub struct MockAuthManager {
    status: AuthStatus,
    token: Option<AuthToken>,
}

impl MockAuthManager {
    pub fn new() -> Self {
        Self {
            status: AuthStatus::Authenticated,
            token: Some(AuthToken::new("mock_token".to_string())),
        }
    }
}

impl Default for MockAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MockAuthManager {
    fn clone(&self) -> Self {
        Self {
            status: self.status.clone(),
            token: self.token.clone(),
        }
    }
}

#[async_trait]
impl AuthManager for MockAuthManager {
    async fn get_auth_status(&self) -> AuthStatus {
        self.status.clone()
    }

    async fn authenticate(&mut self) -> Result<AuthStatus> {
        self.status = AuthStatus::Authenticated;
        self.token = Some(AuthToken::new("mock_token".to_string()));
        Ok(self.status.clone())
    }

    async fn refresh_auth(&mut self) -> Result<AuthStatus> {
        // Mock refresh - just extend expiration
        if let Some(ref mut token) = self.token {
            *token = token
                .clone()
                .with_expiration(chrono::Utc::now() + chrono::Duration::hours(1));
        }
        Ok(self.status.clone())
    }

    async fn is_authenticated(&self) -> bool {
        matches!(self.status, AuthStatus::Authenticated)
    }

    async fn get_access_token(&self) -> Result<String> {
        self.token
            .as_ref()
            .map(|t| t.access_token.clone())
            .ok_or_else(|| SyncError::AuthenticationRequired.into())
    }
}
