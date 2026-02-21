//! Environment variable mapping for AI types

use super::config::AiType;

/// Environment variable mapping
#[derive(Debug, Clone)]
pub struct EnvVarMapping {
    /// Environment variable key
    pub key: &'static str,
    /// Description
    pub description: &'static str,
    /// Whether required
    pub required: bool,
}

/// Get environment variables for a specific AI type
pub fn get_env_vars_for_ai_type(ai_type: AiType) -> Vec<EnvVarMapping> {
    match ai_type {
        AiType::Codex => vec![
            EnvVarMapping {
                key: "OPENAI_API_KEY",
                description: "OpenAI API Key",
                required: true,
            },
            EnvVarMapping {
                key: "OPENAI_BASE_URL",
                description: "API endpoint (optional)",
                required: false,
            },
            EnvVarMapping {
                key: "OPENAI_ORG_ID",
                description: "Organization ID (optional)",
                required: false,
            },
        ],
        AiType::Claude => vec![
            EnvVarMapping {
                key: "ANTHROPIC_API_KEY",
                description: "Anthropic API Key",
                required: true,
            },
            EnvVarMapping {
                key: "ANTHROPIC_BASE_URL",
                description: "API endpoint (optional)",
                required: false,
            },
        ],
        AiType::Gemini => vec![
            EnvVarMapping {
                key: "GOOGLE_API_KEY",
                description: "Google API Key",
                required: true,
            },
            EnvVarMapping {
                key: "https_proxy",
                description: "HTTPS proxy (optional)",
                required: false,
            },
        ],
        AiType::Auto => vec![],
    }
}

// Tests removed - Environment mapping is tested via integration tests
