use std::time::Duration;

pub const CLAUDE_BIN: &str = "claude";
pub const CODEX_BIN: &str = "codex";
pub const GEMINI_BIN: &str = "gemini";
pub const SHARED_NAMESPACE: &str = "agentic-task";
pub const SHARED_MEMORY_SIZE: usize = 80 * 1024 * 1024;

pub const WAIT_INTERVAL_ENV: &str = "AGENTIC_WARDEN_WAIT_INTERVAL_SEC";
pub const LEGACY_WAIT_INTERVAL_ENV: &str = "CODEX_WORKER_WAIT_INTERVAL_SEC";
pub const DEBUG_ENV: &str = "AGENTIC_WARDEN_DEBUG";
pub const LEGACY_DEBUG_ENV: &str = "CODEX_WORKER_DEBUG";

pub const MAX_RECORD_AGE: Duration = Duration::from_secs(12 * 60 * 60);
pub const WAIT_INTERVAL_DEFAULT: Duration = Duration::from_secs(30);
pub const MAX_WAIT_DURATION: Duration = Duration::from_secs(24 * 60 * 60);
