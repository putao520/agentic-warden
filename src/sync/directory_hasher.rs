use super::error::{SyncError, SyncResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectoryHash {
    pub hash: String,
    pub file_count: usize,
    pub total_size: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct DirectoryHasher;

impl Default for DirectoryHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryHasher {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_hash<P: AsRef<Path>>(&self, directory: P) -> SyncResult<DirectoryHash> {
        let dir_path = directory.as_ref();

        if !dir_path.exists() {
            return Err(SyncError::directory_not_found(
                dir_path.to_string_lossy().to_string(),
            ));
        }

        if !dir_path.is_dir() {
            return Err(SyncError::directory_hashing(format!(
                "Path is not a directory: {}",
                dir_path.to_string_lossy()
            )));
        }

        let mut hasher = Sha256::new();
        let mut file_count = 0usize;
        let mut total_size = 0u64;

        // Walk through directory sorted by path for consistent hashing
        let mut entries: Vec<DirEntry> = WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        entries.sort_by(|a, b| a.path().cmp(b.path()));

        for entry in entries {
            let path = entry.path();
            let relative_path = path.strip_prefix(dir_path).map_err(|e| {
                SyncError::directory_hashing(format!("Failed to create relative path: {}", e))
            })?;

            // Add relative path to hash
            hasher.update(relative_path.to_string_lossy().as_bytes());
            hasher.update(b"\0"); // null separator

            // Get file metadata
            let metadata = fs::metadata(path).map_err(SyncError::io)?;

            let file_size = metadata.len();
            let modified_time = metadata.modified().map_err(SyncError::io)?;

            // Add file size and modified time to hash
            hasher.update(file_size.to_le_bytes());
            if let Ok(unix_time) = modified_time.duration_since(std::time::UNIX_EPOCH) {
                hasher.update(unix_time.as_secs().to_le_bytes());
            }

            // Read and hash file content for small files, for large files use a sampling approach
            if file_size <= 1024 * 1024 {
                // 1MB threshold
                let content = fs::read(path).map_err(SyncError::io)?;
                hasher.update(&content);
            } else {
                // For large files, hash first and last 4KB plus file size
                let mut file = fs::File::open(path).map_err(SyncError::io)?;

                let mut buffer = [0u8; 4096];

                // Read first 4KB
                use std::io::Read;
                let bytes_read = file.read(&mut buffer).map_err(SyncError::io)?;
                hasher.update(&buffer[..bytes_read]);

                // Seek to end - 4KB
                if file_size > 4096 {
                    use std::io::Seek;
                    file.seek(std::io::SeekFrom::End(-4096i64))
                        .map_err(SyncError::io)?;

                    let bytes_read = file.read(&mut buffer).map_err(SyncError::io)?;
                    hasher.update(&buffer[..bytes_read]);
                }
            }

            file_count += 1;
            total_size += file_size;
        }

        let hash_result = hasher.finalize();
        let hash_str = format!("{:x}", hash_result);

        Ok(DirectoryHash {
            hash: hash_str,
            file_count,
            total_size,
            timestamp: chrono::Utc::now(),
        })
    }

    #[allow(dead_code)]
    pub fn calculate_multiple_hashes<P: AsRef<Path>>(
        &self,
        directories: &[P],
    ) -> SyncResult<HashMap<String, DirectoryHash>> {
        let mut results = HashMap::new();

        for directory in directories {
            let dir_path = directory.as_ref();
            let dir_name = dir_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    SyncError::directory_hashing(format!(
                        "Invalid directory name: {}",
                        dir_path.to_string_lossy()
                    ))
                })?;

            let hash = self.calculate_hash(directory)?;
            results.insert(dir_name.to_string(), hash);
        }

        Ok(results)
    }

    #[allow(dead_code)]
    pub fn has_changed<P: AsRef<Path>>(
        &self,
        directory: P,
        previous_hash: &str,
    ) -> SyncResult<bool> {
        let current_hash = self.calculate_hash(directory)?;
        Ok(current_hash.hash != previous_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let hasher = DirectoryHasher::new();
        let result = hasher.calculate_hash(temp_dir.path()).unwrap();

        assert!(!result.hash.is_empty());
        assert_eq!(result.file_count, 1);
        assert!(result.total_size > 0);
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let hasher = DirectoryHasher::new();
        let result = hasher.calculate_hash(temp_dir.path()).unwrap();

        assert!(!result.hash.is_empty());
        assert_eq!(result.file_count, 0);
        assert_eq!(result.total_size, 0);
    }

    #[test]
    fn test_nonexistent_directory() {
        let hasher = DirectoryHasher::new();
        let result = hasher.calculate_hash("/nonexistent/path");
        assert!(result.is_err());
    }
}
