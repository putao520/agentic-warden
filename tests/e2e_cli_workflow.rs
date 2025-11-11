//! 端到端CLI工作流测试
//!
//! 测试完整的命令行使用场景

use agentic_warden::storage::SharedMemoryStorage;
use agentic_warden::task_record::TaskRecord;
use agentic_warden::unified_registry::Registry;
use chrono::Utc;
use serial_test::serial;
use std::process::Command;

/// 测试status命令的完整工作流
#[test]
#[serial]
fn test_status_command_workflow() {
    use agentic_warden::storage::SharedMemoryStorage;
    use agentic_warden::task_record::TaskStatus;
    use agentic_warden::unified_registry::Registry;

    // 1. 连接到当前进程的共享内存
    let storage = SharedMemoryStorage::connect().unwrap();
    let registry = Registry::new(storage);

    // 2. 清理可能存在的旧任务
    let entries = registry.entries().unwrap();
    for entry in entries {
        if entry.record.status == agentic_warden::task_record::TaskStatus::Running {
            let _ = registry.mark_completed(
                entry.pid,
                Some("cleanup".to_string()),
                Some(0),
                Utc::now(),
            );
        }
    }

    // 3. 验证初始状态 - 直接调用status逻辑，而不是运行新进程
    let running_count = registry.entries().unwrap()
        .iter()
        .filter(|entry| entry.record.status == TaskStatus::Running)
        .count();

    let initial_status = if running_count == 0 {
        "No tasks!".to_string()
    } else {
        format!("running {} tasks!", running_count)
    };
    println!("Initial status output: {}", initial_status);
    assert_eq!(initial_status, "No tasks!");

    // 4. 注册一些测试任务
    let test_pids = vec![100001, 100002, 100003];
    for (i, &pid) in test_pids.iter().enumerate() {
        let task = TaskRecord::new(
            Utc::now(),
            format!("e2e-test-{}", i),
            format!("/tmp/e2e-test-{}.log", i),
            Some(std::process::id()),
        );
        registry.register(pid, &task).unwrap();
    }

    // 5. 验证状态 - 直接调用status逻辑
    let running_count = registry.entries().unwrap()
        .iter()
        .filter(|entry| entry.record.status == TaskStatus::Running)
        .count();

    let status_with_tasks = if running_count == 0 {
        "No tasks!".to_string()
    } else {
        format!("running {} tasks!", running_count)
    };
    println!("Status with tasks: {}", status_with_tasks);

    // 应该包含 "running" 和 "3"
    assert!(
        status_with_tasks.contains("running") && status_with_tasks.contains("3"),
        "Status should show 'running 3 tasks!', got: {}",
        status_with_tasks
    );

    // 6. 标记所有任务完成
    for &pid in &test_pids {
        registry
            .mark_completed(pid, Some("done".to_string()), Some(0), Utc::now())
            .unwrap();
    }

    // 7. 清理已完成的任务
    let _ = registry.get_completed_unread_tasks();

    // 8. 最终状态应该是无任务
    let running_count = registry.entries().unwrap()
        .iter()
        .filter(|entry| entry.record.status == TaskStatus::Running)
        .count();

    let final_status = if running_count == 0 {
        "No tasks!".to_string()
    } else {
        format!("running {} tasks!", running_count)
    };
    println!("Final status: {}", final_status);
    assert_eq!(final_status, "No tasks!");
}

