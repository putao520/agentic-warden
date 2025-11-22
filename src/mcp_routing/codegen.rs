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
        let timeout = 30 * 60;  // 30 minutes in seconds
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
    /// Uses temporary files to avoid stdin/stdout capture issues
    async fn call_ai_cli(&self, prompt: &str) -> Result<String> {
        eprintln!("   🔍 [CODEX] Starting AI CLI call...");
        eprintln!("   🔍 [CODEX] CLI type: {}", self.cli_type.display_name());
        eprintln!("   🔍 [CODEX] Timeout: {:?}", self.timeout);
        eprintln!("   🔍 [CODEX] Prompt length: {} chars", prompt.len());

        let registry = create_cli_registry()
            .context("Failed to create CLI registry")?;

        eprintln!("   🔍 [CODEX] CLI registry created successfully");

        // Create temporary files for input/output
        let prompt_file = std::env::temp_dir().join(format!("aiw_prompt_{}.txt", std::process::id()));
        let output_file = std::env::temp_dir().join(format!("aiw_output_{}.txt", std::process::id()));

        // Write prompt to temp file
        std::fs::write(&prompt_file, prompt)
            .context("Failed to write prompt to temp file")?;

        // Build args using file input for CODEX
        let cli_args = match self.cli_type.display_name() {
            "codex" => {
                vec![
                    "exec".to_string(),
                    "--dangerously-bypass-approvals-and-sandbox".to_string(),
                ]
            }
            _ => self.cli_type.build_full_access_args(prompt),
        };

        eprintln!("   🔍 [CODEX] CLI args built: {} args", cli_args.len());

        // Convert to OsString for supervisor
        let os_args: Vec<std::ffi::OsString> = cli_args
            .into_iter()
            .map(|s| s.into())
            .collect();

        eprintln!("   🔍 [CODEX] Calling supervisor::execute_cli...");

        // Execute CLI normally (no output capture)
        let exit_code = supervisor::execute_cli(
            &registry,
            &self.cli_type,
            &os_args,
            self.provider.clone(),
        )
        .await;

        eprintln!("   🔍 [CODEX] Supervisor call completed with exit code: {:?}", exit_code);

        // Clean up prompt file
        let _ = std::fs::remove_file(&prompt_file);

        match exit_code {
            Ok(0) => {
                // Give CODEX a moment to write to log files
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Parse log files to get actual CODEX output
                let actual_output = parse_codex_log_output().await?;
                eprintln!("   🔍 [CODEX] Retrieved actual output, length: {}", actual_output.len());
                Ok(actual_output)
            }
            Ok(code) => {
                Err(anyhow!("CLI execution failed with exit code: {}", code))
            }
            Err(e) => {
                Err(anyhow!("CLI execution failed with error: {}", e))
            }
        }
    }
}

/// Parse CODEX log output to get actual AI response
async fn parse_codex_log_output() -> Result<String> {
    // Find the most recent CODEX log file
    let log_dir = std::env::temp_dir().join(".aiw").join("logs");

    let logs = std::fs::read_dir(&log_dir)
        .context("Failed to read log directory")?;

    let mut latest_log: Option<(std::path::PathBuf, std::time::SystemTime)> = None;

    for entry in logs {
        let entry = entry.context("Failed to read log entry")?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("log") {
            let metadata = std::fs::metadata(&path)
                .context("Failed to get log metadata")?;
            let modified = metadata.modified()
                .context("Failed to get modification time")?;

            match &latest_log {
                None => latest_log = Some((path, modified)),
                Some((_, latest_time)) => {
                    if modified > *latest_time {
                        latest_log = Some((path, modified));
                    }
                }
            }
        }
    }

    let (log_path, _) = latest_log.ok_or_else(|| anyhow!("No CODEX log files found"))?;

    eprintln!("   🔍 [CODEX] Reading log file: {:?}", log_path);

    // Read and parse the log file
    let log_content = std::fs::read_to_string(&log_path)
        .context("Failed to read log file")?;

    // Extract CODEX output from log (look for patterns that indicate AI responses)
    let mut output_lines = Vec::new();
    let mut in_codex_output = false;

    for line in log_content.lines() {
        // Look for CODEX response patterns
        if line.contains("thinking") || line.contains("codex") || line.contains("agent_message") {
            in_codex_output = true;
        }

        if in_codex_output && !line.trim().is_empty() {
            // Skip log timestamps and metadata, keep actual content
            if !line.starts_with("[") && !line.starts_with("202") && !line.contains("mcp:") {
                output_lines.push(line);
            }
        }
    }

    let output = if output_lines.is_empty() {
        // If we can't parse structured output, return the last substantial part of the log
        let lines: Vec<&str> = log_content.lines().collect();
        if lines.len() > 10 {
            lines[lines.len()-20..].join("\n")
        } else {
            log_content
        }
    } else {
        output_lines.join("\n")
    };

    // Clean up the output - remove common CODEX CLI artifacts
    let cleaned_output = output
        .replace("thinking", "")
        .replace("codex", "")
        .replace("user", "")
        .replace("assistant", "")
        .trim()
        .to_string();

    if cleaned_output.is_empty() {
        return Err(anyhow!("No usable output found in CODEX logs"));
    }

    Ok(cleaned_output)
}

