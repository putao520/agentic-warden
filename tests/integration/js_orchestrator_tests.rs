use std::sync::Arc;
use std::time::Duration;

use agentic_warden::mcp_routing::js_orchestrator::{
    BoaRuntime, BoaRuntimePool, McpFunctionInjector, McpToolInvoker, SecurityConfig,
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

    let injector_clone = injector.clone();
    let handle = tokio::runtime::Handle::current();
    runtime
        .with_context(move |ctx| {
            injector_clone.inject(ctx, handle.clone())?;
            Ok(())
        })
        .await
        .unwrap();

    let output = runtime
        .execute(
            r#"
            async function workflow() {
                const status = await mcp.call("mock", "git_status", { repo: "demo" });
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
