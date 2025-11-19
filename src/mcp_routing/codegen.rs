//! Code Generation Abstraction
//!
//! Unified interface for workflow planning and JS code generation.
//! Supports multiple backends: Ollama (local LLM) and AI CLI (claude/codex/gemini).

use crate::cli_type::CliType;
use crate::mcp_routing::decision::{CandidateToolInfo, DecisionEngine};
use crate::mcp_routing::js_orchestrator::workflow_planner::{WorkflowPlan, WorkflowPlannerEngine};
use crate::supervisor;
use crate::registry_factory::create_cli_registry;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// Code generator backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenBackend {
    Ollama,
    AiCli,
}

impl CodegenBackend {
    /// Auto-detect backend from environment
    /// - If OPENAI_TOKEN exists → Ollama mode
    /// - Otherwise → AI CLI mode (default: claude)
    pub fn from_env() -> Self {
        if std::env::var("OPENAI_TOKEN").is_ok() {
            Self::Ollama
        } else {
            Self::AiCli
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::AiCli => "ai-cli",
        }
    }
}

/// Factory for creating code generators based on configuration
pub struct CodeGeneratorFactory;

impl CodeGeneratorFactory {
    /// Create code generator from environment variables
    pub fn from_env(
        default_endpoint: String,
        default_model: String,
    ) -> Result<Arc<dyn WorkflowPlannerEngine>> {
        let backend = CodegenBackend::from_env();

        match backend {
            CodegenBackend::Ollama => {
                Self::create_ollama_generator(default_endpoint, default_model)
            }
            CodegenBackend::AiCli => Self::create_ai_cli_generator(),
        }
    }

    /// Create Ollama-based code generator
    fn create_ollama_generator(
        endpoint: String,
        model: String,
    ) -> Result<Arc<dyn WorkflowPlannerEngine>> {
        let timeout = 30;
        let decision_engine = DecisionEngine::new(&endpoint, &model, timeout)?;
        eprintln!("🤖 Ollama code generator initialized: {}", endpoint);
        Ok(Arc::new(decision_engine))
    }

    /// Create AI CLI-based code generator (default: claude)
    fn create_ai_cli_generator() -> Result<Arc<dyn WorkflowPlannerEngine>> {
        // Default to claude if CLI_TYPE not set
        let cli_type_str = std::env::var("CLI_TYPE").unwrap_or_else(|_| "claude".to_string());

        let cli_type = match cli_type_str.to_lowercase().as_str() {
            "claude" => CliType::Claude,
            "codex" => CliType::Codex,
            "gemini" => CliType::Gemini,
            _ => {
                return Err(anyhow!(
                    "Unsupported CLI_TYPE '{}'. Supported: claude, codex, gemini",
                    cli_type_str
                ))
            }
        };

        // Provider can be any string (llmlite, openrouter, anthropic, etc.)
        let provider = std::env::var("CLI_PROVIDER").ok();

        eprintln!(
            "🤖 AI CLI code generator initialized: {} (provider: {:?})",
            cli_type.display_name(),
            provider
        );

        Ok(Arc::new(AiCliCodeGenerator::new(cli_type, provider)))
    }
}

/// AI CLI-based code generator
/// Supports claude, codex, gemini CLI tools with any provider
pub struct AiCliCodeGenerator {
    cli_type: CliType,
    provider: Option<String>,
    timeout: Duration,
}

impl AiCliCodeGenerator {
    /// Create new AI CLI code generator with 12-hour timeout
    /// Provider can be any string: llmlite, openrouter, anthropic, etc.
    pub fn new(cli_type: CliType, provider: Option<String>) -> Self {
        Self {
            cli_type,
            provider,
            timeout: Duration::from_secs(12 * 60 * 60), // 12 hours
        }
    }

