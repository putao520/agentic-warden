//! Complete sync workflow integration tests
//!
//! These tests verify the entire configuration synchronization workflow
//! from start to finish, including error handling and edge cases.

use agentic_warden::sync::{
    config_sync_manager::ConfigSyncManager,
    error::{SyncError, SyncResult},
    sync_command::SyncCommand,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use tokio::time::{Duration, sleep};

/// Helper function to create a test directory with various configuration files
fn create_test_config_directory(base_dir: &Path, name: &str) -> SyncResult<String> {
    let config_dir = base_dir.join(name);
    fs::create_dir_all(&config_dir).map_err(|e| SyncError::IoError(e))?;

    // Create various config files with different formats
    let files = vec![
        (
            "config.json",
            r#"{"app": "test_app", "version": "1.0.0", "debug": true}"#,
        ),
        (
            "settings.yaml",
            "app:\n  name: test_app\n  version: 1.0.0\n  debug: true\n",
        ),
        (".env", "APP_NAME=test_app\nVERSION=1.0.0\nDEBUG=true\n"),
        (
            "config.toml",
            "[app]\nname = \"test_app\"\nversion = \"1.0.0\"\ndebug = true\n",
        ),
        (
            "subdir/nested.conf",
            "[nested_section]\nkey = value\nenabled = true\n",
        ),
    ];

    for (file_path, content) in files {
        let full_path = config_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| SyncError::IoError(e))?;
        }
        fs::write(&full_path, content).map_err(|e| SyncError::IoError(e))?;
    }

    Ok(config_dir.to_string_lossy().to_string())
}

/// Helper function to modify a file in the test directory
fn modify_test_file(config_dir: &str, file_path: &str, new_content: &str) -> SyncResult<()> {
    let full_path = Path::new(config_dir).join(file_path);
    fs::write(&full_path, new_content).map_err(|e| SyncError::IoError(e))?;
    Ok(())
}

#[tokio::test]
async fn test_complete_sync_workflow() {
    // Create temporary directories for testing
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_base = temp_dir.path().join("configs");
    fs::create_dir_all(&config_base).expect("Failed to create config base directory");

    // Create test configuration directories
    let app1_dir = create_test_config_directory(&config_base, "app1")
        .expect("Failed to create app1 config directory");
    let app2_dir = create_test_config_directory(&config_base, "app2")
        .expect("Failed to create app2 config directory");

    // Initialize sync manager
    let mut manager = ConfigSyncManager::new().expect("Failed to create ConfigSyncManager");

    // Step 1: Test initial sync status (should be empty)
    let initial_status = manager
        .get_sync_status()
        .expect("Failed to get initial sync status");
    assert_eq!(
        initial_status.len(),
        0,
        "Initial sync status should be empty"
    );

    // Step 2: Simulate directory sync for app1
    let app1_result = manager
        .push_directory(&app1_dir)
        .await
        .expect("Failed to sync app1 directory");

    // Verify sync result
    assert!(
        app1_result.changed,
        "App1 should be detected as changed (first sync)"
    );
    assert_eq!(app1_result.directory_name, "app1");
    assert!(
        !app1_result.message.is_empty(),
        "Should have a descriptive message"
    );

    // Step 3: Verify sync status is updated
    let status_after_app1 = manager
        .get_sync_status()
        .expect("Failed to get status after app1 sync");
    assert_eq!(
        status_after_app1.len(),
        1,
        "Should have 1 tracked directory"
    );
    assert!(
        status_after_app1.contains_key("app1"),
        "Should track app1 directory"
    );

    let app1_hash = status_after_app1.get("app1").unwrap();
    assert_eq!(app1_hash.file_count, 5, "Should track 5 files in app1");

    // Step 4: Sync app2 directory
    let app2_result = manager
        .push_directory(&app2_dir)
        .await
        .expect("Failed to sync app2 directory");

    assert!(
        app2_result.changed,
        "App2 should be detected as changed (first sync)"
    );
    assert_eq!(app2_result.directory_name, "app2");

    // Step 5: Verify both directories are tracked
    let status_after_app2 = manager
        .get_sync_status()
        .expect("Failed to get status after app2 sync");
    assert_eq!(
        status_after_app2.len(),
        2,
        "Should have 2 tracked directories"
    );
    assert!(status_after_app2.contains_key("app1"));
    assert!(status_after_app2.contains_key("app2"));

    // Step 6: Test no-change scenario - sync app1 again without changes
    let app1_unchanged_result = manager
        .push_directory(&app1_dir)
        .await
        .expect("Failed to sync app1 directory again");

    assert!(
        !app1_unchanged_result.changed,
        "App1 should not be detected as changed"
    );
    assert!(
        app1_unchanged_result
            .message
            .contains("No changes detected")
    );

    // Step 7: Test change detection - modify a file in app1
    sleep(Duration::from_millis(1000)).await; // Ensure different timestamp
    modify_test_file(
        &app1_dir,
        "config.json",
        r#"{"app": "test_app", "version": "1.1.0", "debug": true}"#,
    )
    .expect("Failed to modify test file");

    let app1_changed_result = manager
        .push_directory(&app1_dir)
        .await
        .expect("Failed to sync modified app1 directory");

    assert!(
        app1_changed_result.changed,
        "App1 should be detected as changed after modification"
    );

    // Step 8: Verify hash was updated
    let final_status = manager
        .get_sync_status()
        .expect("Failed to get final sync status");

    let app1_final_hash = final_status.get("app1").unwrap();
    assert_ne!(
        app1_hash.hash, app1_final_hash.hash,
        "Hash should be updated after change"
    );

    // Step 9: Test last sync time tracking
    let last_sync_time = manager
        .get_last_sync_time()
        .expect("Failed to get last sync time");

    let now = chrono::Utc::now();
    let time_diff = now - last_sync_time;
    assert!(time_diff.num_seconds().try_into().unwrap() < 10, "Last sync should be recent");

    // Step 10: Test directory listing
    let sync_dirs = manager
        .config_manager
        .get_sync_directories()
        .expect("Failed to get sync directories");
    // Should include default directories plus our test directories if configured
    assert!(
        !sync_dirs.is_empty(),
        "Should have at least some sync directories"
    );
}

