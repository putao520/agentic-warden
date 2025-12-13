//! Process tree tracking module
//!
//! This module provides functionality to traverse the process tree from the current
//! process up to the root parent, enabling process isolation based on ancestry.
//!
//! Platform strategy:
//! - Linux/macOS: Use psutil for comprehensive process information
//! - Windows: Use sysinfo library for cross-platform process information

#[cfg(unix)]
use psutil::process::Process;

#[cfg(windows)]
use parking_lot::RwLock;
#[cfg(windows)]
use std::cell::RefCell;
#[cfg(windows)]
use std::collections::HashMap;
#[cfg(windows)]
use std::time::{Duration, Instant};
#[cfg(windows)]
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::core::models::{AiCliProcessInfo, ProcessTreeInfo};
use crate::error::{AgenticResult, AgenticWardenError};
use std::path::PathBuf;
use std::sync::OnceLock;
use thiserror::Error;

// Global cache for root parent PID - computed once per process lifetime
static ROOT_PARENT_PID_CACHE: OnceLock<u32> = OnceLock::new();
#[cfg(windows)]
static PROCESS_INFO_CACHE: OnceLock<RwLock<HashMap<u32, CacheEntry>>> = OnceLock::new();
#[cfg(windows)]
const PROCESS_CACHE_TTL: Duration = Duration::from_millis(750);
#[cfg(windows)]
thread_local! {
    static THREAD_SYSINFO: RefCell<SysinfoState> = RefCell::new(SysinfoState::new());
}

#[derive(Error, Debug)]
pub enum ProcessTreeError {
    #[cfg(unix)]
    #[error("Failed to get process information: {0}")]
    ProcessInfo(#[from] psutil::Error),
    #[cfg(windows)]
    #[allow(dead_code)]
    #[error("Failed to get process information: {0}")]
    ProcessInfo(String),
    #[allow(dead_code)]
    #[error("Process not found: {0}")]
    ProcessNotFound(u32),
    #[allow(dead_code)]
    #[error("Permission denied accessing process: {0}")]
    PermissionDenied(u32),
    #[allow(dead_code)]
    #[error("Unsupported platform")]
    UnsupportedPlatform,
    #[error("Process tree validation failed: {0}")]
    Validation(String),
}

// Add support for psutil::process::ProcessError conversion
#[cfg(unix)]
impl From<psutil::process::ProcessError> for ProcessTreeError {
    fn from(err: psutil::process::ProcessError) -> Self {
        // Convert ProcessError to ProcessTreeError using Debug formatting
        use std::io;
        ProcessTreeError::ProcessInfo(psutil::Error::from(io::Error::other(format!("{:?}", err))))
    }
}

#[cfg(windows)]
#[derive(Clone, Debug)]
struct ProcessInfo {
    parent: Option<u32>,
    name: Option<String>,
    cmdline: Option<Vec<String>>,
    executable_path: Option<PathBuf>,
}

#[cfg(windows)]
#[derive(Clone, Debug)]
struct CacheEntry {
    info: ProcessInfo,
    expires_at: Instant,
}

#[cfg(windows)]
#[derive(Debug)]
struct SysinfoState {
    system: System,
}

#[cfg(windows)]
impl SysinfoState {
    fn new() -> Self {
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, true);
        Self { system }
    }

    fn snapshot(&mut self, pid: u32, include_cmdline: bool) -> Option<ProcessInfo> {
        let sys_pid = Pid::from_u32(pid);
        let pid_list = [sys_pid];
        let refresh_kind = if include_cmdline {
            ProcessRefreshKind::everything()
        } else {
            ProcessRefreshKind::new()
        };
        let _ = self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&pid_list),
            true,
            refresh_kind,
        );
        if self.system.process(sys_pid).is_none() {
            self.system.refresh_processes(ProcessesToUpdate::All, true);
        }
        self.system.process(sys_pid).map(|process| {
            let parent = process.parent().map(|p| p.as_u32());
            let name = Some(process.name().to_string_lossy().into_owned());
            let cmdline = if include_cmdline {
                let args: Vec<String> = process
                    .cmd()
                    .iter()
                    .map(|arg| arg.to_string_lossy().into_owned())
                    .collect();
                if args.is_empty() {
                    None
                } else {
                    Some(args)
                }
            } else {
                None
            };
            let executable_path = process.exe().map(|path| path.to_path_buf());
            ProcessInfo {
                parent,
                name,
                cmdline,
                executable_path,
            }
        })
    }
}

