//! 压缩器抽象层
//!
//! 提供统一的压缩器接口，支持多种压缩格式（TAR.GZ, ZIP等）
//! 确保在不同系统下都有一致的接口和使用方法

use super::error::{SyncError, SyncResult};
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

/// 压缩器抽象trait
pub trait Compressor {
    /// 压缩目录到文件
    fn compress_directory(
        &self,
        source_dir: &Path,
        output_file: &Path,
    ) -> SyncResult<CompressionResult>;

    /// 解压文件到目录
    fn extract_archive(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> SyncResult<ExtractionResult>;

    /// 获取压缩器名称
    fn name(&self) -> &'static str;

    /// 获取支持的文件扩展名
    fn file_extension(&self) -> &'static str;
}

/// 压缩结果
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// 压缩后的文件大小（字节）
    pub compressed_size: u64,
    /// 原始目录大小（字节）
    pub original_size: u64,
    /// 压缩耗时（毫秒）
    pub compression_time_ms: u64,
    /// 压缩率
    pub compression_ratio: f64,
}

/// 解压结果
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// 解压的文件数量
    pub file_count: usize,
    /// 解压后的大小（字节）
    pub extracted_size: u64,
    /// 解压耗时（毫秒）
    pub extraction_time_ms: u64,
}

/// 压缩器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// TAR.GZ 格式（Unix标准，跨平台兼容性好）
    TarGz,
    /// ZIP 格式（Windows原生支持）
    Zip,
    /// 7Z 格式（高压缩比）
    SevenZip,
}

impl CompressionType {
    /// 获取压缩器实例
    pub fn create_compressor(self) -> Box<dyn Compressor> {
        match self {
            CompressionType::TarGz => Box::new(TarGzCompressor::new()),
            CompressionType::Zip => Box::new(ZipCompressor::new()),
            CompressionType::SevenZip => Box::new(SevenZipCompressor::new()),
        }
    }

    /// 获取文件扩展名
    pub fn file_extension(self) -> &'static str {
        match self {
            CompressionType::TarGz => "tar.gz",
            CompressionType::Zip => "zip",
            CompressionType::SevenZip => "7z",
        }
    }

    /// 获取默认压缩器
    pub fn default() -> Self {
        CompressionType::TarGz
    }
}

/// 从配置字符串解析压缩器类型
impl std::str::FromStr for CompressionType {
    type Err = SyncError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tar.gz" | "tgz" => Ok(CompressionType::TarGz),
            "zip" => Ok(CompressionType::Zip),
            "7z" => Ok(CompressionType::SevenZip),
            _ => Err(SyncError::ConfigPackingError(format!(
                "Unsupported compression format: {}. Supported: tar.gz, zip, 7z",
                s
            ))),
        }
    }
}

/// 压缩器工厂
pub struct CompressorFactory;

impl CompressorFactory {
    /// 根据配置创建压缩器
    pub fn create_from_config(compression_type: Option<CompressionType>) -> Box<dyn Compressor> {
        compression_type
            .unwrap_or_else(|| CompressionType::default())
            .create_compressor()
    }

    /// 根据文件扩展名推断压缩器
    pub fn infer_from_extension(file_path: &Path) -> SyncResult<Box<dyn Compressor>> {
        if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
            match ext.to_lowercase().as_str() {
                "gz" => {
                    // 检查是否是tar.gz格式
                    if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                        if stem.ends_with(".tar") {
                            Ok(CompressionType::TarGz.create_compressor())
                        } else {
                            // 单独的.gz文件，使用tar.gz
                            Ok(CompressionType::TarGz.create_compressor())
                        }
                    } else {
                        Ok(CompressionType::TarGz.create_compressor())
                    }
                }
                "zip" => Ok(CompressionType::Zip.create_compressor()),
                "7z" => Ok(CompressionType::SevenZip.create_compressor()),
                _ => Ok(CompressionType::default().create_compressor()),
            }
        } else {
            Ok(CompressionType::default().create_compressor())
        }
    }
}

/// TAR.GZ 压缩器实现
pub struct TarGzCompressor;

impl TarGzCompressor {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for TarGzCompressor {
    fn compress_directory(
        &self,
        source_dir: &Path,
        output_file: &Path,
    ) -> SyncResult<CompressionResult> {
        use super::config_packer::ConfigPacker;
        use std::time::Instant;

        let packer = ConfigPacker::new();
        let start_time = Instant::now();

        // 使用现有的ConfigPacker实现
        let compressed_size = packer.pack_directory(source_dir, output_file)?;

        let compression_time = start_time.elapsed().as_millis();
        let original_size = calculate_directory_size(source_dir)?;
        let compression_ratio = compressed_size as f64 / original_size as f64;

        Ok(CompressionResult {
            compressed_size,
            original_size,
            compression_time_ms: compression_time,
            compression_ratio,
        })
    }

