use std::future::Future;

use ollama_rs::generation::chat::{request::ChatMessageRequest, ChatMessage};
use ollama_rs::Ollama;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::runtime::Handle;
use tokio::time::timeout;

use crate::auto_mode::{ExecutionResult, Judgment, LLM_TIMEOUT, OLLAMA_ENDPOINT, OLLAMA_MODEL};
use crate::error::JudgeError;

const PROMPT_TEMPLATE: &str = r#"你是一个 AI CLI 执行结果分析器。请判断以下执行是否成功，是否应该尝试下一个 AI CLI。

**AI CLI 类型**: {cli_type}
**用户任务**: {prompt}
**退出码**: {exit_code}
**标准输出**: {stdout}
**错误输出**: {stderr}

请以 JSON 格式返回判断结果：
{{
  "success": true/false,
  "should_retry": true/false,
  "reason": "判断理由"
}}

判断规则：
- 退出码 0 + 正常输出 → success=true
- 网络错误、API 错误、连接失败 → should_retry=true
- 用户中断、权限问题、非法参数 → should_retry=false
"#;

static SENSITIVE_KV_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(api[_-]?key|token|secret)\s*[:=]\s*([^\s\x22\x27{}]{6,})").expect("regex")
});
static BEARER_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)bearer\s+[A-Za-z0-9_.-]+").expect("regex"));
static SK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"sk-[A-Za-z0-9]{8,}").expect("regex"));

pub struct AiJudge;

impl AiJudge {
    pub fn evaluate(result: &ExecutionResult) -> Result<Judgment, JudgeError> {
        let prompt = Self::build_prompt(result);
        let response = Self::run_async(Self::send_prompt(prompt))?;
        Self::parse_llm_response(&response)
    }

    pub fn build_prompt(result: &ExecutionResult) -> String {
        let prompt = Self::redact_sensitive(&result.prompt);
        let stdout = Self::redact_sensitive(&result.stdout);
        let stderr = Self::redact_sensitive(&result.stderr);

        PROMPT_TEMPLATE
            .replace("{cli_type}", result.cli_type.display_name())
            .replace("{prompt}", &prompt)
            .replace("{exit_code}", &result.exit_code.to_string())
            .replace("{stdout}", &stdout)
            .replace("{stderr}", &stderr)
    }

    pub fn parse_llm_response(response: &str) -> Result<Judgment, JudgeError> {
        let normalized = Self::strip_code_fences(response);
        serde_json::from_str::<Judgment>(&normalized).map_err(|err| {
            JudgeError::InvalidResponse {
                message: err.to_string(),
            }
        })
    }

    async fn send_prompt(prompt: String) -> Result<String, JudgeError> {
        let client = Ollama::try_new(OLLAMA_ENDPOINT).map_err(|err| JudgeError::Api {
            message: err.to_string(),
        })?;

        let request = ChatMessageRequest::new(
            OLLAMA_MODEL.to_string(),
            vec![ChatMessage::user(prompt)],
        );

        let response = timeout(LLM_TIMEOUT, client.send_chat_messages(request))
            .await
            .map_err(|_| JudgeError::Timeout {
                timeout_secs: LLM_TIMEOUT.as_secs(),
            })?
            .map_err(|err| Self::classify_llm_error(&err.to_string()))?;

        Ok(response.message.content)
    }

    fn strip_code_fences(input: &str) -> String {
        let trimmed = input.trim();
        if !trimmed.starts_with("```") {
            return trimmed.to_string();
        }

        let without_fence = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```");
        let without_trailing = match without_fence.rfind("```") {
            Some(index) => &without_fence[..index],
            None => without_fence,
        };
        without_trailing.trim().to_string()
    }

    fn redact_sensitive(input: &str) -> String {
        let masked = SENSITIVE_KV_PATTERN.replace_all(input, "$1: ***");
        let masked = BEARER_PATTERN.replace_all(&masked, "bearer ***");
        let masked = SK_PATTERN.replace_all(&masked, "sk-***");
        masked.to_string()
    }

    fn classify_llm_error(message: &str) -> JudgeError {
        let message_lower = message.to_lowercase();
        if message_lower.contains("connection")
            || message_lower.contains("connection refused")
            || message_lower.contains("failed to connect")
            || message_lower.contains("tcp")
        {
            JudgeError::Unavailable
        } else {
            JudgeError::Api {
                message: message.to_string(),
            }
        }
    }

    fn run_async<T, F>(future: F) -> Result<T, JudgeError>
    where
        F: Future<Output = Result<T, JudgeError>>,
    {
        match Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
            Err(_) => {
                let runtime = tokio::runtime::Runtime::new().map_err(|err| JudgeError::Api {
                    message: err.to_string(),
                })?;
                runtime.block_on(future)
            }
        }
    }
}
