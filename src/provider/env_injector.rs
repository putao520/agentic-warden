#![allow(dead_code)] // 环境变量注入，部分API当前未使用

//! Environment variable injection for AI CLI processes

use std::collections::HashMap;
use std::process::Command;

/// Handles environment variable injection for different AI types
pub struct EnvInjector;

impl EnvInjector {
    /// Inject environment variables into a command
    pub fn inject(cmd: &mut Command, env_vars: HashMap<String, String>) {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    /// Inject environment variables into a command
    pub fn inject_to_command(cmd: &mut Command, env_vars: &HashMap<String, String>) {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    /// Mask sensitive values for display
    pub fn mask_sensitive_value(_key: &str, value: &str) -> String {
        if value.len() <= 8 {
            return "***".to_string();
        }
        format!("{}***{}", &value[..4], &value[value.len() - 4..])
    }

    /// Mask API keys for display
    pub fn mask_api_key(key: &str) -> String {
        if key.len() <= 8 {
            return "***".to_string();
        }
        format!("{}***{}", &key[..4], &key[key.len() - 4..])
    }
}
