pub mod config;
mod decision;
mod embedding;
mod index;
pub mod js_orchestrator; // REQ-013: JS orchestration
pub mod models;
mod pool;
pub mod registry; // REQ-013: Dynamic tool registry

pub use embedding::{FastEmbedder, MockEmbeddingBackend};
pub use index::{MemRoutingIndex, MethodEmbedding, ToolEmbedding};
pub use pool::McpConnectionPool;

pub use decision::{CandidateToolInfo, DecisionEngine, DecisionInput, DecisionOutcome, LlmClient};

use self::{
    config::McpConfigManager,
    index::{ScoredMethod, ScoredTool},
    models::{
        ExecuteToolRequest, ExecuteToolResponse, IntelligentRouteRequest, IntelligentRouteResponse,
        MethodSchemaResponse, RouteExecutionResult, SelectedRoute, ToolVectorRecord,
    },
    pool::DiscoveredTool,
};
use crate::memory::{ConversationHistoryStore, ConversationRecord, MemoryConfig};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::sync::RwLock;

const METHOD_VECTOR_PREFIX: &str = "method";

pub struct IntelligentRouter {
    routing: config::RoutingConfig,
    embedder: FastEmbedder,
    index: Mutex<MemRoutingIndex>,
    history: ConversationHistoryStore,
    decision_engine: Arc<DecisionEngine>,
    connection_pool: Arc<McpConnectionPool>,
    tool_registry: RwLock<HashMap<String, Tool>>,
    dynamic_registry: Option<Arc<registry::DynamicToolRegistry>>, // REQ-013
    js_orchestrator: Option<Arc<js_orchestrator::WorkflowOrchestrator>>, // REQ-013
}

impl IntelligentRouter {
    pub async fn initialize() -> Result<Self> {
        let config_manager = McpConfigManager::load()?;
        let config_arc = Arc::new(config_manager.config().clone());
        let routing = config_arc.routing.clone();

        let memory_config = MemoryConfig::load_from_provider_config()?;
        memory_config.validate()?;

        let embedder = FastEmbedder::new(&memory_config.fastembed_model)?;
        let history =
            ConversationHistoryStore::new(&memory_config.sahome_db_path, embedder.dimension())?;

        let decision_endpoint = config_arc
            .llm
            .endpoint
            .as_deref()
            .unwrap_or(&memory_config.llm_endpoint)
            .to_string();
        let decision_model = config_arc
            .llm
            .model
            .as_deref()
            .unwrap_or(&memory_config.llm_model)
            .to_string();
        let decision_timeout = config_arc.llm.timeout.unwrap_or(30);
        let decision_engine = Arc::new(DecisionEngine::new(
            &decision_endpoint,
            &decision_model,
            decision_timeout,
        )?);

        let dynamic_registry = Arc::new(registry::DynamicToolRegistry::new(Vec::new()));
        let _cleanup_task = dynamic_registry.start_cleanup_task();

        // Enable orchestrator if LLM endpoint is explicitly configured
        // Priority: config file > environment variable
        // Only enable if user explicitly sets either one (not using defaults)
        let llm_explicitly_configured = config_arc.llm.endpoint.is_some()
            || std::env::var("AGENTIC_WARDEN_LLM_ENDPOINT").is_ok();

        let js_orchestrator = if llm_explicitly_configured {
            eprintln!("🤖 LLM orchestration enabled: {}", decision_endpoint);
            Some(Arc::new(js_orchestrator::WorkflowOrchestrator::new(
                decision_engine.clone(),
            )))
        } else {
            eprintln!("🔍 LLM orchestration disabled, vector-only mode");
            None
        };

        let mut index = MemRoutingIndex::new(embedder.dimension())?;
        let connection_pool = Arc::new(McpConnectionPool::new(config_arc.clone()));
        let discovered = connection_pool.warm_up().await?;
        let tool_registry = RwLock::new(HashMap::new());
        let embeddings = build_embeddings(&embedder, &discovered, config_arc.as_ref())?;
        index.rebuild(&embeddings.tools, &embeddings.methods)?;

        populate_registry(&tool_registry, discovered).await;

        Ok(Self {
            routing,
            embedder,
            index: Mutex::new(index),
            history,
            decision_engine,
            connection_pool,
            tool_registry,
            dynamic_registry: Some(dynamic_registry),
            js_orchestrator,
        })
    }

