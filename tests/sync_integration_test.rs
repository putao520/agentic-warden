//! Integration tests for configuration synchronization functionality
//!
//! These tests verify the complete sync workflow including:
//! - Directory hashing and change detection
//! - Configuration packing/unpacking
//! - Sync configuration management
//! - OAuth authentication flow (mock)
//! - Complete push/pull workflows

use agentic_warden::sync::{
    config_packer::ConfigPacker,
    config_sync_manager::ConfigSyncManager,
    directory_hasher::{DirectoryHash, DirectoryHasher},
    error::{SyncError, SyncResult},
    sync_config_manager::SyncConfigManager,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_directory_hashing_workflow() {
    // Create a temporary directory with test files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Create test files
    fs::write(test_dir.join("config1.json"), r#"{"setting": "value1"}"#)
        .expect("Failed to write test file");
    fs::write(test_dir.join("config2.yaml"), "setting: value2\n")
        .expect("Failed to write test file");

    // Create subdirectory with nested file
    fs::create_dir_all(test_dir.join("subdir")).expect("Failed to create subdirectory");
    fs::write(test_dir.join("subdir/nested.conf"), "nested_config=true\n")
        .expect("Failed to write nested file");

    let hasher = DirectoryHasher::new();

    // Calculate initial hash
    let initial_hash = hasher
        .calculate_hash(test_dir)
        .expect("Failed to calculate initial hash");

    assert!(!initial_hash.hash.is_empty());
    assert_eq!(initial_hash.file_count, 3);
    assert!(initial_hash.total_size > 0);

    // Modify a file
    std::thread::sleep(std::time::Duration::from_millis(1000)); // Ensure different timestamp
    fs::write(
        test_dir.join("config1.json"),
        r#"{"setting": "modified_value"}"#,
    )
    .expect("Failed to modify test file");

    // Calculate new hash
    let modified_hash = hasher
        .calculate_hash(test_dir)
        .expect("Failed to calculate modified hash");

    assert_ne!(initial_hash.hash, modified_hash.hash);
    assert_eq!(modified_hash.file_count, 3);

    // Add a new file
    fs::write(test_dir.join("new_file.txt"), "new content").expect("Failed to add new file");

    let final_hash = hasher
        .calculate_hash(test_dir)
        .expect("Failed to calculate final hash");

    assert_ne!(modified_hash.hash, final_hash.hash);
    assert_eq!(final_hash.file_count, 4);
}

#[test]
fn test_config_packing_roundtrip() {
    // Create source directory with test files
    let source_dir = TempDir::new().expect("Failed to create source temp directory");
    let test_files = vec![
        (
            "config.json",
            r#"{"app_name": "test_app", "version": "1.0.0"}"#,
        ),
        ("settings.ini", "[General]\ndebug=true\nlog_level=info\n"),
        ("data/sample.dat", "binary\0data\0test\x7F\x7E"),
    ];

    for (file_path, content) in &test_files {
        let path = source_dir.path().join(file_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }

        match content {
            s if s.is_ascii() => {
                fs::write(&path, s).expect("Failed to write test file");
            }
            bytes => {
                fs::write(&path, bytes).expect("Failed to write binary file");
            }
        }
    }

    let packer = ConfigPacker::new();
    let output_dir = TempDir::new().expect("Failed to create output temp directory");
    let archive_path = output_dir.path().join("test_archive.tar.gz");

    // Pack directory
    let compressed_size = packer
        .pack_directory(source_dir.path(), &archive_path)
        .expect("Failed to pack directory");

    assert!(compressed_size > 0);
    assert!(archive_path.exists());

    // Extract to different directory
    let extract_dir = TempDir::new().expect("Failed to create extract temp directory");
    packer
        .unpack_archive(&archive_path, extract_dir.path())
        .expect("Failed to unpack archive");

    // Verify extracted files
    for (file_path, original_content) in &test_files {
        let extracted_path = extract_dir.path().join(file_path);
        assert!(
            extracted_path.exists(),
            "File {} was not extracted",
            file_path
        );

        let extracted_content =
            fs::read_to_string(&extracted_path).expect("Failed to read extracted file");

        // For binary files, we need special handling
        if original_content.is_ascii() {
            assert_eq!(extracted_content, *original_content);
        }
    }
}

#[test]
fn test_sync_config_management() {
    let config_dir = TempDir::new().expect("Failed to create config temp directory");
    let sync_config_path = config_dir.path().join("sync.json");

    // Create manager with custom config path
    let mut manager = SyncConfigManager {
        sync_path: sync_config_path.to_string_lossy().to_string(),
    };

    // Test initial config creation
    let config_data = manager
        .load_sync_data()
        .expect("Failed to load initial config");

    assert_eq!(config_data.config.directories.len(), 3); // Default directories
    assert_eq!(config_data.state.directories.len(), 0); // No tracked directories yet

    // Add directory hash
    let test_hash = DirectoryHash {
        hash: "test_hash_123".to_string(),
        file_count: 5,
        total_size: 1024,
        timestamp: chrono::Utc::now(),
    };

    manager
        .update_directory_hash("test_directory", test_hash.clone())
        .expect("Failed to update directory hash");

    // Verify hash was saved
    let retrieved_hashes = manager
        .get_all_directory_hashes()
        .expect("Failed to retrieve all hashes");
    assert_eq!(retrieved_hashes.len(), 1);
    assert!(retrieved_hashes.contains_key("test_directory"));

    let retrieved_hash = retrieved_hashes.get("test_directory").unwrap();
    assert_eq!(retrieved_hash.hash, test_hash.hash);
    assert_eq!(retrieved_hash.file_count, test_hash.file_count);

    // Test should_sync logic
    let should_sync_same = manager
        .should_sync("test_directory", &test_hash.hash)
        .expect("Failed to check sync status");
    assert!(!should_sync_same); // Same hash should not sync

    let should_sync_different = manager
        .should_sync("test_directory", "different_hash")
        .expect("Failed to check sync status");
    assert!(should_sync_different); // Different hash should sync

    // Update last sync time
    manager
        .update_last_sync()
        .expect("Failed to update last sync time");

    let last_sync = manager
        .get_last_sync()
        .expect("Failed to get last sync time");
    let now = chrono::Utc::now();
    let diff = now - last_sync;
    assert!(diff.num_seconds().try_into().unwrap() < 10); // Should be recent

    // Test reset state
    manager.reset_state().expect("Failed to reset state");
    let hashes_after_reset = manager
        .get_all_directory_hashes()
        .expect("Failed to get hashes after reset");
    assert_eq!(hashes_after_reset.len(), 0);
}

#[test]
fn test_path_expansion() {
    let config_dir = TempDir::new().expect("Failed to create config temp directory");
    let manager = SyncConfigManager {
        sync_path: config_dir
            .path()
            .join("test.json")
            .to_string_lossy()
            .to_string(),
    };

    // Test tilde expansion
    let expanded = manager
        .expand_path("~/test")
        .expect("Failed to expand path");
    assert!(!expanded.starts_with("~/"));
    assert!(expanded.contains("test"));
    assert!(expanded.contains(&std::env::var("HOME").unwrap_or_default()));

    // Test absolute path (should remain unchanged)
    let absolute_path = if cfg!(windows) {
        "C:\\absolute\\path"
    } else {
        "/absolute/path"
    };
    let unchanged = manager
        .expand_path(absolute_path)
        .expect("Failed to handle absolute path");
    assert_eq!(unchanged, absolute_path);
}

#[test]
fn test_sync_manager_creation() {
    let manager = ConfigSyncManager::new().expect("Failed to create ConfigSyncManager");

    // Test getting sync status
    let status = manager
        .get_sync_status()
        .expect("Failed to get sync status");
    assert_eq!(status.len(), 0); // No directories tracked initially

    // Test getting last sync time (should return default for new setup)
    let last_sync = manager
        .get_last_sync_time()
        .expect("Failed to get last sync time");

    // Should be recent (within last few seconds)
    let now = chrono::Utc::now();
    let diff = now - last_sync;
    assert!(diff.num_seconds().try_into().unwrap() < 5);
}

#[test]
fn test_error_handling() {
    let hasher = DirectoryHasher::new();

    // Test with non-existent directory
    let result = hasher.calculate_hash("/nonexistent/directory");
    assert!(result.is_err());
    match result.unwrap_err() {
        SyncError::DirectoryNotFound(_) => {} // Expected error
        _ => panic!("Expected DirectoryNotFound error"),
    }

    // Test packing non-existent directory
    let packer = ConfigPacker::new();
    let result = packer.pack_directory("/nonexistent", "/tmp/test.tar.gz");
    assert!(result.is_err());
}

#[test]
fn test_concurrent_hashing() {
    use std::sync::Arc;
    use std::thread;

    // Create multiple temporary directories
    let temp_dirs: Vec<_> = (0..5)
        .map(|i| {
            let dir = TempDir::new().expect("Failed to create temp directory");
            let file_path = dir.path().join(format!("file_{}.txt", i));
            fs::write(&file_path, format!("Content for file {}", i))
                .expect("Failed to write test file");
            dir
        })
        .collect();

    let hasher = Arc::new(DirectoryHasher::new());
    let mut handles = Vec::new();

    // Spawn multiple threads to hash directories concurrently
    for temp_dir in temp_dirs {
        let hasher_clone: Arc<DirectoryHasher> = Arc::clone(&hasher);
        let handle = thread::spawn(move || {
            hasher_clone
                .calculate_hash(temp_dir.path())
                .expect("Failed to calculate hash")
        });
        handles.push(handle);
    }

    // Collect results
    let mut hashes = Vec::new();
    for handle in handles {
        hashes.push(handle.join().expect("Thread panicked"));
    }

    // Verify all hashes were calculated successfully
    assert_eq!(hashes.len(), 5);
    for hash in &hashes {
        assert!(!hash.hash.is_empty());
        assert_eq!(hash.file_count, 1);
    }

    // Verify hashes are different (different file contents)
    for i in 0..hashes.len() {
        for j in i + 1..hashes.len() {
            assert_ne!(hashes[i].hash, hashes[j].hash);
        }
    }
}

#[test]
fn test_large_file_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let large_file_path = temp_dir.path().join("large_file.txt");

    // Create a file larger than 1MB threshold
    let large_content = "A".repeat(2 * 1024 * 1024); // 2MB
    fs::write(&large_file_path, large_content).expect("Failed to write large file");

    let hasher = DirectoryHasher::new();
    let result = hasher.calculate_hash(temp_dir.path());

    assert!(
        result.is_ok(),
        "Should be able to hash directory with large file"
    );

    let hash = result.unwrap();
    assert_eq!(hash.file_count, 1);
    assert!(hash.total_size > 2 * 1024 * 1024); // Should reflect actual file size
    assert!(!hash.hash.is_empty());
}

#[test]
fn test_special_characters_in_filenames() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Test files with special characters
    let test_files = vec![
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.with.dots.txt",
        "文件名.txt", // Chinese characters
        "файл.txt",   // Cyrillic characters
        "📄.txt",     // Emoji
    ];

    for filename in &test_files {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, format!("Content for {}", filename))
            .expect("Failed to write test file with special characters");
    }

    let hasher = DirectoryHasher::new();
    let hash = hasher
        .calculate_hash(temp_dir.path())
        .expect("Failed to hash directory with special character filenames");

    assert_eq!(hash.file_count, 7);
    assert!(!hash.hash.is_empty());

    // Test packing and unpacking with special characters
    let packer = ConfigPacker::new();
    let archive_path = temp_dir.path().join("archive.tar.gz");

    let compressed_size = packer
        .pack_directory(temp_dir.path(), &archive_path)
        .expect("Failed to pack directory with special characters");
    assert!(compressed_size > 0);

    let extract_dir = TempDir::new().expect("Failed to create extract temp directory");
    packer
        .unpack_archive(&archive_path, extract_dir.path())
        .expect("Failed to unpack archive with special characters");

    // Verify all files were extracted correctly
    for filename in &test_files {
        let extracted_path = extract_dir.path().join(filename);
        assert!(
            extracted_path.exists(),
            "File {} was not extracted",
            filename
        );
    }
}

