// 导入sync_config.rs中的数据结构定义，避免重复定义
use super::directory_hasher::DirectoryHash;
use super::error::{SyncError, SyncResult};
use super::sync_config::{SyncConfig, SyncData, SyncState};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct SyncConfigManager {
    sync_path: String,
}

impl SyncConfigManager {
    pub fn new() -> SyncResult<Self> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::sync_config("Could not find home directory".to_string()))?;

        let warden_dir = home_dir.join(".aiw");

        // Create directory if it doesn't exist
        fs::create_dir_all(&warden_dir).map_err(|e| {
            SyncError::sync_config(format!("Failed to create aiw directory: {}", e))
        })?;

        let sync_path = warden_dir.join("sync.json");

        Ok(Self {
            sync_path: sync_path.to_string_lossy().to_string(),
        })
    }

    /// Create a SyncConfigManager with a custom path for testing
    ///
    /// This method is primarily used in tests to create isolated configuration managers
    /// that don't interfere with the user's actual configuration files.
    /// It allows tests to use temporary configuration files instead of the real
    /// `~/.agentic-warden/sync.json` file.
    ///
    /// # Arguments
    /// * `path` - Path to the sync configuration file
    ///
    /// # Examples
    /// ```
/// use aiw::sync::sync_config_manager::SyncConfigManager;
    /// use std::path::Path;
    ///
    /// // In tests, use a temporary path
    /// let temp_path = Path::new("/tmp/test_sync.json");
    /// let manager = SyncConfigManager::with_path(temp_path);
    /// ```
    #[allow(dead_code)] // This method is used in tests
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            sync_path: path.as_ref().to_string_lossy().to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn load_config(&self) -> SyncResult<SyncConfig> {
        if !Path::new(&self.sync_path).exists() {
            let default_config = SyncConfig::default();
            self.save_config(&default_config)?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(&self.sync_path)
            .map_err(|e| SyncError::sync_config(format!("Failed to read config file: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| SyncError::sync_config(format!("Failed to parse config file: {}", e)))
    }

    #[allow(dead_code)]
    pub fn save_config(&self, config: &SyncConfig) -> SyncResult<()> {
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| SyncError::sync_config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&self.sync_path, content)
            .map_err(|e| SyncError::sync_config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Load unified sync data
    pub fn load_sync_data(&self) -> SyncResult<SyncData> {
        if !Path::new(&self.sync_path).exists() {
            let default_data = SyncData {
                config: SyncConfig::default(),
                state: SyncState::default(),
            };
            self.save_sync_data(&default_data)?;
            return Ok(default_data);
        }

        let content = fs::read_to_string(&self.sync_path)
            .map_err(|e| SyncError::sync_config(format!("Failed to read sync file: {}", e)))?;

        let data: SyncData = serde_json::from_str(&content)
            .map_err(|e| SyncError::sync_config(format!("Failed to parse sync file: {}", e)))?;

        Ok(data)
    }

    /// Save unified sync data
    pub fn save_sync_data(&self, data: &SyncData) -> SyncResult<()> {
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| SyncError::sync_config(format!("Failed to serialize sync data: {}", e)))?;

        fs::write(&self.sync_path, content)
            .map_err(|e| SyncError::sync_config(format!("Failed to write sync file: {}", e)))?;

        Ok(())
    }

    pub fn load_state(&self) -> SyncResult<SyncState> {
        Ok(self.load_sync_data()?.state)
    }

    pub fn save_state(&self, state: &SyncState) -> SyncResult<()> {
        let mut data = self.load_sync_data()?;
        data.state = state.clone();
        self.save_sync_data(&data)
    }

    pub fn update_directory_hash(
        &self,
        directory_name: &str,
        hash: DirectoryHash,
    ) -> SyncResult<()> {
        let mut state = self.load_state()?;
        state.directories.insert(directory_name.to_string(), hash);
        self.save_state(&state)
    }

    pub fn get_directory_hash(&self, directory_name: &str) -> SyncResult<Option<DirectoryHash>> {
        let state = self.load_state()?;
        Ok(state.directories.get(directory_name).cloned())
    }

    #[allow(dead_code)]
    pub fn get_all_directory_hashes(&self) -> SyncResult<HashMap<String, DirectoryHash>> {
        let state = self.load_state()?;
        Ok(state.directories)
    }

    #[allow(dead_code)]
    pub fn clear_directory_hash(&self, directory_name: &str) -> SyncResult<()> {
        let mut state = self.load_state()?;
        state.directories.remove(directory_name);
        self.save_state(&state)
    }

    #[allow(dead_code)]
    pub fn update_last_sync(&self) -> SyncResult<()> {
        let mut state = self.load_state()?;
        state.last_sync = chrono::Utc::now();
        self.save_state(&state)
    }

    #[allow(dead_code)]
    pub fn get_last_sync(&self) -> SyncResult<chrono::DateTime<chrono::Utc>> {
        let state = self.load_state()?;
        Ok(state.last_sync)
    }

    pub fn should_sync(&self, directory_name: &str, current_hash: &str) -> SyncResult<bool> {
        match self.get_directory_hash(directory_name)? {
            Some(stored_hash) => Ok(stored_hash.hash != current_hash),
            None => Ok(true), // No previous hash, should sync
        }
    }

    pub fn expand_path(&self, path: &str) -> SyncResult<String> {
        if path.starts_with("~/") {
            let home_dir = dirs::home_dir().ok_or_else(|| {
                SyncError::sync_config("Could not find home directory".to_string())
            })?;
            Ok(path.replace("~", &home_dir.to_string_lossy()))
        } else {
            Ok(path.to_string())
        }
    }

    pub fn get_sync_directories(&self) -> SyncResult<Vec<String>> {
        let sync_data = self.load_sync_data()?;
        let mut expanded_dirs = Vec::new();

        for dir in sync_data.config.directories {
            let expanded = self.expand_path(&dir)?;
            expanded_dirs.push(expanded);
        }

        Ok(expanded_dirs)
    }

    pub fn reset_state(&self) -> SyncResult<()> {
        let default_state = SyncState::default();
        self.save_state(&default_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let _config_path = temp_dir.path().join("config.json");
        let _state_path = temp_dir.path().join("state.json");

        // Create manager with custom sync_path (point to a file, not directory)
        let sync_file = temp_dir.path().join("sync.json");
        let manager = SyncConfigManager {
            sync_path: sync_file.to_string_lossy().to_string(),
        };

        // Test default sync data creation
        let sync_data = manager.load_sync_data().unwrap();
        assert_eq!(sync_data.config.directories.len(), 3);
        assert_eq!(sync_data.config.directories[0], "~/.claude");

        // Test config modification
        let mut modified_sync_data = sync_data.clone();
        modified_sync_data.config.auto_sync_enabled = true;
        manager.save_sync_data(&modified_sync_data).unwrap();

        let loaded_sync_data = manager.load_sync_data().unwrap();
        assert!(loaded_sync_data.config.auto_sync_enabled);

        // Test state persistence
        assert_eq!(loaded_sync_data.state.directories.len(), 0);

        let mut modified_sync_data = loaded_sync_data.clone();
        modified_sync_data.state.version = 2;
        manager.save_sync_data(&modified_sync_data).unwrap();

        let final_sync_data = manager.load_sync_data().unwrap();
        assert_eq!(final_sync_data.state.version, 2);
    }

    #[test]
    fn test_path_expansion() {
        let manager = SyncConfigManager::new().unwrap();

        let expanded = manager.expand_path("~/test").unwrap();
        assert!(!expanded.starts_with("~/"));
        assert!(expanded.contains("test"));

        let unchanged = manager.expand_path("/absolute/path").unwrap();
        assert_eq!(unchanged, "/absolute/path");
    }
}