    fn extract_archive(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> SyncResult<ExtractionResult> {
        use super::config_packer::ConfigPacker;
        use std::time::Instant;

        let packer = ConfigPacker::new();
        let start_time = Instant::now();

        packer.unpack_archive(archive_file, target_dir)?;

        let extraction_time = start_time.elapsed().as_millis();
        let (file_count, extracted_size) = count_files_and_size(target_dir)?;

        Ok(ExtractionResult {
            file_count,
            extracted_size,
            extraction_time_ms: extraction_time,
        })
    }

    fn name(&self) -> &'static str {
        "TAR.GZ"
    }

    fn file_extension(&self) -> &'static str {
        "tar.gz"
    }
}

/// ZIP 压缩器实现（跨平台，优先使用系统zip工具）
pub struct ZipCompressor {
    compression_level: u8,
}

impl ZipCompressor {
    pub fn new() -> Self {
        Self {
            compression_level: 6, // 默认压缩级别
        }
    }

    pub fn with_compression_level(level: u8) -> Self {
        Self {
            compression_level: level.min(9),
        }
    }
}

impl Compressor for ZipCompressor {
    fn compress_directory(
        &self,
        source_dir: &Path,
        output_file: &Path,
    ) -> SyncResult<CompressionResult> {
        use std::time::Instant;

        let start_time = Instant::now();

        // 优先尝试使用系统zip工具
        if let Ok(result) = self.compress_with_system_tool(source_dir, output_file) {
            return Ok(result);
        }

        // 回退到zip库
        self.compress_with_zip_library(source_dir, output_file)
    }

    fn extract_archive(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> SyncResult<ExtractionResult> {
        use std::time::Instant;

        let start_time = Instant::now();

        // 优先尝试使用系统解压工具
        if let Ok(result) = self.extract_with_system_tool(archive_file, target_dir) {
            return Ok(result);
        }

        // 回退到zip库
        self.extract_with_zip_library(archive_file, target_dir)
    }

    fn name(&self) -> &'static str {
        "ZIP"
    }

    fn file_extension(&self) -> &'static str {
        "zip"
    }
}

impl ZipCompressor {
    /// 使用系统zip工具压缩（Windows优先，macOS/Linux可用）
    fn compress_with_system_tool(
        &self,
        source_dir: &Path,
        output_file: &Path,
    ) -> SyncResult<CompressionResult> {
        let cmd = if cfg!(target_os = "windows") {
            // Windows: 使用PowerShell的Compress-Archive
            let power_shell = std::env::var("PSModulePath").is_ok();
            if power_shell {
                format!(
                    "powershell -Command \"Compress-Archive -Path '{}' -DestinationPath '{}' -CompressionLevel {}\"",
                    source_dir.display(),
                    output_file.display(),
                    self.compression_level
                )
            } else {
                // 回退到7-Zip（需要单独安装）
                format!(
                    "7z a -mx{} \"{}\" \"{}\"",
                    self.compression_level,
                    output_file.display(),
                    source_dir.display()
                )
            }
        } else if cfg!(target_os = "macos") {
            // macOS: 使用内置的zip命令
            format!(
                "cd \"{}\" && zip -r -{} \"{}\" .",
                source_dir
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .display(),
                self.compression_level,
                output_file.display()
            )
        } else {
            // Linux: 使用zip命令
            format!(
                "cd \"{}\" && zip -r -{} \"{}\" .",
                source_dir
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .display(),
                self.compression_level,
                output_file.display()
            )
        };

        let start_time = std::time::Instant::now();

        match std::process::Command::new("cmd")
            .args(&["/C", cmd])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let compressed_size = fs::metadata(output_file)?.len();
                    let original_size = calculate_directory_size(source_dir)?;
                    let compression_time = start_time.elapsed().as_millis();
                    let compression_ratio = compressed_size as f64 / original_size as f64;

                    Ok(CompressionResult {
                        compressed_size,
                        original_size,
                        compression_time_ms: compression_time,
                        compression_ratio,
                    })
                } else {
                    Err(SyncError::ConfigPackingError(format!(
                        "System zip command failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )))
                }
            }
            Err(e) => Err(SyncError::ConfigPackingError(format!(
                "Failed to execute system zip command: {}",
                e
            ))),
        }
    }

    /// 使用zip库压缩
    fn compress_with_zip_library(
        &self,
        source_dir: &Path,
        output_file: &Path,
    ) -> SyncResult<CompressionResult> {
        // TODO: 如果需要纯Rust实现，可以添加zip crate依赖
        Err(SyncError::ConfigPackingError(
            "zip library compression not implemented. Please install zip command.".to_string(),
        ))
    }