    /// Build a router from explicit dependencies (used for deterministic testing).
    pub fn new_with_components(
        routing: config::RoutingConfig,
        embedder: FastEmbedder,
        index: MemRoutingIndex,
        history: ConversationHistoryStore,
        decision_engine: Arc<DecisionEngine>,
        connection_pool: Arc<McpConnectionPool>,
        tool_registry: RwLock<HashMap<String, Tool>>,
        dynamic_registry: Option<Arc<registry::DynamicToolRegistry>>,
        js_orchestrator: Option<Arc<js_orchestrator::WorkflowOrchestrator>>,
    ) -> Self {
        Self {
            routing,
            embedder,
            index: Mutex::new(index),
            history,
            decision_engine,
            connection_pool,
            tool_registry,
            dynamic_registry,
            js_orchestrator,
        }
    }

    pub async fn intelligent_route(
        &self,
        request: IntelligentRouteRequest,
    ) -> Result<IntelligentRouteResponse> {
        if request.user_request.trim().is_empty() {
            return Ok(IntelligentRouteResponse {
                success: false,
                message: "user_request cannot be empty".into(),
                confidence: 0.0,
                selected_tool: None,
                result: None,
                alternatives: Vec::new(),
                conversation_context: Vec::new(),
                tool_schema: None,
                dynamically_registered: false,
            });
        }

        let embed = self.embedder.embed(&request.user_request)?;

        match self.js_orchestrator.as_ref() {
            None => {
                eprintln!("🔍 LLM not configured, using vector search mode");
                self.vector_mode(&request, &embed).await
            }
            Some(orchestrator) => {
                eprintln!("🤖 Trying LLM orchestration mode...");
                match self
                    .try_orchestrate(orchestrator.as_ref(), &request, &embed)
                    .await
                {
                    Ok(response) => {
                        eprintln!("✅ LLM orchestration succeeded");
                        Ok(response)
                    }
                    Err(err) => {
                        eprintln!("⚠️  LLM failed: {}, falling back to vector mode", err);
                        self.vector_mode(&request, &embed).await
                    }
                }
            }
        }
    }

    /// Execute the vector-search routing pipeline when LLM orchestration is unavailable.
    async fn vector_mode(
        &self,
        request: &IntelligentRouteRequest,
        embed: &[f32],
    ) -> Result<IntelligentRouteResponse> {
        let max_tools = request
            .max_candidates
            .unwrap_or(self.routing.max_tools_per_request);

        let (tool_scores, method_scores) = {
            let index = self.index.lock();
            let tools = index.search_tools(embed, max_tools)?;
            let methods = index.search_methods(embed, max_tools * 2)?;
            (tools, methods)
        };

        if tool_scores.is_empty() {
            return Ok(IntelligentRouteResponse {
                success: false,
                message: "No MCP tools matched the request".into(),
                confidence: 0.0,
                selected_tool: None,
                result: None,
                alternatives: Vec::new(),
                conversation_context: Vec::new(),
                tool_schema: None,
                dynamically_registered: false,
            });
        }

        let candidate_infos = build_candidates(&tool_scores, &method_scores);
        let mut conversation_context = Vec::new();
        if request.session_id.is_some() {
            conversation_context = self.history.top_conversations(embed.to_vec(), 5)?;
        }

        let decision = self
            .decision_engine
            .decide(DecisionInput {
                user_request: request.user_request.clone(),
                candidates: candidate_infos.clone(),
                conversation: conversation_context.clone(),
            })
            .await?;

        let execute_message = match request.execution_mode {
            models::ExecutionMode::Dynamic => {
                format!(
                    "Selected tool: {}::{} (will be dynamically registered)",
                    decision.server, decision.tool
                )
            }
            models::ExecutionMode::Query => {
                format!(
                    "Suggested tool: {}::{} (review and call execute_tool)",
                    decision.server, decision.tool
                )
            }
        };

        if let Some(session) = request.session_id.as_ref() {
            let record = ConversationRecord::new(
                Some(session.clone()),
                "user",
                request.user_request.clone(),
                vec![format!("{}::{}", decision.server, decision.tool)],
            );
            self.history.append(record, embed.to_vec())?;
        }

        Ok(IntelligentRouteResponse {
            success: true,
            confidence: decision.confidence,
            message: execute_message,
            selected_tool: Some(SelectedRoute {
                mcp_server: decision.server.clone(),
                tool_name: decision.tool.clone(),
                arguments: decision.arguments,
                rationale: decision.rationale.clone(),
            }),
            result: None,
            alternatives: candidate_infos
                .into_iter()
                .skip(1)
                .take(2)
                .map(|cand| SelectedRoute {
                    mcp_server: cand.server,
                    tool_name: cand.tool,
                    arguments: Value::Null,
                    rationale: cand.description,
                })
                .collect(),
            conversation_context,
            tool_schema: None,
            dynamically_registered: false,
        })
    }

