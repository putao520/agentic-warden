//! Local path marketplace source implementation.

use crate::commands::market::cache::{copy_dir_recursive, MarketCacheManager};
use crate::commands::market::plugin::PluginManifest;
use crate::commands::market::plugin_io::{
    load_manifest, load_marketplace, marketplace_plugin_root, resolve_plugin_source,
    PluginSourceLocation,
};
use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult, MarketSource};
use async_trait::async_trait;
use chrono::Utc;
use std::path::{Path, PathBuf};
use tokio::task;

#[derive(Clone)]
pub struct LocalSource {
    name: String,
    marketplace_path: PathBuf,
    marketplace_root: PathBuf,
    cache: MarketCacheManager,
}

impl LocalSource {
    pub fn new(name: String, path: PathBuf, cache: MarketCacheManager) -> MarketResult<Self> {
        let (marketplace_path, marketplace_root) = resolve_marketplace_paths(&path)?;
        Ok(Self {
            name,
            marketplace_path,
            marketplace_root,
            cache,
        })
    }

    fn update_timestamp(&self) -> MarketResult<()> {
        self.cache.write_last_update(&self.name, Utc::now())
    }
}

#[async_trait]
impl MarketSource for LocalSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn cache_manager(&self) -> &MarketCacheManager {
        &self.cache
    }

    async fn fetch_marketplace(&self) -> MarketResult<crate::commands::market::plugin::MarketplaceConfig> {
        self.update_timestamp()?;
        load_marketplace(&self.marketplace_path)
    }

    async fn fetch_plugin(
        &self,
        entry: &crate::commands::market::plugin::MarketplacePluginEntry,
    ) -> MarketResult<PluginManifest> {
        let marketplace = self.fetch_marketplace().await?;
        let plugin_root = marketplace_plugin_root(&marketplace, &self.marketplace_root);
        let location = resolve_plugin_source(entry, &plugin_root);
        match location {
            PluginSourceLocation::Local(path) => {
                let plugin_path = path.join(".claude-plugin").join("plugin.json");
                load_manifest(&plugin_path)
            }
            PluginSourceLocation::GitHub { repo, path, reference } => {
                let temp_dir = tempfile::tempdir().map_err(|err| {
                    MarketError::with_source(
                        MarketErrorCode::MarketplaceUnreachable,
                        "Failed to create temp directory",
                        err.into(),
                    )
                })?;
                let temp_path = temp_dir.path().to_path_buf();
                let url = normalize_repo_url(&repo);
                let clone_path = temp_path.clone();
                task::spawn_blocking(move || clone_repo_with_ref(&url, &clone_path, reference.as_deref()))
                    .await
                    .map_err(|err| {
                        MarketError::with_source(
                            MarketErrorCode::MarketplaceUnreachable,
                            "Git clone failed",
                            err.into(),
                        )
                    })??;
                let plugin_path = match path {
                    Some(p) => temp_path.join(p),
                    None => temp_path,
                };
                let manifest_path = plugin_path.join(".claude-plugin").join("plugin.json");
                load_manifest(&manifest_path)
            }
            PluginSourceLocation::Remote { .. } => Err(MarketError::new(
                MarketErrorCode::McpExtractionFailed,
                "Remote plugin sources are not supported for local marketplaces",
            )),
        }
    }

    async fn download_plugin(
        &self,
        entry: &crate::commands::market::plugin::MarketplacePluginEntry,
        plugin_id: &str,
    ) -> MarketResult<PathBuf> {
        let marketplace = self.fetch_marketplace().await?;
        let plugin_root = marketplace_plugin_root(&marketplace, &self.marketplace_root);
        let location = resolve_plugin_source(entry, &plugin_root);
        let plugin_cache = self.cache.ensure_plugin_cache(plugin_id, &self.name)?;
        match location {
            PluginSourceLocation::Local(path) => {
                copy_dir_recursive(&path, &plugin_cache)?;
                Ok(plugin_cache)
            }
            PluginSourceLocation::GitHub { repo, path, reference } => {
                let temp_dir = tempfile::tempdir().map_err(|err| {
                    MarketError::with_source(
                        MarketErrorCode::MarketplaceUnreachable,
                        "Failed to create temp directory",
                        err.into(),
                    )
                })?;
                let temp_path = temp_dir.path().to_path_buf();
                let url = normalize_repo_url(&repo);
                let clone_path = temp_path.clone();
                task::spawn_blocking(move || clone_repo_with_ref(&url, &clone_path, reference.as_deref()))
                    .await
                    .map_err(|err| {
                        MarketError::with_source(
                            MarketErrorCode::MarketplaceUnreachable,
                            "Git clone failed",
                            err.into(),
                        )
                    })??;
                let plugin_path = match path {
                    Some(p) => temp_path.join(p),
                    None => temp_path,
                };
                copy_dir_recursive(&plugin_path, &plugin_cache)?;
                Ok(plugin_cache)
            }
            PluginSourceLocation::Remote { .. } => Err(MarketError::new(
                MarketErrorCode::McpExtractionFailed,
                "Remote plugin sources are not supported for local marketplaces",
            )),
        }
    }

    async fn update(&self) -> MarketResult<()> {
        self.update_timestamp()
    }
}

fn resolve_marketplace_paths(path: &Path) -> MarketResult<(PathBuf, PathBuf)> {
    if path.is_file() {
        let marketplace_path = path.to_path_buf();
        let parent = marketplace_path
            .parent()
            .ok_or_else(|| MarketError::new(MarketErrorCode::MarketplaceFormat, "Invalid marketplace path"))?;
        let marketplace_root = if parent.file_name().and_then(|s| s.to_str()) == Some(".claude-plugin") {
            parent
                .parent()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| MarketError::new(MarketErrorCode::MarketplaceFormat, "Invalid marketplace root"))?
        } else {
            parent.to_path_buf()
        };
        return Ok((marketplace_path, marketplace_root));
    }

    if path.is_dir() {
        let marketplace_path = path.join(".claude-plugin").join("marketplace.json");
        if marketplace_path.exists() {
            return Ok((marketplace_path, path.to_path_buf()));
        }
        let direct_path = path.join("marketplace.json");
        if direct_path.exists() {
            return Ok((direct_path, path.to_path_buf()));
        }
    }

    Err(MarketError::new(
        MarketErrorCode::MarketplaceFormat,
        "Marketplace path is invalid",
    ))
}

fn normalize_repo_url(repo: &str) -> String {
    if repo.starts_with("http://")
        || repo.starts_with("https://")
        || repo.starts_with("git@")
        || repo.starts_with("file://")
    {
        return repo.to_string();
    }
    let mut repo = repo.to_string();
    if !repo.ends_with(".git") {
        repo.push_str(".git");
    }
    format!("https://github.com/{}", repo)
}

fn clone_repo_with_ref(url: &str, path: &Path, reference: Option<&str>) -> MarketResult<()> {
    let mut builder = git2::build::RepoBuilder::new();
    if let Some(reference) = reference {
        builder.branch(reference);
    }
    builder.clone(url, path).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::MarketplaceUnreachable,
            "Failed to clone GitHub repository",
            err.into(),
        )
    })?;
    Ok(())
}
