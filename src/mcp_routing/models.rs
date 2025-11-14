use crate::memory::ConversationRecord;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Routing mode determines how the intelligent router behaves.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum RouteMode {
    /// Auto-execute the selected tool and return result (default behavior).
    #[default]
    Auto,
    /// Register tool dynamically and return prompt (for dynamic registration).
    Dynamic,
    /// Only return tool suggestions without executing (for two-phase negotiation).
    Query,
}

#[derive(Debug, Clone)]
pub struct ToolVectorRecord {
    pub id: String,
    pub server: String,
    pub tool_name: String,
    pub description: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct MethodVectorRecord {
    pub id: String,
    pub server: String,
    pub tool_name: String,
    pub description: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IntelligentRouteRequest {
    pub user_request: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub max_candidates: Option<usize>,
    #[serde(default)]
    pub mode: RouteMode,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IntelligentRouteResponse {
    pub success: bool,
    pub confidence: f32,
    pub message: String,
    pub selected_tool: Option<SelectedRoute>,
    pub result: Option<RouteExecutionResult>,
    #[serde(default)]
    pub alternatives: Vec<SelectedRoute>,
    #[serde(default)]
    #[schemars(skip)]
    pub conversation_context: Vec<ConversationRecord>,
    /// Tool schema when dynamically registered (Dynamic mode only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_schema: Option<Value>,
    /// Indicates if a tool was dynamically registered
    #[serde(default)]
    pub dynamically_registered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectedRoute {
    pub mcp_server: String,
    pub tool_name: String,
    pub arguments: Value,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RouteExecutionResult {
    pub mcp_server: String,
    pub tool_name: String,
    pub duration_ms: u128,
    pub output: Value,
    #[serde(default)]
    pub raw_stdout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MethodSchemaResponse {
    pub success: bool,
    pub schema: Option<Value>,
    pub description: Option<String>,
    pub annotations: Option<Value>,
    pub message: Option<String>,
}

impl Default for IntelligentRouteRequest {
    fn default() -> Self {
        Self {
            user_request: String::new(),
            session_id: None,
            max_candidates: None,
            metadata: HashMap::new(),
        }
    }
}
