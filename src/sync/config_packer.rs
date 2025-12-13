use super::error::{SyncError, SyncResult};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Write;
use std::path::Path;
use tar::Builder;
use tracing::{debug, info, warn};

/// File patterns to exclude from synchronization (blacklist)
const EXCLUDE_PATTERNS: &[&str] = &[
    // === Claude specific cache/session directories ===
    "file-history/",     // Claude file history cache
    "session-env/",      // Claude session environment cache
    "todos/",            // Claude TODO items (temporary)
    "debug/",            // Claude debug files
    "shell-snapshots/",  // Claude shell snapshots
    "statsig/",          // Claude analytics
    ".update.lock",      // Claude update lock file
    "history.jsonl",     // Claude conversation history
    ".credentials.json", // Claude credentials (sensitive)
    // === Codex specific directories ===
    "log/",          // Codex log files
    "sessions/",     // Codex session cache
    "history.jsonl", // Codex conversation history
    // === Gemini specific directories ===
    "tmp/",                  // Gemini temporary files
    "tmp/*/chats/",          // Gemini chat sessions in temp dirs
    "tmp/*/*session-*.json", // Gemini session JSON files
    "antigravity/",          // Gemini entire cache directory (all subdirs excluded)
    // === Common files and directories ===
    "node_modules/", // Node.js dependencies
    ".git/",         // Git repository
    ".gitignore",    // Git ignore file
    ".gitmodules",   // Git submodules
    // === Temporary and cache files ===
    "*.tmp",
    "*.temp",
    "*.cache",
    "*.pid",
    "*.lock",
    "*.swp",
    "*.swo",
    ".DS_Store",
    "Thumbs.db",
    // === Editor and IDE files ===
    ".vscode/",
    ".idea/",
    "*.sublime-*",
    // === Development dependencies ===
    "npm-debug.log*",
    "yarn-debug.log*",
    "yarn-error.log*",
    "package-lock.json", // NPM lock file
    "__pycache__/",
    "*.py[cod]",
    "pip-log.txt",
    "pip-delete-this-directory.txt",
    "target/",
    "Cargo.lock",
    // === Large data files (over 10MB) ===
    "*.mp4",
    "*.avi",
    "*.mov",
    "*.zip",
    "*.tar.gz",
    "*.tgz",
    "*.rar",
    "*.7z",
    // === AI model files and embeddings ===
    "*.bin",
    "*.onnx",
    "*.safetensors",
    "*.gguf",
    "*.h5",
    "*.pb",
    "*.pt",
    "*.pth",
    ".gllm/",            // gllm model cache (models are downloaded separately)
    ".fastembed_cache/", // Legacy - for backwards compatibility
    "embeddings/",
    // === OS-specific files ===
    ".Trashes/",
    ".Spotlight-V100/",
    ".DocumentRevisions-V100/",
    // === Sensitive files ===
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "*.crt",
    "*.ca-bundle",
    // === Generated and auto files ===
    "*~",
    "*.bak",
    "*.orig",
    "*.rej",
];

impl ConfigPacker {
    /// Check if a file should be excluded based on blacklist patterns
    fn should_exclude_file(file_path: &Path, relative_path: &str) -> bool {
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check each exclusion pattern
        for pattern in EXCLUDE_PATTERNS {
            if pattern.ends_with('/') {
                // Directory pattern
                if relative_path.starts_with(pattern)
                    || relative_path.contains(&format!("/{}", pattern))
                {
                    debug!("Excluding directory: {}", relative_path);
                    return true;
                }
            } else if pattern.contains('*') {
                // Wildcard pattern - simple glob matching
                if Self::matches_glob(file_name, pattern)
                    || Self::matches_glob(relative_path, pattern)
                {
                    debug!("Excluding file by pattern '{}': {}", pattern, relative_path);
                    return true;
                }
            } else {
                // Exact match
                if file_name == *pattern || relative_path == *pattern {
                    debug!(
                        "Excluding file by exact match '{}': {}",
                        pattern, relative_path
                    );
                    return true;
                }
            }
        }

        // Also check file size (exclude files larger than 10MB)
        if let Ok(metadata) = file_path.metadata() {
            if metadata.len() > 10 * 1024 * 1024 {
                debug!(
                    "Excluding large file ({}MB): {}",
                    metadata.len() / (1024 * 1024),
                    relative_path
                );
                return true;
            }
        }

        false
    }

