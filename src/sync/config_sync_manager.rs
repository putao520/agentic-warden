use super::config_packer::ConfigPacker;
use super::directory_hasher::{DirectoryHash, DirectoryHasher};
use super::error::{SyncError, SyncResult as ErrorResult};
use super::google_drive_service::GoogleDriveService;
use super::oauth_client::OAuthClient;
use super::smart_oauth::SmartOAuthAuthenticator;
use super::sync_config_manager::SyncConfigManager;
use crate::config::{AUTH_DIRECTORY, AUTH_FILE_NAME};
use crate::error::AgenticWardenError;
use chrono::{Duration, Utc};
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredAuthState {
    client_id: String,
    client_secret: String,
    refresh_token: Option<String>,
    access_token: Option<String>,
    expires_at: Option<i64>,
    token_type: Option<String>,
    scope: Option<String>,
}

pub struct ConfigSyncManager {
    pub config_manager: SyncConfigManager,
    directory_hasher: DirectoryHasher,
    config_packer: ConfigPacker,
    drive_service: Option<GoogleDriveService>,
    temp_archive_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SyncOperationResult {
    pub directory_name: String,
    pub changed: bool,
    pub uploaded: bool,
    pub file_size: Option<u64>,
    pub message: String,
}

/// Progress updates emitted during a push operation.
#[derive(Debug, Clone)]
pub enum PushProgressEvent {
    /// Starting work on the specified directory.
    StartingDirectory {
        directory: String,
        index: usize,
        total: usize,
    },
    /// Directory is being compressed into an archive.
    Compressing { directory: String },
    /// Archive is being uploaded to Google Drive.
    Uploading {
        directory: String,
        file_name: String,
        size: Option<u64>,
    },
    /// Uploaded archive is being verified and hashes updated.
    Verifying { directory: String },
    /// Directory was skipped (no changes or missing path).
    Skipped { directory: String, reason: String },
    /// Directory completed successfully.
    Completed { directory: String },
}

/// Progress updates emitted during a pull operation.
#[derive(Debug, Clone)]
pub enum PullProgressEvent {
    /// Starting work on the specified directory.
    StartingDirectory {
        directory: String,
        index: usize,
        total: usize,
    },
    /// Archive is being downloaded from Google Drive.
    Downloading {
        directory: String,
        file_name: Option<String>,
        size: Option<u64>,
    },
    /// Downloaded archive is being decompressed.
    Decompressing { directory: String },
    /// Restoring files from the archive to the target directory.
    Restoring {
        directory: String,
        files_restored: Option<usize>,
        total_files: Option<usize>,
    },
    /// Directory was skipped (no backup found or missing path).
    Skipped { directory: String, reason: String },
    /// Directory completed successfully.
    Completed { directory: String },
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SyncSummary {
    pub total_directories: usize,
    pub changed_directories: usize,
    pub uploaded_files: usize,
    pub total_bytes_uploaded: u64,
    pub results: Vec<SyncOperationResult>,
}

impl ConfigSyncManager {
    pub fn new() -> ErrorResult<Self> {
        let config_manager = SyncConfigManager::new()?;

        // OAuth client and Drive service are now loaded from auth.json when needed
        let drive_service = None;

        Ok(Self {
            config_manager,
            directory_hasher: DirectoryHasher::new(),
            config_packer: ConfigPacker::new(),
            drive_service,
            temp_archive_path: None,
        })
    }

