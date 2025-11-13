//! Common constants used across the project

/// Provider identifiers
pub mod providers {
    /// Google Drive provider identifier
    pub const GOOGLE_DRIVE: &str = "google-drive";
}

/// Common file names and paths
pub mod files {
    /// Provider configuration file name
    pub const PROVIDERS_JSON: &str = "providers.json";
}

/// Time durations
pub mod duration {
    use std::time::Duration as StdDuration;

    /// Flash message duration in seconds
    pub const FLASH_DURATION_SECS: u64 = 3;
    pub const FLASH_DURATION: StdDuration = StdDuration::from_secs(FLASH_DURATION_SECS);

    /// Provider refresh interval in seconds
    pub const PROVIDER_REFRESH_INTERVAL_SECS: u64 = 5;

    /// Default operation timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
}

/// Common UI messages
pub mod messages {
    /// Generic success messages
    pub const SUCCESS_OPERATION: &str = "Operation completed successfully";
    pub const SUCCESS_SAVED: &str = "Configuration saved successfully";

    /// Generic error messages
    pub const ERROR_OPERATION_FAILED: &str = "Operation failed";
    pub const ERROR_INVALID_INPUT: &str = "Invalid input provided";
    pub const ERROR_NETWORK: &str = "Network error occurred";
    pub const ERROR_FILE_NOT_FOUND: &str = "File not found";
    pub const ERROR_PERMISSION_DENIED: &str = "Permission denied";

    /// Confirmation messages
    pub const CONFIRM_DELETE: &str = "Are you sure you want to delete this item?";
    pub const CONFIRM_CANCEL: &str = "Are you sure you want to cancel?";

    /// Status messages
    pub const STATUS_LOADING: &str = "Loading...";
    pub const STATUS_PROCESSING: &str = "Processing...";
    pub const STATUS_WAITING: &str = "Waiting for updates...";
}

/// Common validation rules
pub mod validation {
    /// Maximum length for text fields
    pub const MAX_TEXT_LENGTH: usize = 1000;

    /// Maximum file size (100MB)
    pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;
}
