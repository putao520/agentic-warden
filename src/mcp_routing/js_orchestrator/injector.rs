//! MCP Function Injector
//!
//! Injects a unified `mcp.call(server, tool, args)` API into Boa runtime instances.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use boa_engine::{
    job::NativeAsyncJob, js_string, object::builtins::JsPromise, object::ObjectInitializer,
    property::Attribute, property::PropertyKey, Context, JsError, JsResult, JsString, JsValue,
    NativeFunction,
};
use boa_gc::{custom_trace, Finalize, Trace};
use serde_json::Value;
use std::sync::Arc;
use tokio::{runtime::Handle, sync::oneshot};

use crate::mcp_routing::pool::McpConnectionPool;

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

    /// Inject a unified `mcp.call(server, tool, args)` function into the JS runtime.
    pub fn inject(&self, context: &mut Context, handle: Handle) -> Result<()> {
        if Self::is_mcp_registered(context)? {
            return Ok(());
        }

        let captures = BoundCallContext {
            invoker: Arc::clone(&self.pool),
            handle,
        };

        let native = NativeFunction::from_copy_closure_with_captures(
            |_, args, binding: &BoundCallContext, context| {
                let (server, tool, payload) = Self::parse_call_args(args, context)?;
                let (promise, resolvers) = JsPromise::new_pending(context);
                let (tx, rx) = oneshot::channel();

                let invoker = Arc::clone(&binding.invoker);
                let tokio_handle = binding.handle.clone();

                tokio_handle.spawn(async move {
                    let response = invoker.call_tool(&server, &tool, payload).await;
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
                                let error_value = JsValue::from(JsString::from(err.to_string()));
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

        let call_function = native.to_js_function(context.realm());
        let mcp_object = ObjectInitializer::new(context)
            .property(js_string!("call"), call_function, Attribute::all())
            .build();
        context
            .register_global_property(js_string!("mcp"), mcp_object, Attribute::all())
            .map_err(|err| anyhow!("Failed to register global mcp object: {err}"))?;

        Ok(())
    }

    fn parse_call_args(
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<(String, String, Value)> {
        let server_value = args.get(0).cloned().unwrap_or_else(JsValue::undefined);
        let tool_value = args.get(1).cloned().unwrap_or_else(JsValue::undefined);
        let payload_value = args.get(2).cloned();

        let server = server_value.to_string(context)?.to_std_string_escaped();
        if server.trim().is_empty() {
            return Err(Self::js_error(
                "mcp.call(server, tool, args) requires a non-empty server name",
            ));
        }

        let tool = tool_value.to_string(context)?.to_std_string_escaped();
        if tool.trim().is_empty() {
            return Err(Self::js_error(
                "mcp.call(server, tool, args) requires a non-empty tool name",
            ));
        }

        let payload = Self::payload_to_json(payload_value, context)?;
        Ok((server, tool, payload))
    }

    fn payload_to_json(value: Option<JsValue>, context: &mut Context) -> JsResult<Value> {
        let json_value = match value.unwrap_or_else(JsValue::undefined).to_json(context)? {
            Some(json) => json,
            None => Value::Null,
        };

        if !(json_value.is_null() || json_value.is_object()) {
            return Err(Self::js_error(
                "mcp.call expects an object payload or null/undefined",
            ));
        }

        Ok(json_value)
    }

    fn is_mcp_registered(context: &mut Context) -> Result<bool> {
        let key: PropertyKey = JsString::from("mcp").into();
        context
            .global_object()
            .has_property(key, context)
            .map_err(|err| anyhow!("Failed to inspect global scope for mcp: {err}"))
    }

    fn js_error(message: impl Into<String>) -> JsError {
        JsError::from_opaque(JsValue::from(JsString::from(message.into())))
    }
}

#[derive(Clone)]
struct BoundCallContext {
    invoker: Arc<dyn McpToolInvoker>,
    handle: Handle,
}

#[allow(unused_variables)]
unsafe impl Trace for BoundCallContext {
    custom_trace!(this, _visitor, {});
}

impl Finalize for BoundCallContext {}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use tokio::sync::Mutex as AsyncMutex;

    use crate::mcp_routing::js_orchestrator::engine::BoaRuntime;

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
    async fn test_mcp_call_injection_and_invocation() {
        let invoker = Arc::new(MockInvoker::new(json!({"ok": true})));
        let injector = McpFunctionInjector::with_invoker(invoker.clone());
        let runtime = BoaRuntime::new().unwrap();
        let handle = Handle::current();
        let injector_clone = injector.clone();

        runtime
            .with_context(move |ctx| injector_clone.inject(ctx, handle.clone()))
            .await
            .unwrap();

        let output = runtime
            .execute(
                r#"
                async function workflow() {
                    const status = await mcp.call("mock", "git_status", { repo: "test" });
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

    #[tokio::test]
    async fn rejects_missing_server_or_tool() {
        let invoker = Arc::new(MockInvoker::new(json!({"ok": true})));
        let injector = McpFunctionInjector::with_invoker(invoker.clone());
        let runtime = BoaRuntime::new().unwrap();
        let handle = Handle::current();
        runtime
            .with_context(move |ctx| injector.inject(ctx, handle.clone()))
            .await
            .unwrap();

        let result = runtime
            .execute(
                r#"
                async function workflow() {
                    return await mcp.call("", "", {});
                }
                workflow();
                "#,
            )
            .await;

        assert!(result.is_err());
    }
}
