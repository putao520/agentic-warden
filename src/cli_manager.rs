//! CLI Manager Module
//!
//! This module provides comprehensive CLI tool management including detection,
//! installation, and updates for AI CLI tools (Claude, Codex, Gemini).

use color_eyre::eyre::Result;
use console::{Term, style};
use dialoguer::{Confirm, Select};
use indicatif::{ProgressBar, ProgressStyle};
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

/// CLI Manager for handling CLI tool operations
pub struct CliManager {
    term: Term,
    tools: Vec<CliTool>,
}

impl CliManager {
    /// Create a new CLI manager
    pub fn new() -> Result<Self> {
        let mut manager = Self {
            term: Term::stdout(),
            tools: Vec::new(),
        };

        // Initialize supported CLI tools
        manager.initialize_tools();
        Ok(manager)
    }

    /// Initialize supported CLI tools
    fn initialize_tools(&mut self) {
        self.tools = vec![
            CliTool {
                name: "Claude CLI".to_string(),
                command: "claude".to_string(),
                npm_package: "@anthropic-ai/claude-cli".to_string(),
                description: "Anthropic Claude AI assistant CLI".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
            CliTool {
                name: "Codex CLI".to_string(),
                command: "codex".to_string(),
                npm_package: "@openai/codex-cli".to_string(),
                description: "OpenAI Codex code generation CLI".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
            CliTool {
                name: "Gemini CLI".to_string(),
                command: "gemini".to_string(),
                npm_package: "@google-ai/gemini-cli".to_string(),
                description: "Google Gemini AI assistant CLI".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
        ];
    }

    /// Detect Node.js installation
    pub fn detect_nodejs(&self) -> Option<String> {
        match Command::new("node").arg("--version").output() {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => None,
        }
    }

    /// Get current OS type
    fn get_os_type() -> &'static str {
        #[cfg(target_os = "windows")]
        return "Windows";

        #[cfg(target_os = "macos")]
        return "macOS";

        #[cfg(target_os = "linux")]
        return "Linux";

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return "Unknown";
    }

    /// Detect CLI tool installation
    fn detect_tool_installation(tool: &mut CliTool) {
        // Special handling for Claude CLI to detect both Native and NPM versions
        if tool.command == "claude" {
            Self::detect_claude_installation(tool);
            return;
        }

        // Standard detection for other tools
        match Command::new(&tool.command).arg("--version").output() {
            Ok(output) if output.status.success() => {
                tool.installed = true;
                tool.version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());

                // Determine installation type and path
                if let Ok(path) = Self::get_command_path(&tool.command) {
                    tool.install_path = Some(path);

                    // Check if it's an NPM installation
                    if let Some(path_str) = tool.install_path.as_ref().and_then(|p| p.to_str()) {
                        if path_str.contains("node_modules") || path_str.contains("npm") {
                            tool.install_type = Some(InstallType::Npm);
                        } else {
                            tool.install_type = Some(InstallType::Native);
                        }
                    }
                }
            }
            _ => {
                tool.installed = false;
                tool.version = None;
                tool.install_type = None;
                tool.install_path = None;
            }
        }
    }

    /// Detect Claude CLI installation (Native + NPM dual support)
    fn detect_claude_installation(tool: &mut CliTool) {
        let os = Self::get_os_type();

        // Check for Native installation first (priority)
        if let Some(native_info) = Self::detect_claude_native(tool, os) {
            tool.installed = true;
            tool.version = native_info.version;
            tool.install_type = Some(InstallType::Native);
            tool.install_path = Some(native_info.path);
            tool.description = format!("Claude CLI (Native) - {}", tool.description);
            return;
        }

        // Check for NPM installation
        if let Some(npm_info) = Self::detect_claude_npm(tool) {
            tool.installed = true;
            tool.version = npm_info.version;
            tool.install_type = Some(InstallType::Npm);
            tool.install_path = Some(npm_info.path);
            tool.description = format!("Claude CLI (NPM) - {}", tool.description);
            return;
        }

        // Not found
        tool.installed = false;
        tool.version = None;
        tool.install_type = None;
        tool.install_path = None;
    }

    /// Detect Claude CLI Native installation
    fn detect_claude_native(_tool: &CliTool, os: &str) -> Option<NativeInstallInfo> {
        let command_names = match os {
            "Windows" => vec!["claude.exe", "claude-cli.exe"],
            "macOS" => vec!["claude", "claude-cli"],
            "Linux" => vec!["claude", "claude-cli"],
            _ => vec!["claude", "claude-cli"],
        };

        for cmd in command_names {
            if let Ok(output) = Command::new(cmd).arg("--version").output()
                && output.status.success()
            {
                if let Ok(path) = Self::get_command_path(cmd) {
                    return Some(NativeInstallInfo {
                        version: Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
                        path,
                    });
                }
            }
        }

        None
    }

    /// Detect Claude CLI NPM installation
    fn detect_claude_npm(tool: &CliTool) -> Option<NpmInstallInfo> {
        if let Ok(output) = Command::new("npm")
            .args(["list", "-g", "--depth=0", &tool.npm_package])
            .output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Parse npm list output to get version and path
            for line in stdout.lines() {
                if line.contains(&tool.npm_package)
                    && let Ok(path) = Self::get_command_path(&tool.command)
                {
                    return Some(NpmInstallInfo {
                        version: Some(
                            line.split_whitespace()
                                .last()
                                .unwrap_or("unknown")
                                .to_string(),
                        ),
                        path,
                    });
                }
            }
        }

        None
    }

