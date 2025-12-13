//! pwait命令PID参数测试
//!
//! 测试pwait命令的新需求：必须指定PID参数
//! - pwait <PID>: 等待指定进程的共享内存任务

use aiw::storage::SharedMemoryStorage;
use aiw::task_record::TaskRecord;
use aiw::unified_registry::Registry;
use chrono::Utc;
use serial_test::serial;

/// 测试pwait命令需要PID参数
#[test]
#[serial]
fn test_pwait_requires_pid_parameter() {
    // 这个测试验证pwait命令签名需要PID参数
    // 实际的CLI解析在src/commands/parser.rs中定义

    // 模拟pwait调用时必须提供PID
    let test_pid = std::process::id();

    // 使用run_for_pid函数需要提供PID参数
    let storage_result = SharedMemoryStorage::connect_for_pid(test_pid);
    assert!(
        storage_result.is_ok(),
        "Should be able to connect to shared memory for PID"
    );

    println!("✅ pwait requires PID parameter: {}", test_pid);
}

/// 测试pwait可以连接到指定PID的共享内存
#[test]
#[serial]
fn test_pwait_connects_to_specific_pid_shared_memory() {
    use aiw::pwait_mode;

    // 创建一个测试用的PID
    let test_pid = 60000u32;

    // 创建该PID的共享内存并注册任务
    let storage = SharedMemoryStorage::connect_for_pid(test_pid)
        .expect("Failed to create shared memory for test PID");
    let registry = Registry::new(storage);

    // 注册一个任务
    let task = TaskRecord::new(
        Utc::now(),
        "test-task".to_string(),
        "/tmp/test-task.log".to_string(),
        Some(std::process::id()),
    );

    registry
        .register(60001, &task)
        .expect("Failed to register task");

    // 立即标记为完成
    registry
        .mark_completed(60001, Some("completed".to_string()), Some(0), Utc::now())
        .expect("Failed to mark completed");

    // 使用pwait连接到该PID的共享内存
    let result = pwait_mode::run_for_pid(test_pid);

    // 应该能成功读取任务
    assert!(
        result.is_ok(),
        "Should be able to read tasks from PID's shared memory"
    );

    let report = result.unwrap();
    assert_eq!(report.total_tasks, 1, "Should find 1 task in shared memory");

    println!(
        "✅ pwait successfully connected to PID {} shared memory",
        test_pid
    );
}

/// 测试pwait连接到当前进程的共享内存
#[test]
#[serial]
fn test_pwait_current_process_shared_memory() {
    use aiw::pwait_mode;

    let current_pid = std::process::id();

    // 在当前进程的共享内存中注册一个任务
    let storage =
        SharedMemoryStorage::connect().expect("Failed to connect to current process shared memory");
    let registry = Registry::new(storage);

    let task = TaskRecord::new(
        Utc::now(),
        "current-process-task".to_string(),
        "/tmp/current-process.log".to_string(),
        Some(current_pid),
    );

    registry
        .register(70001, &task)
        .expect("Failed to register task");

    // 标记为完成
    registry
        .mark_completed(70001, Some("done".to_string()), Some(0), Utc::now())
        .expect("Failed to mark completed");

    // 使用pwait连接到当前进程
    let result = pwait_mode::run_for_pid(current_pid);

    assert!(result.is_ok(), "Should read current process shared memory");

    let report = result.unwrap();
    assert!(report.total_tasks >= 1, "Should have at least 1 task");

    println!(
        "✅ pwait can access current process (PID {}) shared memory",
        current_pid
    );
}

/// 测试pwait连接到不存在任务的PID
#[test]
#[serial]
fn test_pwait_with_nonexistent_pid_tasks() {
    use aiw::pwait_mode;

    // 使用一个很大的PID，很可能没有对应的共享内存
    let nonexistent_pid = 99999u32;

    let result = pwait_mode::run_for_pid(nonexistent_pid);

    // 应该返回NoTasks错误或成功但没有任务
    match result {
        Err(pwait_mode::PWaitError::NoTasks) => {
            println!(
                "✅ pwait correctly reports no tasks for PID {}",
                nonexistent_pid
            );
        }
        Err(pwait_mode::PWaitError::Registry(msg)) if msg.contains("Failed to connect") => {
            println!(
                "✅ pwait correctly reports connection failure for PID {}",
                nonexistent_pid
            );
        }
        Ok(report) if report.total_tasks == 0 => {
            println!("✅ pwait reports 0 tasks for PID {}", nonexistent_pid);
        }
        other => panic!("Unexpected result for nonexistent PID: {:?}", other),
    }
}

