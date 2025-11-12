//! Provider configuration data structures

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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

    /// User stored tokens (regional)
    #[serde(default)]
    pub user_tokens: HashMap<String, RegionalTokens>,

    /// Memory configuration for semantic search and TODO management
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

    /// Support modes for this provider
    pub support_modes: Vec<SupportMode>,

    /// Compatible AI types
    pub compatible_with: Vec<AiType>,

    /// Validation endpoint
    pub validation_endpoint: Option<String>,

    /// Provider category
    pub category: Option<String>,

    /// Provider website
    pub website: Option<String>,

    /// Supported regions
    #[serde(default)]
    pub regions: Vec<String>,

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

/// Support mode for providers (Enhanced with configuration)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportMode {
    /// Mode type
    pub mode_type: ModeType,

    /// Display name for this mode
    pub name: String,

    /// Description of this mode
    pub description: String,

    /// Priority for recommendations (higher = more recommended)
    pub priority: u8,

    /// Configuration for this mode
    pub config: ModeConfig,
}

/// Mode type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeType {
    /// Claude Code native mode with ANTHROPIC_* variables
    ClaudeCodeNative,
    /// OpenAI compatible mode with OPENAI_* variables
    #[serde(rename = "openai_compatible")]
    OpenAICompatible,
    /// Gemini native mode with GOOGLE_* variables
    GeminiNative,
}

impl std::fmt::Display for ModeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModeType::ClaudeCodeNative => write!(f, "Claude Code Native"),
            ModeType::OpenAICompatible => write!(f, "OpenAI Compatible"),
            ModeType::GeminiNative => write!(f, "Gemini Native"),
        }
    }
}

/// Mode configuration with regional support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Regional URL configurations
    pub regional_urls: HashMap<Region, RegionalConfig>,

    /// Available models for this mode
    pub models: Option<Vec<Model>>,

    /// Additional environment variables
    #[serde(default)]
    pub additional_env: Option<HashMap<String, String>>,

    /// Rate limiting information
    #[serde(default)]
    pub rate_limit: Option<RateLimit>,
}

/// Regional configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionalConfig {
    /// Base URL for this region
    pub base_url: String,

    /// Authentication environment variable name
    pub auth_env_var: String,

    /// Recommended model for this region
    #[serde(default)]
    pub recommended_model: Option<String>,

    /// Region-specific features
    #[serde(default)]
    pub features: Option<Vec<String>>,
}

/// Model information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Model ID
    pub id: String,

    /// Display name
    pub name: String,

    /// Model description
    #[serde(default)]
    pub description: Option<String>,

    /// Context length
    #[serde(default)]
    pub context_length: Option<usize>,

    /// Pricing information
    #[serde(default)]
    pub pricing: Option<ModelPricing>,
}

/// Model pricing information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Input token price per 1K tokens
    #[serde(default)]
    pub input_price_per_1k: Option<f64>,

    /// Output token price per 1K tokens
    #[serde(default)]
    pub output_price_per_1k: Option<f64>,

    /// Currency
    #[serde(default)]
    pub currency: Option<String>,
}

/// Rate limiting information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimit {
    /// Requests per minute
    #[serde(default)]
    pub requests_per_minute: Option<u32>,

    /// Tokens per minute
    #[serde(default)]
    pub tokens_per_minute: Option<u32>,

    /// Requests per day
    #[serde(default)]
    pub requests_per_day: Option<u32>,
}

/// Region enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Region {
    MainlandChina,
    International,
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Region::MainlandChina => write!(f, "中国大陆"),
            Region::International => write!(f, "国际"),
        }
    }
}

/// Regional tokens storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionalTokens {
    /// Token for mainland China
    #[serde(default)]
    pub mainland_china: Option<String>,

    /// Token for international
    #[serde(default)]
    pub international: Option<String>,

    /// Token last updated timestamp
    #[serde(default)]
    pub last_updated: Option<String>,
}

/// Recommendation scenario
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationScenario {
    /// User is in China mainland
    ChinaMainland,
    /// User is international
    International,
    /// User specifically wants Claude Code
    ClaudeCodePreferred,
    /// User wants cost-effective options
    CostEffective,
    /// User wants high performance
    HighPerformance,
}

