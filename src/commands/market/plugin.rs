//! Marketplace and plugin data structures.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceOwner {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceMetadata {
    pub description: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "pluginRoot")]
    pub plugin_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceConfig {
    pub name: String,
    pub owner: MarketplaceOwner,
    #[serde(default)]
    pub metadata: Option<MarketplaceMetadata>,
    pub plugins: Vec<MarketplacePluginEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: PluginAuthor,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Option<Vec<String>>,
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Option<Value>,
    pub commands: Option<Value>,
    pub agents: Option<Value>,
    pub hooks: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginSource {
    Path(String),
    Object(PluginSourceObject),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginSourceObject {
    pub repo: Option<String>,
    pub url: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePluginEntry {
    pub name: String,
    pub source: PluginSource,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<PluginAuthor>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServersFile {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Clone)]
pub struct McpServerInfo {
    pub name: String,
    pub config: McpServerConfig,
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: PluginAuthor,
    pub marketplace: String,
    pub source: PluginSource,
    pub has_mcp_servers: bool,
    pub mcp_servers: Vec<McpServerInfo>,
    pub category: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PluginDetail {
    pub entry: MarketplacePluginEntry,
    pub manifest: PluginManifest,
    pub mcp_config: Option<McpServersFile>,
}
