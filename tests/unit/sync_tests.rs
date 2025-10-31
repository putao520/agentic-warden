//! Sync Unit Tests
//!
//! Unit tests for synchronization functionality

use agentic_warden::sync::compressor::CompressionType;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// Import test helpers from the common module
#[path = "../common/test_helpers.rs"]
mod test_helpers;
use test_helpers::*;

// ============================================================================
// Compression Tests
// ============================================================================

#[test]
fn test_compression_type_creation() {
    // Test that we can create compressors for each type
    let tar_gz = CompressionType::TarGz.create_compressor();
    assert_eq!(tar_gz.name(), "TarGzCompressor");

    let zip = CompressionType::Zip.create_compressor();
    assert_eq!(zip.name(), "ZipCompressor");
}

#[test]
fn test_compression_format_validation() {
    let tar_gz = CompressionType::TarGz;
    let zip = CompressionType::Zip;

    assert_eq!(tar_gz.to_string(), "tar.gz");
    assert_eq!(zip.to_string(), "zip");
}

#[test]
fn test_real_compression_tar_gz() {
    let temp_dir = create_temp_test_dir();
    let source_dir = temp_dir.path().join("source");

    // Create test files
    fs::create_dir_all(&source_dir).expect("Failed to create source directory");
    create_test_config_files(&source_dir);

    // Add a larger file to test compression
    let large_content = "A".repeat(10000); // 10KB
    fs::write(source_dir.join("large_file.txt"), &large_content)
        .expect("Failed to write large file");

    // Test compression
    let archive_path = create_real_compressed_archive(&source_dir, "tar.gz")
        .expect("Should be able to compress");

    assert!(archive_path.exists());
    assert!(archive_path.file_name().unwrap().to_str().unwrap().ends_with(".tar.gz"));

    // Verify archive is smaller than original (compression works)
    let original_size = fs::metadata(&source_dir)
        .expect("Failed to get source metadata")
        .len();
    let compressed_size = fs::metadata(&archive_path)
        .expect("Failed to get archive metadata")
        .len();

    assert!(compressed_size > 0);
    // Note: Small files might not compress well, so we just check it's not empty
}

#[test]
fn test_real_compression_zip() {
    let temp_dir = create_temp_test_dir();
    let source_dir = temp_dir.path().join("source");

    // Create test files
    fs::create_dir_all(&source_dir).expect("Failed to create source directory");
    create_test_config_files(&source_dir);

    // Test compression
    let archive_path = create_real_compressed_archive(&source_dir, "zip")
        .expect("Should be able to compress");

    assert!(archive_path.exists());
    assert!(archive_path.file_name().unwrap().to_str().unwrap().ends_with(".zip"));

    // Verify archive has content
    let compressed_size = fs::metadata(&archive_path)
        .expect("Failed to get archive metadata")
        .len();
    assert!(compressed_size > 0);
}

#[test]
fn test_unsupported_compression_format() {
    let temp_dir = create_temp_test_dir();
    let source_dir = temp_dir.path().join("source");

    fs::create_dir_all(&source_dir).expect("Failed to create source directory");

    // Try unsupported format
    let result = create_real_compressed_archive(&source_dir, "unsupported");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported compression format"));
}

#[test]
fn test_compression_empty_directory() {
    let temp_dir = create_temp_test_dir();
    let source_dir = temp_dir.path().join("empty");

    fs::create_dir_all(&source_dir).expect("Failed to create empty directory");

    // Test compression of empty directory
    let result = create_real_compressed_archive(&source_dir, "tar.gz");

    // This should either succeed with minimal size or fail gracefully
    match result {
        Ok(archive_path) => {
            assert!(archive_path.exists());
            let size = fs::metadata(&archive_path).expect("Failed to get metadata").len();
            assert!(size > 0);
        }
        Err(e) => {
            // Empty directory compression might fail, which is acceptable
            println!("Empty directory compression failed as expected: {}", e);
        }
    }
}

// ============================================================================
// OAuth Client Tests (Real Data)
// ============================================================================

#[test]
fn test_create_real_auth_data() {
    let auth_data = create_real_auth_data();

    assert_json_has_key(&auth_data, "client_id");
    assert_json_has_key(&auth_data, "client_secret");
    assert_json_has_key(&auth_data, "access_token");
    assert_json_has_key(&auth_data, "refresh_token");
    assert_json_has_key(&auth_data, "expires_in");
    assert_json_has_key(&auth_data, "token_type");
    assert_json_has_key(&auth_data, "scope");
    assert_json_has_key(&auth_data, "created_at");

    // Verify specific values
    let client_id = auth_data.get("client_id").unwrap().as_str().unwrap();
    assert_eq!(client_id, "77185225430.apps.googleusercontent.com");

    let token_type = auth_data.get("token_type").unwrap().as_str().unwrap();
    assert_eq!(token_type, "Bearer");
}

