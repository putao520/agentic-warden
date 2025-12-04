//! pwait 命令集成测试
//!
//! 测试 pwait 命令的完整工作流程，包括：
//! - 基本命令执行
//! - 与 MCP 注册表的集成
//! - 存储隔离验证

use aiw::registry_factory::RegistryFactory;
use aiw::task_record::TaskRecord;
use chrono::Utc;
use serial_test::serial;
use std::time::Duration;

/// 测试 pwait_mode 在没有任务时的行为
#[test]
#[serial]
fn test_pwait_with_no_tasks() {
    use aiw::pwait_mode;

    let registry = RegistryFactory::instance().get_mcp_registry();

    // 如果注册表中有之前测试残留的任务，确保它们都不是running状态
    // pwait只会等待running任务

    // 直接调用 pwait_mode
    let result = pwait_mode::run_with_registry(&registry);

    // 如果没有任务，应该返回NoTasks错误
    // 如果有已完成的任务，应该成功返回
    match result {
        Err(pwait_mode::PWaitError::NoTasks) => {
            // 这是预期的 - 没有任务
        }
        Ok(report) => {
            // 如果有任务，应该都已完成（没有running任务）
            assert!(!report.timed_out, "Should not timeout");
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

/// 测试 pwait_mode 等待已完成的任务
///
/// 注意：不使用Command spawn新进程，因为InProcessStorage是进程内独享的
#[test]
#[serial]
fn test_pwait_with_completed_task() {
    use aiw::pwait_mode;
    use aiw::task_record::TaskStatus;

    let registry = RegistryFactory::instance().get_mcp_registry();

    // 清理任何现有的任务
    let entries = registry.entries().unwrap();
    for entry in entries {
        if entry.record.status == TaskStatus::Running {
            let _ = registry.mark_completed(
                entry.pid,
                Some("cleanup".to_string()),
                Some(0),
                Utc::now(),
            );
        }
    }
    // 清理已完成的任务
    let _ = registry.get_completed_unread_tasks();

    // 注册一个测试任务
    let test_pid = 99999;
    let task = TaskRecord::new(
        Utc::now(),
        "test-task-001".to_string(),
        "/tmp/test-001.log".to_string(),
        Some(std::process::id()),
    );

    registry
        .register(test_pid, &task)
        .expect("Failed to register task");

    // 立即标记为完成
    registry
        .mark_completed(
            test_pid,
            Some("test completed".to_string()),
            Some(0),
            Utc::now(),
        )
        .expect("Failed to mark task as completed");

    // 直接调用 pwait_mode（在同一进程内）
    let result = pwait_mode::run_with_registry(&registry);

    // 应该成功完成
    assert!(result.is_ok(), "pwait should complete successfully");

    let report = result.unwrap();
    assert_eq!(report.total_tasks, 1, "Should track 1 task");
    assert_eq!(report.completed.len(), 1, "Should complete 1 task");
    assert_eq!(report.completed[0].pid, test_pid);
    assert!(!report.timed_out, "Should not timeout");
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

    // 验证 MCP 注册表能看到 MCP 任务
    let mcp_entries = mcp_registry.entries().expect("Failed to get MCP entries");
    assert!(
        mcp_entries.iter().any(|e| e.pid == mcp_pid),
        "MCP registry should contain MCP task (pid={})",
        mcp_pid
    );
    // 验证 MCP 注册表看不到 CLI 任务
    assert!(
        !mcp_entries.iter().any(|e| e.pid == cli_pid),
        "MCP registry should NOT contain CLI task (pid={})",
        cli_pid
    );

    // 验证 CLI 注册表能看到 CLI 任务
    let cli_entries = cli_registry.entries().expect("Failed to get CLI entries");
    assert!(
        cli_entries.iter().any(|e| e.pid == cli_pid),
        "CLI registry should contain CLI task (pid={})",
        cli_pid
    );
    // 验证 CLI 注册表看不到 MCP 任务
    assert!(
        !cli_entries.iter().any(|e| e.pid == mcp_pid),
        "CLI registry should NOT contain MCP task (pid={})",
        mcp_pid
    );

    // 任务使用唯一PID，不会影响其他测试
}

/// 测试 pwait_mode 不会看到 CLI 任务（存储隔离）
#[test]
#[serial]
fn test_pwait_does_not_see_cli_tasks() {
    use aiw::pwait_mode;

    let mcp_registry = RegistryFactory::instance().get_mcp_registry();
    let cli_registry = RegistryFactory::instance()
        .get_cli_registry()
        .expect("Failed to get CLI registry");

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

    // 验证CLI注册表能看到任务
    let cli_entries = cli_registry.entries().expect("Failed to get CLI entries");
    assert!(
        cli_entries.iter().any(|e| e.pid == cli_pid),
        "CLI registry should contain the task"
    );

    // 验证MCP注册表看不到CLI任务
    let mcp_entries = mcp_registry.entries().expect("Failed to get MCP entries");
    assert!(
        !mcp_entries.iter().any(|e| e.pid == cli_pid),
        "MCP registry should NOT contain CLI task (storage isolation)"
    );

    // pwait应该报告没有MCP任务（因为只有CLI任务）
    let result = pwait_mode::run_with_registry(&mcp_registry);

    match result {
        Err(pwait_mode::PWaitError::NoTasks) => {
            // 这是预期的 - MCP注册表中没有任务
        }
        Ok(report) => {
            // 如果返回成功，应该不包含CLI任务
            assert!(
                !report.completed.iter().any(|c| c.pid == cli_pid),
                "pwait should not see CLI tasks"
            );
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

/// 测试 pwait_mode 等待多个并发任务
///
/// 注意：不使用Command spawn新进程，因为InProcessStorage是进程内独享的
#[test]
#[serial]
fn test_pwait_with_multiple_concurrent_tasks() {
    use aiw::pwait_mode;
    use aiw::task_record::TaskStatus;

    let registry = RegistryFactory::instance().get_mcp_registry();

    // 清理任何现有的任务
    let entries = registry.entries().unwrap();
    for entry in entries {
        if entry.record.status == TaskStatus::Running {
            let _ = registry.mark_completed(
                entry.pid,
                Some("cleanup".to_string()),
                Some(0),
                Utc::now(),
            );
        }
    }
    // 清理已完成的任务
    let _ = registry.get_completed_unread_tasks();

    // 注册3个任务
    let pids = vec![55551, 55552, 55553];
    for (i, &pid) in pids.iter().enumerate() {
        let task = TaskRecord::new(
            Utc::now(),
            format!("concurrent-task-{}", i),
            format!("/tmp/concurrent-{}.log", i),
            Some(std::process::id()),
        );
        registry
            .register(pid, &task)
            .expect("Failed to register task");
    }

    // 在后台线程中模拟任务完成
    let registry_clone = registry.clone();
    let pids_clone = pids.clone();
    std::thread::spawn(move || {
        // 等待100ms后标记所有任务完成
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

    // 直接调用 pwait_mode（在同一进程内）
    let result = pwait_mode::run_with_registry(&registry);

    // 应该成功完成
    assert!(result.is_ok(), "pwait should complete successfully");

    let report = result.unwrap();
    assert_eq!(report.total_tasks, 3, "Should track 3 tasks");
    assert_eq!(report.completed.len(), 3, "Should complete 3 tasks");
    assert!(!report.timed_out, "Should not timeout");

    // 验证所有PID都完成了
    let completed_pids: Vec<u32> = report.completed.iter().map(|c| c.pid).collect();
    for &pid in &pids {
        assert!(
            completed_pids.contains(&pid),
            "PID {} should be in completed tasks",
            pid
        );
    }
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
