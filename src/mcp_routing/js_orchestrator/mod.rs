//! JavaScript Orchestration Module (REQ-013)
//!
//! Provides LLM-driven workflow planning and JS code generation
//! to orchestrate multiple MCP tools into a single callable function.

pub mod engine;
pub mod injector;
pub mod validator;
pub mod workflow_planner;

pub use engine::{BoaRuntime, BoaRuntimePool, SecurityConfig};
pub use injector::{InjectedMcpFunction, McpFunctionInjector, McpToolInvoker};
pub use validator::{JsCodeValidator, ValidationResult};
pub use workflow_planner::{OrchestratedTool, WorkflowOrchestrator};
