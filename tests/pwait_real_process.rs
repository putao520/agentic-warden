//! pwait 命令真实进程端到端测试
//!
//! 本测试文件测试完整的进程生命周期管理：
//! - Spawn真实的系统进程（如sleep）
//! - 使用真实的PID（通过Child::id()获取）
//! - 注册到Registry并追踪
//! - 等待进程真的退出
//! - 验证pwait能正确等待真实进程完成
//!
//! 与 pwait_command.rs 的区别：
//! - pwait_command.rs: 单元/组件测试，使用假PID测试Registry逻辑
//! - pwait_real_process.rs: 端到端测试，使用真实进程和真实PID

use agentic_warden::registry_factory::RegistryFactory;
use agentic_warden::task_record::TaskRecord;
use chrono::Utc;
use serial_test::serial;
use std::process::{Command, Stdio};
use std::time::Duration;

/// 测试 pwait 等待单个真实的 sleep 进程完成
#[test]
#[serial]
fn test_pwait_waits_for_real_sleep_process() {
    use agentic_warden::pwait_mode;

    let registry = RegistryFactory::instance().get_mcp_registry();

    // 1. Spawn真实的sleep进程（睡眠1秒）
    let mut child = Command::new("sleep")
        .arg("1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn sleep process");

    // 2. 获取真实PID
    let real_pid = child.id();
    println!("✅ Spawned real process with PID: {}", real_pid);

    // 3. 注册到Registry（模拟supervisor的行为）
    let task = TaskRecord::new(
        Utc::now(),
        format!("real-sleep-process-{}", real_pid),
        format!("/tmp/real-test-{}.log", real_pid),
        Some(std::process::id()),
    );
    registry
        .register(real_pid, &task)
        .expect("Failed to register real process");

    // 4. 在后台线程等待进程真的退出
    let registry_clone = registry.clone();
    std::thread::spawn(move || {
        let status = child.wait().expect("Failed to wait for child process");
        let exit_code = status.code().unwrap_or(-1);

        println!(
            "✅ Real process {} exited with code: {}",
            real_pid, exit_code
        );

        // 5. 进程退出后更新Registry
        registry_clone
            .mark_completed(
                real_pid,
                Some(format!("exited with code {}", exit_code)),
                Some(exit_code),
                Utc::now(),
            )
            .expect("Failed to mark completed");
    });

    // 6. 使用pwait等待（应该在1秒内完成）
    let result = pwait_mode::run_with_registry(&registry);

    // 7. 验证pwait看到了真实进程的完成
    assert!(result.is_ok(), "pwait should complete successfully");

    let report = result.unwrap();
    assert!(
        report.completed.iter().any(|c| c.pid == real_pid),
        "pwait should track the real process PID {}",
        real_pid
    );
    assert!(!report.timed_out, "Should not timeout");

    println!("✅ Test passed: pwait tracked real process {}", real_pid);
}

