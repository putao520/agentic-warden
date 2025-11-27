//! Boa JavaScript Engine Runtime Pool
//!
//! Provides a pool of Boa runtime instances with security sandboxing.

use anyhow::{anyhow, Result};
use boa_engine::{builtins::promise::PromiseState, Context, JsError, JsValue, Source};
use crossbeam::channel::{unbounded, Receiver, Sender};
use deadpool::managed::{self, Manager, Metrics, Pool, RecycleError};
use deadpool::Runtime;
use std::sync::{mpsc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;

/// Default minimum number of pooled runtimes kept warm.
const DEFAULT_POOL_MIN_SIZE: usize = 5;
/// Maximum pooled runtimes.
const DEFAULT_POOL_MAX_SIZE: usize = 10;
/// Default pool timeout configuration.
const DEFAULT_POOL_TIMEOUT: Duration = Duration::from_secs(30);

/// Security configuration for Boa runtime
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum execution time in milliseconds (default: 10 minutes)
    pub max_execution_time_ms: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_execution_time_ms: 10 * 60 * 1000, // 10 minutes
        }
    }
}

impl SecurityConfig {
    fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.max_execution_time_ms)
    }
}

/// A secure Boa JavaScript runtime instance (not Send).
struct BoaRuntimeInner {
    context: Context,
    security_config: SecurityConfig,
}

impl BoaRuntimeInner {
    fn new(security_config: SecurityConfig) -> Result<Self> {
        let mut context = Context::default();
        Self::configure_context(&mut context, &security_config)?;
        Ok(Self {
            context,
            security_config,
        })
    }

    fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    fn reset(&mut self) -> Result<()> {
        self.context = Context::default();
        Self::configure_context(&mut self.context, &self.security_config)
    }

    fn execute(&mut self, code: &str) -> Result<serde_json::Value> {
        self.execute_inner(code)
    }

    /// Disable dangerous JavaScript global objects for security
    fn disable_dangerous_globals(context: &mut Context) -> Result<()> {
        let dangerous = [
            "eval",
            "Function",
            "require",
            "import",
            "fetch",
            "XMLHttpRequest",
            "WebSocket",
        ];

        for api in dangerous {
            let code = format!("delete globalThis.{}", api);
            context
                .eval(Source::from_bytes(code.as_bytes()))
                .map_err(|e| anyhow!("Failed to disable {}: {}", api, e))?;
        }

        // Prevent prototype pollution attempts via __proto__ access.
        context
            .eval(Source::from_bytes(
                b"delete Object.prototype.__proto__;" as &[u8],
            ))
            .map_err(|e| anyhow!("Failed to lock down __proto__: {}", e))?;

        Ok(())
    }

    fn configure_context(context: &mut Context, _config: &SecurityConfig) -> Result<()> {
        Self::disable_dangerous_globals(context)?;
        // Boa engine limits removed per SPEC REQ-013 update.
        Ok(())
    }

    fn execute_inner(&mut self, code: &str) -> Result<serde_json::Value> {
        let source = Source::from_bytes(code);
        let value = match self.context.eval(source) {
            Ok(value) => value,
            Err(err) => return Err(self.handle_js_error(err)),
        };

        if let Err(err) = self.context.run_jobs() {
            let js_err = JsError::from_opaque(err.to_opaque(&mut self.context));
            return Err(self.handle_js_error(js_err));
        }

        if value.is_promise() {
            self.resolve_promise(value.clone())
        } else {
            self.js_value_to_json(&value)
        }
    }

    fn resolve_promise(&mut self, value: JsValue) -> Result<serde_json::Value> {
        let promise = value
            .as_promise()
            .ok_or_else(|| anyhow!("Expected a promise value"))?;

        match promise.state() {
            PromiseState::Pending => {
                Err(anyhow!("JavaScript promise did not settle before timeout"))
            }
            PromiseState::Fulfilled(result) => self.js_value_to_json(&result),
            PromiseState::Rejected(reason) => {
                let message = reason
                    .to_string(&mut self.context)
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_else(|_| "<unknown>".to_owned());
                Err(anyhow!("JavaScript promise rejected: {}", message))
            }
        }
    }