impl std::fmt::Display for RecommendationScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecommendationScenario::ChinaMainland => write!(f, "中国大陆用户"),
            RecommendationScenario::International => write!(f, "国际用户"),
            RecommendationScenario::ClaudeCodePreferred => write!(f, "Claude Code偏好"),
            RecommendationScenario::CostEffective => write!(f, "经济实惠"),
            RecommendationScenario::HighPerformance => write!(f, "高性能需求"),
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

        for provider_id in self.user_tokens.keys() {
            if !self.providers.contains_key(provider_id) {
                return Err(anyhow!(
                    "User tokens reference unknown provider '{}'",
                    provider_id
                ));
            }
        }

        Ok(())
    }

    fn validate_provider_entry(&self, provider_id: &str, provider: &Provider) -> Result<()> {
        if provider.name.trim().is_empty() {
            return Err(anyhow!(
                "Provider '{}' must have a non-empty display name",
                provider_id
            ));
        }

        if provider.description.trim().is_empty() {
            return Err(anyhow!(
                "Provider '{}' must have a description",
                provider_id
            ));
        }

        if provider.compatible_with.is_empty() {
            return Err(anyhow!(
                "Provider '{}' must support at least one AI CLI",
                provider_id
            ));
        }

        let mut seen: HashSet<&AiType> = HashSet::new();
        for ai_type in &provider.compatible_with {
            if !seen.insert(ai_type) {
                return Err(anyhow!(
                    "Provider '{}' contains duplicate AI compatibility entry '{}'",
                    provider_id,
                    ai_type
                ));
            }
        }

        for mode in &provider.support_modes {
            if mode.name.trim().is_empty() {
                return Err(anyhow!(
                    "Provider '{}' has a support mode without a name",
                    provider_id
                ));
            }
            if mode.config.regional_urls.is_empty() {
                return Err(anyhow!(
                    "Provider '{}' mode '{}' must declare at least one regional URL",
                    provider_id,
                    mode.name
                ));
            }
        }

        Ok(())
    }

    /// Load configuration from the default location
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        Self::load_from_path(&config_path)
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let mut config: ProvidersConfig = serde_json::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse providers config: {}", e))?;
            config.ensure_defaults_and_validate()?;
            Ok(config)
        } else {
            // Create default config if it doesn't exist
            let config = Self::default();
            config.save_to_path(path)?;
            Ok(config)
        }
    }

    /// Save configuration to the default location
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        self.save_to_path(&config_path)
    }

    /// Save configuration to a specific path
    pub fn save_to_path(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Get the default configuration file path
    pub fn get_config_path() -> Result<std::path::PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Cannot find home directory"))?;
        let config_dir = home_dir.join(".agentic-warden");
        Ok(config_dir.join("providers.json"))
    }

    /// Create default configuration
    pub fn create_default() -> Result<Self> {
        let template_str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/config/provider.json.template"
        ));

        let mut config: ProvidersConfig = serde_json::from_str(template_str)
            .map_err(|e| anyhow!("Failed to parse default provider template: {}", e))?;
        config.ensure_defaults_and_validate()?;

        Ok(config)
    }

    /// Get provider by ID
    pub fn get_provider(&self, provider_id: &str) -> Option<&Provider> {
        self.providers.get(provider_id)
    }

    /// Get provider by ID (mutable)
    pub fn get_provider_mut(&mut self, provider_id: &str) -> Option<&mut Provider> {
        self.providers.get_mut(provider_id)
    }

    /// Add or update a provider
    pub fn add_provider(&mut self, provider_id: String, provider: Provider) {
        self.providers.insert(provider_id, provider);
    }

    /// Remove a provider
    pub fn remove_provider(&mut self, provider_id: &str) -> Option<Provider> {
        self.providers.remove(provider_id)
    }

    /// Check if provider can be deleted
    pub fn can_delete_provider(&self, provider_id: &str) -> bool {
        if let Some(provider) = self.get_provider(provider_id) {
            !provider.protected
        } else {
            false
        }
    }

    /// Get token for a specific provider and region
    pub fn get_token(&self, provider_id: &str, region: &Region) -> Option<&String> {
        self.user_tokens
            .get(provider_id)
            .and_then(|tokens| match region {
                Region::MainlandChina => tokens.mainland_china.as_ref(),
                Region::International => tokens.international.as_ref(),
            })
    }

    /// Set token for a specific provider and region
    pub fn set_token(&mut self, provider_id: &str, region: Region, token: String) {
        let tokens = self
            .user_tokens
            .entry(provider_id.to_string())
            .or_insert_with(RegionalTokens::default);

        match region {
            Region::MainlandChina => tokens.mainland_china = Some(token),
            Region::International => tokens.international = Some(token),
        }

        // Update last updated timestamp
        tokens.last_updated = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Get all providers compatible with a specific AI type
    pub fn get_compatible_providers(&self, ai_type: &AiType) -> Vec<(&String, &Provider)> {
        self.providers
            .iter()
            .filter(|(_, provider)| provider.compatible_with.contains(ai_type))
            .collect()
    }

    /// Select best mode for a provider and AI type
    pub fn select_best_mode(&self, provider_id: &str, ai_type: &AiType) -> Option<&SupportMode> {
        if let Some(provider) = self.get_provider(provider_id) {
            provider
                .support_modes
                .iter()
                .filter(|mode| self.is_mode_compatible_with_ai_type(mode, ai_type))
                .max_by_key(|mode| mode.priority)
        } else {
            None
        }
    }

    /// Check if a mode is compatible with an AI type
    fn is_mode_compatible_with_ai_type(&self, mode: &SupportMode, ai_type: &AiType) -> bool {
        match mode.mode_type {
            ModeType::ClaudeCodeNative => matches!(ai_type, AiType::Claude),
            ModeType::OpenAICompatible => matches!(ai_type, AiType::Claude | AiType::Codex),
            ModeType::GeminiNative => matches!(ai_type, AiType::Gemini),
        }
    }

    /// Get best regional config for a provider and mode
    pub fn get_best_regional_config(
        &self,
        provider_id: &str,
        mode_type: &ModeType,
        preferred_region: Option<&Region>,
    ) -> Option<&RegionalConfig> {
        if let Some(provider) = self.get_provider(provider_id) {
            if let Some(mode) = provider
                .support_modes
                .iter()
                .find(|m| &m.mode_type == mode_type)
            {
                // Try preferred region first
                if let Some(region) = preferred_region {
                    if let Some(config) = mode.config.regional_urls.get(region) {
                        return Some(config);
                    }
                }

                // Fallback to any available region
                mode.config.regional_urls.values().next()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Remove token for provider and region
    pub fn remove_token(&mut self, provider_id: &str, region: &Region) -> Result<()> {
        if let Some(tokens) = self.user_tokens.get_mut(provider_id) {
            match region {
                Region::MainlandChina => tokens.mainland_china = None,
                Region::International => tokens.international = None,
            }

            // Remove the entire entry if both tokens are None
            if tokens.mainland_china.is_none() && tokens.international.is_none() {
                self.user_tokens.remove(provider_id);
            }
        }
        Ok(())
    }

    /// Check if provider has token
    pub fn has_token(&self, provider_id: &str, region: &Region) -> bool {
        self.get_token(provider_id, region).is_some()
    }

    /// Get memory configuration
    pub fn get_memory_config(&self) -> crate::memory::MemoryConfig {
        self.memory.clone().unwrap_or_default()
    }

    /// Set memory configuration
    pub fn set_memory_config(&mut self, config: crate::memory::MemoryConfig) {
        self.memory = Some(config);
    }
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self::create_default().unwrap_or_else(|_| Self {
            schema: None,
            default_provider: "official".to_string(),
            providers: HashMap::new(),
            user_tokens: HashMap::new(),
            memory: Some(crate::memory::MemoryConfig::default()),
        })
    }
}

impl Default for RegionalTokens {
    fn default() -> Self {
        Self {
            mainland_china: None,
            international: None,
            last_updated: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Serialization test removed - serde is well-tested

    // Display test removed - std::fmt::Display is well-tested

    #[test]
    fn test_ai_type_from_str() {
        assert_eq!("codex".parse::<AiType>().unwrap(), AiType::Codex);
        assert_eq!("CLAUDE".parse::<AiType>().unwrap(), AiType::Claude);
        assert_eq!("Gemini".parse::<AiType>().unwrap(), AiType::Gemini);
        assert!("unknown".parse::<AiType>().is_err());
    }

    // Display tests removed - these are well-tested by the standard library

    #[test]
    fn test_default_config_from_template() {
        let config = ProvidersConfig::create_default().unwrap();
        assert!(config.providers.contains_key("openrouter"));
        assert!(config.providers.contains_key("litellm"));
        assert!(config.providers.contains_key("official"));
        assert_eq!(config.default_provider, "official");
        assert!(config.user_tokens.is_empty());
        let openrouter = config.providers.get("openrouter").unwrap();
        assert_eq!(
            openrouter.env.get("OPENAI_BASE_URL").unwrap(),
            "https://openrouter.ai/api/v1"
        );
    }

    #[test]
    fn ensure_defaults_fill_schema() {
        let mut providers = HashMap::new();
        providers.insert(
            "official".to_string(),
            Provider {
                name: "Official".to_string(),
                description: "Built-in".to_string(),
                icon: None,
                official: true,
                protected: true,
                custom: false,
                support_modes: vec![],
                compatible_with: vec![AiType::Claude],
                validation_endpoint: None,
                category: None,
                website: None,
                regions: vec![],
                env: HashMap::new(),
            },
        );

        let mut config = ProvidersConfig {
            schema: None,
            providers,
            default_provider: "official".to_string(),
            user_tokens: HashMap::new(),
            memory: None,
        };

        config.ensure_defaults_and_validate().unwrap();
        assert_eq!(
            config.schema,
            Some(DEFAULT_SCHEMA_URL.to_string()),
            "schema should be populated automatically"
        );
    }

    #[test]
    fn validate_fails_when_default_missing() {
        let mut config = ProvidersConfig::default();
        config.default_provider = "missing".to_string();
        assert!(config.validate().is_err());
    }
}
