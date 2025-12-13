//! Common data structures used across multiple modules

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Common result type used across the application
pub type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Generic progress tracking structure
#[derive(Debug, Clone)]
pub struct Progress {
    pub current: u64,
    pub total: u64,
    pub message: Option<String>,
    pub percent: u8,
}

impl Progress {
    pub fn new(total: u64) -> Self {
        Self {
            current: 0,
            total,
            message: None,
            percent: 0,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    pub fn update(&mut self, current: u64, message: Option<String>) {
        self.current = current.min(self.total);
        self.message = message;
        self.percent = if self.total > 0 {
            ((self.current as f64 / self.total as f64) * 100.0) as u8
        } else {
            0
        };
    }

    pub fn is_complete(&self) -> bool {
        self.current >= self.total
    }
}

/// Generic operation result with metadata
#[derive(Debug, Clone)]
pub struct OperationResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
    pub duration_ms: Option<u64>,
    pub metadata: HashMap<String, String>,
}

impl<T> OperationResult<T> {
    pub fn success(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message,
            duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message,
            duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Common item identifier and type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemId {
    pub id: String,
    pub item_type: ItemType,
}

impl ItemId {
    pub fn new(id: String, item_type: ItemType) -> Self {
        Self { id, item_type }
    }
}

/// Common item types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemType {
    File,
    Directory,
    Provider,
    Task,
    Config,
    Sync,
    Auth,
}

/// Common status enum
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// Common timestamp utilities
pub mod timestamp {
    use super::*;

    /// Get current Unix timestamp in milliseconds
    pub fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Get current Unix timestamp in seconds
    pub fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Format timestamp as human-readable string
    pub fn format(timestamp_ms: u64) -> String {
        let datetime = chrono::DateTime::from_timestamp(
            (timestamp_ms / 1000) as i64,
            ((timestamp_ms % 1000) * 1_000_000) as u32,
        );
        datetime
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Invalid timestamp".to_string())
    }
}

/// Common validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Common configuration structure
#[derive(Debug, Clone)]
pub struct CommonConfig {
    pub name: String,
    pub enabled: bool,
    pub settings: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, String>,
}

impl CommonConfig {
    pub fn new(name: String) -> Self {
        Self {
            name,
            enabled: true,
            settings: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn get_setting<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        self.settings
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set_setting<T>(&mut self, key: String, value: T) -> Result<(), serde_json::Error>
    where
        T: serde::Serialize,
    {
        let json_value = serde_json::to_value(value)?;
        self.settings.insert(key, json_value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress() {
        let mut progress = Progress::new(100).with_message("Testing".to_string());
        assert_eq!(progress.percent, 0);
        assert!(!progress.is_complete());

        progress.update(50, Some("Half done".to_string()));
        assert_eq!(progress.percent, 50);
        assert_eq!(progress.message, Some("Half done".to_string()));

        progress.update(100, None);
        assert_eq!(progress.percent, 100);
        assert!(progress.is_complete());
    }

    #[test]
    fn test_operation_result() {
        let result = OperationResult::success(42, "Success".to_string())
            .with_duration(100)
            .with_metadata("key".to_string(), "value".to_string());

        assert!(result.success);
        assert_eq!(result.data, Some(42));
        assert_eq!(result.message, "Success");
        assert_eq!(result.duration_ms, Some(100));
        assert_eq!(result.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::valid();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());

        result.add_error("Test error".to_string());
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_timestamp() {
        let now = timestamp::now_ms();
        assert!(now > 0);

        let formatted = timestamp::format(now);
        assert!(!formatted.is_empty());
        assert!(!formatted.eq("Invalid timestamp"));
    }
}
