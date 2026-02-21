use anyhow::{anyhow, Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process,
    thread,
    time::{Duration, SystemTime},
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

    pub fn update_server_env(
        &mut self,
        name: &str,
        env: HashMap<String, String>,
    ) -> Result<()> {
        let server = self
            .config
            .mcp_servers
            .get_mut(name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", name))?;
        server.env = env;
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        self.ensure_config_dir()?;
        let _lock = ConfigLock::acquire(&self.path)?;
        let content = serde_json::to_string_pretty(&self.config)
            .with_context(|| format!("Failed to serialize MCP config for {}", self.path.display()))?;
        fs::write(&self.path, content)
            .with_context(|| format!("Failed to write MCP config to {}", self.path.display()))?;
        self.last_loaded = fs::metadata(&self.path)
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        Ok(())
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

    fn ensure_config_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create MCP config directory {}", parent.display())
            })?;
        }
        Ok(())
    }
}

const LOCK_RETRY_COUNT: usize = 10;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(100);
const LOCK_STALE_SECS: u64 = 30;

struct ConfigLock {
    path: PathBuf,
    _file: fs::File,
}

impl ConfigLock {
    fn acquire(config_path: &Path) -> Result<Self> {
        let lock_path = lock_path_for(config_path)?;

        for _ in 0..=LOCK_RETRY_COUNT {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    let _ = writeln!(file, "pid={}", process::id());
                    let _ = writeln!(
                        file,
                        "started={}",
                        SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    );
                    return Ok(Self {
                        path: lock_path,
                        _file: file,
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if is_lock_stale(&lock_path)? {
                        let _ = fs::remove_file(&lock_path);
                        continue;
                    }
                    thread::sleep(LOCK_RETRY_DELAY);
                }
                Err(err) => return Err(err.into()),
            }
        }

        Err(anyhow!(
            "MCP config is locked by another process. Try again later."
        ))
    }
}

impl Drop for ConfigLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn lock_path_for(config_path: &Path) -> Result<PathBuf> {
    let file_name = config_path
        .file_name()
        .ok_or_else(|| anyhow!("Invalid MCP config path"))?;
    Ok(config_path.with_file_name(format!("{}.lock", file_name.to_string_lossy())))
}

fn is_lock_stale(path: &Path) -> Result<bool> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err.into()),
    };
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    Ok(age > Duration::from_secs(LOCK_STALE_SECS))
}

impl McpConfig {
    fn validate(&self) -> Result<()> {
        // Empty mcp_servers is valid — AIW itself can serve as an MCP server
        // without any external MCP backends configured.
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
