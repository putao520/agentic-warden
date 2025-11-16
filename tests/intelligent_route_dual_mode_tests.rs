use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use agentic_warden::mcp_routing::config::{LlmSection, McpConfig, RoutingConfig};
use agentic_warden::mcp_routing::js_orchestrator;
use agentic_warden::mcp_routing::js_orchestrator::workflow_planner::{
    InputParam, WorkflowPlan, WorkflowPlannerEngine, WorkflowStep,
};
use agentic_warden::mcp_routing::models::{
    ExecutionMode, IntelligentRouteRequest, MethodVectorRecord, ToolVectorRecord,
};
use agentic_warden::mcp_routing::registry::{DynamicToolRegistry, RegisteredTool};
use agentic_warden::mcp_routing::{
    CandidateToolInfo, DecisionEngine, FastEmbedder, IntelligentRouter, LlmClient,
    McpConnectionPool, MemRoutingIndex, MethodEmbedding, MockEmbeddingBackend, ToolEmbedding,
};
use agentic_warden::memory::ConversationHistoryStore;
use anyhow::{anyhow, Result as AnyResult};
use async_trait::async_trait;
use ollama_rs::generation::chat::{
    request::ChatMessageRequest, ChatMessage, ChatMessageResponse, MessageRole,
};
use rmcp::model::Tool;
use serde_json::{Map, Value};
use tempfile::{tempdir, TempDir};
use tokio::sync::RwLock;

const EMBED_DIM: usize = 3;

#[tokio::test]
async fn route_uses_vector_when_llm_not_configured() {
    let harness = build_router(sample_tools(), decision_json("git", "status"), None);
    let request = IntelligentRouteRequest {
        user_request: "Check git status".into(),
        execution_mode: ExecutionMode::Dynamic,
        ..Default::default()
    };

    let response = harness
        .router
        .intelligent_route(request)
        .await
        .expect("route succeeded");

    assert!(response.success, "vector routing should succeed");
    assert!(!response.dynamically_registered);
    let selected = response.selected_tool.expect("selected tool");
    assert_eq!(selected.mcp_server, "git");
    assert_eq!(selected.tool_name, "status");
    assert!(harness.registry.get_all_tool_definitions().await.is_empty());
}

#[tokio::test]
async fn route_uses_orchestration_when_llm_succeeds() {
    let planner = MockPlanner::success(sample_plan(), sample_js_code());
    let harness = build_router(
        sample_tools(),
        decision_json("git", "status"),
        Some(planner.clone()),
    );
    let request = IntelligentRouteRequest {
        user_request: "Need an orchestrated git report".into(),
        execution_mode: ExecutionMode::Dynamic,
        ..Default::default()
    };

    let response = harness
        .router
        .intelligent_route(request)
        .await
        .expect("route succeeded");

    assert!(response.success);
    assert!(response.dynamically_registered);
    let selected = response.selected_tool.expect("selected orchestrated tool");
    assert_eq!(selected.mcp_server, "orchestrated");
    assert_eq!(selected.tool_name, sample_plan().suggested_name);
    assert!(response.tool_schema.is_some());
    assert_eq!(planner.plan_calls(), 1);

    let registered = harness
        .registry
        .get_tool(&selected.tool_name)
        .await
        .expect("workflow registered");
    assert!(matches!(registered, RegisteredTool::JsOrchestrated(_)));
}

#[tokio::test]
async fn route_falls_back_to_vector_when_llm_fails() {
    let planner = MockPlanner::fail_code(sample_plan(), "code generation failed");
    let harness = build_router(
        sample_tools(),
        decision_json("git", "status"),
        Some(planner.clone()),
    );
    let request = IntelligentRouteRequest {
        user_request: "Summarize repo status".into(),
        execution_mode: ExecutionMode::Dynamic,
        ..Default::default()
    };

    let response = harness
        .router
        .intelligent_route(request)
        .await
        .expect("route succeeded");

    assert!(response.success);
    assert!(!response.dynamically_registered);
    assert_eq!(planner.plan_calls(), 1, "LLM attempt should happen");
    assert!(harness.registry.get_all_tool_definitions().await.is_empty());
}

#[tokio::test]
async fn orchestrated_tool_registered_to_registry() {
    let workflow_name = "git_report_workflow";
    let mut plan = sample_plan();
    plan.suggested_name = workflow_name.into();
    let planner = MockPlanner::success(plan.clone(), sample_js_code());
    let harness = build_router(
        sample_tools(),
        decision_json("git", "status"),
        Some(planner.clone()),
    );
    let request = IntelligentRouteRequest {
        user_request: "Generate git workflow".into(),
        ..Default::default()
    };

    let response = harness.router.intelligent_route(request).await.unwrap();
    assert!(response.success);
    assert_eq!(response.selected_tool.unwrap().tool_name, workflow_name);

    let entry = harness
        .registry
        .get_tool(workflow_name)
        .await
        .expect("registered workflow");

    match entry {
        RegisteredTool::JsOrchestrated(tool) => {
            assert_eq!(
                tool.tool.description.as_deref(),
                Some(plan.description.as_str())
            );
            assert!(tool.js_code.contains("workflow"));
            assert!(tool.js_code.contains("mcp.call"));
        }
        other => panic!("expected JS tool, got {:?}", other),
    }
}

