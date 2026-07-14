//! CLI Tool Detection Module
//!
//! This module provides CLI tool detection and metadata for AI CLI tools
//! (Claude, Codex, Gemini). It focuses on core functionality without
//! interactive UI components.

#![allow(dead_code)] // CLI管理模块，部分功能当前未使用

use crate::patcher::claude::versions::ClaudeVersion;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};


/// Ctrl+B handler for sending process to background
struct CtrlBHandler {
    triggered: Arc<AtomicU8>,
    _task: Option<tokio::task::JoinHandle<()>>,
}

impl CtrlBHandler {
    fn new() -> Self {
        let triggered = Arc::new(AtomicU8::new(0));
        let triggered_clone = triggered.clone();

        let _task = Some(tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if let Some(Ok(events)) = Self::try_read_event() {
                    for event in events {
                        if let Event::Key(KeyEvent {
                            code: KeyCode::Char('b'),
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        }) = event
                        {
                            let count = triggered_clone.fetch_add(1, Ordering::SeqCst);
                            if count == 0 {
                                eprintln!("\n⚠️  Press Ctrl+B again to send to background...");
                            } else {
                                eprintln!("\n🔄 Sending update process to background...");
                                triggered_clone.store(0, Ordering::SeqCst);
                            }
                        }
                    }
                }
            }
        }));

        Self { triggered, _task }
    }

    fn should_detach(&self) -> bool {
        self.triggered.load(Ordering::SeqCst) >= 2
    }

    fn reset(&self) {
        self.triggered.store(0, Ordering::SeqCst);
    }

    fn try_read_event() -> Option<std::io::Result<Vec<Event>>> {
        use crossterm::event;
        if event::poll(Duration::from_millis(0)).ok()? {
            let mut events = Vec::new();
            while event::poll(Duration::from_millis(0)).ok()? {
                if let Ok(event) = event::read() {
                    events.push(event);
                }
            }
            Some(Ok(events))
        } else {
            None
        }
    }
}

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
                name: "Claude Code".to_string(),
                command: "claude".to_string(),
                npm_package: "@anthropic-ai/claude-code".to_string(),
                description: "Anthropic Claude Code CLI tool".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
            CliTool {
                name: "Codex CLI".to_string(),
                command: "codex".to_string(),
                npm_package: "@openai/codex".to_string(),
                description: "OpenAI Codex CLI tool".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
            CliTool {
                name: "Gemini CLI".to_string(),
                command: "gemini".to_string(),
                npm_package: "@google/gemini-cli".to_string(),
                description: "Google Gemini CLI tool".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
            CliTool {
                name: "Grok Build".to_string(),
                command: "grok".to_string(),
                npm_package: String::new(), // Grok 不走 npm
                description: "xAI Grok Build CLI tool".to_string(),
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
    fn detect_install_type_static(path: &Path) -> Option<InstallType> {
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

    /// Auto-install Node.js if not available (cross-platform)
    pub async fn auto_install_nodejs() -> Result<()> {
        // First check if Node.js is already installed
        if Command::new("node")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            println!("✅ Node.js is already installed");
            return Ok(());
        }

        println!("📦 Node.js not detected. Attempting to install via nvm...");

        let os = Self::get_os_type();
        let install_result = match os {
            "Windows" => Self::install_nodejs_windows().await,
            "macOS" | "Linux" => Self::install_nodejs_via_nvm().await,
            _ => {
                anyhow::bail!("Unsupported operating system: {}", os);
            }
        };

        install_result?;

        // Verify installation
        println!("🔍 Verifying Node.js installation...");
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        if Command::new("node")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            println!("✅ Node.js installed successfully!");
            Ok(())
        } else {
            println!("⚠️  Node.js installation completed but not immediately available.");
            println!("   Please restart your terminal and try again.");
            anyhow::bail!("Node.js verification failed - terminal restart required");
        }
    }

    /// Install Node.js via nvm (Linux/macOS unified method)
    async fn install_nodejs_via_nvm() -> Result<()> {
        let os = Self::get_os_type();
        println!("🔧 Installing Node.js via nvm on {}...", os);

        // Step 1: Check if nvm is already installed
        let nvm_check = Command::new("bash")
            .arg("-c")
            .arg("command -v nvm")
            .output();

        let nvm_installed = nvm_check
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !nvm_installed {
            println!("  📥 nvm not found. Installing nvm...");

            // Download and install nvm
            let nvm_install_script =
                "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/master/install.sh | bash";

            let install_result = Command::new("bash")
                .arg("-c")
                .arg(nvm_install_script)
                .status();

            match install_result {
                Ok(status) if status.success() => {
                    println!("  ✅ nvm installed successfully");
                }
                _ => {
                    anyhow::bail!(
                        "Failed to install nvm. Please install manually: \
                        curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/master/install.sh | bash"
                    );
                }
            }

            // Source nvm in current shell session
            println!("  🔄 Loading nvm...");
        } else {
            println!("  ✅ nvm is already installed");
        }

        // Step 2: Install Node.js LTS via nvm
        println!("  📦 Installing Node.js LTS via nvm...");

        // Construct nvm command with proper environment sourcing
        let nvm_install_node = r#"
            export NVM_DIR="$HOME/.nvm"
            [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
            nvm install --lts
            nvm use --lts
            nvm alias default lts/*
        "#;

        let node_install = Command::new("bash")
            .arg("-c")
            .arg(nvm_install_node)
            .status();

        match node_install {
            Ok(status) if status.success() => {
                println!("  ✅ Node.js LTS installed via nvm");
                Ok(())
            }
            _ => {
                anyhow::bail!(
                    "Failed to install Node.js via nvm. Please run manually: \
                    nvm install --lts && nvm use --lts"
                );
            }
        }
    }

    /// Install Node.js on Windows using winget (preferred) or chocolatey
    async fn install_nodejs_windows() -> Result<()> {
        println!("🪟 Installing Node.js on Windows...");

        // Try winget first (available on Windows 10+ by default)
        println!("  Trying winget...");
        let winget_result = Command::new("winget")
            .args([
                "install",
                "OpenJS.NodeJS",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ])
            .status();

        if let Ok(status) = winget_result {
            if status.success() {
                return Ok(());
            }
            println!("  Winget installation failed, trying chocolatey...");
        }

        // Fallback to chocolatey
        println!("  Trying chocolatey...");
        let choco_result = Command::new("choco")
            .args(["install", "nodejs", "-y"])
            .status();

        match choco_result {
            Ok(status) if status.success() => Ok(()),
            _ => {
                anyhow::bail!(
                    "Failed to install Node.js on Windows. Please install manually from https://nodejs.org/ \
                     or install winget/chocolatey first."
                );
            }
        }
    }

    /// Get installation hint for a tool based on OS
    pub fn get_install_hint(&self, command: &str) -> String {
        match command.to_lowercase().as_str() {
            "claude" => "curl -fsSL https://claude.ai/install.sh | sh".to_string(),
            "codex" => "npm install -g @openai/codex".to_string(),
            "gemini" => "npm install -g @google/gemini-cli".to_string(),
            _ => format!("Install {} via appropriate package manager", command),
        }
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

    /// Check latest version from npm registry
    async fn check_latest_version(package: &str) -> Option<String> {
        // URL encode the package name for scoped packages (e.g., @anthropic-ai/claude-cli)
        let encoded_package = urlencoding::encode(package);
        let url = format!("https://registry.npmjs.org/{}/latest", encoded_package);

        match reqwest::get(&url).await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                                return Some(version.to_string());
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to parse npm registry response: {}", e);
                        }
                    }
                } else {
                    eprintln!("NPM registry returned status: {}", response.status());
                }
            }
            Err(e) => {
                eprintln!("Failed to query npm registry: {}", e);
            }
        }
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

/// Execute update/install for AI CLI tools
///
/// If tool_name is None, update all installed tools
/// If tool_name is Some, update/install that specific tool
/// Enhanced update function that updates both AIW and AI CLI tools
pub async fn execute_enhanced_update(tool: Option<&str>) -> Result<(bool, Vec<(String, bool, String)>)> {
    let mut aiw_updated = false;
    let mut cli_results = Vec::new();

    // 指定单工具更新时跳过 AIW 自更新（仅更新指定工具）
    if tool.is_none() {
        // Step 1: Update AIW itself
        println!("🔄 Checking for AIW updates...");
        match update_aiw().await {
            Ok(updated) => {
                aiw_updated = updated;
            }
            Err(e) => {
                eprintln!("⚠️  AIW update failed: {}", e);
                // Continue with AI CLI updates even if AIW update fails
            }
        }
    }

    // Step 2: Update AI CLI tools
    println!("\n🔧 Checking AI CLI tools for updates...");
    match execute_update(tool).await {
        Ok(results) => {
            cli_results = results;
        }
        Err(e) => {
            eprintln!("⚠️  AI CLI tools update failed: {}", e);
        }
    }


    // Check patch compatibility after update (do NOT auto-apply)
    check_patch_compatibility();
    Ok((aiw_updated, cli_results))
}

/// Update AIW itself
async fn update_aiw() -> Result<bool> {
    // Get current version
    let current_version = get_aiw_current_version()?;
    println!("📋 Current AIW version: {}", current_version);

    // Get latest version from NPM
    let latest_version = get_aiw_latest_version().await?;
    println!("📋 Latest AIW version: {}", latest_version);

    // Check if update is needed
    if current_version == latest_version {
        println!("✅ AIW is already up to date!");
        return Ok(false);
    }

    println!("🚀 Starting update to AIW v{}...", latest_version);
    perform_aiw_update(&latest_version).await?;
    println!("✅ AIW updated successfully to v{}!", latest_version);

    Ok(true)
}



/// Check patch compatibility after update and show hint
fn check_patch_compatibility() {
    let version = match get_claude_version_from_string() {
        Ok(v) => v,
        Err(_) => return,
    };

    println!(
        "\n💡 Claude CLI {}.{}.{} detected.",
        version.major, version.minor, version.patch
    );
    println!("   Run `aiw patch status` to check max-token patch state.");
}
fn get_claude_version_from_string() -> anyhow::Result<ClaudeVersion> {
    use std::process::Command;
    
    let output = Command::new("claude")
        .arg("--version")
        .output()?;
    
    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = version_str
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not parse version"))?;
    
    ClaudeVersion::from_string(version)
        .ok_or_else(|| anyhow::anyhow!("Invalid version format"))
}
/// Get current AIW version
fn get_aiw_current_version() -> Result<String> {
    // Use compile-time version directly - no subprocess needed
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// Get latest AIW version from GitHub Releases
async fn get_aiw_latest_version() -> Result<String> {
    println!("  📡 Fetching latest version from GitHub Releases...");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/putao520/agentic-warden/releases/latest")
        .header("User-Agent", "aiw-updater")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "GitHub API returned status {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let json: serde_json::Value = response.json().await?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing tag_name in GitHub release response"))?;

    // Strip leading 'v' if present: "v0.5.59" -> "0.5.59"
    let version = tag.strip_prefix('v').unwrap_or(tag).to_string();
    Ok(version)
}

/// Get platform-specific asset name for GitHub Release download
fn get_platform_asset_name() -> Result<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok("aiw-linux-x64");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok("aiw-windows-x64.exe");

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    anyhow::bail!(
        "Unsupported platform ({}/{}). Please build from source: cargo install --path .",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

/// Perform AIW update by downloading binary from GitHub Releases
async fn perform_aiw_update(version: &str) -> Result<()> {
    let asset = get_platform_asset_name()?;
    let url = format!(
        "https://github.com/putao520/agentic-warden/releases/download/v{}/{}",
        version, asset
    );

    println!("  📦 Downloading AIW v{} from GitHub Releases...", version);
    println!("  📥 {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "aiw-updater")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download release asset: HTTP {}",
            response.status()
        );
    }

    let bytes = response.bytes().await?;

    // Get current executable path
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine executable directory"))?;

    // Write to a temp file in the same directory (ensures atomic rename on same filesystem)
    let tmp_path = exe_dir.join(format!(".aiw-update-{}.tmp", std::process::id()));

    std::fs::write(&tmp_path, &bytes)?;

    // Platform-specific replacement
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))?;
        std::fs::rename(&tmp_path, &current_exe)?;
    }

    #[cfg(windows)]
    {
        // Windows can't overwrite a running exe directly; rename old first
        let old_path = exe_dir.join(".aiw-old.exe");
        let _ = std::fs::remove_file(&old_path); // clean up previous leftover
        std::fs::rename(&current_exe, &old_path)?;
        std::fs::rename(&tmp_path, &current_exe)?;
        let _ = std::fs::remove_file(&old_path); // best-effort cleanup
    }

    println!("  ✅ AIW v{} installed successfully!", version);
    println!("  💡 Please restart your terminal to use the new version.");
    Ok(())
}

/// Original update function for AI CLI tools only
pub async fn execute_update(tool_name: Option<&str>) -> Result<Vec<(String, bool, String)>> {
    let mut detector = CliToolDetector::new();
    detector.detect_all_tools()?;

    let mut results = Vec::new();

    // Determine which tools to process
    let tools_to_process: Vec<&CliTool> = if let Some(name) = tool_name {
        // Single tool mode - find the tool by command name
        match detector.get_tool_by_command(name) {
            Some(tool) => vec![tool],
            None => {
                anyhow::bail!(
                    "Unknown AI CLI tool: {}. Supported: claude, codex, gemini, grok",
                    name
                );
            }
        }
    } else {
        // All installed tools mode
        detector.get_installed_tools()
    };

    if tools_to_process.is_empty() {
        println!("No AI CLI tools to update.");
        println!("Use 'agentic-warden update <tool>' to install a specific tool.");
        return Ok(results);
    }

    // Check if any non-Claude/Grok tools need Node.js
    let needs_nodejs = tools_to_process.iter().any(|t| t.command != "claude" && t.command != "grok");
    if needs_nodejs {
        println!("🔍 Checking Node.js installation...");
        if let Err(e) = CliToolDetector::auto_install_nodejs().await {
            eprintln!("⚠️  Node.js auto-install failed: {}", e);
            eprintln!("Please install Node.js manually from https://nodejs.org/");
            // Don't bail - Claude can still be updated without Node.js
        }
    }

    // Process each tool
    for tool in tools_to_process {
        println!("\n🔧 Processing {}...", tool.name);

        if tool.command == "claude" {
            // Claude uses its own update mechanism, not npm
            let result = update_claude_cli(tool).await;
            results.push(result);
            continue;
        } else if tool.command == "grok" {
            // Grok uses its own download + patch mechanism
            let result = update_grok_cli(tool).await;
            results.push(result);
            continue;
        }

        // Non-Claude tools use npm for installation and updates
        let npm_package = &tool.npm_package;
        let current_version = tool.version.clone();

        // Get latest version from npm
        println!("  Checking latest version...");
        let latest_version = match CliToolDetector::check_latest_version(npm_package).await {
            Some(version) => version,
            None => {
                eprintln!("  ❌ Failed to get latest version for {}", npm_package);
                results.push((
                    tool.name.clone(),
                    false,
                    "Failed to check version".to_string(),
                ));
                continue;
            }
        };

        println!("  Latest version: {}", latest_version);

        if let Some(ref current) = current_version {
            println!("  Current version: {}", current);

            if current == &latest_version {
                println!("  ✅ Already up to date!");
                results.push((tool.name.clone(), true, "Already up to date".to_string()));
                continue;
            }
        } else {
            println!("  Not currently installed");
        }

        // Execute npm install (with real-time output and Ctrl+C handling)
        println!("  Installing...");
        let install_cmd = detector.get_install_hint(&tool.command);

        // Replace version if updating to latest
        let install_cmd = if current_version.is_some() {
            // Update mode - add @latest
            format!("{}@latest", install_cmd)
        } else {
            // Install mode - use as is
            install_cmd
        };

        let ctrlb = CtrlBHandler::new();

        let child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&install_cmd)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| {
                eprintln!("  ❌ Failed to execute command: {}", e);
                results.push((tool.name.clone(), false, format!("Execution error: {}", e)));
            });

        // If spawn failed, continue to next tool
        let mut child = match child {
            Ok(c) => c,
            Err(()) => continue,
        };

        loop {
            tokio::select! {
                result = child.wait() => {
                    match result {
                        Ok(status) if status.success() => {
                            println!("  ✅ Successfully updated/installed!");
                            results.push((tool.name.clone(), true, "Success".to_string()));
                        }
                        Ok(status) => {
                            eprintln!("  ❌ Installation failed with exit code: {:?}", status.code());
                            results.push((tool.name.clone(), false, format!("Failed: {:?}", status.code())));
                        }
                        Err(e) => {
                            eprintln!("  ❌ Failed to wait for process: {}", e);
                            results.push((tool.name.clone(), false, format!("Wait error: {}", e)));
                        }
                    }
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    if ctrlb.should_detach() {
                        eprintln!("  ℹ️  Update process running in background...");
                        ctrlb.reset();
                        results.push((tool.name.clone(), true, "Running in background".to_string()));
                        break;
                    }
                }
            }
        }
    }

    // Print summary
    println!("\n{}", "=".repeat(60));
    println!("📊 Update Summary:");
    for (name, success, message) in &results {
        let status = if *success { "✅" } else { "❌" };
        println!("  {} {} - {}", status, name, message);
    }
    println!("{}", "=".repeat(60));

    Ok(results)
}

/// Update/install Claude CLI using its native mechanism (with real-time output and Ctrl+B handling)
async fn update_claude_cli(tool: &CliTool) -> (String, bool, String) {
    let ctrlb = CtrlBHandler::new();

    if tool.installed {
        // Claude is installed - use `claude update`
        if let Some(ref ver) = tool.version {
            println!("  Current version: {}", ver);
        }
        println!("  Running claude update...");

        let mut child = match tokio::process::Command::new("claude")
            .arg("update")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ❌ Failed to run claude update: {}", e);
                return (tool.name.clone(), false, format!("Execution error: {}", e));
            }
        };

        loop {
            tokio::select! {
                result = child.wait() => {
                    match result {
                        Ok(status) if status.success() => {
                            println!("  ✅ Claude updated successfully!");
                            return (tool.name.clone(), true, "Success".to_string());
                        }
                        Ok(status) => {
                            eprintln!("  ❌ claude update failed with exit code: {:?}", status.code());
                            return (tool.name.clone(), false, format!("Update failed: {:?}", status.code()));
                        }
                        Err(e) => {
                            eprintln!("  ❌ Failed to wait for claude update: {}", e);
                            return (tool.name.clone(), false, format!("Wait error: {}", e));
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    if ctrlb.should_detach() {
                        eprintln!("  ℹ️  Claude update running in background...");
                        return (tool.name.clone(), true, "Running in background".to_string());
                    }
                }
            }
        }
    } else {
        // Claude not installed - use curl installer
        println!("  Installing via curl...");

        let mut child = match tokio::process::Command::new("sh")
            .arg("-c")
            .arg("curl -fsSL https://claude.ai/install.sh | sh")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ❌ Failed to execute curl installer: {}", e);
                return (tool.name.clone(), false, format!("Execution error: {}", e));
            }
        };

        loop {
            tokio::select! {
                result = child.wait() => {
                    match result {
                        Ok(status) if status.success() => {
                            println!("  ✅ Claude installed successfully!");
                            return (tool.name.clone(), true, "Success".to_string());
                        }
                        Ok(status) => {
                            eprintln!("  ❌ Installation failed with exit code: {:?}", status.code());
                            return (tool.name.clone(), false, format!("Installation failed: {:?}", status.code()));
                        }
                        Err(e) => {
                            eprintln!("  ❌ Failed to wait for installer: {}", e);
                            return (tool.name.clone(), false, format!("Wait error: {}", e));
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    if ctrlb.should_detach() {
                        eprintln!("  ℹ️  Claude installer running in background...");
                        return (tool.name.clone(), true, "Running in background".to_string());
                    }
                }
            }
        }
    }
}

/// 从 `grok --version` 输出中提取纯 semver。
///
/// `grok --version` 输出形如 `grok 0.2.99 (b1b49ccb71)`，
/// 而 `check_grok_latest_version` 返回 `0.2.99`（来自 JSON latestVersion）。
/// 直接 `==` 比较会失败触发不必要的下载，故需提取 semver。
fn normalize_grok_version(raw: &str) -> String {
    let trimmed = raw.trim();
    // 在字符串中查找第一个 x.y.z semver 模式
    for token in trimmed.split_whitespace() {
        if let Some(semver) = extract_semver(token) {
            return semver;
        }
    }
    // 没找到 token 级匹配，尝试在整个字符串中找（如 "grok 0.2.99" 没有空格分隔的边界）
    extract_semver(trimmed).unwrap_or_else(|| trimmed.to_string())
}

/// 从字符串中提取首个 `major.minor.patch` semver。
fn extract_semver(s: &str) -> Option<String> {
    // 找第一个数字起始位置
    let bytes = s.as_bytes();
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() {
            start = Some(i);
            break;
        }
    }
    let start = start?;
    let rest = &s[start..];
    // 匹配 \d+.\d+.\d+
    let re = regex::Regex::new(r"^(\d+)\.(\d+)\.(\d+)").unwrap();
    if let Some(capt) = re.captures(rest) {
        let m = capt.get(0).unwrap();
        return Some(m.as_str().to_string());
    }
    None
}

/// 更新 Grok CLI（AIW 自己下载 binary，下载后自动 patch）
async fn update_grok_cli(tool: &CliTool) -> (String, bool, String) {
    use crate::patcher::grok::install::get_grok_binary_path;

    // 1. 版本检查：grok update --check --json 得 latestVersion
    let latest = match check_grok_latest_version().await {
        Some(v) => v,
        None => return (tool.name.clone(), false, "Failed to check latest version".to_string()),
    };

    if tool.installed {
        // 规范化当前版本：grok --version 输出 "grok 0.2.99 (hash)" → 提取 "0.2.99"
        let cur_normalized = tool.version.as_deref().map(normalize_grok_version);
        if let Some(ref cur) = cur_normalized {
            println!("  Current version: {}", cur);
        }
        println!("  Latest version: {}", latest);

        // 已是最新则跳过（使用规范化后的版本比较）
        if cur_normalized.as_deref() == Some(latest.as_str()) {
            println!("  ✅ Already up to date!");
            return (tool.name.clone(), true, "Already up to date".to_string());
        }
    }

    // 2. 下载 binary
    let arch = if cfg!(target_arch = "x86_64") { "linux-x86_64" } else { "linux-arm64" };
    let url = format!("https://x.ai/cli/grok-{}-{}", latest, arch);
    println!("  ⬇️  Downloading from {}", url);

    let tmp_path = get_grok_binary_path()
        .map(|p| p.with_extension("tmp"))
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/grok-download.tmp"));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap();
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                Err(e) => return (tool.name.clone(), false, format!("Download body error: {}", e)),
            };
            if let Err(e) = std::fs::write(&tmp_path, &bytes) {
                return (tool.name.clone(), false, format!("Write tmp error: {}", e));
            }
        }
        Ok(resp) => {
            return (tool.name.clone(), false, format!("Download HTTP {}", resp.status()));
        }
        Err(e) => return (tool.name.clone(), false, format!("Download error: {}", e)),
    }

    // 3. 覆盖到 ~/.grok/downloads/grok-linux-x86_64
    let target = match get_grok_binary_path() {
        Ok(p) => p,
        Err(e) => return (tool.name.clone(), false, format!("Binary path error: {}", e)),
    };
    // 备份旧版
    let _ = std::fs::copy(&target, target.with_extension("bak"));
    if let Err(e) = std::fs::rename(&tmp_path, &target) {
        let _ = std::fs::copy(&tmp_path, &target);
        let _ = std::fs::remove_file(&tmp_path);
        if std::fs::metadata(&target).is_err() {
            return (tool.name.clone(), false, format!("Install move error: {}", e));
        }
    }
    // 设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
    }

    // 更新 version.json
    let _ = update_grok_version_json(&latest);

    println!("  ✅ Grok {} installed", latest);

    // 4. 下载后自动 patch（更新即 patch）
    println!("  🔧 Auto-patching Grok uploads...");
    match apply_grok_patches_after_update(&target) {
        Ok(n) => println!("  ✅ Applied {} patches", n),
        Err(e) => println!("  ⚠️  Auto-patch failed (run 'aiw patch grok-apply' manually): {}", e),
    }

    (tool.name.clone(), true, format!("Updated to {}", latest))
}

async fn check_grok_latest_version() -> Option<String> {
    let out = tokio::process::Command::new("grok")
        .arg("update").arg("--check").arg("--json")
        .output().await.ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    v.get("latestVersion")?.as_str().map(String::from)
}

fn update_grok_version_json(version: &str) -> std::io::Result<()> {
    use crate::patcher::grok::install::get_grok_binary_path;
    let vjson = get_grok_binary_path()
        .map(|p| p.parent().unwrap().parent().unwrap().join("version.json"))
        .unwrap_or_else(|_| std::path::PathBuf::from(format!(
            "{}/.grok/version.json", std::env::var("HOME").unwrap_or_default()
        )));
    let content = format!(
        "{{\"version\":\"{}\",\"stable_version\":null,\"checked_at\":null}}", version
    );
    std::fs::write(&vjson, content)
}

fn apply_grok_patches_after_update(binary_path: &std::path::Path) -> Result<usize, String> {
    use crate::patcher::grok::registry::get_grok_repo_bundle_patches;
    use crate::patcher::apply_file_patch;
    let patches = get_grok_repo_bundle_patches().map_err(|e| e.to_string())?;
    let mut n = 0;
    for patch in &patches {
        if apply_file_patch(binary_path, patch).is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_tool_detector_creation() {
        let detector = CliToolDetector::new();
        assert_eq!(detector.tools.len(), 4);
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

        // Test codex (available on npm)
        let codex_hint = detector.get_install_hint("codex");
        assert!(codex_hint.contains("npm install"));
        assert!(codex_hint.contains("@openai/codex"));

        // Test gemini (available on npm)
        let gemini_hint = detector.get_install_hint("gemini");
        assert!(gemini_hint.contains("npm install"));
        assert!(gemini_hint.contains("@google/gemini-cli"));

        // Test unknown tool
        let unknown_hint = detector.get_install_hint("unknown");
        assert!(unknown_hint.contains("Install"));
    }

    #[test]
    fn test_get_install_hint_claude() {
        let detector = CliToolDetector::new();
        let claude_hint = detector.get_install_hint("claude");

        // Claude uses curl installer, not npm
        assert!(claude_hint.contains("curl"));
        assert!(claude_hint.contains("claude.ai/install.sh"));
    }

    #[test]
    fn test_normalize_grok_version_extracts_semver() {
        // grok --version outputs "grok 0.2.99 (b1b49ccb71)" — must extract "0.2.99"
        assert_eq!(normalize_grok_version("grok 0.2.99 (b1b49ccb71)"), "0.2.99");
    }

    #[test]
    fn test_normalize_grok_version_already_clean() {
        // If version is already clean semver, return as-is
        assert_eq!(normalize_grok_version("0.2.99"), "0.2.99");
    }

    #[test]
    fn test_normalize_grok_version_empty() {
        assert_eq!(normalize_grok_version(""), "");
    }

    #[test]
    fn test_normalize_grok_version_no_match() {
        // No semver pattern found — return trimmed original
        assert_eq!(normalize_grok_version("unknown"), "unknown");
    }
}
