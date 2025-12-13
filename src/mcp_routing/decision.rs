use crate::mcp_routing::js_orchestrator::workflow_planner::{WorkflowPlan, WorkflowPlannerEngine};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage, ChatMessageResponse},
    Ollama,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct CandidateToolInfo {
    pub server: String,
    pub tool: String,
    pub description: String,
    pub schema_snippet: Option<String>,
}

pub struct DecisionInput {
    pub user_request: String,
    pub candidates: Vec<CandidateToolInfo>,
}

#[derive(Debug, Clone)]
pub struct DecisionOutcome {
    pub server: String,
    pub tool: String,
    pub arguments: Value,
    pub rationale: String,
    pub confidence: f32,
}

/// Abstracts LLM chat completion clients so they can be mocked in tests.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a chat completion request to the LLM.
    async fn chat(&self, request: ChatMessageRequest) -> Result<ChatMessageResponse>;
}

struct OllamaChatClient {
    inner: Ollama,
}

impl OllamaChatClient {
    fn new(inner: Ollama) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl LlmClient for OllamaChatClient {
    async fn chat(&self, request: ChatMessageRequest) -> Result<ChatMessageResponse> {
        self.inner
            .send_chat_messages(request)
            .await
            .map_err(|err| anyhow!(err))
    }
}

pub struct DecisionEngine {
    client: Arc<dyn LlmClient>,
    model: String,
    timeout: Duration,
}

impl DecisionEngine {
    pub fn new(endpoint: &str, model: &str, timeout_secs: u64) -> Result<Self> {
        let client = Ollama::try_new(endpoint)?;
        let llm_client: Arc<dyn LlmClient> = Arc::new(OllamaChatClient::new(client));
        Ok(Self::with_client(llm_client, model, timeout_secs))
    }

    /// Construct decision engine with a custom LLM client (used for testing/mocking).
    pub fn with_client(client: Arc<dyn LlmClient>, model: &str, timeout_secs: u64) -> Self {
        Self {
            client,
            model: model.to_string(),
            timeout: Duration::from_secs(timeout_secs.max(5)),
        }
    }

    pub async fn decide(&self, input: DecisionInput) -> Result<DecisionOutcome> {
        if input.candidates.is_empty() {
            return Err(anyhow!("No candidates available for decision engine"));
        }
        let system_prompt = "You are Agentic-Warden's internal router. \
            Choose the best MCP tool for the user request. \
            Respond ONLY with valid JSON in the following shape: \n\
            {\"server\": \"server-name\", \"tool\": \"tool-name\", \"arguments\": {...}, \"rationale\": \"why\", \"confidence\": 0.0-1.0}";

        let user_prompt = build_user_prompt(&input);
        let request = ChatMessageRequest::new(
            self.model.clone(),
            vec![
                ChatMessage::system(system_prompt.to_string()),
                ChatMessage::user(user_prompt),
            ],
        );

        let response = timeout(self.timeout, self.client.chat(request))
            .await
            .map_err(|_| anyhow!("LLM decision timed out"))??;

        parse_decision(&response.message.content, &input.candidates).or_else(|_| {
            // Fallback to first candidate with empty arguments.
            let fallback = &input.candidates[0];
            Ok(DecisionOutcome {
                server: fallback.server.clone(),
                tool: fallback.tool.clone(),
                arguments: Value::Object(Default::default()),
                rationale: "Fallback to top-ranked candidate due to parsing error".into(),
                confidence: 0.25,
            })
        })
    }

    /// Plan an MCP workflow using the LLM and return a normalized plan structure.
    pub async fn plan_workflow(
        &self,
        user_request: &str,
        available_tools: &[CandidateToolInfo],
    ) -> Result<WorkflowPlan> {
        if user_request.trim().is_empty() {
            return Err(anyhow!("user_request cannot be empty"));
        }
        if available_tools.is_empty() {
            return Err(anyhow!(
                "No MCP tools are available to build a workflow plan"
            ));
        }

        let system_prompt = "You are Agentic-Warden's workflow planner. \
            Always respond with JSON that matches the provided schema.";
        let user_prompt = build_planning_prompt(user_request, available_tools);
        let request = ChatMessageRequest::new(
            self.model.clone(),
            vec![
                ChatMessage::system(system_prompt.to_string()),
                ChatMessage::user(user_prompt),
            ],
        );

        let response = timeout(self.timeout, self.client.chat(request))
            .await
            .map_err(|_| anyhow!("LLM workflow planner timed out"))??;

        let mut plan = parse_workflow_plan_response(&response.message.content)
            .context("LLM returned invalid workflow plan JSON")?;
        finalize_workflow_plan(&mut plan, user_request);
        Ok(plan)
    }

