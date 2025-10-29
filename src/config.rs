use std::time::Duration;

pub const CLAUDE_BIN: &str = "claude";
pub const CODEX_BIN: &str = "codex";
pub const GEMINI_BIN: &str = "gemini";
pub const SHARED_NAMESPACE: &str = "agentic-task";
pub const SHARED_MEMORY_SIZE: usize = 4 * 1024 * 1024;

pub const WAIT_INTERVAL_ENV: &str = "AGENTIC_WARDEN_WAIT_INTERVAL_SEC";
pub const LEGACY_WAIT_INTERVAL_ENV: &str = "CODEX_WORKER_WAIT_INTERVAL_SEC";
pub const DEBUG_ENV: &str = "AGENTIC_WARDEN_DEBUG";
pub const LEGACY_DEBUG_ENV: &str = "CODEX_WORKER_DEBUG";
pub const PROCESS_TREE_FEATURE_ENV: &str = "AGENTIC_WARDEN_ENABLE_PROCESS_TREE";

pub const MAX_RECORD_AGE: Duration = Duration::from_secs(12 * 60 * 60);
pub const WAIT_INTERVAL_DEFAULT: Duration = Duration::from_secs(30);
pub const MAX_WAIT_DURATION: Duration = Duration::from_secs(24 * 60 * 60);

/// Check if process tree feature is enabled (default: true)
pub fn is_process_tree_enabled() -> bool {
    match std::env::var(PROCESS_TREE_FEATURE_ENV) {
        Ok(value) => {
            // Accept "true", "1", "yes", "on" as enabled
            !matches!(value.to_lowercase().as_str(), "false" | "0" | "no" | "off")
        }
        Err(_) => true, // Default to enabled for backward compatibility
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_tree_enabled_default() {
        // When env var is not set, should return true
        unsafe {
            std::env::remove_var(PROCESS_TREE_FEATURE_ENV);
        }
        assert!(is_process_tree_enabled());
    }

    #[test]
    fn test_process_tree_enabled_true_values() {
        let true_values = ["true", "1", "yes", "on", "TRUE", "Yes", "ON"];

        for value in &true_values {
            unsafe {
                std::env::set_var(PROCESS_TREE_FEATURE_ENV, value);
            }
            assert!(is_process_tree_enabled());
        }
    }

    #[test]
    fn test_process_tree_enabled_false_values() {
        let false_values = ["false", "0", "no", "off", "FALSE", "No", "OFF"];

        for value in &false_values {
            unsafe {
                std::env::set_var(PROCESS_TREE_FEATURE_ENV, value);
            }
            assert!(!is_process_tree_enabled());
        }
    }

    #[test]
    fn test_process_tree_enabled_invalid_values() {
        // Invalid values should default to enabled
        let invalid_values = ["invalid", "maybe", "2", "-1"];

        for value in &invalid_values {
            unsafe {
                std::env::set_var(PROCESS_TREE_FEATURE_ENV, value);
            }
            assert!(is_process_tree_enabled());
        }
    }
}
