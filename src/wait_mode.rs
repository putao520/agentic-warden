use crate::config::{
    LEGACY_WAIT_INTERVAL_ENV, MAX_WAIT_DURATION, WAIT_INTERVAL_DEFAULT, WAIT_INTERVAL_ENV,
};
use crate::core::models::ProcessTreeInfo;
use crate::core::process_tree::ProcessTreeError;
use crate::error::RegistryError;
use crate::logging::warn;
use crate::platform;
use crate::registry_factory::RegistryFactory;
use crate::storage::{CleanupReason, RegistryEntry};
use crate::task_record::TaskRecord;
use crate::task_record::TaskStatus;
use chrono::{DateTime, Local, Utc};
use std::collections::HashSet;
use std::fmt::Write;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WaitError {
    #[error("registry error: {0}")]
    Registry(#[from] RegistryError),
    #[error("process tree error: {0}")]
    ProcessTree(#[from] ProcessTreeError),
}

pub fn run() -> Result<(), WaitError> {
    let factory = RegistryFactory::instance();
    let cli_registry = factory.get_cli_registry()?;
    let mcp_registry = factory.get_mcp_registry();
    let interval = read_interval();
    let start = Instant::now();
    let mut processed_pids: HashSet<u32> = HashSet::new();
    let mut report = TaskReport::new();

    // Get current process root parent for task filtering (core functionality)
    let current_root_parent = match ProcessTreeInfo::current() {
        Ok(tree_info) => tree_info.get_ai_cli_root(),
        Err(err) => {
            warn(format!("Failed to get process tree info: {}", err));
            None
        }
    };

    let terminate_wrapper = |pid: u32| {
        platform::terminate_process(pid);
        Ok(())
    };

    loop {
        let now = chrono::Utc::now();

        // Process CLI registry tasks
        let cli_cleanups = cli_registry.sweep_stale_entries(
            now,
            platform::process_alive,
            &terminate_wrapper,
        )?;
        for event in cli_cleanups {
            if event.reason == CleanupReason::Timeout {
                continue;
            }

            // Filter tasks by root parent PID
            if !should_process_task(&event.record, current_root_parent) {
                continue;
            }

            let pid = event._pid;
            if processed_pids.insert(pid) {
                let completion = TaskCompletion::from_record(pid, event.record);
                emit_realtime_update(&completion);
                report.add_completion(completion);
            }
        }

        for (pid, record) in cli_registry.get_completed_unread_tasks()? {
            // Filter tasks by root parent PID
            if !should_process_task(&record, current_root_parent) {
                continue;
            }

            if processed_pids.insert(pid) {
                let completion = TaskCompletion::from_record(pid, record);
                emit_realtime_update(&completion);
                report.add_completion(completion);
            }
        }

        // Process MCP registry tasks (without root parent filtering for cross-process)
        let mcp_cleanups = mcp_registry.sweep_stale_entries(
            now,
            platform::process_alive,
            &terminate_wrapper,
        )?;
        for event in mcp_cleanups {
            if event.reason == CleanupReason::Timeout {
                continue;
            }

            let pid = event._pid;
            if processed_pids.insert(pid) {
                let completion = TaskCompletion::from_record(pid, event.record);
                emit_realtime_update(&completion);
                report.add_completion(completion);
            }
        }

        for (pid, record) in mcp_registry.get_completed_unread_tasks()? {
            if processed_pids.insert(pid) {
                let completion = TaskCompletion::from_record(pid, record);
                emit_realtime_update(&completion);
                report.add_completion(completion);
            }
        }

        // Check both registries for running tasks
        let cli_entries = cli_registry.entries()?;
        let mcp_entries = mcp_registry.entries()?;

        let cli_has_running = cli_entries.iter().any(|entry| {
            entry.record.status == TaskStatus::Running
                && should_process_task(&entry.record, current_root_parent)
        });

        let mcp_has_running = mcp_entries.iter().any(|entry| {
            entry.record.status == TaskStatus::Running
        });

        // Only exit when both registries have no running tasks
        if !cli_has_running && !mcp_has_running {
            print_report(&report, None, false, start.elapsed());
            return Ok(());
        }

        if start.elapsed() >= MAX_WAIT_DURATION {
            // Filter running entries to only show related processes
            let mut filtered_entries: Vec<RegistryEntry> = cli_entries
                .iter()
                .filter(|entry| should_process_task(&entry.record, current_root_parent))
                .cloned()
                .collect();

            // Add MCP entries (no filtering for cross-process)
            filtered_entries.extend(mcp_entries.iter().cloned());

            print_report(&report, Some(&filtered_entries), true, start.elapsed());
            return Ok(());
        }

        thread::sleep(interval);
    }
}

/// Check if a task should be processed based on root parent PID
pub fn should_process_task(record: &TaskRecord, current_root_parent: Option<u32>) -> bool {
    // If we don't have root parent info, process all tasks
    let task_root_parent = match record.resolved_root_parent_pid() {
        Some(pid) => pid,
        None => return true, // Process tasks without root parent info
    };

    match current_root_parent {
        Some(current_pid) => task_root_parent == current_pid,
        None => true, // No filtering if we couldn't get current root parent
    }
}

fn read_interval() -> Duration {
    read_env_interval(WAIT_INTERVAL_ENV)
        .or_else(|| read_env_interval(LEGACY_WAIT_INTERVAL_ENV))
        .unwrap_or(WAIT_INTERVAL_DEFAULT)
}

fn read_env_interval(var: &str) -> Option<Duration> {
    match std::env::var(var) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(seconds) if seconds > 0 => Some(Duration::from_secs(seconds)),
            _ => {
                warn(format!(
                    "environment variable {var} invalid, using default 30s"
                ));
                None
            }
        },
        Err(_) => None,
    }
}

