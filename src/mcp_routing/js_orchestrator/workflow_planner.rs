//! LLM-driven Workflow Planner
//!
//! Uses LLM to plan workflows and generate JavaScript orchestration code.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::{HashMap, HashSet}, sync::Arc};

use super::{injector::{InjectedMcpFunction, McpFunctionInjector}, validator::JsCodeValidator};
use crate::mcp_routing::decision::{CandidateToolInfo, DecisionEngine};

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

/// An orchestrated tool (JS function combining multiple MCP tools)
#[derive(Debug, Clone)]
pub struct OrchestratedTool {
    pub name: String,
    pub description: String,
    pub js_code: String,
    pub input_schema: serde_json::Value,
    pub mcp_dependencies: Vec<InjectedMcpFunction>,
}

/// Workflow orchestrator
pub struct WorkflowOrchestrator {
    planner: Arc<dyn WorkflowPlannerEngine>,
}

impl WorkflowOrchestrator {
    /// Create a workflow orchestrator backed by the default decision engine
    pub fn new(decision_engine: Arc<DecisionEngine>) -> Self {
        Self {
            planner: decision_engine,
        }
    }

    /// Create a workflow orchestrator from a custom planner implementation (used in tests)
    pub fn with_planner(planner: Arc<dyn WorkflowPlannerEngine>) -> Self {
        Self { planner }
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

        let input_schema = build_input_schema(&plan.input_params);
        let referenced_functions = extract_mcp_dependencies(&js_code);
        let mcp_dependencies = build_mcp_dependency_list(
            &plan,
            available_tools,
            &referenced_functions,
        )
        .context("Failed to map MCP dependencies to real tools")?;

        Ok(OrchestratedTool {
            name: plan.suggested_name.clone(),
            description: plan.description.clone(),
            js_code,
            input_schema,
            mcp_dependencies,
        })
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

fn extract_mcp_dependencies(code: &str) -> Vec<String> {
    static MCP_FN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\bmcp[A-Z][A-Za-z0-9_]*").unwrap());
    let mut seen = HashSet::new();
    let mut deps = Vec::new();

    for capture in MCP_FN.find_iter(code) {
        let name = capture.as_str().to_string();
        if seen.insert(name.clone()) {
            deps.push(name);
        }
    }

    deps
}

fn build_mcp_dependency_list(
    plan: &WorkflowPlan,
    available_tools: &[CandidateToolInfo],
    referenced: &[String],
) -> Result<Vec<InjectedMcpFunction>> {
    if referenced.is_empty() {
        return Ok(Vec::new());
    }

    let mut referenced_set: HashSet<String> = referenced.iter().cloned().collect();
    let mut description_map = HashMap::new();
    for tool in available_tools {
        description_map.insert(
            (tool.server.clone(), tool.tool.clone()),
            tool.description.clone(),
        );
    }

    let mut deps = Vec::new();
    for step in &plan.steps {
        let Some((server, tool_name)) = parse_step_tool(&step.tool) else {
            continue;
        };
        let function_name = McpFunctionInjector::function_name_for(&tool_name);
        if !referenced_set.remove(&function_name) {
            continue;
        }

        let description = description_map
            .get(&(server.clone(), tool_name.clone()))
            .cloned()
            .filter(|desc| !desc.is_empty())
            .or_else(|| {
                if step.description.is_empty() {
                    None
                } else {
                    Some(step.description.clone())
                }
            })
            .unwrap_or_else(|| format!("Workflow step {}", step.step));

        deps.push(InjectedMcpFunction {
            server,
            name: tool_name,
            description,
        });
    }

    if !referenced_set.is_empty() {
        let unknown = referenced_set.into_iter().collect::<Vec<_>>().join(", ");
        return Err(anyhow!(
            "JavaScript references MCP functions missing from workflow plan: {}",
            unknown
        ));
    }

    Ok(deps)
}

fn parse_step_tool(raw: &str) -> Option<(String, String)> {
    let mut parts = raw.splitn(2, "::");
    let server = parts.next()?.trim();
    let tool = parts.next()?.trim();
    if server.is_empty() || tool.is_empty() {
        return None;
    }
    Some((server.to_string(), tool.to_string()))
}

/// Workflow plan from LLM
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowPlan {
    #[serde(default)]
    pub is_feasible: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub suggested_name: String,
    #[serde(default)]
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
    #[serde(default)]
    pub tool: String, // "server::tool_name"
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub dependencies: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InputParam {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub param_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
}