#[cfg(windows)]
fn process_info_cache() -> &'static RwLock<HashMap<u32, CacheEntry>> {
    PROCESS_INFO_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[cfg(windows)]
fn read_process_info_windows(
    pid: u32,
    require_cmdline: bool,
) -> Result<ProcessInfo, ProcessTreeError> {
    if pid == 0 {
        return Ok(ProcessInfo {
            parent: None,
            name: Some("System Idle Process".to_string()),
            cmdline: None,
            executable_path: None,
        });
    }

    let now = Instant::now();
    {
        let cache_guard = process_info_cache().read();
        if let Some(entry) = cache_guard.get(&pid) {
            if entry.expires_at > now && (!require_cmdline || entry.info.cmdline.is_some()) {
                return Ok(entry.info.clone());
            }
        }
    }

    let snapshot = THREAD_SYSINFO
        .with(|state| state.borrow_mut().snapshot(pid, require_cmdline))
        .ok_or(ProcessTreeError::ProcessNotFound(pid))?;

    {
        let mut cache_guard = process_info_cache().write();
        cache_guard.insert(
            pid,
            CacheEntry {
                info: snapshot.clone(),
                expires_at: now + PROCESS_CACHE_TTL,
            },
        );
    }

    Ok(snapshot)
}

impl ProcessTreeInfo {
    /// Get the current process tree information
    pub fn current() -> AgenticResult<Self> {
        get_process_tree(std::process::id())
    }
}

/// Get the root parent process ID for the current process (cached)
/// This function computes the root parent PID only once per process lifetime
/// It finds the nearest AI CLI process in the process tree, not just any parent
pub fn get_root_parent_pid_cached() -> AgenticResult<u32> {
    let current_pid = std::process::id();

    // Use a simple caching approach - compute if not set
    if let Some(&cached_pid) = ROOT_PARENT_PID_CACHE.get() {
        Ok(cached_pid)
    } else {
        let ai_root_pid = find_ai_cli_root_parent(current_pid)?;
        // Set the cache (ignore if another thread set it first)
        let _ = ROOT_PARENT_PID_CACHE.set(ai_root_pid);
        Ok(ai_root_pid)
    }
}

/// Find the nearest AI CLI process in the process tree
/// If no AI CLI process is found, falls back to the traditional root parent
pub fn find_ai_cli_root_parent(pid: u32) -> AgenticResult<u32> {
    let process_tree = get_process_tree(pid)?;

    process_tree
        .get_ai_cli_root()
        .ok_or_else(|| ProcessTreeError::ProcessNotFound(pid).into())
}

/// Get the specific AI CLI type from a process name and command line
/// Returns: Some("claude"), Some("codex"), Some("gemini"), or None
fn get_ai_cli_type(pid: u32, process_name: &str) -> Option<String> {
    let name_lower = process_name.to_lowercase();

    // Remove .exe extension on Windows for comparison
    let clean_name = if cfg!(windows) && name_lower.ends_with(".exe") {
        &name_lower[..name_lower.len() - 4]
    } else {
        &name_lower
    };

    // Native AI CLI processes - exact matches first
    // Added claude-code for Claude Code support
    match clean_name {
        "claude" | "claude-cli" | "anthropic-claude" | "claude-code" => {
            return Some("claude".to_string())
        }
        "codex" | "codex-cli" | "openai-codex" => return Some("codex".to_string()),
        "gemini" | "gemini-cli" | "google-gemini" => return Some("gemini".to_string()),
        _ => {}
    }

    // Partial matches for variations (exclude claude-desktop to avoid confusion)
    // Also support claude-code in partial matches
    if (clean_name.contains("claude") || clean_name.contains("claude-code"))
        && !clean_name.contains("claude-desktop")
    {
        return Some("claude".to_string());
    }
    if clean_name.contains("codex") {
        return Some("codex".to_string());
    }
    if clean_name.contains("gemini") {
        return Some("gemini".to_string());
    }

    // NPM-based AI CLI processes - only for node-based AI CLIs
    if clean_name == "node" {
        if let Some(ai_type) = detect_npm_ai_cli_type(pid) {
            return Some(ai_type);
        }
    }

    None
}