    /// Attempt to orchestrate a workflow via the JS orchestrator (LLM-first path).
    async fn try_orchestrate(
        &self,
        orchestrator: &js_orchestrator::WorkflowOrchestrator,
        request: &IntelligentRouteRequest,
        embed: &[f32],
    ) -> Result<IntelligentRouteResponse> {
        let (tool_scores, method_scores) = {
            let index = self.index.lock();
            let max_tools = request
                .max_candidates
                .unwrap_or(self.routing.max_tools_per_request);
            let tools = index.search_tools(embed, max_tools)?;
            let methods = index.search_methods(embed, max_tools * 2)?;
            (tools, methods)
        };

        if tool_scores.is_empty() {
            return Err(anyhow!("No candidate tools for orchestration"));
        }

        let candidate_infos = build_candidates(&tool_scores, &method_scores);
        let orchestrated_tool = orchestrator
            .orchestrate(&request.user_request, &candidate_infos)
            .await?;

        let Some(registry) = self.dynamic_registry.as_ref() else {
            return Err(anyhow!("Dynamic registry not initialized"));
        };

        registry
            .register_js_tool(
                orchestrated_tool.name.clone(),
                orchestrated_tool.description.clone(),
                orchestrated_tool.input_schema.clone(),
                orchestrated_tool.js_code.clone(),
            )
            .await?;

        Ok(IntelligentRouteResponse {
            success: true,
            message: format!(
                "Created orchestrated workflow '{}'. Use this tool to solve your request.",
                orchestrated_tool.name
            ),
            confidence: 1.0,
            selected_tool: Some(SelectedRoute {
                mcp_server: "orchestrated".into(),
                tool_name: orchestrated_tool.name.clone(),
                arguments: Value::Object(Default::default()),
                rationale: orchestrated_tool.description.clone(),
            }),
            result: None,
            alternatives: Vec::new(),
            conversation_context: Vec::new(),
            tool_schema: Some(orchestrated_tool.input_schema),
            dynamically_registered: true,
        })
    }

    pub async fn get_method_schema(
        &self,
        server: &str,
        tool: &str,
    ) -> Result<MethodSchemaResponse> {
        let registry = self.tool_registry.read().await;
        let key = registry_key(server, tool);
        let Some(definition) = registry.get(&key) else {
            return Ok(MethodSchemaResponse {
                success: false,
                schema: None,
                description: None,
                annotations: None,
                message: Some(format!("Unknown tool {server}::{tool}")),
            });
        };
        let schema = Value::Object((*definition.input_schema).clone());
        let annotations = definition
            .annotations
            .as_ref()
            .map(|ann| serde_json::to_value(ann).unwrap_or(json!({})));
        Ok(MethodSchemaResponse {
            success: true,
            schema: Some(schema),
            description: definition.description.as_ref().map(|d| d.to_string()),
            annotations,
            message: None,
        })
    }

