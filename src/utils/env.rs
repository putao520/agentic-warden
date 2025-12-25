//! Environment variable utilities with platform-specific behavior.

/// Expand environment variable placeholder (${VAR_NAME})
///
/// # Platform Behavior
/// - **Windows**: Case-insensitive (e.g., `%USERNAME%` == `%username%`)
/// - **Linux/macOS**: Case-sensitive (e.g., `$USER` != `$user`)
///
/// # Examples
/// ```
/// // Linux/macOS
/// assert_eq!(expand_env_var("${HOME}"), "/home/user");
/// assert_eq!(expand_env_var("${UNDEFINED_VAR}"), "${UNDEFINED_VAR}");
///
/// // Windows
/// // Expands case-insensitively
/// ```
pub fn expand_env_var(value: &str) -> String {
    if !value.starts_with("${") || !value.ends_with('}') {
        return value.to_string();
    }

    let var_name = &value[2..value.len() - 1];

    #[cfg(windows)]
    {
        // Windows environment variables are case-insensitive
        std::env::vars()
            .find(|(k, _)| k.eq_ignore_ascii_case(var_name))
            .map(|(_, v)| v)
            .unwrap_or_else(|| value.to_string())
    }

    #[cfg(not(windows))]
    {
        // Linux/macOS environment variables are case-sensitive
        std::env::var(var_name).unwrap_or_else(|_| value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_placeholder() {
        assert_eq!(expand_env_var("plain_value"), "plain_value");
    }

    #[test]
    fn test_empty_placeholder() {
        assert_eq!(expand_env_var("${}"), "${}");
    }

    #[test]
    fn test_malformed_placeholder() {
        // Malformed placeholders are returned as-is
        assert_eq!(expand_env_var("${INCOMPLETE"), "${INCOMPLETE");
        assert_eq!(expand_env_var("NO_CLOSE_BRACE"), "NO_CLOSE_BRACE");
    }

    #[test]
    fn test_expansion_existing_var() {
        // HOME should exist on most systems
        let expanded = expand_env_var("${HOME}");
        // Should not return the placeholder if HOME exists
        if std::env::var("HOME").is_ok() {
            assert_ne!(expanded, "${HOME}");
        }
    }

    #[test]
    fn test_expansion_missing_var() {
        // UNDEFINED_VAR_xyz123 should not exist
        assert_eq!(expand_env_var("${UNDEFINED_VAR_xyz123}"), "${UNDEFINED_VAR_xyz123}");
    }

    #[cfg(windows)]
    #[test]
    fn test_case_insensitive() {
        // This test can only run if we set a known variable
        // For demonstration purposes only
        if std::env::var("PATH").is_ok() {
            let upper = expand_env_var("${PATH}");
            let lower = expand_env_var("${path}");
            // On Windows, both should expand the same way
            assert_eq!(upper, lower);
        }
    }
}
