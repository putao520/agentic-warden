//! Registry utilities and helper functions
//!
//! This module contains utility functions for working with shared memory registries.
//! The main TaskRegistry implementation has been moved to unified_registry to avoid duplication.

use crate::error::RegistryError;
use crate::task_record::TaskRecord;
use serde_json;
use shared_hashmap::SharedMemoryHashMap;

/// Get current process PID
#[allow(dead_code)]
pub fn get_current_process_pid() -> u32 {
    std::process::id()
}

/// Test if a registry contains valid agentic-warden task entries
#[allow(dead_code)]
pub fn test_registry_validity(
    map: &SharedMemoryHashMap<String, String>,
) -> Result<Vec<(String, String)>, RegistryError> {
    let mut valid_entries = Vec::new();

    // Sample a few entries to check if they look like task records
    for (key, value) in map.iter().take(10) {
        // Try to parse as a task record
        if serde_json::from_str::<TaskRecord>(&value).is_ok() {
            valid_entries.push((key.to_string(), value.to_string()));
        }
    }

    Ok(valid_entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SHARED_MEMORY_SIZE;
    use crate::core::shared_map::open_or_create;

    #[test]
    fn test_get_current_process_pid() {
        let pid = get_current_process_pid();
        assert!(pid > 0);
    }

    #[test]
    fn test_registry_validity_with_empty_registry() {
        let namespace = format!("test_validity_empty_{}", std::process::id());
        let map = open_or_create(&namespace, SHARED_MEMORY_SIZE).unwrap();

        let valid_entries = test_registry_validity(&map).unwrap();
        assert_eq!(valid_entries.len(), 0);
    }

    #[test]
    fn test_registry_validity_with_valid_entries() {
        let namespace = format!("test_validity_valid_{}", std::process::id());
        let mut map = open_or_create(&namespace, SHARED_MEMORY_SIZE).unwrap();

        // Insert a valid task record
        let record = TaskRecord::new(
            chrono::Utc::now(),
            "test_cmd".to_string(),
            "/tmp/test.log".to_string(),
            Some(100),
        );
        let json = serde_json::to_string(&record).unwrap();
        map.insert("123".to_string(), json);

        let valid_entries = test_registry_validity(&map).unwrap();
        assert_eq!(valid_entries.len(), 1);
        assert_eq!(valid_entries[0].0, "123");
    }

    #[test]
    fn test_registry_validity_with_invalid_entries() {
        let namespace = format!("test_validity_invalid_{}", std::process::id());
        let mut map = open_or_create(&namespace, SHARED_MEMORY_SIZE).unwrap();

        // Insert invalid data (not a task record)
        map.insert("invalid".to_string(), "not a json".to_string());
        map.insert(
            "wrong_type".to_string(),
            r#"{"type":"not_task_record"}"#.to_string(),
        );

        let valid_entries = test_registry_validity(&map).unwrap();
        assert_eq!(valid_entries.len(), 0);
    }
}
