//! Unified error handling for the agentic-warden project
//!
//! This module provides a comprehensive error handling system with
//! proper error classification, context, and recovery mechanisms.

#![allow(dead_code)] // 错误处理模块，部分辅助函数是公共API

use anyhow::Error as AnyhowError;
use std::collections::HashMap;
use std::fmt;
use std::io;
use thiserror::Error;

/// Main error type for the application
#[derive(Error, Debug)]
pub enum AgenticWardenError {
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Provider service errors
    #[error("Provider error: {provider} - {message}")]
    Provider {
        provider: String,
        message: String,
        error_code: Option<u32>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Task execution errors
    #[error("Task error (ID: {task_id}): {message}")]
    Task {
        task_id: u64,
        message: String,
        exit_code: Option<i32>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Synchronization errors
    #[error("Sync error ({operation}): {message}")]
    Sync {
        message: String,
        operation: SyncOperation,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Authentication errors
    #[error("Authentication error: {message}")]
    Auth {
        message: String,
        provider: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Network errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        url: Option<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Filesystem errors
    #[error("Filesystem error: {message} (path: {path})")]
    Filesystem {
        message: String,
        path: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// TUI errors
    #[error("TUI error: {message}")]
    Tui {
        message: String,
        component: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Process management errors
    #[error("Process error: {message}")]
    Process {
        message: String,
        command: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
        value: Option<String>,
    },

    /// Resource errors (memory, file handles, etc.)
    #[error("Resource error: {message}")]
    Resource {
        message: String,
        resource_type: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Dependency injection errors
    #[error("Dependency injection error: {message}")]
    Dependency {
        message: String,
        service: Option<String>,
    },

    /// Timeout errors
    #[error("Timeout error: {message} (timeout: {timeout_ms}ms)")]
    Timeout {
        message: String,
        timeout_ms: u64,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Concurrency errors
    #[error("Concurrency error: {message}")]
    Concurrency {
        message: String,
        operation: Option<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Unknown errors
    #[error("Unknown error: {message}")]
    Unknown {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {path}")]
    FileNotFound { path: String },
    #[error("Invalid JSON format in config file")]
    InvalidFormat,
    #[error("cli_execution_order must be an array")]
    InvalidType,
    #[error("cli_execution_order must contain exactly 3 AI CLIs")]
    InvalidLength { expected: usize, actual: usize },
    #[error("All elements in cli_execution_order must be strings")]
    InvalidElementType,
    #[error("Invalid CLI type: {value}. Allowed values: codex, claude, gemini")]
    InvalidCliType { value: String },
    #[error("cli_execution_order contains duplicate CLI types")]
    DuplicateCliType,
    #[error("cli_execution_order must contain all 3 CLIs: codex, claude, gemini")]
    IncompleteSet,
    #[error("Config file error: {message}")]
    Io { message: String },
}

#[derive(Error, Debug)]
pub enum JudgeError {
    #[error("Ollama service is not running. Start Ollama and try again.")]
    Unavailable,
    #[error("LLM request timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },
    #[error("Invalid LLM response: {message}")]
    InvalidResponse { message: String },
    #[error("Ollama API error: {message}")]
    Api { message: String },
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("LLM judgment error: {0}")]
    Judge(#[from] JudgeError),
    #[error("Execution stopped: {reason}")]
    Halt { reason: String },
    #[error("All AI CLIs failed. Last error: {message}")]
    AllFailed { message: String },
    #[error("Prompt cannot be empty")]
    EmptyPrompt,
    #[error("Execution failed: {message}")]
    ExecutionFailed { message: String },
}

/// High-level sync operation categories used for user messaging and recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOperation {
    DirectoryHashing,
    ConfigPacking,
    ConfigLoading,
    ConfigSaving,
    ArchiveExtraction,
    Compression,
    GoogleDriveAuth,
    GoogleDriveRequest,
    NetworkProbe,
    Upload,
    Download,
    OAuthCallback,
    StateVerification,
    Discovery,
    Unknown,
}

impl fmt::Display for SyncOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl SyncOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncOperation::DirectoryHashing => "directory_hashing",
            SyncOperation::ConfigPacking => "config_packing",
            SyncOperation::ConfigLoading => "config_loading",
            SyncOperation::ConfigSaving => "config_saving",
            SyncOperation::ArchiveExtraction => "archive_extraction",
            SyncOperation::Compression => "compression",
            SyncOperation::GoogleDriveAuth => "google_drive_auth",
            SyncOperation::GoogleDriveRequest => "google_drive_request",
            SyncOperation::NetworkProbe => "network_probe",
            SyncOperation::Upload => "upload",
            SyncOperation::Download => "download",
            SyncOperation::OAuthCallback => "oauth_callback",
            SyncOperation::StateVerification => "state_verification",
            SyncOperation::Discovery => "discovery",
            SyncOperation::Unknown => "unknown",
        }
    }
}

impl AgenticWardenError {
    /// Get error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            AgenticWardenError::Config { .. } => ErrorCategory::Config,
            AgenticWardenError::Provider { .. } => ErrorCategory::Provider,
            AgenticWardenError::Task { .. } => ErrorCategory::Task,
            AgenticWardenError::Sync { .. } => ErrorCategory::Sync,
            AgenticWardenError::Auth { .. } => ErrorCategory::Auth,
            AgenticWardenError::Network { .. } => ErrorCategory::Network,
            AgenticWardenError::Filesystem { .. } => ErrorCategory::Filesystem,
            AgenticWardenError::Tui { .. } => ErrorCategory::Tui,
            AgenticWardenError::Process { .. } => ErrorCategory::Process,
            AgenticWardenError::Validation { .. } => ErrorCategory::Validation,
            AgenticWardenError::Resource { .. } => ErrorCategory::Resource,
            AgenticWardenError::Dependency { .. } => ErrorCategory::Dependency,
            AgenticWardenError::Timeout { .. } => ErrorCategory::Timeout,
            AgenticWardenError::Concurrency { .. } => ErrorCategory::Concurrency,
            AgenticWardenError::Unknown { .. } => ErrorCategory::Unknown,
        }
    }

    /// Get error severity
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AgenticWardenError::Config { .. } => ErrorSeverity::High,
            AgenticWardenError::Provider { .. } => ErrorSeverity::Medium,
            AgenticWardenError::Task { .. } => ErrorSeverity::Medium,
            AgenticWardenError::Sync { .. } => ErrorSeverity::Medium,
            AgenticWardenError::Auth { .. } => ErrorSeverity::High,
            AgenticWardenError::Network { .. } => ErrorSeverity::Medium,
            AgenticWardenError::Filesystem { .. } => ErrorSeverity::Medium,
            AgenticWardenError::Tui { .. } => ErrorSeverity::Low,
            AgenticWardenError::Process { .. } => ErrorSeverity::Medium,
            AgenticWardenError::Validation { .. } => ErrorSeverity::Low,
            AgenticWardenError::Resource { .. } => ErrorSeverity::High,
            AgenticWardenError::Dependency { .. } => ErrorSeverity::High,
            AgenticWardenError::Timeout { .. } => ErrorSeverity::Medium,
            AgenticWardenError::Concurrency { .. } => ErrorSeverity::High,
            AgenticWardenError::Unknown { .. } => ErrorSeverity::Medium,
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            AgenticWardenError::Config { .. } => false,
            AgenticWardenError::Provider { .. } => true,
            AgenticWardenError::Task { .. } => true,
            AgenticWardenError::Sync { .. } => true,
            AgenticWardenError::Auth { .. } => true,
            AgenticWardenError::Network { .. } => true,
            AgenticWardenError::Filesystem { .. } => false,
            AgenticWardenError::Tui { .. } => true,
            AgenticWardenError::Process { .. } => true,
            AgenticWardenError::Validation { .. } => true,
            AgenticWardenError::Resource { .. } => false,
            AgenticWardenError::Dependency { .. } => false,
            AgenticWardenError::Timeout { .. } => true,
            AgenticWardenError::Concurrency { .. } => false,
            AgenticWardenError::Unknown { .. } => true,
        }
    }

    /// Returns the sync operation associated with the error, if any.
    pub fn sync_operation(&self) -> Option<SyncOperation> {
        if let AgenticWardenError::Sync { operation, .. } = self {
            Some(*operation)
        } else {
            None
        }
    }

    /// Get user-friendly message
    pub fn user_message(&self) -> String {
        match self {
            AgenticWardenError::Config { message, .. } => {
                format!("Configuration problem: {}", message)
            }
            AgenticWardenError::Provider {
                provider, message, ..
            } => {
                format!("Provider '{}' issue: {}", provider, message)
            }
            AgenticWardenError::Task {
                task_id, message, ..
            } => {
                format!("Task #{} failed: {}", task_id, message)
            }
            AgenticWardenError::Sync {
                message, operation, ..
            } => match operation {
                SyncOperation::DirectoryHashing => {
                    format!("Unable to scan configuration directories: {}", message)
                }
                SyncOperation::ConfigPacking | SyncOperation::Compression => {
                    format!("Failed to prepare configuration archive: {}", message)
                }
                SyncOperation::ConfigLoading | SyncOperation::ConfigSaving => {
                    format!("Configuration file problem: {}", message)
                }
                SyncOperation::ArchiveExtraction => {
                    format!("Unable to unpack configuration archive: {}", message)
                }
                SyncOperation::GoogleDriveAuth | SyncOperation::OAuthCallback => {
                    format!("Google Drive authentication required: {}", message)
                }
                SyncOperation::GoogleDriveRequest | SyncOperation::NetworkProbe => {
                    format!("Google Drive request failed: {}", message)
                }
                SyncOperation::Upload => format!("Upload failed: {}", message),
                SyncOperation::Download => format!("Download failed: {}", message),
                SyncOperation::StateVerification => {
                    format!("Could not verify remote state: {}", message)
                }
                SyncOperation::Discovery => format!("Discovery issue: {}", message),
                SyncOperation::Unknown => format!("Sync problem: {}", message),
            },
            AgenticWardenError::Auth {
                message, provider, ..
            } => {
                format!("Authentication with {} failed: {}", provider, message)
            }
            AgenticWardenError::Network { message, .. } => {
                format!("Network issue: {}", message)
            }
            AgenticWardenError::Filesystem { message, .. } => {
                format!("File system problem: {}", message)
            }
            AgenticWardenError::Tui { message, .. } => {
                format!("Interface issue: {}", message)
            }
            AgenticWardenError::Process { message, .. } => {
                format!("Process problem: {}", message)
            }
            AgenticWardenError::Validation { message, .. } => {
                format!("Input validation failed: {}", message)
            }
            AgenticWardenError::Resource { message, .. } => {
                format!("Resource issue: {}", message)
            }
            AgenticWardenError::Dependency { message, .. } => {
                format!("Service setup problem: {}", message)
            }
            AgenticWardenError::Timeout { message, .. } => {
                format!("Operation timed out: {}", message)
            }
            AgenticWardenError::Concurrency { message, .. } => {
                format!("Concurrency issue: {}", message)
            }
            AgenticWardenError::Unknown { message, .. } => {
                format!("Unexpected error: {}", message)
            }
        }
    }

    /// Get technical details for logging
    pub fn technical_details(&self) -> String {
        match self {
            AgenticWardenError::Config { source, .. } => {
                if let Some(src) = source {
                    format!("Config error - Source: {}", src)
                } else {
                    "Config error - No source".to_string()
                }
            }
            AgenticWardenError::Provider {
                provider,
                error_code,
                source,
                ..
            } => {
                let mut details = format!("Provider error - Provider: {}", provider);
                if let Some(code) = error_code {
                    details.push_str(&format!(", Code: {}", code));
                }
                if let Some(src) = source {
                    details.push_str(&format!(", Source: {}", src));
                }
                details
            }
            AgenticWardenError::Task {
                task_id,
                exit_code,
                source,
                ..
            } => {
                let mut details = format!("Task error - Task ID: {}", task_id);
                if let Some(code) = exit_code {
                    details.push_str(&format!(", Exit code: {}", code));
                }
                if let Some(src) = source {
                    details.push_str(&format!(", Source: {}", src));
                }
                details
            }
            _ => format!("Error details: {}", self),
        }
    }
}

impl From<AnyhowError> for AgenticWardenError {
    fn from(err: AnyhowError) -> Self {
        let message = err.to_string();
        AgenticWardenError::Unknown {
            message,
            source: None,
        }
    }
}

impl From<io::Error> for AgenticWardenError {
    fn from(err: io::Error) -> Self {
        AgenticWardenError::Filesystem {
            message: format!("I/O error: {err}"),
            path: "<io>".to_string(),
            source: Some(Box::new(err)),
        }
    }
}

/// Error categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Config,
    Provider,
    Task,
    Sync,
    Auth,
    Network,
    Filesystem,
    Tui,
    Process,
    Validation,
    Resource,
    Dependency,
    Timeout,
    Concurrency,
    Unknown,
}

impl ErrorCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            ErrorCategory::Config => "Configuration",
            ErrorCategory::Provider => "Provider",
            ErrorCategory::Task => "Task",
            ErrorCategory::Sync => "Synchronization",
            ErrorCategory::Auth => "Authentication",
            ErrorCategory::Network => "Network",
            ErrorCategory::Filesystem => "Filesystem",
            ErrorCategory::Tui => "Interface",
            ErrorCategory::Process => "Process",
            ErrorCategory::Validation => "Validation",
            ErrorCategory::Resource => "Resource",
            ErrorCategory::Dependency => "Dependency",
            ErrorCategory::Timeout => "Timeout",
            ErrorCategory::Concurrency => "Concurrency",
            ErrorCategory::Unknown => "Unknown",
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Result type alias for convenience
pub type AgenticResult<T> = Result<T, AgenticWardenError>;

/// Registry-specific errors
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("shared task map init failed: {0}")]
    Shared(String),
    #[error("shared hashmap operation failed: {0}")]
    Map(String),
    #[error("registry mutex poisoned")]
    Poison,
    #[error("record serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("task not found: {0}")]
    TaskNotFound(u32),
    #[error("process tree error: {0}")]
    ProcessTree(String),
}