    fn handle_js_error(&mut self, err: JsError) -> anyhow::Error {
        let message = err.to_string();
        if Self::is_limit_error(&message) {
            let _ = self.reset();
            return anyhow!(
                "JavaScript execution timed out after {} ms",
                self.security_config.max_execution_time_ms
            );
        }

        anyhow!("JS execution failed: {}", message)
    }

    fn is_limit_error(message: &str) -> bool {
        let lowered = message.to_ascii_lowercase();
        lowered.contains("loop iteration limit")
            || lowered.contains("recursion limit")
            || lowered.contains("stack overflow")
    }

    /// Convert Boa JsValue to serde_json::Value
    fn js_value_to_json(&mut self, value: &JsValue) -> Result<serde_json::Value> {
        match value
            .to_json(&mut self.context)
            .map_err(|e| anyhow!("Failed to convert JS value: {}", e))?
        {
            Some(json) => Ok(json),
            None => Ok(serde_json::Value::Null),
        }
    }
}

/// Commands sent to the runtime worker thread.
enum RuntimeCommand {
    Execute {
        code: String,
        responder: oneshot::Sender<Result<serde_json::Value>>,
    },
    WithContext {
        job: Box<dyn FnOnce(&mut BoaRuntimeInner) -> Result<()> + Send>,
        responder: oneshot::Sender<Result<()>>,
    },
    Reset {
        responder: oneshot::Sender<Result<()>>,
    },
    Shutdown,
}

/// Handle to the Boa runtime worker.
pub struct BoaRuntime {
    sender: Sender<RuntimeCommand>,
    security_config: SecurityConfig,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl BoaRuntime {
    /// Create a new Boa runtime with default security sandbox
    pub fn new() -> Result<Self> {
        Self::with_security(SecurityConfig::default())
    }

    /// Create a runtime with a custom security configuration.
    pub fn with_security(security_config: SecurityConfig) -> Result<Self> {
        let (sender, handle) = spawn_worker(security_config.clone())?;
        Ok(Self {
            sender,
            security_config,
            thread: Mutex::new(Some(handle)),
        })
    }

    /// Execute JavaScript code with timeout and sandbox enforcement.
    pub async fn execute(&self, code: &str) -> Result<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(RuntimeCommand::Execute {
                code: code.to_owned(),
                responder: tx,
            })
            .map_err(|_| anyhow!("Boa runtime worker unavailable"))?;

        match timeout(self.security_config.timeout_duration(), async { rx.await }).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(anyhow!("Boa runtime worker dropped response")),
            Err(_) => {
                let _ = self.send_reset().await;
                Err(anyhow!(
                    "JavaScript execution timed out after {} ms",
                    self.security_config.max_execution_time_ms
                ))
            }
        }
    }

    /// Execute a closure with exclusive access to the Boa context.
    pub async fn with_context<F>(&self, job: F) -> Result<()>
    where
        F: FnOnce(&mut Context) -> Result<()> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(RuntimeCommand::WithContext {
                job: Box::new(move |worker| job(worker.context_mut())),
                responder: tx,
            })
            .map_err(|_| anyhow!("Boa runtime worker unavailable"))?;

        rx.await
            .map_err(|_| anyhow!("Boa runtime worker dropped response"))?
    }

    pub async fn reset(&self) -> Result<()> {
        self.send_reset().await
    }

    async fn send_reset(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(RuntimeCommand::Reset { responder: tx })
            .map_err(|_| anyhow!("Boa runtime worker unavailable"))?;
        rx.await
            .map_err(|_| anyhow!("Boa runtime worker dropped response"))?
    }
}

impl Drop for BoaRuntime {
    fn drop(&mut self) {
        let _ = self.sender.send(RuntimeCommand::Shutdown);
        if let Some(handle) = self.thread.lock().ok().and_then(|mut h| h.take()) {
            let _ = handle.join();
        }
    }
}

/// Connection pool for Boa runtime instances backed by `deadpool`.
pub struct BoaRuntimePool {
    pool: Pool<BoaRuntimeManager>,
    #[allow(dead_code)]
    security: SecurityConfig,
}

impl BoaRuntimePool {
    /// Create a new Boa runtime pool with default security configuration.
    pub async fn new() -> Result<Self> {
        Self::with_security(SecurityConfig::default()).await
    }

