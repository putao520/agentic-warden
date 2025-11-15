use crate::{
    config::{MAX_RECORD_AGE, SHARED_MEMORY_SIZE},
    core::models::ProcessTreeInfo,
    core::shared_map::open_or_create,
    error::RegistryError,
    logging::warn,
    task_record::{TaskRecord, TaskStatus},
};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use parking_lot::Mutex;
use shared_hashmap::SharedMemoryHashMap;
use std::sync::Arc;

/// 任务注册表条目
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub pid: u32,
    pub key: String,
    pub record: TaskRecord,
}

/// 清理事件
#[derive(Debug, Clone)]
pub struct CleanupEvent {
    pub _pid: u32,
    pub record: TaskRecord,
    pub reason: CleanupReason,
}

/// 清理原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupReason {
    ProcessExited,
    Timeout,
    ManagerMissing,
}

/// 任务存储的统一接口
/// 提供跨进程（SharedMemory）和进程内（InProcess）两种实现
pub trait TaskStorage: Send + Sync {
    /// 注册新任务
    fn register(&self, pid: u32, record: &TaskRecord) -> Result<(), RegistryError>;

    /// 标记任务完成
    fn mark_completed(
        &self,
        pid: u32,
        result: Option<String>,
        exit_code: Option<i32>,
        completed_at: DateTime<Utc>,
    ) -> Result<(), RegistryError>;

    /// 获取所有任务条目
    fn entries(&self) -> Result<Vec<RegistryEntry>, RegistryError>;

    /// 清理过期任务
    fn sweep_stale_entries<F, G>(
        &self,
        now: DateTime<Utc>,
        is_process_alive: F,
        terminate_process: &G,
    ) -> Result<Vec<CleanupEvent>, RegistryError>
    where
        F: Fn(u32) -> bool,
        G: Fn(u32) -> Result<(), String>;

    /// 获取已完成但未读的任务
    fn get_completed_unread_tasks(&self) -> Result<Vec<(u32, TaskRecord)>, RegistryError>;

    /// 检查是否有运行中的任务
    fn has_running_tasks(&self, filter: Option<&ProcessTreeInfo>) -> Result<bool, RegistryError>;
}

/// 进程内任务存储（线程安全）
/// 用于MCP启动的任务，不跨进程共享
/// 使用DashMap提供高性能的并发访问
#[derive(Debug, Clone)]
pub struct InProcessStorage {
    tasks: Arc<DashMap<u32, TaskRecord>>,
}

impl InProcessStorage {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
        }
    }
}

impl Default for InProcessStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskStorage for InProcessStorage {
    fn register(&self, pid: u32, record: &TaskRecord) -> Result<(), RegistryError> {
        self.tasks.insert(pid, record.clone());
        Ok(())
    }

    fn mark_completed(
        &self,
        pid: u32,
        result: Option<String>,
        exit_code: Option<i32>,
        completed_at: DateTime<Utc>,
    ) -> Result<(), RegistryError> {
        if let Some(mut record) = self.tasks.get_mut(&pid) {
            record.status = TaskStatus::CompletedButUnread;
            record.result = result;
            record.exit_code = exit_code;
            record.completed_at = Some(completed_at);
        } else {
            return Err(RegistryError::TaskNotFound(pid));
        }
        Ok(())
    }

    fn entries(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
        Ok(self
            .tasks
            .iter()
            .map(|entry| RegistryEntry {
                pid: *entry.key(),
                key: entry.key().to_string(),
                record: entry.value().clone(),
            })
            .collect())
    }

