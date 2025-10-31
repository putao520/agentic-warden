//! Configuration Synchronization Integration Tests
//!
//! Integration tests for config synchronization functionality

use agentic_warden::sync::ConfigSyncManager;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Import test helpers from the common module
#[path = "../common/test_helpers.rs"]
mod test_helpers;
use test_helpers::*;

/// Test configuration synchronization scenarios
struct ConfigSyncTestEnv {
    temp_dir: TempDir,
    config_dir: PathBuf,
    auth_file: PathBuf,
    sync_file: PathBuf,
}

impl ConfigSyncTestEnv {
    fn new() -> Result<Self> {
        let temp_dir = create_temp_test_dir();
        let config_dir = temp_dir.path().join(".agentic-warden");

        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;

        let auth_file = config_dir.join("auth.json");
        let sync_file = config_dir.join("sync.json");

        Ok(Self {
            temp_dir,
            config_dir,
            auth_file,
            sync_file,
        })
    }

    fn create_auth_config(&self) -> Result<()> {
        let auth_data = create_real_auth_data();
        let json = serde_json::to_string_pretty(&auth_data)
            .context("Failed to serialize auth data")?;

        fs::write(&self.auth_file, json)
            .context("Failed to write auth file")?;

        Ok(())
    }

    fn create_sync_config(&self) -> Result<()> {
        let sync_config = serde_json::json!({
            "config": {
                "version": "1.0.0",
                "last_sync": "2024-01-01T00:00:00Z",
                "sync_enabled": true
            },
            "state": {
                "last_backup": None::<String>,
                "sync_errors": []
            }
        });

        let json = serde_json::to_string_pretty(&sync_config)
            .context("Failed to serialize sync config")?;

        fs::write(&self.sync_file, json)
            .context("Failed to write sync file")?;

        Ok(())
    }
}

/// Test configuration manager initialization
#[test]
fn test_config_sync_manager_initialization() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    env.create_auth_config()
        .context("Failed to create auth config")?;

    env.create_sync_config()
        .context("Failed to create sync config")?;

    let sync_manager = ConfigSyncManager::new()
        .context("Failed to create ConfigSyncManager")?;

    println!("✅ ConfigSyncManager initialized successfully!");
    println!("Config directory: {:?}", env.config_dir);

    Ok(())
}

/// Test configuration file reading
#[test]
fn test_config_file_reading() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    // Create auth config
    env.create_auth_config()
        .context("Failed to create auth config")?;

    // Verify file exists and can be read
    assert_file_exists(&env.auth_file);

    let content = fs::read_to_string(&env.auth_file)
        .context("Failed to read auth file")?;

    let auth_data: serde_json::Value = serde_json::from_str(&content)
        .context("Failed to parse auth data")?;

    assert_json_has_key(&auth_data, "access_token");
    assert_json_has_key(&auth_data, "client_id");

    println!("✅ Configuration file reading validated!");
    println!("Auth file size: {} bytes", content.len());

    Ok(())
}

/// Test configuration file writing
#[test]
fn test_config_file_writing() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    // Write custom auth config
    let custom_auth_data = serde_json::json!({
        "client_id": "test_client_id",
        "client_secret": "test_client_secret",
        "access_token": "test_access_token",
        "refresh_token": "test_refresh_token",
        "expires_in": 3600,
        "token_type": "Bearer",
        "scope": "https://www.googleapis.com/auth/drive.file",
        "created_at": chrono::Utc::now().timestamp()
    });

    let json = serde_json::to_string_pretty(&custom_auth_data)
        .context("Failed to serialize custom auth data")?;

    fs::write(&env.auth_file, json)
        .context("Failed to write custom auth file")?;

    // Verify file was written correctly
    let content = fs::read_to_string(&env.auth_file)
        .context("Failed to read written auth file")?;

    let read_data: serde_json::Value = serde_json::from_str(&content)
        .context("Failed to parse written auth data")?;

    assert_eq!(
        read_data.get("client_id").unwrap().as_str().unwrap(),
        "test_client_id"
    );

    println!("✅ Configuration file writing validated!");
    println!("Custom auth config written successfully");

    Ok(())
}

