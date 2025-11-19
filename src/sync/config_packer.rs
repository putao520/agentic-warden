use super::error::{SyncError, SyncResult};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Write;
use std::path::Path;
use tar::Builder;
use tracing::{debug, info, warn};

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

    /// Pack AI CLI configurations with selective file inclusion
    /// Only includes specific files as defined in SPEC
    pub fn pack_ai_configs<O: AsRef<Path>>(
        &self,
        config_name: &str,
        output_file: O,
    ) -> SyncResult<u64> {
        let output_path = output_file.as_ref();

        // Create parent directory for output file if it doesn't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SyncError::config_packing(format!("Failed to create output directory: {}", e))
            })?;
        }

        // Create tar.gz file
        let file = fs::File::create(output_path).map_err(|e| {
            SyncError::config_packing(format!("Failed to create output file: {}", e))
        })?;

        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar = Builder::new(encoder);

        let mut file_count = 0;
        // Pack Claude configurations
        if let Some((count, size)) = self.pack_claude_configs(&mut tar)? {
            file_count += count;
            info!(
                "Packed {} files from Claude configuration ({} bytes)",
                count, size
            );
        }

        // Pack Codex configurations
        if let Some((count, size)) = self.pack_codex_configs(&mut tar)? {
            file_count += count;
            info!(
                "Packed {} files from Codex configuration ({} bytes)",
                count, size
            );
        }

        // Pack Gemini configurations
        if let Some((count, size)) = self.pack_gemini_configs(&mut tar)? {
            file_count += count;
            info!(
                "Packed {} files from Gemini configuration ({} bytes)",
                count, size
            );
        }

        if file_count == 0 {
            warn!(
                "No configuration files found to pack for config '{}'",
                config_name
            );
            return Err(SyncError::config_packing(
                "No configuration files found".to_string(),
            ));
        }

        // Finish tar and get compressed file size
        let encoder = tar.into_inner().map_err(|e| {
            SyncError::config_packing(format!("Failed to finish tar creation: {}", e))
        })?;

        let mut file = encoder.finish().map_err(|e| {
            SyncError::config_packing(format!("Failed to finish compression: {}", e))
        })?;

        file.flush().map_err(|e| {
            SyncError::config_packing(format!("Failed to flush output file: {}", e))
        })?;

        // Get file size
        let metadata = fs::metadata(output_path).map_err(|e| {
            SyncError::config_packing(format!("Failed to get output file metadata: {}", e))
        })?;

        info!(
            "Successfully packed configuration '{}' with {} files ({} bytes compressed)",
            config_name,
            file_count,
            metadata.len()
        );

        Ok(metadata.len())
    }

    /// Pack Claude configuration files
    fn pack_claude_configs<W: Write>(
        &self,
        tar: &mut Builder<W>,
    ) -> SyncResult<Option<(usize, u64)>> {
        let claude_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::config_packing("Could not find home directory".to_string()))?
            .join(".claude");

        if !claude_dir.exists() {
            debug!("Claude directory does not exist: {}", claude_dir.display());
            return Ok(None);
        }

        let mut file_count = 0;
        let mut total_size = 0u64;

        // Core files to include
        let files_to_pack = [
            ("CLAUDE.md", "Main memory file"),
            ("settings.json", "Main configuration file"),
        ];

        for (file, description) in &files_to_pack {
            let file_path = claude_dir.join(file);
            if file_path.exists() {
                if let Ok(size) =
                    self.add_file_to_tar(tar, &file_path, &format!(".claude/{}", file))
                {
                    file_count += 1;
                    total_size += size;
                    debug!("Added {}: {} ({} bytes)", description, file, size);
                }
            }
        }

        // Pack agents directory if it exists
        let agents_dir = claude_dir.join("agents");
        if agents_dir.exists() && agents_dir.is_dir() {
            if let Some((count, size)) =
                self.add_directory_to_tar(tar, &agents_dir, ".claude/agents")?
            {
                file_count += count;
                total_size += size;
                debug!(
                    "Added agents directory with {} files ({} bytes)",
                    count, size
                );
            }
        }

        // Pack skills directory and SKILL.md files if it exists
        let skills_dir = claude_dir.join("skills");
        if skills_dir.exists() && skills_dir.is_dir() {
            // Recursively find all SKILL.md files
            if let Ok((count, size)) = self.pack_skills_directory(tar, &skills_dir) {
                file_count += count;
                total_size += size;
                debug!(
                    "Added skills directory with {} SKILL files ({} bytes)",
                    count, size
                );
            }
        }

        // Pack additional directories specified in SPEC REQ-003 (hooks, scripts, commands)
        let additional_dirs = [
            ("hooks", "Claude Code hook handlers and configuration"),
            ("scripts", "Execution scripts and workflow files"),
            ("commands", "Custom slash command definitions"),
        ];

        for (dir_name, description) in &additional_dirs {
            let dir_path = claude_dir.join(dir_name);
            if dir_path.exists() && dir_path.is_dir() {
                if let Some((count, size)) =
                    self.add_directory_to_tar(tar, &dir_path, &format!(".claude/{}", dir_name))?
                {
                    file_count += count;
                    total_size += size;
                    debug!(
                        "Added {} directory with {} files ({} bytes)",
                        dir_name, count, size
                    );
                    info!("Packed {}: {} files, {} bytes", description, count, size);
                }
            }
        }

        // Pack .mcp.json file if it exists (MCP server configuration for Claude)
        let mcp_config_path = claude_dir.join(".mcp.json");
        if mcp_config_path.exists() && mcp_config_path.is_file() {
            if let Ok(size) = self.add_file_to_tar(
                tar,
                &mcp_config_path,
                ".claude/.mcp.json",
            ) {
                file_count += 1;
                total_size += size;
                info!("Packed MCP configuration: .mcp.json ({} bytes)", size);
                debug!("Added .mcp.json: {} bytes", size);
            }
        }

        if file_count > 0 {
            Ok(Some((file_count, total_size)))
        } else {
            Ok(None)
        }
    }

    /// Pack Codex configuration files
    fn pack_codex_configs<W: Write>(
        &self,
        tar: &mut Builder<W>,
    ) -> SyncResult<Option<(usize, u64)>> {
        let codex_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::config_packing("Could not find home directory".to_string()))?
            .join(".codex");

        if !codex_dir.exists() {
            debug!("Codex directory does not exist: {}", codex_dir.display());
            return Ok(None);
        }

        let mut file_count = 0;
        let mut total_size = 0u64;

        // List of files to include
        let files_to_pack = [
            ("auth.json", "Authentication configuration"),
            ("config.toml", "Main configuration file"),
            ("version.json", "Version information"),
            ("agents.md", "Main memory file"),
            ("history.jsonl", "Command history"),
        ];

        for (file, description) in &files_to_pack {
            let file_path = codex_dir.join(file);
            if file_path.exists() {
                if let Ok(size) = self.add_file_to_tar(tar, &file_path, &format!(".codex/{}", file))
                {
                    file_count += 1;
                    total_size += size;
                    debug!("Added {}: {} ({} bytes)", description, file, size);
                }
            }
        }

        if file_count > 0 {
            Ok(Some((file_count, total_size)))
        } else {
            Ok(None)
        }
    }

    /// Pack Gemini configuration files
    fn pack_gemini_configs<W: Write>(
        &self,
        tar: &mut Builder<W>,
    ) -> SyncResult<Option<(usize, u64)>> {
        let gemini_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::config_packing("Could not find home directory".to_string()))?
            .join(".gemini");

        if !gemini_dir.exists() {
            debug!("Gemini directory does not exist: {}", gemini_dir.display());
            return Ok(None);
        }

        let mut file_count = 0;
        let mut total_size = 0u64;

        // List of files to include
        let files_to_pack = [
            ("google_accounts.json", "Google account configuration"),
            ("oauth_creds.json", "OAuth credentials"),
            ("settings.json", "Main configuration file"),
            ("gemini.md", "Main memory file"),
        ];

        for (file, description) in &files_to_pack {
            let file_path = gemini_dir.join(file);
            if file_path.exists() {
                if let Ok(size) =
                    self.add_file_to_tar(tar, &file_path, &format!(".gemini/{}", file))
                {
                    file_count += 1;
                    total_size += size;
                    debug!("Added {}: {} ({} bytes)", description, file, size);
                }
            }
        }

        // Pack tmp directory if it exists
        let tmp_dir = gemini_dir.join("tmp");
        if tmp_dir.exists() && tmp_dir.is_dir() {
            if let Some((count, size)) = self.add_directory_to_tar(tar, &tmp_dir, ".gemini/tmp")? {
                file_count += count;
                total_size += size;
                debug!("Added tmp directory with {} files ({} bytes)", count, size);
            }
        }

        if file_count > 0 {
            Ok(Some((file_count, total_size)))
        } else {
            Ok(None)
        }
    }

    /// Pack skills directory, only including SKILL.md files
    fn pack_skills_directory<W: Write>(
        &self,
        tar: &mut Builder<W>,
        skills_dir: &Path,
    ) -> SyncResult<(usize, u64)> {
        let mut file_count = 0;
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(skills_dir)
            .into_iter()
            .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        {
            let entry = entry.map_err(|e| {
                SyncError::config_packing(format!("Failed to walk skills directory: {}", e))
            })?;

            if entry.file_type().is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.eq_ignore_ascii_case("skill.md") {
                        let path_in_tar = Path::new(".claude/skills")
                            .join(entry.path().strip_prefix(skills_dir).unwrap());
                        if let Ok(size) =
                            self.add_file_to_tar(tar, entry.path(), &path_in_tar.to_string_lossy())
                        {
                            file_count += 1;
                            total_size += size;
                        }
                    }
                }
            }
        }

        Ok((file_count, total_size))
    }

    /// Add a single file to the tar archive
    fn add_file_to_tar<W: Write>(
        &self,
        tar: &mut Builder<W>,
        file_path: &Path,
        tar_path: &str,
    ) -> SyncResult<u64> {
        let mut file = fs::File::open(file_path).map_err(|e| {
            SyncError::config_packing(format!(
                "Failed to open file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        let size = file
            .metadata()
            .map_err(|e| SyncError::config_packing(format!("Failed to get file metadata: {}", e)))?
            .len();

        tar.append_file(tar_path, &mut file)
            .map_err(|e| SyncError::config_packing(format!("Failed to add file to tar: {}", e)))?;

        Ok(size)
    }

    /// Add a directory recursively to the tar archive
    fn add_directory_to_tar<W: Write>(
        &self,
        tar: &mut Builder<W>,
        dir_path: &Path,
        tar_base_path: &str,
    ) -> SyncResult<Option<(usize, u64)>> {
        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(None);
        }

        let mut file_count = 0;
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(dir_path)
            .into_iter()
            .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        {
            let entry = entry.map_err(|e| {
                SyncError::config_packing(format!("Failed to walk directory: {}", e))
            })?;

            let path = entry.path();
            let relative_path = path.strip_prefix(dir_path).unwrap();
            let tar_path = Path::new(tar_base_path).join(relative_path);

            if path.is_file() {
                if let Ok(size) = self.add_file_to_tar(tar, path, &tar_path.to_string_lossy()) {
                    file_count += 1;
                    total_size += size;
                }
            }
        }

        if file_count > 0 {
            Ok(Some((file_count, total_size)))
        } else {
            Ok(None)
        }
    }

    /// Unpack archive to the specified directory
    pub fn unpack_archive<P: AsRef<Path>, O: AsRef<Path>>(
        &self,
        archive_file: P,
        output_dir: O,
    ) -> SyncResult<()> {
        let archive_path = archive_file.as_ref();
        let output_path = output_dir.as_ref();

        if !archive_path.exists() {
            return Err(SyncError::config_packing(format!(
                "Archive file not found: {}",
                archive_path.to_string_lossy()
            )));
        }

        // Create output directory if it doesn't exist
        fs::create_dir_all(output_path).map_err(|e| {
            SyncError::config_packing(format!("Failed to create output directory: {}", e))
        })?;

        // Open and extract archive
        let file = fs::File::open(archive_path).map_err(|e| {
            SyncError::config_packing(format!("Failed to open archive file: {}", e))
        })?;

        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        archive
            .unpack(output_path)
            .map_err(|e| SyncError::config_packing(format!("Failed to unpack archive: {}", e)))?;

        Ok(())
    }

    /// Pack an entire directory (for backward compatibility with old sync system)
    pub fn pack_directory<P: AsRef<Path>, O: AsRef<Path>>(
        &self,
        directory_path: P,
        output_file: O,
    ) -> SyncResult<u64> {
        let dir_path = directory_path.as_ref();
        let output_path = output_file.as_ref();

        // Create parent directory for output file if it doesn't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SyncError::config_packing(format!("Failed to create output directory: {}", e))
            })?;
        }

        // Create tar.gz file
        let file = fs::File::create(output_path).map_err(|e| {
            SyncError::config_packing(format!("Failed to create output file: {}", e))
        })?;

        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar = Builder::new(encoder);

        // Add the entire directory to the archive
        let dir_name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| SyncError::config_packing("Invalid directory name".to_string()))?;

        if self
            .add_directory_to_tar(&mut tar, dir_path, dir_name)?
            .is_some()
        {
            tar.finish().map_err(|e| {
                SyncError::config_packing(format!("Failed to finish tar creation: {}", e))
            })?;

            let encoder = tar
                .into_inner()
                .map_err(|e| SyncError::config_packing(format!("Failed to get encoder: {}", e)))?;

            let mut file = encoder.finish().map_err(|e| {
                SyncError::config_packing(format!("Failed to finish compression: {}", e))
            })?;

            file.flush().map_err(|e| {
                SyncError::config_packing(format!("Failed to flush output file: {}", e))
            })?;

            let metadata = fs::metadata(output_path).map_err(|e| {
                SyncError::config_packing(format!("Failed to get output file metadata: {}", e))
            })?;

            Ok(metadata.len())
        } else {
            Err(SyncError::config_packing(
                "No files found to pack".to_string(),
            ))
        }
    }

    /// Get information about an archive
    pub fn get_archive_info<P: AsRef<Path>>(&self, archive_file: P) -> SyncResult<ArchiveInfo> {
        let archive_path = archive_file.as_ref();

        if !archive_path.exists() {
            return Err(SyncError::config_packing(format!(
                "Archive file not found: {}",
                archive_path.to_string_lossy()
            )));
        }

        let metadata = fs::metadata(archive_path).map_err(|e| {
            SyncError::config_packing(format!("Failed to get archive metadata: {}", e))
        })?;

        let file = fs::File::open(archive_path).map_err(|e| {
            SyncError::config_packing(format!("Failed to open archive file: {}", e))
        })?;

        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        let mut file_count = 0usize;
        let mut total_uncompressed_size = 0u64;

        for entry in archive.entries().map_err(|e| {
            SyncError::config_packing(format!("Failed to read archive entries: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                SyncError::config_packing(format!("Failed to read archive entry: {}", e))
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
        let source_root = source_dir.path().join("payload");
        fs::create_dir_all(&source_root).unwrap();
        let output_dir = TempDir::new().unwrap();
        let archive_file = output_dir.path().join("test.tar.gz");

        // Create some test files
        fs::write(source_root.join("file1.txt"), "Hello, World!").unwrap();
        fs::write(source_root.join("file2.txt"), "Another file").unwrap();

        let packer = ConfigPacker::new();

        // Pack directory
        let compressed_size = packer.pack_directory(&source_root, &archive_file).unwrap();

        assert!(compressed_size > 0);

        // Unpack archive
        packer
            .unpack_archive(&archive_file, output_dir.path())
            .unwrap();

        // Verify files were unpacked
        let unpacked_root = output_dir.path().join("payload");
        assert!(unpacked_root.join("file1.txt").exists());
        assert!(unpacked_root.join("file2.txt").exists());
    }
}
