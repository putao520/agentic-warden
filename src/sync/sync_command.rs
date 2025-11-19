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
        "list" => sync_cmd.execute_list().await,
        "status" => sync_cmd.execute_status().await,
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

    /// Get paths to known AI CLI configuration directories
    fn get_ai_cli_dirs() -> SyncResult<Vec<(String, std::path::PathBuf)>> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::sync_config("Could not find home directory".to_string()))?;

        Ok(vec![
            ("Claude".to_string(), home_dir.join(".claude")),
            ("Codex".to_string(), home_dir.join(".codex")),
            ("Gemini".to_string(), home_dir.join(".gemini")),
        ])
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

        // Check for AI CLI configurations using helper
        let cli_dirs = Self::get_ai_cli_dirs()?;
        let found_configs: Vec<_> = cli_dirs
            .iter()
            .filter(|(_, path)| path.exists())
            .collect();

        if found_configs.is_empty() {
            term.write_line("ℹ️  No AI CLI configurations found.")?;
            term.write_line("")?;
            term.write_line("Expected directories:")?;
            for (name, path) in &cli_dirs {
                term.write_line(&format!("  - {} at {}", name, path.display()))?;
            }
            term.write_line("")?;
            term.write_line("Please install at least one AI CLI tool and try again.")?;
            return Ok(1);
        }

        term.write_line("🔍 Scanning for AI CLI configurations...")?;
        for (name, path) in &found_configs {
            term.write_line(&format!(
                "  ✓ Found {} configuration at {}",
                name,
                path.display()
            ))?;
        }
        term.write_line("")?;

        term.write_line("🔐 Authenticating with Google Drive...")?;
        if let Err(e) = self.manager.authenticate_google_drive().await {
            if let AgenticWardenError::Auth {
                message, provider, ..
            } = &e
            {
                if provider == "google_drive" && message.contains("GOOGLE_CLIENT_ID") {
                    term.write_line("🚫 Google Drive authentication failed:")?;
                    term.write_line(&format!("   {}", message))?;
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
                if provider == "google_drive" && message.contains("GOOGLE_CLIENT_ID") {
                    term.write_line("🚫 Google Drive authentication failed:")?;
                    term.write_line(&format!("   {}", message))?;
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

    /// List available configurations
    pub async fn execute_list(&mut self) -> SyncResult<i32> {
        let term = Term::stdout();

        term.write_line("馃搵 Available configurations:")?;
        term.write_line("")?;

        // Authenticate with Google Drive
        if let Err(_) = self.manager.authenticate_google_drive().await {
            term.write_line("鉂?Not authenticated with Google Drive.")?;
            term.write_line("Please run 'agentic-warden push <config>' to authenticate.")?;
            return Ok(1);
        }

        // List configurations
        let configs = self.manager.list_available_configs().await?;

        if configs.is_empty() {
            term.write_line("  (no configurations found)")?;
        } else {
            for config in configs {
                term.write_line(&format!("  - {}", config))?;
            }
        }

        term.write_line("")?;
        Ok(0)
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

        // Check local configurations using helper
        let cli_dirs = Self::get_ai_cli_dirs()?;

        term.write_line("Local Configurations:")?;
        for (name, path) in &cli_dirs {
            term.write_line(&format!(
                "  {}: {}",
                name,
                if path.exists() {
                    "鉁?Present"
                } else {
                    "鉂?Not found"
                }
            ))?;
        }

        term.write_line("")?;
        Ok(0)
    }
}
