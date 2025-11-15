use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use agentic_warden::mcp_routing::js_orchestrator::workflow_planner::{
    InputParam, WorkflowOrchestrator, WorkflowPlan, WorkflowPlannerEngine, WorkflowStep,
};
use agentic_warden::mcp_routing::{CandidateToolInfo, DecisionEngine, LlmClient};
use anyhow::{anyhow, Result as AnyResult};
use async_trait::async_trait;
use ollama_rs::generation::chat::{
    request::ChatMessageRequest, ChatMessage, ChatMessageResponse, MessageRole,
};

#[tokio::test]
async fn workflow_planning_single_tool() {
    let response = mock_response(
        r#"{
        "is_feasible": true,
        "suggested_name": "git status",
        "description": "Check repo status",
        "steps": [
            {"step": 1, "tool": "repo::git_status", "description": "Get repo status", "dependencies": []}
        ],
        "input_params": [
            {"name": "repo_url", "type": "string", "description": "Repo URL", "required": true}
        ]
    }"#,
    );

    let client = Arc::new(MockLlmClient::new(vec![Ok(response)]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let plan = engine
        .plan_workflow("Check git status", &sample_tools())
        .await
        .unwrap();

    assert!(plan.is_feasible);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].tool, "repo::git_status");
    assert_eq!(plan.input_params[0].name, "repo_url");
    assert_eq!(plan.suggested_name, "git_status");
}

