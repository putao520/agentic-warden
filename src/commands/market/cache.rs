//! Cache path management for marketplace and plugins.

use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult};
use crate::utils::config_paths::ConfigPaths;
use chrono::{DateTime, Utc};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MarketCacheManager {
    pub cache_root: PathBuf,
    pub market_dir: PathBuf,
    pub plugin_dir: PathBuf,
}

impl MarketCacheManager {
    pub fn new() -> MarketResult<Self> {
        let paths = ConfigPaths::new().map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to resolve config paths",
                err.into(),
            )
        })?;
        let cache_root = paths.config_dir.join("cache");
        let market_dir = cache_root.join("market");
        let plugin_dir = cache_root.join("plugins");
        let manager = Self {
            cache_root,
            market_dir,
            plugin_dir,
        };
        manager.ensure_dirs()?;
        Ok(manager)
    }

    fn ensure_dirs(&self) -> MarketResult<()> {
        fs::create_dir_all(&self.market_dir).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to create market cache directory",
                err.into(),
            )
        })?;
        fs::create_dir_all(&self.plugin_dir).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to create plugin cache directory",
                err.into(),
            )
        })?;
        set_permissions_0700(&self.cache_root)?;
        set_permissions_0700(&self.market_dir)?;
        set_permissions_0700(&self.plugin_dir)?;
        Ok(())
    }

    pub fn marketplace_cache_path(&self, name: &str) -> PathBuf {
        self.market_dir.join(name)
    }

    pub fn plugin_cache_path(&self, plugin_id: &str, marketplace: &str) -> PathBuf {
        self.plugin_dir
            .join(format!("{}@{}", plugin_id, marketplace))
    }

    pub fn ensure_marketplace_cache(&self, name: &str) -> MarketResult<PathBuf> {
        let path = self.marketplace_cache_path(name);
        fs::create_dir_all(&path).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to create marketplace cache",
                err.into(),
            )
        })?;
        set_permissions_0700(&path)?;
        Ok(path)
    }

    pub fn ensure_plugin_cache(&self, plugin_id: &str, marketplace: &str) -> MarketResult<PathBuf> {
        let path = self.plugin_cache_path(plugin_id, marketplace);
        fs::create_dir_all(&path).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to create plugin cache",
                err.into(),
            )
        })?;
        set_permissions_0700(&path)?;
        Ok(path)
    }

    pub fn write_last_update(&self, marketplace: &str, timestamp: DateTime<Utc>) -> MarketResult<()> {
        let cache_path = self.ensure_marketplace_cache(marketplace)?;
        let path = cache_path.join(".last_update");
        write_timestamp(&path, timestamp)
    }

    pub fn read_last_update(&self, marketplace: &str) -> Option<DateTime<Utc>> {
        let path = self.marketplace_cache_path(marketplace).join(".last_update");
        read_timestamp(&path)
    }

    pub fn write_installed_at(
        &self,
        plugin_id: &str,
        marketplace: &str,
        timestamp: DateTime<Utc>,
    ) -> MarketResult<()> {
        let cache_path = self.ensure_plugin_cache(plugin_id, marketplace)?;
        let path = cache_path.join(".installed_at");
        write_timestamp(&path, timestamp)
    }
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> MarketResult<()> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to clear plugin cache",
                err.into(),
            )
        })?;
    }
    fs::create_dir_all(dst).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::ConfigWriteFailed,
            "Failed to create plugin cache",
            err.into(),
        )
    })?;
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to read plugin files",
                err.into(),
            )
        })?;
        let relative = entry.path().strip_prefix(src).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to resolve plugin cache path",
                err.into(),
            )
        })?;
        let target = dst.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|err| {
                MarketError::with_source(
                    MarketErrorCode::ConfigWriteFailed,
                    "Failed to create plugin cache directory",
                    err.into(),
                )
            })?;
            continue;
        }
        fs::copy(entry.path(), &target).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to copy plugin file",
                err.into(),
            )
        })?;
    }
    set_permissions_0700(dst)?;
    Ok(())
}

fn write_timestamp(path: &Path, timestamp: DateTime<Utc>) -> MarketResult<()> {
    let mut file = fs::File::create(path).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::ConfigWriteFailed,
            "Failed to write timestamp",
            err.into(),
        )
    })?;
    file.write_all(timestamp.to_rfc3339().as_bytes()).map_err(|err| {
        MarketError::with_source(
            MarketErrorCode::ConfigWriteFailed,
            "Failed to write timestamp",
            err.into(),
        )
    })?;
    set_permissions_0600(path)?;
    Ok(())
}

fn read_timestamp(path: &Path) -> Option<DateTime<Utc>> {
    let contents = fs::read_to_string(path).ok()?;
    DateTime::parse_from_rfc3339(contents.trim())
        .ok()
        .map(|ts| ts.with_timezone(&Utc))
}

fn set_permissions_0700(path: &Path) -> MarketResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|err| {
            MarketError::with_source(
                MarketErrorCode::ConfigWriteFailed,
                "Failed to set cache directory permissions",
                err.into(),
            )
        })?;
    }
    Ok(())
}

fn set_permissions_0600(path: &Path) -> MarketResult<()> {
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