    fn sweep_stale_entries<F, G>(
        &self,
        now: DateTime<Utc>,
        is_process_alive: F,
        terminate_process: &G,
    ) -> Result<Vec<CleanupEvent>, RegistryError>
    where
        F: Fn(u32) -> bool,
        G: Fn(u32) -> Result<(), String>,
    {
        const MAX_AGE_HOURS: i64 = 12;
        let max_age = Duration::hours(MAX_AGE_HOURS);

        let mut cleanup_events = Vec::new();

        let pids_to_cleanup: Vec<(u32, CleanupReason)> = self
            .tasks
            .iter()
            .filter_map(|entry| {
                let pid = *entry.key();
                let record = entry.value();

                // 如果进程已不存在
                if !is_process_alive(pid) {
                    // 如果任务未标记完成，补标记
                    if record.status == TaskStatus::Running {
                        return Some((pid, CleanupReason::ProcessExited));
                    }
                }

                // 如果记录太旧（超过12小时）
                let age = now.signed_duration_since(record.started_at);
                if age > max_age {
                    if record.status == TaskStatus::Running && is_process_alive(pid) {
                        // 尝试终止
                        let _ = terminate_process(pid);
                        return Some((pid, CleanupReason::Timeout));
                    }
                }

                None
            })
            .collect();

        for (pid, cleanup_reason) in pids_to_cleanup {
            if let Some(mut record) = self.tasks.get_mut(&pid) {
                record.status = TaskStatus::CompletedButUnread;
                record.completed_at = Some(now);
                record.cleanup_reason = Some(
                    match cleanup_reason {
                        CleanupReason::ProcessExited => "process_exited",
                        CleanupReason::Timeout => "timeout",
                        CleanupReason::ManagerMissing => "manager_missing",
                    }
                    .to_string(),
                );

                cleanup_events.push(CleanupEvent {
                    _pid: pid,
                    record: record.clone(),
                    reason: cleanup_reason,
                });
            }
        }

        Ok(cleanup_events)
    }

    fn get_completed_unread_tasks(&self) -> Result<Vec<(u32, TaskRecord)>, RegistryError> {
        let completed: Vec<(u32, TaskRecord)> = self
            .tasks
            .iter()
            .filter_map(|entry| {
                let pid = *entry.key();
                let record = entry.value();

                if record.status == TaskStatus::CompletedButUnread {
                    // 标记为已读（从映射中移除）
                    Some((pid, record.clone()))
                } else {
                    None
                }
            })
            .collect();

        // 移除已读的任务
        for (pid, _) in &completed {
            self.tasks.remove(pid);
        }

        Ok(completed)
    }

    fn has_running_tasks(&self, filter: Option<&ProcessTreeInfo>) -> Result<bool, RegistryError> {
        if let Some(tree_filter) = filter {
            Ok(self.tasks.iter().any(|entry| {
                let record = entry.value();
                record.status == TaskStatus::Running
                    && record
                        .process_tree
                        .as_ref()
                        .map(|tree| tree.root_parent_pid == tree_filter.root_parent_pid)
                        .unwrap_or(false)
            }))
        } else {
            Ok(self
                .tasks
                .iter()
                .any(|entry| entry.value().status == TaskStatus::Running))
        }
    }
}

/// 跨进程任务存储（SharedMemory）
/// 用于CLI启动的任务，支持跨进程共享
#[derive(Debug, Clone)]
pub struct SharedMemoryStorage {
    namespace: String,
    map: Arc<Mutex<SharedMemoryHashMap<String, String>>>,
}

impl SharedMemoryStorage {
    /// 连接到当前进程的共享内存
    /// 使用当前进程PID作为命名空间: {PID}_task
    pub fn connect() -> Result<Self, RegistryError> {
        let pid = std::process::id();
        Self::connect_for_pid(pid)
    }

    /// 连接到指定PID的共享内存
    /// 使用格式: {pid}_task
    pub fn connect_for_pid(pid: u32) -> Result<Self, RegistryError> {
        let namespace = format!("{}_task", pid);
        Self::connect_with_namespace(namespace)
    }

    /// 使用指定的命名空间连接
    pub fn connect_with_namespace(namespace: String) -> Result<Self, RegistryError> {
        let map = open_or_create(&namespace, SHARED_MEMORY_SIZE)?;
        Ok(Self {
            namespace,
            map: Arc::new(Mutex::new(map)),
        })
    }

    /// 删除共享内存（用于进程结束时清理）
    pub fn cleanup(&self) -> Result<(), RegistryError> {
        use shared_memory::ShmemConf;

        // 尝试删除共享内存
        if let Ok(mut shmem) = ShmemConf::new()
            .os_id(&self.namespace)
            .size(SHARED_MEMORY_SIZE)
            .open()
        {
            let _ = shmem.set_owner(true);
        }

        Ok(())
    }

