use super::config_sync_manager::ConfigSyncManager;
use super::error::{SyncError, SyncResult};
use crate::error::AgenticWardenError;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};

/// Handle sync commands
pub async fn handle_sync_command(command: &str, config_name: Option<String>) -> SyncResult<i32> {
    let mut sync_cmd = SyncCommand::new()?;

    match command {
        "push" => sync_cmd.execute_push(config_name).await,
        "pull" => sync_cmd.execute_pull(config_name).await,
        "status" => sync_cmd.execute_status().await,
        "reset" => {
            // Reset sync state
            eprintln!("Reset command not yet implemented");
            Ok(0)
        }
        _ => Err(SyncError::sync_config(format!(
            "Unknown sync command: {}",
            command
        ))),
    }
}

pub struct SyncCommand {
    manager: ConfigSyncManager,
}

impl SyncCommand {
    pub fn new() -> SyncResult<Self> {
        Ok(Self {
            manager: ConfigSyncManager::new()?,
        })
    }

    /// Execute push command with a configuration name
    pub async fn execute_push(&mut self, config_name: Option<String>) -> SyncResult<i32> {
        let term = Term::stdout();

        let config_name = match config_name {
            Some(name) => name,
            None => "default".to_string(),
        };

        term.write_line("🚀 Starting configuration sync push...")?;
        term.write_line(&format!("📦 Configuration name: '{}'", config_name))?;
        term.write_line("")?;

        let home_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::sync_config("Could not find home directory".to_string()))?;

        let claude_dir = home_dir.join(".claude");
        let codex_dir = home_dir.join(".codex");
        let gemini_dir = home_dir.join(".gemini");

        let claude_exists = claude_dir.exists();
        let codex_exists = codex_dir.exists();
        let gemini_exists = gemini_dir.exists();

        if !claude_exists && !codex_exists && !gemini_exists {
            term.write_line("ℹ️  No AI CLI configurations found.")?;
            term.write_line("")?;
            term.write_line("Expected directories:")?;
            term.write_line(&format!("  - {}", claude_dir.display()))?;
            term.write_line(&format!("  - {}", codex_dir.display()))?;
            term.write_line(&format!("  - {}", gemini_dir.display()))?;
            term.write_line("")?;
            term.write_line("Please install at least one AI CLI tool and try again.")?;
            return Ok(1);
        }

        term.write_line("🔍 Scanning for AI CLI configurations...")?;
        if claude_exists {
            term.write_line(&format!(
                "  ✓ Found Claude configuration at {}",
                claude_dir.display()
            ))?;
        }
        if codex_exists {
            term.write_line(&format!(
                "  ✓ Found Codex configuration at {}",
                codex_dir.display()
            ))?;
        }
        if gemini_exists {
            term.write_line(&format!(
                "  ✓ Found Gemini configuration at {}",
                gemini_dir.display()
            ))?;
        }
        term.write_line("")?;

        term.write_line("🔐 Authenticating with Google Drive...")?;
        if let Err(e) = self.manager.authenticate_google_drive().await {
            if let AgenticWardenError::Auth {
                message, provider, ..
            } = &e
            {
                if provider == "google_drive" {
                    term.write_line("🚫 Google Drive authentication failed:")?;
                    term.write_line(&format!("   {}", message))?;
                    term.write_line("")?;
                    term.write_line("This app uses built-in OAuth credentials.")?;
                    term.write_line("Please ensure you have a Google account and try again.")?;
                    term.write_line("")?;
                    term.write_line("The error might be temporary. Please try again later.")?;
                    return Ok(1);
                }
            }
            return Err(e);
        }
        term.write_line("✅ Authentication successful!")?;
        term.write_line("")?;

        term.write_line("🔍 Checking for existing configuration...")?;
        let existing_config = self.manager.verify_named_config(&config_name).await?;
        if existing_config {
            term.write_line(&format!(
                "⚠️  Configuration '{}' already exists in Google Drive.",
                config_name
            ))?;
            term.write_line("")?;
            term.write_line("Do you want to overwrite it?")?;
            term.write_line("  [Y] Yes, overwrite")?;
            term.write_line("  [N] No, cancel")?;
            term.write_line("")?;

            use std::io::{self, Write};
            let mut input = String::new();
            loop {
                term.write_str("Your choice [Y/N]: ")?;
                io::stdout().flush()?;
                io::stdin().read_line(&mut input)?;
                match input.trim().to_lowercase().as_str() {
                    "y" | "yes" => {
                        term.write_line("✅ Proceeding with overwrite...")?;
                        term.write_line("")?;
                        break;
                    }
                    "n" | "no" => {
                        term.write_line("🚫 Upload cancelled.")?;
                        return Ok(0);
                    }
                    _ => {
                        term.write_line("Please enter Y or N.")?;
                        input.clear();
                    }
                }
            }
        } else {
            term.write_line("✅ No existing configuration found.")?;
            term.write_line("")?;
        }

        let progress = ProgressBar::new(3);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        progress.set_message("Packing configuration");
        let archive_size = self.manager.pack_named_config(&config_name).await?;
        progress.inc(1);

        progress.set_message("Uploading to Google Drive");
        let uploaded = self.manager.upload_named_config(&config_name).await?;
        progress.inc(1);

        progress.set_message("Verifying upload");
        let verified = self.manager.verify_named_config(&config_name).await?;
        progress.inc(1);