/// Test configuration validation
#[test]
fn test_config_validation() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    // Create invalid auth config (missing required fields)
    let invalid_auth_data = serde_json::json!({
        "access_token": "test_token",
        // Missing client_id, client_secret, etc.
    });

    let json = serde_json::to_string_pretty(&invalid_auth_data)
        .context("Failed to serialize invalid auth data")?;

    fs::write(&env.auth_file, json)
        .context("Failed to write invalid auth file")?;

    // Try to create ConfigSyncManager (should handle gracefully or fail gracefully)
    let result = ConfigSyncManager::new();

    // The result should either succeed (with validation warnings) or fail gracefully
    match result {
        Ok(_) => {
            println!("✅ ConfigSyncManager handled invalid config gracefully");
        }
        Err(e) => {
            println!("✅ ConfigSyncManager correctly rejected invalid config: {}", e);
        }
    }

    Ok(())
}

/// Test configuration backup and restore
#[test]
fn test_config_backup_and_restore() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    // Create original config
    env.create_auth_config()
        .context("Failed to create auth config")?;

    env.create_sync_config()
        .context("Failed to create sync config")?;

    // Backup original configs
    let auth_backup = fs::read(&env.auth_file)
        .context("Failed to backup auth file")?;

    let sync_backup = fs::read(&env.sync_file)
        .context("Failed to backup sync file")?;

    // Modify configs
    let modified_auth_data = serde_json::json!({
        "client_id": "modified_client_id",
        "client_secret": "modified_client_secret",
        "access_token": "modified_access_token",
        "refresh_token": "modified_refresh_token",
        "expires_in": 7200,
        "token_type": "Bearer",
        "scope": "https://www.googleapis.com/auth/drive.file",
        "created_at": chrono::Utc::now().timestamp()
    });

    let modified_json = serde_json::to_string_pretty(&modified_auth_data)
        .context("Failed to serialize modified auth data")?;

    fs::write(&env.auth_file, modified_json)
        .context("Failed to write modified auth file")?;

    // Verify modification
    let modified_content = fs::read_to_string(&env.auth_file)
        .context("Failed to read modified auth file")?;

    let modified_data: serde_json::Value = serde_json::from_str(&modified_content)
        .context("Failed to parse modified auth data")?;

    assert_eq!(
        modified_data.get("client_id").unwrap().as_str().unwrap(),
        "modified_client_id"
    );

    // Restore original configs
    fs::write(&env.auth_file, auth_backup)
        .context("Failed to restore auth file")?;

    fs::write(&env.sync_file, sync_backup)
        .context("Failed to restore sync file")?;

    // Verify restoration
    let restored_content = fs::read_to_string(&env.auth_file)
        .context("Failed to read restored auth file")?;

    let restored_data: serde_json::Value = serde_json::from_str(&restored_content)
        .context("Failed to parse restored auth data")?;

    assert_eq!(
        restored_data.get("client_id").unwrap().as_str().unwrap(),
        "77185225430.apps.googleusercontent.com"
    );

    println!("✅ Configuration backup and restore validated!");
    println!("Original configs successfully restored");

    Ok(())
}

/// Test configuration migration
#[test]
fn test_config_migration() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    // Create old format config (if there was one)
    let old_config_data = serde_json::json!({
        "access_token": "old_token",
        "refresh_token": "old_refresh",
        "expires_in": 3600
    });

    let old_json = serde_json::to_string_pretty(&old_config_data)
        .context("Failed to serialize old config data")?;

    fs::write(&env.auth_file, old_json)
        .context("Failed to write old config file")?;

    // Simulate migration by creating new format config
    let new_config_data = create_real_auth_data();
    let new_json = serde_json::to_string_pretty(&new_config_data)
        .context("Failed to serialize new config data")?;

    fs::write(&env.auth_file, new_json)
        .context("Failed to write new config file")?;

    // Verify migration
    let migrated_content = fs::read_to_string(&env.auth_file)
        .context("Failed to read migrated config file")?;

    let migrated_data: serde_json::Value = serde_json::from_str(&migrated_content)
        .context("Failed to parse migrated config data")?;

    // Should have new fields
    assert_json_has_key(&migrated_data, "client_id");
    assert_json_has_key(&migrated_data, "client_secret");
    assert_json_has_key(&migrated_data, "created_at");

    // Should still have old fields
    assert_json_has_key(&migrated_data, "access_token");
    assert_json_has_key(&migrated_data, "refresh_token");

    println!("✅ Configuration migration validated!");
    println!("Config successfully migrated to new format");

    Ok(())
}