    fn with_map<T>(
        &self,
        f: impl FnOnce(&mut SharedMemoryHashMap<String, String>) -> Result<T, RegistryError>,
    ) -> Result<T, RegistryError> {
        let mut guard = self.map.lock();
        f(&mut guard)
    }

    fn remove_keys(&self, keys: &[String]) -> Result<(), RegistryError> {
        if keys.is_empty() {
            return Ok(());
        }
        self.with_map(|map| {
            for key in keys {
                map.remove(key);
            }
            Ok(())
        })
    }
}

impl TaskStorage for SharedMemoryStorage {
    fn register(&self, pid: u32, record: &TaskRecord) -> Result<(), RegistryError> {
        let key = pid.to_string();
        let value = serde_json::to_string(record)?;
        self.with_map(|map| {
            map.try_insert(key.clone(), value)?;
            Ok(())
        })
    }

    fn mark_completed(
        &self,
        pid: u32,
        result: Option<String>,
        exit_code: Option<i32>,
        completed_at: DateTime<Utc>,
    ) -> Result<(), RegistryError> {
        let key = pid.to_string();
        self.with_map(move |map| {
            let existing = map
                .get(&key)
                .ok_or_else(|| RegistryError::Map(format!("no task found for pid {pid}")))?;
            let record: TaskRecord = serde_json::from_str(&existing)?;
            let updated_record = record.mark_completed(result, exit_code, completed_at);
            let updated_value = serde_json::to_string(&updated_record)?;
            let _ = map.insert(key.clone(), updated_value);
            Ok(())
        })
    }

