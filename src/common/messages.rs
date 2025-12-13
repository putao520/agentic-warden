//! Common messages and error handling utilities

use crate::common::constants::messages;

/// Common user interface messages
pub struct UIMessages;

impl UIMessages {
    /// Get a success message
    pub fn success(msg_type: SuccessType) -> &'static str {
        match msg_type {
            SuccessType::OperationComplete => messages::SUCCESS_OPERATION,
            SuccessType::ConfigurationSaved => messages::SUCCESS_SAVED,
        }
    }

    /// Get an error message
    pub fn error(msg_type: ErrorType) -> &'static str {
        match msg_type {
            ErrorType::OperationFailed => messages::ERROR_OPERATION_FAILED,
            ErrorType::InvalidInput => messages::ERROR_INVALID_INPUT,
            ErrorType::NetworkError => messages::ERROR_NETWORK,
            ErrorType::FileNotFound => messages::ERROR_FILE_NOT_FOUND,
            ErrorType::PermissionDenied => messages::ERROR_PERMISSION_DENIED,
        }
    }

    /// Get a confirmation message
    pub fn confirmation(msg_type: ConfirmationType) -> &'static str {
        match msg_type {
            ConfirmationType::Delete => messages::CONFIRM_DELETE,
            ConfirmationType::Cancel => messages::CONFIRM_CANCEL,
        }
    }

    /// Get a status message
    pub fn status(msg_type: StatusType) -> &'static str {
        match msg_type {
            StatusType::Loading => messages::STATUS_LOADING,
            StatusType::Processing => messages::STATUS_PROCESSING,
            StatusType::Waiting => messages::STATUS_WAITING,
        }
    }
}

/// Types of success messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessType {
    OperationComplete,
    ConfigurationSaved,
}

/// Types of error messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    OperationFailed,
    InvalidInput,
    NetworkError,
    FileNotFound,
    PermissionDenied,
}

/// Types of confirmation messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationType {
    Delete,
    Cancel,
}

/// Types of status messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusType {
    Loading,
    Processing,
    Waiting,
}

/// Helper for creating formatted status messages
pub fn format_progress(percent: u8, message: &str) -> String {
    format!("{}% complete - {}", percent, message)
}

/// Helper for creating error context messages
pub fn error_context(error_type: ErrorType, context: &str) -> String {
    format!("{}: {}", UIMessages::error(error_type), context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_messages() {
        assert_eq!(
            UIMessages::success(SuccessType::OperationComplete),
            "Operation completed successfully"
        );
        assert_eq!(
            UIMessages::error(ErrorType::NetworkError),
            "Network error occurred"
        );
        assert_eq!(
            UIMessages::confirmation(ConfirmationType::Delete),
            "Are you sure you want to delete this item?"
        );
        assert_eq!(UIMessages::status(StatusType::Loading), "Loading...");
    }

    #[test]
    fn test_format_progress() {
        assert_eq!(
            format_progress(50, "Processing files"),
            "50% complete - Processing files"
        );
    }

    #[test]
    fn test_error_context() {
        assert_eq!(
            error_context(ErrorType::FileNotFound, "config.json"),
            "File not found: config.json"
        );
    }
}