#[tokio::test]
async fn orchestration_fails_when_no_candidates() {
    let planner = MockPlanner::success(sample_plan(), sample_js_code());
    let harness = build_router(
        Vec::new(),
        decision_json("git", "status"),
        Some(planner.clone()),
    );
    let request = IntelligentRouteRequest {
        user_request: "Unknown request".into(),
        ..Default::default()
    };

    let response = harness.router.intelligent_route(request).await.unwrap();
    assert!(!response.success);
    assert_eq!(response.message, "No MCP tools matched the request");
    assert_eq!(
        planner.plan_calls(),
        0,
        "planner should not be called without candidates"
    );
    assert!(!response.dynamically_registered);
}

struct RouterHarness {
    router: IntelligentRouter,
    registry: Arc<DynamicToolRegistry>,
    _temp_dir: TempDir,
}

fn build_router(
    tools: Vec<TestTool>,
    llm_response: String,
    planner: Option<Arc<MockPlanner>>,
) -> RouterHarness {
    let backend = Arc::new(MockEmbeddingBackend::new(EMBED_DIM, |text| {
        embed_vector_for(text)
    }));
    let embedder = FastEmbedder::with_backend(backend);

    let mut index = MemRoutingIndex::new(EMBED_DIM).expect("index");
    let (tool_embeddings, method_embeddings) = build_embeddings(&tools);
    index
        .rebuild(&tool_embeddings, &method_embeddings)
        .expect("index rebuild");

    let temp_dir = tempdir().expect("temp dir");
    let history_path = temp_dir.path().join("history.db");
    let history = ConversationHistoryStore::new(&history_path, EMBED_DIM).expect("history store");

    let llm_client = Arc::new(MockLlmClient::new(vec![Ok(mock_response(&llm_response))]));
    let decision_engine = Arc::new(DecisionEngine::with_client(llm_client, "mock", 30));

    let routing = RoutingConfig::default();
    let pool = Arc::new(McpConnectionPool::new(minimal_mcp_config()));

    let mut registry_map = HashMap::new();
    for tool in &tools {
        registry_map.insert(
            format!("{}::{}", tool.server, tool.name),
            build_tool_definition(tool),
        );
    }
    let tool_registry = RwLock::new(registry_map);

    let dynamic_registry = Arc::new(DynamicToolRegistry::new(vec![]));
    let js_orchestrator = planner.as_ref().map(|planner| {
        Arc::new(js_orchestrator::WorkflowOrchestrator::with_planner(
            planner.clone(),
        ))
    });

    let router = IntelligentRouter::new_with_components(
        routing,
        embedder,
        index,
        history,
        decision_engine,
        pool,
        tool_registry,
        Some(dynamic_registry.clone()),
        js_orchestrator,
    );

    RouterHarness {
        router,
        registry: dynamic_registry,
        _temp_dir: temp_dir,
    }
}

fn minimal_mcp_config() -> Arc<McpConfig> {
    Arc::new(McpConfig {
        version: "1.0".into(),
        mcp_servers: HashMap::new(),
        routing: RoutingConfig::default(),
        llm: LlmSection::default(),
    })
}

#[derive(Clone)]
struct TestTool {
    server: &'static str,
    name: &'static str,
    description: &'static str,
    vector: [f32; EMBED_DIM],
}

fn sample_tools() -> Vec<TestTool> {
    vec![
        TestTool {
            server: "git",
            name: "status",
            description: "Check git status",
            vector: [0.95, 0.05, 0.0],
        },
        TestTool {
            server: "reports",
            name: "write_report",
            description: "Summarize git state",
            vector: [0.2, 0.8, 0.0],
        },
    ]
}

fn build_embeddings(tools: &[TestTool]) -> (Vec<ToolEmbedding>, Vec<MethodEmbedding>) {
    let mut tool_embeddings = Vec::new();
    let mut method_embeddings = Vec::new();
    for tool in tools {
        let mut metadata = HashMap::new();
        metadata.insert("server".into(), tool.server.to_string());
        metadata.insert("tool".into(), tool.name.to_string());
        metadata.insert("description".into(), tool.description.to_string());
        metadata.insert(
            "schema".into(),
            serde_json::json!({"type": "object"}).to_string(),
        );

        tool_embeddings.push(ToolEmbedding {
            record: ToolVectorRecord {
                id: format!("{}::{}", tool.server, tool.name),
                server: tool.server.to_string(),
                tool_name: tool.name.to_string(),
                description: tool.description.to_string(),
                metadata: metadata.clone(),
            },
            vector: tool.vector.to_vec(),
        });

        method_embeddings.push(MethodEmbedding {
            record: MethodVectorRecord {
                id: format!("method::{}::{}", tool.server, tool.name),
                server: tool.server.to_string(),
                tool_name: tool.name.to_string(),
                description: tool.description.to_string(),
                metadata,
            },
            vector: tool.vector.to_vec(),
        });
    }
    (tool_embeddings, method_embeddings)
}

