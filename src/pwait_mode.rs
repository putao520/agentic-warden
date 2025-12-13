//! 共享内存任务等待模式
//!
//! pwait_mode用于等待指定PID进程的共享内存任务完成

use crate::{
    config::{MAX_WAIT_DURATION, WAIT_INTERVAL_DEFAULT},
    platform,
    registry_factory::McpRegistry,
    storage::SharedMemoryStorage,
    unified_registry::Registry,
};
use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PWaitError {
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("No tasks to wait for")]
    NoTasks,
}

/// 任务完成信息
#[derive(Debug, Clone)]
pub struct TaskCompletion {
    pub pid: u32,
    pub result: Option<String>,
    pub exit_code: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub log_path: String,
}

/// 等待报告
#[derive(Debug)]
pub struct WaitReport {
    pub total_tasks: usize,
    pub completed: Vec<TaskCompletion>,
    pub timed_out: bool,
    pub duration: Duration,
}

impl WaitReport {
    fn new() -> Self {
        Self {
            total_tasks: 0,
            completed: Vec::new(),
            timed_out: false,
            duration: Duration::from_secs(0),
        }
    }

    fn add_completion(&mut self, completion: TaskCompletion) {
        self.completed.push(completion);
    }

    /// 打印报告
    pub fn print(&self) {
        println!("\n=== Process-Internal Task Wait Report ===");
        println!("Total tasks tracked: {}", self.total_tasks);
        println!("Completed tasks: {}", self.completed.len());
        println!("Duration: {:.1}s", self.duration.as_secs_f64());

        if self.timed_out {
            println!(
                "⚠️  Wait timed out (max {} hours)",
                MAX_WAIT_DURATION.as_secs() / 3600
            );
        }

        if !self.completed.is_empty() {
            println!("\n--- Completed Tasks ---");
            for completion in &self.completed {
                let success = completion.exit_code.map(|c| c == 0).unwrap_or(false);
                let status_icon = if success { "✓" } else { "✗" };

                let duration = if let Some(completed) = completion.completed_at {
                    let dur = completed.signed_duration_since(completion.started_at);
                    format!("{:.1}s", dur.num_milliseconds() as f64 / 1000.0)
                } else {
                    "unknown".to_string()
                };

                println!(
                    "{} PID {} - {} ({})",
                    status_icon,
                    completion.pid,
                    completion.result.as_ref().unwrap_or(&"unknown".to_string()),
                    duration
                );
                println!("  Log: {}", completion.log_path);
            }
        }

        println!("\n=== End of Report ===\n");
    }
}

/// 等待进程内注册表中的所有任务完成
pub fn run_with_registry(registry: &McpRegistry) -> Result<WaitReport, PWaitError> {
    let start = Instant::now();
    let interval = WAIT_INTERVAL_DEFAULT;
    let mut report = WaitReport::new();

    // 检查是否有任务
    let initial_entries = registry
        .entries()
        .map_err(|e| PWaitError::Registry(e.to_string()))?;

    if initial_entries.is_empty() {
        return Err(PWaitError::NoTasks);
    }

    report.total_tasks = initial_entries.len();
    println!(
        "Waiting for {} process-internal tasks to complete...",
        report.total_tasks
    );

    loop {
        let now = Utc::now();

        // 清理过期任务
        let terminate_wrapper = |pid: u32| {
            platform::terminate_process(pid);
            Ok(())
        };
        let _ = registry.sweep_stale_entries(now, platform::process_alive, &terminate_wrapper);

        // 收集已完成的任务
        let completed = registry
            .get_completed_unread_tasks()
            .map_err(|e| PWaitError::Registry(e.to_string()))?;

        for (pid, record) in completed {
            let completion = TaskCompletion {
                pid,
                result: record.result.clone(),
                exit_code: record.exit_code,
                started_at: record.started_at,
                completed_at: record.completed_at,
                log_path: record.log_path.clone(),
            };

            println!(
                "Task PID {} completed: {}",
                pid,
                record.result.as_ref().unwrap_or(&"unknown".to_string())
            );

            report.add_completion(completion);
        }

        // 检查是否还有运行中的任务
        let has_running = registry
            .has_running_tasks(None)
            .map_err(|e| PWaitError::Registry(e.to_string()))?;

        if !has_running {
            println!("All tasks completed!");
            report.duration = start.elapsed();
            return Ok(report);
        }

        // 超时检查
        if start.elapsed() >= MAX_WAIT_DURATION {
            println!(
                "⚠️  Wait timed out after {} hours",
                MAX_WAIT_DURATION.as_secs() / 3600
            );
            report.timed_out = true;
            report.duration = start.elapsed();
            return Ok(report);
        }

        // 等待下一次检查
        std::thread::sleep(interval);
    }
}