    /// Simple glob pattern matching (supports * and ? wildcards)
    fn matches_glob(text: &str, pattern: &str) -> bool {
        // Convert glob pattern to regex pattern
        let regex_pattern = pattern
            .replace('.', r"\.")
            .replace('?', ".")
            .replace('*', ".*");

        // Simple starts_with/ends_with optimization for common patterns
        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
            return text.contains(middle);
        } else if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            return text.ends_with(suffix);
        } else if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return text.starts_with(prefix);
        }

        // For more complex patterns, use simple character-by-character matching
        Self::simple_glob_match(text, pattern)
    }

    /// Simple glob matching implementation
    fn simple_glob_match(text: &str, pattern: &str) -> bool {
        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        Self::glob_match_recursive(&text_chars, 0, &pattern_chars, 0)
    }

    fn glob_match_recursive(
        text: &[char],
        text_pos: usize,
        pattern: &[char],
        pattern_pos: usize,
    ) -> bool {
        // If we've reached the end of both text and pattern, it's a match
        if text_pos >= text.len() && pattern_pos >= pattern.len() {
            return true;
        }

        // If pattern has * but we're at the end, it can match empty
        if pattern_pos < pattern.len()
            && pattern[pattern_pos] == '*'
            && text_pos >= text.len()
            && pattern_pos + 1 == pattern.len()
        {
            return true;
        }

        // If we've reached the end of text but pattern has more non-* characters
        if text_pos >= text.len() && pattern_pos < pattern.len() {
            // Check if remaining pattern characters are all *
            for &c in &pattern[pattern_pos..] {
                if c != '*' {
                    return false;
                }
            }
            return true;
        }

        // If we've reached the end of pattern but text has more
        if pattern_pos >= pattern.len() && text_pos < text.len() {
            return false;
        }

        match pattern[pattern_pos] {
            '?' => {
                // ? matches any single character
                Self::glob_match_recursive(text, text_pos + 1, pattern, pattern_pos + 1)
            }
            '*' => {
                // * matches zero or more characters
                // Try to match the rest of pattern with current position, or advance one character in text
                Self::glob_match_recursive(text, text_pos, pattern, pattern_pos + 1)
                    || Self::glob_match_recursive(text, text_pos + 1, pattern, pattern_pos)
            }
            c => {
                // Literal character match
                text[text_pos] == c
                    && Self::glob_match_recursive(text, text_pos + 1, pattern, pattern_pos + 1)
            }
        }
    }
}

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

        // Pack entire .claude directory using blacklist approach
        match self.add_directory_to_tar(tar, &claude_dir, ".claude")? {
            Some((count, size)) => {
                file_count = count;
                total_size = size;
            }
            None => {
                debug!("No files found in .claude directory");
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

        // Pack entire .codex directory using blacklist approach
        match self.add_directory_to_tar(tar, &codex_dir, ".codex")? {
            Some((count, size)) => {
                file_count = count;
                total_size = size;
            }
            None => {
                debug!("No files found in .codex directory");
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

        // Pack entire .gemini directory using blacklist approach
        match self.add_directory_to_tar(tar, &gemini_dir, ".gemini")? {
            Some((count, size)) => {
                file_count = count;
                total_size = size;
            }
            None => {
                debug!("No files found in .gemini directory");
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
            .filter_entry(|e| {
                let file_name = e.file_name().to_string_lossy();
                // Skip hidden files (starting with .) except for specific config files
                if file_name.starts_with('.')
                    && ![
                        ".claude",
                        ".codex",
                        ".gemini",
                        ".aiw",
                        "auth.json",
                        "config.json",
                        "settings.json",
                    ]
                    .iter()
                    .any(|&allowed| file_name == allowed)
                {
                    return false;
                }
                true
            })
        {
            let entry = entry.map_err(|e| {
                SyncError::config_packing(format!("Failed to walk directory: {}", e))
            })?;

            let path = entry.path();
            let relative_path = path.strip_prefix(dir_path).unwrap();
            let tar_path = Path::new(tar_base_path).join(relative_path);
            let tar_path_str = tar_path.to_string_lossy();

            if path.is_file() {
                // Check if file should be excluded based on blacklist
                if !Self::should_exclude_file(path, &tar_path_str) {
                    if let Ok(size) = self.add_file_to_tar(tar, path, &tar_path_str) {
                        file_count += 1;
                        total_size += size;
                        debug!("Included file: {} ({} bytes)", tar_path_str, size);
                    } else {
                        debug!("Failed to add file to tar: {}", tar_path_str);
                    }
                }
            }
        }

        if file_count > 0 {
            debug!(
                "Added directory {} with {} files ({} bytes)",
                tar_base_path, file_count, total_size
            );
            Ok(Some((file_count, total_size)))
        } else {
            debug!("No files included from directory: {}", tar_base_path);
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