    fn entries(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
        let snapshot: Vec<(String, String)> = {
            let guard = self.map.lock();
            guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let mut entries = Vec::new();
        let mut invalid_keys = Vec::new();

        for (key, value) in snapshot {
            match key.parse::<u32>() {
                Ok(pid) => match serde_json::from_str::<TaskRecord>(&value) {
                    Ok(record) => entries.push(RegistryEntry {
                        pid,
                        key: key.clone(),
                        record,
                    }),
                    Err(err) => {
                        warn(format!("failed to parse task record pid={key}: {err}"));
                        invalid_keys.push(key);
                    }
                },
                Err(_) => {
                    warn(format!("detected invalid pid key: {key}"));
                    invalid_keys.push(key);
                }
            }
        }

        if !invalid_keys.is_empty() {
            self.remove_keys(&invalid_keys)?;
        }

        Ok(entries)
    }

    fn sweep_stale_entries<F, G>(
        &self,
        now: DateTime<Utc>,
        is_process_alive: F,
        terminate_process: &G,
    ) -> Result<Vec<CleanupEvent>, RegistryError>
    where
        F: Fn(u32) -> bool,
        G: Fn(u32) -> Result<(), String>,
    {
        let entries = self.entries()?;
        let mut removals = Vec::new();
        let mut events = Vec::new();

        for mut entry in entries {
            let mut should_cleanup = false;
            let mut cleanup_reason = CleanupReason::ProcessExited;

            // 检查进程是否存活
            if !is_process_alive(entry.pid) {
                should_cleanup = true;
                cleanup_reason = CleanupReason::ProcessExited;
            } else {
                // 检查manager进程
                if let Some(_manager_pid) = entry.record.manager_pid.filter(|&manager_pid| {
                    manager_pid != entry.pid && !is_process_alive(manager_pid)
                }) {
                    let _ = terminate_process(entry.pid);
                    should_cleanup = true;
                    cleanup_reason = CleanupReason::ManagerMissing;
                }

                // 检查是否超时
                if !should_cleanup {
                    let age = now.signed_duration_since(entry.record.started_at);
                    let max_age = Duration::from_std(MAX_RECORD_AGE).unwrap_or(Duration::zero());
                    if age > max_age {
                        let _ = terminate_process(entry.pid);
                        should_cleanup = true;
                        cleanup_reason = CleanupReason::Timeout;
                    }
                }
            }

            if should_cleanup {
                removals.push(entry.pid.to_string());

                // Update record with cleanup reason
                entry.record.cleanup_reason = Some(
                    match cleanup_reason {
                        CleanupReason::ProcessExited => "process_exited",
                        CleanupReason::Timeout => "timeout",
                        CleanupReason::ManagerMissing => "manager_missing",
                    }
                    .to_string(),
                );

                events.push(CleanupEvent {
                    _pid: entry.pid,
                    record: entry.record,
                    reason: cleanup_reason,
                });
            }
        }

        if !removals.is_empty() {
            self.remove_keys(&removals)?;
        }

        Ok(events)
    }

    fn get_completed_unread_tasks(&self) -> Result<Vec<(u32, TaskRecord)>, RegistryError> {
        let entries = self.entries()?;
        let mut completed_pids = Vec::new();

        for entry in &entries {
            if entry.record.status == TaskStatus::CompletedButUnread {
                completed_pids.push(entry.pid);
            }
        }

        // 从共享内存中删除已完成的任务
        for pid in &completed_pids {
            let key = pid.to_string();
            let _ = self.with_map(|map| {
                map.remove(&key);
                Ok::<(), RegistryError>(())
            });
        }

        // 返回已完成的任务
        let completed_tasks: Vec<(u32, TaskRecord)> = entries
            .into_iter()
            .filter(|entry| entry.record.status == TaskStatus::CompletedButUnread)
            .map(|entry| (entry.pid, entry.record))
            .collect();

        Ok(completed_tasks)
    }

    fn has_running_tasks(&self, filter: Option<&ProcessTreeInfo>) -> Result<bool, RegistryError> {
        let entries = self.entries()?;

        if let Some(tree_filter) = filter {
            Ok(entries.iter().any(|entry| {
                entry.record.status == TaskStatus::Running
                    && entry
                        .record
                        .process_tree
                        .as_ref()
                        .map(|tree| tree.root_parent_pid == tree_filter.root_parent_pid)
                        .unwrap_or(false)
            }))
        } else {
            Ok(entries
                .iter()
                .any(|entry| entry.record.status == TaskStatus::Running))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_process_storage_register() {
        let storage = InProcessStorage::new();
        let record = TaskRecord::new(
            Utc::now(),
            "123".to_string(),
            "/tmp/test.log".to_string(),
            Some(100),
        );

        assert!(storage.register(123, &record).is_ok());
        let entries = storage.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pid, 123);
    }

    #[test]
    fn test_in_process_storage_mark_completed() {
        let storage = InProcessStorage::new();
        let record = TaskRecord::new(
            Utc::now(),
            "456".to_string(),
            "/tmp/test.log".to_string(),
            Some(100),
        );

        storage.register(456, &record).unwrap();
        storage
            .mark_completed(456, Some("success".to_string()), Some(0), Utc::now())
            .unwrap();

        let completed = storage.get_completed_unread_tasks().unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].0, 456);
        assert_eq!(completed[0].1.result, Some("success".to_string()));
    }

