//! MCP Function Injector
//!
//! Injects MCP tools as JavaScript async functions into Boa runtime.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use boa_engine::{
    job::NativeAsyncJob, js_string, object::builtins::JsPromise, property::Attribute,
    property::PropertyKey, Context, JsError, JsResult, JsString, JsValue, NativeFunction,
};
use boa_gc::{custom_trace, Finalize, Trace};
use serde_json::Value;
use std::{collections::HashSet, sync::Arc};
use tokio::{runtime::Handle, sync::oneshot};

use crate::mcp_routing::pool::McpConnectionPool;

/// Information about an MCP function to be injected
#[derive(Debug, Clone)]
pub struct InjectedMcpFunction {
    pub server: String,
    pub name: String,
    pub description: String,
}

#[async_trait]
pub trait McpToolInvoker: Send + Sync {
    async fn call_tool(&self, server: &str, tool_name: &str, args: Value) -> Result<Value>;
}

#[async_trait]
impl McpToolInvoker for McpConnectionPool {
    async fn call_tool(&self, server: &str, tool_name: &str, args: Value) -> Result<Value> {
        McpConnectionPool::call_tool(self, server, tool_name, args).await
    }
}

/// MCP function injector
#[derive(Clone)]
pub struct McpFunctionInjector {
    pool: Arc<dyn McpToolInvoker>,
}

impl McpFunctionInjector {
    /// Create a new MCP function injector
    pub fn new(pool: Arc<McpConnectionPool>) -> Self {
        Self { pool }
    }

    /// Construct an injector from a custom invoker (mainly for testing).
    pub fn with_invoker(invoker: Arc<dyn McpToolInvoker>) -> Self {
        Self { pool: invoker }
    }

    /// Inject MCP tools as JavaScript functions
    ///
    /// Each tool is injected with naming convention: `mcp` + CamelCase
    /// Example: `git_status` → `mcpGitStatus`
    pub fn inject_all(
        &self,
        context: &mut Context,
        tools: &[InjectedMcpFunction],
        handle: Handle,
    ) -> Result<()> {
        Self::validate_unique_names(tools)?;

        for tool in tools {
            let function_name = Self::function_name_for(&tool.name);
            if Self::is_function_registered(context, &function_name)? {
                continue;
            }
            let captures = BoundToolContext {
                invoker: Arc::clone(&self.pool),
                server: tool.server.clone(),
                tool: tool.name.clone(),
                handle: handle.clone(),
            };

            let native = NativeFunction::from_copy_closure_with_captures(
                |_, args, binding: &BoundToolContext, context| {
                    let request = Self::args_to_json(args, context)?;
                    let (promise, resolvers) = JsPromise::new_pending(context);
                    let (tx, rx) = oneshot::channel();

                    let invoker = Arc::clone(&binding.invoker);
                    let server = binding.server.clone();
                    let tool = binding.tool.clone();
                    let tokio_handle = binding.handle.clone();

                    tokio_handle.spawn(async move {
                        let response = invoker.call_tool(&server, &tool, request).await;
                        let _ = tx.send(response);
                    });

                    context.enqueue_job(
                        NativeAsyncJob::new(async move |ctx_ref| {
                            let result = rx.await.map_err(|_| {
                                Self::js_error("MCP worker cancelled before returning a result")
                            })?;

                            let mut ctx = ctx_ref.borrow_mut();
                            match result {
                                Ok(value) => {
                                    let js_value = JsValue::from_json(&value, &mut ctx)?;
                                    resolvers
                                        .resolve
                                        .call(&JsValue::undefined(), &[js_value], &mut ctx)
                                        .map(|_| JsValue::undefined())
                                }
                                Err(err) => {
                                    let error_value =
                                        JsValue::from(JsString::from(err.to_string()));
                                    resolvers
                                        .reject
                                        .call(&JsValue::undefined(), &[error_value], &mut ctx)
                                        .map(|_| JsValue::undefined())
                                }
                            }
                        })
                        .into(),
                    );

                    Ok(promise.into())
                },
                captures,
            );

            let js_function = native.to_js_function(context.realm());
            context
                .register_global_property(
                    js_string!(function_name.as_str()),
                    js_function,
                    Attribute::all(),
                )
                .map_err(|err| anyhow!("Failed to register {}: {}", function_name, err))?;
        }

        Ok(())
    }

