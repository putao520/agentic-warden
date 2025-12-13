//! MCP任务启动集成测试
//!
//! 测试MCP服务器启动任务的完整工作流程：
//! - supervisor通过InProcessRegistry注册任务
//! - 多任务并发启动
//! - pwait等待MCP任务完成
//! - 验证与CLI任务的存储隔离

use aiw::registry_factory::{create_cli_registry, create_mcp_registry};
use aiw::task_record::TaskRecord;
use chrono::Utc;
use serial_test::serial;
use std::process::{Command, Stdio};
use std::time::Duration;

/// 测试MCP注册表的基本任务注册流程
///
/// 模拟MCP Server使用InProcessRegistry注册任务
#[test]
#[serial]
fn test_mcp_registry_task_registration() {
    // 1. 获取MCP注册表（模拟MCP Server的行为）
    let mcp_registry = create_mcp_registry();

    // 2. Spawn一个真实进程
    let mut child = Command::new("sleep")
        .arg("0.5")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn sleep");

    let real_pid = child.id();
    println!("✅ MCP spawned process with PID: {}", real_pid);

    // 3. 注册到MCP Registry（模拟supervisor::execute_cli的行为）
    let task = TaskRecord::new(
        Utc::now(),
        format!("mcp-task-{}", real_pid),
        format!("/tmp/mcp-task-{}.log", real_pid),
        Some(std::process::id()),
    );

    mcp_registry
        .register(real_pid, &task)
        .expect("Failed to register MCP task");

    // 4. 验证任务已注册
    let entries = mcp_registry.entries().expect("Failed to get entries");
    assert!(
        entries.iter().any(|e| e.pid == real_pid),
        "MCP registry should contain the task"
    );

    println!("✅ Task registered to InProcessRegistry");

    // 5. 清理
    let _ = child.kill();
    let _ = child.wait();
}

/// 测试MCP并发启动多个任务
///
/// 模拟MCP Server的start_concurrent_tasks行为
#[test]
#[serial]
fn test_mcp_concurrent_task_launching() {
    use aiw::pwait_mode;

    // 1. 获取MCP注册表
    let mcp_registry = create_mcp_registry();

    // 2. 并发启动3个任务（模拟MCP Server行为）
    let mut pids = vec![];
    let mut children = vec![];

    for i in 0..3 {
        let child = Command::new("sleep")
            .arg("0.5")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn sleep");

        let real_pid = child.id();
        pids.push(real_pid);
        children.push(child);

        // 注册到MCP Registry（模拟supervisor行为）
        let task = TaskRecord::new(
            Utc::now(),
            format!("mcp-concurrent-task-{}-{}", i, real_pid),
            format!("/tmp/mcp-concurrent-{}.log", real_pid),
            Some(std::process::id()),
        );

        mcp_registry
            .register(real_pid, &task)
            .expect("Failed to register task");

        println!("✅ MCP started task #{} with PID: {}", i + 1, real_pid);
    }

    // 等待所有进程完成并回收(避免僵尸进程)
    for child in children {
        let _ = child.wait_with_output();
    }

    // 3. 使用pwait等待所有MCP任务（模拟Claude Code执行pwait命令）
    // pwait会自动通过sweep_stale_entries检测进程退出并标记完成
    let result = pwait_mode::run_with_registry(&mcp_registry);

    // 4. 验证所有任务完成
    assert!(result.is_ok(), "pwait should succeed");
    let report = result.unwrap();

    // 注意：total_tasks可能包含之前测试的残留任务（InProcessRegistry是进程内共享的）
    // 所以我们只验证我们启动的任务是否被追踪，不检查总数
    assert!(
        report.total_tasks >= 3,
        "Should track at least 3 MCP tasks (may have residual from previous tests)"
    );

    // 验证所有我们启动的真实PID都被追踪
    let completed_pids: Vec<u32> = report.completed.iter().map(|c| c.pid).collect();
    for &pid in &pids {
        assert!(
            completed_pids.contains(&pid),
            "MCP task PID {} should be tracked",
            pid
        );
    }

    println!(
        "✅ Test passed: MCP launched and tracked {} tasks (total tracked: {})",
        pids.len(),
        report.total_tasks
    );
}