#[tokio::test]
async fn workflow_planning_multi_tool() {
    let response = mock_response(
        r#"{
        "is_feasible": true,
        "suggested_name": "report builder",
        "steps": [
            {"step": 2, "tool": "repo::git_status", "description": "Get status", "dependencies": []},
            {"step": 2, "tool": "reports::generate", "description": "Generate report", "dependencies": [1,1,3]}
        ],
        "input_params": [
            {"name": "repo_url", "type": "string", "description": "Repo URL", "required": true},
            {"name": "format", "type": "string", "description": "Output format", "required": false}
        ]
    }"#,
    );

    let client = Arc::new(MockLlmClient::new(vec![Ok(response)]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let plan = engine
        .plan_workflow("Create a git report", &sample_tools())
        .await
        .unwrap();

    assert_eq!(plan.steps.len(), 2);
    assert_eq!(
        plan.steps.iter().map(|s| s.step).collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(plan
        .steps
        .iter()
        .all(|step| step.dependencies.iter().all(|dep| *dep < step.step)));
    assert_eq!(plan.input_params.len(), 2);
}

#[tokio::test]
async fn workflow_planning_infeasible() {
    let response = mock_response(
        r#"{
        "is_feasible": false,
        "reason": "No deployment tool",
        "steps": [],
        "input_params": []
    }"#,
    );

    let client = Arc::new(MockLlmClient::new(vec![Ok(response)]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let plan = engine
        .plan_workflow("Deploy production", &sample_tools())
        .await
        .unwrap();

    assert!(!plan.is_feasible);
    assert_eq!(plan.reason, "No deployment tool");
}

#[tokio::test]
async fn workflow_planning_input_param_dedup() {
    let response = mock_response(
        r#"{
        "is_feasible": true,
        "steps": [],
        "input_params": [
            {"name": "repo_url", "type": "string", "description": "Repo URL", "required": true},
            {"name": "repo_URL", "type": "number", "description": "duplicate", "required": false}
        ]
    }"#,
    );

    let client = Arc::new(MockLlmClient::new(vec![Ok(response)]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let plan = engine
        .plan_workflow("Dedup params", &sample_tools())
        .await
        .unwrap();

    assert_eq!(plan.input_params.len(), 1);
    assert_eq!(plan.input_params[0].param_type, "string");
}

#[tokio::test]
async fn workflow_planning_invalid_json() {
    let client = Arc::new(MockLlmClient::new(vec![Ok(mock_response("not json"))]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let result = engine.plan_workflow("Invalid", &sample_tools()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn workflow_planning_handles_llm_failure() {
    let client = Arc::new(MockLlmClient::new(vec![Err(anyhow!("LLM offline"))]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let result = engine.plan_workflow("Failure", &sample_tools()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn code_generation_strips_code_fences() {
    let response = mock_response(
        r#"```javascript
async function workflow(input) {
    try {
        const status = await mcpGitStatus({ repo: input.repo_url });
        return { ok: true, status };
    } catch (error) {
        return { ok: false, error: error.message };
    }
}
workflow();
```"#,
    );

    let client = Arc::new(MockLlmClient::new(vec![Ok(response)]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let code = engine.generate_js_code(&base_plan()).await.unwrap();

    assert!(code.contains("async function workflow"));
    assert!(code.contains("mcpGitStatus"));
    assert!(code.contains("try"));
    assert!(code.contains("catch"));
}

#[tokio::test]
async fn code_generation_rejects_infeasible_plan() {
    let client = Arc::new(MockLlmClient::new(vec![]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let mut plan = base_plan();
    plan.is_feasible = false;
    let result = engine.generate_js_code(&plan).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn code_generation_rejects_empty_output() {
    let client = Arc::new(MockLlmClient::new(vec![Ok(mock_response("Ack"))]));
    let engine = DecisionEngine::with_client(client, "mock", 30);
    let result = engine.generate_js_code(&base_plan()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn orchestrate_end_to_end_success() {
    let plan = WorkflowPlan {
        is_feasible: true,
        reason: String::new(),
        suggested_name: "git_report_workflow".into(),
        description: "Create a git report".into(),
        steps: vec![
            WorkflowStep {
                step: 1,
                tool: "repo::git_status".into(),
                description: "Fetch git status".into(),
                dependencies: vec![],
            },
            WorkflowStep {
                step: 2,
                tool: "report::write_report".into(),
                description: "Write summary".into(),
                dependencies: vec![1],
            },
        ],
        input_params: vec![InputParam {
            name: "repo_url".into(),
            param_type: "string".into(),
            description: "Repository URL".into(),
            required: true,
        }],
    };

    let code = sample_js_code();
    let planner = Arc::new(MockWorkflowPlanner::new(
        vec![Ok(plan)],
        vec![Ok(code.into())],
    ));
    let orchestrator = WorkflowOrchestrator::with_planner(planner);
    let tool = orchestrator
        .orchestrate("Create a git report", &sample_tools())
        .await
        .unwrap();

    assert_eq!(tool.name, "git_report_workflow");
    assert_eq!(tool.description, "Create a git report");
    let properties = tool
        .input_schema
        .get("properties")
        .and_then(|v| v.as_object())
        .unwrap();
    let repo_schema = properties.get("repo_url").unwrap().as_object().unwrap();
    assert_eq!(
        repo_schema.get("type").and_then(|v| v.as_str()),
        Some("string")
    );
    let dependencies: Vec<(String, String)> = tool
        .mcp_dependencies
        .iter()
        .map(|dep| (dep.server.clone(), dep.name.clone()))
        .collect();
    assert_eq!(
        dependencies,
        vec![
            ("repo".to_string(), "git_status".to_string()),
            ("report".to_string(), "write_report".to_string()),
        ]
    );
}

#[tokio::test]
async fn orchestrate_handles_infeasible_plan() {
    let plan = WorkflowPlan {
        is_feasible: false,
        reason: "missing tool".into(),
        ..base_plan()
    };
    let planner = Arc::new(MockWorkflowPlanner::new(
        vec![Ok(plan)],
        vec![Ok(sample_js_code().into())],
    ));
    let orchestrator = WorkflowOrchestrator::with_planner(planner);
    let result = orchestrator
        .orchestrate("Infeasible", &sample_tools())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn orchestrate_fails_on_code_validation() {
    let planner = Arc::new(MockWorkflowPlanner::new(
        vec![Ok(base_plan())],
        vec![Ok("function broken( {".into())],
    ));
    let orchestrator = WorkflowOrchestrator::with_planner(planner);
    let result = orchestrator.orchestrate("Bad code", &sample_tools()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn orchestrate_bubbles_planning_error() {
    let planner = Arc::new(MockWorkflowPlanner::new(
        vec![Err(anyhow!("planning failed"))],
        vec![],
    ));
    let orchestrator = WorkflowOrchestrator::with_planner(planner);
    let result = orchestrator
        .orchestrate("Plan error", &sample_tools())
        .await;
    assert!(result.is_err());
}

fn sample_tools() -> Vec<CandidateToolInfo> {
    vec![
        CandidateToolInfo {
            server: "repo".into(),
            tool: "git_status".into(),
            description: "Git status".into(),
            schema_snippet: None,
        },
        CandidateToolInfo {
            server: "reports".into(),
            tool: "generate".into(),
            description: "Generate report".into(),
            schema_snippet: None,
        },
    ]
}

fn base_plan() -> WorkflowPlan {
    WorkflowPlan {
        is_feasible: true,
        reason: String::new(),
        suggested_name: "git_status_workflow".into(),
        description: "Check git status".into(),
        steps: vec![WorkflowStep {
            step: 1,
            tool: "repo::git_status".into(),
            description: "Fetch status".into(),
            dependencies: vec![],
        }],
        input_params: vec![InputParam {
            name: "repo_url".into(),
            param_type: "string".into(),
            description: "Repository URL".into(),
            required: true,
        }],
    }
}

fn sample_js_code() -> &'static str {
    r#"
async function workflow(input) {
    try {
        const status = await mcpGitStatus({ repo: input.repo_url });
        const summary = await mcpWriteReport({
            repo: input.repo_url,
            branch: status.branch,
        });
        return { ok: true, status, summary };
    } catch (error) {
        return { ok: false, error: error.message };
    }
}
"#
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

struct MockWorkflowPlanner {
    plan_results: Mutex<VecDeque<AnyResult<WorkflowPlan>>>,
    code_results: Mutex<VecDeque<AnyResult<String>>>,
}

impl MockWorkflowPlanner {
    fn new(
        plan_results: Vec<AnyResult<WorkflowPlan>>,
        code_results: Vec<AnyResult<String>>,
    ) -> Self {
        Self {
            plan_results: Mutex::new(VecDeque::from(plan_results)),
            code_results: Mutex::new(VecDeque::from(code_results)),
        }
    }
}

#[async_trait]
impl WorkflowPlannerEngine for MockWorkflowPlanner {
    async fn plan_workflow(
        &self,
        _user_request: &str,
        _available_tools: &[CandidateToolInfo],
    ) -> AnyResult<WorkflowPlan> {
        self.plan_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("no plan result configured")))
    }

    async fn generate_js_code(&self, _plan: &WorkflowPlan) -> AnyResult<String> {
        self.code_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("no code result configured")))
    }
}
