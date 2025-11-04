//! Unified error handling for the agentic-warden project
//!
//! This module provides a comprehensive error handling system with
//! proper error classification, context, and recovery mechanisms.

use std::collections::HashMap;
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
    #[error("Sync error: {message}")]
    Sync {
        message: String,
        operation: String,
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

    /// Get user-friendly message
    pub fn user_message(&self) -> String {
        match self {
            AgenticWardenError::Config { message, .. } => {
                format!("Configuration problem: {}", message)
            }
            AgenticWardenError::Provider { provider, message, .. } => {
                format!("Provider '{}' issue: {}", provider, message)
            }
            AgenticWardenError::Task { task_id, message, .. } => {
                format!("Task #{} failed: {}", task_id, message)
            }
            AgenticWardenError::Sync { message, .. } => {
                format!("Sync problem: {}", message)
            }
            AgenticWardenError::Auth { message, provider, .. } => {
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
            AgenticWardenError::Provider { provider, error_code, source, .. } => {
                let mut details = format!("Provider error - Provider: {}", provider);
                if let Some(code) = error_code {
                    details.push_str(&format!(", Code: {}", code));
                }
                if let Some(src) = source {
                    details.push_str(&format!(", Source: {}", src));
                }
                details
            }
            AgenticWardenError::Task { task_id, exit_code, source, .. } => {
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

    pub fn wrap_error<T>(self, result: Result<T, impl Into<AgenticWardenError>>) -> AgenticResult<T> {
        result.map_err(|e| {
            let mut error = e.into();

            // Apply context to the error if possible
            // This would require extending the error types to support context
            // For now, we just return the error as-is

            error
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

    pub fn tui_error(message: impl Into<String>, component: impl Into<String>) -> AgenticWardenError {
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

    pub fn dependency_error(message: impl Into<String>, service: Option<String>) -> AgenticWardenError {
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
    Retry { max_attempts: u32, base_delay_ms: u64 },
    /// Fall back to an alternative approach
    Fallback { alternative: String },
    /// Ask user for intervention
    UserIntervention { message: String },
    /// Log and continue
    LogAndContinue,
    /// Abort the operation
    Abort,
}

impl AgenticWardenError {
    /// Get suggested recovery strategy
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
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
            RecoveryStrategy::Retry { max_attempts, base_delay_ms } => {
                assert_eq!(max_attempts, 3);
                assert_eq!(base_delay_ms, 1000);
            }
            _ => panic!("Expected retry strategy"),
        }

        let config_err = errors::config_error("Invalid syntax");
        match config_err.recovery_strategy() {
            RecoveryStrategy::Abort => {}, // Expected
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
        assert_eq!(context.user_context.get("user_id"), Some(&"123".to_string()));
    }
}