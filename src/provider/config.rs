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

    /// Memory configuration for semantic search
    #[serde(default)]
    pub memory: Option<crate::memory::MemoryConfig>,
}

/// Single Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Display name
    pub name: String,

    /// Provider description
    pub description: String,

    /// Provider icon (emoji)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Official provider (predefined)
    #[serde(default)]
    pub official: bool,

    /// Protected status (cannot be deleted)
    #[serde(default)]
    pub protected: bool,

    /// Custom provider (user created)
    #[serde(default)]
    pub custom: bool,

    /// Compatible AI types
    pub compatible_with: Vec<AiType>,

    /// Environment variables for this provider
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

impl ProvidersConfig {
    fn default_schema() -> String {
        DEFAULT_SCHEMA_URL.to_string()
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

        for (provider_id, provider) in &self.providers {
            self.validate_provider_entry(provider_id, provider)?;
        }

        Ok(())
    }

    /// Validate a single provider entry
    fn validate_provider_entry(&self, provider_id: &str, provider: &Provider) -> Result<()> {
        if provider.name.is_empty() {
            return Err(anyhow!("Provider '{}' has empty name", provider_id));
        }

        if provider.compatible_with.is_empty() {
            return Err(anyhow!(
                "Provider '{}' must specify at least one compatible AI type",
                provider_id
            ));
        }

        Ok(())
    }
}

impl Provider {
    /// Check if this provider is compatible with the given AI type
    pub fn is_compatible_with(&self, ai_type: &AiType) -> bool {
        self.compatible_with.contains(ai_type)
    }

    /// Get environment variables for this provider
    pub fn get_env_vars(&self) -> &HashMap<String, String> {
        &self.env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_compatibility() {
        let provider = Provider {
            name: "Test Provider".to_string(),
            description: "Test description".to_string(),
            icon: None,
            official: false,
            protected: false,
            custom: true,
            compatible_with: vec![AiType::Claude, AiType::Codex],
            env: HashMap::new(),
        };

        assert!(provider.is_compatible_with(&AiType::Claude));
        assert!(provider.is_compatible_with(&AiType::Codex));
        assert!(!provider.is_compatible_with(&AiType::Gemini));
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
    fn test_config_validation() {
        let mut config = ProvidersConfig {
            schema: None,
            providers: HashMap::new(),
            default_provider: "test".to_string(),
            memory: None,
        };

        // Empty providers should fail
        assert!(config.validate().is_err());

        // Add a valid provider
        config.providers.insert(
            "test".to_string(),
            Provider {
                name: "Test".to_string(),
                description: "Test provider".to_string(),
                icon: None,
                official: false,
                protected: false,
                custom: true,
                compatible_with: vec![AiType::Claude],
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