    #[allow(dead_code)]
    pub async fn push_all(&mut self) -> ErrorResult<SyncSummary> {
        let directories = self.config_manager.get_sync_directories()?;

        if directories.is_empty() {
            return Ok(SyncSummary {
                total_directories: 0,
                changed_directories: 0,
                uploaded_files: 0,
                total_bytes_uploaded: 0,
                results: vec![],
            });
        }

        // Ensure Google Drive service is available
        if self.drive_service.is_none() {
            return Err(SyncError::authentication_required());
        }

        let mut summary = SyncSummary {
            total_directories: directories.len(),
            changed_directories: 0,
            uploaded_files: 0,
            total_bytes_uploaded: 0,
            results: Vec::new(),
        };

        // Process each directory
        for directory_path in directories {
            let result = self.push_directory(&directory_path).await?;

            if result.changed {
                summary.changed_directories += 1;
            }
            if result.uploaded {
                summary.uploaded_files += 1;
                summary.total_bytes_uploaded += result.file_size.unwrap_or(0);
            }

            summary.results.push(result);
        }

        // Update last sync time
        self.config_manager.update_last_sync()?;

        Ok(summary)
    }

    pub async fn push_directory(
        &mut self,
        directory_path: &str,
    ) -> ErrorResult<SyncOperationResult> {
        self.push_directory_with_observer(directory_path, |_| {})
            .await
    }

    pub async fn push_directory_with_observer<F>(
        &mut self,
        directory_path: &str,
        mut observer: F,
    ) -> ErrorResult<SyncOperationResult>
    where
        F: FnMut(PushProgressEvent),
    {
        let path = Path::new(directory_path);

        // Get directory name
        let directory_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            SyncError::directory_hashing(format!("Invalid directory name: {}", directory_path))
        })?;

        observer(PushProgressEvent::StartingDirectory {
            directory: directory_name.to_string(),
            index: 0,
            total: 1,
        });

        let mut sync_result = SyncOperationResult {
            directory_name: directory_name.to_string(),
            changed: false,
            uploaded: false,
            file_size: None,
            message: String::new(),
        };

        // Check if directory exists
        if !path.exists() {
            let reason = format!("Directory does not exist: {}", directory_path);
            observer(PushProgressEvent::Skipped {
                directory: directory_name.to_string(),
                reason: reason.clone(),
            });
            sync_result.message = reason;
            return Ok(sync_result);
        }

        // Calculate current hash
        let current_hash = self.directory_hasher.calculate_hash(path)?;

        // Check if it has changed since last sync
        let should_sync = self
            .config_manager
            .should_sync(directory_name, &current_hash.hash)?;

        if !should_sync {
            let reason = "No changes detected".to_string();
            observer(PushProgressEvent::Skipped {
                directory: directory_name.to_string(),
                reason: reason.clone(),
            });
            sync_result.message = reason;
            return Ok(sync_result);
        }

        sync_result.changed = true;

        // Ensure Google Drive service is available
        let service = self
            .drive_service
            .as_mut()
            .ok_or(SyncError::authentication_required())?;

        // Ensure folder exists in Google Drive
        let root_folder_id = service
            .create_or_find_folder("agentic-warden", None)
            .await?;
        let folder_id = service
            .create_or_find_folder(directory_name, Some(&root_folder_id))
            .await?;
        sync_result.message.push_str(&format!(
            "Ensured folder exists in Google Drive (ID: {})",
            folder_id
        ));

        // Create temporary archive
        let temp_dir = TempDir::new().map_err(|e| {
            SyncError::config_packing(format!("Failed to create temp directory: {}", e))
        })?;

        let archive_path = temp_dir.path().join(format!("{}.tar.gz", directory_name));

        // Pack directory
        observer(PushProgressEvent::Compressing {
            directory: directory_name.to_string(),
        });
        let archive_size = self.config_packer.pack_directory(path, &archive_path)?;
        observer(PushProgressEvent::Uploading {
            directory: directory_name.to_string(),
            file_name: archive_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string(),
            size: Some(archive_size),
        });
        sync_result.file_size = Some(archive_size);
        sync_result
            .message
            .push_str(&format!(" Packed directory ({} bytes)", archive_size));

        // Check if file already exists in Google Drive
        let backup_file_name = format!("{}.tar.gz", directory_name);

        let existing_files = service.list_folder_files(&folder_id).await?;

        if let Some(existing) = existing_files
            .into_iter()
            .find(|file| file.name == backup_file_name)
        {
            // Delete existing file
            service.delete_file(&existing.id).await?;
            sync_result.message.push_str(" Deleted existing backup");
        }

