use std::time::Duration;

pub const CLAUDE_BIN: &str = "claude";
pub const CODEX_BIN: &str = "codex";
pub const GEMINI_BIN: &str = "gemini";
pub const SHARED_NAMESPACE: &str = "agentic-task";
// Shared memory size for task registry (16MB)
// Supports approximately 4000+ TaskRecord entries with overhead
// Compatible with both 32-bit and 64-bit Windows systems
pub const SHARED_MEMORY_SIZE: usize = 16 * 1024 * 1024;

pub const WAIT_INTERVAL_ENV: &str = "AGENTIC_WARDEN_WAIT_INTERVAL_SEC";
pub const LEGACY_WAIT_INTERVAL_ENV: &str = "CODEX_WORKER_WAIT_INTERVAL_SEC";
pub const DEBUG_ENV: &str = "AGENTIC_WARDEN_DEBUG";
pub const LEGACY_DEBUG_ENV: &str = "CODEX_WORKER_DEBUG";

// Common constants used across modules
pub const AUTH_DIRECTORY: &str = ".aiw";
pub const AUTH_FILE_NAME: &str = "auth.json";

pub const MAX_RECORD_AGE: Duration = Duration::from_secs(12 * 60 * 60);
pub const WAIT_INTERVAL_DEFAULT: Duration = Duration::from_secs(30);
pub const MAX_WAIT_DURATION: Duration = Duration::from_secs(24 * 60 * 60);

/// Patch 配置：max-token patch 的可配置参数
///
/// 通过 `patch set-max-tokens` 命令持久化用户选择，supervisor 启动
/// Claude CLI 时自动读取并应用。
///
/// - `max_context_tokens`: 默认上下文窗口上限（6 位数，100000~999999，默认 500000）
/// - `auto_compact_window`: autoCompact 阈值（6 位数，默认 500000）
///
/// 等长替换铁律：值必须是 6 位十进制数，否则会改变二进制偏移。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PatchConfig {
    /// 默认上下文窗口上限（6 位数，100000~999999）
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: u32,
    /// autoCompact 阈值（6 位数，100000~999999）
    #[serde(default = "default_auto_compact_window")]
    pub auto_compact_window: u32,
}

fn default_max_context_tokens() -> u32 {
    500000
}

fn default_auto_compact_window() -> u32 {
    500000
}

impl Default for PatchConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: default_max_context_tokens(),
            auto_compact_window: default_auto_compact_window(),
        }
    }
}

impl PatchConfig {
    /// 配置文件路径：`~/.aiw/patch.json`
    fn config_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(AUTH_DIRECTORY).join("patch.json")
    }

    /// 从磁盘加载配置；不存在则返回默认值
    pub fn load() -> std::result::Result<Self, std::io::Error> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
        let cfg: Self = serde_json::from_str(&content).unwrap_or_else(|_| Self::default());
        Ok(cfg)
    }

    /// 保存配置到磁盘（自动校验 6 位数）
    pub fn save(&self) -> std::result::Result<(), std::io::Error> {
        crate::patcher::claude::versions::validate_max_context_tokens(self.max_context_tokens)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        crate::patcher::claude::versions::validate_max_context_tokens(self.auto_compact_window)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_config_defaults() {
        let cfg = PatchConfig::default();
        assert_eq!(cfg.max_context_tokens, 500000);
        assert_eq!(cfg.auto_compact_window, 500000);
    }

    #[test]
    fn test_patch_config_serde_roundtrip() {
        let cfg = PatchConfig {
            max_context_tokens: 300000,
            auto_compact_window: 300000,
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: PatchConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.max_context_tokens, 300000);
        assert_eq!(back.auto_compact_window, 300000);
    }

    #[test]
    fn test_patch_config_serde_missing_fields_use_defaults() {
        // 旧配置文件缺少字段时用默认值
        let s = "{}";
        let back: PatchConfig = serde_json::from_str(s).unwrap();
        assert_eq!(back.max_context_tokens, 500000);
        assert_eq!(back.auto_compact_window, 500000);
    }

    #[test]
    fn test_patch_config_save_rejects_invalid_values() {
        let cfg = PatchConfig {
            max_context_tokens: 99999,
            auto_compact_window: 500000,
        };
        assert!(cfg.save().is_err());
    }
}
