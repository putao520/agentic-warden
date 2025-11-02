//! Environment variable injector for AI CLI processes

use std::collections::HashMap;
use std::process::Command;

pub struct EnvInjector;

impl EnvInjector {
    /// Inject environment variables to a Command object
    ///
    /// This method does not modify the current process's environment variables,
    /// it only passes these environment variables when spawning the child process
    pub fn inject_to_command(cmd: &mut Command, env_vars: &HashMap<String, String>) {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    /// Get a safe display version of environment variable (hide sensitive info)
    pub fn mask_sensitive_value(key: &str, value: &str) -> String {
        // Variables containing KEY, SECRET, TOKEN, PASSWORD need to be hidden
        let sensitive_keywords = ["KEY", "SECRET", "TOKEN", "PASSWORD"];

        if sensitive_keywords
            .iter()
            .any(|kw| key.to_uppercase().contains(kw))
        {
            Self::mask_api_key(value)
        } else {
            value.to_string()
        }
    }

    /// Hide API key (only show first 4 and last 4 characters)
    fn mask_api_key(key: &str) -> String {
        if key.len() <= 8 {
            return "***".to_string();
        }
        format!("{}***{}", &key[..4], &key[key.len() - 4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(
            EnvInjector::mask_api_key("sk-ant-api-1234567890abcdef"),
            "sk-a***cdef"
        );

        assert_eq!(EnvInjector::mask_api_key("short"), "***");
    }

    #[test]
    fn test_mask_sensitive_value() {
        // "sk-1234567890" is 12 chars: show first 4 ("sk-1") and last 4 ("7890")
        assert_eq!(
            EnvInjector::mask_sensitive_value("OPENAI_API_KEY", "sk-1234567890"),
            "sk-1***7890"
        );

        assert_eq!(
            EnvInjector::mask_sensitive_value("OPENAI_BASE_URL", "https://api.openai.com"),
            "https://api.openai.com"
        );

        // "token123456789" is 14 chars: show first 4 ("toke") and last 4 ("6789")
        assert_eq!(
            EnvInjector::mask_sensitive_value("MY_SECRET_TOKEN", "token123456789"),
            "toke***6789"
        );
    }

    #[test]
    fn test_inject_to_command() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_KEY".to_string(), "test_value".to_string());
        env_vars.insert("ANOTHER_KEY".to_string(), "another_value".to_string());

        let mut cmd = Command::new("echo");
        EnvInjector::inject_to_command(&mut cmd, &env_vars);

        // Verify parent process environment variables are not modified
        assert!(std::env::var("TEST_KEY").is_err());
        assert!(std::env::var("ANOTHER_KEY").is_err());

        // Ensure the command carries the injected environment variables
        let collected: HashMap<_, _> = cmd
            .get_envs()
            .filter_map(|(key, value)| {
                value.map(|v| {
                    (
                        key.to_string_lossy().into_owned(),
                        v.to_string_lossy().into_owned(),
                    )
                })
            })
            .collect();

        assert_eq!(collected.get("TEST_KEY"), Some(&"test_value".to_string()));
        assert_eq!(
            collected.get("ANOTHER_KEY"),
            Some(&"another_value".to_string())
        );
    }

    #[test]
    fn mask_sensitive_value_handles_short_values() {
        assert_eq!(EnvInjector::mask_sensitive_value("API_KEY", "short"), "***");
        assert_eq!(
            EnvInjector::mask_sensitive_value("TOKEN", "12345678"),
            "***"
        );
    }
}
