use super::config_sync_manager::ConfigSyncManager;
use super::error::{SyncError, SyncResult};
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};

pub struct SyncCommand {
    manager: ConfigSyncManager,
}

impl SyncCommand {
    pub fn new() -> SyncResult<Self> {
        Ok(Self {
            manager: ConfigSyncManager::new()?,
        })
    }

    pub async fn execute_push(&mut self, directories: Option<Vec<String>>) -> SyncResult<i32> {
        let term = Term::stdout();

        term.write_line("🚀 Starting configuration sync push...")?;
        term.write_line("")?;

        // Authenticate with Google Drive
        term.write_line("🔐 Authenticating with Google Drive...")?;
        if let Err(e) = self.manager.authenticate_google_drive().await {
            match e {
                SyncError::GoogleDriveError(msg) if msg.contains("GOOGLE_CLIENT_ID") => {
                    term.write_line("❌ Google Drive authentication failed:")?;
                    term.write_line(&format!("   {}", msg))?;
                    term.write_line("")?;
                    term.write_line("To set up Google Drive sync, please set the following environment variables:")?;
                    term.write_line("  export GOOGLE_CLIENT_ID='your_client_id'")?;
                    term.write_line("  export GOOGLE_CLIENT_SECRET='your_client_secret'")?;
                    term.write_line("")?;
                    term.write_line(
                        "You can get these credentials from the Google Cloud Console:",
                    )?;
                    term.write_line("1. Go to https://console.cloud.google.com/")?;
                    term.write_line("2. Create a new project or select existing one")?;
                    term.write_line("3. Enable Google Drive API")?;
                    term.write_line("4. Create OAuth 2.0 credentials")?;
                    term.write_line("5. Add http://localhost:8080 to authorized redirect URIs")?;
                    return Ok(1);
                }
                _ => return Err(e),
            }
        }
        term.write_line("✅ Authentication successful!")?;
        term.write_line("")?;

        // Determine which directories to sync
        let sync_directories = if let Some(dirs) = directories {
            dirs
        } else {
            // Use default directories
            self.manager.config_manager.get_sync_directories()?
        };

        if sync_directories.is_empty() {
            term.write_line("ℹ️  No directories to sync.")?;
            return Ok(0);
        }

        term.write_line(&format!(
            "📁 Checking {} directories for changes...",
            sync_directories.len()
        ))?;
        term.write_line("")?;

        // Create progress bar
        let progress = ProgressBar::new(sync_directories.len() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut total_changed = 0;
        let mut total_uploaded = 0;
        let mut total_bytes = 0u64;
        let mut results = Vec::new();

        for directory in &sync_directories {
            progress.set_message(format!(
                "Processing {}",
                std::path::Path::new(directory)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(directory)
            ));

            match self.manager.push_directory(directory).await {
                Ok(result) => {
                    if result.changed {
                        total_changed += 1;
                        term.write_line(&format!(
                            "📦 {}: {}",
                            result.directory_name, result.message
                        ))?;

                        if result.uploaded {
                            total_uploaded += 1;
                            if let Some(size) = result.file_size {
                                total_bytes += size;
                                term.write_line(&format!("   📤 Uploaded {} bytes", size))?;
                            }
                        }
                    } else {
                        term.write_line(&format!("✅ {}: No changes", result.directory_name))?;
                    }
                    results.push(result);
                }
                Err(e) => {
                    term.write_line(&format!(
                        "❌ {}: {}",
                        std::path::Path::new(directory)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(directory),
                        e
                    ))?;
                }
            }

            progress.inc(1);
        }

        progress.finish_with_message("Sync complete");
        term.write_line("")?;

        // Summary
        term.write_line("📊 Sync Summary:")?;
        term.write_line(&format!("   Total directories: {}", sync_directories.len()))?;
        term.write_line(&format!("   Changed: {}", total_changed))?;
        term.write_line(&format!("   Uploaded: {}", total_uploaded))?;
        if total_bytes > 0 {
            term.write_line(&format!("   Total bytes: {}", total_bytes))?;
        }
        term.write_line("")?;

        if total_uploaded > 0 {
            term.write_line("🎉 Push completed successfully!")?;
            Ok(0)
        } else if total_changed > 0 {
            term.write_line("ℹ️  Some directories had changes but uploads may have failed.")?;
            Ok(1)
        } else {
            term.write_line("✅ All directories are up to date. Nothing to push.")?;
            Ok(0)
        }
    }

    pub async fn execute_pull(&mut self, directories: Option<Vec<String>>) -> SyncResult<i32> {
        let term = Term::stdout();

        term.write_line("📥 Starting configuration sync pull...")?;
        term.write_line("")?;

        // Authenticate with Google Drive
        term.write_line("🔐 Authenticating with Google Drive...")?;
        self.manager.authenticate_google_drive().await?;
        term.write_line("✅ Authentication successful!")?;
        term.write_line("")?;

        // Determine which directories to sync
        let sync_directories = if let Some(dirs) = directories {
            dirs
        } else {
            // Use default directories
            self.manager.config_manager.get_sync_directories()?
        };

        if sync_directories.is_empty() {
            term.write_line("ℹ️  No directories to sync.")?;
            return Ok(0);
        }

        term.write_line(&format!(
            "📁 Checking {} directories for updates...",
            sync_directories.len()
        ))?;
        term.write_line("")?;

        // Create progress bar
        let progress = ProgressBar::new(sync_directories.len() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut total_updated = 0;
        let mut total_bytes = 0u64;
        let mut results = Vec::new();

        for directory in &sync_directories {
            progress.set_message(format!(
                "Processing {}",
                std::path::Path::new(directory)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(directory)
            ));

            match self.manager.pull_directory(directory).await {
                Ok(result) => {
                    if result.changed {
                        total_updated += 1;
                        term.write_line(&format!(
                            "📦 {}: {}",
                            result.directory_name, result.message
                        ))?;

                        if let Some(size) = result.file_size {
                            total_bytes += size;
                            term.write_line(&format!("   📥 Downloaded {} bytes", size))?;
                        }
                    } else {
                        term.write_line(&format!("✅ {}: No updates", result.directory_name))?;
                    }
                    results.push(result);
                }
                Err(e) => {
                    term.write_line(&format!(
                        "❌ {}: {}",
                        std::path::Path::new(directory)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(directory),
                        e
                    ))?;
                }
            }

            progress.inc(1);
        }

        progress.finish_with_message("Sync complete");
        term.write_line("")?;

        // Summary
        term.write_line("📊 Sync Summary:")?;
        term.write_line(&format!("   Total directories: {}", sync_directories.len()))?;
        term.write_line(&format!("   Updated: {}", total_updated))?;
        if total_bytes > 0 {
            term.write_line(&format!("   Total bytes: {}", total_bytes))?;
        }
        term.write_line("")?;

        if total_updated > 0 {
            term.write_line("🎉 Pull completed successfully!")?;
            Ok(0)
        } else {
            term.write_line("✅ All directories are up to date. Nothing to pull.")?;
            Ok(0)
        }
    }

    pub async fn execute_status(&self) -> SyncResult<i32> {
        let term = Term::stdout();

        term.write_line("📊 Configuration Sync Status")?;
        term.write_line("=".repeat(50).as_str())?;
        term.write_line("")?;

        // Get last sync time
        match self.manager.get_last_sync_time() {
            Ok(last_sync) => {
                term.write_line(&format!(
                    "Last sync: {}",
                    last_sync.format("%Y-%m-%d %H:%M:%S UTC")
                ))?;
            }
            Err(_) => {
                term.write_line("Last sync: Never")?;
            }
        }
        term.write_line("")?;

        // Get sync status for all directories
        let sync_status = self.manager.get_sync_status()?;

        if sync_status.is_empty() {
            term.write_line("No synchronized directories found.")?;
            return Ok(0);
        }

        term.write_line("Synchronized directories:")?;
        term.write_line("")?;

        for (dir_name, hash) in sync_status {
            term.write_line(&format!("📁 {}", dir_name))?;
            term.write_line(&format!("   Hash: {}", hash.hash))?;
            term.write_line(&format!("   Files: {}", hash.file_count))?;
            term.write_line(&format!("   Size: {} bytes", hash.total_size))?;
            term.write_line(&format!(
                "   Last checked: {}",
                hash.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            ))?;
            term.write_line("")?;
        }

        Ok(0)
    }

    pub fn execute_reset(&mut self) -> SyncResult<i32> {
        let term = Term::stdout();

        term.write_line("🔄 Resetting sync state...")?;

        // Confirm reset
        let confirm = dialoguer::Confirm::new()
            .with_prompt("This will clear all sync history. Are you sure?")
            .default(false)
            .interact()
            .map_err(|e| {
                SyncError::SyncConfigError(format!("Failed to get confirmation: {}", e))
            })?;

        if !confirm {
            term.write_line("❌ Reset cancelled.")?;
            return Ok(0);
        }

        self.manager.reset_sync_state()?;

        term.write_line("✅ Sync state reset successfully!")?;
        term.write_line("Next sync will treat all directories as new.")?;

        Ok(0)
    }

    pub fn execute_list_directories(&self) -> SyncResult<i32> {
        let term = Term::stdout();

        term.write_line("📁 Monitored Directories")?;
        term.write_line("=".repeat(30).as_str())?;
        term.write_line("")?;

        let directories = self.manager.config_manager.get_sync_directories()?;

        if directories.is_empty() {
            term.write_line("No directories configured for sync.")?;
            return Ok(0);
        }

        for (i, directory) in directories.iter().enumerate() {
            let path = std::path::Path::new(directory);
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(directory);

            term.write_line(&format!("{}. {}", i + 1, dir_name))?;
            term.write_line(&format!("   Path: {}", directory))?;

            if path.exists() {
                if let Ok(metadata) = std::fs::metadata(path)
                    && let Ok(modified) = metadata.modified()
                        && let Ok(unix_time) = modified.duration_since(std::time::UNIX_EPOCH) {
                            term.write_line(&format!(
                                "   Last modified: {}",
                                chrono::DateTime::from_timestamp(unix_time.as_secs() as i64, 0)
                                    .unwrap_or_default()
                                    .format("%Y-%m-%d %H:%M:%S")
                            ))?;
                        }
                term.write_line("   Status: ✅ Exists")?;
            } else {
                term.write_line("   Status: ❌ Not found")?;
            }
            term.write_line("")?;
        }

        Ok(0)
    }
}

pub async fn handle_sync_command(
    command: &str,
    directories: Option<Vec<String>>,
) -> SyncResult<i32> {
    let mut sync_cmd = SyncCommand::new()?;

    match command.to_lowercase().as_str() {
        "push" => sync_cmd.execute_push(directories).await,
        "pull" => sync_cmd.execute_pull(directories).await,
        "status" => sync_cmd.execute_status().await,
        "reset" => sync_cmd.execute_reset(),
        "list" => sync_cmd.execute_list_directories(),
        _ => {
            eprintln!("Unknown sync command: {}", command);
            eprintln!("Available commands: push, pull, status, reset, list");
            Ok(1)
        }
    }
}
