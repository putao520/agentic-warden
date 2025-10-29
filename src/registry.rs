use crate::config::{MAX_RECORD_AGE, SHARED_MEMORY_SIZE, SHARED_NAMESPACE};
use crate::logging::{debug, warn};
use crate::process_tree::get_root_parent_pid_cached;
use crate::shared_map::{SharedMapError, open_or_create};
use crate::task_record::{TaskRecord, TaskStatus};
use chrono::{DateTime, Duration, Utc};
use shared_hashmap::SharedMemoryHashMap;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug)]
pub struct TaskRegistry {
    map: Mutex<SharedMemoryHashMap<String, String>>,
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
    Shared(#[from] SharedMapError),
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

impl From<crate::process_tree::ProcessTreeError> for RegistryError {
    fn from(value: crate::process_tree::ProcessTreeError) -> Self {
        RegistryError::Map(format!("Process tree error: {}", value))
    }
}

impl TaskRegistry {
    pub fn connect() -> Result<Self, RegistryError> {
        let root_parent_pid = get_root_parent_pid_cached()?;
        let namespace = format!("{}-{}", SHARED_NAMESPACE, root_parent_pid);
        let map = open_or_create(&namespace, SHARED_MEMORY_SIZE)?;
        Ok(Self {
            map: Mutex::new(map),
        })
    }

    /// Connect to a test registry with a custom namespace (for testing purposes only)
    #[cfg(any(test, feature = "testing"))]
    pub fn connect_test(namespace: &str) -> Result<Self, RegistryError> {
        let map = open_or_create(namespace, 1024 * 1024)?; // 1MB for tests
        Ok(Self {
            map: Mutex::new(map),
        })
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
            let guard = self.map.lock().map_err(|_| RegistryError::Poison)?;
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
        let mut guard = self.map.lock().map_err(|_| RegistryError::Poison)?;
        f(&mut guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_record(pid: u32) -> TaskRecord {
        TaskRecord::new(
            Utc::now(),
            format!("test-{}", pid),
            format!("/tmp/test-{}.log", pid),
            Some(pid + 1000), // manager PID
        )
    }

    #[test]
    fn test_registry_register_and_remove() {
        let test_namespace = format!("test_registry_{}", std::process::id());
        let registry = TaskRegistry::connect_test(&test_namespace).unwrap();

        let test_pid = 12345;
        let record = create_test_record(test_pid);

        // Test registration
        registry.register(test_pid, &record).unwrap();

        // Test retrieval via entries
        let entries = registry.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pid, test_pid);
        assert_eq!(entries[0].record.log_id, record.log_id);

        // Test removal
        let removed_record = registry.remove(test_pid).unwrap();
        assert!(removed_record.is_some());
        assert_eq!(removed_record.unwrap().log_id, record.log_id);

        // Verify it's gone
        let entries = registry.entries().unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_registry_mark_completed() {
        let test_namespace = format!("test_completed_{}", std::process::id());
        let registry = TaskRegistry::connect_test(&test_namespace).unwrap();

        let test_pid = 54321;
        let record = create_test_record(test_pid);
        registry.register(test_pid, &record).unwrap();

        // Mark as completed
        let completed_time = Utc::now();
        let result = Some("Test completed successfully".to_string());
        let exit_code = Some(0);

        registry
            .mark_completed(test_pid, result.clone(), exit_code, completed_time)
            .unwrap();

        // Verify completion
        let entries = registry.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].record.status, TaskStatus::CompletedButUnread);
        assert_eq!(entries[0].record.result, result);
        assert_eq!(entries[0].record.exit_code, exit_code);
        assert_eq!(entries[0].record.completed_at, Some(completed_time));
    }

    #[test]
    fn test_get_completed_unread_tasks() {
        let test_namespace = format!("test_unread_{}", std::process::id());
        let registry = TaskRegistry::connect_test(&test_namespace).unwrap();

        let running_pid = 11111;
        let completed_pid = 22222;

        // Register a running task
        let running_record = create_test_record(running_pid);
        registry.register(running_pid, &running_record).unwrap();

        // Register and complete a task
        let completed_record = create_test_record(completed_pid);
        registry.register(completed_pid, &completed_record).unwrap();
        registry
            .mark_completed(completed_pid, Some("Done".to_string()), Some(0), Utc::now())
            .unwrap();

        // Get completed unread tasks
        let unread = registry.get_completed_unread_tasks().unwrap();
        assert_eq!(unread.len(), 1);
        assert_eq!(unread[0].0, completed_pid);
        assert_eq!(unread[0].1.status, TaskStatus::CompletedButUnread);
    }

    #[test]
    fn test_duplicate_register_overwrites() {
        let test_namespace = format!("test_duplicate_{}", std::process::id());
        let registry = TaskRegistry::connect_test(&test_namespace).unwrap();

        let test_pid = 99999;
        let record1 = create_test_record(test_pid);
        let record2 = TaskRecord::new(
            Utc::now(),
            "updated-record".to_string(),
            "/tmp/updated.log".to_string(),
            Some(88888),
        );

        // Register first record
        registry.register(test_pid, &record1).unwrap();
        let entries = registry.entries().unwrap();
        assert_eq!(entries[0].record.log_id, record1.log_id);

        // Register second record with same PID (should overwrite)
        registry.register(test_pid, &record2).unwrap();
        let entries = registry.entries().unwrap();
        assert_eq!(entries.len(), 1); // Still only one entry
        assert_eq!(entries[0].record.log_id, record2.log_id); // But with updated data
    }

    #[test]
    fn test_remove_nonexistent_returns_none() {
        let test_namespace = format!("test_nonexistent_{}", std::process::id());
        let registry = TaskRegistry::connect_test(&test_namespace).unwrap();

        let nonexistent_pid = 999999;
        let removed = registry.remove(nonexistent_pid).unwrap();
        assert!(removed.is_none());
    }
}
