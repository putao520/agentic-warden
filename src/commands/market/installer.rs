//! Plugin installer implementation.

use crate::commands::market::cache::MarketCacheManager;
use crate::commands::market::config::{ConfigStore, InstalledPlugin, PluginsFile};
use crate::commands::market::plugin::{McpServersFile, PluginDetail};
use crate::commands::market::plugin_io::{extract_mcp_config, load_manifest};
use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult, MarketSource};
use chrono::Utc;
use dialoguer::{Confirm, Input};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub struct PluginInstaller {
    pub config: ConfigStore,
    pub cache: MarketCacheManager,
}

impl PluginInstaller {
    pub fn new() -> MarketResult<Self> {
        let config = ConfigStore::new()?;
        config.migrate_legacy_configs()?;
        let cache = MarketCacheManager::new()?;
        Ok(Self { config, cache })
    }

    pub async fn install(
        &self,
        source: &dyn MarketSource,
        detail: &PluginDetail,
        env_vars: HashMap<String, String>,
        skip_env: bool,
    ) -> MarketResult<InstalledPlugin> {
        let plugin_id = detail.manifest.name.clone();
        let cache_path = source
            .download_plugin(&detail.entry, &plugin_id)
            .await?;
        let manifest_path = cache_path.join(".claude-plugin").join("plugin.json");
        let manifest = load_manifest(&manifest_path)?;
        let mcp_config = extract_mcp_config(&manifest, &cache_path)?
            .ok_or_else(|| MarketError::new(MarketErrorCode::PluginMissingMcp, "Plugin has no MCP servers"))?;
        if mcp_config.mcp_servers.is_empty() {
            return Err(MarketError::new(
                MarketErrorCode::PluginMissingMcp,
                "Plugin has no MCP servers",
            ));
        }

        let mut normalized = normalize_mcp_env(mcp_config, &env_vars, skip_env)?;
        merge_mcp_config(&self.config, &mut normalized)?;

        let installed_at = Utc::now();
        self.cache
            .write_installed_at(&plugin_id, source.name(), installed_at)?;
        let installed = InstalledPlugin {
            version: manifest.version,
            installed_at: installed_at.to_rfc3339(),
            enabled: true,
            source: source.name().to_string(),
        };

        let mut plugins = self.config.load_plugins()?;
        plugins
            .plugins
            .insert(format!("{}@{}", plugin_id, source.name()), installed.clone());
        self.config.save_plugins(&plugins)?;

        let mut settings = self.config.load_settings()?;
        settings
            .enabled_plugins
            .insert(plugin_id, true);
        self.config.save_settings(&settings)?;

        Ok(installed)
    }

    pub fn list_installed(&self) -> MarketResult<PluginsFile> {
        self.config.load_plugins()
    }

    pub fn remove_plugin(&self, plugin_name: &str) -> MarketResult<Vec<String>> {
        let mut plugins = self.config.load_plugins()?;
        let mut removed = Vec::new();
        let keys: Vec<String> = plugins
            .plugins
            .keys()
            .filter(|key| key.split('@').next() == Some(plugin_name))
            .cloned()
            .collect();
        if keys.is_empty() {
            return Err(MarketError::new(
                MarketErrorCode::PluginNotFound,
                "Plugin not installed",
            ));
        }

        for key in keys {
            if let Some(record) = plugins.plugins.remove(&key) {
                let cache_path = self
                    .cache
                    .plugin_cache_path(plugin_name, &record.source);
                let _ = remove_mcp_entries(&self.config, &cache_path);
                let _ = fs::remove_dir_all(&cache_path);
                removed.push(key);
            }
        }
        self.config.save_plugins(&plugins)?;
        let mut settings = self.config.load_settings()?;
        settings.enabled_plugins.remove(plugin_name);
        self.config.save_settings(&settings)?;
        Ok(removed)
    }

