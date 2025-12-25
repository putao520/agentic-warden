//! Plugin commands.

use crate::commands::market::cache::MarketCacheManager;
use crate::commands::market::cli_utils::{
    fetch_plugin_detail, fetch_plugin_entry, fetch_plugin_metadata, load_sources, parse_plugin_reference,
    split_plugin_key,
};
use crate::commands::market::config_utils::{parse_env_pairs, parse_installed_at};
use crate::commands::market::filter::McpFilter;
use crate::commands::market::installer::PluginInstaller;
use crate::commands::market::plugin::{PluginDetail, PluginMetadata};
use crate::commands::market::plugin_io::{build_plugin_detail, extract_mcp_config, load_manifest};
use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult};
use dialoguer::FuzzySelect;

pub async fn browse_plugins(
    market: Option<String>,
    category: Option<String>,
    tags: Option<String>,
) -> MarketResult<()> {
    let sources = load_sources().await?;
    let mut plugins = Vec::new();
    for (name, source) in sources.iter() {
        if let Some(filter) = &market {
            if filter != name {
                continue;
            }
        }
        plugins.extend(fetch_plugin_metadata(source).await?);
    }

    let mut filtered = PluginMetadata::filter_mcp_plugins(plugins);
    if let Some(category) = category {
        filtered.retain(|plugin| plugin.category.as_deref() == Some(category.as_str()));
    }
    if let Some(tags) = tags {
        let tag_set: Vec<String> = tags.split(',').map(|t| t.trim().to_string()).collect();
        filtered.retain(|plugin| tag_set.iter().all(|t| plugin.tags.contains(t)));
    }

    if filtered.is_empty() {
        println!("No MCP plugins found.");
        return Ok(());
    }

    let items: Vec<String> = filtered
        .iter()
        .map(|plugin| format!("{}/{} - {}", plugin.marketplace, plugin.name, plugin.description))
        .collect();
    let selection = FuzzySelect::new()
        .with_prompt("Select a plugin")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::InvalidEnvironment,
                "Failed to open selection prompt",
                err.into(),
            )
        })?;
    let chosen = &filtered[selection];
    print_plugin_detail(chosen).await
}

pub async fn search_plugins(query: String, market: Option<String>) -> MarketResult<()> {
    let sources = load_sources().await?;
    let mut plugins = Vec::new();
    for (name, source) in sources.iter() {
        if let Some(filter) = &market {
            if filter != name {
                continue;
            }
        }
        plugins.extend(fetch_plugin_metadata(source).await?);
    }
    let query_lower = query.to_lowercase();
    let results: Vec<PluginMetadata> = PluginMetadata::filter_mcp_plugins(plugins)
        .into_iter()
        .filter(|plugin| {
            plugin.name.to_lowercase().contains(&query_lower)
                || plugin.description.to_lowercase().contains(&query_lower)
                || plugin
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&query_lower))
        })
        .collect();

    println!("Searching for \"{}\"... ({} results found)", query, results.len());
    for plugin in results {
        println!("\n{}/{}", plugin.marketplace, plugin.name);
        println!("  Description: {}", plugin.description);
        if !plugin.tags.is_empty() {
            println!("  Tags: {}", plugin.tags.join(", "));
        }
    }
    Ok(())
}

pub async fn plugin_info(plugin: String) -> MarketResult<()> {
    let (name, marketplace) = parse_plugin_reference(&plugin)?;
    let sources = load_sources().await?;
    let source = sources.get(&marketplace).ok_or_else(|| {
        MarketError::new(
            MarketErrorCode::MarketplaceNotFound,
            "Marketplace not found",
        )
    })?;
    let result = fetch_plugin_detail(source, &name).await?;
    let detail = build_plugin_detail(result.entry, result.manifest, &result.root)?;
    print_full_detail(&detail, &marketplace)
}

pub async fn plugin_install(plugin: String, env_vars: Vec<String>, skip_env: bool) -> MarketResult<()> {
    let (name, marketplace) = parse_plugin_reference(&plugin)?;
    let sources = load_sources().await?;
    let source = sources.get(&marketplace).ok_or_else(|| {
        MarketError::new(
            MarketErrorCode::MarketplaceNotFound,
            "Marketplace not found",
        )
    })?;
    let (entry, manifest) = fetch_plugin_entry(source, &name).await?;
    let detail = PluginDetail {
        entry,
        manifest,
        mcp_config: None,
    };
    let installer = PluginInstaller::new()?;
    let env_vars = parse_env_pairs(&env_vars)?;
    installer.install(source.as_ref(), &detail, env_vars, skip_env).await?;
    println!("✓ Plugin installed: {}", plugin);
    println!("  MCP config: {}", installer.config.mcp_path().display());
    Ok(())
}

