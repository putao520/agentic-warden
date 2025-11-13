use agentic_warden::mcp_routing::{
    models::{IntelligentRouteRequest, IntelligentRouteResponse, MethodSchemaResponse},
    IntelligentRouter,
};
use std::future::Future;
use rmcp::{
    handler::server::tool::{Parameters, ToolRouter},
    handler::server::ServerHandler,
    tool, tool_handler, tool_router, Json, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MethodSchemaParams {
    pub mcp_server: String,
    pub tool_name: String,
}

#[derive(Clone)]
pub struct AgenticWardenMcpServer {
    router: Arc<IntelligentRouter>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AgenticWardenMcpServer {
    pub async fn bootstrap() -> Result<Self, String> {
        let router = IntelligentRouter::initialize()
            .await
            .map_err(|e| format!("Failed to initialise intelligent router: {e}"))?;
        Ok(Self {
            router: Arc::new(router),
            tool_router: Self::tool_router(),
        })
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

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🚀 Agentic-Warden intelligent MCP router ready (stdio transport)");
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        self.serve(transport).await?.waiting().await?;
        Ok(())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for AgenticWardenMcpServer {}
