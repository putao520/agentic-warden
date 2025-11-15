pub mod capability_detector;
pub mod dynamic_tools;

use crate::mcp_routing::{
    models::{
        ExecuteToolRequest, ExecuteToolResponse, IntelligentRouteRequest,
        IntelligentRouteResponse, MethodSchemaResponse,
    },
    IntelligentRouter,
};
use crate::memory::{ConversationHistoryStore, ConversationSearchResult};
use capability_detector::ClientCapabilities;
use dynamic_tools::DynamicToolManager;
use std::future::Future;
use rmcp::{
    handler::server::tool::{ToolCallContext, ToolRouter},
    handler::server::wrapper::Parameters,
    handler::server::ServerHandler,
    model::{Implementation, InitializeRequestParam, InitializeResult, ServerCapabilities},
    service::{Peer, RequestContext, RoleServer},
    tool, Json, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::path::PathBuf;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MethodSchemaParams {
    pub mcp_server: String,
    pub tool_name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchHistoryParams {
    /// The search query to find relevant conversation history.
    pub query: String,
    /// Maximum number of results to return (default: 10).
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_limit() -> usize {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TaskSpec {
    /// AI CLI type (claude, codex, or gemini).
    pub ai_type: String,
    /// Task description/prompt for the AI.
    pub task: String,
    /// Optional provider name (e.g., "openrouter", "anthropic").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StartConcurrentTasksParams {
    /// Array of tasks to launch concurrently.
    pub tasks: Vec<TaskSpec>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TaskLaunchResult {
    /// Process ID of the launched task.
    pub pid: u32,
    /// AI CLI type used.
    pub ai_type: String,
    /// Task description.
    pub task: String,
    /// Provider used (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Log file path.
    pub log_file: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetTaskCommandParams {
    /// AI CLI type (claude, codex, or gemini).
    pub ai_type: String,
    /// Task description/prompt for the AI.
    pub task: String,
    /// Optional provider name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TaskCommandResult {
    /// Full command string to execute.
    pub command: String,
    /// AI CLI type.
    pub ai_type: String,
    /// Optional provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Clone)]
pub struct AgenticWardenMcpServer {
    router: Arc<IntelligentRouter>,
    tool_router: ToolRouter<Self>,
    embedder: Arc<tokio::sync::Mutex<TextEmbedding>>,
    history_store: Arc<ConversationHistoryStore>,
    // Client capability detection
    client_capabilities: Arc<RwLock<Option<ClientCapabilities>>>,
    // Dynamic tool registration manager
    dynamic_tools: DynamicToolManager,
    // Store peer for sending notifications
    peer: Arc<RwLock<Option<rmcp::service::Peer<RoleServer>>>>,
}

#[rmcp::tool_router(router = tool_router)]
impl AgenticWardenMcpServer {
    pub async fn bootstrap() -> Result<Self, String> {
        let router = IntelligentRouter::initialize()
            .await
            .map_err(|e| format!("Failed to initialise intelligent router: {e}"))?;

        // Initialize FastEmbed for conversation search
        let embedder = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(false)
        )
        .map_err(|e| format!("Failed to initialize FastEmbed: {e}"))?;

        // Initialize conversation history store
        let db_path = Self::get_history_db_path()
            .map_err(|e| format!("Failed to get history DB path: {e}"))?;
        let history_store = ConversationHistoryStore::new(&db_path, 384)
            .map_err(|e| format!("Failed to initialize conversation history store: {e}"))?;

        Ok(Self {
            router: Arc::new(router),
            tool_router: Self::tool_router(),
            embedder: Arc::new(tokio::sync::Mutex::new(embedder)),
            history_store: Arc::new(history_store),
            client_capabilities: Arc::new(RwLock::new(None)),
            dynamic_tools: DynamicToolManager::new(),
            peer: Arc::new(RwLock::new(None)),
        })
    }

    fn get_history_db_path() -> Result<PathBuf, String> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Failed to get config directory".to_string())?
            .join("agentic-warden");

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;

        Ok(config_dir.join("conversation_history.db"))
    }

    #[tool(
        name = "intelligent_route",
        description = "Route user request to best MCP tool. Returns tool selection (LLM/vector). Auto-chooses execution mode (dynamic/query) based on client capabilities."
    )]
    pub async fn intelligent_route_tool(
        &self,
        params: Parameters<IntelligentRouteRequest>,
    ) -> Result<Json<IntelligentRouteResponse>, String> {
        use crate::mcp_routing::models::ExecutionMode;

        let mut request = params.0;

        // Auto-select execution mode based on client capabilities
        // (only if not explicitly overridden by caller)
        if request.execution_mode == ExecutionMode::Dynamic {
            if let Some(caps) = self.client_capabilities.read().await.as_ref() {
                if !caps.supports_dynamic_tools {
                    // Client doesn't support dynamic registration, use query mode
                    request.execution_mode = ExecutionMode::Query;
                    eprintln!("   ⚠️  Switching to Query mode (client doesn't support dynamic tools)");
                }
            }
        }

        let mut response = self.router
            .intelligent_route(request.clone())
            .await
            .map_err(|err| err.to_string())?;

        // Handle dynamic registration mode
        if request.execution_mode == ExecutionMode::Dynamic {
            if let Some(ref selected) = response.selected_tool {
                // Get the tool schema
                let schema_response = self.router
                    .get_method_schema(&selected.mcp_server, &selected.tool_name)
                    .await
                    .map_err(|err| err.to_string())?;

                if let Some(schema) = schema_response.schema {
                    // Register the tool dynamically
                    let description = schema_response.description
                        .unwrap_or_else(|| selected.rationale.clone());

                    let is_new = self.dynamic_tools.register_tool(
                        selected.mcp_server.clone(),
                        selected.tool_name.clone(),
                        description.clone(),
                        schema.clone(),
                    ).await;

                    // Send notification if this is a new tool
                    if is_new {
                        eprintln!("📝 Dynamically registered tool: {}", selected.tool_name);

                        // Send ToolListChangedNotification to client
                        if let Some(peer) = self.peer.read().await.as_ref() {
                            // Note: Notification sending disabled due to rmcp API constraints
                            // The client should re-query tools after receiving intelligent_route response
                            eprintln!("   📝 Tool '{}' registered - client should re-query tool list", selected.tool_name);
                        }
                    }

                    response.tool_schema = Some(schema);
                    response.dynamically_registered = true;
                    response.message = format!(
                        "Tool '{}' registered. Call it directly with full context for accurate parameters.",
                        selected.tool_name
                    );
                }
            }
        }

        Ok(Json(response))
    }

    #[tool(
        name = "get_method_schema",
        description = "Return the JSON schema for a given MCP server tool."
    )]
    pub async fn get_method_schema_tool(
        &self,
        params: Parameters<MethodSchemaParams>,
    ) -> Result<Json<MethodSchemaResponse>, String> {
        self.router
            .get_method_schema(&params.0.mcp_server, &params.0.tool_name)
            .await
            .map(Json)
            .map_err(|err| err.to_string())
    }

    #[tool(
        name = "search_history",
        description = "Search conversation history using semantic similarity. Returns relevant past conversations with extracted TODO items. Each result includes conversation context and associated TODO list (markdown checkboxes, TODO: markers, Action Items)."
    )]
    pub async fn search_history_tool(
        &self,
        params: Parameters<SearchHistoryParams>,
    ) -> Result<Json<Vec<ConversationSearchResult>>, String> {
        // Generate embedding for the query
        let query = &params.0.query;
        let limit = params.0.limit;

        let embeddings = self
            .embedder
            .lock()
            .await
            .embed(vec![query.clone()], None)
            .map_err(|e| format!("Failed to generate embedding: {e}"))?;

        let query_embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| "No embedding generated".to_string())?;

        // Search conversation history
        let results = self
            .history_store
            .search_with_scores(query_embedding, limit)
            .map_err(|e| format!("Failed to search conversation history: {e}"))?;

        Ok(Json(results))
    }

    #[tool(
        name = "execute_tool",
        description = "Execute a specific MCP tool with confirmed parameters. Used in two-phase negotiation mode."
    )]
    pub async fn execute_tool_mcp(
        &self,
        params: Parameters<ExecuteToolRequest>,
    ) -> Result<Json<ExecuteToolResponse>, String> {
        self.router
            .execute_tool(params.0)
            .await
            .map(Json)
            .map_err(|err| err.to_string())
    }

    #[tool(
        name = "start_concurrent_tasks",
        description = "Launch multiple AI CLI tasks concurrently. Returns process IDs and log file paths for each task."
    )]
    pub async fn start_concurrent_tasks_tool(
        &self,
        params: Parameters<StartConcurrentTasksParams>,
    ) -> Result<Json<Vec<TaskLaunchResult>>, String> {
        use crate::cli_type::parse_cli_type;
        use crate::registry_factory::create_mcp_registry;
        use crate::supervisor;
        use std::ffi::OsString;

        let registry = create_mcp_registry();

        let mut handles = Vec::new();

        // Launch all tasks concurrently using tokio::spawn
        for task_spec in params.0.tasks {
            let registry_clone = registry.clone();
            let handle = tokio::spawn(async move {
                // Parse AI CLI type
                let cli_type = parse_cli_type(&task_spec.ai_type)
                    .ok_or_else(|| format!("Invalid AI type: {}. Must be claude, codex, or gemini", task_spec.ai_type))?;

                // Build command arguments
                let cli_args = cli_type.build_full_access_args(&task_spec.task);
                let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

                // Launch task via supervisor (this will wait for completion in background)
                let _exit_code = supervisor::execute_cli(&registry_clone, &cli_type, &os_args, task_spec.provider.clone())
                    .await
                    .map_err(|e| format!("Failed to launch {} task: {}", task_spec.ai_type, e))?;

                // Note: For now we can't easily get PID and log_file from execute_cli
                // This is a limitation of the current supervisor design
                // Return placeholder result
                Ok::<TaskLaunchResult, String>(TaskLaunchResult {
                    pid: 0, // Placeholder - actual PID tracked in registry
                    ai_type: task_spec.ai_type.clone(),
                    task: task_spec.task.clone(),
                    provider: task_spec.provider.clone(),
                    log_file: format!("/tmp/mcp-task-{}.log", task_spec.ai_type),
                })
            });
            handles.push(handle);
        }

        // Wait for all tasks to start (but not complete)
        // Note: This is still blocking on task spawn, but tasks run in background
        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await
                .map_err(|e| format!("Task spawn failed: {}", e))??;
            results.push(result);
        }

        Ok(Json(results))
    }

    #[tool(
        name = "get_task_command",
        description = "Get the command string to launch a single AI CLI task. Returns the full command without executing it."
    )]
    pub async fn get_task_command_tool(
        &self,
        params: Parameters<GetTaskCommandParams>,
    ) -> Result<Json<TaskCommandResult>, String> {
        use crate::cli_type::parse_cli_type;

        // Validate AI CLI type
        let _cli_type = parse_cli_type(&params.0.ai_type)
            .ok_or_else(|| format!("Invalid AI type: {}. Must be claude, codex, or gemini", params.0.ai_type))?;

        // Build command string
        let mut command_parts = vec!["agentic-warden".to_string(), params.0.ai_type.clone()];

        if let Some(ref provider) = params.0.provider {
            command_parts.push("-p".to_string());
            command_parts.push(provider.clone());
        }

        // Escape single quotes in task
        let escaped_task = params.0.task.replace("'", "'\\''");
        command_parts.push(format!("'{}'", escaped_task));

        let command = command_parts.join(" ");

        Ok(Json(TaskCommandResult {
            command,
            ai_type: params.0.ai_type,
            provider: params.0.provider,
        }))
    }


    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🚀 Agentic-Warden intelligent MCP router ready (stdio transport)");
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        self.serve(transport).await?.waiting().await?;
        Ok(())
    }
}

