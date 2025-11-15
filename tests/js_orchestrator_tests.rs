use std::sync::Arc;
use std::time::Duration;

use agentic_warden::mcp_routing::js_orchestrator::{
    BoaRuntime, BoaRuntimePool, InjectedMcpFunction, JsCodeValidator, McpFunctionInjector,
    McpToolInvoker, SecurityConfig,
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex as AsyncMutex;

#[tokio::test]
async fn sandbox_blocks_dangerous_globals() {
    let runtime = BoaRuntime::new().unwrap();

    assert!(runtime.execute("eval('1+1')").await.is_err());
    assert!(runtime.execute("Function('return 42')()").await.is_err());
    let require_type = runtime.execute("typeof require").await.unwrap();
    assert_eq!(require_type, json!("undefined"));
    let proto_type = runtime.execute("typeof ({}).__proto__").await.unwrap();
    assert_eq!(proto_type, json!("undefined"));
}

#[tokio::test]
async fn timeout_limit_is_enforced() {
    let runtime = BoaRuntime::with_security(SecurityConfig {
        max_execution_time_ms: 50,
        ..SecurityConfig::default()
    })
    .unwrap();

    let err = runtime.execute("while(true) {}").await.unwrap_err();
    assert!(err.to_string().contains("timed out"));
}

#[tokio::test]
async fn memory_limit_is_checked() {
    let runtime = BoaRuntime::with_security(SecurityConfig {
        max_memory_mb: 1,
        ..SecurityConfig::default()
    })
    .unwrap();

    let err = runtime.execute("42").await.unwrap_err();
    assert!(err.to_string().contains("memory usage"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_pool_limits_connections() {
    let pool = BoaRuntimePool::new().await.unwrap();
    let mut guards = Vec::new();
    for _ in 0..10 {
        guards.push(pool.acquire().await.unwrap());
    }

    let wait_result = tokio::time::timeout(Duration::from_millis(200), pool.acquire()).await;
    assert!(
        wait_result.is_err(),
        "expected acquire to block when pool exhausted"
    );

    drop(guards.pop());
    let handle = tokio::time::timeout(Duration::from_millis(500), pool.acquire())
        .await
        .expect("slot released")
        .unwrap();
    drop(handle);
}

#[tokio::test]
async fn injected_mcp_function_executes() {
    let invoker = Arc::new(MockInvoker::new(json!({"branch": "main"})));
    let injector = McpFunctionInjector::with_invoker(invoker.clone());
    let runtime = BoaRuntime::new().unwrap();

    let tools = vec![InjectedMcpFunction {
        server: "mock".into(),
        name: "git_status".into(),
        description: "mock".into(),
    }];
    let injector_clone = injector.clone();
    let handle = tokio::runtime::Handle::current();
    runtime
        .with_context(move |ctx| {
            injector_clone.inject_all(ctx, &tools, handle.clone())?;
            Ok(())
        })
        .await
        .unwrap();

    let output = runtime
        .execute(
            r#"
            async function workflow() {
                const status = await mcpGitStatus({ repo: "demo" });
                return status.branch;
            }
            workflow();
            "#,
        )
        .await
        .unwrap();

    assert_eq!(output, json!("main"));
    assert_eq!(*invoker.calls.lock().await, 1);
}

#[tokio::test]
async fn validator_detects_security_issues() {
    let valid_code = r#"
        async function workflow() {
            const status = await mcpGitStatus();
            return status.prompt;
        }
        workflow();
    "#;

    let validation = JsCodeValidator::validate(valid_code).unwrap();
    assert!(validation.passed);

    let invalid = "eval('dangerous')";
    let validation = JsCodeValidator::validate(invalid).unwrap();
    assert!(!validation.passed);

    let runtime_error = r#"
        async function workflow() {
            throw new Error('boom');
        }
        workflow();
    "#;
    let validation = JsCodeValidator::validate(runtime_error).unwrap();
    assert!(!validation.passed);
    assert!(validation.errors[0].contains("Dry-run failed"));
}

struct MockInvoker {
    value: serde_json::Value,
    calls: AsyncMutex<usize>,
}

impl MockInvoker {
    fn new(value: serde_json::Value) -> Self {
        Self {
            value,
            calls: AsyncMutex::new(0),
        }
    }
}

#[async_trait]
impl McpToolInvoker for MockInvoker {
    async fn call_tool(
        &self,
        _server: &str,
        _tool_name: &str,
        _args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut guard = self.calls.lock().await;
        *guard += 1;
        Ok(self.value.clone())
    }
}
