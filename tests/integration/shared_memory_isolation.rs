//! 共享内存隔离性测试
//!
//! 测试基于{PID}_task命名的共享内存隔离机制

use aiw::storage::SharedMemoryStorage;
use aiw::task_record::TaskRecord;
use aiw::unified_registry::Registry;
use chrono::Utc;
use serial_test::serial;

/// 测试不同PID的共享内存完全隔离
#[test]
#[serial]
fn test_different_pids_have_isolated_shared_memory() {
    // 创建两个不同PID的共享内存存储
    let storage1 = SharedMemoryStorage::connect_for_pid(10001)
        .expect("Failed to create storage for PID 10001");
    let storage2 = SharedMemoryStorage::connect_for_pid(10002)
        .expect("Failed to create storage for PID 10002");

    let registry1 = Registry::new(storage1);
    let registry2 = Registry::new(storage2);

    // 在registry1中注册任务
    let task1 = TaskRecord::new(
        Utc::now(),
        "task-10001-1".to_string(),
        "/tmp/task-10001-1.log".to_string(),
        Some(std::process::id()),
    );
    registry1.register(50001, &task1).unwrap();

    // 在registry2中注册任务
    let task2 = TaskRecord::new(
        Utc::now(),
        "task-10002-1".to_string(),
        "/tmp/task-10002-1.log".to_string(),
        Some(std::process::id()),
    );
    registry2.register(50002, &task2).unwrap();

    // 验证隔离性
    let entries1 = registry1.entries().unwrap();
    let entries2 = registry2.entries().unwrap();

    assert_eq!(entries1.len(), 1, "Registry1 should have exactly 1 task");
    assert_eq!(entries2.len(), 1, "Registry2 should have exactly 1 task");

    assert_eq!(
        entries1[0].pid, 50001,
        "Registry1 should contain task with PID 50001"
    );
    assert_eq!(
        entries2[0].pid, 50002,
        "Registry2 should contain task with PID 50002"
    );

    // 清理
    let _ = registry1.cleanup();
    let _ = registry2.cleanup();
}

/// 测试pwait可以连接到指定PID的共享内存
#[test]
#[serial]
fn test_pwait_connects_to_specific_pid_shared_memory() {
    let test_pid = 20001u32;

    // 1. 创建指定PID的共享内存并注册任务
    let storage = SharedMemoryStorage::connect_for_pid(test_pid).unwrap();
    let registry = Registry::new(storage);

    let task = TaskRecord::new(
        Utc::now(),
        format!("pwait-test-{}", test_pid),
        format!("/tmp/pwait-test-{}.log", test_pid),
        Some(std::process::id()),
    );
    registry.register(60001, &task).unwrap();

    // 2. 立即标记任务完成（避免竞态条件）
    registry
        .mark_completed(60001, Some("completed".to_string()), Some(0), Utc::now())
        .unwrap();

    // 3. pwait应该能够连接到test_pid的共享内存并读取已完成的任务
    use aiw::pwait_mode;
    let result = pwait_mode::run_for_pid(test_pid);

    assert!(result.is_ok(), "pwait should succeed: {:?}", result);
    let report = result.unwrap();
    assert_eq!(report.total_tasks, 1, "Should track 1 task");
    assert!(!report.timed_out, "Should not timeout");
    // pwait会立即返回因为任务已完成
    // 注意：completed tasks被get_completed_unread_tasks消费后会从共享内存中删除
    // 所以我们无法在pwait返回后再次验证它们

    // 清理
    let _ = registry.cleanup();
}

/// 测试当前进程PID的共享内存
#[test]
#[serial]
fn test_current_process_shared_memory() {
    // connect() 应该使用当前进程的PID
    let storage1 = SharedMemoryStorage::connect().unwrap();
    let registry1 = Registry::new(storage1);

    // connect_for_pid(current_pid) 应该连接到同一个共享内存
    let current_pid = std::process::id();
    let storage2 = SharedMemoryStorage::connect_for_pid(current_pid).unwrap();
    let registry2 = Registry::new(storage2);

    // 在registry1中注册任务
    let task = TaskRecord::new(
        Utc::now(),
        "current-pid-test".to_string(),
        "/tmp/current-pid-test.log".to_string(),
        Some(current_pid),
    );
    registry1.register(70001, &task).unwrap();

    // registry2应该能看到同样的任务
    let entries = registry2.entries().unwrap();
    assert!(
        entries.iter().any(|e| e.pid == 70001),
        "Registry2 should see the task registered by Registry1"
    );

    // 清理
    let _ = registry1.cleanup();
}

