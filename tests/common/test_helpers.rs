//! Common Test Utilities
//!
//! Essential utilities for testing agentic-warden functionality

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::iter;
use tempfile::TempDir;
use anyhow::{Context, Result};

// ============================================================================
// Directory and File Management
// ============================================================================

/// Create a temporary test directory
pub fn create_temp_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary test directory")
}

/// Create test configuration files in a directory
pub fn create_test_config_files(dir: &Path) {
    let config_data = serde_json::json!({
        "name": "test_config",
        "version": "1.0.0",
        "settings": {
            "debug": true,
            "timeout": 30
        }
    });

    let config_path = dir.join("test_config.json");
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&config_data).unwrap(),
    )
    .expect("Failed to write test config file");

    let db_config_data = serde_json::json!({
        "database": {
            "host": "localhost",
            "port": 5432
        }
    });

    let db_config_path = dir.join("database_config.json");
    fs::write(
        &db_config_path,
        serde_json::to_string_pretty(&db_config_data).unwrap(),
    )
    .expect("Failed to write database config file");

    let readme_content = "Test configuration directory\n\nContains test configuration files.";
    fs::write(dir.join("README.txt"), readme_content)
        .expect("Failed to write README file");
}

/// Count files in a directory recursively
pub fn count_files(dir: &Path) -> std::io::Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            count += 1;
        } else if path.is_dir() {
            count += count_files(&path)?;
        }
    }
    Ok(count)
}

// ============================================================================
// Provider Test Utilities (Real Data)
// ============================================================================

#[cfg(test)]
use agentic_warden::provider::config::{AiType, Provider};

/// Create a real Provider for testing
#[cfg(test)]
pub fn create_real_provider(ai_type: AiType, description: &str) -> Provider {
    let mut env = HashMap::new();

    match ai_type {
        AiType::Codex => {
            env.insert("OPENAI_API_KEY".to_string(), "sk-proj-real-test-key-123456".to_string());
            env.insert("OPENAI_BASE_URL".to_string(), "https://api.openai.com/v1".to_string());
            env.insert("OPENAI_ORG_ID".to_string(), "org-real-test-123".to_string());
        }
        AiType::Claude => {
            env.insert("ANTHROPIC_API_KEY".to_string(), "sk-ant-real-test-key-123456".to_string());
            env.insert("ANTHROPIC_BASE_URL".to_string(), "https://api.anthropic.com".to_string());
        }
        AiType::Gemini => {
            env.insert("GOOGLE_API_KEY".to_string(), "AIza-real-test-key-123456".to_string());
        }
    }

    Provider {
        description: description.to_string(),
        compatible_with: vec![ai_type],
        env,
    }
}

/// Create a real multi-AI provider
#[cfg(test)]
pub fn create_real_multi_provider(description: &str) -> Provider {
    let mut env = HashMap::new();
    env.insert("OPENAI_API_KEY".to_string(), "sk-proj-real-test-key-123456".to_string());
    env.insert("OPENAI_BASE_URL".to_string(), "https://api.openai.com/v1".to_string());
    env.insert("OPENAI_ORG_ID".to_string(), "org-real-test-123".to_string());
    env.insert("ANTHROPIC_API_KEY".to_string(), "sk-ant-real-test-key-123456".to_string());
    env.insert("ANTHROPIC_BASE_URL".to_string(), "https://api.anthropic.com".to_string());
    env.insert("GOOGLE_API_KEY".to_string(), "AIza-real-test-key-123456".to_string());

    Provider {
        description: description.to_string(),
        compatible_with: vec![AiType::Codex, AiType::Claude, AiType::Gemini],
        env,
    }
}

// ============================================================================
// Authentication Test Utilities (Real Data)
// ============================================================================

/// Create real authentication data for testing
pub fn create_real_auth_data() -> serde_json::Value {
    serde_json::json!({
        "client_id": "77185225430.apps.googleusercontent.com",
        "client_secret": "d-FL95Q19q7MQmFpd7hHD0Ty",
        "access_token": "ya29.real_access_token_for_testing_1234567890",
        "refresh_token": "1//real_refresh_token_for_testing_1234567890",
        "expires_in": 3600,
        "token_type": "Bearer",
        "scope": "https://www.googleapis.com/auth/drive.file",
        "created_at": chrono::Utc::now().timestamp()
    })
}

/// Get the standard auth file path
pub fn get_auth_path() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot find home directory")
        .join(".agentic-warden")
        .join("auth.json")
}

/// Create a real auth.json file
pub fn create_real_auth_file() -> std::io::Result<PathBuf> {
    let auth_path = get_auth_path();
    let auth_dir = auth_path.parent().unwrap();

    fs::create_dir_all(auth_dir)?;
    fs::write(
        &auth_path,
        serde_json::to_string_pretty(&create_real_auth_data())?,
    )?;

    Ok(auth_path)
}

/// Remove the auth.json file
pub fn cleanup_auth_file() -> std::io::Result<()> {
    let auth_path = get_auth_path();
    if auth_path.exists() {
        fs::remove_file(auth_path)?;
    }
    Ok(())
}

