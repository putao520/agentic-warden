use crate::sync::directory_hasher::DirectoryHash;

/// Network connectivity status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkStatus {
    Online,
    Offline,
    Unknown,
}
use crate::sync::error::{SyncError, SyncResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const SYNC_FILE_NAME: &str = "sync.json";

/// Combined sync configuration and state persisted on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncData {
    pub config: SyncConfig,
    pub state: SyncState,
}

impl Default for SyncData {
    fn default() -> Self {
        Self {
            config: SyncConfig::default(),
            state: SyncState::default(),
        }
    }
}

/// Sync configuration describing what should be synchronised.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub directories: Vec<String>,
    pub auto_sync_enabled: bool,
    pub sync_interval_minutes: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            directories: vec![
                "~/.claude".to_string(),
                "~/.codex".to_string(),
                "~/.gemini".to_string(),
            ],
            auto_sync_enabled: false,
            sync_interval_minutes: 60,
        }
    }
}

/// Runtime sync state including cached hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub directories: HashMap<String, DirectoryHash>,
    pub last_sync: DateTime<Utc>,
    pub version: u32,
    pub network_status: Option<NetworkStatus>,
    pub network_last_checked: DateTime<Utc>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            directories: HashMap::new(),
            last_sync: Utc::now(),
            version: 1,
            network_status: None,
            network_last_checked: Utc::now(),
        }
    }
}

/// Resolve the default sync file path within the aiw directory.
pub fn default_sync_file_path() -> SyncResult<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        SyncError::sync_config("Could not determine the home directory".to_string())
    })?;

    let warden_dir = home_dir.join(".aiw");
    fs::create_dir_all(&warden_dir).map_err(|err| {
        SyncError::sync_config(format!("Failed to create config directory: {err}"))
    })?;

    Ok(warden_dir.join(SYNC_FILE_NAME))
}

/// Load sync data from the default location creating it if necessary.
pub fn load_sync_data() -> SyncResult<SyncData> {
    let path = default_sync_file_path()?;
    load_sync_data_from(&path)
}

/// Load sync data from a custom location.
pub fn load_sync_data_from(path: impl AsRef<Path>) -> SyncResult<SyncData> {
    let path = path.as_ref();

    if !path.exists() {
        let data = SyncData::default();
        save_sync_data_to(path, &data)?;
        return Ok(data);
    }

    let content = fs::read_to_string(path)
        .map_err(|err| SyncError::sync_config(format!("Failed to read sync file: {err}")))?;

    serde_json::from_str(&content)
        .map_err(|err| SyncError::sync_config(format!("Invalid sync file: {err}")))
}

/// Save sync data to the default location.
pub fn save_sync_data(data: &SyncData) -> SyncResult<()> {
    let path = default_sync_file_path()?;
    save_sync_data_to(&path, data)
}

/// Save sync data to a specific path (used by tests).
pub fn save_sync_data_to(path: impl AsRef<Path>, data: &SyncData) -> SyncResult<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            SyncError::sync_config(format!("Failed to create parent directory: {err}"))
        })?;
    }

    let content = serde_json::to_string_pretty(data)
        .map_err(|err| SyncError::sync_config(format!("Failed to serialise sync data: {err}")))?;

    fs::write(path, content)
        .map_err(|err| SyncError::sync_config(format!("Failed to write sync data: {err}")))
}

/// Save network status to the sync configuration.
pub fn save_network_status(status: NetworkStatus) -> SyncResult<()> {
    let mut data = load_sync_data()?;
    data.state.network_status = Some(status);
    data.state.network_last_checked = Utc::now();
    save_sync_data(&data)
}

/// Expand tilde based paths into absolute directories.
pub fn expand_path(path: &str) -> SyncResult<String> {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = dirs::home_dir().ok_or_else(|| {
            SyncError::sync_config("Could not determine the home directory".to_string())
        })?;
        Ok(home.join(stripped).to_string_lossy().into_owned())
    } else {
        Ok(path.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_data_is_created() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("sync.json");

        let data = load_sync_data_from(&file).unwrap();
        assert_eq!(data.config.directories.len(), 3);
        assert!(file.exists());
    }

    #[test]
    fn can_save_and_load_custom_state() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("sync.json");

        let mut data = SyncData::default();
        data.state.directories.insert(
            "test".to_string(),
            DirectoryHash {
                hash: "abc123".to_string(),
                file_count: 0,
                total_size: 0,
                timestamp: Utc::now(),
            },
        );

        save_sync_data_to(&file, &data).unwrap();

        let loaded = load_sync_data_from(&file).unwrap();
        assert_eq!(loaded.state.directories["test"].hash, "abc123");
    }

    #[test]
    fn expand_path_handles_tilde() {
        let expanded = expand_path("~/documents").unwrap();
        assert!(!expanded.starts_with("~/"));
        assert!(expanded.ends_with("documents"));
    }

    #[test]
    fn should_sync_defaults_to_true() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("sync.json");

        save_sync_data_to(&file, &SyncData::default()).unwrap();
        assert!(should_sync_with_file(&file, "missing", "hash").unwrap());
    }

    fn should_sync_with_file(file: &Path, name: &str, hash: &str) -> SyncResult<bool> {
        let data = load_sync_data_from(file)?;
        Ok(match data.state.directories.get(name) {
            Some(entry) => entry.hash != hash,
            None => true,
        })
    }
}
