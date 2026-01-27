use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::auto_mode::{default_execution_entries, ExecutionEntry};
use crate::cli_type::CliType;
use crate::error::ConfigError;
use crate::utils::config_paths::ConfigPaths;

const ALLOWED_CLI_TYPES: [&str; 3] = ["codex", "claude", "gemini"];

pub struct ExecutionOrderConfig;

impl ExecutionOrderConfig {
    /// 获取 CLI+Provider 执行顺序（新格式）
    pub fn get_execution_entries() -> Result<Vec<ExecutionEntry>, ConfigError> {
        Self::load_execution_entries()
    }

    /// 验证执行条目
    pub fn validate_entries(entries: &[ExecutionEntry]) -> Result<(), ConfigError> {
        if entries.is_empty() {
            return Err(ConfigError::InvalidLength {
                expected: 1,
                actual: 0,
            });
        }

        for entry in entries {
            if !ALLOWED_CLI_TYPES.contains(&entry.cli.to_lowercase().as_str()) {
                return Err(ConfigError::InvalidCliType {
                    value: entry.cli.clone(),
                });
            }
            // Provider 验证：允许 "auto" 或任何非空字符串
            if entry.provider.trim().is_empty() {
                return Err(ConfigError::InvalidCliType {
                    value: format!("empty provider for cli '{}'", entry.cli),
                });
            }
        }

        Ok(())
    }

    /// 重置为默认配置
    pub fn reset_to_default() -> Vec<ExecutionEntry> {
        default_execution_entries()
    }

    /// 保存执行顺序
    pub fn save_entries(entries: &[ExecutionEntry]) -> Result<(), ConfigError> {
        Self::validate_entries(entries)?;

        let path = Self::config_path()?;
        let mut config = Self::load_config_object(&path)?;

        let entries_json: Vec<Value> = entries
            .iter()
            .map(|e| {
                let mut obj = Map::new();
                obj.insert("cli".to_string(), Value::String(e.cli.clone()));
                obj.insert("provider".to_string(), Value::String(e.provider.clone()));
                Value::Object(obj)
            })
            .collect();

        config.insert(
            "auto_execution_order".to_string(),
            Value::Array(entries_json),
        );

        Self::write_config(&path, Value::Object(config))?;
        Ok(())
    }

    fn load_execution_entries() -> Result<Vec<ExecutionEntry>, ConfigError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(default_execution_entries());
        }

        let config = Self::load_config_value(&path)?;
        let order_value = match &config {
            Value::Object(map) => map.get("auto_execution_order"),
            _ => return Err(ConfigError::InvalidFormat),
        };

        let order_value = match order_value {
            Some(value) => value,
            None => return Ok(default_execution_entries()),
        };

        let entries = Self::parse_entries_value(order_value)?;
        Self::validate_entries(&entries)?;
        Ok(entries)
    }

    fn parse_entries_value(value: &Value) -> Result<Vec<ExecutionEntry>, ConfigError> {
        let array = value.as_array().ok_or(ConfigError::InvalidType)?;
        let mut entries = Vec::with_capacity(array.len());

        for item in array {
            let obj = item.as_object().ok_or(ConfigError::InvalidElementType)?;

            let cli = obj
                .get("cli")
                .and_then(|v| v.as_str())
                .ok_or(ConfigError::InvalidElementType)?
                .to_string();

            let provider = obj
                .get("provider")
                .and_then(|v| v.as_str())
                .ok_or(ConfigError::InvalidElementType)?
                .to_string();

            entries.push(ExecutionEntry { cli, provider });
        }

        Ok(entries)
    }

    fn load_config_value(path: &Path) -> Result<Value, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|err| ConfigError::Io { message: err.to_string() })?;
        serde_json::from_str(&content).map_err(|_| ConfigError::InvalidFormat)
    }

    fn load_config_object(path: &Path) -> Result<Map<String, Value>, ConfigError> {
        let paths = ConfigPaths::new()
            .map_err(|err| ConfigError::Io { message: err.to_string() })?;
        paths
            .ensure_dirs()
            .map_err(|err| ConfigError::Io { message: err.to_string() })?;

        if !path.exists() {
            return Ok(Map::new());
        }

        let value = Self::load_config_value(path)?;
        match value {
            Value::Object(map) => Ok(map),
            _ => Err(ConfigError::InvalidFormat),
        }
    }

    fn write_config(path: &Path, value: Value) -> Result<(), ConfigError> {
        let payload = serde_json::to_string_pretty(&value)
            .map_err(|err| ConfigError::Io { message: err.to_string() })?;
        fs::write(path, payload).map_err(|err| ConfigError::Io {
            message: err.to_string(),
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)
                .map_err(|err| ConfigError::Io { message: err.to_string() })?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms).map_err(|err| ConfigError::Io {
                message: err.to_string(),
            })?;
        }

        Ok(())
    }

    fn config_path() -> Result<PathBuf, ConfigError> {
        let paths =
            ConfigPaths::new().map_err(|err| ConfigError::Io { message: err.to_string() })?;
        Ok(paths.config_file)
    }
}