impl ServerHandler for AgenticWardenMcpServer {
    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        // Get base tools from tool_router
        let mut tools: Vec<rmcp::model::Tool> = self.tool_router.list_all();

        // Add dynamically registered tools
        let dynamic_tools = self.dynamic_tools.list_tools().await;
        tools.extend(dynamic_tools);

        Ok(rmcp::model::ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        // First, try to call base tools via tool_router
        if self.tool_router.has_route(&request.name) {
            // This is a base tool, delegate to tool_router
            let tool_context = ToolCallContext::new(self, request, context);
            return self.tool_router.call(tool_context).await;
        }

        // Check if this is a dynamically registered tool
        if let Some(mcp_server) = self.dynamic_tools.get_server(&request.name).await {
            // Proxy call to the underlying MCP server
            let result = self
                .router
                .execute_tool(crate::mcp_routing::models::ExecuteToolRequest {
                    mcp_server,
                    tool_name: request.name.to_string(),
                    arguments: serde_json::Value::Object(request.arguments.unwrap_or_default()),
                    session_id: None,
                })
                .await
                .map_err(|e| {
                    rmcp::ErrorData::internal_error(format!("Tool execution failed: {}", e), None)
                })?;

            if result.success {
                let content_str = result.result.as_ref()
                    .map(|r| serde_json::to_string_pretty(&r.output).unwrap_or_default())
                    .unwrap_or_default();
                let structured = result.result.map(|r| r.output);

                Ok(rmcp::model::CallToolResult {
                    content: vec![rmcp::model::Content::text(content_str)],
                    structured_content: structured,
                    is_error: None,
                    meta: None,
                })
            } else {
                Err(rmcp::ErrorData::internal_error(result.message, None))
            }
        } else {
            // Tool not found in either base or dynamic tools
            Err(rmcp::ErrorData::method_not_found::<rmcp::model::CallToolRequestMethod>())
        }
    }