/// 测试MCP和CLI存储隔离在任务启动时的表现
///
/// 验证MCP启动的任务不会被CLI的wait命令看到
#[test]
#[serial]
fn test_mcp_cli_storage_isolation_during_launch() {
    // 1. 获取两个注册表
    let mcp_registry = create_mcp_registry();
    let cli_registry = create_cli_registry().expect("Failed to get CLI registry");

    // 2. MCP启动一个任务
    let mut mcp_child = Command::new("sleep")
        .arg("0.5")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn MCP task");

    let mcp_pid = mcp_child.id();

    let mcp_task = TaskRecord::new(
        Utc::now(),
        format!("mcp-launch-{}", mcp_pid),
        format!("/tmp/mcp-launch-{}.log", mcp_pid),
        Some(std::process::id()),
    );

    mcp_registry
        .register(mcp_pid, &mcp_task)
        .expect("Failed to register MCP task");

    // 3. CLI启动另一个任务
    let mut cli_child = Command::new("sleep")
        .arg("0.5")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn CLI task");

    let cli_pid = cli_child.id();

    let cli_task = TaskRecord::new(
        Utc::now(),
        format!("cli-launch-{}", cli_pid),
        format!("/tmp/cli-launch-{}.log", cli_pid),
        Some(std::process::id()),
    );

    cli_registry
        .register(cli_pid, &cli_task)
        .expect("Failed to register CLI task");

    println!("✅ MCP task PID: {}, CLI task PID: {}", mcp_pid, cli_pid);

    // 4. 验证存储隔离
    let mcp_entries = mcp_registry.entries().expect("Failed to get MCP entries");
    let cli_entries = cli_registry.entries().expect("Failed to get CLI entries");

    // MCP看不到CLI任务
    assert!(
        !mcp_entries.iter().any(|e| e.pid == cli_pid),
        "MCP registry should NOT see CLI task (InProcessRegistry isolation)"
    );

    // CLI看不到MCP任务
    assert!(
        !cli_entries.iter().any(|e| e.pid == mcp_pid),
        "CLI registry should NOT see MCP task (SharedMemory isolation)"
    );

    println!("✅ Storage isolation verified during task launching");

    // 5. 清理
    let _ = mcp_child.kill();
    let _ = cli_child.kill();
    let _ = mcp_child.wait();
    let _ = cli_child.wait();
}

/// 测试MCP任务快速完成的场景
///
/// 验证即使任务在pwait启动前完成，也能正确追踪
#[test]
#[serial]
fn test_mcp_task_quick_completion() {
    use aiw::pwait_mode;

    let mcp_registry = create_mcp_registry();

    // 1. 启动一个立即退出的任务
    let mut child = Command::new("sleep")
        .arg("0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn");

    let pid = child.id();

    let task = TaskRecord::new(
        Utc::now(),
        format!("mcp-quick-{}", pid),
        format!("/tmp/mcp-quick-{}.log", pid),
        Some(std::process::id()),
    );

    mcp_registry
        .register(pid, &task)
        .expect("Failed to register");

    // 2. 等待进程退出并立即标记
    let status = child.wait().expect("Failed to wait");
    let exit_code = status.code().unwrap_or(-1);

    mcp_registry
        .mark_completed(
            pid,
            Some(format!("quick complete: {}", exit_code)),
            Some(exit_code),
            Utc::now(),
        )
        .expect("Failed to mark completed");

    println!("✅ MCP task PID {} completed quickly", pid);

    // 3. pwait应该能看到已完成的任务
    let result = pwait_mode::run_with_registry(&mcp_registry);
    assert!(result.is_ok(), "pwait should handle pre-completed tasks");

    let report = result.unwrap();
    assert!(
        report.completed.iter().any(|c| c.pid == pid),
        "Should track pre-completed MCP task"
    );

    println!("✅ Test passed: pwait tracked pre-completed MCP task");
}

/// 测试MCP注册表的线程安全性
///
/// 模拟MCP Server并发处理多个请求
#[test]
#[serial]
fn test_mcp_registry_concurrent_access() {
    use std::thread;

    let mcp_registry = create_mcp_registry();

    // 启动5个线程，每个线程注册一个任务
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let registry = mcp_registry.clone();
            thread::spawn(move || {
                // Spawn真实进程
                let child = Command::new("sleep")
                    .arg("0.2")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("Failed to spawn");

                let pid = child.id();

                // 注册到MCP Registry
                let task = TaskRecord::new(
                    Utc::now(),
                    format!("concurrent-mcp-{}-{}", i, pid),
                    format!("/tmp/concurrent-mcp-{}.log", pid),
                    Some(std::process::id()),
                );

                registry.register(pid, &task).expect("Failed to register");

                // 不持有Child，让进程自然运行和退出
                std::mem::forget(child);

                pid
            })
        })
        .collect();

    // 等待所有线程完成注册
    let pids: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect();

    // 等待一小段时间让进程退出
    std::thread::sleep(Duration::from_millis(500));

    // 验证所有任务都已注册
    let entries = mcp_registry.entries().expect("Failed to get entries");

    // 所有PID都应该被追踪
    for &pid in &pids {
        let found = entries.iter().any(|e| e.pid == pid);
        assert!(
            found,
            "Thread-registered task PID {} should be in registry",
            pid
        );
        println!("✅ Thread registered MCP task PID: {}", pid);
    }

    println!(
        "✅ Test passed: {} threads safely accessed MCP registry",
        pids.len()
    );
}
