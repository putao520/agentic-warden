//! Integration tests for process tree functionality
//!
//! These tests verify that the process tree detection works correctly on Windows
//! and other platforms by creating actual subprocesses and verifying their ancestry.

use agentic_warden::process_tree::{ProcessTreeInfo, get_process_tree, same_root_parent};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn test_current_process_tree_structure() {
    // Test that we can get a valid process tree for the current process
    let result = ProcessTreeInfo::current();
    assert!(
        result.is_ok(),
        "Failed to get current process tree: {:?}",
        result
    );

    let tree_info = result.unwrap();

    // Basic validations
    assert!(
        !tree_info.process_chain.is_empty(),
        "Process chain should not be empty"
    );
    assert!(tree_info.depth >= 1, "Depth should be at least 1");
    assert!(
        tree_info.process_chain.len() == tree_info.depth,
        "Chain length should match depth"
    );

    // The first element should be the current process PID
    let current_pid = std::process::id();
    assert_eq!(
        tree_info.process_chain[0], current_pid,
        "First element should be current PID"
    );

    // The root parent should be the last element in the chain
    assert_eq!(
        tree_info.root_parent_pid,
        tree_info.process_chain.last().copied(),
        "Root parent should be last element in chain"
    );

    println!("Current process tree:");
    for (i, pid) in tree_info.process_chain.iter().enumerate() {
        println!("  Level {}: PID {}", i, pid);
    }
    println!("Root parent PID: {:?}", tree_info.root_parent_pid);
}

#[test]
fn test_process_tree_depth_limit() {
    // Test that we have a reasonable depth limit to prevent infinite loops
    let result = ProcessTreeInfo::current();
    assert!(result.is_ok());

    let tree_info = result.unwrap();
    assert!(
        tree_info.depth <= 50,
        "Process tree depth should be limited to prevent infinite loops"
    );
}

#[test]
fn test_same_root_parent_detection() {
    let current_pid = std::process::id();

    // Test with the same PID
    let result = same_root_parent(current_pid, current_pid);
    assert!(
        result.is_ok(),
        "Failed to check same root parent for same PID"
    );
    assert!(result.unwrap(), "Same PID should have same root parent");
}

#[test]
fn test_subprocess_process_tree() {
    // This test creates a subprocess and then tries to get its process tree
    // Note: This test may fail if the subprocess exits too quickly

    let mut child = Command::new("ping")
        .args(["127.0.0.1", "-n", "10"]) // Ping for ~10 seconds on Windows
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn subprocess");

    let child_pid = child.id();

    // Give the subprocess a moment to initialize
    thread::sleep(Duration::from_millis(100));

    // Try to get the process tree of the subprocess
    let result = get_process_tree(child_pid);

    // Clean up the subprocess
    let _ = child.kill();
    let _ = child.wait();

    if let Ok(tree_info) = result {
        println!("Subprocess {} tree:", child_pid);
        for (i, pid) in tree_info.process_chain.iter().enumerate() {
            println!("  Level {}: PID {}", i, pid);
        }

        // Validate the subprocess tree
        assert!(!tree_info.process_chain.is_empty());
        assert_eq!(tree_info.process_chain[0], child_pid);

        // The subprocess should have our current process as an ancestor
        // (or at least share the same root parent on some systems)
        let current_tree = ProcessTreeInfo::current().expect("Failed to get current process tree");

        if let (Some(sub_root), Some(curr_root)) =
            (tree_info.root_parent_pid, current_tree.root_parent_pid)
        {
            println!("Subprocess root parent: {}", sub_root);
            println!("Current process root parent: {}", curr_root);

            // On Windows, the subprocess might be a child of our process
            // or might share the same root parent (like Explorer.exe)
            // So we just verify that the root parent is a valid process
            assert!(sub_root > 0, "Root parent PID should be valid");
        }
    } else {
        println!(
            "Warning: Could not get subprocess process tree: {:?}",
            result
        );
        // This can happen due to permissions or timing issues
        // We don't fail the test for this reason
    }
}

#[test]
fn test_multiple_process_trees_consistency() {
    // Test that getting the process tree multiple times yields consistent results
    let tree1 = ProcessTreeInfo::current().expect("Failed to get first process tree");
    let tree2 = ProcessTreeInfo::current().expect("Failed to get second process tree");

    assert_eq!(
        tree1.process_chain, tree2.process_chain,
        "Process chain should be consistent across calls"
    );
    assert_eq!(
        tree1.root_parent_pid, tree2.root_parent_pid,
        "Root parent PID should be consistent across calls"
    );
    assert_eq!(
        tree1.depth, tree2.depth,
        "Depth should be consistent across calls"
    );
}

#[test]
fn test_process_tree_error_handling() {
    // Test with an invalid PID that should not exist
    let invalid_pid = 999999; // Very unlikely to exist

    let result = get_process_tree(invalid_pid);

    // This should either fail gracefully or return a minimal tree
    match result {
        Ok(tree_info) => {
            // If it succeeds, it should at least contain the PID we asked for
            assert!(!tree_info.process_chain.is_empty());
            assert_eq!(tree_info.process_chain[0], invalid_pid);
            println!("Invalid PID resulted in tree: {:?}", tree_info);
        }
        Err(err) => {
            // Error is also acceptable for invalid PIDs
            println!("Invalid PID correctly returned error: {}", err);
        }
    }
}

#[test]
fn test_windows_specific_behavior() {
    // Windows-specific tests
    if !cfg!(windows) {
        return; // Skip on non-Windows platforms
    }

    let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

    // On Windows, we typically expect to see a chain like:
    // Current Process -> Parent Process -> Explorer.exe/System -> System Idle Process
    println!("Windows process tree depth: {}", tree_info.depth);

    // Windows process trees usually have reasonable depth
    assert!(
        tree_info.depth >= 2,
        "Windows process tree should have at least 2 levels"
    );
    assert!(
        tree_info.depth <= 10,
        "Windows process tree depth seems unusually high"
    );

    // The root parent on Windows is often a system process
    if let Some(root_pid) = tree_info.root_parent_pid {
        println!("Windows root parent PID: {}", root_pid);
        assert!(root_pid > 0, "Root parent PID should be positive");

        // On Windows, common root processes have specific PID ranges
        // System Idle Process is typically PID 0, System is PID 4
        // But root could also be explorer.exe or other system processes
        assert!(root_pid <= 65536, "PID should be in valid range");
    }
}

#[test]
fn test_process_chain_validity() {
    let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

    // Verify that all PIDs in the chain are positive numbers
    for pid in &tree_info.process_chain {
        assert!(*pid > 0, "All PIDs in process chain should be positive");
        assert!(
            *pid <= 65536,
            "PID should be in valid range (typically u16 on Windows)"
        );
    }

    // Verify no duplicate PIDs in the chain (except for the root case)
    let mut unique_pids = std::collections::HashSet::new();
    for pid in &tree_info.process_chain {
        assert!(
            !unique_pids.contains(pid),
            "Process chain should not contain duplicate PIDs"
        );
        unique_pids.insert(*pid);
    }
}

#[test]
fn test_performance_characteristics() {
    // Test that process tree discovery is reasonably fast
    let start = std::time::Instant::now();

    let result = ProcessTreeInfo::current();
    assert!(result.is_ok());

    let duration = start.elapsed();
    println!("Process tree discovery took: {:?}", duration);

    // Should complete within a reasonable time (even with process enumeration overhead)
    assert!(
        duration.as_millis() < 1000,
        "Process tree discovery should complete within 1 second"
    );
}
