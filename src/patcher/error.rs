//! 补丁错误类型定义
//!
//! 用于运行时内存补丁的错误处理。

use crate::error::AgenticWardenError;
use std::io;
use thiserror::Error;

/// 补丁操作结果类型
pub type PatchResult<T> = Result<T, PatchError>;

/// 补丁操作错误
#[derive(Error, Debug)]
pub enum PatchError {
    #[error("Process not found: PID {pid}")]
    ProcessNotFound { pid: u32 },

    #[error("Patch pattern not found: {pattern}")]
    PatternNotFound { pattern: String, hint: Option<String> },

    #[error("Failed to read process memory: {reason}")]
    ReadFailed { reason: String },

    #[error("Failed to write process memory: {reason}")]
    WriteFailed { reason: String },

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Process has exited: PID {pid}")]
    ProcessExited { pid: u32 },

    #[error("Memory region is not writable: address {address:#x}")]
    NotWritable { address: usize },

    #[error("Unsupported architecture: {arch}")]
    UnsupportedArch { arch: String },

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<PatchError> for AgenticWardenError {
    fn from(err: PatchError) -> Self {
        AgenticWardenError::Process {
            message: err.to_string(),
            command: "patch".to_string(),
            source: None,
        }
    }
}

impl PatchError {
    pub fn process_not_found(pid: u32) -> Self {
        PatchError::ProcessNotFound { pid }
    }

    pub fn pattern_not_found(pattern: impl Into<String>) -> Self {
        PatchError::PatternNotFound {
            pattern: pattern.into(),
            hint: None,
        }
    }

    pub fn read_failed(reason: impl Into<String>) -> Self {
        PatchError::ReadFailed {
            reason: reason.into(),
        }
    }

    pub fn write_failed(reason: impl Into<String>) -> Self {
        PatchError::WriteFailed {
            reason: reason.into(),
        }
    }
}
