//! Shared helpers for plugin marketplace CLI.

use crate::commands::market::cache::MarketCacheManager;
use crate::commands::market::config::ConfigStore;
use crate::commands::market::plugin::{MarketplacePluginEntry, PluginManifest, PluginMetadata};
use crate::commands::market::plugin_io::{build_metadata, extract_mcp_config};
use crate::commands::market::source::{
    MarketError, MarketErrorCode, MarketResult, MarketSource, MarketplaceSourceConfig,
    MarketplaceSettingsEntry,
};
use crate::commands::market::{github_source::GithubSource, local_source::LocalSource, remote_source::RemoteSource};
use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

pub struct PluginFetchResult {
    pub entry: MarketplacePluginEntry,
    pub manifest: PluginManifest,
    pub root: PathBuf,
}

pub fn parse_marketplace_source(input: &str) -> MarketResult<(MarketplaceSourceConfig, String)> {
    let path = PathBuf::from(input);
    if path.exists() {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("local-marketplace")
            .to_string();
        return Ok((
            MarketplaceSourceConfig::Local {
                path: path.to_string_lossy().to_string(),
            },
            name,
        ));
    }

    if let Ok(url) = Url::parse(input) {
        if url.host_str() == Some("github.com") {
            let repo = url
                .path()
                .trim_start_matches('/')
                .trim_end_matches(".git")
                .to_string();
            let name = repo.split('/').last().unwrap_or("market").to_string();
            return Ok((MarketplaceSourceConfig::Github { repo }, name));
        }
        let name = url
            .path_segments()
            .and_then(|s| s.last())
            .unwrap_or("remote-marketplace")
            .trim_end_matches(".json")
            .to_string();
        return Ok((
            MarketplaceSourceConfig::Remote {
                url: input.to_string(),
            },
            name,
        ));
    }

    if input.split('/').count() == 2 {
        let name = input.split('/').last().unwrap_or("market").to_string();
        return Ok((
            MarketplaceSourceConfig::Github {
                repo: input.to_string(),
            },
            name,
        ));
    }

    Err(MarketError::new(
        MarketErrorCode::MarketplaceUnreachable,
        "Invalid marketplace source",
    ))
}

pub fn build_source(
    name: &str,
    entry: &MarketplaceSettingsEntry,
) -> MarketResult<Box<dyn MarketSource>> {
    let cache = MarketCacheManager::new()?;
    match &entry.source {
        MarketplaceSourceConfig::Github { repo } => Ok(Box::new(GithubSource::new(
            name.to_string(),
            repo.clone(),
            cache,
        ))),
        MarketplaceSourceConfig::Local { path } => Ok(Box::new(LocalSource::new(
            name.to_string(),
            PathBuf::from(path),
            cache,
        )?)),
        MarketplaceSourceConfig::Remote { url } => Ok(Box::new(RemoteSource::new(
            name.to_string(),
            url.clone(),
            cache,
        )?)),
    }
}

pub async fn load_sources() -> MarketResult<HashMap<String, Box<dyn MarketSource>>> {
    let store = ConfigStore::new()?;
    let settings = store.load_settings()?;
    let mut map = HashMap::new();
    for (name, entry) in settings.extra_known_marketplaces.iter() {
        if !entry.enabled {
            continue;
        }
        let source = build_source(name, entry)?;
        map.insert(name.clone(), source);
    }
    Ok(map)
}

pub async fn fetch_plugin_metadata(source: &Box<dyn MarketSource>) -> MarketResult<Vec<PluginMetadata>> {
    let marketplace = source.fetch_marketplace().await?;
    let mut metadata = Vec::new();
    for entry in marketplace.plugins.iter() {
        let manifest = match source.fetch_plugin(entry).await {
            Ok(manifest) => manifest,
            Err(err) => {
                eprintln!("Warning: failed to read plugin {}: {}", entry.name, err);
                continue;
            }
        };
        let mcp_config = if manifest.mcp_servers.as_ref().map(|v| v.is_object()).unwrap_or(false) {
            extract_mcp_config(&manifest, PathBuf::from(".").as_path()).ok().flatten()
        } else {
            None
        };
        metadata.push(build_metadata(entry, &manifest, source.name(), mcp_config));
    }
    Ok(metadata)
}

pub async fn fetch_plugin_detail(
    source: &Box<dyn MarketSource>,
    plugin_name: &str,
) -> MarketResult<PluginFetchResult> {
    let marketplace = source.fetch_marketplace().await?;
    let entry = marketplace
        .plugins
        .iter()
        .find(|entry| entry.name == plugin_name)
        .cloned()
        .ok_or_else(|| MarketError::new(MarketErrorCode::PluginNotFound, "Plugin not found"))?;
    let manifest = source.fetch_plugin(&entry).await?;
    let cache_path = source.download_plugin(&entry, plugin_name).await?;
    Ok(PluginFetchResult {
        entry,
        manifest,
        root: cache_path,
    })
}

pub async fn fetch_plugin_entry(
    source: &Box<dyn MarketSource>,
    plugin_name: &str,
) -> MarketResult<(MarketplacePluginEntry, PluginManifest)> {
    let marketplace = source.fetch_marketplace().await?;
    let entry = marketplace
        .plugins
        .iter()
        .find(|entry| entry.name == plugin_name)
        .cloned()
        .ok_or_else(|| MarketError::new(MarketErrorCode::PluginNotFound, "Plugin not found"))?;
    let manifest = source.fetch_plugin(&entry).await?;
    Ok((entry, manifest))
}

pub fn parse_plugin_reference(value: &str) -> MarketResult<(String, String)> {
    let mut parts = value.split('@');
    let name = parts.next().unwrap_or_default();
    let market = parts.next();
    if name.is_empty() || market.is_none() {
        return Err(MarketError::new(
            MarketErrorCode::PluginNotFound,
            "Plugin reference must be <plugin>@<market>",
        ));
    }
    Ok((name.to_string(), market.unwrap().to_string()))
}

pub fn split_plugin_key(key: &str) -> (&str, &str) {
    let mut parts = key.split('@');
    let name = parts.next().unwrap_or(key);
    let market = parts.next().unwrap_or("unknown");
    (name, market)
}

pub fn source_display(source: &MarketplaceSourceConfig) -> String {
    match source {
        MarketplaceSourceConfig::Github { repo } => repo.clone(),
        MarketplaceSourceConfig::Local { path } => path.clone(),
        MarketplaceSourceConfig::Remote { url } => url.clone(),
    }
}
