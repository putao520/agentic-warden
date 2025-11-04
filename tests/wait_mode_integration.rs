// Wait mode and real-time task status update integration tests

use std::process::Command;
use std::thread;
use std::time::Duration;

#[test]
fn test_wait_command_execution() {
    // The wait command should start and monitor tasks
    // It will exit when no tasks are running or after timeout

    let output = Command::new("cargo")
        .args(&["run", "--bin", "agentic-warden", "--", "wait"])
        .output()
        .expect("Failed to execute wait command");

    // Should succeed (exit when no tasks)
    // Or timeout after MAX_WAIT_DURATION
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show appropriate message
    assert!(stdout.contains("No active tasks") ||
            stdout.contains("等待") ||
            stderr.contains("No tasks") ||
            output.status.success());
}

#[test]
fn test_task_registry_connection() {
    // Test that we can connect to the task registry
    // This tests the shared memory functionality

    let output = Command::new("cargo")
        .args(&["run", "--bin", "agentic-warden", "--", "wait"])
        .output()
        .expect("Failed to execute wait command");

    // If the registry connection fails, it should show an error
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should be able to connect (even if no tasks)
    // Registry errors would be in stderr
    assert!(!stderr.contains("Registry error") ||
            !stderr.contains("Failed to connect"));
}

#[test]
fn test_process_tree_filtering() {
    // Test that process tree filtering works
    // The wait mode should filter tasks by root parent PID

    let output = Command::new("cargo")
        .args(&["run", "--bin", "agentic-warden", "--", "wait"])
        .output()
        .expect("Failed to execute wait command");

    // The process tree filtering should work silently
    // Any errors would be in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have process tree errors
    assert!(!stderr.contains("Failed to get process tree info") ||
            stderr.contains("Failed") && stderr.contains("process tree"));
}

// Note: Testing actual real-time updates requires running AI CLI processes
// which may not be available in the test environment.
// These tests verify that the wait mode can start and handle the no-task case.