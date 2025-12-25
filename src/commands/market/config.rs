//! JSON config management and legacy migration.

use crate::commands::market::plugin::{McpServerConfig, McpServerConfigWrite};
use crate::commands::market::source::{
    default_marketplaces, MarketError, MarketErrorCode, MarketResult, MarketplaceSettingsEntry,
};
use crate::utils::config_paths::ConfigPaths;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsFile {
    #[serde(rename = "extraKnownMarketplaces")]
    pub extra_known_marketplaces: HashMap<String, MarketplaceSettingsEntry>,
    #[serde(rename = "enabledPlugins", default)]
    pub enabled_plugins: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginsFile {
    pub plugins: HashMap<String, InstalledPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub version: String,
    pub installed_at: String,
    pub enabled: bool,
    pub source: String,
}

/// Internal format for MCP config (uses parsed McpServerConfig enum)
#[derive(Debug, Clone, Default)]
pub struct McpConfigFile {
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// Serializable format for mcp.json (stdio only)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfigFileWrite {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfigWrite>,
}

impl From<&McpConfigFile> for McpConfigFileWrite {
    fn from(file: &McpConfigFile) -> Self {
        let mcp_servers = file
            .mcp_servers
            .iter()
            .filter_map(|(name, config)| {
                config.to_write_format().map(|write_cfg| (name.clone(), write_cfg))
            })
            .collect();

        Self { mcp_servers }
    }
}

impl Serialize for McpConfigFile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let write_format: McpConfigFileWrite = self.into();
        write_format.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for McpConfigFile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Try to deserialize as the tagged enum format
        let write_format = McpConfigFileWrite::deserialize(deserializer)?;

        // Convert all configs to the parsed format
        let mcp_servers = write_format
            .mcp_servers
            .into_iter()
            .map(|(name, write_cfg)| {
                let config = McpServerConfig::Stdio {
                    command: write_cfg.command,
                    args: write_cfg.args,
                    env: if write_cfg.env.is_empty() {
                        None
                    } else {
                        Some(write_cfg.env)
                    },
                };
                (name, config)
            })
            .collect();

        Ok(McpConfigFile { mcp_servers })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegacyMcpConfig {
    #[serde(rename = "mcpServers", alias = "mcp_servers", default)]
    pub mcp_servers: HashMap<String, LegacyMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegacyMcpServer {
    pub source: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

pub struct ConfigStore {
    settings_path: PathBuf,
    plugins_path: PathBuf,
    mcp_path: PathBuf,
    config_dir: PathBuf,
}

impl ConfigStore {
    pub fn new() -> MarketResult<Self> {
        let paths = ConfigPaths::new().map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to resolve config paths",
                err.into(),
            )
        })?;
        paths.ensure_dirs().map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to create config directories",
                err.into(),
            )
        })?;
        let settings_path = paths.config_dir.join("settings.json");
        let plugins_path = paths.config_dir.join("plugins.json");
        let mcp_path = paths.config_dir.join("mcp.json");
        let store = Self {
            settings_path,
            plugins_path,
            mcp_path,
            config_dir: paths.config_dir,
        };
        store.ensure_permissions()?;
        Ok(store)
    }

    fn ensure_permissions(&self) -> MarketResult<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.config_dir, fs::Permissions::from_mode(0o700)).map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Failed to set config directory permissions",
                    err.into(),
                )
            })?;
        }
        Ok(())
    }

    pub fn migrate_legacy_configs(&self) -> MarketResult<()> {
        let legacy_path = self.config_dir.join("mcp_servers.yaml");
        if !legacy_path.exists() {
            return Ok(());
        }

        let contents = fs::read_to_string(&legacy_path).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to read legacy mcp_servers.yaml",
                err.into(),
            )
        })?;
        let legacy: LegacyMcpConfig = serde_yaml::from_str(&contents).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to parse legacy mcp_servers.yaml",
                err.into(),
            )
        })?;
        let mut mcp_config = self.load_mcp()?;
        for (name, server) in legacy.mcp_servers {
            mcp_config.mcp_servers.entry(name).or_insert(McpServerConfig::Stdio {
                command: server.command,
                args: server.args,
                env: if server.env.is_empty() {
                    None
                } else {
                    Some(server.env)
                },
            });
        }
        self.save_mcp(&mcp_config)?;
        backup_file(&legacy_path)?;
        Ok(())
    }

    pub fn load_settings(&self) -> MarketResult<SettingsFile> {
        let mut settings = if self.settings_path.exists() {
            let contents = fs::read_to_string(&self.settings_path).map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Failed to read settings.json",
                    err.into(),
                )
            })?;
            serde_json::from_str(&contents).map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Invalid settings.json format",
                    err.into(),
                )
            })?
        } else {
            SettingsFile::default()
        };
        for (name, entry) in default_marketplaces() {
            settings.extra_known_marketplaces.entry(name).or_insert(entry);
        }
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &SettingsFile) -> MarketResult<()> {
        write_json_file(&self.settings_path, settings)
    }

    pub fn load_plugins(&self) -> MarketResult<PluginsFile> {
        if self.plugins_path.exists() {
            let contents = fs::read_to_string(&self.plugins_path).map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Failed to read plugins.json",
                    err.into(),
                )
            })?;
            let plugins = serde_json::from_str(&contents).map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Invalid plugins.json format",
                    err.into(),
                )
            })?;
            Ok(plugins)
        } else {
            Ok(PluginsFile::default())
        }
    }

    pub fn save_plugins(&self, plugins: &PluginsFile) -> MarketResult<()> {
        write_json_file(&self.plugins_path, plugins)
    }

    pub fn load_mcp(&self) -> MarketResult<McpConfigFile> {
        if self.mcp_path.exists() {
            let contents = fs::read_to_string(&self.mcp_path).map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Failed to read mcp.json",
                    err.into(),
                )
            })?;
            let config = serde_json::from_str(&contents).map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Invalid mcp.json format",
                    err.into(),
                )
            })?;
            Ok(config)
        } else {
            Ok(McpConfigFile::default())
        }
    }

    pub fn save_mcp(&self, config: &McpConfigFile) -> MarketResult<()> {
        write_json_file(&self.mcp_path, config)
    }

    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    pub fn plugins_path(&self) -> &Path {
        &self.plugins_path
    }

    pub fn mcp_path(&self) -> &Path {
        &self.mcp_path
    }
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> MarketResult<()> {
    let data = serde_json::to_string_pretty(value).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::ConfigWriteFailed,
            "Failed to serialize JSON",
            err.into(),
        )
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to create config directory",
                err.into(),
            )
        })?;
    }
    fs::write(path, data).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::ConfigWriteFailed,
            "Failed to write config file",
            err.into(),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to set config file permissions",
                err.into(),
            )
        })?;
    }
    Ok(())
}

fn backup_file(path: &Path) -> MarketResult<()> {
    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let file_name = path
        .file_name()
        .ok_or_else(|| MarketError::new(MarketErrorCode::ConfigWriteFailed, "Invalid backup file"))?
        .to_string_lossy();
    let backup_name = format!("{}.bak-{}", file_name, timestamp);
    let backup = path.with_file_name(backup_name);
    fs::rename(path, backup).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::ConfigWriteFailed,
            "Failed to backup legacy config",
            err.into(),
        )
    })?;
    Ok(())
}
