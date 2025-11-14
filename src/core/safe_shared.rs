use crate::error::{AgenticResult, AgenticWardenError};
use anyhow::Result;
use fs4::FileExt;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SafeSharedError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Lock acquisition failed: {0}")]
    LockAcquisition(String),
    #[error("File state corrupted: {0}")]
    CorruptedState(String),
}

/// Safe file-based shared state using JSON serialization and file locks
///
/// This replaces the unsafe shared_hashmap with a modern, cross-platform
/// approach that uses file locking and JSON serialization for inter-process
/// communication.
pub struct SafeSharedState<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Clone + Eq + std::hash::Hash,
    V: Serialize + for<'de> Deserialize<'de> + Clone,
{
    file_path: PathBuf,
    lock_file: PathBuf,
    state: Arc<Mutex<HashMap<K, V>>>,
}

impl<K, V> SafeSharedState<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Clone + Eq + std::hash::Hash,
    V: Serialize + for<'de> Deserialize<'de> + Clone,
{
    /// Create or open a safe shared state with the given name
    pub fn open(name: &str) -> Result<Self, SafeSharedError> {
        let base_dir = std::env::temp_dir().join("agentic-warden");
        std::fs::create_dir_all(&base_dir)?;

        let file_path = base_dir.join(format!("{}.json", name));
        let lock_file = base_dir.join(format!("{}.lock", name));

        // Initialize empty state
        let state = Arc::new(Mutex::new(HashMap::new()));

        let mut shared_state = Self {
            file_path,
            lock_file,
            state,
        };

        // Load existing state if file exists
        shared_state.load_state()?;

        Ok(shared_state)
    }

    /// Load state from disk with file locking
    fn load_state(&self) -> Result<(), SafeSharedError> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.lock_file)?;

        // Acquire exclusive lock
        lock_file.lock_exclusive().map_err(|e| {
            SafeSharedError::LockAcquisition(format!("Failed to acquire lock: {}", e))
        })?;

        let result = (|| {
            let mut file = File::open(&self.file_path)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            if !contents.trim().is_empty() {
                let loaded_state: HashMap<K, V> = serde_json::from_str(&contents)?;
                *self.state.lock() = loaded_state;
            }

            Ok::<(), SafeSharedError>(())
        })();

        // Always release the lock
        drop(lock_file);

        result
    }

    /// Save state to disk with file locking
    fn save_state(&self) -> Result<(), SafeSharedError> {
        // Ensure directory exists
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.lock_file)?;

        // Acquire exclusive lock
        lock_file.lock_exclusive().map_err(|e| {
            SafeSharedError::LockAcquisition(format!("Failed to acquire lock: {}", e))
        })?;

        let result = (|| {
            let state_json = serde_json::to_string_pretty(&*self.state.lock())?;

            // Write to a temporary file first, then rename to avoid corruption
            let temp_path = self.file_path.with_extension("tmp");
            {
                let mut temp_file = File::create(&temp_path)?;
                temp_file.write_all(state_json.as_bytes())?;
                temp_file.sync_all()?;
            }

            // Atomic rename
            std::fs::rename(&temp_path, &self.file_path)?;

            Ok::<(), SafeSharedError>(())
        })();

        // Always release the lock
        drop(lock_file);

        result
    }

    /// Get a value from the shared state
    pub fn get(&self, key: &K) -> Option<V> {
        self.state.lock().get(key).cloned()
    }

    /// Insert a value into the shared state and persist to disk
    pub fn insert(&self, key: K, value: V) -> Result<(), SafeSharedError> {
        self.state.lock().insert(key.clone(), value);
        self.save_state()
    }

    /// Remove a value from the shared state and persist to disk
    pub fn remove(&self, key: &K) -> Option<V> {
        let result = self.state.lock().remove(key);
        if result.is_some() {
            let _ = self.save_state(); // Ignore save errors for remove operations
        }
        result
    }

    /// Check if a key exists in the shared state
    pub fn contains_key(&self, key: &K) -> bool {
        self.state.lock().contains_key(key)
    }

    /// Iterate over all key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (K, V)> {
        self.state.lock().clone().into_iter()
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.state.lock().len()
    }

    /// Check if the shared state is empty
    pub fn is_empty(&self) -> bool {
        self.state.lock().is_empty()
    }

    /// Clear all entries and persist to disk
    pub fn clear(&self) -> Result<(), SafeSharedError> {
        self.state.lock().clear();
        self.save_state()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_safe_shared_state_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
        let shared = SafeSharedState::<String, String>::open("test_basic")?;

        // Test insert and get
        shared.insert("key1".to_string(), "value1".to_string())?;
        assert_eq!(shared.get(&"key1".to_string()), Some("value1".to_string()));

        // Test contains_key
        assert!(shared.contains_key(&"key1".to_string()));
        assert!(!shared.contains_key(&"nonexistent".to_string()));

        // Test remove
        assert_eq!(shared.remove(&"key1".to_string()), Some("value1".to_string()));
        assert!(!shared.contains_key(&"key1".to_string()));

        // Cleanup
        shared.clear()?;

        Ok(())
    }

    #[test]
    fn test_safe_shared_state_persistence() -> Result<(), Box<dyn std::error::Error>> {
        let name = "test_persistence";

        {
            let shared1 = SafeSharedState::<String, String>::open(name)?;
            shared1.insert("persistent".to_string(), "data".to_string())?;
        }

        {
            let shared2 = SafeSharedState::<String, String>::open(name)?;
            assert_eq!(shared2.get(&"persistent".to_string()), Some("data".to_string()));
        }

        // Cleanup
        let shared = SafeSharedState::<String, String>::open(name)?;
        shared.clear()?;

        Ok(())
    }

    #[test]
    fn test_concurrent_access() -> Result<(), Box<dyn std::error::Error>> {
        let shared = Arc::new(SafeSharedState::<String, i32>::open("test_concurrent")?);
        let mut handles = vec![];

        // Spawn multiple threads that write to the shared state
        for i in 0..10 {
            let shared_clone = shared.clone();
            let handle = thread::spawn(move || -> Result<(), SafeSharedError> {
                for j in 0..10 {
                    let key = format!("thread_{}_key_{}", i, j);
                    shared_clone.insert(key, i * 10 + j)?;
                    thread::sleep(Duration::from_millis(1));
                }
                Ok(())
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap()?;
        }

        // Verify all data was written
        let mut count = 0;
        for (key, value) in shared.iter() {
            println!("{}: {}", key, value);
            count += 1;
        }
        assert_eq!(count, 100); // 10 threads × 10 keys each

        // Cleanup
        shared.clear()?;

        Ok(())
    }
}