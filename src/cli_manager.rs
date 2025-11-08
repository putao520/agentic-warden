//! CLI Tool Detection Module
//!
//! This module provides CLI tool detection and metadata for AI CLI tools
//! (Claude, Codex, Gemini). It focuses on core functionality without
//! interactive UI components.

#![allow(dead_code)] // CLI管理模块，部分功能当前未使用

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// CLI Tool information
#[derive(Debug, Clone)]
pub struct CliTool {
    pub name: String,
    pub command: String,
    pub npm_package: String,
    pub description: String,
    pub installed: bool,
    pub version: Option<String>,
    pub install_type: Option<InstallType>,
    pub install_path: Option<PathBuf>,
}

/// Installation type
#[derive(Debug, Clone, PartialEq)]
pub enum InstallType {
    Native, // Native executable
    Npm,    // NPM package
    #[allow(dead_code)]
    Unknown, // Unknown installation type
}

/// Native installation information
#[derive(Debug, Clone)]
pub struct NativeInstallInfo {
    pub version: Option<String>,
    pub path: PathBuf,
}

/// NPM installation information
#[derive(Debug, Clone)]
pub struct NpmInstallInfo {
    pub version: Option<String>,
    pub path: PathBuf,
}

/// CLI Tool Detector for non-interactive operations
#[allow(dead_code)]
pub struct CliToolDetector {
    tools: Vec<CliTool>,
}

impl Default for CliToolDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl CliToolDetector {
    /// Create a new CLI tool detector
    pub fn new() -> Self {
        let mut detector = Self { tools: Vec::new() };
        detector.initialize_tools();
        detector
    }

