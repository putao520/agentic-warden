pub mod config;
mod decision;
mod embedding;
mod index;
pub mod models;
mod pool;

use self::{
    config::McpConfigManager,
    decision::{CandidateToolInfo, DecisionEngine, DecisionInput},
    embedding::FastEmbedder,
    index::{MemRoutingIndex, MethodEmbedding, ScoredMethod, ScoredTool, ToolEmbedding},
    models::{
        IntelligentRouteRequest, IntelligentRouteResponse, MethodSchemaResponse,
        RouteExecutionResult, SelectedRoute, ToolVectorRecord,
    },
    pool::{DiscoveredTool, McpConnectionPool},
};
use crate::memory::{ConversationHistoryStore, ConversationRecord, MemoryConfig};
use anyhow::Result;
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
    decision_engine: DecisionEngine,
    connection_pool: McpConnectionPool,
    tool_registry: RwLock<HashMap<String, Tool>>,
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
        let decision_engine =
            DecisionEngine::new(&decision_endpoint, &decision_model, decision_timeout)?;

        let mut index = MemRoutingIndex::new(embedder.dimension())?;
        let connection_pool = McpConnectionPool::new(config_arc.clone());
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
        })
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
        let max_tools = request
            .max_candidates
            .unwrap_or(self.routing.max_tools_per_request);

        let (tool_scores, method_scores) = {
            let index = self.index.lock();
            let tools = index.search_tools(&embed, max_tools)?;
            let methods = index.search_methods(&embed, max_tools * 2)?;
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
            conversation_context = self.history.top_conversations(embed.clone(), 5)?;
        }

        let decision = self
            .decision_engine
            .decide(DecisionInput {
                user_request: request.user_request.clone(),
                candidates: candidate_infos.clone(),
                conversation: conversation_context.clone(),
            })
            .await?;

        use models::RouteMode;

        // For Query and Dynamic modes, don't execute the tool
        let (result, execute_message) = match request.mode {
            RouteMode::Auto => {
                // Execute the tool in auto mode
                let start = Instant::now();
                let execution = self
                    .connection_pool
                    .call_tool(&decision.server, &decision.tool, decision.arguments.clone())
                    .await;
                let duration = start.elapsed().as_millis();

                match execution {
                    Ok(output) => (
                        Some(RouteExecutionResult {
                            mcp_server: decision.server.clone(),
                            tool_name: decision.tool.clone(),
                            duration_ms: duration,
                            output,
                            raw_stdout: None,
                        }),
                        "Tool executed successfully".to_string(),
                    ),
                    Err(err) => {
                        return Ok(IntelligentRouteResponse {
                            success: false,
                            confidence: decision.confidence,
                            message: format!("Tool execution failed: {err}"),
                            selected_tool: None,
                            result: None,
                            alternatives: Vec::new(),
                            conversation_context,
                            tool_schema: None,
                            dynamically_registered: false,
                        });
                    }
                }
            }
            RouteMode::Dynamic => (
                None,
                format!("Selected tool: {}::{}", decision.server, decision.tool),
            ),
            RouteMode::Query => (
                None,
                "Tool suggestion ready (not executed)".to_string(),
            ),
        };

        if let Some(session) = request.session_id.as_ref() {
            let record = ConversationRecord::new(
                Some(session.clone()),
                "user",
                request.user_request.clone(),
                vec![format!("{}::{}", decision.server, decision.tool)],
            );
            self.history.append(record, embed.clone())?;
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
            result,
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
