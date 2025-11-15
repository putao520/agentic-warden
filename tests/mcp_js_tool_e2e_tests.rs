use std::collections::HashMap;
use std::sync::Arc;

use agentic_warden::mcp::JsToolExecutor;
use agentic_warden::mcp_routing::js_orchestrator::engine::{BoaRuntimePool, SecurityConfig};
use agentic_warden::mcp_routing::js_orchestrator::injector::{
    InjectedMcpFunction, McpFunctionInjector, McpToolInvoker,
};
use agentic_warden::mcp_routing::registry::{JsOrchestratedTool, ToolMetadata};
use anyhow::Result as AnyResult;
use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::{json, Map, Value};
use tokio::sync::Mutex as AsyncMutex;

#[tokio::test]
async fn test_js_tool_execution_success() {
    let invoker = Arc::new(RecordingInvoker::with_response(
        "git::git_status",
        json!({"branch": "main"}),
    ));
    let executor = build_executor(invoker.clone() as Arc<dyn McpToolInvoker>, None).await;

    let tool = build_js_tool(
        r#"
async function workflow(input) {
    const status = await mcpGitStatus({ repo: input.repo });
    return { ok: true, branch: status.branch };
}
"#,
        vec![InjectedMcpFunction {
            server: "git".into(),
            name: "git_status".into(),
            description: "Git status".into(),
        }],
    );

    let report = executor
        .execute(&tool, json!({"repo": "demo"}))
        .await
        .expect("workflow success");

    assert_eq!(report.output, json!({"ok": true, "branch": "main"}));
}

#[tokio::test]
async fn test_js_tool_with_mcp_dependency() {
    let mut responses = HashMap::new();
    responses.insert(
        "git::git_status".to_string(),
        json!({"files_changed": 2}),
    );
    responses.insert(
        "reports::write_report".to_string(),
        json!({"path": "REPORT.md"}),
    );
    let invoker = Arc::new(RecordingInvoker::new(responses));
    let executor = build_executor(invoker.clone() as Arc<dyn McpToolInvoker>, None).await;

    let tool = build_js_tool(
        r#"
async function workflow(input) {
    const status = await mcpGitStatus({ repo: input.repo });
    const report = await mcpWriteReport({ repo: input.repo, delta: status.files_changed });
    return { status, report };
}
"#,
        vec![
            InjectedMcpFunction {
                server: "git".into(),
                name: "git_status".into(),
                description: "Git status".into(),
            },
            InjectedMcpFunction {
                server: "reports".into(),
                name: "write_report".into(),
                description: "Write report".into(),
            },
        ],
    );

    let report = executor
        .execute(&tool, json!({"repo": "demo"}))
        .await
        .expect("workflow success");

    assert_eq!(report.output["report"]["path"], json!("REPORT.md"));
    let calls = invoker.calls().await;
    assert_eq!(calls, vec!["git::git_status", "reports::write_report"]);
}

#[tokio::test]
async fn test_js_tool_input_validation() {
    let invoker = Arc::new(RecordingInvoker::with_response(
        "git::git_status",
        json!({"branch": "main"}),
    ));
    let executor = build_executor(invoker as Arc<dyn McpToolInvoker>, None).await;
    let tool = build_js_tool(
        r#"
async function workflow(input) {
    if (!input.repo) {
        throw new Error('repo is required');
    }
    return input.repo;
}
"#,
        vec![],
    );

    let err = executor.execute(&tool, json!({})).await.unwrap_err();
    let err_string = err.to_string();
    println!("Error message: {}", err_string);
    assert!(err_string.contains("repo is required"), "Expected error to contain 'repo is required', but got: {}", err_string);
}

#[tokio::test]
async fn test_js_tool_timeout_handling() {
    let pool = Arc::new(
        BoaRuntimePool::with_security(SecurityConfig {
            max_execution_time_ms: 50,
            ..SecurityConfig::default()
        })
        .await
        .expect("pool"),
    );
    let invoker = Arc::new(RecordingInvoker::default());
    let executor = build_executor(invoker as Arc<dyn McpToolInvoker>, Some(pool)).await;
    let tool = build_js_tool(
        r#"
async function workflow() {
    while (true) {}
}
"#,
        vec![],
    );

    let err = executor
        .execute(&tool, Value::Null)
        .await
        .expect_err("timeout error");
    assert!(err.to_string().to_lowercase().contains("timed out"));
}

#[tokio::test]
async fn test_js_tool_runtime_error() {
    let executor = build_executor(
        Arc::new(RecordingInvoker::default()) as Arc<dyn McpToolInvoker>,
        None,
    )
    .await;
    let tool = build_js_tool(
        r#"
async function workflow() {
    throw new Error('boom');
}
"#,
        vec![],
    );

    let err = executor
        .execute(&tool, Value::Null)
        .await
        .expect_err("runtime error");
    assert!(err.to_string().contains("boom"));
}

async fn build_executor(
    invoker: Arc<dyn McpToolInvoker>,
    pool: Option<Arc<BoaRuntimePool>>,
) -> JsToolExecutor {
    let injector = Arc::new(McpFunctionInjector::with_invoker(invoker));
    let pool = match pool {
        Some(pool) => pool,
        None => Arc::new(BoaRuntimePool::new().await.expect("boa pool")),
    };
    JsToolExecutor::new(pool, injector)
}

fn build_js_tool(code: &str, deps: Vec<InjectedMcpFunction>) -> JsOrchestratedTool {
    JsOrchestratedTool {
        tool: Tool {
            name: "workflow".into(),
            title: None,
            description: Some("Test workflow".into()),
            input_schema: Arc::new(Map::new()),
            output_schema: None,
            icons: None,
            annotations: None,
        },
        js_code: code.into(),
        mcp_dependencies: deps,
        metadata: ToolMetadata::new(60),
    }
}

#[derive(Default)]
struct RecordingInvoker {
    responses: HashMap<String, Value>,
    calls: AsyncMutex<Vec<String>>,
}

impl RecordingInvoker {
    fn new(responses: HashMap<String, Value>) -> Self {
        Self {
            responses,
            calls: AsyncMutex::new(Vec::new()),
        }
    }

    fn with_response(key: &str, value: Value) -> Self {
        let mut responses = HashMap::new();
        responses.insert(key.to_string(), value);
        Self::new(responses)
    }

    async fn calls(&self) -> Vec<String> {
        self.calls.lock().await.clone()
    }
}

#[async_trait]
impl McpToolInvoker for RecordingInvoker {
    async fn call_tool(&self, server: &str, tool_name: &str, _args: Value) -> AnyResult<Value> {
        let key = format!("{}::{}", server, tool_name);
        self.calls.lock().await.push(key.clone());
        Ok(self
            .responses
            .get(&key)
            .cloned()
            .unwrap_or_else(|| json!({"server": server, "tool": tool_name})))
    }
}