    /// Generate JavaScript workflow code for a feasible plan via the LLM.
    pub async fn generate_js_code(&self, plan: &WorkflowPlan) -> Result<String> {
        if !plan.is_feasible {
            return Err(anyhow!(
                "Cannot generate code for infeasible workflow: {}",
                plan.reason
            ));
        }
        if plan.steps.is_empty() {
            return Err(anyhow!(
                "Workflow plan must contain at least one step before code generation"
            ));
        }

        let system_prompt = "You are Agentic-Warden's JavaScript code generator. \
            Produce ONLY JavaScript that satisfies the requirements.";
        let user_prompt = build_codegen_prompt(plan);
        let request = ChatMessageRequest::new(
            self.model.clone(),
            vec![
                ChatMessage::system(system_prompt.to_string()),
                ChatMessage::user(user_prompt),
            ],
        );

        let response = timeout(self.timeout, self.client.chat(request))
            .await
            .map_err(|_| anyhow!("LLM code generator timed out"))??;

        let code = strip_code_fences(&response.message.content);
        if code.trim().is_empty() {
            return Err(anyhow!(
                "LLM returned empty JavaScript code for workflow {}",
                plan.suggested_name
            ));
        }

        if !code.contains("async function workflow") {
            return Err(anyhow!(
                "Generated JavaScript missing `async function workflow` signature"
            ));
        }
        if !(code.contains("try") && code.contains("catch")) {
            return Err(anyhow!(
                "Generated JavaScript must include try/catch error handling"
            ));
        }

        Ok(code)
    }

    /// Generic LLM chat helper used by schema correction and other ad-hoc prompts.
    pub(crate) async fn chat_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let request = ChatMessageRequest::new(
            self.model.clone(),
            vec![
                ChatMessage::system(system_prompt.to_string()),
                ChatMessage::user(user_prompt.to_string()),
            ],
        );

        let response = timeout(self.timeout, self.client.chat(request))
            .await
            .map_err(|_| anyhow!("LLM chat completion timed out"))??;

        let content = strip_code_fences(&response.message.content);
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("LLM returned empty response for chat completion"));
        }

        Ok(trimmed.to_string())
    }
}

