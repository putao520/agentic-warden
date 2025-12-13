use anyhow::{anyhow, Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tracing::debug;

const DEFAULT_CONFIG_FILE: &str = "mcp.json";
const DEFAULT_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(rename = "mcpServers", alias = "mcp_servers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,

    // Optional fields for Claude Code compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<HealthCheckConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub interval: u64,
    pub timeout: u64,
}

// Routing configuration constants - hardcoded per design decision
pub const DEFAULT_MAX_TOOLS_PER_REQUEST: usize = 10;
pub const DEFAULT_CLUSTERING_THRESHOLD: f32 = 0.7;
pub const DEFAULT_RERANK_TOP_K: usize = 5;
pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.5;

pub struct McpConfigManager {
    path: PathBuf,
    config: McpConfig,
    last_loaded: Option<SystemTime>,
}

impl McpConfigManager {
    pub fn load() -> Result<Self> {
        let path = resolve_config_path()?;

        // Try to load from file, but handle missing file gracefully
        let (config, last_loaded) = if path.exists() {
            let metadata = fs::metadata(&path)?;
            let last_loaded = metadata.modified().ok();
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read MCP config from {}", path.display()))?;
            let mut config: McpConfig = serde_json::from_str(&content)
                .with_context(|| format!("Invalid JSON in {}", path.display()))?;

            // Apply environment variable overrides based on mcp.json structure
            Self::apply_env_overrides(&mut config)?;
            config.validate()?;

            (config, last_loaded)
        } else {
            // Create default config
            let mut config = McpConfig {
                version: DEFAULT_VERSION.to_string(),
                mcp_servers: std::collections::HashMap::new(),
            };

            // Apply environment variable overrides
            Self::apply_env_overrides(&mut config)?;
            config.validate()?;

            (config, None)
        };

        Ok(Self {
            path,
            config,
            last_loaded,
        })
    }

    /// Apply OpenAI environment variable configuration
    /// LLM configuration is managed exclusively through environment variables per REQ-013
    fn apply_env_overrides(_config: &mut McpConfig) -> Result<()> {
        // Validate OpenAI environment variables if present
        if let Ok(token) = std::env::var("OPENAI_TOKEN") {
            Self::validate_openai_token(&token)?;
            debug!("OpenAI token configured via environment variable");
        }

        if let Ok(endpoint) = std::env::var("OPENAI_ENDPOINT") {
            Self::validate_openai_endpoint(&endpoint)?;
            debug!(
                "OpenAI endpoint configured via environment variable: {}",
                endpoint
            );
        }

        if let Ok(model) = std::env::var("OPENAI_MODEL") {
            debug!(
                "OpenAI model configured via environment variable: {}",
                model
            );
        }

        Ok(())
    }

    /// Validate OpenAI token format
    fn validate_openai_token(token: &str) -> Result<()> {
        if token.is_empty() {
            return Err(anyhow!("OPENAI_TOKEN cannot be empty"));
        }

        if !token.starts_with("sk-") {
            return Err(anyhow!(
                "OPENAI_TOKEN must start with 'sk-' prefix. Invalid token format detected."
            ));
        }

        Ok(())
    }

    /// Validate OpenAI endpoint URL format
    fn validate_openai_endpoint(endpoint: &str) -> Result<()> {
        let url = endpoint
            .parse::<url::Url>()
            .with_context(|| format!("Invalid OpenAI endpoint URL: {}", endpoint))?;

        if url.scheme() != "https" && url.scheme() != "http" {
            return Err(anyhow!(
                "OpenAI endpoint must use http or https protocol. Invalid URL: {}",
                endpoint
            ));
        }

        Ok(())
    }

    pub fn config(&self) -> &McpConfig {
        &self.config
    }

    pub fn enabled_servers(&self) -> Vec<(String, McpServerConfig)> {
        self.config
            .mcp_servers
            .iter()
            .filter(|(_, cfg)| {
                // Filter by enabled field if present (Claude Code compatibility)
                cfg.enabled.unwrap_or(true)
            })
            .map(|(name, cfg)| (name.clone(), cfg.clone()))
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
            return Err(anyhow!("No MCP servers configured in mcp.json"));
        }

        for (name, server) in &self.mcp_servers {
            if server.command.trim().is_empty() {
                return Err(anyhow!("Server '{}' is missing a command", name));
            }
        }
        Ok(())
    }
}

fn resolve_config_path() -> Result<PathBuf> {
    // Only support global config at ~/.aiw/mcp.json
    // 100% compatible with Claude Code and other AI tools
    let home = home_dir().ok_or_else(|| anyhow!("Cannot find home directory"))?;
    Ok(home.join(".aiw").join(DEFAULT_CONFIG_FILE))
}

fn default_version() -> String {
    DEFAULT_VERSION.to_string()
}