    /// Create a runtime pool with the provided security configuration.
    pub async fn with_security(security: SecurityConfig) -> Result<Self> {
        let manager = BoaRuntimeManager {
            security_config: security.clone(),
        };

        let pool = Pool::builder(manager)
            .max_size(DEFAULT_POOL_MAX_SIZE)
            .runtime(Runtime::Tokio1)
            .wait_timeout(Some(DEFAULT_POOL_TIMEOUT))
            .create_timeout(Some(DEFAULT_POOL_TIMEOUT))
            .recycle_timeout(Some(DEFAULT_POOL_TIMEOUT))
            .build()
            .map_err(|err| anyhow!("Failed to build Boa runtime pool: {}", err))?;

        let pool_wrapper = Self { pool, security };
        pool_wrapper.prime_minimum_runtimes().await?;
        Ok(pool_wrapper)
    }

    async fn prime_minimum_runtimes(&self) -> Result<()> {
        let mut handles = Vec::with_capacity(DEFAULT_POOL_MIN_SIZE);
        for _ in 0..DEFAULT_POOL_MIN_SIZE {
            handles.push(
                self.pool
                    .get()
                    .await
                    .map_err(|err| anyhow!("Failed to warm up Boa pool: {}", err))?,
            );
        }
        drop(handles);
        Ok(())
    }

    /// Get a runtime from the pool
    pub async fn acquire(&self) -> Result<PooledBoaRuntime> {
        self.pool
            .get()
            .await
            .map_err(|err| anyhow!("Failed to acquire Boa runtime: {}", err))
    }

    /// Inspect current pool status (mostly for testing and observability).
    pub fn status(&self) -> deadpool::Status {
        self.pool.status()
    }
}

/// Managed Boa runtime handle used by the pool.
pub type PooledBoaRuntime = managed::Object<BoaRuntimeManager>;

pub struct BoaRuntimeManager {
    security_config: SecurityConfig,
}

impl Manager for BoaRuntimeManager {
    type Type = BoaRuntime;
    type Error = anyhow::Error;

    fn create(&self) -> impl std::future::Future<Output = Result<Self::Type, Self::Error>> + Send {
        let config = self.security_config.clone();
        async move { BoaRuntime::with_security(config) }
    }

    fn recycle(
        &self,
        obj: &mut Self::Type,
        _metrics: &Metrics,
    ) -> impl std::future::Future<Output = managed::RecycleResult<Self::Error>> + Send {
        async move {
            obj.reset()
                .await
                .map_err(|err| RecycleError::Backend(err.into()))?;
            Ok(())
        }
    }
}

fn spawn_worker(
    security_config: SecurityConfig,
) -> Result<(Sender<RuntimeCommand>, JoinHandle<()>)> {
    let (sender, receiver) = unbounded();
    let (init_tx, init_rx) = mpsc::channel();

    let handle = thread::Builder::new()
        .name("boa-runtime-worker".into())
        .spawn(move || match BoaRuntimeInner::new(security_config) {
            Ok(mut worker) => {
                let _ = init_tx.send(Ok(()));
                worker_loop(&mut worker, receiver);
            }
            Err(err) => {
                let _ = init_tx.send(Err(err));
            }
        })
        .map_err(|err| anyhow!("Failed to spawn Boa runtime worker: {err}"))?;

    init_rx
        .recv()
        .map_err(|_| anyhow!("Boa runtime worker failed to initialize"))??;

    Ok((sender, handle))
}

fn worker_loop(worker: &mut BoaRuntimeInner, receiver: Receiver<RuntimeCommand>) {
    for command in receiver {
        match command {
            RuntimeCommand::Execute { code, responder } => {
                let _ = responder.send(worker.execute(&code));
            }
            RuntimeCommand::WithContext { job, responder } => {
                let _ = responder.send(job(worker));
            }
            RuntimeCommand::Reset { responder } => {
                let _ = responder.send(worker.reset());
            }
            RuntimeCommand::Shutdown => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_boa_runtime_creation() {
        let runtime = BoaRuntime::new().expect("runtime");
        let result = runtime.execute("1+1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dangerous_globals_disabled() {
        let runtime = BoaRuntime::new().unwrap();
        let result = runtime.execute("typeof eval").await.unwrap();
        assert_eq!(result, serde_json::Value::String("undefined".into()));
    }
}
