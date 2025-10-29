use super::error::{SyncError, SyncResult};
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs;
use std::io::Write;
use std::path::Path;
use tar::Builder;

pub struct ConfigPacker;

impl Default for ConfigPacker {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigPacker {
    pub fn new() -> Self {
        Self
    }

    pub fn pack_directory<P: AsRef<Path>, O: AsRef<Path>>(
        &self,
        source_dir: P,
        output_file: O,
    ) -> SyncResult<u64> {
        let source_path = source_dir.as_ref();
        let output_path = output_file.as_ref();

        if !source_path.exists() {
            return Err(SyncError::DirectoryNotFound(
                source_path.to_string_lossy().to_string(),
            ));
        }

        if !source_path.is_dir() {
            return Err(SyncError::ConfigPackingError(format!(
                "Source path is not a directory: {}",
                source_path.to_string_lossy()
            )));
        }

        // Create parent directory for output file if it doesn't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SyncError::ConfigPackingError(format!("Failed to create output directory: {}", e))
            })?;
        }

        // Create tar.gz file
        let file = fs::File::create(output_path).map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to create output file: {}", e))
        })?;

        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar = Builder::new(encoder);

        // Add directory to tar
        tar.append_dir_all(".", source_path).map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to add directory to tar: {}", e))
        })?;

        // Finish tar and get compressed file size
        let encoder = tar.into_inner().map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to finish tar creation: {}", e))
        })?;

        let mut file = encoder.finish().map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to finish compression: {}", e))
        })?;

        file.flush().map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to flush output file: {}", e))
        })?;

        // Get file size
        let metadata = fs::metadata(output_path).map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to get output file metadata: {}", e))
        })?;

        Ok(metadata.len())
    }

    pub fn unpack_archive<P: AsRef<Path>, O: AsRef<Path>>(
        &self,
        archive_file: P,
        output_dir: O,
    ) -> SyncResult<()> {
        let archive_path = archive_file.as_ref();
        let output_path = output_dir.as_ref();

        if !archive_path.exists() {
            return Err(SyncError::ConfigPackingError(format!(
                "Archive file not found: {}",
                archive_path.to_string_lossy()
            )));
        }

        // Create output directory if it doesn't exist
        fs::create_dir_all(output_path).map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to create output directory: {}", e))
        })?;

        // Open and extract archive
        let file = fs::File::open(archive_path).map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to open archive file: {}", e))
        })?;

        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        archive.unpack(output_path).map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to unpack archive: {}", e))
        })?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_archive_info<P: AsRef<Path>>(&self, archive_file: P) -> SyncResult<ArchiveInfo> {
        let archive_path = archive_file.as_ref();

        if !archive_path.exists() {
            return Err(SyncError::ConfigPackingError(format!(
                "Archive file not found: {}",
                archive_path.to_string_lossy()
            )));
        }

        let metadata = fs::metadata(archive_path).map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to get archive metadata: {}", e))
        })?;

        let file = fs::File::open(archive_path).map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to open archive file: {}", e))
        })?;

        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        let mut file_count = 0usize;
        let mut total_uncompressed_size = 0u64;

        for entry in archive.entries().map_err(|e| {
            SyncError::ConfigPackingError(format!("Failed to read archive entries: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                SyncError::ConfigPackingError(format!("Failed to read archive entry: {}", e))
            })?;

            if entry.header().entry_type().is_file() {
                file_count += 1;
                total_uncompressed_size += entry.header().size().unwrap_or(0);
            }
        }

        Ok(ArchiveInfo {
            compressed_size: metadata.len(),
            uncompressed_size: total_uncompressed_size,
            file_count,
            created_at: metadata
                .modified()
                .unwrap_or(std::time::SystemTime::now())
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ArchiveInfo {
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub file_count: usize,
    pub created_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_pack_and_unpack() {
        let source_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();
        let archive_file = output_dir.path().join("test.tar.gz");

        // Create some test files
        fs::write(source_dir.path().join("file1.txt"), "Hello, World!").unwrap();
        fs::write(source_dir.path().join("file2.txt"), "Another file").unwrap();

        let packer = ConfigPacker::new();

        // Pack directory
        let compressed_size = packer
            .pack_directory(source_dir.path(), &archive_file)
            .unwrap();
        assert!(compressed_size > 0);
        assert!(archive_file.exists());

        // Get archive info
        let info = packer.get_archive_info(&archive_file).unwrap();
        assert_eq!(info.file_count, 2);
        assert!(info.compressed_size > 0);
        assert!(info.uncompressed_size > 0);

        // Unpack to different directory
        let extract_dir = TempDir::new().unwrap();
        packer
            .unpack_archive(&archive_file, extract_dir.path())
            .unwrap();

        // Verify unpacked files
        assert!(extract_dir.path().join("file1.txt").exists());
        assert!(extract_dir.path().join("file2.txt").exists());

        let content1 = fs::read_to_string(extract_dir.path().join("file1.txt")).unwrap();
        assert_eq!(content1, "Hello, World!");

        let content2 = fs::read_to_string(extract_dir.path().join("file2.txt")).unwrap();
        assert_eq!(content2, "Another file");
    }

    #[test]
    fn test_pack_nonexistent_directory() {
        let packer = ConfigPacker::new();
        let result = packer.pack_directory("/nonexistent/path", "output.tar.gz");
        assert!(result.is_err());
    }
}