fn build_user_prompt(input: &DecisionInput) -> String {
    let mut prompt = String::new();
    prompt.push_str("User request:\n");
    prompt.push_str(&input.user_request);
    prompt.push_str("\n\nCandidate tools:\n");

    for (idx, candidate) in input.candidates.iter().enumerate() {
        prompt.push_str(&format!(
            "{idx}. {server}::{tool}\nDescription: {desc}\n",
            idx = idx + 1,
            server = candidate.server,
            tool = candidate.tool,
            desc = candidate.description
        ));
        if let Some(schema) = &candidate.schema_snippet {
            prompt.push_str("Schema preview: ");
            prompt.push_str(schema);
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt.push_str(
        "\nReturn JSON with the best server/tool, a parsable arguments object, \
        reasoning, and confidence between 0 and 1.",
    );
    prompt
}

fn parse_decision(content: &str, candidates: &[CandidateToolInfo]) -> Result<DecisionOutcome> {
    #[derive(Deserialize)]
    struct Decision {
        server: Option<String>,
        tool: String,
        #[serde(default)]
        arguments: Value,
        rationale: Option<String>,
        confidence: Option<f32>,
    }

    let mut trimmed = content.trim();
    if trimmed.starts_with("```") {
        trimmed = trimmed.trim_start_matches("`");
        trimmed = trimmed.trim_start_matches("json");
        trimmed = trimmed.trim();
        trimmed = trimmed.trim_matches('`');
    }
    let decision: Decision = serde_json::from_str(trimmed)?;
    let tool = decision.tool;
    let server = decision
        .server
        .or_else(|| {
            candidates
                .iter()
                .find(|c| c.tool == tool)
                .map(|c| c.server.clone())
        })
        .ok_or_else(|| anyhow!("Decision response missing server field"))?;

    Ok(DecisionOutcome {
        server,
        tool,
        arguments: normalize_arguments(decision.arguments),
        rationale: decision
            .rationale
            .unwrap_or_else(|| "No rationale provided".into()),
        confidence: decision.confidence.unwrap_or(0.5).clamp(0.0, 1.0),
    })
}

fn normalize_arguments(value: Value) -> Value {
    match value {
        Value::Null => Value::Object(Default::default()),
        Value::Object(_) => value,
        Value::String(text) => {
            serde_json::from_str(&text).unwrap_or_else(|_| json!({ "value": text }))
        }
        other => json!({ "value": other }),
    }
}

fn parse_workflow_plan_response(content: &str) -> Result<WorkflowPlan> {
    let cleaned = strip_code_fences(content);
    serde_json::from_str::<WorkflowPlan>(&cleaned).or_else(|primary_err| {
        if let Some(snippet) = extract_outermost_json(&cleaned) {
            serde_json::from_str::<WorkflowPlan>(&snippet).map_err(|err| {
                anyhow!(
                    "Failed to parse workflow plan JSON (primary: {primary_err}, fallback: {err})"
                )
            })
        } else {
            Err(anyhow!(
                "Failed to locate JSON workflow plan in LLM response: {primary_err}"
            ))
        }
    })
}

fn finalize_workflow_plan(plan: &mut WorkflowPlan, user_request: &str) {
    normalize_workflow_plan(plan);

    let suggested = if plan.suggested_name.trim().is_empty() {
        derive_workflow_name(user_request)
    } else {
        let candidate = enforce_snake_case(&plan.suggested_name);
        if candidate.is_empty() {
            derive_workflow_name(user_request)
        } else {
            candidate
        }
    };
    plan.suggested_name = suggested;

    if plan.description.trim().is_empty() {
        plan.description = format!("Workflow for {}", user_request.trim());
    } else {
        plan.description = plan.description.trim().to_string();
    }

    if !plan.is_feasible && plan.reason.trim().is_empty() {
        plan.reason = "LLM marked the request as infeasible".into();
    } else {
        plan.reason = plan.reason.trim().to_string();
    }
}

fn normalize_workflow_plan(plan: &mut WorkflowPlan) {
    plan.steps.retain(|step| !step.tool.trim().is_empty());
    let mut next_idx = 1usize;
    let mut seen_steps = HashSet::new();

    for step in plan.steps.iter_mut() {
        step.tool = step.tool.trim().to_string();
        if step.tool.is_empty() {
            continue;
        }
        if step.step == 0 || seen_steps.contains(&step.step) {
            while seen_steps.contains(&next_idx) {
                next_idx += 1;
            }
            step.step = next_idx;
            next_idx += 1;
        }
        seen_steps.insert(step.step);
        if step.description.trim().is_empty() {
            step.description = format!("Call {}", step.tool);
        } else {
            step.description = step.description.trim().to_string();
        }
    }

    plan.steps.sort_by_key(|s| s.step);
    let valid_steps: HashSet<usize> = plan.steps.iter().map(|s| s.step).collect();
    for step in plan.steps.iter_mut() {
        step.dependencies
            .retain(|dep| valid_steps.contains(dep) && *dep != step.step);
        step.dependencies.sort_unstable();
        step.dependencies.dedup();
    }

    for param in plan.input_params.iter_mut() {
        param.name = param.name.trim().to_string();
        param.param_type = sanitize_param_type(&param.param_type);
        if param.description.trim().is_empty() {
            param.description = format!("Input for {}", param.name);
        } else {
            param.description = param.description.trim().to_string();
        }
    }

    let mut seen_params = HashSet::new();
    plan.input_params.retain(|param| {
        if param.name.is_empty() {
            return false;
        }
        let key = param.name.to_lowercase();
        if seen_params.contains(&key) {
            return false;
        }
        seen_params.insert(key);
        true
    });
}

fn sanitize_param_type(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "number" => "number".into(),
        "boolean" => "boolean".into(),
        "object" => "object".into(),
        _ => "string".into(),
    }
}

fn strip_code_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    let mut lines = trimmed.lines();
    lines.next(); // discard opening fence
    let mut body: Vec<&str> = lines.collect();
    if let Some(last) = body.last() {
        if last.trim().starts_with("```") && last.trim().chars().all(|c| c == '`') {
            body.pop();
        }
    }
    let joined = body.join("\n");
    joined.trim().trim_end_matches('`').trim_end().to_string()
}