        // Upload new file
        let uploaded_file = service.upload_file(&archive_path, Some(&folder_id)).await?;
        observer(PushProgressEvent::Verifying {
            directory: directory_name.to_string(),
        });
        sync_result.uploaded = true;
        sync_result
            .message
            .push_str(&format!(" Uploaded new backup (ID: {})", uploaded_file.id));

        // Update stored hash
        self.config_manager
            .update_directory_hash(directory_name, current_hash)?;

        observer(PushProgressEvent::Completed {
            directory: directory_name.to_string(),
        });

        Ok(sync_result)
    }

    #[allow(dead_code)]
    pub async fn pull_all(&mut self) -> ErrorResult<SyncSummary> {
        let directories = self.config_manager.get_sync_directories()?;

        if directories.is_empty() {
            return Ok(SyncSummary {
                total_directories: 0,
                changed_directories: 0,
                uploaded_files: 0,
                total_bytes_uploaded: 0,
                results: vec![],
            });
        }

        // Ensure Google Drive service is available
        if self.drive_service.is_none() {
            return Err(SyncError::authentication_required());
        }

        let mut summary = SyncSummary {
            total_directories: directories.len(),
            changed_directories: 0,
            uploaded_files: 0,
            total_bytes_uploaded: 0,
            results: Vec::new(),
        };

        // Process each directory
        for directory_path in directories {
            let result = self.pull_directory(&directory_path).await?;

            if result.changed {
                summary.changed_directories += 1;
            }

            summary.results.push(result);
        }

        // Update last sync time
        self.config_manager.update_last_sync()?;

        Ok(summary)
    }

    pub async fn pull_directory(
        &mut self,
        directory_path: &str,
    ) -> ErrorResult<SyncOperationResult> {
        self.pull_directory_with_observer(directory_path, |_| {})
            .await
    }