    /// 使用系统解压工具
    fn extract_with_system_tool(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> SyncResult<ExtractionResult> {
        let cmd = if cfg!(target_os = "windows") {
            // Windows: 使用PowerShell的Expand-Archive
            if std::env::var("PSModulePath").is_ok() {
                format!(
                    "powershell -Command \"Expand-Archive -Path '{}' -DestinationPath '{}'\"",
                    archive_file.display(),
                    target_dir.display()
                )
            } else {
                // 回退到7-Zip
                format!(
                    "7z x \"{}\" -o\"{}\"",
                    archive_file.display(),
                    target_dir.display()
                )
            }
        } else {
            // Unix系统: 使用unzip命令
            format!(
                "unzip \"{}\" -d \"{}\"",
                archive_file.display(),
                target_dir.display()
            )
        };

        let start_time = std::time::Instant::now();

        match std::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let extraction_time = start_time.elapsed().as_millis();
                    let (file_count, extracted_size) = count_files_and_size(target_dir)?;

                    Ok(ExtractionResult {
                        file_count,
                        extracted_size,
                        extraction_time_ms: extraction_time,
                    })
                } else {
                    Err(SyncError::ConfigPackingError(format!(
                        "System extract command failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )))
                }
            }
            Err(e) => Err(SyncError::ConfigPackingError(format!(
                "Failed to execute system extract command: {}",
                e
            ))),
        }
    }

    /// 使用zip库解压
    fn extract_with_zip_library(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> SyncResult<ExtractionResult> {
        // TODO: 如果需要纯Rust实现，可以添加zip crate依赖
        Err(SyncError::ConfigPackingError(
            "zip library extraction not implemented. Please install unzip command.".to_string(),
        ))
    }
}

/// 7Z 压缩器实现（使用7-Zip系统工具）
pub struct SevenZipCompressor {
    compression_level: u8,
}

impl SevenZipCompressor {
    pub fn new() -> Self {
        Self {
            compression_level: 9, // 最高压缩级别
        }
    }

    pub fn with_compression_level(level: u8) -> Self {
        Self {
            compression_level: level.min(9),
        }
    }
}

impl Compressor for SevenZipCompressor {
    fn compress_directory(
        &self,
        source_dir: &Path,
        output_file: &Path,
    ) -> SyncResult<CompressionResult> {
        use std::time::Instant;

        let start_time = Instant::now();

        // 7-Zip命令
        let cmd = format!(
            "7z a -mx{} \"{}\" \"{}\"",
            self.compression_level,
            output_file.display(),
            source_dir.display()
        );

        match std::process::Command::new("cmd")
            .args(&["/C", cmd])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let compressed_size = fs::metadata(output_file)?.len();
                    let original_size = calculate_directory_size(source_dir)?;
                    let compression_time = start_time.elapsed().as_millis();
                    let compression_ratio = compressed_size as f64 / original_size as f64;

                    Ok(CompressionResult {
                        compressed_size,
                        original_size,
                        compression_time_ms: compression_time,
                        compression_ratio,
                    })
                } else {
                    Err(SyncError::ConfigPackingError(format!(
                        "7-Zip command failed: {}",
                        String::utf8_lossy(&output.stderr)
                    )))
                }
            }
            Err(e) => Err(SyncError::ConfigPackingError(format!(
                "Failed to execute 7-Zip command: {}. Please install 7-Zip from https://www.7-zip.org/",
                e
            ))),
        }
    }

    fn extract_archive(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> SyncResult<ExtractionResult> {
        use std::time::Instant;

        let start_time = Instant::now();

        // 7-Zip解压命令
        let cmd = format!(
            "7z x \"{}\" -o\"{}\"",
            archive_file.display(),
            target_dir.display()
        );

        match std::process::Command::new("cmd")
            .args(&["/C", cmd])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let extraction_time = start_time.elapsed().as_millis();
                    let (file_count, extracted_size) = count_files_and_size(target_dir)?;

                    Ok(ExtractionResult {
                        file_count,
                        extracted_size,
                        extraction_time_ms: extraction_time,
                    })
                } else {
                    Err(SyncError::ConfigPackingError(format!(
                        "7-Zip extraction failed: {}",
                        String::utf8_lossy(&output.stderr)
                    )))
                }
            }
            Err(e) => Err(SyncError::ConfigPackingError(format!(
                "Failed to execute 7-Zip command: {}. Please install 7-Zip from https://www.7-zip.org/",
                e
            ))),
        }
    }

    fn name(&self) -> &'static str {
        "7Z"
    }

    fn file_extension(&self) -> &'static str {
        "7z"
    }
}

/// 计算目录大小
fn calculate_directory_size(dir_path: &Path) -> SyncResult<u64> {
    use std::fs;
    use walkdir::WalkDir;

    let mut total_size = 0u64;

    for entry in WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Ok(metadata) = entry.metadata() {
            total_size += metadata.len();
        }
    }

    Ok(total_size)
}

/// 统计文件数量和大小
fn count_files_and_size(dir_path: &Path) -> SyncResult<(usize, u64)> {
    use std::fs;
    use walkdir::WalkDir;

    let mut file_count = 0usize;
    let mut total_size = 0u64;

    for entry in WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        file_count += 1;
        if let Ok(metadata) = entry.metadata() {
            total_size += metadata.len();
        }
    }

    Ok((file_count, total_size))
}