    /// Call AI CLI with prompt and get response
    /// Uses the unified AI CLI execution infrastructure
    async fn call_ai_cli(&self, prompt: &str) -> Result<String> {
        let registry = create_cli_registry()
            .context("Failed to create CLI registry")?;

        // Build args with full access permissions
        let cli_args = self.cli_type.build_full_access_args(prompt);

        // Convert to OsString for supervisor
        let os_args: Vec<std::ffi::OsString> = cli_args
            .into_iter()
            .map(|s| s.into())
            .collect();

        // Execute CLI and capture output
        supervisor::execute_cli_with_output(
            &registry,
            &self.cli_type,
            &os_args,
            self.provider.clone(),
            self.timeout,
        )
        .await
        .context(format!("Failed to execute {} CLI", self.cli_type.display_name()))
    }
}

#[async_trait]
impl WorkflowPlannerEngine for AiCliCodeGenerator {
    async fn plan_workflow(
        &self,
        user_request: &str,
        available_tools: &[CandidateToolInfo],
    ) -> Result<WorkflowPlan> {
        if user_request.trim().is_empty() {
            return Err(anyhow!("user_request cannot be empty"));
        }
        if available_tools.is_empty() {
            return Err(anyhow!("No MCP tools available for workflow planning"));
        }

        let prompt = build_planning_prompt(user_request, available_tools);
        let response = self.call_ai_cli(&prompt).await?;

        // Extract JSON from response
        let json_str = extract_json_from_response(&response)
            .ok_or_else(|| anyhow!("AI CLI response does not contain valid JSON"))?;

        let mut plan: WorkflowPlan = serde_json::from_str(&json_str)
            .context("Failed to parse workflow plan JSON from AI CLI")?;

        // Normalize plan
        finalize_workflow_plan(&mut plan, user_request);

        Ok(plan)
    }

    async fn generate_js_code(&self, plan: &WorkflowPlan) -> Result<String> {
        if !plan.is_feasible {
            return Err(anyhow!(
                "Cannot generate code for infeasible workflow: {}",
                plan.reason
            ));
        }
        if plan.steps.is_empty() {
            return Err(anyhow!("Workflow plan must contain at least one step"));
        }

        let prompt = build_codegen_prompt(plan);
        let response = self.call_ai_cli(&prompt).await?;

        // Extract code from response
        let code = strip_code_fences(&response);

        if code.trim().is_empty() {
            return Err(anyhow!("AI CLI returned empty JavaScript code"));
        }

        // Basic validation
        if !code.contains("async function workflow") {
            return Err(anyhow!(
                "Generated JavaScript missing `async function workflow` signature"
            ));
        }

        Ok(code)
    }
}

/// Build planning prompt
fn build_planning_prompt(user_request: &str, tools: &[CandidateToolInfo]) -> String {
    let tools_list = tools
        .iter()
        .map(|tool| {
            let mut block = format!(
                "- **{}::{}**\n  Description: {}\n",
                tool.server, tool.tool, tool.description
            );
            if let Some(schema) = &tool.schema_snippet {
                block.push_str(&format!("  Schema: {}\n", schema));
            }
            block
        })
        .collect::<String>();

    format!(
        r#"You are Agentic-Warden's workflow planner. Analyze if the user request can be accomplished using available MCP tools.

## User Request:
{}

## Available MCP Tools:
{}

## Your Task:
1. Determine if this request is feasible with the available tools
2. If YES: Plan the step-by-step workflow
3. If NO: Explain why it's not feasible

## Response Format (JSON only):
```json
{{
  "is_feasible": true/false,
  "reason": "explanation if not feasible",
  "suggested_name": "workflow_name_in_snake_case",
  "description": "Brief workflow description",
  "steps": [
    {{
      "step": 1,
      "tool": "server::tool_name",
      "description": "What this step does",
      "dependencies": []
    }}
  ],
  "input_params": [
    {{
      "name": "param_name",
      "type": "string|number|boolean",
      "description": "Parameter description",
      "required": true/false
    }}
  ]
}}
```

Respond with ONLY the JSON, no additional text."#,
        user_request, tools_list
    )
}