/// Enhanced detection for NPM AI CLI processes using command line inspection
pub fn detect_npm_ai_cli_type(pid: u32) -> Option<String> {
    #[cfg(unix)]
    {
        detect_npm_ai_cli_type_unix(pid)
    }

    #[cfg(windows)]
    {
        detect_npm_ai_cli_type_windows(pid)
    }
}

#[cfg(unix)]
fn detect_npm_ai_cli_type_unix(pid: u32) -> Option<String> {
    get_command_line(pid)
        .and_then(|cmd| analyze_cmdline_for_ai_cli(&cmd))
        .or_else(|| Some("node".to_string()))
}

fn build_ai_cli_process_info(pid: u32) -> Option<AiCliProcessInfo> {
    let process_name = get_process_name(pid)?;
    let ai_type = get_ai_cli_type(pid, &process_name)?;
    let mut info = AiCliProcessInfo::new(pid, ai_type).with_process_name(process_name);

    if let Some(cmdline) = get_command_line(pid) {
        let npm_flag = is_npm_command_line(&cmdline);
        info = info
            .with_command_line(cmdline)
            .with_is_npm_package(npm_flag);
    }

    if let Some(path) = get_executable_path(pid) {
        info = info.with_executable_path(Some(path));
    }

    info.validate().ok()?;
    Some(info)
}

fn is_npm_command_line(cmdline: &str) -> bool {
    let cmd = cmdline.to_lowercase();
    cmd.contains("npm exec") || cmd.contains("npx ")
}

