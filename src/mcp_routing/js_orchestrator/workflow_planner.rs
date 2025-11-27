//! LLM-driven Workflow Planner
//!
//! Uses LLM to plan workflows and generate JavaScript orchestration code.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use super::{
    schema_corrector::{IterativeSchemaFixer, SchemaCorrector},
    schema_validator::SchemaValidator,
    validator::JsCodeValidator,
};
use crate::mcp_routing::decision::{CandidateToolInfo, DecisionEngine};

/// Deserialize null as empty string
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Abstraction over engines capable of planning workflows and generating code.
#[async_trait]
pub trait WorkflowPlannerEngine: Send + Sync {
    async fn plan_workflow(
        &self,
        user_request: &str,
        available_tools: &[CandidateToolInfo],
    ) -> Result<WorkflowPlan>;

    async fn generate_js_code(&self, plan: &WorkflowPlan) -> Result<String>;
}

/// Information for direct proxy registration (single tool, no JS)
#[derive(Debug, Clone)]
pub struct ProxyToolInfo {
    pub server: String,
    pub tool_name: String,
}

/// An orchestrated tool result
#[derive(Debug, Clone)]
pub struct OrchestratedTool {
    pub name: String,
    pub description: String,
    /// None = direct proxy (use proxy_info), Some = JS orchestrated
    pub js_code: Option<String>,
    pub input_schema: serde_json::Value,
    /// Present when needs_orchestration=false (direct proxy)
    pub proxy_info: Option<ProxyToolInfo>,
}

/// Workflow orchestrator
pub struct WorkflowOrchestrator {
    planner: Arc<dyn WorkflowPlannerEngine>,
    decision_engine: Option<Arc<DecisionEngine>>,
}

impl WorkflowOrchestrator {
    /// Create a workflow orchestrator backed by the default decision engine
    pub fn new(decision_engine: Arc<DecisionEngine>) -> Self {
        Self {
            planner: decision_engine.clone(),
            decision_engine: Some(decision_engine),
        }
    }

    /// Create a workflow orchestrator from a custom planner implementation (used in tests)
    pub fn with_planner(planner: Arc<dyn WorkflowPlannerEngine>) -> Self {
        Self {
            planner,
            decision_engine: None,
        }
    }

    /// Orchestrate a workflow from user request
    ///
    /// Steps:
    /// 1. LLM plans workflow (feasibility + steps)
    /// 2. LLM generates JS code
    /// 3. Validate code
    /// 4. Return orchestrated tool definition
    pub async fn orchestrate(
        &self,
        user_request: &str,
        available_tools: &[CandidateToolInfo],
    ) -> Result<OrchestratedTool> {
        if user_request.trim().is_empty() {
            return Err(anyhow!("user_request cannot be empty"));
        }
        if available_tools.is_empty() {
            return Err(anyhow!("No MCP tools supplied for workflow orchestration"));
        }

        let plan = self
            .planner
            .plan_workflow(user_request, available_tools)
            .await
            .context("Workflow planning failed")?;

        if !plan.is_feasible {
            let reason = if plan.reason.trim().is_empty() {
                "LLM did not provide a reason".to_string()
            } else {
                plan.reason.clone()
            };
            return Err(anyhow!("Workflow is not feasible: {reason}"));
        }

        // Optimization: If needs_orchestration=false and single step, use direct proxy
        if !plan.needs_orchestration && plan.steps.len() == 1 {
            let step = &plan.steps[0];
            // Parse "server::tool_name" format
            let parts: Vec<&str> = step.tool.split("::").collect();
            if parts.len() == 2 {
                let server = parts[0].to_string();
                let tool_name = parts[1].to_string();

                eprintln!(
                    "   ⚡ [OPTIMIZATION] Direct proxy mode: {}::{}",
                    server, tool_name
                );

                // Build schema from plan's input_params
                let input_schema = build_input_schema(&plan.input_params);

                return Ok(OrchestratedTool {
                    name: plan.suggested_name.clone(),
                    description: plan.description.clone(),
                    js_code: None, // No JS needed
                    input_schema,
                    proxy_info: Some(ProxyToolInfo { server, tool_name }),
                });
            }
        }

        // Full JS orchestration path
        eprintln!("   🔧 [ORCHESTRATION] Generating JS workflow...");

        let js_code = self
            .planner
            .generate_js_code(&plan)
            .await
            .context("JavaScript code generation failed")?;

        let validation = JsCodeValidator::validate(&js_code)
            .context("Failed to validate generated JavaScript")?;
        if !validation.passed {
            let message = if validation.errors.is_empty() {
                "unknown validation failure".to_string()
            } else {
                validation.errors.join("; ")
            };
            return Err(anyhow!(
                "Generated workflow JavaScript failed validation: {message}"
            ));
        }

        let built_schema = build_input_schema(&plan.input_params);
        let input_schema = match self
            .decision_engine
            .as_ref()
            .map(|engine| IterativeSchemaFixer::new(Arc::clone(engine)))
        {
            Some(schema_fixer) => match schema_fixer
                .fix_schema_with_retry(
                    &plan.suggested_name,
                    &plan.description,
                    &js_code,
                    built_schema.clone(),
                )
                .await
            {
                Ok(schema) => schema,
                Err(e) => {
                    eprintln!("⚠️  Iterative schema fixing failed: {}", e);
                    eprintln!("ℹ️  Falling back to static SchemaCorrector");
                    self.fallback_schema_correction(&js_code, built_schema)?
                }
            },
            None => self.fallback_schema_correction(&js_code, built_schema)?,
        };

        Ok(OrchestratedTool {
            name: plan.suggested_name.clone(),
            description: plan.description.clone(),
            js_code: Some(js_code),
            input_schema,
            proxy_info: None,
        })
    }
}