/// Build code generation prompt
fn build_codegen_prompt(plan: &WorkflowPlan) -> String {
    let steps_json = serde_json::to_string_pretty(&plan.steps).unwrap_or_else(|_| "[]".into());

    format!(
        r#"You are Agentic-Warden's JavaScript code generator. Generate a workflow function based on the plan.

## Workflow Plan:
Name: {}
Description: {}
Steps:
{}

## MCP Call Contract:
- Use the injected global `mcp` object
- Invoke tools via: await mcp.call("<server>", "<tool_name>", {{ /* args */ }});
- Always pass an object payload (use {{}} when no arguments are needed)
- Wrap calls in try/catch and surface clear, actionable error messages

## Code Generation Requirements:
1. Create an async function named 'workflow' that takes an 'input' parameter
2. Access input parameters via input.paramName (e.g., input.repo_url)
3. Call MCP tools using mcp.call(server, tool, args)
4. Include try-catch error handling for each MCP call
5. Return a structured result object
6. Add comments explaining each step

## Example:
```javascript
async function workflow(input) {{
  try {{
    // Step 1: Call first MCP tool
    const result1 = await mcp.call("server1", "tool1", {{ arg: input.param }});

    // Step 2: Use result in next call
    const result2 = await mcp.call("server2", "tool2", {{ data: result1 }});

    return {{ success: true, data: result2 }};
  }} catch (error) {{
    return {{ success: false, error: error.message }};
  }}
}}
```

Respond with ONLY the JavaScript code, no markdown fences, no explanation."#,
        plan.suggested_name, plan.description, steps_json
    )
}

/// Extract JSON from response (handle markdown code blocks)
fn extract_json_from_response(response: &str) -> Option<String> {
    // Try to find JSON in code blocks first
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start..].find("```") {
            let json_start = start + 7; // "```json".len()
            let json_end = start + end;
            if json_start < json_end {
                return Some(response[json_start..json_end].trim().to_string());
            }
        }
    }

    // Try to find raw JSON
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if start < end {
                return Some(response[start..=end].to_string());
            }
        }
    }

    None
}

/// Strip code fences from generated code
fn strip_code_fences(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();

    // Remove starting fence
    if let Some(first) = lines.first() {
        if first.trim().starts_with("```") {
            lines.remove(0);
        }
    }

    // Remove ending fence
    if let Some(last) = lines.last() {
        if last.trim().starts_with("```") && last.trim().chars().all(|c| c == '`') {
            lines.pop();
        }
    }

    lines.join("\n").trim().to_string()
}

/// Finalize workflow plan (normalize step numbers and dependencies)
fn finalize_workflow_plan(plan: &mut WorkflowPlan, user_request: &str) {
    // Normalize step numbers
    for (idx, step) in plan.steps.iter_mut().enumerate() {
        step.step = idx + 1;
    }

    // Validate dependencies
    for step in &mut plan.steps {
        step.dependencies.retain(|dep| *dep < step.step);
    }

    // Set suggested_name if empty
    if plan.suggested_name.trim().is_empty() {
        plan.suggested_name = derive_workflow_name(user_request);
    }
}

/// Derive workflow name from user request
fn derive_workflow_name(user_request: &str) -> String {
    let base = user_request
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join("_");

    if base.is_empty() {
        "workflow_plan".to_string()
    } else {
        format!("{}_workflow", base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_markdown() {
        let response = r#"Here is the plan:
```json
{"is_feasible": true, "reason": ""}
```
That's it!"#;

        let json = extract_json_from_response(response).unwrap();
        assert!(json.contains("is_feasible"));
    }

    #[test]
    fn test_extract_json_raw() {
        let response = r#"{"is_feasible": false}"#;
        let json = extract_json_from_response(response).unwrap();
        assert_eq!(json, r#"{"is_feasible": false}"#);
    }

    #[test]
    fn test_strip_code_fences() {
        let code = r#"```javascript
async function workflow() {}
```"#;
        let stripped = strip_code_fences(code);
        assert_eq!(stripped, "async function workflow() {}");
    }

    #[test]
    fn test_derive_workflow_name() {
        assert_eq!(
            derive_workflow_name("Create a git report with status"),
            "create_a_git_report_with_workflow"
        );
        assert_eq!(derive_workflow_name(""), "workflow_plan");
    }
}
