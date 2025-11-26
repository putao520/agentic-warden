use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Execution mode for intelligent routing (automatically chosen based on client capabilities).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Register tool dynamically for client to call (when client supports dynamic tools).
    #[default]
    Dynamic,
    /// Return suggestions for two-phase negotiation (fallback for legacy clients).
    Query,
}

/// Decision engine mode (LLM ReAct vs Vector Search).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DecisionMode {
    /// Auto-select based on LLM endpoint availability (default).
    #[default]
    Auto,
    /// Force use of LLM ReAct for decision making.
    LlmReact,
    /// Force use of vector search for decision making.
    Vector,
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
    /// Decision engine to use (auto/llm/vector). Auto selects based on LLM endpoint availability.
    #[serde(default)]
    pub decision_mode: DecisionMode,
    /// Execution mode (dynamic/query). Usually auto-selected based on client capabilities.
    #[serde(default)]
    pub execution_mode: ExecutionMode,
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

/// Request to execute a specific tool with confirmed parameters.
/// Used in two-phase negotiation mode after AI reviews the suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteToolRequest {
    pub mcp_server: String,
    pub tool_name: String,
    pub arguments: Value,
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Response from executing a specific tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteToolResponse {
    pub success: bool,
    pub message: String,
    pub result: Option<RouteExecutionResult>,
}

impl Default for IntelligentRouteRequest {
    fn default() -> Self {
        Self {
            user_request: String::new(),
            session_id: None,
            max_candidates: None,
            decision_mode: DecisionMode::Auto,
            execution_mode: ExecutionMode::Dynamic,
            metadata: HashMap::new(),
        }
    }
}
