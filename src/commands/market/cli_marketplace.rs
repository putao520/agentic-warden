//! Marketplace management commands.

use crate::commands::market::cache::MarketCacheManager;
use crate::commands::market::cli_utils::{build_source, parse_marketplace_source, source_display};
use crate::commands::market::config::ConfigStore;
use crate::commands::market::filter::McpFilter;
use crate::commands::market::plugin::PluginMetadata;
use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult, MarketplaceSettingsEntry};
use crate::commands::parser::MarketplaceAction;

pub async fn handle_marketplace_action(action: MarketplaceAction) -> MarketResult<()> {
    match action {
        MarketplaceAction::Add { repo_url, name } => marketplace_add(repo_url, name).await,
        MarketplaceAction::List => marketplace_list().await,
        MarketplaceAction::Remove { name } => marketplace_remove(name).await,
        MarketplaceAction::Update { name } => marketplace_update(name).await,
    }
}

async fn marketplace_add(repo_url: String, name: Option<String>) -> MarketResult<()> {
    let store = ConfigStore::new()?;
    let mut settings = store.load_settings()?;

    let (source, derived_name) = parse_marketplace_source(&repo_url)?;
    let market_name = name.unwrap_or(derived_name);
    if settings.extra_known_marketplaces.contains_key(&market_name) {
        return Err(MarketError::new(
            MarketErrorCode::MarketplaceExists,
            "Marketplace already exists",
        ));
    }
    settings.extra_known_marketplaces.insert(
        market_name.clone(),
        MarketplaceSettingsEntry {
            source,
            enabled: true,
        },
    );
    store.save_settings(&settings)?;

    let source = build_source(&market_name, settings.extra_known_marketplaces.get(&market_name).unwrap())?;
    let marketplace = source.fetch_marketplace().await?;
    println!("✓ Added marketplace: {}", market_name);
    println!("  Source: {}", repo_url);
    println!("  Cache: {}", source.cache_manager().marketplace_cache_path(&market_name).display());
    println!("  Plugins: {} found", marketplace.plugins.len());
    Ok(())
}

async fn marketplace_list() -> MarketResult<()> {
    let store = ConfigStore::new()?;
    let settings = store.load_settings()?;
    let cache = MarketCacheManager::new()?;
    println!("Plugin Marketplaces:");

    for (name, entry) in settings.extra_known_marketplaces.iter() {
        let status = if entry.enabled { "✓" } else { "⊘" };
        let source_desc = source_display(&entry.source);
        let updated = cache
            .read_last_update(name)
            .map(|ts| ts.to_rfc3339())
            .unwrap_or_else(|| "never".to_string());
        let plugin_count = match build_source(name, entry) {
            Ok(source) => {
                if let Ok(plugins) = crate::commands::market::cli_utils::fetch_plugin_metadata(&source).await {
                    let mcp_count = PluginMetadata::filter_mcp_plugins(plugins.clone()).len();
                    format!("{} plugins ({} MCP-compatible)", plugins.len(), mcp_count)
                } else {
                    "unknown".to_string()
                }
            }
            Err(_) => "unknown".to_string(),
        };
        println!("  {} {} ({})", status, name, source_desc);
        println!("    - {}", plugin_count);
        println!("    - Updated: {}", updated);
    }
    Ok(())
}

async fn marketplace_remove(name: String) -> MarketResult<()> {
    let store = ConfigStore::new()?;
    let mut settings = store.load_settings()?;
    if settings.extra_known_marketplaces.remove(&name).is_none() {
        return Err(MarketError::new(
            MarketErrorCode::MarketplaceNotFound,
            "Marketplace not found",
        ));
    }
    store.save_settings(&settings)?;
    let cache = MarketCacheManager::new()?;
    let cache_path = cache.marketplace_cache_path(&name);
    if cache_path.exists() {
        let _ = std::fs::remove_dir_all(&cache_path);
    }
    println!("✓ Removed marketplace: {}", name);
    println!("  Cache cleared: {}", cache_path.display());
    Ok(())
}

async fn marketplace_update(name: Option<String>) -> MarketResult<()> {
    let store = ConfigStore::new()?;
    let settings = store.load_settings()?;
    let mut sources = Vec::new();
    for (market_name, entry) in settings.extra_known_marketplaces.iter() {
        if let Some(target) = &name {
            if target != market_name {
                continue;
            }
        }
        if entry.enabled {
            sources.push((market_name.clone(), build_source(market_name, entry)?));
        }
    }
    if sources.is_empty() {
        return Err(MarketError::new(
            MarketErrorCode::MarketplaceNotFound,
            "Marketplace not found",
        ));
    }

    println!("🔄 Updating marketplace caches...");
    for (name, source) in sources {
        source.update().await?;
        let marketplace = source.fetch_marketplace().await?;
        println!("  ✓ {}: {} plugins", name, marketplace.plugins.len());
    }
    Ok(())
}
