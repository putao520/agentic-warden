//! Marketplace file IO and MCP extraction helpers.

use crate::commands::market::plugin::{
    McpServerInfo, McpServersFile, MarketplaceConfig, MarketplacePluginEntry, PluginDetail,
    PluginManifest, PluginMetadata, PluginSource,
};
use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult};
use crate::commands::market::validator::validate_manifest;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn load_marketplace(path: &Path) -> MarketResult<MarketplaceConfig> {
    let contents = std::fs::read_to_string(path).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::MarketplaceFormat,
            format!("Failed to read marketplace.json: {}", path.display()),
            err.into(),
        )
    })?;
    let config: MarketplaceConfig = serde_json::from_str(&contents).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::MarketplaceFormat,
            "Invalid marketplace.json format",
            err.into(),
        )
    })?;
    Ok(config)
}

pub fn load_manifest(path: &Path) -> MarketResult<PluginManifest> {
    let contents = std::fs::read_to_string(path).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::PluginNotFound,
            format!("Failed to read plugin.json: {}", path.display()),
            err.into(),
        )
    })?;
    let manifest: PluginManifest = serde_json::from_str(&contents).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::McpExtractionFailed,
            "Invalid plugin.json format",
            err.into(),
        )
    })?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn marketplace_plugin_root(config: &MarketplaceConfig, marketplace_root: &Path) -> PathBuf {
    let plugin_root = config
        .metadata
        .as_ref()
        .and_then(|meta| meta.plugin_root.clone())
        .unwrap_or_else(|| "./plugins".to_string());
    resolve_path_placeholder(marketplace_root, &plugin_root)
}

pub fn resolve_path_placeholder(base: &Path, raw: &str) -> PathBuf {
    if raw.contains("${CLAUDE_PLUGIN_ROOT}") {
        let replaced = raw.replace("${CLAUDE_PLUGIN_ROOT}", base.to_string_lossy().as_ref());
        PathBuf::from(replaced)
    } else {
        let candidate = PathBuf::from(raw);
        if candidate.is_absolute() {
            candidate
        } else {
            base.join(candidate)
        }
    }
}

pub fn resolve_plugin_source(
    entry: &MarketplacePluginEntry,
    plugin_root: &Path,
) -> PluginSourceLocation {
    match &entry.source {
        PluginSource::Path(path) => {
            let base = resolve_plugin_base(plugin_root, path);
            PluginSourceLocation::Local(resolve_path_placeholder(&base, path))
        }
        PluginSource::Object(obj) => {
            if let Some(repo) = &obj.repo {
                PluginSourceLocation::GitHub {
                    repo: repo.clone(),
                    path: obj.path.clone(),
                    reference: obj.reference.clone(),
                }
            } else if let Some(url) = &obj.url {
                PluginSourceLocation::Remote {
                    url: url.clone(),
                    path: obj.path.clone(),
                }
            } else if let Some(path) = &obj.path {
                let base = resolve_plugin_base(plugin_root, path);
                PluginSourceLocation::Local(resolve_path_placeholder(&base, path))
            } else {
                PluginSourceLocation::Local(plugin_root.join(&entry.name))
            }
        }
    }
}

pub enum PluginSourceLocation {
    Local(PathBuf),
    GitHub {
        repo: String,
        path: Option<String>,
        reference: Option<String>,
    },
    Remote {
        url: String,
        path: Option<String>,
    },
}

fn resolve_plugin_base(plugin_root: &Path, path: &str) -> PathBuf {
    let normalized = path.trim_start_matches("./");
    let first = normalized.split('/').next().unwrap_or_default();

    // Check if path starts with known top-level directories
    // These should be resolved relative to marketplace root
    if first == "external_plugins" || first == "plugins" {
        if let Some(parent) = plugin_root.parent() {
            // Go up to marketplace root (parent of ./plugins or ./external_plugins)
            if parent.file_name().and_then(|s| s.to_str()) == Some("plugins") {
                // If we're at ./plugins, parent is marketplace root
                if let Some(grandparent) = parent.parent() {
                    return grandparent.to_path_buf();
                }
            }
            return parent.to_path_buf();
        }
    }

    // Original logic for checking if path is under plugin_root
    if let Some(root_name) = plugin_root.file_name().and_then(|s| s.to_str()) {
        if first == root_name {
            if let Some(parent) = plugin_root.parent() {
                return parent.to_path_buf();
            }
        }
    }
    plugin_root.to_path_buf()
}

