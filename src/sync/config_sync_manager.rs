use super::config_packer::ConfigPacker;
use super::directory_hasher::{DirectoryHash, DirectoryHasher};
use super::error::{SyncError, SyncResult as ErrorResult};
use super::google_drive_client::{GoogleDriveClient, GoogleDriveConfig};
use super::sync_config_manager::SyncConfigManager;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub struct ConfigSyncManager {
    pub config_manager: SyncConfigManager,
    directory_hasher: DirectoryHasher,
    config_packer: ConfigPacker,
    google_drive_client: Option<GoogleDriveClient>,
}

#[derive(Debug, Clone)]
pub struct SyncOperationResult {
    pub directory_name: String,
    pub changed: bool,
    pub uploaded: bool,
    pub file_size: Option<u64>,
    pub message: String,
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

        // Google Drive client is now loaded from auth.json when needed
        let google_drive_client = None;

        Ok(Self {
            config_manager,
            directory_hasher: DirectoryHasher::new(),
            config_packer: ConfigPacker::new(),
            google_drive_client,
        })
    }

    /// Create with Google Drive client (deprecated - use auth.json instead)
    #[allow(dead_code)]
    pub fn with_google_drive_config(
        drive_config: super::google_drive_client::GoogleDriveConfig,
    ) -> ErrorResult<Self> {
        let config_manager = SyncConfigManager::new()?;

        Ok(Self {
            config_manager,
            directory_hasher: DirectoryHasher::new(),
            config_packer: ConfigPacker::new(),
            google_drive_client: Some(GoogleDriveClient::new(drive_config)),
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

        // Ensure Google Drive client is available
        if self.google_drive_client.is_none() {
            return Err(SyncError::AuthenticationRequired);
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
        let path = Path::new(directory_path);

        // Get directory name
        let directory_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            SyncError::DirectoryHashingError(format!("Invalid directory name: {}", directory_path))
        })?;

        let mut sync_result = SyncOperationResult {
            directory_name: directory_name.to_string(),
            changed: false,
            uploaded: false,
            file_size: None,
            message: String::new(),
        };

        // Check if directory exists
        if !path.exists() {
            sync_result.message = format!("Directory does not exist: {}", directory_path);
            return Ok(sync_result);
        }

        // Calculate current hash
        let current_hash = self.directory_hasher.calculate_hash(path)?;

        // Check if it has changed since last sync
        let should_sync = self
            .config_manager
            .should_sync(directory_name, &current_hash.hash)?;

        if !should_sync {
            sync_result.message = "No changes detected".to_string();
            return Ok(sync_result);
        }

        sync_result.changed = true;

        // Ensure Google Drive client is available
        let client = self
            .google_drive_client
            .as_mut()
            .ok_or(SyncError::AuthenticationRequired)?;

        // Ensure folder exists in Google Drive
        let folder_id = client.ensure_folder_exists(directory_name).await?;
        sync_result.message.push_str(&format!(
            "Ensured folder exists in Google Drive (ID: {})",
            folder_id
        ));

        // Create temporary archive
        let temp_dir = TempDir::new().map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to create temp directory: {}", e))
        })?;

        let archive_path = temp_dir.path().join(format!("{}.tar.gz", directory_name));

        // Pack directory
        let archive_size = self.config_packer.pack_directory(path, &archive_path)?;
        sync_result.file_size = Some(archive_size);
        sync_result
            .message
            .push_str(&format!(" Packed directory ({} bytes)", archive_size));

        // Check if file already exists in Google Drive
        let existing_file = client
            .find_file(&folder_id, &format!("{}.tar.gz", directory_name))
            .await?;

        if let Some(existing) = existing_file {
            // Delete existing file
            client.delete_file(&existing.id).await?;
            sync_result.message.push_str(" Deleted existing backup");
        }

        // Upload new file
        let uploaded_file = client.upload_file(&archive_path, &folder_id).await?;
        sync_result.uploaded = true;
        sync_result
            .message
            .push_str(&format!(" Uploaded new backup (ID: {})", uploaded_file.id));

        // Update stored hash
        self.config_manager
            .update_directory_hash(directory_name, current_hash)?;

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

        // Ensure Google Drive client is available
        if self.google_drive_client.is_none() {
            return Err(SyncError::AuthenticationRequired);
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
        let path = Path::new(directory_path);

        // Get directory name
        let directory_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            SyncError::DirectoryHashingError(format!("Invalid directory name: {}", directory_path))
        })?;

        let mut sync_result = SyncOperationResult {
            directory_name: directory_name.to_string(),
            changed: false,
            uploaded: false, // Not applicable for pull
            file_size: None,
            message: String::new(),
        };

        // Ensure Google Drive client is available
        let client = self
            .google_drive_client
            .as_mut()
            .ok_or(SyncError::AuthenticationRequired)?;

        // Ensure base folder exists
        let base_folder_id = client.ensure_folder_exists("agentic-warden").await?;

        // Find the specific directory folder
        let files = client.list_files(&base_folder_id).await?;
        let target_folder = files.iter().find(|f| {
            f.name == directory_name && f.mime_type == "application/vnd.google-apps.folder"
        });

        let target_folder = match target_folder {
            Some(folder) => folder,
            None => {
                sync_result.message = format!("No backup found for directory: {}", directory_name);
                return Ok(sync_result);
            }
        };

        // List files in the target folder
        let folder_files = client.list_files(&target_folder.id).await?;

        if folder_files.is_empty() {
            sync_result.message = format!("No backup files found in directory: {}", directory_name);
            return Ok(sync_result);
        }

        // Find the most recent backup file
        let backup_file = folder_files
            .iter()
            .max_by(|a, b| a.modified_time.cmp(&b.modified_time));

        let backup_file = match backup_file {
            Some(file) => file,
            None => {
                sync_result.message = "No valid backup files found".to_string();
                return Ok(sync_result);
            }
        };

        sync_result.message.push_str(&format!(
            "Found backup: {} ({} bytes)",
            backup_file.name,
            backup_file.size.as_deref().unwrap_or("unknown")
        ));

        // Create temporary directory for download
        let temp_dir = TempDir::new().map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to create temp directory: {}", e))
        })?;

        let local_archive_path = temp_dir.path().join(&backup_file.name);

        // Download the file
        client
            .download_file(&backup_file.id, local_archive_path.to_str().unwrap())
            .await?;
        sync_result.message.push_str(" Downloaded backup file");

        // Backup existing directory if it exists
        if path.exists() {
            let backup_path = format!(
                "{}.backup.{}",
                directory_path,
                chrono::Utc::now().timestamp()
            );
            fs::rename(directory_path, &backup_path).map_err(SyncError::IoError)?;
            sync_result
                .message
                .push_str(&format!(" Backed up existing directory to {}", backup_path));
        }

        // Extract the archive
        self.config_packer
            .unpack_archive(&local_archive_path, path)?;
        sync_result.changed = true;
        sync_result.message.push_str(" Extracted backup");

        // Update stored hash
        let new_hash = self.directory_hasher.calculate_hash(path)?;
        self.config_manager
            .update_directory_hash(directory_name, new_hash)?;

        Ok(sync_result)
    }

    pub fn get_sync_status(&self) -> ErrorResult<HashMap<String, DirectoryHash>> {
        self.config_manager.get_all_directory_hashes()
    }

    pub fn get_last_sync_time(&self) -> ErrorResult<chrono::DateTime<chrono::Utc>> {
        self.config_manager.get_last_sync()
    }

    pub async fn authenticate_google_drive(&mut self) -> ErrorResult<()> {
        if self.google_drive_client.is_none() {
            // Try to load from auth.json
            if let Some(drive_config) = GoogleDriveClient::load_auth_config()? {
                self.google_drive_client = Some(GoogleDriveClient::new(drive_config));
            } else {
                // Start OAuth setup flow since no config exists
                println!("🔧 Google Drive authentication not configured");
                println!("Starting OAuth setup flow...");

                // Get OAuth credentials from user
                let client_id = dialoguer::Input::<String>::new()
                    .with_prompt("Enter Google Client ID")
                    .interact_text()
                    .map_err(|e| {
                        SyncError::GoogleDriveError(format!("Failed to read client ID: {}", e))
                    })?;

                let client_secret = dialoguer::Password::new()
                    .with_prompt("Enter Google Client Secret")
                    .interact()
                    .map_err(|e| {
                        SyncError::GoogleDriveError(format!("Failed to read client secret: {}", e))
                    })?;

                let drive_config = GoogleDriveConfig {
                    client_id,
                    client_secret,
                    access_token: None,
                    refresh_token: None,
                    base_folder_id: None,
                    token_expires_at: None,
                };

                self.google_drive_client = Some(GoogleDriveClient::new(drive_config));
            }
        }

        // Authenticate if needed
        if let Some(client) = &mut self.google_drive_client {
            client.authenticate().await?;
        }

        Ok(())
    }

    pub fn reset_sync_state(&self) -> ErrorResult<()> {
        self.config_manager.reset_state()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let manager = ConfigSyncManager::new().unwrap();
        let status = manager.get_sync_status().unwrap();
        assert_eq!(status.len(), 0);
    }

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