        progress.finish_with_message("Sync complete");
        term.write_line("")?;

        term.write_line("📊 Sync Summary:")?;
        term.write_line(&format!("   Configuration: {}", config_name))?;
        term.write_line(&format!("   Archive size: {} bytes", archive_size))?;
        term.write_line(&format!(
            "   Upload status: {}",
            if uploaded { "Success" } else { "Failed" }
        ))?;
        term.write_line(&format!(
            "   Verification: {}",
            if verified { "Passed" } else { "Failed" }
        ))?;
        term.write_line("")?;

        if uploaded && verified {
            term.write_line(&format!(
                "🎉 Configuration '{}' successfully synced to Google Drive!",
                config_name
            ))?;
            Ok(0)
        } else {
            term.write_line("⚠️  Sync completed with warnings.")?;
            Ok(1)
        }
    }

    /// Execute pull command with a configuration name
    pub async fn execute_pull(&mut self, config_name: Option<String>) -> SyncResult<i32> {
        let term = Term::stdout();

        let config_name = match config_name {
            Some(name) => name,
            None => "default".to_string(),
        };

        term.write_line("🚀 Starting configuration sync pull...")?;
        term.write_line(&format!("📦 Configuration name: '{}'", config_name))?;
        term.write_line("")?;

        term.write_line("🔐 Authenticating with Google Drive...")?;
        if let Err(e) = self.manager.authenticate_google_drive().await {
            if let AgenticWardenError::Auth {
                message, provider, ..
            } = &e
            {
                if provider == "google_drive" {
                    term.write_line("🚫 Google Drive authentication failed:")?;
                    term.write_line(&format!("   {}", message))?;
                    term.write_line("")?;
                    term.write_line("This app uses built-in OAuth credentials.")?;
                    term.write_line("Please ensure you have a Google account and try again.")?;
                    term.write_line("")?;
                    term.write_line("The error might be temporary. Please try again later.")?;
                    return Ok(1);
                }
            }
            return Err(e);
        }
        term.write_line("✅ Authentication successful!")?;
        term.write_line("")?;

        let progress = ProgressBar::new(3);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        progress.set_message("Downloading from Google Drive");
        let downloaded = self.manager.download_named_config(&config_name).await?;
        progress.inc(1);

        if !downloaded {
            progress.finish_with_message("No configuration found");
            term.write_line("")?;
            term.write_line(&format!(
                "ℹ️  No configuration named '{}' found in Google Drive.",
                config_name
            ))?;
            term.write_line("Available configurations:")?;

            let configs = self.manager.list_available_configs().await?;
            if configs.is_empty() {
                term.write_line("  (none)")?;
            } else {
                for config in configs {
                    term.write_line(&format!("  - {}", config))?;
                }
            }

            term.write_line("")?;
            return Ok(1);
        }

        progress.set_message("Extracting configuration");
        let extracted = self.manager.extract_named_config(&config_name).await?;
        progress.inc(1);

        progress.set_message("Verifying extraction");
        let verified = self.manager.verify_extraction(&config_name).await?;
        progress.inc(1);

        progress.finish_with_message("Pull complete");
        term.write_line("")?;

        term.write_line("📊 Pull Summary:")?;
        term.write_line(&format!("   Configuration: {}", config_name))?;
        term.write_line(&format!(
            "   Extracted: {}",
            if extracted { "Success" } else { "Failed" }
        ))?;
        term.write_line(&format!(
            "   Verified: {}",
            if verified { "Success" } else { "Failed" }
        ))?;
        term.write_line("")?;

        if extracted && verified {
            term.write_line(&format!(
                "🎉 Configuration '{}' successfully pulled from Google Drive!",
                config_name
            ))?;
            Ok(0)
        } else {
            term.write_line("⚠️  Pull completed with warnings.")?;
            Ok(1)
        }
    }

    /// Show sync status
    pub async fn execute_status(&mut self) -> SyncResult<i32> {
        let term = Term::stdout();

        term.write_line("馃搳 Sync Status:")?;
        term.write_line("")?;

        // Check authentication status
        match self.manager.check_google_drive_auth().await {
            Ok(authenticated) => {
                if authenticated {
                    term.write_line("  Google Drive: 鉁?Connected")?;
                } else {
                    term.write_line("  Google Drive: 鉂?Not authenticated")?;
                }
            }
            Err(_) => {
                term.write_line("  Google Drive: 鉂?Unknown (check failed)")?;
            }
        }

        term.write_line("")?;

        // Check local configurations
        let home_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::sync_config("Could not find home directory".to_string()))?;

        let claude_dir = home_dir.join(".claude");
        let codex_dir = home_dir.join(".codex");
        let gemini_dir = home_dir.join(".gemini");

        term.write_line("Local Configurations:")?;
        term.write_line(&format!(
            "  Claude: {}",
            if claude_dir.exists() {
                "鉁?Present"
            } else {
                "鉂?Not found"
            }
        ))?;
        term.write_line(&format!(
            "  Codex: {}",
            if codex_dir.exists() {
                "鉁?Present"
            } else {
                "鉂?Not found"
            }
        ))?;
        term.write_line(&format!(
            "  Gemini: {}",
            if gemini_dir.exists() {
                "鉁?Present"
            } else {
                "鉂?Not found"
            }
        ))?;

        term.write_line("")?;
        Ok(0)
    }
}