    async fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, rmcp::ErrorData> {
        // Create initial capabilities (before testing)
        let mut capabilities = ClientCapabilities::from_init_request(&request);

        eprintln!("🔌 MCP Client connected:");
        eprintln!("   Name: {}", capabilities.client_name);
        eprintln!("   Version: {}", capabilities.client_version);
        eprintln!("   Testing dynamic tools support...");

        // Clone Arc for background task
        let client_capabilities = Arc::clone(&self.client_capabilities);
        let peer = context.peer.clone();

        // Spawn background task to test dynamic tools support
        // We delay a bit to allow the MCP initialization handshake to complete
        tokio::spawn(async move {
            // Wait for initialization to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Test if client supports dynamic tool registration
            let supports = ClientCapabilities::test_dynamic_tools_support(&peer).await;

            // Update capabilities
            if let Some(caps) = client_capabilities.write().await.as_mut() {
                caps.supports_dynamic_tools = supports;

                eprintln!("   Mode: {}",
                    if supports {
                        "✅ Dynamic registration (primary mode)"
                    } else {
                        "⚠️  Two-phase negotiation (fallback mode)"
                    }
                );
            }
        });

        // Save initial capabilities
        *self.client_capabilities.write().await = Some(capabilities);

        // Save peer for sending notifications later
        *self.peer.write().await = Some(context.peer.clone());

        // Return server info and capabilities
        Ok(InitializeResult {
            protocol_version: request.protocol_version,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            server_info: Implementation {
                name: "agentic-warden".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Agentic Warden MCP Server".to_string()),
                icons: None,
                website_url: Some("https://github.com/putao520/agentic-warden".to_string()),
            },
            instructions: None,
        })
    }
}