#[tokio::test]
async fn test_sync_command_interface() {
    // Test the command interface that would be used by CLI
    let mut sync_cmd = SyncCommand::new().expect("Failed to create SyncCommand");

    // Test status command
    let status_result = sync_cmd
        .execute_status()
        .await
        .expect("Failed to execute status command");
    assert_eq!(status_result, 0, "Status command should return 0 (success)");

    // Test list directories command
    let list_result = sync_cmd
        .execute_list_directories()
        .expect("Failed to execute list directories command");
    assert_eq!(
        list_result, 0,
        "List directories command should return 0 (success)"
    );
}

#[tokio::test]
async fn test_error_handling_workflows() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mut manager = ConfigSyncManager::new().expect("Failed to create ConfigSyncManager");

    // Test with non-existent directory
    let nonexistent_dir = temp_dir
        .path()
        .join("nonexistent")
        .to_string_lossy()
        .to_string();
    let result = manager.push_directory(&nonexistent_dir).await;

    assert!(
        result.is_err(),
        "Should return error for non-existent directory"
    );
    match result.unwrap_err() {
        SyncError::DirectoryHashingError(_) | SyncError::DirectoryNotFound(_) => {
            // Expected error types
        }
        _ => panic!("Expected directory-related error"),
    }

    // Test with file instead of directory
    let test_file = temp_dir.path().join("test_file.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");

    let file_path = test_file.to_string_lossy().to_string();
    let result = manager.push_directory(&file_path).await;

    assert!(
        result.is_err(),
        "Should return error for file instead of directory"
    );
}

#[tokio::test]
async fn test_concurrent_sync_operations() {
    use std::sync::Arc;
    use tokio::task;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_base = temp_dir.path().join("concurrent_test");
    fs::create_dir_all(&config_base).expect("Failed to create config base directory");

    // Create multiple test directories
    let mut dirs = Vec::new();
    for i in 0..5 {
        let dir_path = create_test_config_directory(&config_base, &format!("app{}", i))
            .expect("Failed to create test directory");
        dirs.push(dir_path);
    }

    let manager = Arc::new(tokio::sync::Mutex::new(
        ConfigSyncManager::new().expect("Failed to create ConfigSyncManager"),
    ));

    // Spawn multiple tasks to sync directories concurrently
    let mut handles = Vec::new();
    for dir_path in dirs {
        let manager_clone: Arc<tokio::sync::Mutex<ConfigSyncManager>> = Arc::clone(&manager);
        let handle = task::spawn(async move {
            let mut mgr = manager_clone.lock().await;
            mgr.push_directory(&dir_path).await
        });
        handles.push(handle);
    }

    // Wait for all sync operations to complete
    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(result) => match result {
                Ok(sync_result) => {
                    assert!(
                        sync_result.changed,
                        "Each directory should be detected as changed on first sync"
                    );
                    results.push(sync_result.directory_name);
                }
                Err(e) => panic!("Sync operation failed: {}", e),
            },
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    // Verify all directories were synced
    assert_eq!(results.len(), 5, "Should have synced 5 directories");

    let final_manager = manager.lock().await;
    let final_status = final_manager
        .get_sync_status()
        .expect("Failed to get final status");
    assert_eq!(final_status.len(), 5, "Should have 5 tracked directories");
}

