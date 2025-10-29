//! Integration tests for wait mode process tree filtering
//!
//! These tests verify that wait mode correctly filters tasks by root parent PID
//! and maintains backward compatibility.

use chrono::Utc;
use agentic_warden::{
    config::{PROCESS_TREE_FEATURE_ENV, is_process_tree_enabled},
    registry::TaskRegistry,
    task_record::{TaskRecord, TaskStatus},
    wait_mode::should_process_task,
};
use std::env;

#[test]
fn test_should_process_task_logic() {
    let base_time = Utc::now();

    // Create test tasks with different root parent PIDs
    let task_root_100 = TaskRecord::new(
        base_time,
        "task1".to_string(),
        "/tmp/task1.log".to_string(),
        Some(1000),
    )
    .with_process_tree(vec![1000, 100], Some(100), 2);

    let task_root_200 = TaskRecord::new(
        base_time,
        "task2".to_string(),
        "/tmp/task2.log".to_string(),
        Some(2000),
    )
    .with_process_tree(vec![2000, 200], Some(200), 2);

    let task_no_root = TaskRecord::new(
        base_time,
        "task3".to_string(),
        "/tmp/task3.log".to_string(),
        Some(3000),
    );

    // Test filtering by root parent 100
    assert!(should_process_task(&task_root_100, Some(100)));
    assert!(!should_process_task(&task_root_200, Some(100)));
    assert!(should_process_task(&task_no_root, Some(100))); // Should include tasks without root info

    // Test filtering by root parent 200
    assert!(!should_process_task(&task_root_100, Some(200)));
    assert!(should_process_task(&task_root_200, Some(200)));
    assert!(should_process_task(&task_no_root, Some(200)));

    // Test no filtering (None = process all tasks)
    assert!(should_process_task(&task_root_100, None));
    assert!(should_process_task(&task_root_200, None));
    assert!(should_process_task(&task_no_root, None));
}

#[test]
fn test_task_registry_integration() {
    // Test that TaskRegistry can store and retrieve tasks with process tree info
    // Use unique namespace with timestamp to avoid conflicts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let namespace = format!("test_task_registry_integration_{}", timestamp);
    let registry = TaskRegistry::connect_test(&namespace).expect("Failed to connect to registry");
    let base_time = Utc::now();

    // Create tasks with process tree info
    let task1 = TaskRecord::new(
        base_time,
        "1234".to_string(),
        "/tmp/1234.log".to_string(),
        Some(1000),
    )
    .with_process_tree(vec![1000, 100], Some(100), 2);

    let task2 = TaskRecord::new(
        base_time,
        "5678".to_string(),
        "/tmp/5678.log".to_string(),
        Some(2000),
    )
    .with_process_tree(vec![2000, 200], Some(200), 2);

    // Register tasks
    registry
        .register(1234, &task1)
        .expect("Failed to register task1");
    registry
        .register(5678, &task2)
        .expect("Failed to register task2");

    // Test retrieval by root parent
    let root_100_tasks = registry
        .get_tasks_by_root_parent(Some(100))
        .expect("Failed to get tasks by root parent 100");

    let root_200_tasks = registry
        .get_tasks_by_root_parent(Some(200))
        .expect("Failed to get tasks by root parent 200");

    let all_tasks = registry
        .get_tasks_by_root_parent(None)
        .expect("Failed to get all tasks");

    assert_eq!(root_100_tasks.len(), 1);
    assert_eq!(root_100_tasks[0].pid, 1234);
    assert_eq!(root_100_tasks[0].record.root_parent_pid, Some(100));

    assert_eq!(root_200_tasks.len(), 1);
    assert_eq!(root_200_tasks[0].pid, 5678);
    assert_eq!(root_200_tasks[0].record.root_parent_pid, Some(200));

    assert!(
        all_tasks.len() >= 2,
        "Should retrieve all tasks including existing ones"
    );

    // Cleanup
    let _ = registry.remove(1234);
    let _ = registry.remove(5678);
}

#[test]
fn test_process_tree_feature_flag() {
    // Save original environment state
    let original = env::var(PROCESS_TREE_FEATURE_ENV);

    // Test explicit enable
    unsafe {
        env::set_var(PROCESS_TREE_FEATURE_ENV, "true");
    }
    assert!(
        is_process_tree_enabled(),
        "Should be enabled when set to true"
    );

    // Test explicit disable
    unsafe {
        env::set_var(PROCESS_TREE_FEATURE_ENV, "false");
    }
    assert!(
        !is_process_tree_enabled(),
        "Should be disabled when set to false"
    );

    // Test default behavior (unset)
    unsafe {
        env::remove_var(PROCESS_TREE_FEATURE_ENV);
    }
    assert!(is_process_tree_enabled(), "Should be enabled by default");

    // Restore original state
    match original {
        Ok(value) => unsafe { env::set_var(PROCESS_TREE_FEATURE_ENV, value) },
        Err(_) => unsafe { env::remove_var(PROCESS_TREE_FEATURE_ENV) },
    }
}

