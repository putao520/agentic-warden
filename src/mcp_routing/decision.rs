use crate::memory::ConversationRecord;
use anyhow::{anyhow, Result};
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage},
    Ollama,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
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
    pub conversation: Vec<ConversationRecord>,
}

#[derive(Debug, Clone)]
pub struct DecisionOutcome {
    pub server: String,
    pub tool: String,
    pub arguments: Value,
    pub rationale: String,
    pub confidence: f32,
}

pub struct DecisionEngine {
    client: Ollama,
    model: String,
    timeout: Duration,
}

impl DecisionEngine {
    pub fn new(endpoint: &str, model: &str, timeout_secs: u64) -> Result<Self> {
        let client = Ollama::try_new(endpoint)?;
        Ok(Self {
            client,
            model: model.to_string(),
            timeout: Duration::from_secs(timeout_secs.max(5)),
        })
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

        let response = timeout(self.timeout, self.client.send_chat_messages(request))
            .await
            .map_err(|_| anyhow!("LLM decision timed out"))?
            .map_err(|err| anyhow!(err))?;

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

    if !input.conversation.is_empty() {
        prompt.push_str("Relevant conversation context:\n");
        for convo in &input.conversation {
            prompt.push_str(&format!(
                "- [{}] {}: {}\n",
                convo.timestamp, convo.role, convo.content
            ));
        }
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
