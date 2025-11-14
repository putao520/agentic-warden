mod capability_detector;

use agentic_warden::mcp_routing::{
    models::{IntelligentRouteRequest, IntelligentRouteResponse, MethodSchemaResponse},
    IntelligentRouter,
};
use agentic_warden::memory::{ConversationHistoryStore, ConversationSearchResult};
use capability_detector::ClientCapabilities;
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
        description = "Route the user request to the best MCP tool and execute it automatically."
    )]
    pub async fn intelligent_route_tool(
        &self,
        params: Parameters<IntelligentRouteRequest>,
    ) -> Result<Json<IntelligentRouteResponse>, String> {
        self.router
            .intelligent_route(params.0)
            .await
            .map(Json)
            .map_err(|err| err.to_string())
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

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🚀 Agentic-Warden intelligent MCP router ready (stdio transport)");
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        self.serve(transport).await?.waiting().await?;
        Ok(())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for AgenticWardenMcpServer {
    async fn initialize(
        &self,
        request: InitializeRequestParam,
        _context: rmcp::handler::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<InitializeResult, rmcp::McpError> {
        // Detect client capabilities
        let capabilities = ClientCapabilities::from_init_request(&request);

        eprintln!("🔌 MCP Client connected:");
        eprintln!("   Name: {}", capabilities.client_name);
        eprintln!("   Version: {}", capabilities.client_version);
        eprintln!("   Dynamic tools: {}",
            if capabilities.supports_dynamic_tools {
                "✅ Supported (using dynamic registration)"
            } else {
                "⚠️  Not supported (using fallback mode)"
            }
        );

        // Save capabilities
        *self.client_capabilities.write().await = Some(capabilities);

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