fn emit_realtime_update(task: &TaskCompletion) {
    let exit_code = task
        .exit_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "未提供".to_string());
    let status_word = if task.is_success() {
        "完成"
    } else {
        "失败"
    };
    let header = format!(
        "{} 任务{} PID={} (exit_code: {}) @ {}",
        task.status_icon(),
        status_word,
        task.pid,
        exit_code,
        task.completed_time_local()
    );
    let log_line = format!("日志文件: {}", task.log_path);
    let summary_line = format!("{}: {}", task.summary_label(), task.summary_text());

    if task.is_success() {
        println!("{header}");
        println!("{log_line}");
        println!("{summary_line}");
    } else {
        eprintln!("{header}");
        eprintln!("{log_line}");
        eprintln!("{summary_line}");
    }
}

fn print_report(
    report: &TaskReport,
    running_entries: Option<&[RegistryEntry]>,
    timed_out: bool,
    wait_elapsed: Duration,
) {
    let mut buffer = String::new();
    report
        .render(&mut buffer, running_entries, timed_out, wait_elapsed)
        .expect("rendering wait report");
    println!("{buffer}");
}

#[derive(Clone)]
struct TaskCompletion {
    pid: u32,
    log_path: String,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    exit_code: Option<i32>,
    result: Option<String>,
    cleanup_reason: Option<String>,
}

impl TaskCompletion {
    fn from_record(pid: u32, mut record: TaskRecord) -> Self {
        let completed_at = record.completed_at.unwrap_or_else(Utc::now);
        record.completed_at = Some(completed_at);
        Self {
            pid,
            log_path: record.log_path,
            started_at: record.started_at,
            completed_at,
            exit_code: record.exit_code,
            result: record.result,
            cleanup_reason: record.cleanup_reason,
        }
    }

    fn is_success(&self) -> bool {
        self.cleanup_reason.is_none() && self.exit_code.unwrap_or(0) == 0
    }

    fn status_icon(&self) -> &'static str {
        if self.is_success() {
            "✅"
        } else {
            "❌"
        }
    }

    fn completed_time_local(&self) -> String {
        self.completed_at
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    }

    fn summary_label(&self) -> &'static str {
        if self.is_success() {
            "结果摘要"
        } else {
            "错误摘要"
        }
    }

    fn summary_text(&self) -> String {
        if let Some(result) = &self.result {
            result.clone()
        } else if let Some(reason) = &self.cleanup_reason {
            format!("任务被清理: {reason}")
        } else if self.is_success() {
            "任务成功完成，但未提供摘要。".to_string()
        } else {
            "任务失败，未提供错误摘要。".to_string()
        }
    }
}

struct TaskReport {
    completions: Vec<TaskCompletion>,
    earliest_start: Option<DateTime<Utc>>,
    latest_completion: Option<DateTime<Utc>>,
}

impl TaskReport {
    fn new() -> Self {
        Self {
            completions: Vec::new(),
            earliest_start: None,
            latest_completion: None,
        }
    }

    fn add_completion(&mut self, completion: TaskCompletion) {
        if self
            .earliest_start
            .is_none_or(|current| completion.started_at < current)
        {
            self.earliest_start = Some(completion.started_at);
        }
        if self
            .latest_completion
            .is_none_or(|current| completion.completed_at > current)
        {
            self.latest_completion = Some(completion.completed_at);
        }
        self.completions.push(completion);
    }

