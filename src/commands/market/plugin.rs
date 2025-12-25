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
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_description")]
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

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_description() -> String {
    "No description provided".to_string()
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

/// MCP server transport type (parsed from .mcp.json, NOT written to mcp.json)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum McpServerConfig {
    /// stdio transport (local executable) - supported by AIW
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: Option<HashMap<String, String>>,
    },
    /// HTTP transport - NOT YET SUPPORTED by AIW
    Http {
        #[serde(rename = "type")]
        transport_type: String,
        url: String,
        #[serde(default)]
        headers: Option<HashMap<String, String>>,
    },
    /// SSE transport - NOT YET SUPPORTED by AIW
    Sse {
        #[serde(rename = "type")]
        transport_type: String,
        url: String,
    },
}

/// Format for writing to mcp.json (stdio only, compatible with AIW MCP routing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigWrite {
    pub command: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl McpServerConfig {
    /// Check if this is a stdio transport (supported by AIW)
    pub fn is_stdio(&self) -> bool {
        matches!(self, McpServerConfig::Stdio { .. })
    }

    /// Get the command for stdio transports
    pub fn get_command(&self) -> Option<&str> {
        match self {
            McpServerConfig::Stdio { command, .. } => Some(command),
            _ => None,
        }
    }

    /// Get the args for stdio transports
    pub fn get_args(&self) -> Option<&[String]> {
        match self {
            McpServerConfig::Stdio { args, .. } => Some(args),
            _ => None,
        }
    }

    /// Get the env for stdio transports
    pub fn get_env(&self) -> Option<&HashMap<String, String>> {
        match self {
            McpServerConfig::Stdio { env, .. } => env.as_ref(),
            _ => None,
        }
    }

    /// Get mutable env for stdio transports
    pub fn get_env_mut(&mut self) -> Option<&mut HashMap<String, String>> {
        match self {
            McpServerConfig::Stdio { env, .. } => env.as_mut(),
            _ => None,
        }
    }

    /// Convert to write format (only for stdio)
    pub fn to_write_format(&self) -> Option<McpServerConfigWrite> {
        match self {
            McpServerConfig::Stdio { command, args, env } => Some(McpServerConfigWrite {
                command: command.clone(),
                args: args.clone(),
                env: env.clone().unwrap_or_default(),
            }),
            _ => None,
        }
    }
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
