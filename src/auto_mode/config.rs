use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::auto_mode::DEFAULT_EXECUTION_ORDER;
use crate::cli_type::CliType;
use crate::error::ConfigError;
use crate::utils::config_paths::ConfigPaths;

const ALLOWED_CLI_TYPES: [&str; 3] = ["codex", "claude", "gemini"];

pub struct ExecutionOrderConfig;

impl ExecutionOrderConfig {
    pub fn get_order() -> Result<Vec<CliType>, ConfigError> {
        let order = Self::load_order_strings()?;
        Self::order_to_types(&order)
    }

    pub fn validate_order(order: &[String]) -> Result<(), ConfigError> {
        if order.len() != ALLOWED_CLI_TYPES.len() {
            return Err(ConfigError::InvalidLength {
                expected: ALLOWED_CLI_TYPES.len(),
                actual: order.len(),
            });
        }

        for value in order {
            if !ALLOWED_CLI_TYPES.contains(&value.as_str()) {
                return Err(ConfigError::InvalidCliType {
                    value: value.clone(),
                });
            }
        }

        let unique: HashSet<_> = order.iter().collect();
        if unique.len() != ALLOWED_CLI_TYPES.len() {
            return Err(ConfigError::DuplicateCliType);
        }

        let required: HashSet<_> = ALLOWED_CLI_TYPES.iter().copied().collect();
        let current: HashSet<_> = order.iter().map(|s| s.as_str()).collect();
        if current != required {
            return Err(ConfigError::IncompleteSet);
        }

        Ok(())
    }

    pub fn reset_to_default() -> Vec<CliType> {
        vec![CliType::Codex, CliType::Gemini, CliType::Claude]
    }

    pub fn save_order(order: &[CliType]) -> Result<(), ConfigError> {
        let order_strings = order
            .iter()
            .map(|cli_type| cli_type.display_name().to_string())
            .collect::<Vec<_>>();
        Self::validate_order(&order_strings)?;

        let path = Self::config_path()?;
        let mut config = Self::load_config_object(&path)?;
        config.insert(
            "cli_execution_order".to_string(),
            Value::Array(order_strings.into_iter().map(Value::String).collect()),
        );

        Self::write_config(&path, Value::Object(config))?;
        Ok(())
    }

    fn order_to_types(order: &[String]) -> Result<Vec<CliType>, ConfigError> {
        order
            .iter()
            .map(|value| match value.as_str() {
                "codex" => Ok(CliType::Codex),
                "claude" => Ok(CliType::Claude),
                "gemini" => Ok(CliType::Gemini),
                _ => Err(ConfigError::InvalidCliType {
                    value: value.clone(),
                }),
            })
            .collect()
    }

    fn load_order_strings() -> Result<Vec<String>, ConfigError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default_order_strings());
        }

        let config = Self::load_config_value(&path)?;
        let order_value = match &config {
            Value::Object(map) => map.get("cli_execution_order"),
            _ => return Err(ConfigError::InvalidFormat),
        };
        let order_value = match order_value {
            Some(value) => value,
            None => return Ok(Self::default_order_strings()),
        };

        let order = Self::parse_order_value(order_value)?;
        Self::validate_order(&order)?;
        Ok(order)
    }

    fn parse_order_value(value: &Value) -> Result<Vec<String>, ConfigError> {
        let array = value.as_array().ok_or(ConfigError::InvalidType)?;
        let mut order = Vec::with_capacity(array.len());
        for item in array {
            let value = item.as_str().ok_or(ConfigError::InvalidElementType)?;
            order.push(value.to_string());
        }
        Ok(order)
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

    fn default_order_strings() -> Vec<String> {
        DEFAULT_EXECUTION_ORDER
            .iter()
            .map(|value| value.to_string())
            .collect()
    }
}
