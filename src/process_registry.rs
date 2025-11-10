
//! 进程内任务注册表 - 专门用于MCP启动的任务
//!
//! 与TaskRegistry（跨进程SharedMemory）不同，InProcessRegistry使用线程安全的HashMap
//! 仅在当前进程内共享任务状态，适用于MCP启动的任务管理

use crate::{
    core::models::ProcessTreeInfo,
    error::RegistryError,
    storage::{CleanupEvent, InProcessStorage, RegistryEntry, TaskStorage},
    task_record::TaskRecord,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// 进程内任务注册表
/// 使用InProcessStorage存储任务，不跨进程共享
#[derive(Debug, Clone)]
pub struct InProcessRegistry {
    storage: Arc<InProcessStorage>,
}

impl InProcessRegistry {
    /// 创建新的进程内注册表
    pub fn new() -> Self {
        Self {
            storage: Arc::new(InProcessStorage::new()),
        }
    }

    /// 注册新任务
    pub fn register(&self, pid: u32, record: &TaskRecord) -> Result<(), RegistryError> {
        self.storage.register(pid, record)
    }

    /// 标记任务完成
    pub fn mark_completed(
        &self,
        pid: u32,
        result: Option<String>,
        exit_code: Option<i32>,
        completed_at: DateTime<Utc>,
    ) -> Result<(), RegistryError> {
        self.storage
            .mark_completed(pid, result, exit_code, completed_at)
    }

    /// 获取所有任务条目
    pub fn entries(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
        self.storage.entries()
    }

    /// 清理过期任务
    pub fn sweep_stale_entries<F, G>(
        &self,
        now: DateTime<Utc>,
        is_process_alive: F,
        terminate_process: &G,
    ) -> Result<Vec<CleanupEvent>, RegistryError>
    where
        F: Fn(u32) -> bool,
        G: Fn(u32) -> Result<(), String>,
    {
        self.storage
            .sweep_stale_entries(now, is_process_alive, terminate_process)
    }

    /// 获取已完成但未读的任务
    pub fn get_completed_unread_tasks(&self) -> Result<Vec<(u32, TaskRecord)>, RegistryError> {
        self.storage.get_completed_unread_tasks()
    }

    /// 检查是否有运行中的任务
    pub fn has_running_tasks(
        &self,
        filter: Option<&ProcessTreeInfo>,
    ) -> Result<bool, RegistryError> {
        self.storage.has_running_tasks(filter)
    }
}

impl Default for InProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_task(log_id: &str) -> TaskRecord {
        TaskRecord::new(
            Utc::now(),
            log_id.to_string(),
            format!("/tmp/{}.log", log_id),
            Some(std::process::id()),
        )
    }

    #[test]
    fn test_register_task() {
        let registry = InProcessRegistry::new();
        let task = create_test_task("task-001");
        let pid = 12345;

        assert!(registry.register(pid, &task).is_ok());

        let entries = registry.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pid, pid);
    }

    #[test]
    fn test_mark_completed() {
        let registry = InProcessRegistry::new();
        let task = create_test_task("task-complete");
        let pid = 20000;

        registry.register(pid, &task).unwrap();
        registry
            .mark_completed(pid, Some("Success".to_string()), Some(0), Utc::now())
            .unwrap();

        let completed = registry.get_completed_unread_tasks().unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].0, pid);
    }

    #[test]
    fn test_sweep_stale() {
        let registry = InProcessRegistry::new();
        let task = create_test_task("task-stale");
        let pid = 30000;

        registry.register(pid, &task).unwrap();

        let is_alive = |check_pid: u32| check_pid != 30000;
        let terminate = |_: u32| Ok(());

        let events = registry
            .sweep_stale_entries(Utc::now(), is_alive, &terminate)
            .unwrap();

        assert!(!events.is_empty());
    }

    #[test]
    fn test_multiple_registries_are_independent() {
        let registry1 = InProcessRegistry::new();
        let registry2 = InProcessRegistry::new();

        let task1 = create_test_task("task-r1");
        let task2 = create_test_task("task-r2");

        registry1.register(100, &task1).unwrap();
        registry2.register(200, &task2).unwrap();

        // 两个注册表应该是独立的
        let entries1 = registry1.entries().unwrap();
        let entries2 = registry2.entries().unwrap();

        assert_eq!(entries1.len(), 1);
        assert_eq!(entries2.len(), 1);
        assert_eq!(entries1[0].pid, 100);
        assert_eq!(entries2[0].pid, 200);
    }
}
