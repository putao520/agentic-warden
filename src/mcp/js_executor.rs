//! JS workflow execution helper.
//!
//! Encapsulates Boa runtime interactions and MCP injector wiring
//! for executing orchestrated JS workflows.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use tokio::runtime::Handle;

use crate::mcp_routing::js_orchestrator::engine::BoaRuntimePool;
use crate::mcp_routing::js_orchestrator::injector::McpFunctionInjector;
use crate::mcp_routing::registry::JsOrchestratedTool;

pub struct JsToolExecutor {
    runtime_pool: Arc<BoaRuntimePool>,
    injector: Arc<McpFunctionInjector>,
}

#[derive(Debug)]
pub struct JsExecutionReport {
    pub output: Value,
    pub duration_ms: u128,
}

impl JsToolExecutor {
    pub fn new(runtime_pool: Arc<BoaRuntimePool>, injector: Arc<McpFunctionInjector>) -> Self {
        Self {
            runtime_pool,
            injector,
        }
    }

    pub async fn execute(
        &self,
        tool: &JsOrchestratedTool,
        input: Value,
    ) -> Result<JsExecutionReport> {
        let runtime = self
            .runtime_pool
            .acquire()
            .await
            .context("Failed to lock Boa runtime from pool")?;
        let handle = Handle::current();
        let injector = Arc::clone(&self.injector);
        runtime
            .with_context(move |ctx| injector.inject(ctx, handle.clone()))
            .await
            .context("Failed to inject MCP functions into Boa runtime")?;

        let script = build_invocation_script(&tool.js_code, &input)?;
        let start = Instant::now();
        let output = runtime
            .execute(&script)
            .await
            .map_err(|e| anyhow!("Workflow '{}' execution failed: {}", tool.tool.name, e))?;
        Ok(JsExecutionReport {
            output,
            duration_ms: start.elapsed().as_millis(),
        })
    }
}

fn build_invocation_script(code: &str, input: &Value) -> Result<String> {
    let payload = serde_json::to_string(input)?;
    if !code.contains("async function workflow") {
        return Err(anyhow!(
            "Generated JS code must define `async function workflow`"
        ));
    }

    Ok(format!(
        "const __agenticInput = {payload};\n{code}\nif (typeof workflow !== 'function') {{ throw new Error('workflow() must be defined'); }}\nworkflow(__agenticInput);",
        payload = payload,
        code = code
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_routing::js_orchestrator::injector::McpToolInvoker;
    use crate::mcp_routing::registry::ToolMetadata;
    use anyhow::Result as AnyResult;
    use async_trait::async_trait;
    use rmcp::model::Tool;
    use serde_json::json;
    use tokio::sync::Mutex as AsyncMutex;

    struct MockInvoker {
        value: Value,
        calls: AsyncMutex<usize>,
    }

    impl MockInvoker {
        fn new(value: Value) -> Self {
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
            _args: Value,
        ) -> AnyResult<Value> {
            let mut guard = self.calls.lock().await;
            *guard += 1;
            Ok(self.value.clone())
        }
    }

    fn build_tool(name: &str) -> Tool {
        Tool {
            name: name.to_string().into(),
            title: None,
            description: Some("test".into()),
            input_schema: Arc::new(serde_json::Map::new()),
            output_schema: None,
            icons: None,
            annotations: None,
            execution: None,
            meta: None,
        }
    }

    #[tokio::test]
    async fn executor_runs_workflow() {
        let pool = Arc::new(BoaRuntimePool::new().await.unwrap());
        let invoker = Arc::new(MockInvoker::new(json!({"status": "ok"})));
        let injector = Arc::new(McpFunctionInjector::with_invoker(invoker.clone()));
        let executor = JsToolExecutor::new(pool, injector);

        let tool = JsOrchestratedTool {
            tool: build_tool("workflow"),
            js_code: r#"
async function workflow(input) {
    const status = await mcp.call("mock", "sample", { value: input.value });
    return status.status;
}
"#
            .into(),
            metadata: ToolMetadata::new(60),
        };

        let report = executor
            .execute(&tool, json!({"value": 1}))
            .await
            .expect("execution");
        assert_eq!(report.output, json!("ok"));
        assert!(report.duration_ms <= 1000);
    }
}
