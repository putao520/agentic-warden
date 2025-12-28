//! Provider configuration data structures

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_SCHEMA_URL: &str = "https://agentic-warden.dev/schema/provider.json";

/// Provider configuration file root structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// JSON Schema (optional)
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// All provider configurations
    pub providers: HashMap<String, Provider>,

    /// Default provider name
    pub default_provider: String,
}

/// Single Provider configuration - 最简化版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// API Token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    /// Base URL for API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Scenario description - when to use this provider
    /// 场景描述 - 何时使用此供应商
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,

    /// Compatible AI CLI types (None = compatible with all types)
    /// 兼容的 AI CLI 类型（None 表示兼容所有类型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatible_with: Option<Vec<AiType>>,

    /// All environment variables (includes token and base_url mappings)
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// AI type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, std::hash::Hash)]
#[serde(rename_all = "lowercase")]
pub enum AiType {
    Codex,
    Claude,
    Gemini,
}

impl std::fmt::Display for AiType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiType::Codex => write!(f, "codex"),
            AiType::Claude => write!(f, "claude"),
            AiType::Gemini => write!(f, "gemini"),
        }
    }
}

impl std::str::FromStr for AiType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "codex" => Ok(AiType::Codex),
            "claude" => Ok(AiType::Claude),
            "gemini" => Ok(AiType::Gemini),
            _ => Err(format!("Unknown AI type: {}", s)),
        }
    }
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self::create_default()
    }
}

impl ProvidersConfig {
    fn default_schema() -> String {
        DEFAULT_SCHEMA_URL.to_string()
    }

    /// Create a default configuration with official provider
    pub fn create_default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "official".to_string(),
            Provider {
                token: None,
                base_url: None,
                scenario: None,
                compatible_with: None,
                env: HashMap::new(),
            },
        );

        Self {
            schema: Some(Self::default_schema()),
            providers,
            default_provider: "official".to_string(),
        }
    }

    /// Load configuration from file (placeholder - use manager for actual loading)
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Self = serde_json::from_str(&content)?;
        config.ensure_defaults_and_validate()?;
        Ok(config)
    }

    /// Add a provider to the configuration
    pub fn add_provider(&mut self, id: String, provider: Provider) {
        self.providers.insert(id, provider);
    }

    /// Remove a provider from the configuration
    pub fn remove_provider(&mut self, id: &str) -> Result<()> {
        if id == self.default_provider {
            return Err(anyhow!("Cannot remove the default provider '{}'", id));
        }
        if !self.can_delete_provider(id) {
            return Err(anyhow!(
                "Provider '{}' is protected and cannot be deleted",
                id
            ));
        }
        self.providers.remove(id);
        Ok(())
    }

    /// Get a provider by ID
    pub fn get_provider(&self, id: &str) -> Option<&Provider> {
        self.providers.get(id)
    }

    /// Check if a provider can be deleted
    pub fn can_delete_provider(&self, id: &str) -> bool {
        // Protect the "official" provider
        id != "official" && id != self.default_provider
    }

    /// Ensure optional fields have defaults applied
    pub fn ensure_defaults(&mut self) {
        if self.schema.is_none() {
            self.schema = Some(Self::default_schema());
        }
    }

    /// Apply defaults and validate configuration integrity
    pub fn ensure_defaults_and_validate(&mut self) -> Result<()> {
        self.ensure_defaults();
        self.validate()
    }

    /// Validate configuration integrity
    pub fn validate(&self) -> Result<()> {
        if self.providers.is_empty() {
            return Err(anyhow!("Provider configuration is empty"));
        }

        if !self.providers.contains_key(&self.default_provider) {
            return Err(anyhow!(
                "Default provider '{}' does not exist",
                self.default_provider
            ));
        }

        Ok(())
    }
}

impl Provider {
    /// Check if this provider is compatible with given AI type
    /// Returns true if compatible_with is None (compatible with all) or contains the ai_type
    pub fn is_compatible_with(&self, ai_type: &AiType) -> bool {
        match &self.compatible_with {
            None => true, // None means compatible with all types
            Some(types) => types.contains(ai_type),
        }
    }