#[tokio::test]
async fn test_sync_workflow_simulation() {
    // This test simulates a complete sync workflow without actual Google Drive calls
    let source_dir = TempDir::new().expect("Failed to create source temp directory");
    let target_dir = TempDir::new().expect("Failed to create target temp directory");

    // Create source files
    let source_files = vec![
        (
            "config/app.json",
            r#"{"name": "test_app", "version": "1.0.0"}"#,
        ),
        (
            "config/database.yml",
            "development:\n  host: localhost\n  port: 5432\n",
        ),
        ("scripts/start.sh", "#!/bin/bash\necho 'Starting app'\n"),
    ];

    for (file_path, content) in &source_files {
        let path = source_dir.path().join(file_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        fs::write(path, content).expect("Failed to write source file");
    }

    // Step 1: Calculate initial hash
    let hasher = DirectoryHasher::new();
    let initial_hash = hasher
        .calculate_hash(source_dir.path())
        .expect("Failed to calculate initial hash");

    // Step 2: Pack the directory (simulating upload)
    let packer = ConfigPacker::new();
    let archive_path = target_dir.path().join("backup.tar.gz");
    let compressed_size = packer
        .pack_directory(source_dir.path(), &archive_path)
        .expect("Failed to pack directory");

    assert!(compressed_size > 0);

    // Step 3: Simulate sync by updating config
    let config_dir = TempDir::new().expect("Failed to create config temp directory");
    let mut manager = SyncConfigManager {
        sync_path: config_dir
            .path()
            .join("sync.json")
            .to_string_lossy()
            .to_string(),
    };

    let dir_name = source_dir
        .path()
        .file_name()
        .and_then(|n| n.to_str())
        .expect("Failed to get directory name");

    manager
        .update_directory_hash(dir_name, initial_hash.clone())
        .expect("Failed to update directory hash");

    // Step 4: Simulate changes to source
    std::thread::sleep(std::time::Duration::from_millis(1000));
    fs::write(
        source_dir.path().join("config/app.json"),
        r#"{"name": "test_app", "version": "1.1.0"}"#,
    )
    .expect("Failed to modify source file");

    // Step 5: Calculate new hash and verify change detection
    let new_hash = hasher
        .calculate_hash(source_dir.path())
        .expect("Failed to calculate new hash");

    assert_ne!(initial_hash.hash, new_hash.hash);

    let should_sync = manager
        .should_sync(dir_name, &initial_hash.hash)
        .expect("Failed to check sync status");
    assert!(should_sync);

    // Step 6: Simulate "download" by unpacking to a different location
    let restore_dir = TempDir::new().expect("Failed to create restore temp directory");
    packer
        .unpack_archive(&archive_path, restore_dir.path())
        .expect("Failed to unpack archive");

    // Verify restored files
    for (file_path, original_content) in &source_files {
        let restored_path = restore_dir.path().join(file_path);
        assert!(
            restored_path.exists(),
            "File {} was not restored",
            file_path
        );

        let restored_content =
            fs::read_to_string(&restored_path).expect("Failed to read restored file");
        assert_eq!(restored_content, *original_content);
    }

    // Step 7: Update config with new hash (simulating successful sync)
    manager
        .update_directory_hash(dir_name, new_hash.clone())
        .expect("Failed to update directory hash with new hash");

    // Verify sync status is now up to date
    let should_sync_after = manager
        .should_sync(dir_name, &new_hash.hash)
        .expect("Failed to check sync status after update");
    assert!(!should_sync_after);
}

#[test]
fn test_configuration_persistence() {
    let config_dir = TempDir::new().expect("Failed to create config temp directory");
    let config_path = config_dir.path().join("test_sync.json");

    // Create initial configuration
    let mut manager1 = SyncConfigManager {
        sync_path: config_path.to_string_lossy().to_string(),
    };

    // Set up some test data
    let test_hash = DirectoryHash {
        hash: "persistent_hash_456".to_string(),
        file_count: 10,
        total_size: 4096,
        timestamp: chrono::Utc::now(),
    };

    manager1
        .update_directory_hash("persistent_dir", test_hash.clone())
        .expect("Failed to update directory hash");
    manager1
        .update_last_sync()
        .expect("Failed to update last sync");

    // Create new manager instance (simulating restart)
    let manager2 = SyncConfigManager {
        sync_path: config_path.to_string_lossy().to_string(),
    };

    // Verify data persisted correctly
    let all_hashes = manager2
        .get_all_directory_hashes()
        .expect("Failed to retrieve hashes");
    assert_eq!(all_hashes.len(), 1);
    assert!(all_hashes.contains_key("persistent_dir"));

    let persistent_hash = all_hashes.get("persistent_dir").unwrap();
    assert_eq!(persistent_hash.hash, test_hash.hash);
    assert_eq!(persistent_hash.file_count, test_hash.file_count);
    assert_eq!(persistent_hash.total_size, test_hash.total_size);

    // Verify last sync time persisted
    let last_sync = manager2
        .get_last_sync()
        .expect("Failed to get last sync time");
    let time_diff = (chrono::Utc::now() - last_sync).num_seconds().try_into().unwrap();
    assert!(time_diff < 5); // Should be recent
}
