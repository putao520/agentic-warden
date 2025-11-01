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
                    term.write_line(
                        "5. Add urn:ietf:wg:oauth:2.0:oob to authorized redirect URIs",
                    )?;
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
        use crate::process_tree::get_process_name;
        use crate::registry::TaskRegistry;
        use std::collections::HashMap;
        use std::time::Duration;
        use tokio::time::interval;

        let term = Term::stdout();

        // Clear screen for real-time display
        term.clear_screen()?;

        // Connect to task registry
        let _registry = match TaskRegistry::connect() {
            Ok(reg) => reg,
            Err(e) => {
                term.write_line("❌ Failed to connect to task registry")?;
                term.write_line(&format!("Error: {}", e))?;
                term.write_line("")?;
                term.write_line("💡 Make sure agentic-warden is running to see task status")?;
                term.write_line("")?;
                term.write_line("Press Ctrl+C to exit")?;
                return Ok(1);
            }
        };

        term.write_line("🎯 Real-time Task Monitor (Press Ctrl+C to exit)")?;
        term.write_line("=".repeat(60).as_str())?;
        term.write_line("")?;

        // Set up real-time updates every 2 seconds
        let mut update_interval = interval(Duration::from_secs(2));
        let mut last_task_count = 0;

        loop {
            // Clear from cursor down for next update
            term.clear_to_end_of_screen()?;

            // Move cursor back to line 4 for display (after header)
            print!("\x1b[4;0H");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();

            // Use global task discovery to see tasks from all agentic-warden instances
            match TaskRegistry::get_all_global_tasks() {
                Ok(entries) => {
                    // Check if task count changed
                    let current_count = entries.len();
                    let count_changed = current_count != last_task_count;
                    last_task_count = current_count;

                    if entries.is_empty() {
                        term.write_line("📋 No active tasks found")?;
                        term.write_line("")?;
                        term.write_line("💡 Start a task with:")?;
                        term.write_line("   agentic-warden claude \"your task description\"")?;
                        term.write_line("   agentic-warden all \"review this code\"")?;
                        term.write_line("")?;
                        term.write_line("🔄 Next update: 2s")?;
                    } else {
                        term.write_line(&format!("📊 Total Active Tasks: {}", current_count))?;
                        if count_changed {
                            term.write_line("🆕 Task list updated!")?;
                        }
                        term.write_line("")?;

                        // Group tasks by root parent process
                        let mut grouped_tasks: HashMap<u32, Vec<_>> = HashMap::new();
                        let mut process_names: HashMap<u32, String> = HashMap::new();

                        for entry in &entries {
                            let root_pid = entry.task.root_parent_pid.unwrap_or(entry.task_id);
                            grouped_tasks
                                .entry(root_pid)
                                .or_default()
                                .push(entry.clone());

                            // Get process name for root parent with AI CLI type detection
                            process_names.entry(root_pid).or_insert_with(|| {
                                if let Some(mut process_name) = get_process_name(root_pid) {
                                    // Check if this is a Node.js process running an AI CLI
                                    if process_name.to_lowercase().contains("node")
                                        && let Some(ai_type) =
                                            crate::process_tree::detect_npm_ai_cli_type(
                                                &process_name,
                                            )
                                    {
                                        process_name = format!("{} ({})", process_name, ai_type);
                                    }
                                    process_name
                                } else {
                                    format!("Process-{}", root_pid)
                                }
                            });
                        }

                        // Display tasks by root parent process
                        for (root_pid, tasks) in grouped_tasks {
                            let process_name = process_names
                                .get(&root_pid)
                                .cloned()
                                .unwrap_or_else(|| format!("Process-{}", root_pid));
                            term.write_line(&format!(
                                "🖥️  {} (PID: {}) - {} tasks",
                                process_name,
                                root_pid,
                                tasks.len()
                            ))?;
                            term.write_line("-".repeat(50).as_str())?;

                            for task in tasks {
                                let status_icon = match task.task.status {
                                    crate::task_record::TaskStatus::Running => "🟢",
                                    crate::task_record::TaskStatus::CompletedButUnread => "✅",
                                };

                                term.write_line(&format!(
                                    "  {} Task PID: {} | Started: {}",
                                    status_icon,
                                    task.task_id,
                                    task.task.started_at.format("%H:%M:%S")
                                ))?;

                                // Show log file location (shortened)
                                let log_path = &task.task.log_path;
                                if log_path.len() > 60 {
                                    let short_path =
                                        format!("...{}", &log_path[log_path.len() - 57..]);
                                    term.write_line(&format!("     📁 Log: {}", short_path))?;
                                } else {
                                    term.write_line(&format!("     📁 Log: {}", log_path))?;
                                }

                                // Show additional info for running tasks
                                if matches!(
                                    task.task.status,
                                    crate::task_record::TaskStatus::Running
                                ) {
                                    if let Some(manager_pid) = task.task.manager_pid {
                                        term.write_line(&format!(
                                            "     👨‍💼 Manager PID: {}",
                                            manager_pid
                                        ))?;
                                    }

                                    // Show running duration
                                    let duration = chrono::Utc::now() - task.task.started_at;
                                    let total_minutes = duration.num_seconds() / 60;
                                    if total_minutes < 60 {
                                        term.write_line(&format!(
                                            "     ⏱️  Running for {}m",
                                            total_minutes
                                        ))?;
                                    } else {
                                        term.write_line(&format!(
                                            "     ⏱️  Running for {}h {}m",
                                            (total_minutes / 60),
                                            total_minutes % 60
                                        ))?;
                                    }

                                    // Show process tree depth if available
                                    if task.task.process_tree_depth > 0 {
                                        term.write_line(&format!(
                                            "     🌳 Tree Depth: {}",
                                            task.task.process_tree_depth
                                        ))?;
                                    }
                                }

                                // Show completion info for completed tasks
                                if matches!(
                                    task.task.status,
                                    crate::task_record::TaskStatus::CompletedButUnread
                                ) {
                                    if let Some(completed_at) = task.task.completed_at {
                                        term.write_line(&format!(
                                            "     ✅ Completed: {}",
                                            completed_at.format("%H:%M:%S")
                                        ))?;
                                    }
                                    if let Some(exit_code) = task.task.exit_code {
                                        let status =
                                            if exit_code == 0 { "Success" } else { "Failed" };
                                        term.write_line(&format!(
                                            "     🎯 Status: {} (exit code: {})",
                                            status, exit_code
                                        ))?;
                                    }
                                }

                                term.write_line("")?;
                            }
                        }

                        // Show summary
                        let running_count = entries
                            .iter()
                            .filter(|e| {
                                matches!(e.task.status, crate::task_record::TaskStatus::Running)
                            })
                            .count();

                        let completed_count = entries
                            .iter()
                            .filter(|e| {
                                matches!(
                                    e.task.status,
                                    crate::task_record::TaskStatus::CompletedButUnread
                                )
                            })
                            .count();

                        term.write_line("📈 Summary:")?;
                        term.write_line(&format!("   🟢 Running: {}", running_count))?;
                        term.write_line(&format!("   ✅ Completed: {}", completed_count))?;
                        term.write_line(&format!(
                            "   ❌ Other: {}",
                            entries.len() - running_count - completed_count
                        ))?;
                        term.write_line("")?;
                        term.write_line("💡 Use 'agentic-warden wait' to monitor task completion")?;
                    }

                    term.write_line("")?;
                    term.write_line("🔄 Next update: 2s | Press Ctrl+C to exit")?;
                }
                Err(e) => {
                    term.write_line("❌ Failed to retrieve task status")?;
                    term.write_line(&format!("Error: {}", e))?;
                    term.write_line("")?;
                    term.write_line("🔄 Retrying in 2s...")?;
                }
            }

            // Wait for next update interval
            tokio::select! {
                _ = update_interval.tick() => {
                    // Continue loop for next update
                }
                _ = tokio::signal::ctrl_c() => {
                    term.write_line("\n\n👋 Exiting task monitor...")?;
                    break;
                }
            }
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
                    && let Ok(unix_time) = modified.duration_since(std::time::UNIX_EPOCH)
                {
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
            eprintln!("Available commands:");
            eprintln!("  push      - Push configuration to Google Drive");
            eprintln!("  pull      - Pull configuration from Google Drive");
            eprintln!("  status    - Show real-time task status");
            eprintln!("  reset     - Reset sync state");
            eprintln!("  list      - List monitored directories");
            Ok(1)
        }
    }
}