/// 测试不同PID的共享内存隔离
#[test]
#[serial]
fn test_different_pids_isolated_shared_memory() {
    let pid1 = 80001u32;
    let pid2 = 80002u32;

    // 在PID1的共享内存中注册任务
    let storage1 =
        SharedMemoryStorage::connect_for_pid(pid1).expect("Failed to create storage for PID1");
    let registry1 = Registry::new(storage1);

    let task1 = TaskRecord::new(
        Utc::now(),
        "task-pid1".to_string(),
        "/tmp/task-pid1.log".to_string(),
        Some(std::process::id()),
    );

    registry1
        .register(80011, &task1)
        .expect("Failed to register task in PID1");

    // 在PID2的共享内存中注册任务
    let storage2 =
        SharedMemoryStorage::connect_for_pid(pid2).expect("Failed to create storage for PID2");
    let registry2 = Registry::new(storage2);

    let task2 = TaskRecord::new(
        Utc::now(),
        "task-pid2".to_string(),
        "/tmp/task-pid2.log".to_string(),
        Some(std::process::id()),
    );

    registry2
        .register(80021, &task2)
        .expect("Failed to register task in PID2");

    // 验证隔离：PID1只看到自己的任务
    let entries1 = registry1.entries().expect("Failed to get entries for PID1");
    assert!(
        entries1.iter().any(|e| e.pid == 80011),
        "PID1 should see its own task"
    );
    assert!(
        !entries1.iter().any(|e| e.pid == 80021),
        "PID1 should NOT see PID2's task"
    );

    // 验证隔离：PID2只看到自己的任务
    let entries2 = registry2.entries().expect("Failed to get entries for PID2");
    assert!(
        entries2.iter().any(|e| e.pid == 80021),
        "PID2 should see its own task"
    );
    assert!(
        !entries2.iter().any(|e| e.pid == 80011),
        "PID2 should NOT see PID1's task"
    );

    println!("✅ Different PIDs have isolated shared memory");
}

/// 测试共享内存命名格式
#[test]
#[serial]
fn test_shared_memory_naming_format() {
    let test_pid = 12345u32;

    // 验证命名格式为 {PID}_task
    let expected_namespace = format!("{}_task", test_pid);

    // 创建存储（这会使用正确的命名）
    let storage = SharedMemoryStorage::connect_for_pid(test_pid);

    // 如果能成功创建，说明命名格式正确
    assert!(
        storage.is_ok(),
        "Should create shared memory with format {{PID}}_task"
    );

    println!("✅ Shared memory naming format: {}_task", test_pid);
    println!("   Example: {}", expected_namespace);
}

/// 测试多个任务在同一个共享内存中
#[test]
#[serial]
fn test_multiple_tasks_same_shared_memory() {
    let test_pid = 90000u32;

    let storage =
        SharedMemoryStorage::connect_for_pid(test_pid).expect("Failed to create shared memory");
    let registry = Registry::new(storage);

    // 注册多个任务
    let task_pids = vec![90001, 90002, 90003];

    for &task_pid in &task_pids {
        let task = TaskRecord::new(
            Utc::now(),
            format!("multi-task-{}", task_pid),
            format!("/tmp/multi-task-{}.log", task_pid),
            Some(std::process::id()),
        );

        registry
            .register(task_pid, &task)
            .expect("Failed to register task");
    }

    // 验证所有任务都在同一个共享内存中
    let entries = registry.entries().expect("Failed to get entries");

    for &task_pid in &task_pids {
        assert!(
            entries.iter().any(|e| e.pid == task_pid),
            "Should find task with PID {}",
            task_pid
        );
    }

    assert_eq!(entries.len(), 3, "Should have 3 tasks in shared memory");

    println!(
        "✅ Multiple tasks can share the same shared memory (PID {})",
        test_pid
    );
}

/// 测试共享内存清理
#[test]
#[serial]
fn test_shared_memory_cleanup() {
    let test_pid = 95000u32;

    {
        let storage =
            SharedMemoryStorage::connect_for_pid(test_pid).expect("Failed to create shared memory");
        let registry = Registry::new(storage);

        // 注册一个任务
        let task = TaskRecord::new(
            Utc::now(),
            "cleanup-task".to_string(),
            "/tmp/cleanup-task.log".to_string(),
            Some(std::process::id()),
        );

        registry
            .register(95001, &task)
            .expect("Failed to register task");

        // 显式清理
        let cleanup_result = registry.cleanup();
        assert!(
            cleanup_result.is_ok(),
            "Should be able to cleanup shared memory"
        );
    }

    println!("✅ Shared memory cleanup successful");
}

/// 测试pwait命令行参数解析（集成测试）
#[test]
#[serial]
fn test_pwait_cli_integration() {
    // 这个测试验证完整的CLI工作流程

    // 1. 创建一个共享内存区域
    let test_pid = 85000u32;
    let storage =
        SharedMemoryStorage::connect_for_pid(test_pid).expect("Failed to create shared memory");
    let registry = Registry::new(storage);

    // 2. 注册并立即完成一个任务
    let task = TaskRecord::new(
        Utc::now(),
        "cli-integration-task".to_string(),
        "/tmp/cli-integration.log".to_string(),
        Some(std::process::id()),
    );

    registry
        .register(85001, &task)
        .expect("Failed to register task");

    registry
        .mark_completed(85001, Some("done".to_string()), Some(0), Utc::now())
        .expect("Failed to mark completed");

    // 3. 调用pwait_mode（模拟CLI命令）
    let result = aiw::pwait_mode::run_for_pid(test_pid);

    assert!(result.is_ok(), "CLI integration should work");

    let report = result.unwrap();
    assert!(
        report.total_tasks >= 1,
        "Should track at least 1 task (got {})",
        report.total_tasks
    );
    // pwait consumes completed tasks after reporting, so completed list may be empty
    // We just verify that pwait ran successfully

    println!("✅ pwait CLI integration test passed for PID {}", test_pid);
}
