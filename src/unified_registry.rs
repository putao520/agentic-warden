//! 通用任务注册表 - 基于存储层的泛型实现
//!
//! 使用泛型消除代码重复，支持不同的存储后端

use crate::{
    core::models::ProcessTreeInfo,
    error::RegistryError,
    storage::{CleanupEvent, RegistryEntry, TaskStorage},
    task_record::TaskRecord,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// 通用任务注册表
///
/// 泛型设计，可以使用任何实现了 TaskStorage trait 的存储后端
///
/// # 示例
///
/// ```rust
/// use aiw::unified_registry::Registry;
/// use aiw::storage::{InProcessStorage, SharedMemoryStorage};
///
/// // 进程内注册表（用于MCP启动的任务）
/// let mcp_registry = Registry::new(InProcessStorage::new());
///
/// // 跨进程注册表（用于CLI启动的任务，使用当前进程PID）
/// let cli_registry = Registry::new(SharedMemoryStorage::connect().unwrap());
/// ```
#[derive(Debug)]
pub struct Registry<S: TaskStorage> {
    storage: Arc<S>,
}

impl<S: TaskStorage> Clone for Registry<S> {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
        }
    }
}

impl<S: TaskStorage> Registry<S> {
    /// 创建新的注册表实例
    pub fn new(storage: S) -> Self {
        Self {
            storage: Arc::new(storage),
        }
    }

    /// 从Arc创建（用于共享现有存储）
    pub fn from_arc(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// 获取存储的引用
    pub fn storage(&self) -> &Arc<S> {
        &self.storage
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

// 为了方便使用，提供类型别名
use crate::storage::{InProcessStorage, SharedMemoryStorage};

/// 进程内注册表类型别名
pub type InProcessRegistry = Registry<InProcessStorage>;

/// 跨进程注册表类型别名
pub type SharedMemoryRegistry = Registry<SharedMemoryStorage>;

/// 便捷构造函数
impl Registry<InProcessStorage> {
    /// 创建新的进程内注册表
    pub fn in_process() -> Self {
        Self::new(InProcessStorage::new())
    }
}

impl Registry<SharedMemoryStorage> {
    /// 连接到跨进程注册表
    pub fn shared_memory() -> Result<Self, RegistryError> {
        Ok(Self::new(SharedMemoryStorage::connect()?))
    }

    /// 连接到指定PID的共享内存
    pub fn shared_memory_for_pid(pid: u32) -> Result<Self, RegistryError> {
        Ok(Self::new(SharedMemoryStorage::connect_for_pid(pid)?))
    }

    /// 使用指定命名空间连接
    pub fn shared_memory_with_namespace(namespace: String) -> Result<Self, RegistryError> {
        Ok(Self::new(SharedMemoryStorage::connect_with_namespace(
            namespace,
        )?))
    }

    /// 清理共享内存（进程结束时调用）
    pub fn cleanup(&self) -> Result<(), RegistryError> {
        self.storage.cleanup()
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
    fn test_in_process_registry() {
        let registry = Registry::in_process();
        let task = create_test_task("test-001");
        let pid = 12345;

        assert!(registry.register(pid, &task).is_ok());

        let entries = registry.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pid, pid);
    }

    #[test]
    fn test_registry_mark_completed() {
        let registry = Registry::in_process();
        let task = create_test_task("test-complete");
        let pid = 20000;

        registry.register(pid, &task).unwrap();
        registry
            .mark_completed(pid, Some("success".to_string()), Some(0), Utc::now())
            .unwrap();

        let completed = registry.get_completed_unread_tasks().unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].0, pid);
    }

    #[test]
    fn test_multiple_registries_independent() {
        let registry1 = Registry::in_process();
        let registry2 = Registry::in_process();

        let task1 = create_test_task("reg1-task");
        let task2 = create_test_task("reg2-task");

        registry1.register(100, &task1).unwrap();
        registry2.register(200, &task2).unwrap();

        // 两个注册表应该独立
        let entries1 = registry1.entries().unwrap();
        let entries2 = registry2.entries().unwrap();

        assert_eq!(entries1.len(), 1);
        assert_eq!(entries2.len(), 1);
        assert_eq!(entries1[0].pid, 100);
        assert_eq!(entries2[0].pid, 200);
    }

    #[test]
    fn test_shared_storage_access() {
        let storage = Arc::new(InProcessStorage::new());

        // 从同一个存储创建两个注册表
        let registry1 = Registry::from_arc(Arc::clone(&storage));
        let registry2 = Registry::from_arc(Arc::clone(&storage));

        let task = create_test_task("shared-task");
        registry1.register(300, &task).unwrap();

        // registry2 应该能看到 registry1 注册的任务
        let entries = registry2.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pid, 300);
    }
}
