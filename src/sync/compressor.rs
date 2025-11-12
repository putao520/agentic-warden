//! Compressor Abstraction Layer
//!
//! Provides unified compressor interface supporting multiple compression formats (TAR.GZ, ZIP, etc.)
//! Ensures consistent interface and usage across different systems

use super::error::{SyncError, SyncResult};
use std::fs;
use std::path::Path;

/// Compressor abstraction trait
#[allow(dead_code)]
pub trait Compressor {
    /// Compress directory to file
    fn compress_directory(
        &self,
        source_dir: &Path,
        output_file: &Path,
    ) -> SyncResult<CompressionResult>;

    /// Extract archive to directory
    fn extract_archive(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> SyncResult<ExtractionResult>;

    /// Get compressor name
    fn name(&self) -> &'static str;

    /// Get supported file extensions
    fn file_extension(&self) -> &'static str;
}

/// Compression result
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// Compressed file size (bytes)
    pub compressed_size: u64,
    /// Original directory size (bytes)
    pub original_size: u64,
    /// Compression time spent (milliseconds)
    pub compression_time_ms: u64,
    /// Compression ratio
    pub compression_ratio: f64,
}

/// Extraction result
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Number of extracted files
    pub file_count: usize,
    /// Size after extraction (bytes)
    pub extracted_size: u64,
    /// Extraction time spent (milliseconds)
    pub extraction_time_ms: u64,
}

/// Compressor types
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// TAR.GZ format (Unix standard, good cross-platform compatibility)
    TarGz,
    /// ZIP format (Windows native support)
    Zip,
    /// 7Z format (high compression ratio)
    SevenZip,
}

#[allow(dead_code)]
impl CompressionType {
    /// Get compressor instance
    pub fn create_compressor(self) -> Box<dyn Compressor> {
        match self {
            CompressionType::TarGz => Box::new(TarGzCompressor::new()),
            CompressionType::Zip => Box::new(ZipCompressor::new()),
            CompressionType::SevenZip => Box::new(SevenZipCompressor::new()),
        }
    }

    /// Get file extension
    pub fn file_extension(self) -> &'static str {
        match self {
            CompressionType::TarGz => "tar.gz",
            CompressionType::Zip => "zip",
            CompressionType::SevenZip => "7z",
        }
    }

    /// Get default compressor type based on platform
    pub fn default_for_platform() -> Self {
        #[cfg(target_os = "windows")]
        {
            CompressionType::Zip
        }
        #[cfg(not(target_os = "windows"))]
        {
            CompressionType::TarGz
        }
    }
}

/// TAR.GZ compressor implementation (cross-platform)
#[allow(dead_code)]
pub struct TarGzCompressor;

impl TarGzCompressor {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for TarGzCompressor {
    fn compress_directory(
        &self,
        _source_dir: &Path,
        _output_file: &Path,
    ) -> SyncResult<CompressionResult> {
        // Implementation would go here
        Err(SyncError::not_implemented())
    }

    fn extract_archive(
        &self,
        _archive_file: &Path,
        _target_dir: &Path,
    ) -> SyncResult<ExtractionResult> {
        // Implementation would go here
        Err(SyncError::not_implemented())
    }

    fn name(&self) -> &'static str {
        "TAR.GZ"
    }

    fn file_extension(&self) -> &'static str {
        "tar.gz"
    }
}

/// ZIP compressor implementation (cross-platform, prioritizes system zip tool)
#[allow(dead_code)]
pub struct ZipCompressor;

impl ZipCompressor {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for ZipCompressor {
    fn compress_directory(
        &self,
        _source_dir: &Path,
        _output_file: &Path,
    ) -> SyncResult<CompressionResult> {
        // Implementation would go here
        Err(SyncError::not_implemented())
    }

    fn extract_archive(
        &self,
        _archive_file: &Path,
        _target_dir: &Path,
    ) -> SyncResult<ExtractionResult> {
        // Implementation would go here
        Err(SyncError::not_implemented())
    }

    fn name(&self) -> &'static str {
        "ZIP"
    }

    fn file_extension(&self) -> &'static str {
        "zip"
    }
}

/// 7Z compressor implementation (requires 7z tool)
#[allow(dead_code)]
pub struct SevenZipCompressor;

impl SevenZipCompressor {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for SevenZipCompressor {
    fn compress_directory(
        &self,
        _source_dir: &Path,
        _output_file: &Path,
    ) -> SyncResult<CompressionResult> {
        // Implementation would go here
        Err(SyncError::not_implemented())
    }

    fn extract_archive(
        &self,
        _archive_file: &Path,
        _target_dir: &Path,
    ) -> SyncResult<ExtractionResult> {
        // Implementation would go here
        Err(SyncError::not_implemented())
    }

    fn name(&self) -> &'static str {
        "7Z"
    }

    fn file_extension(&self) -> &'static str {
        "7z"
    }
}

/// Compression utilities
#[allow(dead_code)]
pub mod utils {
    use super::*;

    /// Get directory size recursively
    pub fn get_directory_size(dir: &Path) -> SyncResult<u64> {
        let mut total_size = 0u64;

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    total_size += get_directory_size(&path)?;
                } else {
                    total_size += entry.metadata()?.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Calculate compression ratio
    pub fn calculate_compression_ratio(original: u64, compressed: u64) -> f64 {
        if original == 0 {
            0.0
        } else {
            (1.0 - (compressed as f64 / original as f64)) * 100.0
        }
    }

    /// Format bytes to human readable string
    /// Re-exported from common::utils for backward compatibility
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type_extensions() {
        assert_eq!(CompressionType::TarGz.file_extension(), "tar.gz");
        assert_eq!(CompressionType::Zip.file_extension(), "zip");
        assert_eq!(CompressionType::SevenZip.file_extension(), "7z");
    }

    #[test]
    fn test_format_bytes() {
        // Test the function in the utils namespace
        assert_eq!(utils::format_bytes(512), "512 B");
        assert_eq!(utils::format_bytes(1024), "1.00 KB");
        assert_eq!(utils::format_bytes(1048576), "1.00 MB");
    }

    #[test]
    fn test_compression_ratio() {
        assert_eq!(utils::calculate_compression_ratio(1000, 500), 50.0);
        assert_eq!(utils::calculate_compression_ratio(0, 0), 0.0);
    }
}