    fn validate_unique_names(tools: &[InjectedMcpFunction]) -> Result<()> {
        let mut seen = HashSet::new();
        for tool in tools {
            let name = Self::function_name_for(&tool.name);
            if !seen.insert(name.clone()) {
                return Err(anyhow!("Duplicate MCP function detected: {}", name));
            }
        }
        Ok(())
    }

    fn args_to_json(args: &[JsValue], context: &mut Context) -> JsResult<Value> {
        let value = args.get(0).cloned().unwrap_or_else(JsValue::undefined);
        let json = match value.to_json(context)? {
            Some(json) => json,
            None => Value::Null,
        };

        if !(json.is_null() || json.is_object()) {
            return Err(Self::js_error(
                "MCP tools expect an object argument or null",
            ));
        }

        Ok(json)
    }

    pub fn function_name_for(raw_name: &str) -> String {
        let camel = Self::to_camel_case(raw_name);
        let mut chars = camel.chars();
        let mut result = String::from("mcp");
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            result.extend(chars);
        } else {
            result.push_str("Tool");
        }
        result
    }

    fn is_function_registered(context: &mut Context, name: &str) -> Result<bool> {
        let key: PropertyKey = JsString::from(name).into();
        context
            .global_object()
            .has_property(key, context)
            .map_err(|err| anyhow!("Failed to inspect global scope for {name}: {err}"))
    }

    fn js_error(message: impl Into<String>) -> JsError {
        JsError::from_opaque(JsValue::from(JsString::from(message.into())))
    }

    /// Convert snake_case to CamelCase
    #[allow(dead_code)]
    fn to_camel_case(name: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for ch in name.chars() {
            if ch == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(ch);
            }
        }

        result
    }
}

#[derive(Clone)]
struct BoundToolContext {
    invoker: Arc<dyn McpToolInvoker>,
    server: String,
    tool: String,
    handle: Handle,
}

#[allow(unused_variables)]
unsafe impl Trace for BoundToolContext {
    custom_trace!(this, _visitor, {});
}

impl Finalize for BoundToolContext {}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
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
        async fn call_tool(&self, _server: &str, _tool_name: &str, _args: Value) -> Result<Value> {
            let mut guard = self.calls.lock().await;
            *guard += 1;
            Ok(self.value.clone())
        }
    }

    #[tokio::test]
    async fn test_to_camel_case() {
        assert_eq!(
            McpFunctionInjector::to_camel_case("git_status"),
            "gitStatus"
        );
        assert_eq!(McpFunctionInjector::to_camel_case("read_file"), "readFile");
        assert_eq!(
            McpFunctionInjector::to_camel_case("list_tools"),
            "listTools"
        );
    }

    #[tokio::test]
    async fn test_function_injection_and_call() {
        use crate::mcp_routing::js_orchestrator::engine::BoaRuntime;

        let invoker = Arc::new(MockInvoker::new(json!({"ok": true})));
        let injector = McpFunctionInjector::with_invoker(invoker.clone());
        let runtime = BoaRuntime::new().unwrap();

        let tools = vec![InjectedMcpFunction {
            server: "mock".into(),
            name: "git_status".into(),
            description: "mock".into(),
        }];
        let injector_clone = injector.clone();
        let handle = Handle::current();
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
                    const status = await mcpGitStatus({ repo: "test" });
                    return status.ok;
                }
                workflow();
                "#,
            )
            .await
            .unwrap();

        assert_eq!(output, json!(true));
        assert_eq!(*invoker.calls.lock().await, 1);
    }
}
