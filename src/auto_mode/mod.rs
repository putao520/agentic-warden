use std::time::Duration;

use crate::cli_type::CliType;

pub mod config;
pub mod executor;

pub const DEFAULT_EXECUTION_ORDER: [&str; 3] = ["codex", "gemini", "claude"];

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub cli_type: CliType,
    pub prompt: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