    fn total_count(&self) -> usize {
        self.completions.len()
    }

    fn successful_count(&self) -> usize {
        self.completions.iter().filter(|c| c.is_success()).count()
    }

    fn failed_count(&self) -> usize {
        self.total_count() - self.successful_count()
    }

    fn total_duration(&self) -> Option<chrono::Duration> {
        match (self.earliest_start, self.latest_completion) {
            (Some(start), Some(end)) => Some(end.signed_duration_since(start)),
            _ => None,
        }
    }

    fn render(
        &self,
        buffer: &mut String,
        running_entries: Option<&[RegistryEntry]>,
        timed_out: bool,
        wait_elapsed: Duration,
    ) -> Result<(), std::fmt::Error> {
        writeln!(buffer, "## 📋 任务执行完成报告")?;
        if timed_out {
            writeln!(buffer, "\n⚠️ 等待已达到最大时长，仍检测到未完成的任务。")?;
        }

        writeln!(buffer, "\n### ✅ 已完成任务列表")?;
        if self.completions.is_empty() {
            writeln!(buffer, "- 暂无完成任务")?;
        } else {
            let mut items = self.completions.clone();
            items.sort_by_key(|item| item.completed_at);
            for (idx, completion) in items.iter().enumerate() {
                writeln!(buffer, "{}. **PID**: {}", idx + 1, completion.pid)?;
                writeln!(
                    buffer,
                    "   - **状态**: {}",
                    completion.status_icon_with_exit_code()
                )?;
                writeln!(buffer, "   - **日志文件**: {}", completion.log_path)?;
                writeln!(
                    buffer,
                    "   - **完成时间**: {}",
                    completion.completed_time_local()
                )?;
                writeln!(
                    buffer,
                    "   - **{}**: {}",
                    completion.summary_label(),
                    completion.summary_text()
                )?;
            }
        }

        let total_duration = self
            .total_duration()
            .or_else(|| chrono::Duration::from_std(wait_elapsed).ok())
            .unwrap_or_else(chrono::Duration::zero);
        writeln!(buffer, "\n### 📊 执行统计")?;
        writeln!(buffer, "- 总任务数: {}", self.total_count())?;
        writeln!(buffer, "- 成功: {}个", self.successful_count())?;
        writeln!(buffer, "- 失败: {}个", self.failed_count())?;
        writeln!(
            buffer,
            "- 总耗时: {}",
            format_human_duration(total_duration)
        )?;

        writeln!(buffer, "\n### 📂 完整日志文件路径")?;
        let mut log_paths: Vec<String> = Vec::new();
        if self.completions.is_empty() {
            writeln!(buffer, "- 无可用日志")?;
        } else {
            let mut paths: Vec<&String> = self.completions.iter().map(|c| &c.log_path).collect();
            paths.sort();
            paths.dedup();
            for path in &paths {
                writeln!(buffer, "- {path}")?;
            }
            log_paths = paths.iter().map(|path| (*path).clone()).collect();
        }

        if let Some(entries) = running_entries {
            let running: Vec<&RegistryEntry> = entries
                .iter()
                .filter(|entry| entry.record.status == TaskStatus::Running)
                .collect();
            if !running.is_empty() {
                writeln!(buffer, "\n### ⏳ 仍在运行的任务")?;
                for entry in running {
                    let started = entry
                        .record
                        .started_at
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %H:%M:%S");
                    writeln!(
                        buffer,
                        "- PID {} (启动于 {started}) -> {}",
                        entry.pid, entry.record.log_path
                    )?;
                }
            }
        }

        writeln!(
            buffer,
            "\n现在请基于上述结果继续你的工作，必要时查看日志文件。"
        )?;
        writeln!(buffer, "\n### 🧠 Claude 日志阅读提示")?;
        writeln!(
            buffer,
            "- Claude，请分批次读取体积较大的日志文件，避免一次性请求全部内容。"
        )?;
        writeln!(
            buffer,
            "- 请在读取日志时使用 `offset`/`limit` 参数来控制输出范围，逐段检查关键信息。"
        )?;
        if log_paths.is_empty() {
            writeln!(
                buffer,
                "- 当前没有可供阅读的日志文件路径，可在任务完成后再尝试。"
            )?;
        } else {
            writeln!(buffer, "- 建议按照以下路径逐个读取日志：")?;
            for path in &log_paths {
                writeln!(buffer, "  - {path}")?;
            }
        }
        writeln!(
            buffer,
            "- 读取完一批内容后，请说明下一步需要的 `offset`/`limit` 或指出新的文件路径，以便继续协助你。"
        )?;
        Ok(())
    }
}