fn extract_outermost_json(text: &str) -> Option<String> {
    let mut depth = 0usize;
    let mut start_idx = None;
    for (idx, ch) in text.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start_idx = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start_idx {
                        return Some(text[start..=idx].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn derive_workflow_name(user_request: &str) -> String {
    let base = enforce_snake_case(user_request);
    let mut name = if base.is_empty() {
        "workflow_plan".to_string()
    } else {
        base
    };

    if name.len() > 48 {
        name.truncate(48);
        name = name.trim_end_matches('_').to_string();
    }

    if !name.ends_with("_workflow") {
        if name.is_empty() {
            name = "workflow_plan".into();
        }
        if name.len() > 40 {
            name.truncate(40);
            name = name.trim_end_matches('_').to_string();
        }
        if !name.is_empty() {
            name.push_str("_workflow");
        }
    }

    if name.is_empty() {
        "workflow_plan".into()
    } else {
        name
    }
}

fn enforce_snake_case(value: &str) -> String {
    let mut result = String::new();
    let mut last_was_underscore = true;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            last_was_underscore = false;
        } else if !last_was_underscore {
            result.push('_');
            last_was_underscore = true;
        }
    }
    result.trim_matches('_').to_string()
}

fn build_planning_prompt(user_request: &str, tools: &[CandidateToolInfo]) -> String {
    format!(
        r#"
You are Agentic-Warden's workflow planner. Analyze if the user request can be accomplished using available MCP tools.

## User Request:
{}

## Available MCP Tools:
{}

## Your Task:
1. Determine if this request is feasible with the available tools
2. If YES: Plan the step-by-step workflow
3. If NO: Explain why it's not feasible

## Output Format (JSON only):
{{
  "is_feasible": true/false,
  "reason": "explanation if not feasible",
  "suggested_name": "snake_case_workflow_name",
  "description": "Brief description of what this workflow does",
  "steps": [
    {{"step": 1, "tool": "server::tool_name", "description": "what this step does", "dependencies": []}},
    {{"step": 2, "tool": "server::tool_name", "description": "what this step does", "dependencies": [1]}}
  ],
  "input_params": [
    {{"name": "param_name", "type": "string", "description": "what it's for", "required": true}}
  ]
}}
    "#,
        user_request,
        format_tools(tools)
    )
}

fn format_tools(tools: &[CandidateToolInfo]) -> String {
    if tools.is_empty() {
        return "No tools registered.".into();
    }

    tools
        .iter()
        .enumerate()
        .map(|(idx, tool)| {
            let mut block = format!(
                "{}. {}::{}\n   Description: {}\n",
                idx + 1,
                tool.server,
                tool.tool,
                tool.description
            );
            if let Some(schema) = &tool.schema_snippet {
                block.push_str("   Schema: ");
                block.push_str(schema);
                block.push('\n');
            }
            block
        })
        .collect::<String>()
}

fn build_codegen_prompt(plan: &WorkflowPlan) -> String {
    let steps_json = serde_json::to_string_pretty(&plan.steps).unwrap_or_else(|_| "[]".into());
    format!(
        r#"
You are Agentic-Warden's JavaScript code generator. Generate a workflow function based on the plan.

## Workflow Plan:
Name: {}
Description: {}
Steps:
{}

## MCP Call Contract:
- Use the injected global `mcp` object.
- Invoke tools via: await mcp.call("<server>", "<tool_name>", {{ /* args */ }});
- Always pass an object payload (use {{}} when no arguments are needed).
- Wrap calls in try/catch and surface clear, actionable error messages.

## Code Generation Requirements:
1. Create an async function named 'workflow' that takes an 'input' parameter
2. Access input parameters via input.paramName (e.g., input.repo_url)
3. Call MCP tools using mcp.call(server, tool, args)
4. Include try-catch error handling
5. Return a structured result object
6. Add comments explaining each step

## Output:
Provide ONLY the JavaScript code, no markdown, no explanation.
    "#,
        plan.suggested_name, plan.description, steps_json,
    )
}

#[async_trait]
impl WorkflowPlannerEngine for DecisionEngine {
    async fn plan_workflow(
        &self,
        user_request: &str,
        available_tools: &[CandidateToolInfo],
    ) -> Result<WorkflowPlan> {
        DecisionEngine::plan_workflow(self, user_request, available_tools).await
    }

    async fn generate_js_code(&self, plan: &WorkflowPlan) -> Result<String> {
        DecisionEngine::generate_js_code(self, plan).await
    }
}