    /// Initialize known AI CLI tools
    fn initialize_tools(&mut self) {
        self.tools = vec![
            CliTool {
                name: "Claude CLI".to_string(),
                command: "claude".to_string(),
                npm_package: "@anthropic-ai/claude-cli".to_string(),
                description: "Anthropic Claude CLI tool".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
            CliTool {
                name: "Codex CLI".to_string(),
                command: "codex".to_string(),
                npm_package: "@openai/codex-cli".to_string(),
                description: "OpenAI Codex CLI tool".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
            CliTool {
                name: "Gemini CLI".to_string(),
                command: "gemini".to_string(),
                npm_package: "@google-ai/gemini-cli".to_string(),
                description: "Google Gemini CLI tool".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
        ];
    }

    /// Detect all CLI tools status
    pub fn detect_all_tools(&mut self) -> Result<()> {
        for tool in &mut self.tools {
            Self::detect_tool_installation_static(tool);
        }
        Ok(())
    }

    /// Get all tools
    pub fn get_tools(&self) -> &[CliTool] {
        &self.tools
    }

    /// Get installed tools only
    pub fn get_installed_tools(&self) -> Vec<&CliTool> {
        self.tools.iter().filter(|tool| tool.installed).collect()
    }

    /// Get uninstalled tools only
    pub fn get_uninstalled_tools(&self) -> Vec<&CliTool> {
        self.tools.iter().filter(|tool| !tool.installed).collect()
    }

    /// Get tool by command name
    pub fn get_tool_by_command(&self, command: &str) -> Option<&CliTool> {
        self.tools.iter().find(|tool| tool.command == command)
    }

    /// Detect tool installation status
    fn detect_tool_installation_static(tool: &mut CliTool) {
        // Check if command is available
        if let Ok(path) = which::which(&tool.command) {
            tool.installed = true;
            tool.install_path = Some(path.clone());

            // Try to detect installation type
            tool.install_type = Self::detect_install_type_static(&path);

            // Try to get version
            tool.version = Self::get_tool_version_static(&tool.command);
        } else {
            tool.installed = false;
            tool.install_path = None;
            tool.install_type = None;
            tool.version = None;
        }
    }

    /// Detect installation type from path
    fn detect_install_type_static(path: &PathBuf) -> Option<InstallType> {
        if let Some(path_str) = path.to_str() {
            if path_str.contains("node_modules") || path_str.contains("npm") {
                return Some(InstallType::Npm);
            }
        }
        Some(InstallType::Native)
    }

    /// Get tool version
    fn get_tool_version_static(command: &str) -> Option<String> {
        Command::new(command)
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Check if Node.js is available
    pub fn detect_nodejs(&self) -> Option<String> {
        Command::new("node")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Check if npm is available
    pub fn detect_npm(&self) -> bool {
        Command::new("npm")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get installation hint for a tool
    pub fn get_install_hint(&self, command: &str) -> String {
        const INSTALL_HINTS: &[(&str, &str)] = &[
            ("codex", "npm install -g @openai/codex-cli"),
            ("claude", "npm install -g @anthropic-ai/claude-cli"),
            ("gemini", "npm install -g @google-ai/gemini-cli"),
        ];

        INSTALL_HINTS
            .iter()
            .find(|(cmd, _)| cmd.eq_ignore_ascii_case(command))
            .map(|(_, hint)| hint.to_string())
            .unwrap_or_else(|| format!("Install {} via appropriate package manager", command))
    }

    /// Check for updates (non-interactive version check)
    pub async fn check_for_updates(&self) -> Result<Vec<(String, Option<String>, Option<String>)>> {
        let mut updates = Vec::new();

        for tool in &self.tools {
            if !tool.installed {
                continue;
            }

            let current_version = tool.version.clone();
            let latest_version = Self::check_latest_version(&tool.npm_package).await;

            updates.push((tool.name.clone(), current_version, latest_version));
        }

        Ok(updates)
    }

    /// Check latest version from npm (simplified version)
    async fn check_latest_version(_package: &str) -> Option<String> {
        // This is a simplified implementation
        // In a real scenario, you might want to query npm registry API
        // For now, return None to indicate version check is not implemented
        None
    }

    /// Get OS type for tool recommendations
    pub fn get_os_type() -> &'static str {
        #[cfg(target_os = "windows")]
        return "Windows";
        #[cfg(target_os = "macos")]
        return "macOS";
        #[cfg(target_os = "linux")]
        return "Linux";
        #[cfg(target_os = "freebsd")]
        return "FreeBSD";
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "freebsd"
        )))]
        return "Unknown";
    }
}

/// Convenience function to create a detector and detect all tools
pub async fn detect_ai_cli_tools() -> Result<Vec<CliTool>> {
    let mut detector = CliToolDetector::new();
    detector.detect_all_tools()?;
    Ok(detector.tools)
}

/// Get installation commands for all uninstalled tools
pub fn get_install_commands() -> Vec<(String, String)> {
    let detector = CliToolDetector::new();
    detector
        .get_uninstalled_tools()
        .into_iter()
        .map(|tool| (tool.name.clone(), detector.get_install_hint(&tool.command)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_tool_detector_creation() {
        let detector = CliToolDetector::new();
        assert_eq!(detector.tools.len(), 3);
    }

    #[test]
    fn test_get_install_commands() {
        let commands = get_install_commands();
        // Should return commands for tools that aren't installed
        assert!(!commands.is_empty());
    }

    #[test]
    fn test_os_type_detection() {
        let os_type = CliToolDetector::get_os_type();
        assert!(!os_type.is_empty());
        assert_ne!(os_type, "Unknown");
    }

    #[test]
    fn test_get_tool_by_command() {
        let detector = CliToolDetector::new();
        let claude_tool = detector.get_tool_by_command("claude");
        assert!(claude_tool.is_some());
        assert_eq!(claude_tool.unwrap().command, "claude");

        let nonexistent = detector.get_tool_by_command("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_get_install_hint() {
        let detector = CliToolDetector::new();
        let hint = detector.get_install_hint("claude");
        assert!(hint.contains("npm install"));

        let unknown_hint = detector.get_install_hint("unknown");
        assert!(unknown_hint.contains("Install"));
    }
}