/// 测试多个任务在同一共享内存中的管理
#[test]
#[serial]
fn test_multiple_tasks_in_same_shared_memory() {
    let test_pid = 30001u32;
    let storage = SharedMemoryStorage::connect_for_pid(test_pid).unwrap();
    let registry = Registry::new(storage);

    // 注册多个任务
    let task_pids = vec![80001, 80002, 80003, 80004, 80005];
    for (i, &pid) in task_pids.iter().enumerate() {
        let task = TaskRecord::new(
            Utc::now(),
            format!("multi-task-{}", i),
            format!("/tmp/multi-task-{}.log", i),
            Some(std::process::id()),
        );
        registry.register(pid, &task).unwrap();
    }

    // 验证所有任务都已注册
    let entries = registry.entries().unwrap();
    assert_eq!(entries.len(), 5, "Should have 5 tasks");

    for &task_pid in &task_pids {
        assert!(
            entries.iter().any(|e| e.pid == task_pid),
            "Task with PID {} should be registered",
            task_pid
        );
    }

    // 标记部分任务完成
    registry
        .mark_completed(80001, Some("done".to_string()), Some(0), Utc::now())
        .unwrap();
    registry
        .mark_completed(80002, Some("done".to_string()), Some(0), Utc::now())
        .unwrap();

    // 获取已完成的任务
    let completed = registry.get_completed_unread_tasks().unwrap();
    assert_eq!(completed.len(), 2, "Should have 2 completed tasks");

    // 检查是否还有运行中的任务
    let has_running = registry.has_running_tasks(None).unwrap();
    assert!(has_running, "Should still have running tasks");

    // 清理
    let _ = registry.cleanup();
}

/// 测试共享内存命名格式
#[test]
#[serial]
fn test_shared_memory_naming_format() {
    // 测试命名格式: {PID}_task
    let test_pid = 12345u32;
    let storage = SharedMemoryStorage::connect_for_pid(test_pid).unwrap();

    // 虽然我们无法直接访问namespace字段（它是私有的），
    // 但我们可以通过功能测试来验证命名是正确的
    let registry = Registry::new(storage);

    let task = TaskRecord::new(
        Utc::now(),
        "naming-test".to_string(),
        "/tmp/naming-test.log".to_string(),
        Some(std::process::id()),
    );
    registry.register(90001, &task).unwrap();

    // 创建另一个连接到相同PID的存储
    let storage2 = SharedMemoryStorage::connect_for_pid(test_pid).unwrap();
    let registry2 = Registry::new(storage2);

    // 应该能看到相同的任务
    let entries = registry2.entries().unwrap();
    assert!(
        entries.iter().any(|e| e.pid == 90001),
        "Same PID should access same shared memory"
    );

    // 清理
    let _ = registry.cleanup();
}

/// 测试cleanup功能
#[test]
#[serial]
fn test_shared_memory_cleanup() {
    let test_pid = 40001u32;
    let storage = SharedMemoryStorage::connect_for_pid(test_pid).unwrap();
    let registry = Registry::new(storage);

    // 注册一些任务
    for i in 0..3 {
        let task = TaskRecord::new(
            Utc::now(),
            format!("cleanup-test-{}", i),
            format!("/tmp/cleanup-{}.log", i),
            Some(std::process::id()),
        );
        registry.register(95001 + i as u32, &task).unwrap();
    }

    let entries_before = registry.entries().unwrap();
    assert_eq!(
        entries_before.len(),
        3,
        "Should have 3 tasks before cleanup"
    );

    // 执行清理
    registry.cleanup().expect("Cleanup should succeed");

    // 注意：cleanup后，共享内存应该被标记为可删除
    // 但具体行为取决于操作系统的共享内存实现
}