/// Backup existing auth file
pub fn backup_auth_file() -> std::io::Result<Option<Vec<u8>>> {
    let auth_path = get_auth_path();
    if auth_path.exists() {
        Ok(Some(fs::read(&auth_path)?))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Compression Test Utilities (Real Data)
// ============================================================================

/// Create a real compressed archive for testing
pub fn create_real_compressed_archive(source_dir: &Path, format: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    use agentic_warden::sync::compressor::CompressionType;

    let archive_path = source_dir.with_extension(format);

    let compression_type = match format {
        "tar.gz" | "tgz" => CompressionType::TarGz,
        "zip" => CompressionType::Zip,
        _ => return Err("Unsupported compression format".into()),
    };

    let compressor = compression_type.create_compressor();
    let result = compressor.compress_directory(source_dir, &archive_path)?;

    if result.compressed_size == 0 {
        return Err("Compression failed - no data written".into());
    }

    if result.compression_ratio > 0.95 && result.original_size > 1000 {
        return Err("Compression ratio too poor - possible error".into());
    }

    Ok(archive_path)
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that a file contains specific content
pub fn assert_file_contains(file_path: &Path, content: &str) {
    let file_content = fs::read_to_string(file_path)
        .expect(&format!("Failed to read file: {:?}", file_path));

    assert!(
        file_content.contains(content),
        "File {:?} does not contain expected content: {}",
        file_path,
        content
    );
}

/// Assert that a file exists
pub fn assert_file_exists(path: &Path) {
    assert!(path.exists(), "File should exist: {:?}", path);
}

/// Assert that a file does not exist
pub fn assert_file_not_exists(path: &Path) {
    assert!(!path.exists(), "File should not exist: {:?}", path);
}

/// Assert that two JSON values are equal
pub fn assert_json_eq(expected: &serde_json::Value, actual: &serde_json::Value) {
    assert_eq!(
        expected,
        actual,
        "JSON values are not equal.\nExpected: {}\nActual: {}",
        serde_json::to_string_pretty(expected).unwrap(),
        serde_json::to_string_pretty(actual).unwrap()
    );
}

/// Assert that a JSON value contains a specific key
pub fn assert_json_has_key(json: &serde_json::Value, key: &str) {
    assert!(
        json.get(key).is_some(),
        "JSON does not contain key: {}. JSON: {}",
        key,
        serde_json::to_string_pretty(json).unwrap()
    );
}

/// Assert that a HashMap contains a specific key
pub fn assert_hashmap_contains<K, V>(map: &HashMap<K, V>, key: &K)
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
{
    assert!(
        map.contains_key(key),
        "HashMap does not contain key: {:?}",
        key
    );
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Format bytes into human readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["bytes", "KB", "MB", "GB"];
    const THRESHOLD: f64 = 1024.0;

    if bytes < 1024 {
        return format!("{} bytes", bytes);
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Measure execution time of a function
pub fn measure_time<F, R>(f: F) -> (R, std::time::Duration)
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Generate a random alphanumeric string
#[cfg(test)]
pub fn generate_random_string(length: usize) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut rng = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let _hasher = DefaultHasher::new();

    iter::repeat_with(|| {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (rng % 36) as usize;
        if idx < 10 {
            char::from_digit(idx as u32, 10).unwrap()
        } else {
            ((idx - 10) as u8 + b'a') as char
        }
    })
    .take(length)
    .collect()
}

// ============================================================================
// Test Modules
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_test_dir() {
        let temp_dir = create_temp_test_dir();
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_create_test_config_files() {
        let temp_dir = create_temp_test_dir();
        create_test_config_files(temp_dir.path());

        assert_file_exists(&temp_dir.path().join("test_config.json"));
        assert_file_exists(&temp_dir.path().join("database_config.json"));
        assert_file_exists(&temp_dir.path().join("README.txt"));
    }

    #[test]
    fn test_assert_file_contains() {
        let temp_dir = create_temp_test_dir();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "Hello World!").unwrap();

        assert_file_contains(&test_file, "Hello");
        assert_file_contains(&test_file, "World");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_measure_time() {
        let (result, duration) = measure_time(|| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);
        assert!(duration >= std::time::Duration::from_millis(10));
    }

    #[test]
    fn test_create_real_auth_data() {
        let auth_data = create_real_auth_data();
        assert_json_has_key(&auth_data, "access_token");
        assert_json_has_key(&auth_data, "refresh_token");
        assert_json_has_key(&auth_data, "token_type");
        assert_json_has_key(&auth_data, "client_id");
        assert_json_has_key(&auth_data, "created_at");
    }

    #[test]
    fn test_generate_random_string() {
        let s1 = generate_random_string(10);
        let s2 = generate_random_string(10);

        assert_eq!(s1.len(), 10);
        assert_eq!(s2.len(), 10);
        assert_ne!(s1, s2); // Should be different
    }
}