pub async fn list_installed(show_disabled: bool) -> MarketResult<()> {
    let installer = PluginInstaller::new()?;
    let plugins = installer.list_installed()?;
    println!("Installed Plugins ({}):", plugins.plugins.len());
    for (key, record) in plugins.plugins.iter() {
        if !show_disabled && !record.enabled {
            continue;
        }
        let status = if record.enabled { "✓" } else { "⊘" };
        let installed_at = parse_installed_at(&record.installed_at)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| record.installed_at.clone());
        let (name, marketplace) = split_plugin_key(key);
        let servers = read_plugin_servers(&installer.cache, name, marketplace)
            .unwrap_or_default();
        println!("\n{} {} ({})", status, name, marketplace);
        println!("  Status: {}", if record.enabled { "enabled" } else { "disabled" });
        if !servers.is_empty() {
            println!("  MCP Servers: {}", servers.join(", "));
        }
        println!("  Installed: {}", installed_at);
    }
    Ok(())
}

pub async fn remove_plugin(plugin: String) -> MarketResult<()> {
    let installer = PluginInstaller::new()?;
    let (plugin_name, _market) = split_plugin_key(&plugin);
    let removed = installer.remove_plugin(plugin_name)?;
    println!("Removing {}...", plugin);
    for entry in removed {
        println!("✓ Removed {}", entry);
    }
    Ok(())
}

pub async fn set_plugin_enabled(plugin: String, enabled: bool) -> MarketResult<()> {
    let installer = PluginInstaller::new()?;
    let (plugin_name, _market) = split_plugin_key(&plugin);
    installer.set_enabled(plugin_name, enabled)?;
    if enabled {
        println!("✓ Enabled {}", plugin);
    } else {
        println!("✓ Disabled {}", plugin);
    }
    Ok(())
}

async fn print_plugin_detail(plugin: &PluginMetadata) -> MarketResult<()> {
    let plugin_ref = format!("{}@{}", plugin.name, plugin.marketplace);
    plugin_info(plugin_ref).await
}

fn print_full_detail(detail: &PluginDetail, marketplace: &str) -> MarketResult<()> {
    println!("Plugin: {}@{}", detail.manifest.name, marketplace);
    println!("Version: {}", detail.manifest.version);
    println!("Author: {}", detail.manifest.author.name);
    if let Some(license) = &detail.manifest.license {
        println!("License: {}", license);
    }
    if let Some(homepage) = &detail.manifest.homepage {
        println!("Homepage: {}", homepage);
    }
    println!("\nDescription: {}", detail.manifest.description);
    if let Some(config) = &detail.mcp_config {
        println!("\nMCP Servers:");
        for (name, server) in config.mcp_servers.iter() {
            println!("  {}:", name);
            if let Some(command) = server.get_command() {
                println!("    Command: {}", command);
                if let Some(args) = server.get_args() {
                    println!("    Args: {}", args.join(", "));
                }
                if let Some(env) = server.get_env() {
                    if !env.is_empty() {
                        println!("    Environment Variables:");
                        for key in env.keys() {
                            println!("      - {}", key);
                        }
                    }
                }
            } else {
                println!("    Type: HTTP/SSE (not yet supported by AIW)");
            }
        }
    } else {
        println!("\nMCP Servers: none");
    }
    Ok(())
}

fn read_plugin_servers(
    cache: &MarketCacheManager,
    plugin_name: &str,
    marketplace: &str,
) -> MarketResult<Vec<String>> {
    let cache_path = cache.plugin_cache_path(plugin_name, marketplace);
    let manifest_path = cache_path.join(".claude-plugin").join("plugin.json");
    if !manifest_path.exists() {
        return Ok(Vec::new());
    }
    let manifest = load_manifest(&manifest_path)?;
    let mcp_config = extract_mcp_config(&manifest, &cache_path)?;
    Ok(mcp_config
        .map(|cfg| cfg.mcp_servers.keys().cloned().collect())
        .unwrap_or_default())
}
