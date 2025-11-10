//! pwait 命令集成测试
//!
//! 测试 pwait 命令的完整工作流程，包括：
//! - 基本命令执行
//! - 与 MCP 注册表的集成
//! - 存储隔离验证

use agentic_warden::registry_factory::RegistryFactory;
use agentic_warden::task_record::TaskRecord;
use chrono::Utc;
use serial_test::serial;
use std::process::Command;
use std::time::Duration;

/// 测试 pwait 命令在没有任务时的行为
#[test]
#[serial]
fn test_pwait_with_no_tasks() {
    // 注意：即使注册表中有任务，pwait也会报告"no tasks"如果没有正在运行的任务

    // 执行 pwait 命令
    let output = Command::new(env!("CARGO_BIN_EXE_agentic-warden"))
        .arg("pwait")
        .output()
        .expect("Failed to execute pwait command");

    // 应该返回成功（退出码0）
    assert_eq!(output.status.code(), Some(0));

    // 应该输出 "No MCP tasks to wait for"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No MCP tasks to wait for"),
        "Expected 'No MCP tasks to wait for', got: {}",
        stderr
    );
}

/// 测试 pwait 命令等待已完成的任务
#[test]
#[serial]
fn test_pwait_with_completed_task() {
    let registry = RegistryFactory::instance().get_mcp_registry();

    // 注意：使用唯一的PID，避免与其他测试冲突

    // 注册一个测试任务
    let test_pid = 99999;
    let task = TaskRecord::new(
        Utc::now(),
        "test-task-001".to_string(),
        "/tmp/test-001.log".to_string(),
        Some(std::process::id()),
    );

    registry.register(test_pid, &task).expect("Failed to register task");

    // 立即标记为完成
    registry
        .mark_completed(
            test_pid,
            Some("test completed".to_string()),
            Some(0),
            Utc::now(),
        )
        .expect("Failed to mark task as completed");

    // 在子线程中执行 pwait（避免阻塞测试）
    let handle = std::thread::spawn(|| {
        Command::new(env!("CARGO_BIN_EXE_agentic-warden"))
            .arg("pwait")
            .output()
            .expect("Failed to execute pwait command")
    });

    // 等待命令完成（最多5秒）
    let output = handle.join().expect("Thread panicked");

    // 应该返回成功
    assert_eq!(output.status.code(), Some(0));

    // 应该包含任务完成信息
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Task PID") || stdout.contains("completed"),
        "Expected task completion message, got: {}",
        stdout
    );

    // 任务会自动过期或在下次sweep时清理
}

/// 测试 MCP 注册表和 CLI 注册表的存储隔离
#[test]
#[serial]
fn test_storage_isolation_mcp_vs_cli() {
    let mcp_registry = RegistryFactory::instance().get_mcp_registry();
    let cli_registry = RegistryFactory::instance()
        .get_cli_registry()
        .expect("Failed to get CLI registry");

    // 注意：使用唯一的PID，避免与其他测试冲突

    // 在 MCP 注册表中注册一个任务
    let mcp_pid = 88888;
    let mcp_task = TaskRecord::new(
        Utc::now(),
        "mcp-task".to_string(),
        "/tmp/mcp-task.log".to_string(),
        Some(std::process::id()),
    );
    mcp_registry
        .register(mcp_pid, &mcp_task)
        .expect("Failed to register MCP task");

    // 在 CLI 注册表中注册一个任务
    let cli_pid = 77777;
    let cli_task = TaskRecord::new(
        Utc::now(),
        "cli-task".to_string(),
        "/tmp/cli-task.log".to_string(),
        Some(std::process::id()),
    );
    cli_registry
        .register(cli_pid, &cli_task)
        .expect("Failed to register CLI task");

    // 验证 MCP 注册表只能看到 MCP 任务
    let mcp_entries = mcp_registry.entries().expect("Failed to get MCP entries");
    assert_eq!(mcp_entries.len(), 1, "MCP registry should have exactly 1 task");
    assert_eq!(mcp_entries[0].pid, mcp_pid);

    // 验证 CLI 注册表只能看到 CLI 任务
    let cli_entries = cli_registry.entries().expect("Failed to get CLI entries");
    assert_eq!(cli_entries.len(), 1, "CLI registry should have exactly 1 task");
    assert_eq!(cli_entries[0].pid, cli_pid);

    // 任务使用唯一PID，不会影响其他测试
}