    /// Execute a specific tool with confirmed parameters.
    /// Used in two-phase negotiation mode (fallback for clients without dynamic registration).
    pub async fn execute_tool(&self, request: ExecuteToolRequest) -> Result<ExecuteToolResponse> {
        let start = Instant::now();
        let execution = self
            .connection_pool
            .call_tool(
                &request.mcp_server,
                &request.tool_name,
                request.arguments.clone(),
            )
            .await;
        let duration = start.elapsed().as_millis();

        match execution {
            Ok(output) => Ok(ExecuteToolResponse {
                success: true,
                message: "Tool executed successfully".to_string(),
                result: Some(RouteExecutionResult {
                    mcp_server: request.mcp_server,
                    tool_name: request.tool_name,
                    duration_ms: duration,
                    output,
                    raw_stdout: None,
                }),
            }),
            Err(err) => Ok(ExecuteToolResponse {
                success: false,
                message: format!("Tool execution failed: {err}"),
                result: None,
            }),
        }
    }

    pub fn connection_pool(&self) -> Arc<McpConnectionPool> {
        Arc::clone(&self.connection_pool)
    }
}

struct PreparedEmbeddings {
    tools: Vec<ToolEmbedding>,
    methods: Vec<MethodEmbedding>,
}

fn build_embeddings(
    embedder: &FastEmbedder,
    tools: &[DiscoveredTool],
    config: &config::McpConfig,
) -> Result<PreparedEmbeddings> {
    let mut tool_embeddings = Vec::new();
    let mut method_embeddings = Vec::new();

    for tool in tools {
        let category = config
            .mcp_servers
            .get(&tool.server)
            .and_then(|cfg| cfg.category.clone())
            .unwrap_or_else(|| "uncategorized".into());

        let description = tool
            .definition
            .description
            .as_deref()
            .unwrap_or("No description provided")
            .to_string();
        let schema_value = Value::Object((*tool.definition.input_schema).clone());
        let schema_string = schema_value.to_string();

        let doc = format!(
            "{server}::{tool}\nCategory: {category}\nDescription: {description}\nInput Schema: {schema}",
            server = tool.server,
            tool = tool.definition.name,
            category = category,
            description = description,
            schema = schema_string
        );
        let vector = embedder.embed(&doc)?;
        let mut metadata = HashMap::new();
        metadata.insert("server".into(), tool.server.clone());
        metadata.insert("tool".into(), tool.definition.name.to_string());
        metadata.insert("description".into(), description.clone());
        metadata.insert("category".into(), category);
        metadata.insert("schema".into(), schema_string.clone());

        tool_embeddings.push(ToolEmbedding {
            record: ToolVectorRecord {
                id: format!("{}::{}", tool.server, tool.definition.name),
                server: tool.server.clone(),
                tool_name: tool.definition.name.to_string(),
                description: description.clone(),
                metadata: metadata.clone(),
            },
            vector: vector.clone(),
        });

        method_embeddings.push(MethodEmbedding {
            record: crate::mcp_routing::models::MethodVectorRecord {
                id: format!(
                    "{METHOD_VECTOR_PREFIX}::{}::{}",
                    tool.server, tool.definition.name
                ),
                server: tool.server.clone(),
                tool_name: tool.definition.name.to_string(),
                description: description,
                metadata,
            },
            vector,
        });
    }

    Ok(PreparedEmbeddings {
        tools: tool_embeddings,
        methods: method_embeddings,
    })
}

async fn populate_registry(registry: &RwLock<HashMap<String, Tool>>, tools: Vec<DiscoveredTool>) {
    let mut guard = registry.write().await;
    guard.clear();
    for tool in tools {
        guard.insert(
            registry_key(&tool.server, &tool.definition.name),
            tool.definition,
        );
    }
}

fn build_candidates(tools: &[ScoredTool], methods: &[ScoredMethod]) -> Vec<CandidateToolInfo> {
    let method_map: HashMap<String, &ScoredMethod> = methods
        .iter()
        .map(|method| (registry_key(&method.server, &method.tool), method))
        .collect();

    tools
        .iter()
        .map(|tool| {
            let key = registry_key(&tool.server, &tool.tool);
            let schema = method_map
                .get(&key)
                .and_then(|method| method.metadata.get("schema").cloned());
            CandidateToolInfo {
                server: tool.server.clone(),
                tool: tool.tool.clone(),
                description: tool.description.clone().unwrap_or_default(),
                schema_snippet: schema,
            }
        })
        .collect()
}

fn registry_key(server: &str, tool: &str) -> String {
    format!("{server}::{tool}")
}
