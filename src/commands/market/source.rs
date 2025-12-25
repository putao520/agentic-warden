//! Marketplace source traits and configuration.

use crate::commands::market::cache::MarketCacheManager;
use crate::commands::market::plugin::{MarketplaceConfig, MarketplacePluginEntry, PluginManifest};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketErrorCode {
    MarketplaceUnreachable,
    MarketplaceFormat,
    MarketplaceExists,
    MarketplaceNotFound,
    PluginNotFound,
    PluginMissingMcp,
    McpExtractionFailed,
    ConfigWriteFailed,
    InvalidEnvironment,
}

impl MarketErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            MarketErrorCode::MarketplaceUnreachable => "MCP-MKT-001",
            MarketErrorCode::MarketplaceFormat => "MCP-MKT-002",
            MarketErrorCode::MarketplaceExists => "MCP-MKT-003",
            MarketErrorCode::MarketplaceNotFound => "MCP-MKT-004",
            MarketErrorCode::PluginNotFound => "MCP-MKT-005",
            MarketErrorCode::PluginMissingMcp => "MCP-MKT-006",
            MarketErrorCode::McpExtractionFailed => "MCP-MKT-007",
            MarketErrorCode::ConfigWriteFailed => "MCP-MKT-008",
            MarketErrorCode::InvalidEnvironment => "MCP-MKT-009",
        }
    }
}

#[derive(Debug)]
pub struct MarketError {
    pub code: MarketErrorCode,
    pub message: String,
    pub source: Option<anyhow::Error>,
}

impl MarketError {
    pub fn new(code: MarketErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(code: MarketErrorCode, message: impl Into<String>, source: anyhow::Error) -> Self {
        Self {
            code,
            message: message.into(),
            source: Some(source),
        }
    }
}

impl fmt::Display for MarketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for MarketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|err| err.as_ref() as _)
    }
}

pub type MarketResult<T> = Result<T, MarketError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MarketplaceSourceConfig {
    Github { repo: String },
    Local { path: String },
    Remote { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSettingsEntry {
    pub source: MarketplaceSourceConfig,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone)]
pub struct MarketplaceDefinition {
    pub name: String,
    pub entry: MarketplaceSettingsEntry,
}

#[async_trait]
pub trait MarketSource: Send + Sync {
    fn name(&self) -> &str;
    fn cache_manager(&self) -> &MarketCacheManager;
    async fn fetch_marketplace(&self) -> MarketResult<MarketplaceConfig>;
    async fn fetch_plugin(&self, entry: &MarketplacePluginEntry) -> MarketResult<PluginManifest>;
    async fn download_plugin(
        &self,
        entry: &MarketplacePluginEntry,
        plugin_id: &str,
    ) -> MarketResult<PathBuf>;
    async fn update(&self) -> MarketResult<()>;
}

pub fn default_marketplaces() -> HashMap<String, MarketplaceSettingsEntry> {
    let mut map = HashMap::new();
    map.insert(
        "claude-code-official".to_string(),
        MarketplaceSettingsEntry {
            source: MarketplaceSourceConfig::Github {
                repo: "anthropics/claude-plugins-official".to_string(),
            },
            enabled: true,
        },
    );
    map.insert(
        "aiw-official".to_string(),
        MarketplaceSettingsEntry {
            source: MarketplaceSourceConfig::Github {
                repo: "putao520/aiw-plugins".to_string(),
            },
            enabled: true,
        },
    );
    map
}
