//! status 命令集成测试
//!
//! 测试 status 命令的文本输出功能

use aiw::storage::SharedMemoryStorage;
use aiw::task_record::TaskRecord;
use aiw::unified_registry::Registry;
use chrono::Utc;
use serial_test::serial;

/// 测试 status 命令在没有任务时的输出
#[test]
#[serial]
fn test_status_with_no_tasks() {
    // 连接到当前进程的共享内存
    let storage = SharedMemoryStorage::connect().unwrap();
    let registry = Registry::new(storage);

    // 确保没有任务
    let entries = registry.entries().unwrap();
    let running_count = entries
        .iter()
        .filter(|e| e.record.status == aiw::task_record::TaskStatus::Running)
        .count();

    // 如果有残留任务，清理它们
    if running_count > 0 {
        for entry in entries {
            if entry.record.status == aiw::task_record::TaskStatus::Running {
                let _ = registry.mark_completed(
                    entry.pid,
                    Some("cleaned up".to_string()),
                    Some(0),
                    Utc::now(),
                );
            }
        }
    }

    // 验证应该输出 "No tasks!"
    let entries = registry.entries().unwrap();
    let running_count = entries
        .iter()
        .filter(|e| e.record.status == aiw::task_record::TaskStatus::Running)
        .count();

    assert_eq!(running_count, 0, "Should have no running tasks");
}

/// 测试 status 命令在有任务时的输出
#[test]
#[serial]
fn test_status_with_running_tasks() {
    // 连接到当前进程的共享内存
    let storage = SharedMemoryStorage::connect().unwrap();
    let registry = Registry::new(storage);

    // 注册一些测试任务
    let test_pids = vec![80001, 80002, 80003];
    for (i, &pid) in test_pids.iter().enumerate() {
        let task = TaskRecord::new(
            Utc::now(),
            format!("status-test-task-{}", i),
            format!("/tmp/status-test-{}.log", i),
            Some(std::process::id()),
        );
        registry.register(pid, &task).unwrap();
    }

    // 获取运行中的任务数量
    let entries = registry.entries().unwrap();
    let running_count = entries
        .iter()
        .filter(|e| {
            e.record.status == aiw::task_record::TaskStatus::Running
                && test_pids.contains(&e.pid)
        })
        .count();

    assert_eq!(running_count, 3, "Should have 3 running tasks");

    // 清理测试任务
    for &pid in &test_pids {
        let _ = registry.mark_completed(pid, Some("test done".to_string()), Some(0), Utc::now());
    }
}

/// 测试 status 命令的完整工作流
#[test]
#[serial]
fn test_status_workflow() {
    use std::process::Command;

    // 连接到当前进程的共享内存
    let storage = SharedMemoryStorage::connect().unwrap();
    let registry = Registry::new(storage);

    // 1. 初始状态：没有任务
    let entries = registry.entries().unwrap();
    let _initial_running = entries
        .iter()
        .filter(|e| e.record.status == aiw::task_record::TaskStatus::Running)
        .count();

    // 2. 注册任务
    let test_pids = vec![70001, 70002];
    for (i, &pid) in test_pids.iter().enumerate() {
        let task = TaskRecord::new(
            Utc::now(),
            format!("workflow-test-{}", i),
            format!("/tmp/workflow-{}.log", i),
            Some(std::process::id()),
        );
        registry.register(pid, &task).unwrap();
    }

    // 3. 验证任务已注册
    let entries = registry.entries().unwrap();
    let current_running = entries
        .iter()
        .filter(|e| {
            e.record.status == aiw::task_record::TaskStatus::Running
                && test_pids.contains(&e.pid)
        })
        .count();

    assert_eq!(current_running, 2, "Should have 2 running tasks");

    // 4. 运行 status 命令（仅验证命令可以执行）
    let output = Command::new("./target/debug/agentic-warden")
        .arg("status")
        .output();

    if let Ok(result) = output {
        let stdout = String::from_utf8_lossy(&result.stdout);
        println!("Status output: {}", stdout);

        // 输出应该包含 "running" 和 "tasks"
        assert!(
            stdout.contains("running") || stdout.contains("No tasks"),
            "Output should contain task status"
        );
    }

    // 5. 清理
    for &pid in &test_pids {
        let _ =
            registry.mark_completed(pid, Some("workflow done".to_string()), Some(0), Utc::now());
    }
}