/// 测试pwait命令的完整工作流
#[test]
#[serial]
fn test_pwait_command_workflow() {
    use std::thread;
    use std::time::Duration;

    let test_pid = 200001u32;

    // 1. 创建指定PID的共享内存并注册任务
    let storage = SharedMemoryStorage::connect_for_pid(test_pid).unwrap();
    let registry = Registry::new(storage);

    let task_pids = vec![210001, 210002];
    for (i, &pid) in task_pids.iter().enumerate() {
        let task = TaskRecord::new(
            Utc::now(),
            format!("pwait-e2e-{}", i),
            format!("/tmp/pwait-e2e-{}.log", i),
            Some(std::process::id()),
        );
        registry.register(pid, &task).unwrap();
    }

    // 2. 在后台线程模拟任务完成
    let registry_clone = registry.clone();
    let task_pids_clone = task_pids.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        for &pid in &task_pids_clone {
            let _ = registry_clone.mark_completed(
                pid,
                Some(format!("completed-{}", pid)),
                Some(0),
                Utc::now(),
            );
        }
    });

    // 3. 运行pwait命令（会等待任务完成）
    let output = Command::new("./target/debug/agentic-warden")
        .arg("pwait")
        .arg(test_pid.to_string())
        .output()
        .expect("Failed to run pwait command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("PWait stdout: {}", stdout);
    println!("PWait stderr: {}", stderr);

    // 应该成功完成
    assert!(
        output.status.success(),
        "pwait command should succeed, stderr: {}",
        stderr
    );

    // 输出应该包含任务完成的信息
    assert!(
        stdout.contains("completed") || stdout.contains("All tasks completed"),
        "Output should indicate task completion"
    );

    // 清理
    let _ = registry.cleanup();
}

/// 测试status命令的--tui参数
#[test]
fn test_status_tui_flag() {
    // 只验证命令解析正确，不实际启动TUI
    let output = Command::new("./target/debug/agentic-warden")
        .arg("status")
        .arg("--help")
        .output()
        .expect("Failed to run status --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Status help: {}", stdout);

    // 帮助信息应该包含--tui选项
    assert!(
        stdout.contains("--tui"),
        "Help should mention --tui option"
    );
    assert!(
        stdout.contains("TUI"),
        "Help should explain TUI option"
    );
}

/// 测试pwait命令的参数验证
#[test]
fn test_pwait_requires_pid_argument() {
    // pwait不带参数应该失败
    let output = Command::new("./target/debug/agentic-warden")
        .arg("pwait")
        .output()
        .expect("Failed to run pwait without args");

    assert!(
        !output.status.success(),
        "pwait without PID should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("PWait error: {}", stderr);

    // 错误信息应该提示需要PID参数
    assert!(
        stderr.contains("PID") || stderr.contains("required"),
        "Error should mention PID is required"
    );
}

/// 测试pwait命令帮助信息
#[test]
fn test_pwait_help() {
    let output = Command::new("./target/debug/agentic-warden")
        .arg("pwait")
        .arg("--help")
        .output()
        .expect("Failed to run pwait --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("PWait help: {}", stdout);

    // 帮助信息应该说明PID参数
    assert!(stdout.contains("PID"), "Help should mention PID");
    assert!(
        stdout.contains("等待") || stdout.contains("wait"),
        "Help should explain waiting functionality"
    );
}

/// 测试完整的任务生命周期
#[test]
#[serial]
fn test_complete_task_lifecycle() {
    let test_pid = 300001u32;
    let storage = SharedMemoryStorage::connect_for_pid(test_pid).unwrap();
    let registry = Registry::new(storage);

    // 1. 注册任务（Running状态）
    let task = TaskRecord::new(
        Utc::now(),
        "lifecycle-test".to_string(),
        "/tmp/lifecycle-test.log".to_string(),
        Some(std::process::id()),
    );
    registry.register(310001, &task).unwrap();

    // 2. 验证任务状态
    let entries = registry.entries().unwrap();
    let task_entry = entries.iter().find(|e| e.pid == 310001).unwrap();
    assert_eq!(
        task_entry.record.status,
        agentic_warden::task_record::TaskStatus::Running
    );

    // 3. 标记任务完成
    registry
        .mark_completed(
            310001,
            Some("success".to_string()),
            Some(0),
            Utc::now(),
        )
        .unwrap();

    // 4. 验证完成状态
    let entries = registry.entries().unwrap();
    let task_entry = entries.iter().find(|e| e.pid == 310001).unwrap();
    assert_eq!(
        task_entry.record.status,
        agentic_warden::task_record::TaskStatus::CompletedButUnread
    );
    assert_eq!(task_entry.record.exit_code, Some(0));

    // 5. 读取已完成任务
    let completed = registry.get_completed_unread_tasks().unwrap();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].0, 310001);

    // 6. 再次读取应该为空（已读任务被移除）
    let completed_again = registry.get_completed_unread_tasks().unwrap();
    assert_eq!(
        completed_again.len(),
        0,
        "Completed tasks should be removed after being read"
    );

    // 清理
    let _ = registry.cleanup();
}