/// 测试 pwait 等待多个真实并发进程
#[test]
#[serial]
fn test_pwait_waits_for_multiple_real_processes() {
    use agentic_warden::pwait_mode;
    use agentic_warden::task_record::TaskStatus;

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

    // 1. Spawn 3个真实的sleep进程
    let mut children = vec![];
    let mut real_pids = vec![];

    for i in 0..3 {
        let child = Command::new("sleep")
            .arg("1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn sleep process");

        let real_pid = child.id();
        real_pids.push(real_pid);

        // 注册到Registry
        let task = TaskRecord::new(
            Utc::now(),
            format!("concurrent-real-process-{}-{}", i, real_pid),
            format!("/tmp/concurrent-real-{}.log", real_pid),
            Some(std::process::id()),
        );
        registry
            .register(real_pid, &task)
            .expect("Failed to register process");

        children.push(child);
        println!("✅ Spawned real process #{} with PID: {}", i + 1, real_pid);
    }

    // 2. 在后台线程等待所有进程退出
    let registry_clone = registry.clone();
    std::thread::spawn(move || {
        for mut child in children {
            let pid = child.id();
            let status = child.wait().expect("Failed to wait for child");
            let exit_code = status.code().unwrap_or(-1);

            println!("✅ Real process {} completed with code {}", pid, exit_code);

            registry_clone
                .mark_completed(
                    pid,
                    Some(format!("exited with code {}", exit_code)),
                    Some(exit_code),
                    Utc::now(),
                )
                .expect("Failed to mark completed");
        }
    });

    // 3. 使用pwait等待所有进程
    let result = pwait_mode::run_with_registry(&registry);

    // 4. 验证所有真实PID都被追踪
    assert!(result.is_ok(), "pwait should complete successfully");

    let report = result.unwrap();
    assert_eq!(report.total_tasks, 3, "Should track 3 real processes");
    assert_eq!(report.completed.len(), 3, "Should complete 3 tasks");
    assert!(!report.timed_out, "Should not timeout");

    // 验证所有真实PID都在完成列表中
    let completed_pids: Vec<u32> = report.completed.iter().map(|c| c.pid).collect();
    for &real_pid in &real_pids {
        assert!(
            completed_pids.contains(&real_pid),
            "Real PID {} should be in completed tasks",
            real_pid
        );
    }

    println!(
        "✅ Test passed: pwait tracked {} real processes: {:?}",
        real_pids.len(),
        real_pids
    );
}

/// 测试存储隔离：MCP注册表使用真实PID，CLI注册表使用另一个真实PID
#[test]
#[serial]
fn test_storage_isolation_with_real_pids() {
    let mcp_registry = RegistryFactory::instance().get_mcp_registry();
    let cli_registry = RegistryFactory::instance()
        .get_cli_registry()
        .expect("Failed to get CLI registry");

    // 1. Spawn一个真实进程给MCP注册表
    let mut mcp_child = Command::new("sleep")
        .arg("0.5")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn MCP process");

    let mcp_real_pid = mcp_child.id();

    // 2. Spawn另一个真实进程给CLI注册表
    let mut cli_child = Command::new("sleep")
        .arg("0.5")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn CLI process");

    let cli_real_pid = cli_child.id();

    println!(
        "✅ Spawned MCP process PID: {}, CLI process PID: {}",
        mcp_real_pid, cli_real_pid
    );

    // 3. 注册到各自的Registry
    let mcp_task = TaskRecord::new(
        Utc::now(),
        format!("mcp-real-{}", mcp_real_pid),
        format!("/tmp/mcp-real-{}.log", mcp_real_pid),
        Some(std::process::id()),
    );
    mcp_registry
        .register(mcp_real_pid, &mcp_task)
        .expect("Failed to register MCP task");

    let cli_task = TaskRecord::new(
        Utc::now(),
        format!("cli-real-{}", cli_real_pid),
        format!("/tmp/cli-real-{}.log", cli_real_pid),
        Some(std::process::id()),
    );
    cli_registry
        .register(cli_real_pid, &cli_task)
        .expect("Failed to register CLI task");

    // 4. 验证存储隔离
    let mcp_entries = mcp_registry.entries().expect("Failed to get MCP entries");
    assert!(
        mcp_entries.iter().any(|e| e.pid == mcp_real_pid),
        "MCP registry should contain MCP real PID {}",
        mcp_real_pid
    );
    assert!(
        !mcp_entries.iter().any(|e| e.pid == cli_real_pid),
        "MCP registry should NOT contain CLI real PID {} (storage isolation!)",
        cli_real_pid
    );

    let cli_entries = cli_registry.entries().expect("Failed to get CLI entries");
    assert!(
        cli_entries.iter().any(|e| e.pid == cli_real_pid),
        "CLI registry should contain CLI real PID {}",
        cli_real_pid
    );
    assert!(
        !cli_entries.iter().any(|e| e.pid == mcp_real_pid),
        "CLI registry should NOT contain MCP real PID {} (storage isolation!)",
        mcp_real_pid
    );

    println!("✅ Storage isolation verified with real PIDs");

    // 5. 清理进程
    let _ = mcp_child.kill();
    let _ = cli_child.kill();
    let _ = mcp_child.wait();
    let _ = cli_child.wait();
}

