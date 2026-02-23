mod capability_generator; // REQ-013: Capability description generation
pub mod codegen;
pub mod config;
pub mod config_watcher;
mod decision;
mod embedding;
mod index;
pub mod js_orchestrator; // REQ-013: JS orchestration
pub mod models;
mod pool;
pub mod registry; // REQ-013: Dynamic tool registry

pub use embedding::{EmbeddingBackend, MockEmbeddingBackend};
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
use anyhow::{anyhow, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use memvdb::normalize;
use parking_lot::Mutex;
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::sync::RwLock;

const METHOD_VECTOR_PREFIX: &str = "method";

pub struct IntelligentRouter {
    embedder: Arc<Mutex<TextEmbedding>>,
    index: Mutex<MemRoutingIndex>,
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

        // Initialize embedder with all-MiniLM-L6-v2 via fastembed (ONNX Runtime)
        let embedder = Arc::new(Mutex::new(
            TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                    .with_show_download_progress(true)
            )
            .map_err(|e| anyhow!("Failed to initialize fastembed: {}", e))?
        ));

        // Initialize code generator using factory pattern
        let decision_endpoint = std::env::var("OPENAI_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let decision_model =
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "qwen3:1.7b".to_string());

        // Discover downstream MCP tools first (needed for capability description)
        let connection_pool = Arc::new(McpConnectionPool::new(config_arc.clone()));
        let discovered = connection_pool.warm_up().await?;

        // REQ-013 Phase 1: Generate capability description
        let capability_generator = capability_generator::CapabilityGenerator::new();

        let capability_description = capability_generator
            .generate_capability_description(&discovered)?;

        eprintln!(
            "📝 Generated capability description: {}",
            capability_description
        );

        // Construct base tools (persistent tools shown in list_tools)
        let base_tools = vec![Tool {
            name: "intelligent_route".into(),
            title: Some("Intelligent Tool Router".into()),
            description: Some(capability_description.into()),
            input_schema: Arc::new(serde_json::from_value(json!({
                "type": "object",
                "properties": {
                    "user_request": {
                        "type": "string",
                        "description": "Natural language description of what you want to accomplish"
                    },
                    "max_candidates": {
                        "type": "integer",
                        "description": "Maximum number of candidate tools to consider (default: 3)",
                        "minimum": 1,
                        "maximum": 10
                    }
                },
                "required": ["user_request"]
            }))?),
            output_schema: None,
            icons: None,
            annotations: None,
            execution: None,
            meta: None,
        }];

        // Create dynamic registry with max 5 dynamic tools (REQ-013: FIFO eviction)
        let registry_config = registry::RegistryConfig {
            max_dynamic_tools: 5,
            default_ttl_seconds: 86400, // 1 day TTL (effectively permanent)
            cleanup_interval_seconds: 3600, // 1 hour cleanup
        };
        let dynamic_registry = Arc::new(registry::DynamicToolRegistry::with_config(
            base_tools,
            registry_config,
        ));
        let _cleanup_task = dynamic_registry.start_cleanup_task();

        // Check if external LLM API is available for orchestration
        let has_external_api = std::env::var("OPENAI_TOKEN").is_ok()
            || std::env::var("OPENAI_ENDPOINT")
                .ok()
                .map(|v| v != "http://localhost:11434")
                .unwrap_or(false);

        let (decision_engine, js_orchestrator) = if has_external_api {
            // External API available: try to create js_orchestrator
            match codegen::CodeGeneratorFactory::from_env(
                decision_endpoint.clone(),
                decision_model.clone(),
            ) {
                Ok(generator) => {
                    let decision_engine = Arc::new(DecisionEngine::new(
                        &decision_endpoint,
                        &decision_model,
                        120,
                    )?);
                    let orchestrator = Some(Arc::new(
                        js_orchestrator::WorkflowOrchestrator::with_planner(generator),
                    ));
                    (decision_engine, orchestrator)
                }
                Err(e) => {
                    eprintln!("⚠️  Code generator initialization failed: {}", e);
                    eprintln!("🔍 Falling back to vector-only mode");
                    let decision_engine = Arc::new(DecisionEngine::new(
                        &decision_endpoint,
                        &decision_model,
                        120,
                    )?);
                    (decision_engine, None)
                }
            }
        } else {
            // No external API: skip js_orchestrator, use vector + single-step LLM decision
            eprintln!("🔍 No external LLM API detected (set OPENAI_TOKEN or OPENAI_ENDPOINT to enable orchestration)");
            let decision_engine = Arc::new(DecisionEngine::new(
                &decision_endpoint,
                &decision_model,
                120,
            )?);
            (decision_engine, None)
        };

        let mut index = MemRoutingIndex::new(384)?; // all-MiniLM-L6-v2 dimension
        let tool_registry = RwLock::new(HashMap::new());
        let embeddings = build_embeddings(&embedder, &discovered, config_arc.as_ref())?;
        index.rebuild(&embeddings.tools, &embeddings.methods)?;

        populate_registry(&tool_registry, discovered).await;

        Ok(Self {
            embedder,
            index: Mutex::new(index),
            decision_engine,
            connection_pool,
            tool_registry,
            dynamic_registry: Some(dynamic_registry),
            js_orchestrator,
        })
    }

    /// Build a router from explicit dependencies (used for deterministic testing).
    pub fn new_with_components(
        embedder: Arc<Mutex<TextEmbedding>>,
        index: MemRoutingIndex,
        decision_engine: Arc<DecisionEngine>,
        connection_pool: Arc<McpConnectionPool>,
        tool_registry: RwLock<HashMap<String, Tool>>,
        dynamic_registry: Option<Arc<registry::DynamicToolRegistry>>,
        js_orchestrator: Option<Arc<js_orchestrator::WorkflowOrchestrator>>,
    ) -> Self {
        Self {
            embedder,
            index: Mutex::new(index),
            decision_engine,
            connection_pool,
            tool_registry,
            dynamic_registry,
            js_orchestrator,
        }
    }

    /// Get the dynamic tool registry (for sharing with MCP server)
    pub fn dynamic_registry(&self) -> Option<Arc<registry::DynamicToolRegistry>> {
        self.dynamic_registry.clone()
    }

    /// Get read access to the downstream tool registry.
    pub fn tool_registry(&self) -> &RwLock<HashMap<String, Tool>> {
        &self.tool_registry
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
                tool_schema: None,
                dynamically_registered: false,
            });
        }

        let embed = self.embedder
            .lock()
            .embed(vec![request.user_request.clone()], None)
            .map_err(|e| anyhow!("Embedding generation failed: {}", e))?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No embedding generated"))?;
        let embed = normalize(&embed);

        // Query mode: skip LLM orchestration, use vector search only (no tool registration)
        if matches!(request.execution_mode, models::ExecutionMode::Query) {
            eprintln!("🔍 Query mode: using vector search (no tool registration)");
            return self.vector_mode(&request, &embed).await;
        }

        // Dynamic mode: fast-path via vector search when top match is high-confidence,
        // otherwise try full LLM orchestration (which can take minutes).
        match self.js_orchestrator.as_ref() {
            None => {
                eprintln!("🔍 LLM not configured, using vector search mode");
                self.vector_mode(&request, &embed).await
            }
            Some(orchestrator) => {
                // Fast-path: if vector search yields a high-confidence single-tool match,
                // skip the heavy LLM orchestration pipeline (plan + codegen + schema fix).
                let fast_threshold = 0.75_f32;
                let top_score = {
                    let index = self.index.lock();
                    index
                        .search_tools(&embed, 1)
                        .ok()
                        .and_then(|scores| scores.into_iter().next())
                        .map(|st| st.score)
                };

                if let Some(score) = top_score {
                    if score >= fast_threshold {
                        eprintln!(
                            "⚡ High-confidence vector match ({:.2}), using fast vector_mode (skipping LLM orchestration)",
                            score
                        );
                        return self.vector_mode(&request, &embed).await;
                    }
                }

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
            .unwrap_or(config::DEFAULT_MAX_TOOLS_PER_REQUEST);

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
                tool_schema: None,
                dynamically_registered: false,
            });
        }

        let candidate_infos = build_candidates(&tool_scores, &method_scores);

        // Try LLM decision first, fall back to pure vector top-1 if LLM unavailable
        let (server, tool, arguments, rationale, confidence) = match self
            .decision_engine
            .decide(DecisionInput {
                user_request: request.user_request.clone(),
                candidates: candidate_infos.clone(),
            })
            .await
        {
            Ok(decision) => {
                eprintln!("✅ Vector mode: LLM decision succeeded");
                (
                    decision.server,
                    decision.tool,
                    decision.arguments,
                    decision.rationale,
                    decision.confidence,
                )
            }
            Err(e) => {
                eprintln!("⚠️  Vector mode: LLM unavailable ({}), using top vector match", e);
                let top = &candidate_infos[0];
                (
                    top.server.clone(),
                    top.tool.clone(),
                    Value::Object(Default::default()),
                    "Best vector match (LLM unavailable)".to_string(),
                    0.6, // reasonable default confidence for top vector match
                )
            }
        };

        let execute_message = match request.execution_mode {
            models::ExecutionMode::Dynamic => {
                format!(
                    "Selected tool: {}::{} (will be dynamically registered)",
                    server, tool
                )
            }
            models::ExecutionMode::Query => {
                format!(
                    "Suggested tool: {}::{} (review and call execute_tool)",
                    server, tool
                )
            }
        };

        Ok(IntelligentRouteResponse {
            success: true,
            confidence,
            message: execute_message,
            selected_tool: Some(SelectedRoute {
                mcp_server: server,
                tool_name: tool,
                arguments,
                rationale,
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
        eprintln!("   🔍 [DEBUG] try_orchestrate started");

        // BUG FIX #1: For orchestration, pass ALL tools to LLM planner, not just top vector matches
        // The LLM needs complete tool visibility to plan optimal workflows
        let candidate_infos: Vec<CandidateToolInfo> = {
            let registry = self.tool_registry.read().await;
            registry
                .iter()
                .map(|(key, tool_def)| {
                    let parts: Vec<&str> = key.split("::").collect();
                    let server = parts.get(0).map(|s| s.to_string()).unwrap_or_default();
                    let tool_name = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                    let description = tool_def
                        .description
                        .as_ref()
                        .map(|d| d.to_string())
                        .unwrap_or_default();
                    let schema = serde_json::to_string(&*tool_def.input_schema).ok();

                    CandidateToolInfo {
                        server,
                        tool: tool_name,
                        description,
                        schema_snippet: schema,
                    }
                })
                .collect()
        };

        eprintln!(
            "   🔍 [DEBUG] Passing {} tools to orchestrator (all available tools)",
            candidate_infos.len()
        );

        if candidate_infos.is_empty() {
            return Err(anyhow!("No candidate tools for orchestration"));
        }

        eprintln!("   🔍 [DEBUG] Calling orchestrator.orchestrate()...");

        let orchestrated_tool = match orchestrator
            .orchestrate(&request.user_request, &candidate_infos)
            .await
        {
            Ok(tool) => {
                eprintln!("   ✅ [DEBUG] Orchestration succeeded: {}", tool.name);
                tool
            }
            Err(e) => {
                eprintln!("   ❌ [DEBUG] Orchestration failed: {}", e);
                return Err(e);
            }
        };

        let Some(registry) = self.dynamic_registry.as_ref() else {
            return Err(anyhow!("Dynamic registry not initialized"));
        };

        // Decide registration type based on optimization result
        let (mcp_server, message) = if let Some(proxy_info) = &orchestrated_tool.proxy_info {
            // Direct proxy mode: register as proxied tool (no JS wrapper)
            let tool_key = format!("{}::{}", proxy_info.server, proxy_info.tool_name);
            let tool_def = {
                let tool_registry = self.tool_registry.read().await;
                tool_registry.get(&tool_key).cloned()
            };

            let tool = match tool_def {
                Some(def) => rmcp::model::Tool {
                    name: orchestrated_tool.name.clone().into(),
                    title: None,
                    description: Some(orchestrated_tool.description.clone().into()),
                    input_schema: def.input_schema.clone(),
                    output_schema: None,
                    icons: None,
                    annotations: None,
                    execution: None,
                    meta: None,
                },
                None => {
                    // Fallback: create tool with schema from plan
                    let schema_map = match &orchestrated_tool.input_schema {
                        serde_json::Value::Object(map) => map.clone(),
                        _ => serde_json::Map::new(),
                    };
                    rmcp::model::Tool {
                        name: orchestrated_tool.name.clone().into(),
                        title: None,
                        description: Some(orchestrated_tool.description.clone().into()),
                        input_schema: std::sync::Arc::new(schema_map),
                        output_schema: None,
                        icons: None,
                        annotations: None,
                        execution: None,
                        meta: None,
                    }
                }
            };

            registry
                .register_proxied_tool(
                    proxy_info.server.clone(),
                    proxy_info.tool_name.clone(),
                    tool,
                )
                .await?;

            (
                proxy_info.server.clone(),
                format!(
                    "Registered '{}' (proxy to {}::{}). Use this tool directly.",
                    orchestrated_tool.name, proxy_info.server, proxy_info.tool_name
                ),
            )
        } else if let Some(js_code) = &orchestrated_tool.js_code {
            // JS orchestration mode: register as JS tool
            registry
                .register_js_tool(
                    orchestrated_tool.name.clone(),
                    orchestrated_tool.description.clone(),
                    orchestrated_tool.input_schema.clone(),
                    js_code.clone(),
                )
                .await?;

            (
                "orchestrated".to_string(),
                format!(
                    "Created orchestrated workflow '{}'. Use this tool to solve your request.",
                    orchestrated_tool.name
                ),
            )
        } else {
            return Err(anyhow!(
                "Invalid orchestrated tool: neither proxy_info nor js_code present"
            ));
        };

        Ok(IntelligentRouteResponse {
            success: true,
            message,
            confidence: 1.0,
            selected_tool: Some(SelectedRoute {
                mcp_server: mcp_server.into(),
                tool_name: orchestrated_tool.name.clone(),
                arguments: Value::Object(Default::default()),
                rationale: orchestrated_tool.description.clone(),
            }),
            result: None,
            alternatives: Vec::new(),
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
    embedder: &Arc<Mutex<TextEmbedding>>,
    tools: &[DiscoveredTool],
    _config: &config::McpConfig,
) -> Result<PreparedEmbeddings> {
    // Collect all docs for batch embedding (much faster than one-by-one)
    let mut docs = Vec::with_capacity(tools.len());
    let mut metas: Vec<(String, String, String, HashMap<String, String>)> = Vec::with_capacity(tools.len());

    for tool in tools {
        let category = "uncategorized".to_string();
        let description = tool
            .definition
            .description
            .as_deref()
            .unwrap_or("No description provided")
            .to_string();
        let schema_value = Value::Object((*tool.definition.input_schema).clone());
        let schema_string = schema_value.to_string();

        let doc = format!(
            "{tool}\nDescription: {description}",
            tool = tool.definition.name,
            description = description,
        );
        docs.push(doc);

        let mut metadata = HashMap::new();
        metadata.insert("server".into(), tool.server.clone());
        metadata.insert("tool".into(), tool.definition.name.to_string());
        metadata.insert("description".into(), description.clone());
        metadata.insert("category".into(), category);
        metadata.insert("schema".into(), schema_string);
        metas.push((tool.server.clone(), tool.definition.name.to_string(), description, metadata));
    }

    // Batch embed all documents at once
    let vectors = embedder
        .lock()
        .embed(docs, None)
        .map_err(|e| anyhow!("Batch embedding failed: {}", e))?;

    let mut tool_embeddings = Vec::with_capacity(vectors.len());
    let mut method_embeddings = Vec::with_capacity(vectors.len());

    for (vector, (server, tool_name, description, metadata)) in vectors.into_iter().zip(metas) {
        let vector = normalize(&vector);

        tool_embeddings.push(ToolEmbedding {
            record: ToolVectorRecord {
                id: format!("{}::{}", server, tool_name),
                server: server.clone(),
                tool_name: tool_name.clone(),
                description: description.clone(),
                metadata: metadata.clone(),
            },
            vector: vector.clone(),
        });

        method_embeddings.push(MethodEmbedding {
            record: crate::mcp_routing::models::MethodVectorRecord {
                id: format!("{METHOD_VECTOR_PREFIX}::{server}::{tool_name}"),
                server,
                tool_name,
                description,
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
