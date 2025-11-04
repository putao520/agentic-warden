// Basic integration tests for agentic-warden
//
// These tests verify core functionality without depending on specific
// implementation details that might change.

use std::process::Command;

#[test]
fn test_version_command() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "agentic-warden", "--", "--version"])
        .output()
        .expect("Failed to execute version command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("agentic-warden"));
    assert!(stdout.contains("v0.3.0"));
}

#[test]
fn test_help_command() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "agentic-warden", "--", "--help"])
        .output()
        .expect("Failed to execute help command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Universal AI Agent Manager"));
    assert!(stdout.contains("USAGE"));
}

#[test]
fn test_examples_command() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "agentic-warden", "--", "examples"])
        .output()
        .expect("Failed to execute examples command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EXAMPLES:"));
    assert!(stdout.contains("claude"));
}

#[test]
fn test_invalid_command() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "agentic-warden", "--", "invalid-command"])
        .output()
        .expect("Failed to execute invalid command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should show error about unsupported command
}

// Note: We don't test TUI functionality here because it requires interactive terminal
// TUI tests should be done manually or with specialized TUI testing frameworks