impl WorkflowOrchestrator {
    fn fallback_schema_correction(&self, js_code: &str, schema: Value) -> Result<Value> {
        let validation = SchemaValidator::validate(&schema);
        if validation.is_valid {
            if !validation.warnings.is_empty() {
                eprintln!(
                    "⚠️  Input schema warnings: {}",
                    validation.warnings.join("; ")
                );
            }
            return Ok(schema);
        }

        eprintln!(
            "⚠️  Input schema failed validation, attempting autocorrect: {}",
            validation.errors.join("; ")
        );
        let corrected = SchemaCorrector::correct(js_code, schema.clone())
            .context("Failed to self-correct workflow input schema from generated code")?;
        if !corrected.applied_fixes.is_empty() {
            eprintln!(
                "ℹ️  Applied schema fixes: {}",
                corrected.applied_fixes.join("; ")
            );
        }
        if !corrected.warnings.is_empty() {
            eprintln!(
                "⚠️  Schema warnings after correction: {}",
                corrected.warnings.join("; ")
            );
        }

        Ok(corrected.schema)
    }
}

fn build_input_schema(params: &[InputParam]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for param in params {
        let mut schema = serde_json::Map::new();
        schema.insert("type".into(), Value::String(param.param_type.clone()));
        if !param.description.is_empty() {
            schema.insert(
                "description".into(),
                Value::String(param.description.clone()),
            );
        }
        properties.insert(param.name.clone(), Value::Object(schema));
        if param.required {
            required.push(Value::String(param.name.clone()));
        }
    }

    let mut root = serde_json::Map::new();
    root.insert("type".into(), Value::String("object".into()));
    root.insert("properties".into(), Value::Object(properties));
    if !required.is_empty() {
        root.insert("required".into(), Value::Array(required));
    }

    Value::Object(root)
}

/// Workflow plan from LLM
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowPlan {
    #[serde(default)]
    pub is_feasible: bool,
    /// Whether JS orchestration is needed (multi-step, data transform, loops, etc.)
    /// false = single tool direct proxy, true = generate JS workflow
    #[serde(default)]
    pub needs_orchestration: bool,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub reason: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub suggested_name: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub description: String,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
    #[serde(default)]
    pub input_params: Vec<InputParam>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowStep {
    #[serde(default)]
    pub step: usize,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub tool: String, // "server::tool_name"
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub description: String,
    #[serde(default)]
    pub dependencies: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InputParam {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub name: String,
    #[serde(
        rename = "type",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub param_type: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub description: String,
    #[serde(default)]
    pub required: bool,
}