    /// Get the path of a command
    fn get_command_path(command: &str) -> Result<PathBuf> {
        let output = Command::new("where")
            .arg(command)
            .output()
            .or_else(|_| Command::new("which").arg(command).output())?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let path_str = stdout.lines().next().unwrap_or("").trim();

            if !path_str.is_empty() {
                return Ok(PathBuf::from(path_str));
            }
        }

        Err(color_eyre::eyre::eyre!("Command not found"))
    }

    /// Check if NPM is available
    fn check_npm_available(&self) -> bool {
        Command::new("npm")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Install CLI tool via NPM
    async fn install_via_npm(&self, tool: &CliTool, global: bool) -> Result<()> {
        println!("📦 Installing {} via NPM...", tool.name);

        let mut args = vec!["install"];
        if global {
            args.push("-g");
        }
        args.push(&tool.npm_package);

        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        progress.set_message("Installing...");
        progress.enable_steady_tick(std::time::Duration::from_millis(100));

        let output = Command::new("npm").args(&args).output()?;

        progress.finish();

        if output.status.success() {
            println!("✅ {} installed successfully!", tool.name);
            if !output.stderr.is_empty() {
                println!(
                    "ℹ️  Installation info: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        } else {
            println!("❌ Failed to install {}:", tool.name);
            println!("{}", String::from_utf8_lossy(&output.stderr));
            return Err(color_eyre::eyre::eyre!("Installation failed"));
        }

        Ok(())
    }

    /// Update CLI tool via NPM
    async fn update_via_npm(&self, tool: &CliTool) -> Result<()> {
        println!("🔄 Updating {} via NPM...", tool.name);

        let args = vec!["update", "-g", &tool.npm_package];

        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        progress.set_message("Updating...");
        progress.enable_steady_tick(std::time::Duration::from_millis(100));

        let output = Command::new("npm").args(&args).output()?;

        progress.finish();

        if output.status.success() {
            println!("✅ {} updated successfully!", tool.name);
        } else {
            println!("❌ Failed to update {}:", tool.name);
            println!("{}", String::from_utf8_lossy(&output.stderr));
            return Err(color_eyre::eyre::eyre!("Update failed"));
        }

        Ok(())
    }

    /// Install Claude CLI with intelligent Native/NPM selection
    async fn install_claude_cli(&mut self) -> Result<()> {
        println!("🚀 Installing Claude CLI...");

        let os = Self::get_os_type();
        println!("📱 Detected OS: {}", os);

        // Check if already installed
        let claude_tool_idx = self
            .tools
            .iter_mut()
            .position(|t| t.command == "claude")
            .expect("Claude CLI tool should exist");

        Self::detect_claude_installation(&mut self.tools[claude_tool_idx]);

        if self.tools[claude_tool_idx].installed {
            println!("✅ Claude CLI is already installed!");
            println!(
                "   Version: {}",
                self.tools[claude_tool_idx]
                    .version
                    .as_ref()
                    .unwrap_or(&"Unknown".to_string())
            );
            println!("   Type: {:?}", self.tools[claude_tool_idx].install_type);
            println!("   Path: {:?}", self.tools[claude_tool_idx].install_path);
            println!();
            println!("🔄 To update Claude CLI, use option '2. Check for Updates'");
            return Ok(());
        }

        // Try Native installation first
        println!("🥇 Attempting Native installation (preferred)...");
        let os_clone = os.to_string();
        match self.try_install_claude_native(&os_clone).await {
            Ok(_) => {
                println!("✅ Native installation successful!");

                // Refresh detection after installation
                Self::detect_claude_installation(&mut self.tools[claude_tool_idx]);
                if self.tools[claude_tool_idx].installed {
                    println!(
                        "   Version: {}",
                        self.tools[claude_tool_idx]
                            .version
                            .as_ref()
                            .unwrap_or(&"Unknown".to_string())
                    );
                    println!("   Type: {:?}", self.tools[claude_tool_idx].install_type);
                    println!("   Path: {:?}", self.tools[claude_tool_idx].install_path);
                }
                return Ok(());
            }
            Err(e) => {
                println!("⚠️  Native installation failed: {}", e);
                println!("📦 Falling back to NPM installation...");
            }
        }

        // Fallback to NPM installation
        if let Err(e) = self.install_claude_via_npm().await {
            println!("❌ Smart installation failed: {}", e);
            println!();
            println!("📋 Manual Installation Instructions:");
            println!("1. Visit: https://github.com/anthropics/claude-cli/releases");
            println!("2. Download the latest release for your OS");
            println!("3. Follow the installation instructions for your platform");
            println!();
            println!("📦 Alternative NPM installation:");
            println!("   npm install -g @anthropic-ai/claude-cli");
            return Err(e);
        }

        println!("✅ Claude CLI installation completed!");

        // Refresh detection
        Self::detect_claude_installation(&mut self.tools[claude_tool_idx]);

        if self.tools[claude_tool_idx].installed {
            println!(
                "   Version: {}",
                self.tools[claude_tool_idx]
                    .version
                    .as_ref()
                    .unwrap_or(&"Unknown".to_string())
            );
            println!("   Type: {:?}", self.tools[claude_tool_idx].install_type);
            println!("   Path: {:?}", self.tools[claude_tool_idx].install_path);
        }

        Ok(())
    }

    /// Try to install Claude CLI Native version
    async fn try_install_claude_native(&self, os: &str) -> Result<()> {
        println!("🪟  Native installation for {}...", os);

        // For now, provide instructions for manual native installation
        // In a future version, this could download and install automatically
        match os {
            "Windows" => {
                println!("🪟  Windows Native Installation");
                println!("💡 Manual steps required:");
                println!("   1. Download claude-cli.exe from GitHub releases");
                println!("   2. Save to a directory in your PATH");
                println!("   3. Run from command line: claude-cli.exe --version");
            }
            "macOS" => {
                println!("🍎  macOS Native Installation");
                if Command::new("brew").arg("--version").output().is_ok() {
                    println!("🍺  Installing via Homebrew...");
                    let output = Command::new("brew").arg("install").arg("claude").output()?;
                    if output.status.success() {
                        println!("✅ Homebrew installation successful!");
                        return Ok(());
                    } else {
                        println!("⚠️  Homebrew installation failed");
                        println!("   {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                println!("💡 Manual installation:");
                println!("   1. Download claude-cli.dmg from GitHub releases");
                println!("   2. Mount and drag to Applications");
                println!("   3. Add to PATH if needed");
            }
            "Linux" => {
                println!("🐧  Linux Native Installation");
                println!("💡 Manual installation:");
                println!("   1. Download AppImage or binary from GitHub releases");
                println!("   2. Make executable: chmod +x claude-cli.AppImage");
                println!("   3. Move to PATH: sudo mv claude-cli.AppImage /usr/local/bin/claude");
            }
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Unsupported OS for native installation"
                ));
            }
        }

        println!("❌ Native installation requires manual steps");
        println!("💡  Please follow the instructions above or use NPM installation");

        Err(color_eyre::eyre::eyre!(
            "Native installation requires manual intervention"
        ))
    }

    /// Install Claude CLI via NPM
    async fn install_claude_via_npm(&self) -> Result<()> {
        println!("📦 Installing Claude CLI via NPM...");

        let claude_tool = self
            .tools
            .iter()
            .find(|t| t.command == "claude")
            .expect("Claude CLI tool should exist");

        self.install_via_npm(claude_tool, true).await
    }

    /// Detect and setup all CLI tools
    async fn detect_and_setup_all_tools(&mut self) -> Result<()> {
        println!(
            "\n{}",
            style("🔍 Detecting and Setting Up CLI Tools").bold().blue()
        );
        println!("{}", "-".repeat(50));

        // Detect Node.js first
        match self.detect_nodejs() {
            Some(version) => {
                println!("✅ Node.js found: {}", version);
            }
            None => {
                println!("❌ Node.js not found");
                println!("💡 Node.js is required for NPM-based CLI tools");
                println!("📥 Please install Node.js from https://nodejs.org/");
                println!();

                if !Confirm::new()
                    .with_prompt("Continue without Node.js? (NPM tools won't be available)")
                    .default(false)
                    .interact()?
                {
                    return Ok(());
                }
            }
        }

        // Check NPM availability
        let npm_available = self.check_npm_available();
        if npm_available {
            println!("✅ NPM is available");
        } else {
            println!("❌ NPM not found");
        }

        println!();

        // Detect each tool
        for tool in &mut self.tools {
            Self::detect_tool_installation(tool);

            if tool.installed {
                let install_type_str = match tool.install_type {
                    Some(InstallType::Npm) => "NPM",
                    Some(InstallType::Native) => "Native",
                    Some(InstallType::Unknown) => "Unknown",
                    None => "Unknown",
                };

                println!(
                    "✅ {}: {} ({})",
                    tool.name,
                    tool.version
                        .as_ref()
                        .unwrap_or(&"Unknown version".to_string()),
                    install_type_str
                );
            } else {
                println!("❌ {}: Not installed", tool.name);
            }
        }

        println!();

        // Ask if user wants to install missing tools
        let missing_tools: Vec<&CliTool> = self.tools.iter().filter(|t| !t.installed).collect();

        if !missing_tools.is_empty() && npm_available {
            println!("📦 Found {} missing tools:", missing_tools.len());
            for tool in &missing_tools {
                println!("   - {}", tool.name);
            }
            println!();

            if Confirm::new()
                .with_prompt("Would you like to install the missing tools via NPM?")
                .default(true)
                .interact()?
            {
                for tool in &missing_tools {
                    if let Err(e) = self.install_via_npm(tool, true).await {
                        println!("⚠️  Failed to install {}: {}", tool.name, e);
                    }
                }
            }
        } else if !missing_tools.is_empty() && !npm_available {
            println!(
                "⚠️  {} tools are missing but NPM is not available.",
                missing_tools.len()
            );
            println!("💡 Please install NPM or install the tools manually.");
        }

        Ok(())
    }

    /// Check for tool updates
    async fn check_for_updates(&mut self) -> Result<()> {
        println!("\n{}", style("🔄 Checking for Updates").bold().blue());
        println!("{}", "-".repeat(30));

        if !self.check_npm_available() {
            println!("❌ NPM is not available. Cannot check for updates.");
            return Ok(());
        }

        let mut updatable_tools = Vec::new();

        for tool in &mut self.tools {
            if tool.installed && tool.install_type == Some(InstallType::Npm) {
                // Check if there's an update available
                let args = vec!["outdated", "-g", &tool.npm_package];

                match Command::new("npm").args(&args).output() {
                    Ok(output) => {
                        // npm outdated returns non-zero exit code if updates are available
                        if !output.status.success() {
                            updatable_tools.push(tool.clone());
                        }
                    }
                    _ => {
                        println!("⚠️  Could not check updates for {}", tool.name);
                    }
                }
            }
        }

        if updatable_tools.is_empty() {
            println!("✅ All NPM-installed tools are up to date!");
        } else {
            println!("📦 Updates available for:");
            for tool in &updatable_tools {
                println!(
                    "   - {} (current: {})",
                    tool.name,
                    tool.version.as_ref().unwrap_or(&"Unknown".to_string())
                );
            }
            println!();

            if Confirm::new()
                .with_prompt("Would you like to update these tools?")
                .default(true)
                .interact()?
            {
                for tool in &updatable_tools {
                    if let Err(e) = self.update_via_npm(tool).await {
                        println!("⚠️  Failed to update {}: {}", tool.name, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Run the CLI management flow
    pub async fn run_management_flow(&mut self) -> Result<()> {
        self.show_welcome();
        self.show_main_menu().await
    }

    /// Display welcome message
    fn show_welcome(&self) {
        self.term.clear_screen().unwrap();
        println!(
            "{}",
            style("🚀 Welcome to Agentic Warden CLI Manager")
                .bold()
                .cyan()
        );
        println!("{}", "=".repeat(50));
        println!("Universal AI CLI tool manager with cloud sync");
        println!("OS: {}", Self::get_os_type());
        println!();
    }

    /// Show main menu and handle user selection
    async fn show_main_menu(&mut self) -> Result<()> {
        loop {
            let items = vec![
                "🔍 Detect and Setup CLI Tools",
                "🚀 Install Claude CLI (Smart)",
                "🔄 Check for Updates",
                "⚙️  Configure CLI Tools",
                "📋 List Installed Tools",
                "🚪 Exit",
            ];

            let selection = Select::new()
                .with_prompt("What would you like to do?")
                .items(&items)
                .interact()?;

            match selection {
                0 => self.detect_and_setup_all_tools().await?,
                1 => self.install_claude_cli().await?,
                2 => self.check_for_updates().await?,
                3 => self.configure_tools().await?,
                4 => self.list_installed_tools().await?,
                5 => {
                    println!("{}", style("👋 Goodbye!").green());
                    break;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    /// List installed tools
    async fn list_installed_tools(&mut self) -> Result<()> {
        println!("\n{}", style("📋 Installed CLI Tools").bold().blue());
        println!("{}", "-".repeat(30));

        // Refresh tool status
        for tool in &mut self.tools {
            Self::detect_tool_installation(tool);
        }

        if self.tools.iter().all(|t| !t.installed) {
            println!("❌ No AI CLI tools are installed.");
            println!();
            println!("💡 Use option 1 to detect and setup CLI tools.");
        } else {
            for tool in &self.tools {
                if tool.installed {
                    let install_type_str = match tool.install_type {
                        Some(InstallType::Npm) => "NPM",
                        Some(InstallType::Native) => "Native",
                        Some(InstallType::Unknown) => "Unknown",
                        None => "Unknown",
                    };

                    let path_str = tool
                        .install_path
                        .as_ref()
                        .and_then(|p| p.to_str())
                        .unwrap_or("Unknown");

                    println!("✅ {}", style(&tool.name).bold());
                    println!("   Command: {}", tool.command);
                    println!(
                        "   Version: {}",
                        tool.version.as_ref().unwrap_or(&"Unknown".to_string())
                    );
                    println!("   Type: {}", install_type_str);
                    println!("   Path: {}", path_str);
                    println!("   Description: {}", tool.description);
                    println!();
                }
            }
        }

        Confirm::new()
            .with_prompt("Press Enter to continue...")
            .default(true)
            .interact()?;

        Ok(())
    }

    /// Configure CLI tools
    async fn configure_tools(&self) -> Result<()> {
        println!("\n{}", style("⚙️  CLI Configuration").bold().blue());
        println!("{}", "-".repeat(25));

        println!("ℹ️  Configuration features will be available in future versions:");
        println!("   • API key management");
        println!("   • Tool preferences and settings");
        println!("   • Custom installation paths");
        println!("   • Proxy configuration");
        println!();

        Confirm::new()
            .with_prompt("Press Enter to continue...")
            .default(true)
            .interact()?;

        Ok(())
    }
}