    pub fn set_enabled(&self, plugin_name: &str, enabled: bool) -> MarketResult<()> {
        let mut plugins = self.config.load_plugins()?;
        let mut found = false;
        for (key, plugin) in plugins.plugins.iter_mut() {
            if key.split('@').next() == Some(plugin_name) {
                plugin.enabled = enabled;
                found = true;
            }
        }
        if !found {
            return Err(MarketError::new(
                MarketErrorCode::PluginNotFound,
                "Plugin not installed",
            ));
        }
        self.config.save_plugins(&plugins)?;
        let mut settings = self.config.load_settings()?;
        settings.enabled_plugins.insert(plugin_name.to_string(), enabled);
        self.config.save_settings(&settings)?;
        Ok(())
    }
}

fn normalize_mcp_env(
    mut config: McpServersFile,
    provided: &HashMap<String, String>,
    skip_env: bool,
) -> MarketResult<McpServersFile> {
    let mut required = HashSet::new();
    for server in config.mcp_servers.values() {
        for (key, value) in &server.env {
            if is_placeholder(value) {
                required.insert(key.clone());
            }
        }
    }

    let mut resolved = HashMap::new();
    for name in required {
        if let Some(value) = provided.get(&name) {
            resolved.insert(name.clone(), value.clone());
            continue;
        }
        if let Ok(existing) = std::env::var(&name) {
            if skip_env {
                resolved.insert(name.clone(), existing);
                continue;
            }
            let use_existing = Confirm::new()
                .with_prompt(format!("Found {} in environment. Use it?", name))
                .default(true)
                .interact()
                .map_err(|err| {
                    MarketError::with_source(
                        MarketErrorCode::InvalidEnvironment,
                        "Failed to confirm environment value",
                        err.into(),
                    )
                })?;
            if use_existing {
                resolved.insert(name.clone(), existing);
                continue;
            }
        }
        if skip_env {
            continue;
        }
        let input: String = Input::new()
            .with_prompt(format!("Enter {}", name))
            .validate_with(|value: &String| {
                if value.trim().is_empty() {
                    Err("Value cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact()
            .map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::InvalidEnvironment,
                    "Failed to read environment variable",
                    err.into(),
                )
            })?;
        resolved.insert(name.clone(), input);
    }

    for server in config.mcp_servers.values_mut() {
        for (key, value) in server.env.iter_mut() {
            if is_placeholder(value) {
                if let Some(resolved_value) = resolved.get(key) {
                    std::env::set_var(key, resolved_value);
                }
                *value = format!("${{{}}}", key);
            }
        }
    }

    Ok(config)
}

fn is_placeholder(value: &str) -> bool {
    if value.trim().is_empty() {
        return true;
    }
    if value.starts_with("${") && value.ends_with('}') {
        return true;
    }
    value.starts_with('$')
}

fn merge_mcp_config(store: &ConfigStore, new_config: &mut McpServersFile) -> MarketResult<()> {
    let mut mcp_config = store.load_mcp()?;
    for (name, server) in new_config.mcp_servers.drain() {
        if mcp_config.mcp_servers.contains_key(&name) {
            eprintln!("Warning: MCP server '{}' already exists, overwriting.", name);
        }
        mcp_config.mcp_servers.insert(name, server);
    }
    store.save_mcp(&mcp_config)?;
    Ok(())
}

fn remove_mcp_entries(store: &ConfigStore, plugin_cache: &Path) -> MarketResult<()> {
    let manifest_path = plugin_cache.join(".claude-plugin").join("plugin.json");
    if !manifest_path.exists() {
        return Ok(());
    }
    let manifest = load_manifest(&manifest_path)?;
    let mcp_config = extract_mcp_config(&manifest, plugin_cache)?;
    let Some(mcp_config) = mcp_config else {
        return Ok(());
    };
    let mut config = store.load_mcp()?;
    for key in mcp_config.mcp_servers.keys() {
        config.mcp_servers.remove(key);
    }
    store.save_mcp(&config)?;
    Ok(())
}