impl From<shared_hashmap::Error> for RegistryError {
    fn from(value: shared_hashmap::Error) -> Self {
        RegistryError::Map(value.to_string())
    }
}

impl From<crate::core::process_tree::ProcessTreeError> for RegistryError {
    fn from(value: crate::core::process_tree::ProcessTreeError) -> Self {
        RegistryError::ProcessTree(value.to_string())
    }
}

impl From<AgenticWardenError> for RegistryError {
    fn from(value: AgenticWardenError) -> Self {
        RegistryError::Map(format!("Agentic error: {}", value))
    }
}

impl From<crate::core::shared_map::SharedMapError> for RegistryError {
    fn from(value: crate::core::shared_map::SharedMapError) -> Self {
        RegistryError::Shared(value.to_string())
    }
}

/// Error context builder
pub struct ErrorContext {
    category: Option<ErrorCategory>,
    severity: Option<ErrorSeverity>,
    component: Option<String>,
    operation: Option<String>,
    user_context: HashMap<String, String>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            category: None,
            severity: None,
            component: None,
            operation: None,
            user_context: HashMap::new(),
        }
    }

    pub fn category(mut self, category: ErrorCategory) -> Self {
        self.category = Some(category);
        self
    }

    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn component(mut self, component: impl Into<String>) -> Self {
        self.component = Some(component.into());
        self
    }

    pub fn operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    pub fn context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.user_context.insert(key.into(), value.into());
        self
    }

    pub fn wrap_error<T>(
        self,
        result: Result<T, impl Into<AgenticWardenError>>,
    ) -> AgenticResult<T> {
        result.map_err(|e| {
            // Apply context to the error if possible
            // This would require extending the error types to support context
            // For now, we just return the error as-is

            e.into()
        })
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for creating common errors
pub mod errors {
    use super::*;

    pub fn config_error(message: impl Into<String>) -> AgenticWardenError {
        AgenticWardenError::Config {
            message: message.into(),
            source: None,
        }
    }

    pub fn provider_error(
        provider: impl Into<String>,
        message: impl Into<String>,
    ) -> AgenticWardenError {
        AgenticWardenError::Provider {
            provider: provider.into(),
            message: message.into(),
            error_code: None,
            source: None,
        }
    }

    pub fn task_error(
        task_id: u64,
        message: impl Into<String>,
        exit_code: Option<i32>,
    ) -> AgenticWardenError {
        AgenticWardenError::Task {
            task_id,
            message: message.into(),
            exit_code,
            source: None,
        }
    }

    pub fn auth_error(
        message: impl Into<String>,
        provider: impl Into<String>,
    ) -> AgenticWardenError {
        AgenticWardenError::Auth {
            message: message.into(),
            provider: provider.into(),
            source: None,
        }
    }

    pub fn network_error(message: impl Into<String>) -> AgenticWardenError {
        AgenticWardenError::Network {
            message: message.into(),
            url: None,
            source: None,
        }
    }

    pub fn sync_error(operation: SyncOperation, message: impl Into<String>) -> AgenticWardenError {
        AgenticWardenError::Sync {
            message: message.into(),
            operation,
            source: None,
        }
    }

    pub fn sync_error_with_source(
        operation: SyncOperation,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> AgenticWardenError {
        AgenticWardenError::Sync {
            message: message.into(),
            operation,
            source: Some(Box::new(source)),
        }
    }

    pub fn filesystem_error(
        message: impl Into<String>,
        path: impl Into<String>,
    ) -> AgenticWardenError {
        AgenticWardenError::Filesystem {
            message: message.into(),
            path: path.into(),
            source: None,
        }
    }

    pub fn tui_error(
        message: impl Into<String>,
        component: impl Into<String>,
    ) -> AgenticWardenError {
        AgenticWardenError::Tui {
            message: message.into(),
            component: component.into(),
            source: None,
        }
    }

    pub fn validation_error(
        message: impl Into<String>,
        field: Option<String>,
        value: Option<String>,
    ) -> AgenticWardenError {
        AgenticWardenError::Validation {
            message: message.into(),
            field,
            value,
        }
    }

    pub fn dependency_error(
        message: impl Into<String>,
        service: Option<String>,
    ) -> AgenticWardenError {
        AgenticWardenError::Dependency {
            message: message.into(),
            service,
        }
    }

    pub fn timeout_error(message: impl Into<String>, timeout_ms: u64) -> AgenticWardenError {
        AgenticWardenError::Timeout {
            message: message.into(),
            timeout_ms,
            source: None,
        }
    }

    pub fn concurrency_error(message: impl Into<String>) -> AgenticWardenError {
        AgenticWardenError::Concurrency {
            message: message.into(),
            operation: None,
            source: None,
        }
    }
}

/// Error recovery strategies
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff
    Retry {
        max_attempts: u32,
        base_delay_ms: u64,
    },
    /// Fall back to an alternative approach
    Fallback { alternative: String },
    /// Ask user for intervention
    UserIntervention { message: String },
    /// Log and continue
    LogAndContinue,
    /// Abort the operation
    Abort,
}

/// Rich, user-friendly error information used by the TUI and CLI.
#[derive(Debug, Clone)]
pub struct UserFacingError {
    pub title: String,
    pub message: String,
    pub hint: Option<String>,
    pub recovery: RecoveryStrategy,
}

impl AgenticWardenError {
    /// Get suggested recovery strategy
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            AgenticWardenError::Sync { operation, .. } => match operation {
                SyncOperation::GoogleDriveRequest
                | SyncOperation::NetworkProbe
                | SyncOperation::Upload
                | SyncOperation::Download => RecoveryStrategy::Retry {
                    max_attempts: 3,
                    base_delay_ms: 2000,
                },
                SyncOperation::GoogleDriveAuth | SyncOperation::OAuthCallback => {
                    RecoveryStrategy::UserIntervention {
                        message:
                            "Open the OAuth screen in the TUI to re-authorize Google Drive access"
                                .to_string(),
                    }
                }
                SyncOperation::StateVerification | SyncOperation::Discovery => {
                    RecoveryStrategy::LogAndContinue
                }
                SyncOperation::DirectoryHashing
                | SyncOperation::ConfigPacking
                | SyncOperation::ConfigLoading
                | SyncOperation::ConfigSaving
                | SyncOperation::ArchiveExtraction
                | SyncOperation::Compression => RecoveryStrategy::Abort,
                SyncOperation::Unknown => RecoveryStrategy::LogAndContinue,
            },
            AgenticWardenError::Network { .. } => RecoveryStrategy::Retry {
                max_attempts: 3,
                base_delay_ms: 1000,
            },
            AgenticWardenError::Provider { .. } => RecoveryStrategy::Fallback {
                alternative: "Use different provider or offline mode".to_string(),
            },
            AgenticWardenError::Auth { .. } => RecoveryStrategy::UserIntervention {
                message: "Please check your credentials and try again".to_string(),
            },
            AgenticWardenError::Timeout { .. } => RecoveryStrategy::Retry {
                max_attempts: 2,
                base_delay_ms: 5000,
            },
            AgenticWardenError::Config { .. } => RecoveryStrategy::Abort,
            AgenticWardenError::Filesystem { .. } => RecoveryStrategy::Abort,
            AgenticWardenError::Resource { .. } => RecoveryStrategy::Abort,
            AgenticWardenError::Dependency { .. } => RecoveryStrategy::Abort,
            AgenticWardenError::Validation { .. } => RecoveryStrategy::UserIntervention {
                message: "Please correct the input and try again".to_string(),
            },
            _ => RecoveryStrategy::LogAndContinue,
        }
    }

    /// Convert into a user-facing payload with actionable hints.
    pub fn to_user_facing(&self) -> UserFacingError {
        let hint = match self {
            AgenticWardenError::Config { .. } => Some(
                "Open the Provider screen in the TUI and fix the invalid configuration.".to_string(),
            ),
            AgenticWardenError::Provider { .. } => Some(
                "Switch to another provider or update credentials via `agentic-warden provider`."
                    .to_string(),
            ),
            AgenticWardenError::Task { .. } => Some(
                "Inspect the generated task log (see the Status screen) for command output."
                    .to_string(),
            ),
            AgenticWardenError::Sync { operation, .. } => match operation {
                SyncOperation::DirectoryHashing => Some(
                    "Ensure ~/.claude, ~/.codex or ~/.gemini exist and are readable.".to_string(),
                ),
                SyncOperation::ConfigPacking | SyncOperation::Compression => Some(
                    "Close editors that may lock the files and rerun the push operation."
                        .to_string(),
                ),
                SyncOperation::ConfigLoading | SyncOperation::ConfigSaving => Some(
                    "Validate JSON files under ~/.aiw/providers and retry.".to_string(),
                ),
                SyncOperation::ArchiveExtraction => Some(
                    "Remove the corrupted archive under ~/.aiw/sync and pull again."
                        .to_string(),
                ),
                SyncOperation::GoogleDriveAuth | SyncOperation::OAuthCallback => Some(
                    "Open the Push/Pull screen and re-run the Google Drive OAuth flow."
                        .to_string(),
                ),
                SyncOperation::GoogleDriveRequest
                | SyncOperation::NetworkProbe
                | SyncOperation::Upload
                | SyncOperation::Download => Some(
                    "Check your network connection and confirm the Google Drive API credentials."
                        .to_string(),
                ),
                SyncOperation::StateVerification | SyncOperation::Discovery => Some(
                    "Refresh the remote state from the Status screen to rebuild local caches."
                        .to_string(),
                ),
                SyncOperation::Unknown => None,
            },
            AgenticWardenError::Auth { .. } => Some(
                "Re-run OAuth from the dashboard or set the provider credentials via env vars."
                    .to_string(),
            ),
            AgenticWardenError::Network { .. } => Some(
                "Check VPN / proxy settings and retry once the connection stabilizes.".to_string(),
            ),
            AgenticWardenError::Filesystem { .. } => Some(
                "Ensure the path exists and Agentic-Warden has permission to read/write it."
                    .to_string(),
            ),
            AgenticWardenError::Tui { .. } => {
                Some("Press `r` to refresh the UI or restart the dashboard.".to_string())
            }
            AgenticWardenError::Process { .. } => Some(
                "Confirm the CLI binary is installed and accessible on PATH (set CLAUDE_BIN / CODEX_BIN / GEMINI_BIN if needed).".to_string(),
            ),
            AgenticWardenError::Validation { .. } => {
                Some("Correct the provided value and submit again.".to_string())
            }
            AgenticWardenError::Resource { .. } => Some(
                "Close other Agentic-Warden sessions to free the shared resource.".to_string(),
            ),
            AgenticWardenError::Dependency { .. } => Some(
                "Restart Agentic-Warden to re-initialize background services.".to_string(),
            ),
            AgenticWardenError::Timeout { .. } => {
                Some("Wait a few seconds and retry the operation.".to_string())
            }
            AgenticWardenError::Concurrency { .. } => Some(
                "Let the active operation finish before starting another one.".to_string(),
            ),
            AgenticWardenError::Unknown { .. } => Some(
                "Open ~/.aiw/logs/latest.log for detailed diagnostics.".to_string(),
            ),
        };

        UserFacingError {
            title: format!("{} Error", self.category().display_name()),
            message: self.user_message(),
            hint,
            recovery: self.recovery_strategy(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        let config_err = errors::config_error("test");
        assert_eq!(config_err.category(), ErrorCategory::Config);
        assert_eq!(config_err.severity(), ErrorSeverity::High);
        assert!(!config_err.is_recoverable());

        let network_err = errors::network_error("test");
        assert_eq!(network_err.category(), ErrorCategory::Network);
        assert_eq!(network_err.severity(), ErrorSeverity::Medium);
        assert!(network_err.is_recoverable());
    }

    #[test]
    fn test_error_messages() {
        let provider_err = errors::provider_error("openrouter", "API key invalid");
        assert!(provider_err.user_message().contains("openrouter"));
        assert!(provider_err.user_message().contains("API key invalid"));

        let task_err = errors::task_error(123, "Command not found", Some(127));
        assert!(task_err.user_message().contains("123"));
        assert!(task_err.user_message().contains("Command not found"));
    }

    #[test]
    fn test_recovery_strategies() {
        let network_err = errors::network_error("Connection failed");
        match network_err.recovery_strategy() {
            RecoveryStrategy::Retry {
                max_attempts,
                base_delay_ms,
            } => {
                assert_eq!(max_attempts, 3);
                assert_eq!(base_delay_ms, 1000);
            }
            _ => panic!("Expected retry strategy"),
        }

        let config_err = errors::config_error("Invalid syntax");
        match config_err.recovery_strategy() {
            RecoveryStrategy::Abort => {} // Expected
            _ => panic!("Expected abort strategy"),
        }
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new()
            .component("test_component")
            .operation("test_operation")
            .context("user_id", "123");

        assert_eq!(context.component, Some("test_component".to_string()));
        assert_eq!(context.operation, Some("test_operation".to_string()));
        assert_eq!(
            context.user_context.get("user_id"),
            Some(&"123".to_string())
        );
    }

    #[test]
    fn sync_operation_helpers_work() {
        let err = errors::sync_error(SyncOperation::Upload, "network reset");
        assert_eq!(err.sync_operation(), Some(SyncOperation::Upload));

        let payload = err.to_user_facing();
        assert!(payload.message.contains("network reset"));
        assert!(payload.title.contains("Synchronization"));
        assert!(payload.hint.unwrap().contains("network"));
    }
}