/// 测试 pwait 命令不会看到 CLI 任务
#[test]
#[serial]
fn test_pwait_does_not_see_cli_tasks() {
    let mcp_registry = RegistryFactory::instance().get_mcp_registry();
    let cli_registry = RegistryFactory::instance()
        .get_cli_registry()
        .expect("Failed to get CLI registry");

    // 注意：使用唯一的PID，避免与其他测试冲突

    // 只在 CLI 注册表中注册任务
    let cli_pid = 66666;
    let cli_task = TaskRecord::new(
        Utc::now(),
        "cli-only-task".to_string(),
        "/tmp/cli-only.log".to_string(),
        Some(std::process::id()),
    );
    cli_registry
        .register(cli_pid, &cli_task)
        .expect("Failed to register CLI task");

    // 标记 CLI 任务为运行中
    // (默认状态就是 Running)

    // 执行 pwait 命令（应该报告没有任务）
    let output = Command::new(env!("CARGO_BIN_EXE_agentic-warden"))
        .arg("pwait")
        .output()
        .expect("Failed to execute pwait command");

    // 应该返回成功
    assert_eq!(output.status.code(), Some(0));

    // 应该报告没有 MCP 任务
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No MCP tasks to wait for"),
        "pwait should not see CLI tasks, got: {}",
        stderr
    );

    // 任务使用唯一PID，不影响其他测试
}

/// 测试多个并发任务的等待
#[test]
#[serial]
fn test_pwait_with_multiple_concurrent_tasks() {
    let registry = RegistryFactory::instance().get_mcp_registry();

    // 注意：使用唯一的PID，避免与其他测试冲突

    // 注册3个任务
    let pids = vec![55551, 55552, 55553];
    for (i, &pid) in pids.iter().enumerate() {
        let task = TaskRecord::new(
            Utc::now(),
            format!("concurrent-task-{}", i),
            format!("/tmp/concurrent-{}.log", i),
            Some(std::process::id()),
        );
        registry.register(pid, &task).expect("Failed to register task");
    }

    // 在后台线程中模拟任务完成
    let registry_clone = registry.clone();
    let pids_clone = pids.clone();
    std::thread::spawn(move || {
        // 等待1秒后标记所有任务完成
        std::thread::sleep(Duration::from_millis(100));
        for &pid in &pids_clone {
            let _ = registry_clone.mark_completed(
                pid,
                Some(format!("task {} completed", pid)),
                Some(0),
                Utc::now(),
            );
        }
    });

    // 执行 pwait 命令
    let output = Command::new(env!("CARGO_BIN_EXE_agentic-warden"))
        .arg("pwait")
        .output()
        .expect("Failed to execute pwait command");

    // 应该返回成功
    assert_eq!(output.status.code(), Some(0));

    // 应该包含任务完成报告
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("tasks") || stdout.contains("completed"),
        "Expected completion report, got: {}",
        stdout
    );

    // 任务使用唯一PID，不影响其他测试
}

/// 测试 RegistryFactory 的线程安全性
#[test]
#[serial]
fn test_registry_factory_thread_safety() {
    use std::thread;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let factory = RegistryFactory::instance();
                let mcp_reg = factory.get_mcp_registry();

                // 每个线程注册一个任务
                let pid = 40000 + i;
                let task = TaskRecord::new(
                    Utc::now(),
                    format!("thread-task-{}", i),
                    format!("/tmp/thread-{}.log", i),
                    Some(std::process::id()),
                );

                mcp_reg.register(pid, &task).expect("Failed to register");
                pid
            })
        })
        .collect();

    // 等待所有线程完成
    let pids: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect();

    // 验证所有任务都已注册
    let registry = RegistryFactory::instance().get_mcp_registry();
    let entries = registry.entries().expect("Failed to get entries");

    // 应该至少有10个任务（可能有其他测试残留的）
    assert!(
        entries.len() >= 10,
        "Expected at least 10 tasks, got {}",
        entries.len()
    );

    // 验证所有PID都在注册表中
    let registered_pids: Vec<_> = entries.iter().map(|e| e.pid).collect();
    for &pid in &pids {
        assert!(
            registered_pids.contains(&pid),
            "PID {} not found in registry",
            pid
        );
    }

    // 任务使用唯一PID，不影响其他测试
}