#[cfg(unix)]
fn get_command_line(pid: u32) -> Option<String> {
    use std::fs::File;
    use std::io::Read;

    let cmdline_path = format!("/proc/{pid}/cmdline");
    let mut file = File::open(cmdline_path).ok()?;
    let mut raw = String::new();
    file.read_to_string(&mut raw).ok()?;
    let cleaned = raw.replace('\0', " ").trim().to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[cfg(windows)]
fn get_command_line(pid: u32) -> Option<String> {
    read_process_info_windows(pid, true)
        .ok()
        .and_then(|info| info.cmdline)
        .map(|cmd| cmd.join(" "))
        .filter(|cmd| !cmd.trim().is_empty())
}

#[cfg(unix)]
fn get_executable_path(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/exe")).ok()
}

#[cfg(windows)]
fn get_executable_path(pid: u32) -> Option<PathBuf> {
    read_process_info_windows(pid, false)
        .ok()
        .and_then(|info| info.executable_path)
}

#[cfg(windows)]
fn detect_npm_ai_cli_type_windows(pid: u32) -> Option<String> {
    get_command_line(pid)
        .and_then(|cmd| analyze_cmdline_for_ai_cli(&cmd))
        .or_else(|| Some("node".to_string()))
}

/// Analyze command line string to identify specific AI CLI type
#[allow(dead_code)]
fn analyze_cmdline_for_ai_cli(cmdline: &str) -> Option<String> {
    let cmdline_lower = cmdline.to_lowercase();

    // Direct AI CLI execution via npm/npx
    if cmdline_lower.contains("claude-cli") || cmdline_lower.contains("@anthropic-ai/claude-cli") {
        return Some("claude".to_string());
    }
    if cmdline_lower.contains("claude-code") {
        return Some("claude".to_string());
    }
    if cmdline_lower.contains("codex-cli") {
        return Some("codex".to_string());
    }
    if cmdline_lower.contains("gemini-cli") || cmdline_lower.contains("@google/generative-ai-cli") {
        return Some("gemini".to_string());
    }

    // npm exec patterns
    if cmdline_lower.contains("npm exec") {
        if cmdline_lower.contains("@anthropic-ai/claude") {
            return Some("claude".to_string());
        }
        if cmdline_lower.contains("codex") {
            return Some("codex".to_string());
        }
        if cmdline_lower.contains("gemini") {
            return Some("gemini".to_string());
        }
    }

    // npx patterns
    if cmdline_lower.contains("npx") {
        if cmdline_lower.contains("@anthropic-ai/claude") {
            return Some("claude".to_string());
        }
        if cmdline_lower.contains("codex") {
            return Some("codex".to_string());
        }
        if cmdline_lower.contains("gemini") {
            return Some("gemini".to_string());
        }
    }

    // Node.js with module paths
    if cmdline_lower.contains("node_modules") {
        if cmdline_lower.contains("claude-cli") || cmdline_lower.contains("claude-code") {
            return Some("claude".to_string());
        }
        if cmdline_lower.contains("codex-cli") {
            return Some("codex".to_string());
        }
        if cmdline_lower.contains("gemini-cli") {
            return Some("gemini".to_string());
        }
    }

    // Generic Node.js detection if no specific AI CLI found
    Some("node".to_string())
}

/// Get the process tree from a given PID up to the root parent
fn get_process_tree_internal(pid: u32) -> Result<ProcessTreeInfo, ProcessTreeError> {
    let mut chain = Vec::new();

    // Start with the current process
    let mut current_pid = pid;
    chain.push(current_pid);
    let mut ai_cli_info: Option<AiCliProcessInfo> = None;

    // Traverse up the process tree
    for _ in 0..50 {
        match get_parent_pid(current_pid)? {
            Some(parent_pid) => {
                if parent_pid == current_pid || parent_pid == 0 {
                    // We've reached the root or found a loop
                    break;
                }

                chain.push(parent_pid);
                if ai_cli_info.is_none() {
                    ai_cli_info = build_ai_cli_process_info(parent_pid);
                }
                current_pid = parent_pid;

                // Check if we've reached a known root process
                if is_root_process(parent_pid) {
                    break;
                }
            }
            None => {
                break;
            }
        }
    }

    let info = ProcessTreeInfo::new(chain).with_ai_cli_process(ai_cli_info);
    info.validate()
        .map_err(|err| ProcessTreeError::Validation(err.to_string()))?;
    Ok(info)
}

pub fn get_process_tree(pid: u32) -> AgenticResult<ProcessTreeInfo> {
    get_process_tree_internal(pid).map_err(AgenticWardenError::from)
}

/// Get the parent PID for a given process using platform-specific methods
fn get_parent_pid(pid: u32) -> Result<Option<u32>, ProcessTreeError> {
    #[cfg(windows)]
    {
        get_parent_pid_windows(pid)
    }

    #[cfg(unix)]
    {
        get_parent_pid_unix(pid)
    }
}

/// Windows-specific implementation backed by a cached sysinfo snapshot
#[cfg(windows)]
fn get_parent_pid_windows(pid: u32) -> Result<Option<u32>, ProcessTreeError> {
    if pid == 0 {
        return Ok(None);
    }
    match read_process_info_windows(pid, false) {
        Ok(info) => Ok(info.parent.filter(|parent| *parent != pid)),
        Err(ProcessTreeError::ProcessNotFound(_)) => Ok(None),
        Err(err) => Err(err),
    }
}

/// Unix-specific implementation using psutil
#[cfg(unix)]
fn get_parent_pid_unix(pid: u32) -> Result<Option<u32>, ProcessTreeError> {
    let process = Process::new(pid.into())?;
    match process.ppid() {
        Ok(parent_pid_opt) => {
            // ppid() returns Option<u32>, already the correct type
            Ok(parent_pid_opt)
        }
        Err(err) => Err(err.into()),
    }
}

/// Check if a PID represents a root process
fn is_root_process(pid: u32) -> bool {
    #[cfg(windows)]
    {
        // Windows root processes
        pid == 0 || pid == 4 || pid == 1
    }

    #[cfg(unix)]
    {
        // Unix root processes
        pid == 1 || pid == 0
    }
}

/// Get process name for a given PID (platform-specific)
#[allow(dead_code)]
pub fn get_process_name(pid: u32) -> Option<String> {
    #[cfg(windows)]
    {
        get_process_name_windows(pid)
    }

    #[cfg(unix)]
    {
        get_process_name_unix(pid)
    }
}

/// Windows process name implementation using sysinfo
#[cfg(windows)]
#[allow(dead_code)]
fn get_process_name_windows(pid: u32) -> Option<String> {
    read_process_info_windows(pid, false)
        .ok()
        .and_then(|info| info.name)
}

/// Unix process name implementation using psutil
#[cfg(unix)]
fn get_process_name_unix(pid: u32) -> Option<String> {
    match Process::new(pid.into()) {
        Ok(process) => match process.name() {
            Ok(name) => Some(name),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

/// Check if two processes have the same root parent
#[allow(dead_code)]
pub fn same_root_parent(pid1: u32, pid2: u32) -> AgenticResult<bool> {
    let tree1 = get_process_tree(pid1)?;
    let tree2 = get_process_tree(pid2)?;

    match (tree1.root_parent_pid, tree2.root_parent_pid) {
        (Some(root1), Some(root2)) => Ok(root1 == root2),
        _ => Ok(false),
    }
}

/// Get direct parent PID using fallback methods
#[allow(dead_code)]
pub fn get_direct_parent_pid_fallback() -> Option<u32> {
    get_parent_pid(std::process::id()).ok().flatten()
}

fn process_tree_issue(operation: &str, message: impl Into<String>) -> AgenticWardenError {
    AgenticWardenError::Process {
        message: message.into(),
        command: format!("process_tree::{operation}"),
        source: None,
    }
}

#[allow(dead_code)]
fn process_tree_issue_with_source(
    operation: &str,
    message: impl Into<String>,
    source: impl std::error::Error + Send + Sync + 'static,
) -> AgenticWardenError {
    AgenticWardenError::Process {
        message: message.into(),
        command: format!("process_tree::{operation}"),
        source: Some(Box::new(source)),
    }
}

impl From<ProcessTreeError> for AgenticWardenError {
    fn from(err: ProcessTreeError) -> Self {
        match err {
            #[cfg(unix)]
            ProcessTreeError::ProcessInfo(source) => {
                process_tree_issue_with_source("info", "Failed to get process information", source)
            }
            #[cfg(windows)]
            ProcessTreeError::ProcessInfo(message) => process_tree_issue(
                "info",
                format!("Failed to get process information: {message}"),
            ),
            ProcessTreeError::ProcessNotFound(pid) => {
                process_tree_issue("lookup", format!("Process {pid} not found"))
            }
            ProcessTreeError::PermissionDenied(pid) => process_tree_issue(
                "permission",
                format!("Permission denied accessing process {pid}"),
            ),
            ProcessTreeError::UnsupportedPlatform => process_tree_issue(
                "platform",
                "Unsupported platform for process tree inspection",
            ),
            ProcessTreeError::Validation(message) => process_tree_issue("validate", message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    use sysinfo::System;

    #[test]
    fn test_current_process_tree() {
        let result = ProcessTreeInfo::current();
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert!(!tree.process_chain.is_empty());
        assert!(tree.depth >= 1);
        assert_eq!(tree.process_chain[0], std::process::id());
    }

    #[test]
    fn test_same_root_parent_current() {
        let current_pid = std::process::id();
        let result = same_root_parent(current_pid, current_pid);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_direct_parent_fallback() {
        let parent_pid = get_direct_parent_pid_fallback();
        assert!(parent_pid.is_some(), "Should be able to get parent PID");

        let parent = parent_pid.unwrap();
        assert!(parent > 0, "Parent PID should be positive");
    }

    #[test]
    fn test_root_process_detection() {
        #[cfg(windows)]
        {
            assert!(is_root_process(0), "PID 0 should be root on Windows");
            assert!(is_root_process(4), "PID 4 should be root on Windows");
        }

        #[cfg(unix)]
        {
            assert!(is_root_process(1), "PID 1 should be root on Unix");
        }
    }

    #[test]
    fn test_process_chain_validity() {
        let tree = ProcessTreeInfo::current().expect("Failed to get process tree");

        // Verify all PIDs are positive
        for pid in &tree.process_chain {
            assert!(*pid > 0, "All PIDs in process chain should be positive");
        }

        // Verify no duplicates (except possible root)
        let mut seen = std::collections::HashSet::new();
        for pid in &tree.process_chain {
            assert!(
                !seen.contains(pid),
                "Process chain should not contain duplicate PIDs"
            );
            seen.insert(*pid);
        }
    }

    #[test]
    fn test_process_name_retrieval() {
        let current_pid = std::process::id();
        let process_name = get_process_name(current_pid);

        // Process name should be available
        if let Some(name) = process_name {
            println!("Current process name: {}", name);
            assert!(!name.is_empty(), "Process name should not be empty");
        }
    }

    #[test]
    fn test_analyze_cmdline_detects_specific_cli() {
        let claude_cmd = "node ./node_modules/@anthropic-ai/claude-cli/bin/run.js ask";
        assert_eq!(
            analyze_cmdline_for_ai_cli(claude_cmd),
            Some("claude".to_string())
        );

        let codex_cmd = "npx codex-cli chat --model gpt-4";
        assert_eq!(
            analyze_cmdline_for_ai_cli(codex_cmd),
            Some("codex".to_string())
        );

        let gemini_cmd = "npm exec @google/generative-ai-cli -- text";
        assert_eq!(
            analyze_cmdline_for_ai_cli(gemini_cmd),
            Some("gemini".to_string())
        );
    }

    #[test]
    fn test_analyze_cmdline_defaults_to_node() {
        let generic_cmd = "node ./scripts/custom-runner.js";
        assert_eq!(
            analyze_cmdline_for_ai_cli(generic_cmd),
            Some("node".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_unix_psutil_integration() {
        // Test that we can use psutil on Unix systems
        let current_pid = std::process::id();
        let process = Process::new(current_pid.into());
        assert!(
            process.is_ok(),
            "Should be able to access current process via psutil"
        );

        if let Ok(proc) = process {
            // Test getting parent PID via psutil
            let ppid_result = proc.ppid();
            assert!(
                ppid_result.is_ok(),
                "Should be able to get parent PID via psutil"
            );

            // Test getting process name via psutil
            let name_result = proc.name();
            println!("Process name via psutil: {:?}", name_result);
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_sysinfo_integration() {
        // Test that we can use sysinfo on Windows
        let mut system = System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let current_pid = std::process::id();
        let found_process = system
            .processes()
            .values()
            .find(|p| p.pid().as_u32() == current_pid);

        assert!(
            found_process.is_some(),
            "Should be able to find current process via sysinfo"
        );

        if let Some(process) = found_process {
            println!(
                "Current process found: {} (PID: {})",
                process.name().to_string_lossy(),
                process.pid().as_u32()
            );
            assert!(
                !process.name().is_empty(),
                "Process name should not be empty"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_process_info_cache_roundtrip() {
        let pid = std::process::id();
        let info_a =
            read_process_info_windows(pid, false).expect("Process info should be available");
        assert!(info_a.name.is_some());

        let info_b = read_process_info_windows(pid, false)
            .expect("Process info should be cached and still available");
        assert!(info_b.name.is_some());
    }
}