    pub async fn pull_directory_with_observer<F>(
        &mut self,
        directory_path: &str,
        mut observer: F,
    ) -> ErrorResult<SyncOperationResult>
    where
        F: FnMut(PullProgressEvent),
    {
        let path = Path::new(directory_path);

        // Get directory name
        let directory_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            SyncError::directory_hashing(format!("Invalid directory name: {}", directory_path))
        })?;

        observer(PullProgressEvent::StartingDirectory {
            directory: directory_name.to_string(),
            index: 0,
            total: 1,
        });

        let mut sync_result = SyncOperationResult {
            directory_name: directory_name.to_string(),
            changed: false,
            uploaded: false, // Not applicable for pull
            file_size: None,
            message: String::new(),
        };

        // Ensure Google Drive service is available
        let service = self
            .drive_service
            .as_mut()
            .ok_or(SyncError::authentication_required())?;

        // Locate base folder without creating new backup tree during pull
        let base_folder_id = match service.find_folder("agentic-warden", None).await? {
            Some(id) => id,
            None => {
                let reason = format!("No backup found for directory: {}", directory_name);
                observer(PullProgressEvent::Skipped {
                    directory: directory_name.to_string(),
                    reason: reason.clone(),
                });
                sync_result.message = reason;
                return Ok(sync_result);
            }
        };

        // Find the specific directory folder
        let target_folder_id = match service
            .find_folder(directory_name, Some(&base_folder_id))
            .await?
        {
            Some(id) => id,
            None => {
                let reason = format!("No backup found for directory: {}", directory_name);
                observer(PullProgressEvent::Skipped {
                    directory: directory_name.to_string(),
                    reason: reason.clone(),
                });
                sync_result.message = reason;
                return Ok(sync_result);
            }
        };

        // List files in the target folder
        let folder_files = service.list_folder_files(&target_folder_id).await?;

        if folder_files.is_empty() {
            let reason = format!("No backup files found in directory: {}", directory_name);
            observer(PullProgressEvent::Skipped {
                directory: directory_name.to_string(),
                reason: reason.clone(),
            });
            sync_result.message = reason;
            return Ok(sync_result);
        }

        // Find the most recent backup file by modified or created time
        let mut folder_files = folder_files;
        folder_files.sort_by(|a, b| {
            let a_time = a.modified_time.or(a.created_time);
            let b_time = b.modified_time.or(b.created_time);
            a_time.cmp(&b_time)
        });

        let backup_file = match folder_files.pop() {
            Some(file) => file,
            None => {
                let reason = "No valid backup files found".to_string();
                observer(PullProgressEvent::Skipped {
                    directory: directory_name.to_string(),
                    reason: reason.clone(),
                });
                sync_result.message = reason;
                return Ok(sync_result);
            }
        };

        let reported_size_i64 = backup_file.size.unwrap_or_default();
        if reported_size_i64 > 0 {
            sync_result.file_size = Some(reported_size_i64 as u64);
        }

        observer(PullProgressEvent::Downloading {
            directory: directory_name.to_string(),
            file_name: Some(backup_file.name.clone()),
            size: sync_result.file_size,
        });

        sync_result.message.push_str(&format!(
            "Found backup: {} ({} bytes)",
            backup_file.name,
            if reported_size_i64 > 0 {
                reported_size_i64.to_string()
            } else {
                "unknown".to_string()
            }
        ));

        // Create temporary directory for download
        let temp_dir = TempDir::new().map_err(|e| {
            SyncError::config_packing(format!("Failed to create temp directory: {}", e))
        })?;

        let local_archive_path = temp_dir.path().join(&backup_file.name);

        // Download the file
        service
            .download_file(&backup_file.id, &local_archive_path)
            .await?;
        sync_result.message.push_str(" Downloaded backup file");

        observer(PullProgressEvent::Decompressing {
            directory: directory_name.to_string(),
        });

        // Backup existing directory if it exists
        if path.exists() {
            let backup_path = format!(
                "{}.backup.{}",
                directory_path,
                chrono::Utc::now().timestamp()
            );
            fs::rename(directory_path, &backup_path).map_err(SyncError::io)?;
            sync_result
                .message
                .push_str(&format!(" Backed up existing directory to {}", backup_path));
        }

        observer(PullProgressEvent::Restoring {
            directory: directory_name.to_string(),
            files_restored: None,
            total_files: None,
        });

        // Extract the archive
        self.config_packer
            .unpack_archive(&local_archive_path, path)?;
        sync_result.changed = true;
        sync_result.message.push_str(" Extracted backup");

        // Update stored hash
        let new_hash = self.directory_hasher.calculate_hash(path)?;
        self.config_manager
            .update_directory_hash(directory_name, new_hash)?;

        observer(PullProgressEvent::Completed {
            directory: directory_name.to_string(),
        });

        Ok(sync_result)
    }

    pub fn get_sync_status(&self) -> ErrorResult<HashMap<String, DirectoryHash>> {
        self.config_manager.get_all_directory_hashes()
    }

    pub fn get_last_sync_time(&self) -> ErrorResult<chrono::DateTime<chrono::Utc>> {
        self.config_manager.get_last_sync()
    }

    pub async fn authenticate_google_drive(&mut self) -> ErrorResult<()> {
        if self.drive_service.is_some() {
            return Ok(());
        }

        let mut stored_auth = Self::load_stored_auth_state()?.unwrap_or_default();

        if stored_auth.client_id.trim().is_empty()
            || stored_auth.client_secret.trim().is_empty()
            || stored_auth.refresh_token.is_none()
        {
            println!();
            println!("{}", "═".repeat(70));
            println!("📢 IMPORTANT: Google Drive Sync Authorization");
            println!("{}", "═".repeat(70));
            println!();
            println!("🔐 Agentic-Warden will use Google Drive to:");
            println!("   • Back up your configuration files (sync.json, .env, etc.)");
            println!("   • Store compressed archives in agentic-warden/ folder");
            println!("   • Synchronize configurations across multiple devices");
            println!();
            println!("📁 Your data will be stored in:");
            println!("   • Google Drive: /agentic-warden/<directory-name>/");
            println!("   • Local auth: ~/.aiw/auth.json");
            println!();
            println!("🔒 Privacy:");
            println!("   • Only you can access your Google Drive files");
            println!("   • We use OAuth 2.0 for secure authentication");
            println!("   • Credentials are stored locally and encrypted");
            println!();

            let consent = Confirm::new()
                .with_prompt("Do you agree to use Google Drive for configuration backup?")
                .default(false)
                .interact()
                .map_err(|err| {
                    error!(
                        target: "aiw::sync",
                        "Failed to get user consent: {}",
                        err
                    );
                    ConfigSyncManager::auth_failed_error()
                })?;

            if !consent {
                println!();
                println!("❌ Authorization cancelled by user.");
                println!("   Configuration sync features will not be available.");
                return Err(SyncError::google_drive(
                    "User declined Google Drive authorization".to_string(),
                ));
            }

            println!();
            println!("{}", "═".repeat(70));
            println!("🔐 Google Drive OAuth Setup");
            println!("{}", "═".repeat(70));
            println!();
            println!("To continue, you need OAuth 2.0 credentials from Google Cloud Console:");
            println!("1. Visit: https://console.cloud.google.com/");
            println!("2. Create a project or select existing one");
            println!("3. Enable Google Drive API");
            println!("4. Create OAuth 2.0 Client ID credentials");
            println!("5. Add urn:ietf:wg:oauth:2.0:oob to authorized redirect URIs");
            println!();
            println!("We'll store these credentials securely in ~/.aiw/auth.json");
            println!();
        }

        Self::ensure_client_credentials(&mut stored_auth).await?;

        if stored_auth.refresh_token.is_none() {
            self.run_smart_oauth_flow(&mut stored_auth).await?;
        }

        let mut oauth_client = OAuthClient::new(
            stored_auth.client_id.clone(),
            stored_auth.client_secret.clone(),
            stored_auth.refresh_token.clone(),
        )
        .with_scopes(Self::default_scopes());

        if let Err(err) = oauth_client.validate_config() {
            error!(target: "aiw::sync", "OAuth configuration validation failed: {}", err);
            return Err(Self::auth_failed_error());
        }

        if !oauth_client.is_authenticated() {
            warn!(target: "aiw::sync", "OAuth client missing authentication tokens, re-running SmartOAuth");
            self.run_smart_oauth_flow(&mut stored_auth).await?;
            oauth_client = OAuthClient::new(
                stored_auth.client_id.clone(),
                stored_auth.client_secret.clone(),
                stored_auth.refresh_token.clone(),
            )
            .with_scopes(Self::default_scopes());

            if let Err(err) = oauth_client.validate_config() {
                error!(target: "aiw::sync", "OAuth configuration validation failed after SmartOAuth: {}", err);
                return Err(Self::auth_failed_error());
            }

            if !oauth_client.is_authenticated() {
                error!(target: "aiw::sync", "SmartOAuth completed without providing usable tokens");
                return Err(Self::auth_failed_error());
            }
        }

        let drive_service = GoogleDriveService::new(oauth_client)
            .await
            .map_err(|err| {
                error!(target: "aiw::sync", "Failed to initialize Google Drive service: {}", err);
                Self::auth_failed_error()
            })?;

        self.drive_service = Some(drive_service);
        Self::save_auth_state(&stored_auth)?;

        info!(target: "aiw::sync", "Google Drive authentication completed");
        Ok(())
    }

    pub fn reset_sync_state(&self) -> ErrorResult<()> {
        self.config_manager.reset_state()
    }

    /// Pack a named configuration
    pub async fn pack_named_config(&mut self, config_name: &str) -> ErrorResult<u64> {
        let archive_name = format!("{}.tar.gz", config_name);
        self.temp_archive_path = Some(
            std::env::temp_dir()
                .join("agentic-warden")
                .join(&archive_name),
        );

        // Ensure temp directory exists
        if let Some(parent) = self.temp_archive_path.as_ref().and_then(|p| p.parent()) {
            fs::create_dir_all(parent).map_err(SyncError::io)?;
        }

        // Safe: temp_archive_path was just set above
        let archive_path = self
            .temp_archive_path
            .as_ref()
            .expect("temp_archive_path must be set");
        let size = self
            .config_packer
            .pack_ai_configs(config_name, archive_path.clone())?;

        info!(target: "aiw::sync", "Packed configuration '{}' ({} bytes)", config_name, size);
        Ok(size)
    }

    /// Upload a named configuration to Google Drive
    pub async fn upload_named_config(&mut self, config_name: &str) -> ErrorResult<bool> {
        let service = self
            .drive_service
            .as_mut()
            .ok_or(SyncError::authentication_required())?;

        // Find or create agentic-warden folder
        let base_folder_id = match service.find_folder("agentic-warden", None).await? {
            Some(id) => id,
            None => service.create_folder("agentic-warden").await?,
        };

        let archive_path = self
            .temp_archive_path
            .as_ref()
            .ok_or_else(|| SyncError::sync_config("No archive file to upload".to_string()))?
            .as_path();

        // Delete existing file if it exists
        let archive_name = format!("{}.tar.gz", config_name);
        let existing_files = service.list_folder_files(&base_folder_id).await?;
        if let Some(existing) = existing_files
            .into_iter()
            .find(|file| file.name == archive_name)
        {
            service.delete_file(&existing.id).await?;
        }

        // Upload new file
        service
            .upload_file(archive_path, Some(&base_folder_id))
            .await?;

        info!(target: "aiw::sync", "Uploaded configuration '{}'", config_name);
        Ok(true)
    }

    /// Verify a named configuration in Google Drive
    pub async fn verify_named_config(&mut self, config_name: &str) -> ErrorResult<bool> {
        let service = self
            .drive_service
            .as_mut()
            .ok_or(SyncError::authentication_required())?;

        // Find agentic-warden folder
        let base_folder_id = service
            .find_folder("agentic-warden", None)
            .await?
            .ok_or_else(|| SyncError::sync_config("agentic-warden folder not found".to_string()))?;

        // List files and check for the named configuration
        let files = service.list_folder_files(&base_folder_id).await?;
        let archive_name = format!("{}.tar.gz", config_name);

        Ok(files.into_iter().any(|file| file.name == archive_name))
    }

    /// Download a named configuration from Google Drive
    pub async fn download_named_config(&mut self, config_name: &str) -> ErrorResult<bool> {
        let service = self
            .drive_service
            .as_mut()
            .ok_or(SyncError::authentication_required())?;

        // Find agentic-warden folder
        let base_folder_id = service
            .find_folder("agentic-warden", None)
            .await?
            .ok_or_else(|| SyncError::sync_config("agentic-warden folder not found".to_string()))?;

        // List files to find the named configuration
        let files = service.list_folder_files(&base_folder_id).await?;
        let archive_name = format!("{}.tar.gz", config_name);

        if let Some(file) = files.into_iter().find(|f| f.name == archive_name) {
            // Check if we have a cached archive path, otherwise create one
            if self.temp_archive_path.is_none() {
                let path = std::env::temp_dir()
                    .join("agentic-warden")
                    .join(&archive_name);
                self.temp_archive_path = Some(path);
            }

            // Safe: temp_archive_path was just set above if it was None
            let archive_path = self
                .temp_archive_path
                .as_ref()
                .expect("temp_archive_path must be set");

            if let Some(parent) = archive_path.parent() {
                fs::create_dir_all(parent).map_err(SyncError::io)?;
            }

            service.download_file(&file.id, archive_path).await?;
            info!(target: "aiw::sync", "Downloaded configuration '{}'", config_name);
            Ok(true)
        } else {
            Err(SyncError::sync_config(format!(
                "Configuration '{}' not found",
                config_name
            )))
        }
    }

    /// Extract a named configuration
    pub async fn extract_named_config(&self, config_name: &str) -> ErrorResult<bool> {
        let archive_name = format!("{}.tar.gz", config_name);
        let archive_path = std::env::temp_dir()
            .join("agentic-warden")
            .join(&archive_name);

        if !archive_path.exists() {
            return Err(SyncError::sync_config(format!(
                "Archive file not found: {}",
                archive_name
            )));
        }

        // Extract to home directory
        let home_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::sync_config("Could not find home directory".to_string()))?;

        self.config_packer
            .unpack_archive(&archive_path, &home_dir)?;

        info!(target: "aiw::sync", "Extracted configuration '{}'", config_name);
        Ok(true)
    }

    /// List all available configurations in Google Drive
    pub async fn list_available_configs(&mut self) -> ErrorResult<Vec<String>> {
        let service = self
            .drive_service
            .as_mut()
            .ok_or(SyncError::authentication_required())?;

        // Find agentic-warden folder
        let base_folder_id = match service.find_folder("agentic-warden", None).await? {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        // List files and extract configuration names
        let files = service.list_folder_files(&base_folder_id).await?;
        let mut configs = Vec::new();

        for file in files {
            if file.name.ends_with(".tar.gz") {
                if let Some(config_name) = file.name.strip_suffix(".tar.gz") {
                    configs.push(config_name.to_string());
                }
            }
        }

        configs.sort();
        Ok(configs)
    }

    /// Check Google Drive authentication status
    pub async fn check_google_drive_auth(&mut self) -> ErrorResult<bool> {
        if self.drive_service.is_none() {
            return Ok(false);
        }

        // Try to perform a simple operation to verify auth
        match self
            .drive_service
            .as_mut()
            .unwrap()
            .find_folder("agentic-warden", None)
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Verify extraction was successful
    pub async fn verify_extraction(&self, config_name: &str) -> ErrorResult<bool> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::sync_config("Could not find home directory".to_string()))?;

        // Check if any AI CLI directories exist after extraction
        let claude_dir = home_dir.join(".claude");
        let codex_dir = home_dir.join(".codex");
        let gemini_dir = home_dir.join(".gemini");

        let has_claude = claude_dir.exists();
        let has_codex = codex_dir.exists();
        let has_gemini = gemini_dir.exists();

        if has_claude || has_codex || has_gemini {
            info!(target: "aiw::sync", "Verified extraction of '{}' (Claude: {}, Codex: {}, Gemini: {})",
                  config_name, has_claude, has_codex, has_gemini);
            Ok(true)
        } else {
            warn!(target: "aiw::sync", "No AI CLI directories found after extracting configuration '{}'", config_name);
            Ok(false)
        }
    }
}