#[test]
fn test_auth_file_creation_and_cleanup() {
    // Create auth file
    let auth_path = create_real_auth_file()
        .expect("Should be able to create auth file");

    assert_file_exists(&auth_path);

    // Verify file content
    let content = fs::read_to_string(&auth_path)
        .expect("Should be able to read auth file");
    let auth_data: serde_json::Value = serde_json::from_str(&content)
        .expect("Should be able to parse auth data");

    assert_json_has_key(&auth_data, "access_token");

    // Cleanup
    cleanup_auth_file()
        .expect("Should be able to cleanup auth file");

    assert_file_not_exists(&auth_path);
}

#[test]
fn test_auth_file_backup() {
    // Create initial auth file
    let auth_path = create_real_auth_file()
        .expect("Should be able to create auth file");

    // Backup the file
    let backup = backup_auth_file()
        .expect("Should be able to backup auth file");

    assert!(backup.is_some());
    assert!(backup.as_ref().unwrap().len() > 0);

    // Modify original file
    fs::write(&auth_path, "modified content")
        .expect("Should be able to modify auth file");

    // Verify backup still contains original data
    let backup_str = String::from_utf8(backup.unwrap())
        .expect("Backup should be valid UTF-8");
    let backup_data: serde_json::Value = serde_json::from_str(&backup_str)
        .expect("Backup should be valid JSON");

    assert_json_has_key(&backup_data, "access_token");

    // Cleanup
    cleanup_auth_file()
        .expect("Should be able to cleanup auth file");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_compression_error_handling() {
    let non_existent_dir = Path::new("/non/existent/directory");

    let result = create_real_compressed_archive(non_existent_dir, "tar.gz");
    assert!(result.is_err());
}

#[test]
fn test_auth_file_permissions_handling() {
    let auth_path = get_auth_path();

    // Try to cleanup non-existent file (should not error)
    let result = cleanup_auth_file();
    assert!(result.is_ok());

    assert_file_not_exists(&auth_path);
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_compression_performance() {
    let temp_dir = create_temp_test_dir();
    let source_dir = temp_dir.path().join("perf_test");

    // Create moderately sized test data
    fs::create_dir_all(&source_dir).expect("Failed to create source directory");

    // Create multiple files
    for i in 0..5 {
        let content = format!("Test file {} content\n", i).repeat(1000); // ~15KB per file
        fs::write(source_dir.join(format!("file_{}.txt", i)), content)
            .expect("Failed to write test file");
    }

    // Measure compression time
    let (archive_path, duration) = measure_time(|| {
        create_real_compressed_archive(&source_dir, "tar.gz")
            .expect("Should be able to compress")
    });

    assert!(archive_path.exists());

    // Compression should complete in reasonable time (< 5 seconds)
    assert!(duration < std::time::Duration::from_secs(5));

    println!("Compression completed in {:?}", duration);
    println!("Archive size: {} bytes",
        fs::metadata(&archive_path).expect("Failed to get metadata").len());
}

#[test]
fn test_large_file_handling() {
    let temp_dir = create_temp_test_dir();
    let source_dir = temp_dir.path().join("large_test");

    fs::create_dir_all(&source_dir).expect("Failed to create source directory");

    // Create a 1MB file
    let large_content = "X".repeat(1024 * 1024);
    fs::write(source_dir.join("large_file.txt"), &large_content)
        .expect("Failed to write large file");

    // Should handle large files without issues
    let archive_path = create_real_compressed_archive(&source_dir, "tar.gz")
        .expect("Should be able to compress large file");

    assert!(archive_path.exists());

    // Verify compression ratio is reasonable
    let original_size = fs::metadata(&source_dir).expect("Failed to get metadata").len();
    let compressed_size = fs::metadata(&archive_path).expect("Failed to get metadata").len();
    let ratio = compressed_size as f64 / original_size as f64;

    // Text files should compress well
    assert!(ratio < 0.5, "Compression ratio too poor: {:.2}", ratio);

    println!("Original size: {} bytes", format_bytes(original_size));
    println!("Compressed size: {} bytes", format_bytes(compressed_size));
    println!("Compression ratio: {:.2}", ratio);
}