use anyhow::{anyhow, Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

const DEFAULT_CONFIG_FILE: &str = ".mcp.json";
const DEFAULT_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(rename = "mcpServers", alias = "mcp_servers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub llm: LlmSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    #[serde(default = "enabled_true")]
    pub enabled: bool,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckConfig {
    #[serde(default = "enabled_true")]
    pub enabled: bool,
    #[serde(default = "default_health_interval")]
    pub interval: u64,
    #[serde(default = "default_health_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingConfig {
    #[serde(default = "default_max_tools")]
    pub max_tools_per_request: usize,
    #[serde(default = "default_clustering_threshold")]
    pub clustering_threshold: f32,
    #[serde(default = "default_rerank_top_k")]
    pub rerank_top_k: usize,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmSection {
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub timeout: Option<u64>,
}

pub struct McpConfigManager {
    path: PathBuf,
    config: McpConfig,
    last_loaded: Option<SystemTime>,
}

impl McpConfigManager {
    pub fn load() -> Result<Self> {
        let path = resolve_config_path()?;
        let metadata = fs::metadata(&path)?;
        let last_loaded = metadata.modified().ok();
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read MCP config from {}", path.display()))?;
        let config: McpConfig = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in {}", path.display()))?;
        config.validate()?;
        Ok(Self {
            path,
            config,
            last_loaded,
        })
    }

    pub fn config(&self) -> &McpConfig {
        &self.config
    }

    pub fn enabled_servers(&self) -> Vec<(String, McpServerConfig)> {
        self.config
            .mcp_servers
            .iter()
            .filter_map(|(name, cfg)| {
                if cfg.enabled {
                    Some((name.clone(), cfg.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn reload_if_changed(&mut self) -> Result<bool> {
        let metadata = fs::metadata(&self.path)?;
        let modified = metadata.modified().ok();
        if modified.is_some() && modified == self.last_loaded {
            return Ok(false);
        }
        let content = fs::read_to_string(&self.path)?;
        let config: McpConfig = serde_json::from_str(&content)?;
        config.validate()?;
        self.config = config;
        self.last_loaded = modified;
        Ok(true)
    }
}

impl McpConfig {
    fn validate(&self) -> Result<()> {
        if self.mcp_servers.is_empty() {
            return Err(anyhow!("No MCP servers configured in .mcp.json"));
        }

        for (name, server) in &self.mcp_servers {
            if server.command.trim().is_empty() {
                return Err(anyhow!("Server '{}' is missing a command", name));
            }
            if server
                .description
                .as_deref()
                .map(str::is_empty)
                .unwrap_or(true)
            {
                return Err(anyhow!("Server '{}' requires a description", name));
            }
            if server
                .category
                .as_deref()
                .map(str::is_empty)
                .unwrap_or(true)
            {
                return Err(anyhow!(
                    "Server '{}' requires a category (development/system/utility/ai/other)",
                    name
                ));
            }
            if server.health_check.interval == 0 || server.health_check.timeout == 0 {
                return Err(anyhow!(
                    "Server '{}' health_check interval/timeout must be > 0",
                    name
                ));
            }
        }
        Ok(())
    }
}

fn resolve_config_path() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("AGENTIC_WARDEN_MCP_CONFIG") {
        let path = PathBuf::from(custom);
        if path.exists() {
            return Ok(path);
        }
    }

    let mut candidates = Vec::new();
    if let Some(home) = home_dir() {
        candidates.push(home.join(".agentic-warden").join(DEFAULT_CONFIG_FILE));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(DEFAULT_CONFIG_FILE));
    }

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "Unable to locate .mcp.json. Expected at ~/.agentic-warden/.mcp.json or ./{}",
        DEFAULT_CONFIG_FILE
    ))
}

fn default_version() -> String {
    DEFAULT_VERSION.to_string()
}

fn enabled_true() -> bool {
    true
}

fn default_health_interval() -> u64 {
    60
}

fn default_health_timeout() -> u64 {
    10
}

fn default_max_tools() -> usize {
    10
}

fn default_clustering_threshold() -> f32 {
    0.7
}

fn default_rerank_top_k() -> usize {
    5
}

fn default_similarity_threshold() -> f32 {
    0.5
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            max_tools_per_request: default_max_tools(),
            clustering_threshold: default_clustering_threshold(),
            rerank_top_k: default_rerank_top_k(),
            similarity_threshold: default_similarity_threshold(),
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: default_health_interval(),
            timeout: default_health_timeout(),
        }
    }
}