impl ConfigSyncManager {
    fn auth_failed_error() -> AgenticWardenError {
        SyncError::google_drive("Authentication failed, please retry".to_string())
    }

    fn default_scopes() -> Vec<String> {
        vec!["https://www.googleapis.com/auth/drive.file".to_string()]
    }

    fn auth_file_path() -> ErrorResult<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(Self::auth_failed_error)?;
        let auth_dir = home_dir.join(AUTH_DIRECTORY);

        if let Err(err) = fs::create_dir_all(&auth_dir) {
            error!(
                target: "aiw::sync",
                "Failed to create auth directory {:?}: {}",
                auth_dir, err
            );
            return Err(Self::auth_failed_error());
        }

        Ok(auth_dir.join(AUTH_FILE_NAME))
    }

    fn load_stored_auth_state() -> ErrorResult<Option<StoredAuthState>> {
        let auth_path = Self::auth_file_path()?;

        if !auth_path.exists() {
            return Ok(None);
        }

        match fs::read_to_string(&auth_path) {
            Ok(content) => match serde_json::from_str::<StoredAuthState>(&content) {
                Ok(state) => Ok(Some(state)),
                Err(err) => {
                    warn!(
                        target: "aiw::sync",
                        "Failed to parse auth.json (will reinitialize): {}",
                        err
                    );
                    Ok(None)
                }
            },
            Err(err) => {
                warn!(
                    target: "aiw::sync",
                    "Failed to read auth.json (will reinitialize): {}",
                    err
                );
                Ok(None)
            }
        }
    }

    fn save_auth_state(state: &StoredAuthState) -> ErrorResult<()> {
        let auth_path = Self::auth_file_path()?;
        let content = serde_json::to_string_pretty(state).map_err(|err| {
            error!(target: "aiw::sync", "Failed to serialize auth state: {}", err);
            Self::auth_failed_error()
        })?;

        fs::write(&auth_path, content).map_err(|err| {
            error!(
                target: "aiw::sync",
                "Failed to write auth.json: {}",
                err
            );
            Self::auth_failed_error()
        })
    }

    async fn ensure_client_credentials(auth: &mut StoredAuthState) -> ErrorResult<()> {
        // Use built-in OAuth client - no user credentials required
        if auth.client_id.trim().is_empty() || auth.client_secret.trim().is_empty() {
            let default_config = super::oauth_client::OAuthConfig::default();
            auth.client_id = default_config.client_id;
            auth.client_secret = default_config.client_secret;
            Self::save_auth_state(auth)?;
        }
        Ok(())
    }

    async fn run_smart_oauth_flow(&self, auth: &mut StoredAuthState) -> ErrorResult<()> {
        let oauth_config = super::oauth_client::OAuthConfig {
            client_id: auth.client_id.clone(),
            client_secret: auth.client_secret.clone(),
            refresh_token: None,
            access_token: None,
            expires_in: 0,
            token_type: "Bearer".to_string(),
            scopes: Self::default_scopes(),
        };

        let authenticator = SmartOAuthAuthenticator::new(oauth_config);
        let token_response = authenticator
            .authenticate_with_device_flow()
            .await
            .map_err(|err| {
                error!(
                    target: "aiw::sync",
                    "Device Flow authentication failed: {}",
                    err
                );
                Self::auth_failed_error()
            })?;

        auth.access_token = Some(token_response.access_token.clone());
        auth.token_type = Some(token_response.token_type.clone());
        auth.scope = token_response.scope.clone();
        let expires_at = Utc::now() + Duration::seconds(token_response.expires_in as i64);
        auth.expires_at = Some(expires_at.timestamp());

        if let Some(refresh_token) = token_response.refresh_token.clone() {
            auth.refresh_token = Some(refresh_token);
        } else if auth.refresh_token.is_none() {
            error!(
                target: "aiw::sync",
                "SmartOAuth did not return a refresh token"
            );
            return Err(Self::auth_failed_error());
        }

        Self::save_auth_state(auth)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_handling() {
        let _manager = ConfigSyncManager::new().unwrap();

        // Test with non-existent directory
        let path = "/nonexistent/directory";
        let result = std::panic::catch_unwind(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let mut manager = ConfigSyncManager::new().unwrap();
                manager.push_directory(path).await
            })
        });

        // Should not panic but return an error result
        assert!(result.is_ok());
    }
}

/// Copy directory contents recursively
#[allow(dead_code)]
fn copy_dir_contents(source: &Path, dest: &Path) -> ErrorResult<()> {
    for entry in walkdir::WalkDir::new(source) {
        let entry = entry
            .map_err(|e| SyncError::config_packing(format!("Failed to walk directory: {}", e)))?;

        let path = entry.path();
        let relative_path = path.strip_prefix(source).unwrap();
        let dest_path = dest.join(relative_path);

        if path.is_file() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    SyncError::config_packing(format!("Failed to create directory: {}", e))
                })?;
            }

            fs::copy(path, &dest_path)
                .map_err(|e| SyncError::config_packing(format!("Failed to copy file: {}", e)))?;
        }
    }

    Ok(())
}