fn build_tool_definition(tool: &TestTool) -> Tool {
    let mut schema = Map::new();
    schema.insert("type".into(), Value::String("object".into()));

    Tool {
        name: tool.name.into(),
        title: None,
        description: Some(tool.description.into()),
        input_schema: Arc::new(schema),
        output_schema: None,
        icons: None,
        annotations: None,
    }
}

fn embed_vector_for(text: &str) -> Vec<f32> {
    let lower = text.to_lowercase();
    if lower.contains("status") {
        vec![0.99, 0.01, 0.0]
    } else if lower.contains("report") || lower.contains("summary") {
        vec![0.2, 0.8, 0.0]
    } else {
        vec![0.05, 0.05, 0.9]
    }
}

fn decision_json(server: &str, tool: &str) -> String {
    format!(
        "{{\"server\":\"{}\",\"tool\":\"{}\",\"arguments\":{{}},\"rationale\":\"mock\",\"confidence\":0.95}}",
        server, tool
    )
}

fn sample_plan() -> WorkflowPlan {
    WorkflowPlan {
        is_feasible: true,
        reason: String::new(),
        suggested_name: "git_report_workflow".into(),
        description: "Combine git status and reporting".into(),
        steps: vec![
            WorkflowStep {
                step: 1,
                tool: "git::status".into(),
                description: "Check git status".into(),
                dependencies: Vec::new(),
            },
            WorkflowStep {
                step: 2,
                tool: "reports::write_report".into(),
                description: "Summarize changes".into(),
                dependencies: vec![1],
            },
        ],
        input_params: vec![InputParam {
            name: "repo_path".into(),
            param_type: "string".into(),
            description: "Path to the repo".into(),
            required: true,
        }],
    }
}

fn sample_js_code() -> String {
    "async function workflow(input) {\n    const status = await mcp.call(\"git\", \"status\", { repo: input.repo_path });\n    const summary = await mcp.call(\"reports\", \"write_report\", { repo: input.repo_path });\n    return { status, summary };\n}\n"
        .into()
}

struct MockPlanner {
    plan_fn: Box<dyn Fn() -> AnyResult<WorkflowPlan> + Send + Sync>,
    code_fn: Box<dyn Fn(&WorkflowPlan) -> AnyResult<String> + Send + Sync>,
    plan_calls: AtomicUsize,
}

impl MockPlanner {
    fn success(plan: WorkflowPlan, js_code: String) -> Arc<Self> {
        let plan_arc = Arc::new(plan);
        let code_arc = Arc::new(js_code);
        Arc::new(Self {
            plan_fn: Box::new(move || Ok((*plan_arc).clone())),
            code_fn: Box::new(move |_| Ok((*code_arc).clone())),
            plan_calls: AtomicUsize::new(0),
        })
    }

    fn fail_code(plan: WorkflowPlan, message: &'static str) -> Arc<Self> {
        let plan_arc = Arc::new(plan);
        Arc::new(Self {
            plan_fn: Box::new(move || Ok((*plan_arc).clone())),
            code_fn: Box::new(move |_| Err(anyhow!(message))),
            plan_calls: AtomicUsize::new(0),
        })
    }

    fn plan_calls(&self) -> usize {
        self.plan_calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl WorkflowPlannerEngine for MockPlanner {
    async fn plan_workflow(
        &self,
        _user_request: &str,
        _available_tools: &[CandidateToolInfo],
    ) -> AnyResult<WorkflowPlan> {
        self.plan_calls.fetch_add(1, Ordering::SeqCst);
        (self.plan_fn)()
    }

    async fn generate_js_code(&self, plan: &WorkflowPlan) -> AnyResult<String> {
        (self.code_fn)(plan)
    }
}

struct MockLlmClient {
    responses: Mutex<VecDeque<AnyResult<ChatMessageResponse>>>,
}

impl MockLlmClient {
    fn new(responses: Vec<AnyResult<ChatMessageResponse>>) -> Self {
        Self {
            responses: Mutex::new(VecDeque::from(responses)),
        }
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn chat(&self, _request: ChatMessageRequest) -> AnyResult<ChatMessageResponse> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("no mock response configured")))
    }
}

fn mock_response(content: &str) -> ChatMessageResponse {
    ChatMessageResponse {
        model: "mock".into(),
        created_at: "2024-01-01T00:00:00Z".into(),
        message: ChatMessage {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Vec::new(),
            images: None,
            thinking: None,
        },
        done: true,
        final_data: None,
    }
}