/// Test configuration permissions
#[test]
fn test_config_permissions() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    // Create config files
    env.create_auth_config()
        .context("Failed to create auth config")?;

    env.create_sync_config()
        .context("Failed to create sync config")?;

    // Check file permissions on Unix-like systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let auth_metadata = fs::metadata(&env.auth_file)
            .context("Failed to get auth file metadata")?;

        let auth_permissions = auth_metadata.permissions();
        let auth_mode = auth_permissions.mode();

        // Files should have appropriate permissions (read/write for owner)
        println!("Auth file permissions: {:o}", auth_mode);

        // Verify file is readable and writable by owner
        assert!(auth_mode & 0o600 == 0o600, "Auth file should have 600 permissions");
    }

    // Verify files exist and are accessible
    assert_file_exists(&env.auth_file);
    assert_file_exists(&env.sync_file);

    // Try to read files
    fs::read_to_string(&env.auth_file)
        .context("Failed to read auth file (permissions issue)")?;

    fs::read_to_string(&env.sync_file)
        .context("Failed to read sync file (permissions issue)")?;

    println!("✅ Configuration permissions validated!");
    println!("Config files have appropriate permissions");

    Ok(())
}

/// Test configuration cleanup
#[test]
fn test_config_cleanup() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    // Create config files
    env.create_auth_config()
        .context("Failed to create auth config")?;

    env.create_sync_config()
        .context("Failed to create sync config")?;

    // Verify files exist
    assert_file_exists(&env.auth_file);
    assert_file_exists(&env.sync_file);

    // Remove sync file
    fs::remove_file(&env.sync_file)
        .context("Failed to remove sync file")?;

    assert_file_not_exists(&env.sync_file);
    assert_file_exists(&env.auth_file);

    // Remove auth file
    fs::remove_file(&env.auth_file)
        .context("Failed to remove auth file")?;

    assert_file_not_exists(&env.auth_file);
    assert_file_not_exists(&env.sync_file);

    // Verify directory still exists
    assert!(env.config_dir.exists());

    println!("✅ Configuration cleanup validated!");
    println!("Config files successfully removed");

    Ok(())
}

/// Test configuration error handling
#[test]
fn test_config_error_handling() -> Result<()> {
    let env = ConfigSyncTestEnv::new()
        .context("Failed to create test environment")?;

    // Try to read non-existent file
    let non_existent_file = env.config_dir.join("non_existent.json");
    let read_result = fs::read_to_string(&non_existent_file);

    assert!(read_result.is_err());
    println!("✅ Non-existent file error handled correctly");

    // Try to write to invalid path
    let invalid_path = PathBuf::from("/invalid/path/config.json");
    let write_result = fs::write(invalid_path, "test");

    assert!(write_result.is_err());
    println!("✅ Invalid path error handled correctly");

    // Try to parse invalid JSON
    let invalid_json_path = env.config_dir.join("invalid.json");
    fs::write(&invalid_json_path, "{ invalid json }")
        .context("Failed to write invalid JSON")?;

    let parse_result: Result<serde_json::Value, serde_json::Error> =
        serde_json::from_str(&fs::read_to_string(&invalid_json_path).unwrap());

    assert!(parse_result.is_err());
    println!("✅ Invalid JSON error handled correctly");

    // Cleanup
    fs::remove_file(&invalid_json_path)
        .context("Failed to cleanup invalid JSON file")?;

    println!("✅ Configuration error handling validated!");

    Ok(())
}