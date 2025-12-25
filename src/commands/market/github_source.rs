//! GitHub marketplace source implementation.

use crate::commands::market::cache::{copy_dir_recursive, MarketCacheManager};
use crate::commands::market::plugin::PluginManifest;
use crate::commands::market::plugin_io::{
    load_manifest, load_marketplace, marketplace_plugin_root, resolve_plugin_source,
    PluginSourceLocation,
};
use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult, MarketSource};
use async_trait::async_trait;
use chrono::Utc;
use git2::{build::RepoBuilder, FetchOptions, Repository};
use std::path::{Path, PathBuf};
use tokio::task;

#[derive(Clone)]
pub struct GithubSource {
    name: String,
    repo: String,
    cache: MarketCacheManager,
}

impl GithubSource {
    pub fn new(name: String, repo: String, cache: MarketCacheManager) -> Self {
        Self { name, repo, cache }
    }

    fn repo_url(&self) -> String {
        if self.repo.starts_with("http://")
            || self.repo.starts_with("https://")
            || self.repo.starts_with("git@")
            || self.repo.starts_with("file://")
        {
            return self.repo.clone();
        }
        if self.repo.contains("github.com/") {
            return self.repo.clone();
        }
        let mut repo = self.repo.clone();
        if !repo.ends_with(".git") {
            repo.push_str(".git");
        }
        format!("https://github.com/{}", repo)
    }

    async fn ensure_repo(&self) -> MarketResult<PathBuf> {
        let path = self.cache.ensure_marketplace_cache(&self.name)?;
        let git_path = path.join(".git");
        if git_path.exists() {
            return Ok(path);
        }
        let url = self.repo_url();
        let clone_path = path.clone();
        task::spawn_blocking(move || clone_repo(&url, &clone_path))
            .await
            .map_err(|err| MarketError::with_source(MarketErrorCode::MarketplaceUnreachable, "Git clone failed", err.into()))??;
        self.cache
            .write_last_update(&self.name, Utc::now())?;
        Ok(path)
    }

    async fn fetch_repo_update(&self) -> MarketResult<()> {
        let path = self.cache.ensure_marketplace_cache(&self.name)?;
        if !path.join(".git").exists() {
            let url = self.repo_url();
            task::spawn_blocking(move || clone_repo(&url, &path))
                .await
                .map_err(|err| {
                    MarketError::with_source(
                        MarketErrorCode::MarketplaceUnreachable,
                        "Git clone failed",
                        err.into(),
                    )
                })??;
        } else {
            task::spawn_blocking(move || fetch_repo(&path))
                .await
                .map_err(|err| {
                    MarketError::with_source(
                        MarketErrorCode::MarketplaceUnreachable,
                        "Git fetch failed",
                        err.into(),
                    )
                })??;
        }
        self.cache
            .write_last_update(&self.name, Utc::now())?;
        Ok(())
    }
}

#[async_trait]
impl MarketSource for GithubSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn cache_manager(&self) -> &MarketCacheManager {
        &self.cache
    }

    async fn fetch_marketplace(&self) -> MarketResult<crate::commands::market::plugin::MarketplaceConfig> {
        let repo_path = self.ensure_repo().await?;
        let marketplace_path = repo_path.join(".claude-plugin").join("marketplace.json");
        load_marketplace(&marketplace_path)
    }

    async fn fetch_plugin(
        &self,
        entry: &crate::commands::market::plugin::MarketplacePluginEntry,
    ) -> MarketResult<PluginManifest> {
        let repo_path = self.ensure_repo().await?;
        let marketplace = self.fetch_marketplace().await?;
        let plugin_root = marketplace_plugin_root(&marketplace, &repo_path);
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
                "Remote plugin sources are not supported for GitHub marketplaces",
            )),
        }
    }

    async fn download_plugin(
        &self,
        entry: &crate::commands::market::plugin::MarketplacePluginEntry,
        plugin_id: &str,
    ) -> MarketResult<PathBuf> {
        let repo_path = self.ensure_repo().await?;
        let marketplace = self.fetch_marketplace().await?;
        let plugin_root = marketplace_plugin_root(&marketplace, &repo_path);
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
                "Remote plugin sources are not supported for GitHub marketplaces",
            )),
        }
    }

    async fn update(&self) -> MarketResult<()> {
        self.fetch_repo_update().await
    }
}

fn clone_repo(url: &str, path: &Path) -> MarketResult<()> {
    Repository::clone(url, path).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::MarketplaceUnreachable,
            "Failed to clone GitHub repository",
            err.into(),
        )
    })?;
    Ok(())
}

fn fetch_repo(path: &Path) -> MarketResult<()> {
    let repo = Repository::open(path).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::MarketplaceUnreachable,
            "Failed to open Git repository",
            err.into(),
        )
    })?;
    let mut remote = repo.find_remote("origin").map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::MarketplaceUnreachable,
            "Failed to find git remote",
            err.into(),
        )
    })?;
    let mut fetch_opts = FetchOptions::new();
    remote
        .fetch(
            &[
                "refs/heads/*:refs/remotes/origin/*",
                "refs/tags/*:refs/tags/*",
            ],
            Some(&mut fetch_opts),
            None,
        )
        .map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::MarketplaceUnreachable,
                "Failed to fetch GitHub repository",
                err.into(),
            )
        })?;
    Ok(())
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
    let mut builder = RepoBuilder::new();
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
