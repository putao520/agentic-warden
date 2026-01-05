use std::time::Duration;

use serde::Deserialize;

use crate::cli_type::CliType;

pub mod config;
pub mod executor;
pub mod judge;

pub const DEFAULT_EXECUTION_ORDER: [&str; 3] = ["codex", "gemini", "claude"];
pub const LLM_TIMEOUT: Duration = Duration::from_secs(5);
pub const OLLAMA_MODEL: &str = "qwen3:1.7b";  // 使用已有模型
pub const OLLAMA_ENDPOINT: &str = "http://localhost:11434";

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub cli_type: CliType,
    pub prompt: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Judgment {
    pub success: bool,
    pub should_retry: bool,
    pub reason: String,
}
