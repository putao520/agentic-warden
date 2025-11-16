//! JavaScript Orchestration Module (REQ-013)
//!
//! Provides LLM-driven workflow planning and JS code generation
//! to orchestrate multiple MCP tools into a single callable function.

pub mod engine;
pub mod injector;
pub mod prompts;
pub mod schema_corrector;
pub mod schema_validator;
pub mod validator;
pub mod workflow_planner;

pub use engine::{BoaRuntime, BoaRuntimePool, SecurityConfig};
pub use injector::{McpFunctionInjector, McpToolInvoker};
pub use schema_corrector::{IterativeSchemaFixer, SchemaCorrectionResult, SchemaCorrector};
pub use schema_validator::{SchemaValidationResult, SchemaValidator};
pub use validator::{JsCodeValidator, ValidationResult};
pub use workflow_planner::{OrchestratedTool, WorkflowOrchestrator};
