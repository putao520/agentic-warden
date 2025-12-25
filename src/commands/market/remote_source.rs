//! Remote URL marketplace source implementation.

use crate::commands::market::cache::MarketCacheManager;
use crate::commands::market::plugin::{MarketplaceConfig, MarketplacePluginEntry, PluginManifest};
use crate::commands::market::plugin_io::{load_manifest, resolve_path_placeholder};
use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult, MarketSource};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use url::Url;

#[derive(Clone)]
pub struct RemoteSource {
    name: String,
    marketplace_url: Url,
    cache: MarketCacheManager,
    client: Client,
}

impl RemoteSource {
    pub fn new(name: String, url: String, cache: MarketCacheManager) -> MarketResult<Self> {
        let marketplace_url = Url::parse(&url).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Invalid marketplace URL",
                err.into(),
            )
        })?;
        Ok(Self {
            name,
            marketplace_url,
            cache,
            client: Client::new(),
        })
    }

    fn base_url(&self) -> MarketResult<Url> {
        let mut base = self.marketplace_url.clone();
        if base.path().ends_with("marketplace.json") {
            base.path_segments_mut()
                .map_err(|_| MarketError::new(MarketErrorCode::MarketplaceUnreachable, "Invalid marketplace URL"))?
                .pop();
        }
        Ok(base)
    }

    async fn download_marketplace(&self) -> MarketResult<MarketplaceConfig> {
        let resp = self
            .client
            .get(self.marketplace_url.clone())
            .send()
            .await
            .map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::MarketplaceUnreachable,
                    "Failed to download marketplace.json",
                    err.into(),
                )
            })?;
        let text = resp.text().await.map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Failed to read marketplace.json",
                err.into(),
            )
        })?;
        let config: MarketplaceConfig = serde_json::from_str(&text).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceFormat,
                "Invalid marketplace.json format",
                err.into(),
            )
        })?;
        let cache_path = self.cache.ensure_marketplace_cache(&self.name)?;
        let cache_file = cache_path.join("marketplace.json");
        std::fs::write(&cache_file, &text).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to cache marketplace.json",
                err.into(),
            )
        })?;
        self.cache.write_last_update(&self.name, Utc::now())?;
        Ok(config)
    }

    fn plugin_base_url(&self, config: &MarketplaceConfig) -> MarketResult<Url> {
        let base = self.base_url()?;
        let plugin_root = config
            .metadata
            .as_ref()
            .and_then(|meta| meta.plugin_root.clone())
            .unwrap_or_else(|| "./plugins".to_string());
        let plugin_root_path = plugin_root.trim_start_matches("./");
        base.join(plugin_root_path)
            .map_err(|err| MarketError::with_source(MarketErrorCode::MarketplaceUnreachable, "Invalid plugin root", err.into()))
    }

    fn resolve_plugin_url(
        &self,
        config: &MarketplaceConfig,
        entry: &MarketplacePluginEntry,
    ) -> MarketResult<Url> {
        let base = self.base_url()?;
        let plugin_base = self.plugin_base_url(config)?;
        let path = match &entry.source {
            crate::commands::market::plugin::PluginSource::Path(p) => p.clone(),
            crate::commands::market::plugin::PluginSource::Object(obj) => {
                if let Some(url) = &obj.url {
                    return Url::parse(url).map_err(|err| {
                        MarketError::with_source(
                            MarketErrorCode::MarketplaceUnreachable,
                            "Invalid plugin URL",
                            err.into(),
                        )
                    });
                }
                obj.path.clone().unwrap_or_else(|| entry.name.clone())
            }
        };
        let resolved = resolve_path_placeholder(PathBuf::from("/").as_path(), &path);
        let rel = resolved.to_string_lossy().trim_start_matches('/').to_string();
        let plugin_root = config
            .metadata
            .as_ref()
            .and_then(|meta| meta.plugin_root.clone())
            .unwrap_or_else(|| "./plugins".to_string());
        let root_name = plugin_root.trim_start_matches("./").split('/').next().unwrap_or("");
        let base_url = if root_name.is_empty() {
            plugin_base.clone()
        } else if rel.starts_with(root_name) {
            base.clone()
        } else {
            plugin_base.clone()
        };
        let rel_with_slash = format!("{}/", rel);
        base_url.join(&rel_with_slash).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Invalid plugin URL",
                err.into(),
            )
        })
    }

    async fn download_file(&self, url: Url, dest: PathBuf) -> MarketResult<()> {
        let resp = self.client.get(url).send().await.map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Failed to download plugin file",
                err.into(),
            )
        })?;
        let bytes = resp.bytes().await.map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Failed to read plugin file",
                err.into(),
            )
        })?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await.map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Failed to create plugin cache",
                    err.into(),
                )
            })?;
        }
        let mut file = fs::File::create(&dest).await.map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to write plugin cache",
                err.into(),
            )
        })?;
        file.write_all(&bytes).await.map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to write plugin cache",
                err.into(),
            )
        })?;
        Ok(())
    }
}

#[async_trait]
impl MarketSource for RemoteSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn cache_manager(&self) -> &MarketCacheManager {
        &self.cache
    }

    async fn fetch_marketplace(&self) -> MarketResult<MarketplaceConfig> {
        self.download_marketplace().await
    }

    async fn fetch_plugin(&self, entry: &MarketplacePluginEntry) -> MarketResult<PluginManifest> {
        let config = self.fetch_marketplace().await?;
        let plugin_url = self.resolve_plugin_url(&config, entry)?;
        let manifest_url = plugin_url.join(".claude-plugin/plugin.json").map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Invalid plugin manifest URL",
                err.into(),
            )
        })?;
        let resp = self.client.get(manifest_url).send().await.map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Failed to download plugin.json",
                err.into(),
            )
        })?;
        let text = resp.text().await.map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Failed to read plugin.json",
                err.into(),
            )
        })?;
        let cache_path = self.cache.ensure_marketplace_cache(&self.name)?;
        let manifest_cache = cache_path.join(format!("plugin-{}.json", entry.name));
        std::fs::write(&manifest_cache, &text).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to cache plugin.json",
                err.into(),
            )
        })?;
        load_manifest(&manifest_cache)
    }

    async fn download_plugin(&self, entry: &MarketplacePluginEntry, plugin_id: &str) -> MarketResult<PathBuf> {
        let config = self.fetch_marketplace().await?;
        let plugin_url = self.resolve_plugin_url(&config, entry)?;
        let plugin_cache = self.cache.ensure_plugin_cache(plugin_id, &self.name)?;
        let manifest_dest = plugin_cache.join(".claude-plugin").join("plugin.json");
        let manifest_url = plugin_url.join(".claude-plugin/plugin.json").map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Invalid plugin manifest URL",
                err.into(),
            )
        })?;
        self.download_file(manifest_url, manifest_dest.clone()).await?;
        let manifest = load_manifest(&manifest_dest)?;
        if let Some(value) = manifest.mcp_servers {
            if let Some(path) = value.as_str() {
                let mcp_url = plugin_url.join(path).map_err(|err| {
                    MarketError::with_source(
                        MarketErrorCode::MarketplaceUnreachable,
                        "Invalid MCP config URL",
                        err.into(),
                    )
                })?;
                let dest = plugin_cache.join(path);
                self.download_file(mcp_url, dest).await?;
            }
        }
        Ok(plugin_cache)
    }

    async fn update(&self) -> MarketResult<()> {
        let _ = self.fetch_marketplace().await?;
        Ok(())
    }
}
