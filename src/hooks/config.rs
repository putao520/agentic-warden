//! Claude Code hooks configuration management.
//!
//! Automatically installs and uninstalls hooks to Claude Code's configuration
//! when agentic-warden starts/stops.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Claude Code hook configuration for a single event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Command to execute when hook is triggered.
    pub command: String,
    /// Whether to pass data via stdin.
    #[serde(default)]
    pub stdin: bool,
}

/// Claude Code hooks configuration file format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeHooksConfig {
    #[serde(flatten)]
    pub hooks: HashMap<String, HookConfig>,
}

impl ClaudeHooksConfig {
    /// Load hooks configuration from Claude Code config directory.
    pub fn load() -> Result<Self> {
        let path = Self::get_config_path()?;

        if !path.exists() {
            // Config file doesn't exist, return empty config
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read Claude hooks config: {:?}", path))?;

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse Claude hooks config: {:?}", path))
    }

    /// Save hooks configuration to Claude Code config directory.
    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize hooks config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write Claude hooks config: {:?}", path))?;

        Ok(())
    }

    /// Get the path to Claude Code's hooks configuration file.
    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("Failed to get config directory"))?
            .join("claude");

        Ok(config_dir.join("hooks.json"))
    }

    /// Check if agentic-warden hooks are already installed.
    pub fn is_installed(&self) -> bool {
        self.hooks.get("SessionEnd")
            .map(|h| h.command.contains("agentic-warden"))
            .unwrap_or(false)
            || self.hooks.get("PreCompact")
                .map(|h| h.command.contains("agentic-warden"))
                .unwrap_or(false)
    }

    /// Install agentic-warden hooks (SessionEnd and PreCompact).
    pub fn install_hooks(&mut self) -> Result<()> {
        let hook_config = HookConfig {
            command: "agentic-warden hooks handle".to_string(),
            stdin: true,
        };

        // Install SessionEnd hook
        self.hooks.insert("SessionEnd".to_string(), hook_config.clone());

        // Install PreCompact hook
        self.hooks.insert("PreCompact".to_string(), hook_config);

        Ok(())
    }

    /// Uninstall agentic-warden hooks.
    pub fn uninstall_hooks(&mut self) -> Result<()> {
        // Only remove if it's our hook
        if let Some(hook) = self.hooks.get("SessionEnd") {
            if hook.command.contains("agentic-warden") {
                self.hooks.remove("SessionEnd");
            }
        }

        if let Some(hook) = self.hooks.get("PreCompact") {
            if hook.command.contains("agentic-warden") {
                self.hooks.remove("PreCompact");
            }
        }

        Ok(())
    }
}

/// Install hooks to Claude Code configuration.
///
/// This is idempotent - if hooks are already installed, it does nothing.
pub fn install_hooks() -> Result<()> {
    let mut config = ClaudeHooksConfig::load()
        .context("Failed to load Claude Code hooks config")?;

    if config.is_installed() {
        // Already installed, nothing to do
        return Ok(());
    }

    config.install_hooks()
        .context("Failed to install hooks to config")?;

    config.save()
        .context("Failed to save Claude Code hooks config")?;

    eprintln!("✅ Installed agentic-warden hooks to Claude Code");
    Ok(())
}

/// Uninstall hooks from Claude Code configuration.
///
/// This is idempotent - if hooks are not installed, it does nothing.
pub fn uninstall_hooks() -> Result<()> {
    let mut config = ClaudeHooksConfig::load()
        .context("Failed to load Claude Code hooks config")?;

    if !config.is_installed() {
        // Not installed, nothing to do
        return Ok(());
    }

    config.uninstall_hooks()
        .context("Failed to uninstall hooks from config")?;

    config.save()
        .context("Failed to save Claude Code hooks config")?;

    eprintln!("✅ Uninstalled agentic-warden hooks from Claude Code");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_config_serialization() {
        let config = HookConfig {
            command: "agentic-warden hooks handle".to_string(),
            stdin: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("agentic-warden"));
        assert!(json.contains("\"stdin\":true"));
    }

    #[test]
    fn test_hooks_config_install() {
        let mut config = ClaudeHooksConfig::default();
        assert!(!config.is_installed());

        config.install_hooks().unwrap();
        assert!(config.is_installed());

        assert!(config.hooks.contains_key("SessionEnd"));
        assert!(config.hooks.contains_key("PreCompact"));
    }

    #[test]
    fn test_hooks_config_uninstall() {
        let mut config = ClaudeHooksConfig::default();
        config.install_hooks().unwrap();
        assert!(config.is_installed());

        config.uninstall_hooks().unwrap();
        assert!(!config.is_installed());
    }
}