    #[test]
    fn test_in_process_storage_sweep_stale() {
        let storage = InProcessStorage::new();
        let old_time = Utc::now() - Duration::hours(13);
        let record = TaskRecord::new(
            old_time,
            "789".to_string(),
            "/tmp/test.log".to_string(),
            Some(100),
        );

        storage.register(789, &record).unwrap();

        let is_alive = |_: u32| false;
        let terminate = |_: u32| Ok(());

        let events = storage
            .sweep_stale_entries(Utc::now(), is_alive, &terminate)
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]._pid, 789);
    }

    #[cfg(test)]
    mod concurrency_tests {
        use super::*;
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex as StdMutex};
        use std::thread;
        use std::time::Instant;

        // 为了比较性能，创建一个使用Mutex<HashMap>的旧版InProcessStorage
        #[derive(Debug)]
        struct LegacyInProcessStorage {
            tasks: Arc<StdMutex<HashMap<u32, TaskRecord>>>,
        }

        impl LegacyInProcessStorage {
            fn new() -> Self {
                Self {
                    tasks: Arc::new(StdMutex::new(HashMap::new())),
                }
            }
        }

        impl TaskStorage for LegacyInProcessStorage {
            fn register(&self, pid: u32, record: &TaskRecord) -> Result<(), RegistryError> {
                let mut tasks = self.tasks.lock().unwrap();
                tasks.insert(pid, record.clone());
                Ok(())
            }

            fn entries(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
                let tasks = self.tasks.lock().unwrap();
                Ok(tasks
                    .iter()
                    .map(|(&pid, record)| RegistryEntry {
                        pid,
                        key: pid.to_string(),
                        record: record.clone(),
                    })
                    .collect())
            }

            // 简化的其他方法实现用于性能测试
            fn mark_completed(
                &self,
                _pid: u32,
                _result: Option<String>,
                _exit_code: Option<i32>,
                _completed_at: DateTime<Utc>,
            ) -> Result<(), RegistryError> {
                Ok(())
            }

            fn sweep_stale_entries<F, G>(
                &self,
                _now: DateTime<Utc>,
                _is_process_alive: F,
                _terminate_process: &G,
            ) -> Result<Vec<CleanupEvent>, RegistryError> {
                Ok(Vec::new())
            }

            fn get_completed_unread_tasks(&self) -> Result<Vec<(u32, TaskRecord)>, RegistryError> {
                Ok(Vec::new())
            }

            fn has_running_tasks(
                &self,
                _filter: Option<&ProcessTreeInfo>,
            ) -> Result<bool, RegistryError> {
                Ok(false)
            }
        }

        #[test]
        fn test_concurrent_performance_comparison() {
            const NUM_THREADS: usize = 8;
            const OPERATIONS_PER_THREAD: usize = 1000;
            const NUM_PIDS: usize = NUM_THREADS * OPERATIONS_PER_THREAD;

            let dashmap_storage = Arc::new(InProcessStorage::new());
            let legacy_storage = Arc::new(LegacyInProcessStorage::new());

            // 测试DashMap性能
            let start = Instant::now();
            let mut handles = Vec::new();

            for thread_id in 0..NUM_THREADS {
                let storage = Arc::clone(&dashmap_storage);
                let handle = thread::spawn(move || {
                    for i in 0..OPERATIONS_PER_THREAD {
                        let pid = (thread_id * OPERATIONS_PER_THREAD + i) as u32;
                        let record = TaskRecord::new(
                            Utc::now(),
                            format!("cmd_{}", pid),
                            format!("/tmp/log_{}.log", pid),
                            Some(pid),
                        );
                        storage.register(pid, &record).unwrap();
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
            let dashmap_duration = start.elapsed();

            // 测试Mutex<HashMap>性能
            let start = Instant::now();
            let mut handles = Vec::new();

            for thread_id in 0..NUM_THREADS {
                let storage = Arc::clone(&legacy_storage);
                let handle = thread::spawn(move || {
                    for i in 0..OPERATIONS_PER_THREAD {
                        let pid = (thread_id * OPERATIONS_PER_THREAD + i) as u32;
                        let record = TaskRecord::new(
                            Utc::now(),
                            format!("cmd_{}", pid),
                            format!("/tmp/log_{}.log", pid),
                            Some(pid),
                        );
                        storage.register(pid, &record).unwrap();
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
            let legacy_duration = start.elapsed();

            // 验证结果
            let dashmap_entries = dashmap_storage.entries().unwrap();
            let legacy_entries = legacy_storage.entries().unwrap();

            assert_eq!(dashmap_entries.len(), NUM_PIDS);
            assert_eq!(legacy_entries.len(), NUM_PIDS);

            println!("=== 并发性能测试结果 ===");
            println!("DashMap:   {:?} ({} 操作)", dashmap_duration, NUM_PIDS);
            println!("Mutex<HashMap>: {:?} ({} 操作)", legacy_duration, NUM_PIDS);

            if dashmap_duration < legacy_duration {
                let speedup =
                    legacy_duration.as_nanos() as f64 / dashmap_duration.as_nanos() as f64;
                println!("DashMap 速度提升: {:.2}x", speedup);
            }

            // DashMap应该更快或至少不相差太大
            assert!(
                dashmap_duration <= legacy_duration * 2,
                "DashMap performance regression detected"
            );
        }
    }
}
