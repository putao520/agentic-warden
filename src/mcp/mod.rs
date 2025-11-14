mod capability_detector;
mod dynamic_tools;

use agentic_warden::mcp_routing::{
    models::{
        ExecuteToolRequest, ExecuteToolResponse, IntelligentRouteRequest,
        IntelligentRouteResponse, MethodSchemaResponse,
    },
    IntelligentRouter,
};
use agentic_warden::memory::{ConversationHistoryStore, ConversationSearchResult};
use capability_detector::ClientCapabilities;
use dynamic_tools::DynamicToolManager;
use std::future::Future;
use rmcp::{
    handler::server::tool::{Parameters, ToolRouter},
    handler::server::ServerHandler,
    model::{Implementation, InitializeRequestParam, InitializeResult, ServerCapabilities},
    tool, tool_handler, tool_router, Json, ServiceExt,
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

#[derive(Clone)]
pub struct AgenticWardenMcpServer {
    router: Arc<IntelligentRouter>,
    tool_router: ToolRouter<Self>,
    embedder: Arc<TextEmbedding>,
    history_store: Arc<ConversationHistoryStore>,
    // Client capability detection
    client_capabilities: Arc<RwLock<Option<ClientCapabilities>>>,
    // Dynamic tool registration manager
    dynamic_tools: DynamicToolManager,
    // Store peer for sending notifications
    peer: Arc<RwLock<Option<rmcp::service::server::ClientSink>>>,
}

#[tool_router]
impl AgenticWardenMcpServer {
    pub async fn bootstrap() -> Result<Self, String> {
        let router = IntelligentRouter::initialize()
            .await
            .map_err(|e| format!("Failed to initialise intelligent router: {e}"))?;

        // Initialize FastEmbed for conversation search
        let embedder = TextEmbedding::try_new(InitOptions {
            model_name: EmbeddingModel::AllMiniLML6V2,
            show_download_progress: false,
            ..Default::default()
        })
        .map_err(|e| format!("Failed to initialize FastEmbed: {e}"))?;

        // Initialize conversation history store
        let db_path = Self::get_history_db_path()
            .map_err(|e| format!("Failed to get history DB path: {e}"))?;
        let history_store = ConversationHistoryStore::new(&db_path, 384)
            .map_err(|e| format!("Failed to initialize conversation history store: {e}"))?;

        Ok(Self {
            router: Arc::new(router),
            tool_router: Self::tool_router(),
            embedder: Arc::new(embedder),
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
                            let notification = rmcp::model::ToolListChangedNotification::default();
                            if let Err(e) = peer.send_notification(notification.into()).await {
                                eprintln!("   ⚠️  Failed to send ToolListChangedNotification: {}", e);
                            } else {
                                eprintln!("   ✅ Sent ToolListChangedNotification to client");
                            }
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
        description = "Search conversation history using semantic similarity. Returns relevant past conversations based on the query."
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

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🚀 Agentic-Warden intelligent MCP router ready (stdio transport)");
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        self.serve(transport).await?.waiting().await?;
        Ok(())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for AgenticWardenMcpServer {
    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::handler::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::McpError> {
        // Get base tools from tool_router
        let mut tools: Vec<rmcp::model::Tool> = self.tool_router.tools().to_vec();

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
        _context: rmcp::handler::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::McpError> {
        // Check if this is a dynamically registered tool
        if let Some(mcp_server) = self.dynamic_tools.get_server(&request.name).await {
            // Proxy call to the underlying MCP server
            let result = self
                .router
                .execute_tool(crate::mcp_routing::models::ExecuteToolRequest {
                    mcp_server,
                    tool_name: request.name.clone(),
                    arguments: request.arguments.unwrap_or(serde_json::Value::Object(Default::default())),
                    session_id: None,
                })
                .await
                .map_err(|e| rmcp::McpError::internal_error(&e.to_string()))?;

            if result.success {
                Ok(rmcp::model::CallToolResult {
                    content: vec![rmcp::model::Content::text(
                        serde_json::to_string_pretty(&result.result).unwrap_or_default()
                    )],
                    is_error: None,
                })
            } else {
                Err(rmcp::McpError::internal_error(&result.message))
            }
        } else {
            // Not a dynamic tool, shouldn't happen (base tools handled by tool_router)
            Err(rmcp::McpError::method_not_found::<rmcp::model::CallToolRequestMethod>())
        }
    }

    async fn initialize(
        &self,
        request: InitializeRequestParam,
        context: rmcp::handler::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<InitializeResult, rmcp::McpError> {
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
            },
            instructions: None,
        })
    }
}
