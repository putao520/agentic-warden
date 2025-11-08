#![allow(dead_code)] // 任务注册表，部分API函数当前未使用

use crate::config::{MAX_RECORD_AGE, SHARED_MEMORY_SIZE, SHARED_NAMESPACE};
use crate::core::process_tree::get_root_parent_pid_cached;
use crate::core::shared_map::{open_or_create, SharedMapError};
use crate::error::AgenticWardenError;
use crate::logging::{debug, warn};
use crate::task_record::{TaskRecord, TaskStatus};
use crate::utils::get_instance_id;
use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex as ParkingMutex;
use shared_hashmap::SharedMemoryHashMap;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex, OnceLock, Weak};
use thiserror::Error;

/// Represents a connected task registry with metadata
#[derive(Debug)]
pub struct ConnectedRegistry {
    pub instance_id: u32,
    pub process_id: u32,
    pub registry: Arc<TaskRegistry>,
}

/// Represents a task from any registry with additional context
#[derive(Debug, Clone)]
pub struct GlobalTaskEntry {
    #[allow(dead_code)]
    pub instance_id: u32,
    #[allow(dead_code)]
    pub process_id: u32,
    pub task_id: u32,
    pub task: TaskRecord,
}

#[derive(Debug, Clone)]
pub struct TaskRegistry {
    inner: Arc<RegistryInner>,
}

#[derive(Debug)]
struct RegistryInner {
    #[allow(dead_code)]
    namespace: String,
    map: ParkingMutex<SharedMemoryHashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub pid: u32,
    pub key: String,
    pub record: TaskRecord,
}

#[derive(Debug)]
pub struct CleanupEvent {
    pub _pid: u32,
    pub record: TaskRecord,
    pub reason: CleanupReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupReason {
    ProcessExited,
    Timeout,
    ManagerMissing,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("shared task map init failed: {0}")]
    Shared(String),
    #[error("shared hashmap operation failed: {0}")]
    Map(String),
    #[error("registry mutex poisoned")]
    Poison,
    #[error("record serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
}

impl From<shared_hashmap::Error> for RegistryError {
    fn from(value: shared_hashmap::Error) -> Self {
        RegistryError::Map(value.to_string())
    }
}

impl From<crate::core::process_tree::ProcessTreeError> for RegistryError {
    fn from(value: crate::core::process_tree::ProcessTreeError) -> Self {
        RegistryError::Map(format!("Process tree error: {}", value))
    }
}

impl From<AgenticWardenError> for RegistryError {
    fn from(value: AgenticWardenError) -> Self {
        RegistryError::Map(format!("Agentic error: {}", value))
    }
}

impl From<SharedMapError> for RegistryError {
    fn from(value: SharedMapError) -> Self {
        RegistryError::Shared(value.to_string())
    }
}

/// Get current process PID
#[allow(dead_code)]
fn get_current_process_pid() -> u32 {
    std::process::id()
}

/// Test if a registry contains valid agentic-warden task entries
#[allow(dead_code)]
fn test_registry_validity(
    map: &SharedMemoryHashMap<String, String>,
) -> Result<Vec<(String, String)>, RegistryError> {
    let mut valid_entries = Vec::new();

    // Sample a few entries to check if they look like task records
    for (key, value) in map.iter().take(10) {
        // Try to parse as a task record
        if serde_json::from_str::<TaskRecord>(&value).is_ok() {
            valid_entries.push((key.clone(), value.clone()));
        }
    }

    Ok(valid_entries)
}