/// Generate mock response for testing purposes (deprecated)
fn generate_mock_response(prompt: &str) -> String {
    if prompt.contains("workflow") || prompt.contains("plan") {
        r#"{
  "is_feasible": true,
  "reason": "",
  "suggested_name": "example_workflow",
  "description": "Example workflow generated from prompt",
  "steps": [
    {
      "step": 1,
      "tool": "server::tool1",
      "description": "Example step 1",
      "dependencies": []
    }
  ],
  "input_params": []
}"#.to_string()
    } else if prompt.contains("JavaScript") || prompt.contains("function") {
        r#"async function workflow(input) {
  try {
    // Example JavaScript workflow
    const result = await mcp.call("server", "tool", {});
    return { success: true, data: result };
  } catch (error) {
    return { success: false, error: error.message };
  }
}"#.to_string()
    } else {
        "Mock response for testing purposes".to_string()
    }
}

/// Extract the final agent message from CODEX JSONL output
fn extract_final_message_from_jsonl(jsonl_output: &str) -> String {
    eprintln!("   🔍 [CODEX] Parsing JSONL output with {} lines", jsonl_output.lines().count());

    let mut final_message = String::new();

    for line in jsonl_output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Parse each JSONL line
        if let Ok(json_obj) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(obj) = json_obj.as_object() {
                // Look for "item" objects with type "agent_message"
                if let Some(item) = obj.get("item").and_then(|v| v.as_object()) {
                    if let Some(item_type) = item.get("type").and_then(|v| v.as_str()) {
                        if item_type == "agent_message" {
                            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                final_message = text.to_string();
                                eprintln!("   🔍 [CODEX] Found final agent message");
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if final_message.is_empty() {
        eprintln!("   🔍 [CODEX] No final message found, returning raw output");
        jsonl_output.to_string()
    } else {
        final_message
    }
}

#[async_trait]
impl WorkflowPlannerEngine for AiCliCodeGenerator {
    async fn plan_workflow(
        &self,
        user_request: &str,
        available_tools: &[CandidateToolInfo],
    ) -> Result<WorkflowPlan> {
        eprintln!("   🔍 [PLANNER] Starting plan_workflow...");

        if user_request.trim().is_empty() {
            return Err(anyhow!("user_request cannot be empty"));
        }
        if available_tools.is_empty() {
            return Err(anyhow!("No MCP tools available for workflow planning"));
        }

        eprintln!("   🔍 [PLANNER] Input validated, {} tools available", available_tools.len());

        let prompt = build_planning_prompt(user_request, available_tools);
        eprintln!("   🔍 [PLANNER] Planning prompt built, length: {}", prompt.len());

        eprintln!("   🔍 [PLANNER] Calling AI CLI for workflow planning...");
        let response = self.call_ai_cli(&prompt).await?;
        eprintln!("   🔍 [PLANNER] AI CLI response received, length: {}", response.len());

        // Extract JSON from response
        let json_str = extract_json_from_response(&response)
            .ok_or_else(|| anyhow!("AI CLI response does not contain valid JSON"))?;

        eprintln!("   🔍 [PLANNER] JSON extracted, length: {}", json_str.len());

        let mut plan: WorkflowPlan = serde_json::from_str(&json_str)
            .context("Failed to parse workflow plan JSON from AI CLI")?;

        eprintln!("   🔍 [PLANNER] Workflow plan parsed, feasible: {}, steps: {}", plan.is_feasible, plan.steps.len());

        // Normalize plan
        finalize_workflow_plan(&mut plan, user_request);

        eprintln!("   🔍 [PLANNER] Workflow plan finalized successfully");
        Ok(plan)
    }

    async fn generate_js_code(&self, plan: &WorkflowPlan) -> Result<String> {
        eprintln!("   🔍 [CODEGEN] Starting JavaScript code generation...");

        if !plan.is_feasible {
            return Err(anyhow!(
                "Cannot generate code for infeasible workflow: {}",
                plan.reason
            ));
        }
        if plan.steps.is_empty() {
            return Err(anyhow!("Workflow plan must contain at least one step"));
        }

        eprintln!("   🔍 [CODEGEN] Plan validation passed, generating code for {} steps", plan.steps.len());

        let prompt = build_codegen_prompt(plan);
        eprintln!("   🔍 [CODEGEN] Code generation prompt built, length: {}", prompt.len());

        eprintln!("   🔍 [CODEGEN] Calling AI CLI for JavaScript generation...");
        let response = self.call_ai_cli(&prompt).await?;
        eprintln!("   🔍 [CODEGEN] AI CLI response received, length: {}", response.len());

        // Extract code from response
        let code = strip_code_fences(&response);

        if code.trim().is_empty() {
            return Err(anyhow!("AI CLI returned empty JavaScript code"));
        }

        eprintln!("   🔍 [CODEGEN] JavaScript code extracted, length: {}", code.len());

        // Strict validation - 100% compliance required
        let required_elements = vec![
            ("async function workflow(input)", "Missing exact function signature"),
            ("return { success: true", "Missing success return format"),
            ("return { success: false", "Missing error return format"),
            ("await mcp.call(", "Missing MCP calls"),
            ("try {", "Missing try block"),
            ("catch (", "Missing catch block"),
        ];

        for (element, error_msg) in required_elements {
            if !code.contains(element) {
                return Err(anyhow!(
                    "Generated JavaScript validation failed: {} - missing '{}'",
                    error_msg, element
                ));
            }
        }

        // Ensure no markdown fences
        if code.contains("```") {
            return Err(anyhow!("Generated JavaScript contains markdown fences"));
        }

        eprintln!("   🔍 [CODEGEN] JavaScript code validation passed");
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
        r#"You are Agentic-Warden's JavaScript code generator. Generate a workflow function based EXACTLY on the provided plan.

## Workflow Plan:
Name: {}
Description: {}
Steps:
{}

## CRITICAL REQUIREMENTS - MUST FOLLOW EXACTLY:

1. **Function Signature**: MUST be exactly: `async function workflow(input)`

2. **Step Implementation**: You MUST implement EACH step from the plan EXACTLY as specified:
   - Use the EXACT server and tool names from the plan
   - Follow the EXACT step order from the plan
   - Use the step descriptions as comments

3. **MCP Call Format**: ALWAYS use: `await mcp.call("server_name", "tool_name", {{ arguments }})`

4. **Error Handling**: Wrap EVERY mcp.call in try-catch with structured error response

5. **Return Format**: MUST return `{{ success: true, data: result }}` or `{{ success: false, error: error.message }}`

## Strict Implementation Template:
```javascript
async function workflow(input) {{
  try {{
    // Step 1: [Step 1 description from plan]
    const step1Result = await mcp.call("server_from_step1", "tool_from_step1", {{ /* args based on input */ }});

    // Step 2: [Step 2 description from plan]
    const step2Result = await mcp.call("server_from_step2", "tool_from_step2", {{ /* use step1Result if needed */ }});

    // Continue with all steps...

    return {{ success: true, data: finalResult }};
  }} catch (error) {{
    return {{ success: false, error: error.message }};
  }}
}}
```

## IMPORTANT:
- DO NOT deviate from the plan steps
- DO NOT add extra functionality not in the plan
- DO NOT change the function signature
- MUST use the exact server and tool names from the plan
- MUST implement ALL steps in the plan

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