pub fn extract_mcp_config(
    manifest: &PluginManifest,
    plugin_root: &Path,
) -> MarketResult<Option<McpServersFile>> {
    // First check manifest.mcp_servers field if present
    if let Some(value) = &manifest.mcp_servers {
        if value.is_string() {
            let path_value = value.as_str().unwrap_or_default();
            let resolved = resolve_path_placeholder(plugin_root, path_value);
            return load_mcp_config(&resolved).map(Some);
        }

        if value.is_object() {
            if value.get("mcpServers").is_some() {
                let file: McpServersFile = serde_json::from_value(value.clone()).map_err(|err| {
                    MarketError::with_source(
                        MarketErrorCode::McpExtractionFailed,
                        "Invalid mcpServers object",
                        err.into(),
                    )
                })?;
                return Ok(Some(file));
            }

            let map: HashMap<String, crate::commands::market::plugin::McpServerConfig> =
                serde_json::from_value(value.clone()).map_err(|err| {
                    MarketError::with_source(
                        MarketErrorCode::McpExtractionFailed,
                        "Invalid mcpServers map",
                        err.into(),
                    )
                })?;
            return Ok(Some(McpServersFile { mcp_servers: map }));
        }

        return Err(MarketError::new(
            MarketErrorCode::McpExtractionFailed,
            "Unsupported mcpServers format",
        ));
    }

    // Check for .mcp.json file in plugin directory
    let mcp_json_path = plugin_root.join(".mcp.json");
    if mcp_json_path.exists() {
        return load_mcp_config(&mcp_json_path).map(Some);
    }

    Ok(None)
}

pub fn load_mcp_config(path: &Path) -> MarketResult<McpServersFile> {
    let contents = std::fs::read_to_string(path).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::McpExtractionFailed,
            format!("Failed to read MCP config: {}", path.display()),
            err.into(),
        )
    })?;

    // Try parsing with mcpServers wrapper first
    if let Ok(config) = serde_json::from_str::<McpServersFile>(&contents) {
        return Ok(config);
    }

    // Try parsing as plain HashMap (without mcpServers wrapper)
    use crate::commands::market::plugin::McpServerConfig;
    let server_map: HashMap<String, McpServerConfig> =
        serde_json::from_str(&contents).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::McpExtractionFailed,
                "Invalid MCP config format (expected stdio command/args or http/sse type/url)",
                err.into(),
            )
        })?;

    // Filter to only stdio transports (supported by AIW)
    let stdio_map: HashMap<String, McpServerConfig> = server_map
        .into_iter()
        .filter(|(_, config)| config.is_stdio())
        .collect();

    if stdio_map.is_empty() {
        return Err(MarketError::new(
            MarketErrorCode::McpExtractionFailed,
            "No stdio-based MCP servers found (only http/sse transports which are not yet supported)",
        ));
    }

    Ok(McpServersFile { mcp_servers: stdio_map })
}

pub fn build_metadata(
    entry: &MarketplacePluginEntry,
    manifest: &PluginManifest,
    marketplace: &str,
    mcp_config: Option<McpServersFile>,
) -> PluginMetadata {
    let mut mcp_servers = Vec::new();
    if let Some(config) = mcp_config {
        for (name, cfg) in config.mcp_servers {
            mcp_servers.push(McpServerInfo { name, config: cfg });
        }
    }
    let has_mcp_servers = manifest.mcp_servers.is_some();
    PluginMetadata {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        description: manifest.description.clone(),
        author: manifest.author.clone(),
        marketplace: marketplace.to_string(),
        source: entry.source.clone(),
        has_mcp_servers,
        mcp_servers,
        category: entry.category.clone(),
        tags: entry.tags.clone().unwrap_or_default(),
    }
}

pub fn build_plugin_detail(
    entry: MarketplacePluginEntry,
    manifest: PluginManifest,
    plugin_root: &Path,
) -> MarketResult<PluginDetail> {
    let mcp_config = extract_mcp_config(&manifest, plugin_root)?;
    Ok(PluginDetail {
        entry,
        manifest,
        mcp_config,
    })
}

pub fn inline_mcp_config(value: &Value) -> Option<McpServersFile> {
    if !value.is_object() {
        return None;
    }
    if value.get("mcpServers").is_some() {
        return serde_json::from_value(value.clone()).ok();
    }
    let map: HashMap<String, crate::commands::market::plugin::McpServerConfig> =
        serde_json::from_value(value.clone()).ok()?;
    Some(McpServersFile { mcp_servers: map })
}