impl TaskCompletion {
    fn status_icon_with_exit_code(&self) -> String {
        let exit_code = self
            .exit_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "未提供".to_string());
        if let Some(reason) = &self.cleanup_reason {
            format!(
                "{} {} (exit_code: {exit_code}, cleanup: {reason})",
                self.status_icon(),
                if self.is_success() {
                    "完成"
                } else {
                    "失败"
                }
            )
        } else {
            format!(
                "{} {} (exit_code: {exit_code})",
                self.status_icon(),
                if self.is_success() {
                    "完成"
                } else {
                    "失败"
                }
            )
        }
    }
}

fn format_human_duration(duration: chrono::Duration) -> String {
    let mut seconds = duration.num_seconds();
    if seconds < 0 {
        seconds = 0;
    }
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let remaining_seconds = seconds % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}小时"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}分"));
    }
    if remaining_seconds > 0 || parts.is_empty() {
        parts.push(format!("{remaining_seconds}秒"));
    }

    parts.join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_record::TaskRecord;
    use chrono::Utc;
    use serial_test::serial;
    use std::env;

    /// Helper to safely set environment variable for testing
    ///
    /// Note: Environment variables are process-global, so these tests must run serially
    /// using the #[serial] attribute from serial_test crate.
    fn set_test_env(key: &str, value: &str) {
        env::set_var(key, value);
    }

    /// Helper to safely clear test environment variables
    fn clear_test_env() {
        env::remove_var(WAIT_INTERVAL_ENV);
        env::remove_var(LEGACY_WAIT_INTERVAL_ENV);
    }

    /// Cleanup guard that ensures environment variables are cleared after test
    struct EnvCleanup;

    impl Drop for EnvCleanup {
        fn drop(&mut self) {
            clear_test_env();
        }
    }

    #[test]
    #[serial] // Ensure tests run one at a time to avoid env var conflicts
    fn prefers_primary_interval_env() {
        let _cleanup = EnvCleanup;
        clear_test_env();

        set_test_env(WAIT_INTERVAL_ENV, "45");
        assert_eq!(read_interval(), Duration::from_secs(45));
    }

    #[test]
    #[serial] // Ensure tests run one at a time to avoid env var conflicts
    fn falls_back_to_legacy_env() {
        let _cleanup = EnvCleanup;
        clear_test_env();

        set_test_env(LEGACY_WAIT_INTERVAL_ENV, "90");
        assert_eq!(read_interval(), Duration::from_secs(90));
    }

    #[test]
    #[serial] // Ensure tests run one at a time to avoid env var conflicts
    fn returns_default_on_invalid_values() {
        let _cleanup = EnvCleanup;
        clear_test_env();

        set_test_env(WAIT_INTERVAL_ENV, "not-a-number");
        assert_eq!(read_interval(), WAIT_INTERVAL_DEFAULT);
    }

    #[test]
    fn test_should_process_task_filtering() {
        let base_time = Utc::now();

        // Task with root parent PID 100
        let task_with_root = TaskRecord::new(
            base_time,
            "1001".to_string(),
            "/tmp/1001.log".to_string(),
            Some(1000),
        );
        let task_with_root = task_with_root
            .with_process_tree_info(ProcessTreeInfo::new(vec![1000, 100]))
            .expect("process tree should attach");

        // Task without root parent info (backward compatibility)
        let task_without_root = TaskRecord::new(
            base_time,
            "1002".to_string(),
            "/tmp/1002.log".to_string(),
            Some(1000),
        );

        // Task with different root parent PID
        let task_different_root = TaskRecord::new(
            base_time,
            "1003".to_string(),
            "/tmp/1003.log".to_string(),
            Some(1000),
        );
        let task_different_root = task_different_root
            .with_process_tree_info(ProcessTreeInfo::new(vec![2000, 200]))
            .expect("process tree should attach");

        // Test filtering by root parent 100
        assert!(should_process_task(&task_with_root, Some(100)));
        assert!(should_process_task(&task_without_root, Some(100))); // Should include tasks without root info
        assert!(!should_process_task(&task_different_root, Some(100)));

        // Test with no filter (None means process all tasks)
        assert!(should_process_task(&task_with_root, None));
        assert!(should_process_task(&task_without_root, None));
        assert!(should_process_task(&task_different_root, None));
    }
}