/// 等待指定的进程内注册表中的任务
///
/// 这个函数适用于MCP Server等长期运行的进程，用于等待其启动的任务完成
pub async fn wait_async(registry: &McpRegistry) -> Result<WaitReport, PWaitError> {
    let start = Instant::now();
    let interval = WAIT_INTERVAL_DEFAULT;
    let mut report = WaitReport::new();

    // 检查是否有任务
    let initial_entries = registry
        .entries()
        .map_err(|e| PWaitError::Registry(e.to_string()))?;

    if initial_entries.is_empty() {
        return Err(PWaitError::NoTasks);
    }

    report.total_tasks = initial_entries.len();
    tracing::info!(
        "Waiting for {} process-internal tasks to complete...",
        report.total_tasks
    );

    loop {
        let now = Utc::now();

        // 清理过期任务
        let terminate_wrapper = |pid: u32| {
            platform::terminate_process(pid);
            Ok(())
        };
        let _ = registry.sweep_stale_entries(now, platform::process_alive, &terminate_wrapper);

        // 收集已完成的任务
        let completed = registry
            .get_completed_unread_tasks()
            .map_err(|e| PWaitError::Registry(e.to_string()))?;

        for (pid, record) in completed {
            let completion = TaskCompletion {
                pid,
                result: record.result.clone(),
                exit_code: record.exit_code,
                started_at: record.started_at,
                completed_at: record.completed_at,
                log_path: record.log_path.clone(),
            };

            tracing::info!(
                "Task PID {} completed: {}",
                pid,
                record.result.as_ref().unwrap_or(&"unknown".to_string())
            );

            report.add_completion(completion);
        }

        // 检查是否还有运行中的任务
        let has_running = registry
            .has_running_tasks(None)
            .map_err(|e| PWaitError::Registry(e.to_string()))?;

        if !has_running {
            tracing::info!("All tasks completed!");
            report.duration = start.elapsed();
            return Ok(report);
        }

        // 超时检查
        if start.elapsed() >= MAX_WAIT_DURATION {
            tracing::warn!(
                "Wait timed out after {} hours",
                MAX_WAIT_DURATION.as_secs() / 3600
            );
            report.timed_out = true;
            report.duration = start.elapsed();
            return Ok(report);
        }

        // 异步等待下一次检查
        tokio::time::sleep(interval).await;
    }
}

/// 等待指定PID进程的共享内存任务完成
///
/// 这是主要的入口函数，用于从命令行调用
pub fn run_for_pid(pid: u32) -> Result<WaitReport, PWaitError> {
    // 连接到指定PID的共享内存
    let storage = SharedMemoryStorage::connect_for_pid(pid).map_err(|e| {
        PWaitError::Registry(format!(
            "Failed to connect to shared memory for PID {}: {}",
            pid, e
        ))
    })?;

    let registry = Registry::new(storage);

    // 使用现有的等待逻辑
    run_with_registry_generic(&registry, pid)
}

/// 通用的等待逻辑（支持任何类型的Registry）
fn run_with_registry_generic<S: crate::storage::TaskStorage>(
    registry: &Registry<S>,
    target_pid: u32,
) -> Result<WaitReport, PWaitError> {
    let start = Instant::now();
    let interval = WAIT_INTERVAL_DEFAULT;
    let mut report = WaitReport::new();

    // 检查是否有任务
    let initial_entries = registry
        .entries()
        .map_err(|e| PWaitError::Registry(e.to_string()))?;

    if initial_entries.is_empty() {
        return Err(PWaitError::NoTasks);
    }

    report.total_tasks = initial_entries.len();
    println!(
        "Waiting for {} tasks from PID {} to complete...",
        report.total_tasks, target_pid
    );

    loop {
        let now = Utc::now();

        // 清理过期任务
        let terminate_wrapper = |pid: u32| {
            platform::terminate_process(pid);
            Ok(())
        };
        let _ = registry.sweep_stale_entries(now, platform::process_alive, &terminate_wrapper);

        // 收集已完成的任务
        let completed = registry
            .get_completed_unread_tasks()
            .map_err(|e| PWaitError::Registry(e.to_string()))?;

        for (pid, record) in completed {
            let completion = TaskCompletion {
                pid,
                result: record.result.clone(),
                exit_code: record.exit_code,
                started_at: record.started_at,
                completed_at: record.completed_at,
                log_path: record.log_path.clone(),
            };

            println!(
                "Task PID {} completed: {}",
                pid,
                record.result.as_ref().unwrap_or(&"unknown".to_string())
            );

            report.add_completion(completion);
        }

        // 检查是否还有运行中的任务
        let has_running = registry
            .has_running_tasks(None)
            .map_err(|e| PWaitError::Registry(e.to_string()))?;

        if !has_running {
            println!("All tasks completed!");
            report.duration = start.elapsed();
            return Ok(report);
        }

        // 超时检查
        if start.elapsed() >= MAX_WAIT_DURATION {
            println!(
                "⚠️  Wait timed out after {} hours",
                MAX_WAIT_DURATION.as_secs() / 3600
            );
            report.timed_out = true;
            report.duration = start.elapsed();
            return Ok(report);
        }

        // 等待下一次检查
        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_record::TaskRecord;
    use crate::unified_registry::InProcessRegistry;
    use chrono::Utc;

    #[test]
    fn test_wait_with_no_tasks() {
        let registry = InProcessRegistry::new(crate::storage::InProcessStorage::new());
        let result = run_with_registry(&registry);
        assert!(matches!(result, Err(PWaitError::NoTasks)));
    }

    #[test]
    fn test_wait_with_completed_tasks() {
        let registry = InProcessRegistry::new(crate::storage::InProcessStorage::new());

        // 注册一个任务
        let task = TaskRecord::new(
            Utc::now(),
            "123".to_string(),
            "/tmp/test.log".to_string(),
            Some(std::process::id()),
        );
        registry.register(123, &task).unwrap();

        // 立即标记为完成
        registry
            .mark_completed(123, Some("success".to_string()), Some(0), Utc::now())
            .unwrap();

        // 等待应该立即返回
        let result = run_with_registry(&registry);
        assert!(result.is_ok());

        let report = result.unwrap();
        assert_eq!(report.total_tasks, 1);
        assert_eq!(report.completed.len(), 1);
        assert!(!report.timed_out);
    }
}
