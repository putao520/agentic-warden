//! Process tree tracking module
//!
//! This module provides functionality to traverse the process tree from the current
//! process up to the root parent, enabling process isolation based on ancestry.
//!
//! Platform strategy:
//! - Linux/macOS: Use psutil for comprehensive process information
//! - Windows: Use sysinfo library for cross-platform process information

#[cfg(unix)]
use psutil::process::{Process, ProcessCollector};

use std::sync::OnceLock;
use thiserror::Error;

// Global cache for root parent PID - computed once per process lifetime
static ROOT_PARENT_PID_CACHE: OnceLock<u32> = OnceLock::new();

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
}

/// Process tree information containing the full process chain
#[derive(Debug, Clone)]
pub struct ProcessTreeInfo {
    /// Chain of PIDs from current process to root parent (inclusive)
    pub process_chain: Vec<u32>,
    /// Root parent PID (first element in process_chain)
    pub root_parent_pid: Option<u32>,
    /// Depth of the process tree
    pub depth: usize,
}

impl ProcessTreeInfo {
    /// Get the current process tree information
    pub fn current() -> Result<Self, ProcessTreeError> {
        get_process_tree(std::process::id())
    }
}

/// Get the root parent process ID for the current process (cached)
/// This function computes the root parent PID only once per process lifetime
pub fn get_root_parent_pid_cached() -> Result<u32, ProcessTreeError> {
    Ok(*ROOT_PARENT_PID_CACHE.get_or_init(|| {
        // Only compute once per process lifetime
        let current_pid = std::process::id();
        get_parent_pid(current_pid).unwrap_or(current_pid)
    }))
}

/// Get the process tree from a given PID up to the root parent
pub fn get_process_tree(pid: u32) -> Result<ProcessTreeInfo, ProcessTreeError> {
    let mut chain = Vec::new();

    // Start with the current process
    let mut current_pid = pid;
    chain.push(current_pid);

    // Traverse up the process tree
    for _ in 0..50 {
        // Limit depth to prevent infinite loops
        match get_parent_pid(current_pid) {
            Some(parent_pid) => {
                if parent_pid == current_pid || parent_pid == 0 {
                    // We've reached the root or found a loop
                    break;
                }

                chain.push(parent_pid);
                current_pid = parent_pid;

                // Check if we've reached a known root process
                if is_root_process(parent_pid) {
                    break;
                }
            }
            None => {
                // Can't get parent info, stop here
                break;
            }
        }
    }

    let depth = chain.len();
    let root_parent_pid = if chain.len() > 1 {
        chain.last().copied()
    } else {
        Some(pid)
    };

    Ok(ProcessTreeInfo {
        process_chain: chain,
        root_parent_pid,
        depth,
    })
}

/// Get the parent PID for a given process using platform-specific methods
fn get_parent_pid(pid: u32) -> Option<u32> {
    #[cfg(windows)]
    {
        get_parent_pid_windows(pid)
    }

    #[cfg(unix)]
    {
        get_parent_pid_unix(pid)
    }
}

/// Windows-specific implementation using sysinfo library
#[cfg(windows)]
fn get_parent_pid_windows(pid: u32) -> Option<u32> {
    let mut system = sysinfo::System::new();

    // Refresh all processes to get complete information
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    // Find the process with the target PID
    if let Some(process) = system.processes().get(&(pid as usize).into()) {
        return process.parent().map(|p| p.as_u32());
    }

    None
}

/// Unix-specific implementation using psutil
#[cfg(unix)]
fn get_parent_pid_unix(pid: u32) -> Option<u32> {
    match Process::new(pid.into()) {
        Ok(process) => match process.ppid() {
            Ok(parent_pid) => Some(parent_pid as u32),
            Err(_) => None,
        },
        Err(_) => None,
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
    let mut system = sysinfo::System::new();

    // Refresh all processes to get complete information
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    // Find the process with the target PID
    if let Some(process) = system.processes().get(&(pid as usize).into()) {
        return Some(process.name().to_string_lossy().into_owned());
    }

    None
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
pub fn same_root_parent(pid1: u32, pid2: u32) -> Result<bool, ProcessTreeError> {
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
    get_parent_pid(std::process::id())
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
}
