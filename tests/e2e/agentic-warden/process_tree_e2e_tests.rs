//! Process Tree E2E Tests
//! Tests REQ-001: AI CLI进程树追踪

mod common;

use agentic_warden::core::process_tree;
use common::{
    detect_root_ai_cli, get_process_env_vars, get_process_tree, get_shared_memory, set_process_env,
    spawn_ai_cli,
};
use anyhow::Result;
use serial_test::serial;
use std::collections::HashMap;

#[tokio::test]
#[serial]
async fn test_process_tree_isolation_multi_ai_cli() -> Result<()> {
    let claude = spawn_ai_cli("claude", "task1").await?;
    let codex = spawn_ai_cli("codex", "task2").await?;
    let gemini = spawn_ai_cli("gemini", "task3").await?;

    let tree = get_process_tree().await?;
    assert!(tree.groups.len() >= 3, "Expected at least 3 process groups, got {}", tree.groups.len());

    let claude_memory = get_shared_memory(claude.pid as i32).await;
    let codex_memory = get_shared_memory(codex.pid as i32).await;
    let gemini_memory = get_shared_memory(gemini.pid as i32).await;

    assert_ne!(claude_memory.namespace, codex_memory.namespace);
    assert_ne!(codex_memory.namespace, gemini_memory.namespace);
    assert_ne!(claude_memory.namespace, gemini_memory.namespace);

    // Ensure we can attach env vars per process without cross-contamination
    set_process_env(
        claude.pid,
        HashMap::from([("CLAUDE_ONLY".into(), "1".into())]),
    );
    let claude_env = get_process_env_vars(claude.pid as i32).await;
    assert_eq!(claude_env.get("CLAUDE_ONLY").map(String::as_str), Some("1"));
    let codex_env = get_process_env_vars(codex.pid as i32).await;
    assert!(!codex_env.contains_key("CLAUDE_ONLY"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_root_ai_cli_detection() -> Result<()> {
    let task = spawn_ai_cli("codex", "deep-tree").await?;
    let root = detect_root_ai_cli(task.pid as i32).await;

    let root = root.expect("root ai cli should be detected");
    assert_eq!(root.ai_type, "codex");
    assert_eq!(root.pid, task.pid);
    assert!(
        !root.process_name.is_empty(),
        "mock ai cli should include process name"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_cross_platform_process_detection() -> Result<()> {
    let current_pid = std::process::id();
    let tree = process_tree::get_process_tree(current_pid)?;
    assert!(
        !tree.process_chain.is_empty(),
        "process chain should never be empty"
    );
    assert_eq!(tree.process_chain[0], current_pid);

    #[cfg(target_os = "windows")]
    {
        assert!(tree.root_parent_pid.is_some(), "windows root parent detected");
    }

    #[cfg(not(target_os = "windows"))]
    {
        assert!(tree.root_parent_pid.is_some(), "unix root parent detected");
    }

    Ok(())
}