    /// Get all environment variables including token and base_url
    pub fn get_all_env_vars(&self) -> HashMap<String, String> {
        let mut env = self.env.clone();

        // Add token if present
        if let Some(token) = &self.token {
            // Try to infer the token env var name, default to ANTHROPIC_API_KEY
            if !env.contains_key("ANTHROPIC_API_KEY") && !env.contains_key("OPENAI_API_KEY") {
                env.insert("ANTHROPIC_API_KEY".to_string(), token.clone());
            }
        }

        // Add base_url if present
        if let Some(base_url) = &self.base_url {
            if !env.contains_key("ANTHROPIC_BASE_URL") && !env.contains_key("OPENAI_BASE_URL") {
                env.insert("ANTHROPIC_BASE_URL".to_string(), base_url.clone());
            }
        }

        env
    }

    /// Create a default provider with only env vars (for backward compatibility)
    pub fn from_env(env: HashMap<String, String>) -> Self {
        Self {
            token: None,
            base_url: None,
            scenario: None,
            compatible_with: None,
            env,
        }
    }

    /// Get a summary string for display
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(scenario) = &self.scenario {
            parts.push(format!("scenario: {}", scenario));
        }
        if self.token.is_some() {
            parts.push("token: ✓".to_string());
        }
        if let Some(url) = &self.base_url {
            parts.push(format!("url: {}", url));
        }
        if !self.env.is_empty() {
            parts.push(format!("env: {} vars", self.env.len()));
        }
        if parts.is_empty() {
            "empty".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_env_vars() {
        let provider = Provider {
            token: Some("sk-test-token".to_string()),
            base_url: Some("https://api.example.com".to_string()),
            scenario: None,
            compatible_with: None,
            env: {
                let mut map = HashMap::new();
                map.insert("CUSTOM_VAR".to_string(), "value".to_string());
                map
            },
        };

        let all_env = provider.get_all_env_vars();
        assert!(all_env.contains_key("ANTHROPIC_API_KEY"));
        assert!(all_env.contains_key("ANTHROPIC_BASE_URL"));
        assert!(all_env.contains_key("CUSTOM_VAR"));
        assert_eq!(all_env.get("ANTHROPIC_API_KEY").unwrap(), "sk-test-token");
    }

    #[test]
    fn test_aitype_from_str() {
        use std::str::FromStr;
        assert_eq!(AiType::from_str("codex").unwrap(), AiType::Codex);
        assert_eq!(AiType::from_str("claude").unwrap(), AiType::Claude);
        assert_eq!(AiType::from_str("gemini").unwrap(), AiType::Gemini);
        assert_eq!(AiType::from_str("CLAUDE").unwrap(), AiType::Claude);
        assert!(AiType::from_str("unknown").is_err());
    }

    #[test]
    fn test_provider_with_scenario() {
        let provider = Provider {
            token: Some("sk-test".to_string()),
            base_url: Some("https://api.example.com".to_string()),
            scenario: Some("Best for production workloads".to_string()),
            compatible_with: None,
            env: HashMap::new(),
        };

        let summary = provider.summary();
        assert!(summary.contains("scenario:"));
        assert!(summary.contains("Best for production workloads"));
    }

    #[test]
    fn test_provider_backward_compatibility() {
        let json = r#"{"token":"sk-test","base_url":"https://api.test.com","env":{}}"#;
        let provider: Provider = serde_json::from_str(json).expect("should deserialize");
        assert!(provider.scenario.is_none());
    }

    #[test]
    fn test_config_validation() {
        let mut config = ProvidersConfig {
            schema: None,
            providers: HashMap::new(),
            default_provider: "test".to_string(),
        };

        // Empty providers should fail
        assert!(config.validate().is_err());

        // Add a valid provider
        config.providers.insert(
            "test".to_string(),
            Provider {
                token: Some("sk-test".to_string()),
                base_url: Some("https://api.test.com".to_string()),
                scenario: None,
                compatible_with: None,
                env: HashMap::new(),
            },
        );

        // Should pass now
        assert!(config.validate().is_ok());

        // Invalid default provider should fail
        config.default_provider = "nonexistent".to_string();
        assert!(config.validate().is_err());
    }
}