/// 测试快速完成的进程（进程在pwait启动前就已经退出）
#[test]
#[serial]
fn test_pwait_handles_already_completed_real_process() {
    use agentic_warden::pwait_mode;

    let registry = RegistryFactory::instance().get_mcp_registry();

    // 1. Spawn一个立即退出的进程（sleep 0）
    let mut child = Command::new("sleep")
        .arg("0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn process");

    let real_pid = child.id();
    println!("✅ Spawned fast-exit process with PID: {}", real_pid);

    // 2. 注册到Registry
    let task = TaskRecord::new(
        Utc::now(),
        format!("fast-exit-{}", real_pid),
        format!("/tmp/fast-exit-{}.log", real_pid),
        Some(std::process::id()),
    );
    registry
        .register(real_pid, &task)
        .expect("Failed to register");

    // 3. 等待进程退出并立即标记完成
    let status = child.wait().expect("Failed to wait");
    let exit_code = status.code().unwrap_or(-1);
    registry
        .mark_completed(
            real_pid,
            Some(format!("exited with code {}", exit_code)),
            Some(exit_code),
            Utc::now(),
        )
        .expect("Failed to mark completed");

    println!("✅ Process {} already completed", real_pid);

    // 4. 现在调用pwait（进程已经完成）
    let result = pwait_mode::run_with_registry(&registry);

    // 5. pwait应该立即返回，看到已完成的任务
    assert!(result.is_ok(), "pwait should succeed");

    let report = result.unwrap();
    assert!(
        report.completed.iter().any(|c| c.pid == real_pid),
        "Should track already-completed process"
    );

    println!(
        "✅ Test passed: pwait handled pre-completed process {}",
        real_pid
    );
}

/// 测试进程被kill的情况（非正常退出）
#[test]
#[serial]
fn test_pwait_handles_killed_process() {
    use agentic_warden::pwait_mode;

    let registry = RegistryFactory::instance().get_mcp_registry();

    // 1. Spawn一个长时间运行的进程
    let mut child = Command::new("sleep")
        .arg("60")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn process");

    let real_pid = child.id();
    println!("✅ Spawned long-running process with PID: {}", real_pid);

    // 2. 注册到Registry
    let task = TaskRecord::new(
        Utc::now(),
        format!("kill-test-{}", real_pid),
        format!("/tmp/kill-test-{}.log", real_pid),
        Some(std::process::id()),
    );
    registry
        .register(real_pid, &task)
        .expect("Failed to register");

    // 3. 在后台线程：等待100ms后kill进程
    let registry_clone = registry.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));

        println!("⚡ Killing process {}", real_pid);
        let _ = child.kill();

        let status = child.wait().expect("Failed to wait after kill");
        let exit_code = status.code().unwrap_or(-1);

        println!("✅ Killed process {} exit code: {}", real_pid, exit_code);

        registry_clone
            .mark_completed(
                real_pid,
                Some(format!("killed, exit code {}", exit_code)),
                Some(exit_code),
                Utc::now(),
            )
            .expect("Failed to mark completed");
    });

    // 4. pwait应该能等待被kill的进程
    let result = pwait_mode::run_with_registry(&registry);

    // 5. 验证pwait正确处理
    assert!(result.is_ok(), "pwait should handle killed process");

    let report = result.unwrap();
    assert!(
        report.completed.iter().any(|c| c.pid == real_pid),
        "Should track killed process"
    );

    println!("✅ Test passed: pwait handled killed process {}", real_pid);
}