impl TaskRegistry {
    pub fn connect() -> Result<Self, RegistryError> {
        let root_parent_pid = get_root_parent_pid_cached()?;
        let namespace = format!("{}-{}", SHARED_NAMESPACE, root_parent_pid);
        Self::connect_with_namespace(namespace)
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn connect_test(namespace: &str) -> Result<Self, RegistryError> {
        Self::connect_with_namespace(namespace.to_string())
    }

    fn connect_with_namespace(namespace: String) -> Result<Self, RegistryError> {
        if let Some(existing) = registry_pool_lookup(&namespace) {
            return Ok(Self { inner: existing });
        }

        let map = open_or_create(&namespace, SHARED_MEMORY_SIZE)?;
        let inner = Arc::new(RegistryInner {
            namespace: namespace.clone(),
            map: ParkingMutex::new(map),
        });

        let pool = registry_pool();
        let mut guard = pool.lock().map_err(|_poisoned| {
            // Mutex poisoning indicates a panic occurred while holding the lock
            // We can attempt to recover by accessing the poisoned data
            tracing::error!(
                "Registry pool mutex is poisoned, attempting recovery for namespace '{}'",
                namespace
            );

            // Return a specific error that can be handled by the caller
            RegistryError::Poison
        }).or_else(|e| {
            // Attempt to recover from poisoned mutex
            // In this case, we'll try to continue with the existing registry
            // or create a new one if recovery isn't possible
            tracing::warn!("Attempting to recover from poisoned registry pool");
            Err(e)
        })?;

        match guard.entry(namespace.clone()) {
            Entry::Occupied(mut entry) => {
                if let Some(existing) = entry.get().upgrade() {
                    return Ok(Self { inner: existing });
                }
                entry.insert(Arc::downgrade(&inner));
            }
            Entry::Vacant(entry) => {
                entry.insert(Arc::downgrade(&inner));
            }
        }

        Ok(Self { inner })
    }

    pub fn register(&self, pid: u32, record: &TaskRecord) -> Result<(), RegistryError> {
        let key = pid.to_string();
        let value = serde_json::to_string(record)?;
        self.with_map(|map| {
            map.try_insert(key.clone(), value)?;
            Ok(())
        })
    }

    pub fn mark_completed(
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

    pub fn remove(&self, pid: u32) -> Result<Option<TaskRecord>, RegistryError> {
        let key = pid.to_string();
        let removed = self.with_map(|map| Ok(map.remove(&key)))?;
        match removed {
            Some(text) => Ok(Some(serde_json::from_str(&text)?)),
            None => Ok(None),
        }
    }

    pub fn remove_by_pid(&self, pid: u32) -> Result<Option<TaskRecord>, RegistryError> {
        self.remove(pid)
    }

    pub fn entries(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
        let snapshot: Vec<(String, String)> = {
            let guard = self.inner.map.lock();
            guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let mut entries = Vec::new();
        let mut invalid_keys = Vec::new();

        for (key, value) in snapshot {
            match key.parse::<u32>() {
                Ok(pid) => match serde_json::from_str::<TaskRecord>(&value) {
                    Ok(record) => entries.push(RegistryEntry { pid, key, record }),
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

    pub fn get_completed_unread_tasks(&self) -> Result<Vec<(u32, TaskRecord)>, RegistryError> {
        let entries = self.entries()?;
        Ok(entries
            .into_iter()
            .filter_map(|entry| {
                (entry.record.status == TaskStatus::CompletedButUnread)
                    .then_some((entry.pid, entry.record))
            })
            .collect())
    }

    pub fn sweep_stale_entries<F>(
        &self,
        now: DateTime<Utc>,
        process_alive: F,
        terminate: &dyn Fn(u32),
    ) -> Result<Vec<CleanupEvent>, RegistryError>
    where
        F: Fn(u32) -> bool,
    {
        let entries = self.entries()?;
        let mut removals = Vec::new();
        let mut events = Vec::new();

        for entry in entries {
            let mut reason = None;
            if !process_alive(entry.pid) {
                reason = Some(CleanupReason::ProcessExited);
            } else {
                if let Some(manager_pid) = entry
                    .record
                    .manager_pid
                    .filter(|&manager_pid| manager_pid != entry.pid && !process_alive(manager_pid))
                {
                    debug(format!(
                        "manager pid={manager_pid} missing, terminating Codex child pid={}",
                        entry.pid
                    ));
                    terminate(entry.pid);
                    reason = Some(CleanupReason::ManagerMissing);
                }
                if reason.is_none() {
                    let age = now.signed_duration_since(entry.record.started_at);
                    if age > Duration::from_std(MAX_RECORD_AGE).unwrap_or(Duration::zero()) {
                        debug(format!(
                            "pid={} exceeded age {:.1}h, performing timeout cleanup",
                            entry.pid,
                            age.num_minutes() as f64 / 60.0
                        ));
                        terminate(entry.pid);
                        reason = Some(CleanupReason::Timeout);
                    }
                }
            }

            if let Some(reason) = reason {
                removals.push(entry.key.clone());
                events.push(CleanupEvent {
                    _pid: entry.pid,
                    record: entry.record.with_cleanup_reason(match reason {
                        CleanupReason::ProcessExited => "process_exited",
                        CleanupReason::Timeout => "timeout_cleanup",
                        CleanupReason::ManagerMissing => "manager_missing",
                    }),
                    reason,
                });
            }
        }

        if !removals.is_empty() {
            self.remove_keys(&removals)?;
        }

        Ok(events)
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

    fn with_map<T>(
        &self,
        f: impl FnOnce(&mut SharedMemoryHashMap<String, String>) -> Result<T, RegistryError>,
    ) -> Result<T, RegistryError> {
        let mut guard = self.inner.map.lock();
        f(&mut guard)
    }

    /// 发现所有可用的注册表（全局扫描）
    pub fn discover_all_registries() -> Result<Vec<ConnectedRegistry>, anyhow::Error> {
        let mut registries = Vec::new();

        // 获取当前进程的注册表作为基础
        let current_instance_id = get_instance_id();

        // 尝试连接到从1到100范围内可能的注册表（大大减少扫描次数）
        for instance_id in 1..=100 {
            if instance_id == current_instance_id {
                continue; // 跳过当前实例
            }

            // 尝试连接到该实例的共享内存
            match Self::connect_to_instance(instance_id) {
                Ok(connected) => {
                    // 检查该实例是否有任务，没有任务则不显示
                    let entries = connected.registry.entries().unwrap_or_default();
                    if !entries.is_empty() {
                        println!(
                            "发现实例 {} (PID: {}) - {} 个任务",
                            instance_id,
                            connected.process_id,
                            entries.len()
                        );
                        registries.push(connected);
                    }
                }
                Err(_) => {
                    // 连接失败，说明该实例不存在或不可访问
                }
            }
        }

        Ok(registries)
    }

    /// 连接到特定实例的注册表
    pub fn connect_to_instance(instance_id: u32) -> Result<ConnectedRegistry, anyhow::Error> {
        let namespace = format!("agentic-warden-{}", instance_id);

        // 尝试连接到已存在的共享内存
        let registry =
            TaskRegistry::connect_with_namespace(namespace).map_err(anyhow::Error::from)?;

        // 获取该实例的进程ID（从任务信息中推断）
        let process_id = instance_id; // 简化：使用实例ID作为进程ID

        Ok(ConnectedRegistry {
            instance_id,
            process_id,
            registry: Arc::new(registry),
        })
    }

    /// 获取所有全局任务
    pub fn get_all_global_tasks() -> Result<Vec<GlobalTaskEntry>, anyhow::Error> {
        let registries = Self::discover_all_registries()?;
        let mut global_tasks = Vec::new();

        // 添加当前实例的任务
        let current_instance_id = get_instance_id();
        let namespace = format!("agentic-warden-{}", current_instance_id);
        let current_registry = TaskRegistry::connect_with_namespace(namespace)?;
        let current_entries = current_registry.entries().unwrap_or_default();

        for entry in current_entries {
            global_tasks.push(GlobalTaskEntry {
                instance_id: current_instance_id,
                process_id: std::process::id(),
                task_id: entry.pid,
                task: entry.record,
            });
        }

        // 添加其他实例的任务
        for connected_registry in registries {
            let entries = connected_registry.registry.entries().unwrap_or_default();

            for entry in entries {
                global_tasks.push(GlobalTaskEntry {
                    instance_id: connected_registry.instance_id,
                    process_id: connected_registry.process_id,
                    task_id: entry.pid,
                    task: entry.record,
                });
            }
        }

        Ok(global_tasks)
    }
}

fn registry_pool() -> &'static StdMutex<HashMap<String, Weak<RegistryInner>>> {
    static REGISTRY_POOL: OnceLock<StdMutex<HashMap<String, Weak<RegistryInner>>>> =
        OnceLock::new();
    REGISTRY_POOL.get_or_init(|| StdMutex::new(HashMap::new()))
}

fn registry_pool_lookup(namespace: &str) -> Option<Arc<RegistryInner>> {
    let pool = registry_pool();
    match pool.lock() {
        Ok(guard) => guard.get(namespace).and_then(|weak| weak.upgrade()),
        Err(_) => None,
    }
}
