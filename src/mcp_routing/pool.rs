use crate::mcp_routing::config::{McpConfig, McpServerConfig};
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
    config: Arc<McpConfig>,
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
            config,
            handles: RwLock::new(HashMap::new()),
        }
    }

    pub async fn warm_up(&self) -> Result<Vec<DiscoveredTool>> {
        let mut all = Vec::new();
        for (name, server) in self
            .config
            .mcp_servers
            .iter()
            .filter(|(_, cfg)| cfg.enabled)
        {
            let handle = self
                .ensure_handle(name.clone(), server.clone())
                .await?;
            let mut tools = handle.list_tools().await?;
            all.append(&mut tools);
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
        let config = self
            .config
            .mcp_servers
            .get(server)
            .ok_or_else(|| anyhow!("Unknown MCP server '{}'", server))?
            .clone();

        let handle = self
            .ensure_handle(server.to_string(), config)
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
        if let Some(content) = result.content {
            let aggregated = content
                .into_iter()
                .map(|chunk| to_value(&chunk).unwrap_or(Value::Null).to_string())
                .collect::<Vec<_>>()
                .join("\n");
            return Ok(Value::String(aggregated));
        }
        Ok(Value::Null)
    }
}

async fn spawn_client(config: &McpServerConfig) -> Result<RunningService<RoleClient, ClientInfo>> {
    let transport = TokioChildProcess::new(Command::new(&config.command).configure(|cmd| {
        cmd.args(&config.args);
        cmd.kill_on_drop(true);
    }))?;

    let mut info = ClientInfo::default();
    info.client_info.name = "agentic-warden-router".into();

    info.serve(transport).await.map_err(|err| anyhow!(err))
}