#[test]
fn test_backward_compatibility() {
    let base_time = Utc::now();

    // Create a task without process tree info (like old versions)
    let old_style_task = TaskRecord::new(
        base_time,
        "old_task".to_string(),
        "/tmp/old_task.log".to_string(),
        Some(999),
    );

    // This task should always be processed regardless of filter
    assert!(should_process_task(&old_style_task, Some(100)));
    assert!(should_process_task(&old_style_task, Some(200)));
    assert!(should_process_task(&old_style_task, None));

    // Test with registry
    // Use unique namespace with timestamp to avoid conflicts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let namespace = format!("test_backward_compatibility_{}", timestamp);
    let registry = TaskRegistry::connect_test(&namespace).expect("Failed to connect to registry");
    registry
        .register(999, &old_style_task)
        .expect("Failed to register old-style task");

    // Should be included in all filtered queries
    for root_filter in &[None, Some(100), Some(200)] {
        let tasks = registry
            .get_tasks_by_root_parent(*root_filter)
            .expect("Failed to get tasks");
        assert!(
            tasks.iter().any(|t| t.pid == 999),
            "Old-style task should be included in query with filter {:?}",
            root_filter
        );
    }

    // Cleanup
    let _ = registry.remove(999);
}

#[test]
fn test_task_serialization_round_trip() {
    let base_time = Utc::now();

    let original_task = TaskRecord::new(
        base_time,
        "test_serialization".to_string(),
        "/tmp/test.log".to_string(),
        Some(1000),
    )
    .with_process_tree(vec![1000, 500, 1], Some(1), 3);

    // Serialize to JSON
    let json = serde_json::to_string(&original_task).expect("Failed to serialize");
    println!("Serialized task: {}", json);

    // Deserialize back
    let deserialized: TaskRecord = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify all fields are preserved
    assert_eq!(deserialized.process_chain, vec![1000, 500, 1]);
    assert_eq!(deserialized.root_parent_pid, Some(1));
    assert_eq!(deserialized.process_tree_depth, 3);
    assert_eq!(deserialized.log_id, "test_serialization");
    assert_eq!(deserialized.manager_pid, Some(1000));
    assert_eq!(deserialized.status, TaskStatus::Running);
}

#[test]
fn test_mark_completed_preserves_process_tree() {
    let base_time = Utc::now();

    let mut task = TaskRecord::new(
        base_time,
        "completion_test".to_string(),
        "/tmp/completion.log".to_string(),
        Some(1000),
    )
    .with_process_tree(vec![1000, 100], Some(100), 2);

    let completed_time = Utc::now();
    task = task.mark_completed(Some("test_result".to_string()), Some(0), completed_time);

    // Process tree should be preserved
    assert_eq!(task.process_chain, vec![1000, 100]);
    assert_eq!(task.root_parent_pid, Some(100));
    assert_eq!(task.process_tree_depth, 2);

    // Completion info should be updated
    assert_eq!(task.status, TaskStatus::CompletedButUnread);
    assert_eq!(task.result, Some("test_result".to_string()));
    assert_eq!(task.exit_code, Some(0));
    assert_eq!(task.completed_at, Some(completed_time));
}

#[test]
fn test_cross_process_isolation() {
    // This test simulates what would happen with multiple processes
    // creating tasks with different root parents

    let base_time = Utc::now();

    // Simulate tasks from different terminal sessions
    let task_session1 = TaskRecord::new(
        base_time,
        "session1_task".to_string(),
        "/tmp/session1.log".to_string(),
        Some(1000),
    )
    .with_process_tree(
        vec![1000, 500, 100], // Root parent 100
        Some(100),
        3,
    );

    let task_session2 = TaskRecord::new(
        base_time,
        "session2_task".to_string(),
        "/tmp/session2.log".to_string(),
        Some(2000),
    )
    .with_process_tree(
        vec![2000, 1500, 200], // Root parent 200
        Some(200),
        3,
    );

    // Simulate wait mode from session 1 (root parent 100)
    assert!(should_process_task(&task_session1, Some(100)));
    assert!(!should_process_task(&task_session2, Some(100)));

    // Simulate wait mode from session 2 (root parent 200)
    assert!(!should_process_task(&task_session1, Some(200)));
    assert!(should_process_task(&task_session2, Some(200)));

    // Simulate wait mode without process tree info (old behavior)
    assert!(should_process_task(&task_session1, None));
    assert!(should_process_task(&task_session2, None));
}
