use crate::mcp_routing::config::{McpConfig, McpServerConfig};
use crate::utils::env;
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use rmcp::{
    model::{CallToolRequestParam, ClientInfo, Tool},
    service::{RoleClient, RunningService, ServiceExt},
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use serde_json::{to_value, Value};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::RwLock};

#[derive(Debug, Clone)]
pub struct DiscoveredTool {
    pub server: String,
    pub definition: Tool,
}

pub struct McpConnectionPool {
    config: Arc<RwLock<Arc<McpConfig>>>,
    handles: RwLock<HashMap<String, Arc<McpServerHandle>>>,
}

struct ServerState {
    running: RunningService<RoleClient, ClientInfo>,
    last_refresh: Instant,
    tools: Vec<Tool>,
}

pub struct McpServerHandle {
    name: String,
    state: Mutex<ServerState>,
}

impl McpConnectionPool {
    pub fn new(config: Arc<McpConfig>) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            handles: RwLock::new(HashMap::new()),
        }
    }

    /// Update the configuration and manage server lifecycle (used for hot reload)
    pub async fn update_config(&self, new_config: Arc<McpConfig>) {
        let old_config = self.config.read().await.clone();

        // Update config first
        {
            let mut config_guard = self.config.write().await;
            *config_guard = new_config.clone();
        }

        // Find servers that need to be shut down
        let mut to_remove = Vec::new();
        {
            let handles = self.handles.read().await;
            for (name, _handle) in handles.iter() {
                let should_remove = match new_config.mcp_servers.get(name) {
                    None => {
                        // Server removed from config
                        eprintln!("🗑️  Shutting down removed MCP server: {}", name);
                        true
                    }
                    Some(server_config) => {
                        // Server disabled
                        if !server_config.enabled.unwrap_or(true) {
                            eprintln!("⏸️  Shutting down disabled MCP server: {}", name);
                            true
                        } else {
                            // Check if config changed (command, args, or env)
                            let old_server = old_config.mcp_servers.get(name);
                            let config_changed = match old_server {
                                None => true, // New server
                                Some(old) => {
                                    old.command != server_config.command
                                        || old.args != server_config.args
                                        || old.env != server_config.env
                                }
                            };

                            if config_changed {
                                eprintln!("🔄 Restarting MCP server with changed config: {}", name);
                            }
                            config_changed
                        }
                    }
                };

                if should_remove {
                    to_remove.push(name.clone());
                }
            }
        }

        // Remove servers (dropping the handle kills the child process via kill_on_drop)
        {
            let mut handles = self.handles.write().await;
            for name in to_remove {
                if let Some(handle) = handles.remove(&name) {
                    drop(handle);
                }
            }
        }

        eprintln!("✅ MCP configuration reloaded");
    }

    /// Get current configuration
    pub async fn get_config(&self) -> Arc<McpConfig> {
        self.config.read().await.clone()
    }

    pub async fn warm_up(&self) -> Result<Vec<DiscoveredTool>> {
        let mut all = Vec::new();
        let config = self.config.read().await.clone();

        for (name, server) in config.mcp_servers.iter() {
            // Skip disabled servers (Claude Code compatibility)
            if !server.enabled.unwrap_or(true) {
                continue;
            }

            match self.ensure_handle(name.clone(), server.clone()).await {
                Ok(handle) => match handle.list_tools().await {
                    Ok(mut tools) => {
                        eprintln!("✅ Connected to MCP server '{}': {} tools", name, tools.len());
                        all.append(&mut tools);
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to list tools from '{}': {}", name, e);
                    }
                },
                Err(e) => {
                    eprintln!("⚠️  Failed to connect to MCP server '{}': {}", name, e);
                }
            }
        }
        Ok(all)
    }

    pub async fn ensure_handle(
        &self,
        name: String,
        config: McpServerConfig,
    ) -> Result<Arc<McpServerHandle>> {
        if let Some(existing) = self.handles.read().await.get(&name).cloned() {
            return Ok(existing);
        }

        let mut guard = self.handles.write().await;
        if let Some(existing) = guard.get(&name).cloned() {
            return Ok(existing);
        }

        let handle = Arc::new(McpServerHandle::spawn(name.clone(), config).await?);
        guard.insert(name, handle.clone());
        Ok(handle)
    }

    pub async fn call_tool(&self, server: &str, tool_name: &str, args: Value) -> Result<Value> {
        let config = self.config.read().await.clone();
        let server_config = config
            .mcp_servers
            .get(server)
            .ok_or_else(|| anyhow!("Unknown MCP server '{}'", server))?
            .clone();

        let handle = self
            .ensure_handle(server.to_string(), server_config)
            .await
            .context("Failed to initialize MCP server connection")?;

        handle.call_tool(tool_name, args).await
    }
}

impl McpServerHandle {
    async fn spawn(name: String, config: McpServerConfig) -> Result<Self> {
        let running = spawn_client(&config).await?;
        let tools = running.peer().list_all_tools().await?;

        Ok(Self {
            name,
            state: Mutex::new(ServerState {
                running,
                last_refresh: Instant::now(),
                tools,
            }),
        })
    }

    pub async fn list_tools(&self) -> Result<Vec<DiscoveredTool>> {
        let needs_refresh = {
            let state = self.state.lock();
            state.last_refresh.elapsed() > Duration::from_secs(60)
        };
        if needs_refresh {
            let peer = {
                let state = self.state.lock();
                state.running.peer().clone()
            };
            let refreshed = peer.list_all_tools().await?;
            let state = &mut *self.state.lock();
            state.tools = refreshed;
            state.last_refresh = Instant::now();
        }
        let tools = {
            let state = self.state.lock();
            state.tools.clone()
        };
        Ok(tools
            .into_iter()
            .map(|definition| DiscoveredTool {
                server: self.name.clone(),
                definition,
            })
            .collect())
    }

    pub async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value> {
        let peer = {
            let state = self.state.lock();
            state.running.peer().clone()
        };
        let arguments = match args {
            Value::Object(map) => Some(map),
            Value::Null => None,
            _ => {
                return Err(anyhow!(
                    "Tool arguments must be an object, received {}",
                    args
                ))
            }
        };
        let param = CallToolRequestParam {
            name: tool_name.to_string().into(),
            arguments,
        };
        let result = peer.call_tool(param).await?;
        if let Some(structured) = result.structured_content {
            return Ok(structured);
        }
        if !result.content.is_empty() {
            let aggregated = result
                .content
                .into_iter()
                .map(|chunk| to_value(&chunk).unwrap_or(Value::Null).to_string())
                .collect::<Vec<_>>()
                .join("\n");
            return Ok(Value::String(aggregated));
        }
        Ok(Value::Null)
    }
}

/// Expand environment variable placeholder (${VAR_NAME})
/// Windows: case-insensitive, Linux/macOS: case-sensitive
fn expand_env_var(value: &str) -> String {
    env::expand_env_var(value)
}

async fn spawn_client(config: &McpServerConfig) -> Result<RunningService<RoleClient, ClientInfo>> {
    let transport = TokioChildProcess::new(Command::new(&config.command).configure(|cmd| {
        cmd.args(&config.args);
        // Pass environment variables to the MCP server process
        for (key, value) in &config.env {
            // Expand environment variable placeholders (${VAR_NAME})
            let expanded_value = expand_env_var(value);
            cmd.env(key, expanded_value);
        }
        cmd.kill_on_drop(true);
    }))?;

    let mut info = ClientInfo::default();
    info.client_info.name = "agentic-warden-router".into();

    info.serve(transport).await.map_err(|err| anyhow!(err))
}