#[tokio::test]
async fn test_large_directory_sync() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let large_dir = temp_dir.path().join("large_config");
    fs::create_dir_all(&large_dir).expect("Failed to create large directory");

    // Create many files to test performance and handling of large directories
    for i in 0..100 {
        let file_path = large_dir.join(format!("config_{}.json", i));
        let content = format!(
            r#"{{"file_id": {}, "content": "test content for file {}"}}"#,
            i, i
        );
        fs::write(&file_path, content).expect("Failed to write test file");
    }

    // Create some subdirectories with files
    for subdir_num in 0..5 {
        let subdir = large_dir.join(format!("subdir_{}", subdir_num));
        fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

        for file_num in 0..10 {
            let file_path = subdir.join(format!("nested_{}.txt", file_num));
            let content = format!("Nested file {} in subdirectory {}", file_num, subdir_num);
            fs::write(&file_path, content).expect("Failed to write nested file");
        }
    }

    let mut manager = ConfigSyncManager::new().expect("Failed to create ConfigSyncManager");

    let sync_result = manager
        .push_directory(&large_dir.to_string_lossy().to_string())
        .await
        .expect("Failed to sync large directory");

    assert!(
        sync_result.changed,
        "Large directory should be detected as changed"
    );
    assert!(
        sync_result.file_size.is_some(),
        "Should have file size information"
    );

    // Verify all files were tracked
    let status = manager
        .get_sync_status()
        .expect("Failed to get sync status");

    let large_dir_name = large_dir
        .file_name()
        .and_then(|n| n.to_str())
        .expect("Failed to get directory name");

    let hash_info = status
        .get(large_dir_name)
        .expect("Directory should be tracked");
    assert_eq!(
        hash_info.file_count, 150,
        "Should track 150 files (100 + 5*10)"
    );
}

#[tokio::test]
async fn test_special_character_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let special_dir = temp_dir.path().join("special_chars");
    fs::create_dir_all(&special_dir).expect("Failed to create special directory");

    // Test files with various special characters
    let special_files = vec![
        ("file with spaces.txt", "content with spaces"),
        ("file-with-dashes.json", "{\"dash\": true}"),
        ("file_with_underscores.yaml", "underscore: true"),
        ("file.with.dots.ini", "[dots]\nenabled = true"),
        ("文件名.txt", "中文内容"),    // Chinese
        ("файл.txt", "Русский текст"), // Russian
        ("📄.txt", "Emoji content"),   // Emoji
        ("café.txt", "Café content"),  // Accented characters
    ];

    for (filename, content) in special_files {
        let file_path = special_dir.join(filename);
        fs::write(&file_path, content).expect("Failed to write special character file");
    }

    let mut manager = ConfigSyncManager::new().expect("Failed to create ConfigSyncManager");

    let sync_result = manager
        .push_directory(&special_dir.to_string_lossy().to_string())
        .await
        .expect("Failed to sync directory with special characters");

    assert!(
        sync_result.changed,
        "Directory with special characters should be detected as changed"
    );

    // Verify directory name handling
    let dir_name = sync_result.directory_name;
    assert_eq!(dir_name, "special_chars");
}

#[test]
fn test_sync_state_reset() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_dir = create_test_config_directory(temp_dir.path(), "test_app")
        .expect("Failed to create test directory");

    let mut manager = ConfigSyncManager::new().expect("Failed to create ConfigSyncManager");

    // Sync a directory to create state
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(async { manager.push_directory(&config_dir).await })
        .expect("Failed to sync directory");

    // Verify state exists
    let status_before = manager
        .get_sync_status()
        .expect("Failed to get status before reset");
    assert_eq!(status_before.len(), 1, "Should have 1 tracked directory");

    // Reset state
    manager
        .config_manager
        .reset_state()
        .expect("Failed to reset state");

    // Verify state is cleared
    let status_after = manager
        .get_sync_status()
        .expect("Failed to get status after reset");
    assert_eq!(
        status_after.len(),
        0,
        "Should have no tracked directories after reset"
    );

    // Last sync time should still be accessible
    let last_sync = manager
        .get_last_sync_time()
        .expect("Failed to get last sync time");
    let now = chrono::Utc::now();
    let time_diff = now - last_sync;
    assert!(
        time_diff.num_seconds().try_into().unwrap() < 10,
        "Last sync should be recent after reset"
    );
}

#[tokio::test]
async fn test_configuration_directory_expansion() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_base = temp_dir.path().join("expanded");
    fs::create_dir_all(&config_base).expect("Failed to create config base directory");

    let test_dir = create_test_config_directory(&config_base, "test_expansion")
        .expect("Failed to create test directory");

    let manager = ConfigSyncManager::new().expect("Failed to create ConfigSyncManager");

    // Test absolute path handling
    let absolute_path = test_dir.clone();
    let expanded_absolute = manager
        .config_manager
        .expand_path(&absolute_path)
        .expect("Failed to expand absolute path");
    assert_eq!(
        expanded_absolute, absolute_path,
        "Absolute path should remain unchanged"
    );

    // Test tilde expansion (if supported on the platform)
    let home_dir = dirs::home_dir();
    if let Some(home) = home_dir {
        let tilde_path = format!("~/test_config");
        let expanded_tilde = manager
            .config_manager
            .expand_path(&tilde_path)
            .expect("Failed to expand tilde path");
        assert!(expanded_tilde.starts_with(&home.to_string_lossy().to_string()));
        assert!(expanded_tilde.ends_with("test_config"));
    }
}
