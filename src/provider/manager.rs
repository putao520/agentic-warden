//! Provider configuration manager

use super::config::{AiType, Provider, ProvidersConfig, Region};
use super::env_mapping::get_env_vars_for_ai_type;
use super::error::{ProviderError, ProviderResult};
use crate::config::AUTH_DIRECTORY;
use anyhow::Result;
use std::{fs, path::PathBuf};

const NEW_PROVIDER_FILE_NAME: &str = "providers.json";

/// Provider configuration manager
pub struct ProviderManager {
    config_path: PathBuf,
    providers_config: ProvidersConfig,
}

impl ProviderManager {
    fn ensure_provider_exists(&self, provider_id: &str) -> ProviderResult<()> {
        if self.providers_config.providers.contains_key(provider_id) {
            Ok(())
        } else {
            Err(ProviderError::ProviderNotFound(provider_id.to_string()))
        }
    }

    fn ensure_mutable_id(&self, provider_id: &str) -> ProviderResult<()> {
        if provider_id.eq_ignore_ascii_case("official") {
            return Err(ProviderError::ReservedName(provider_id.to_string()));
        }
        Ok(())
    }

    fn ensure_can_delete(&self, provider_id: &str) -> ProviderResult<()> {
        self.ensure_provider_exists(provider_id)?;
        self.ensure_mutable_id(provider_id)?;

        if provider_id == self.providers_config.default_provider {
            return Err(ProviderError::InvalidConfig(format!(
                "Cannot remove default provider '{}'. Set another default first.",
                provider_id
            )));
        }

        if !self.providers_config.can_delete_provider(provider_id) {
            return Err(ProviderError::InvalidConfig(format!(
                "Provider '{}' is protected and cannot be deleted",
                provider_id
            )));
        }

        Ok(())
    }

    fn validate_provider(&self, provider_id: &str, provider: &Provider) -> ProviderResult<()> {
        // Validate provider ID
        const MAX_ID_LENGTH: usize = 100;
        if provider_id.trim().is_empty() {
            return Err(ProviderError::InvalidConfig(
                "Provider ID cannot be empty".to_string()
            ));
        }
        if provider_id.len() > MAX_ID_LENGTH {
            return Err(ProviderError::InvalidConfig(format!(
                "Provider ID '{}' exceeds maximum length of {} characters",
                provider_id, MAX_ID_LENGTH
            )));
        }
        // Prevent path traversal in provider IDs
        if provider_id.contains("..") || provider_id.contains('/') || provider_id.contains('\\') {
            return Err(ProviderError::InvalidConfig(format!(
                "Provider ID '{}' contains invalid characters (.. / \\)",
                provider_id
            )));
        }

        // Validate provider name
        const MAX_NAME_LENGTH: usize = 200;
        if provider.name.trim().is_empty() {
            return Err(ProviderError::InvalidConfig(format!(
                "Display name for provider '{}' cannot be empty",
                provider_id
            )));
        }
        if provider.name.len() > MAX_NAME_LENGTH {
            return Err(ProviderError::InvalidConfig(format!(
                "Display name for provider '{}' exceeds maximum length of {} characters",
                provider_id, MAX_NAME_LENGTH
            )));
        }

        // Validate description length
        const MAX_DESC_LENGTH: usize = 1000;
        if provider.description.len() > MAX_DESC_LENGTH {
            return Err(ProviderError::InvalidConfig(format!(
                "Description for provider '{}' exceeds maximum length of {} characters",
                provider_id, MAX_DESC_LENGTH
            )));
        }

        // Validate website URL format
        if let Some(website) = &provider.website {
            if !website.starts_with("http://") && !website.starts_with("https://") {
                return Err(ProviderError::InvalidConfig(format!(
                    "Website URL for provider '{}' must start with http:// or https://",
                    provider_id
                )));
            }
            // Basic URL validation - check for suspicious patterns
            if website.contains("..") || website.contains("javascript:") || website.contains("data:") {
                return Err(ProviderError::InvalidConfig(format!(
                    "Website URL for provider '{}' contains suspicious patterns",
                    provider_id
                )));
            }
        }

        // Validate AI type compatibility
        if provider.compatible_with.is_empty() {
            return Err(ProviderError::InvalidConfig(format!(
                "Provider '{}' must be compatible with at least one AI type",
                provider_id
            )));
        }

        // Check for duplicate AI types
        let mut seen = Vec::new();
        for ai_type in &provider.compatible_with {
            if seen.contains(ai_type) {
                return Err(ProviderError::InvalidConfig(format!(
                    "Provider '{}' has duplicate AI compatibility entry '{}'",
                    provider_id, ai_type
                )));
            }
            seen.push(ai_type.clone());

            // Validate required environment variables
            let required_vars = get_env_vars_for_ai_type(ai_type.clone());
            for mapping in required_vars.into_iter().filter(|m| m.required) {
                let value = provider.env.get(mapping.key).map(|s| s.trim());
                if value.is_none() || value == Some("") {
                    return Err(ProviderError::InvalidConfig(format!(
                        "Provider '{}' missing required environment variable '{}' for {}",
                        provider_id, mapping.key, ai_type
                    )));
                }

                // Validate environment variable values
                if let Some(val) = value {
                    const MAX_ENV_VAR_LENGTH: usize = 10000;
                    if val.len() > MAX_ENV_VAR_LENGTH {
                        return Err(ProviderError::InvalidConfig(format!(
                            "Environment variable '{}' for provider '{}' exceeds maximum length",
                            mapping.key, provider_id
                        )));
                    }

                    // Check for null bytes (security issue)
                    if val.contains('\0') {
                        return Err(ProviderError::InvalidConfig(format!(
                            "Environment variable '{}' for provider '{}' contains null bytes",
                            mapping.key, provider_id
                        )));
                    }
                }
            }
        }

        // Validate environment variable keys
        for (key, value) in &provider.env {
            // Check for valid environment variable names
            if key.is_empty() || key.starts_with(char::is_numeric) {
                return Err(ProviderError::InvalidConfig(format!(
                    "Invalid environment variable name '{}' for provider '{}'",
                    key, provider_id
                )));
            }

            // Check for shell injection attempts in keys
            if key.contains(&[';', '|', '&', '`', '$', '(', ')'][..]) {
                return Err(ProviderError::InvalidConfig(format!(
                    "Environment variable name '{}' contains shell metacharacters",
                    key
                )));
            }

            // Validate value doesn't contain null bytes
            if value.contains('\0') {
                return Err(ProviderError::InvalidConfig(format!(
                    "Environment variable '{}' value contains null bytes",
                    key
                )));
            }
        }

        Ok(())
    }

    /// Create a new ProviderManager
    pub fn new() -> ProviderResult<Self> {
        let config_path = Self::get_config_path()?;

        // Load or create configuration
        let providers_config = if config_path.exists() {
            Self::load_from_file(&config_path)?
        } else {
            let providers_config = ProvidersConfig::create_default()?;
            Self::save_to_file(&config_path, &providers_config)?;
            providers_config
        };

        Ok(Self {
            config_path,
            providers_config,
        })
    }

    /// Create a ProviderManager with a custom path (useful for testing)
    pub fn new_with_path<P: Into<PathBuf>>(path: P) -> ProviderResult<Self> {
        let config_path = path.into();
        let providers_config = Self::load_from_file(&config_path)?;

        Ok(Self {
            config_path,
            providers_config,
        })
    }

    /// Get configuration file path
    fn get_config_path() -> ProviderResult<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            ProviderError::ConfigLoadError("Cannot find home directory".to_string())
        })?;

        let config_dir = home_dir.join(AUTH_DIRECTORY);

        // Ensure directory exists
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;

            // Set directory permissions (Unix only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&config_dir)?.permissions();
                perms.set_mode(0o700); // rwx------
                fs::set_permissions(&config_dir, perms)?;
            }
        }

        Ok(config_dir.join(NEW_PROVIDER_FILE_NAME))
    }

    /// Load configuration from file
    fn load_from_file(path: &PathBuf) -> ProviderResult<ProvidersConfig> {
        let content =
            fs::read_to_string(path).map_err(|e| ProviderError::ConfigLoadError(e.to_string()))?;

        let mut config: ProvidersConfig = serde_json::from_str(&content)
            .map_err(|e| ProviderError::ConfigLoadError(format!("Invalid JSON: {}", e)))?;

        config
            .ensure_defaults_and_validate()
            .map_err(|e| ProviderError::ConfigLoadError(e.to_string()))?;

        Ok(config)
    }

    /// Save configuration to file
    fn save_to_file(path: &PathBuf, config: &ProvidersConfig) -> ProviderResult<()> {
        let json = serde_json::to_string_pretty(config)?;
        fs::write(path, json)?;

        // Set file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600); // rw-------
            fs::set_permissions(path, perms)?;
        }

        Ok(())
    }

    /// Save current configuration
    pub fn save(&self) -> ProviderResult<()> {
        Self::save_to_file(&self.config_path, &self.providers_config)?;
        Ok(())
    }

    /// Get providers configuration
    pub fn get_providers_config(&self) -> &ProvidersConfig {
        &self.providers_config
    }

    /// Get mutable providers configuration
    pub fn get_providers_config_mut(&mut self) -> &mut ProvidersConfig {
        &mut self.providers_config
    }

    /// Get provider by name
    pub fn get_provider(&self, name: &str) -> ProviderResult<&Provider> {
        self.providers_config
            .get_provider(name)
            .ok_or_else(|| ProviderError::ProviderNotFound(name.to_string()))
    }

    /// Get default provider
    pub fn get_default_provider(&self) -> Option<(String, &Provider)> {
        let name = self.providers_config.default_provider.clone();
        let provider = self.providers_config.providers.get(&name)?;
        Some((name, provider))
    }

    /// Validate provider compatibility with AI type
    pub fn validate_compatibility(
        &self,
        provider_name: &str,
        ai_type: AiType,
    ) -> ProviderResult<()> {
        let provider = self.get_provider(provider_name)?;

        if !provider.compatible_with.contains(&ai_type) {
            let compatible = provider
                .compatible_with
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            return Err(ProviderError::IncompatibleProvider {
                provider: provider_name.to_string(),
                ai_type: ai_type.to_string(),
                compatible,
            });
        }

        Ok(())
    }

    /// Add new provider
    pub fn add_provider(&mut self, name: String, provider: Provider) -> ProviderResult<()> {
        self.ensure_mutable_id(&name)?;
        if self.providers_config.providers.contains_key(&name) {
            return Err(ProviderError::DuplicateProvider(name));
        }

        self.validate_provider(&name, &provider)?;
        self.providers_config.add_provider(name.clone(), provider);
        self.save()?;
        Ok(())
    }

    /// Update existing provider
    pub fn update_provider(&mut self, name: &str, provider: Provider) -> ProviderResult<()> {
        self.ensure_provider_exists(name)?;
        self.ensure_mutable_id(name)?;
        self.validate_provider(name, &provider)?;

        self.providers_config
            .add_provider(name.to_string(), provider);
        self.save()?;
        Ok(())
    }

    /// Remove provider
    pub fn remove_provider(&mut self, name: &str) -> ProviderResult<()> {
        self.ensure_can_delete(name)?;

        self.providers_config
            .remove_provider(name)
            .ok_or_else(|| ProviderError::ProviderNotFound(name.to_string()))?;
        self.save()?;
        Ok(())
    }

    /// Set default provider
    pub fn set_default(&mut self, name: &str) -> ProviderResult<()> {
        // Verify provider exists
        self.ensure_provider_exists(name)?;

        self.providers_config.default_provider = name.to_string();
        self.save()?;
        Ok(())
    }

    /// Set default provider (alternative method)
    pub fn set_default_provider(&mut self, provider_id: &str) -> Result<()> {
        if !self.providers_config.providers.contains_key(provider_id) {
            return Err(anyhow::anyhow!("Provider '{}' not found", provider_id));
        }
        self.providers_config.default_provider = provider_id.to_string();
        self.save()?;
        Ok(())
    }

    /// List all providers
    pub fn list_providers(&self) -> Vec<(&String, &Provider)> {
        let mut providers: Vec<_> = self.providers_config.providers.iter().collect();
        providers.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));
        providers
    }

    /// Get default provider name
    pub fn default_provider_name(&self) -> &str {
        &self.providers_config.default_provider
    }

    // ===== Token Management =====

    /// Get token for provider and region
    pub fn get_token(&self, provider_id: &str, region: &Region) -> Option<String> {
        self.providers_config
            .get_token(provider_id, region)
            .cloned()
    }

    /// Set token for provider and region
    pub fn set_token(&mut self, provider_id: &str, region: Region, token: String) -> Result<()> {
        self.providers_config.set_token(provider_id, region, token);
        self.save()?;
        Ok(())
    }

    /// Remove token for provider and region
    pub fn remove_token(&mut self, provider_id: &str, region: &Region) -> Result<()> {
        self.providers_config.remove_token(provider_id, region)?;
        self.save()?;
        Ok(())
    }

    /// Check if provider has token for region
    pub fn has_token(&self, provider_id: &str, region: &Region) -> bool {
        self.providers_config.has_token(provider_id, region)
    }

    /// Add custom provider
    pub fn add_custom_provider(&mut self, provider_id: String, provider: Provider) -> Result<()> {
        self.add_provider(provider_id, provider)?;
        Ok(())
    }

    /// Get all regional tokens for a provider
    pub fn get_regional_tokens(
        &self,
        provider_id: &str,
    ) -> Option<&crate::provider::config::RegionalTokens> {
        self.providers_config.user_tokens.get(provider_id)
    }

    // ===== SPEC-Compliant Advanced Methods =====

    /// Get all providers compatible with specified AI type
    ///
    /// Returns a list of (provider_id, provider) tuples for providers that
    /// support the given AI CLI type.
    ///
    /// # Arguments
    /// * `ai_type` - The AI type to filter by (Codex, Claude, Gemini)
    ///
    /// # Returns
    /// Vector of tuples containing provider ID and provider reference
    ///
    /// # Example
    /// ```no_run
    /// use agentic_warden::provider::manager::ProviderManager;
    /// use agentic_warden::provider::config::AiType;
    ///
    /// let manager = ProviderManager::new().unwrap();
    /// let claude_providers = manager.get_compatible_providers(&AiType::Claude);
    /// for (id, provider) in claude_providers {
    ///     println!("{}: {}", id, provider.name);
    /// }
    /// ```
    pub fn get_compatible_providers(&self, ai_type: &AiType) -> Vec<(&String, &Provider)> {
        self.providers_config
            .providers
            .iter()
            .filter(|(_, provider)| provider.compatible_with.contains(ai_type))
            .collect()
    }

    /// Validate all providers in configuration
    ///
    /// Performs validation on all providers and returns a list of warnings.
    /// Providers with errors will be included in the warnings list.
    ///
    /// # Returns
    /// `Ok(warnings)` - Vector of warning messages
    /// `Err(error)` - Critical validation failure
    ///
    /// # Example
    /// ```no_run
    /// use agentic_warden::provider::manager::ProviderManager;
    ///
    /// let manager = ProviderManager::new().unwrap();
    /// match manager.validate_all_providers() {
    ///     Ok(warnings) if warnings.is_empty() => {
    ///         println!("All providers valid");
    ///     }
    ///     Ok(warnings) => {
    ///         for warning in warnings {
    ///             eprintln!("Warning: {}", warning);
    ///         }
    ///     }
    ///     Err(e) => eprintln!("Validation failed: {}", e),
    /// }
    /// ```
    pub fn validate_all_providers(&self) -> ProviderResult<Vec<String>> {
        let mut warnings = Vec::new();

        // Validate each provider
        for (provider_id, provider) in &self.providers_config.providers {
            match self.validate_provider(provider_id, provider) {
                Ok(_) => {}
                Err(e) => {
                    warnings.push(format!("Provider '{}': {}", provider_id, e));
                }
            }
        }

        // Validate default provider exists
        if !self
            .providers_config
            .providers
            .contains_key(&self.providers_config.default_provider)
        {
            warnings.push(format!(
                "Default provider '{}' does not exist",
                self.providers_config.default_provider
            ));
        }

        // Check for providers without tokens (optional warning)
        for (provider_id, provider) in &self.providers_config.providers {
            if !provider.custom && !self.providers_config.user_tokens.contains_key(provider_id) {
                let needs_token = provider
                    .env
                    .values()
                    .any(|v| v.contains("{{token}}") || v.contains("${TOKEN}"));
                if needs_token {
                    warnings.push(format!(
                        "Provider '{}' may require token configuration",
                        provider_id
                    ));
                }
            }
        }

        Ok(warnings)
    }

    /// Reset configuration to defaults
    ///
    /// Resets the entire configuration to factory defaults, removing all
    /// custom providers and user tokens. This operation cannot be undone.
    ///
    /// # Returns
    /// `Ok(())` on success, `Err` on failure to save
    ///
    /// # Example
    /// ```no_run
    /// use agentic_warden::provider::manager::ProviderManager;
    ///
    /// let mut manager = ProviderManager::new().unwrap();
    /// manager.reset_to_defaults().unwrap();
    /// ```
    pub fn reset_to_defaults(&mut self) -> ProviderResult<()> {
        self.providers_config = ProvidersConfig::create_default()?;
        self.save()?;
        Ok(())
    }

    /// Export configuration to file
    ///
    /// Exports the current configuration to a JSON file for backup or sharing.
    /// The exported file includes all providers and user tokens.
    ///
    /// # Arguments
    /// * `path` - Path where configuration will be exported
    ///
    /// # Returns
    /// `Ok(())` on success, `Err` on IO failure
    ///
    /// # Example
    /// ```no_run
    /// use agentic_warden::provider::manager::ProviderManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ProviderManager::new().unwrap();
    /// manager.export_config(&PathBuf::from("/tmp/providers_backup.json")).unwrap();
    /// ```
    pub fn export_config(&self, path: &PathBuf) -> ProviderResult<()> {
        Self::save_to_file(path, &self.providers_config)?;
        Ok(())
    }

    /// Import configuration from file
    ///
    /// Imports provider configuration from a JSON file. Can either replace
    /// the current configuration or merge with it.
    ///
    /// # Arguments
    /// * `path` - Path to configuration file to import
    /// * `merge` - If true, merge with existing config; if false, replace entirely
    ///
    /// # Returns
    /// `Ok(())` on success, `Err` on validation or IO failure
    ///
    /// # Example
    /// ```no_run
    /// use agentic_warden::provider::manager::ProviderManager;
    /// use std::path::PathBuf;
    ///
    /// let mut manager = ProviderManager::new().unwrap();
    ///
    /// // Replace entire configuration
    /// manager.import_config(&PathBuf::from("/tmp/providers.json"), false).unwrap();
    ///
    /// // Merge with existing configuration
    /// manager.import_config(&PathBuf::from("/tmp/extra_providers.json"), true).unwrap();
    /// ```
    pub fn import_config(&mut self, path: &PathBuf, merge: bool) -> ProviderResult<()> {
        let imported_config = Self::load_from_file(path)?;

        if merge {
            // Merge imported providers with existing ones
            for (provider_id, provider) in imported_config.providers {
                // Validate before adding
                self.validate_provider(&provider_id, &provider)?;

                // Add or update provider
                self.providers_config
                    .add_provider(provider_id.clone(), provider);
            }

            // Merge tokens
            for (provider_id, imported_regional_tokens) in imported_config.user_tokens {
                if let Some(existing_tokens) = self.providers_config.user_tokens.get_mut(&provider_id)
                {
                    // Merge regional tokens - update non-None values
                    if imported_regional_tokens.mainland_china.is_some() {
                        existing_tokens.mainland_china = imported_regional_tokens.mainland_china;
                    }
                    if imported_regional_tokens.international.is_some() {
                        existing_tokens.international = imported_regional_tokens.international;
                    }
                    if imported_regional_tokens.last_updated.is_some() {
                        existing_tokens.last_updated = imported_regional_tokens.last_updated;
                    }
                } else {
                    self.providers_config
                        .user_tokens
                        .insert(provider_id, imported_regional_tokens);
                }
            }

            // Keep existing default provider if imported default doesn't exist locally
            if !self
                .providers_config
                .providers
                .contains_key(&imported_config.default_provider)
            {
                // Keep current default
            } else {
                // Optionally update default provider
                self.providers_config.default_provider = imported_config.default_provider;
            }
        } else {
            // Replace entire configuration
            self.providers_config = imported_config;
        }

        self.save()?;
        Ok(())
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            let config_path = PathBuf::from("providers.json");
            let providers_config = ProvidersConfig::default();
            Self {
                config_path,
                providers_config,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_get_config_path() {
        let path = ProviderManager::get_config_path().unwrap();
        assert!(path.to_string_lossy().contains(".agentic-warden"));
        assert!(path.to_string_lossy().contains("providers.json"));
    }

    #[test]
    fn test_validate_compatibility_success() {
        let mut providers = HashMap::new();
        providers.insert(
            "test".to_string(),
            Provider {
                name: "test".to_string(),
                description: "Test".to_string(),
                icon: None,
                official: false,
                protected: false,
                custom: false,
                support_modes: vec![],
                compatible_with: vec![AiType::Codex],
                validation_endpoint: None,
                category: None,
                website: None,
                regions: vec![],
                env: HashMap::new(),
            },
        );

        let providers_config = ProvidersConfig {
            schema: None,
            providers,
            default_provider: "test".to_string(),
            user_tokens: HashMap::new(),
        };

        let manager = ProviderManager {
            config_path: PathBuf::new(),
            providers_config,
        };

        assert!(manager
            .validate_compatibility("test", AiType::Codex)
            .is_ok());
    }

    #[test]
    fn test_validate_compatibility_failure() {
        let mut providers = HashMap::new();
        providers.insert(
            "test".to_string(),
            Provider {
                name: "test".to_string(),
                description: "Test".to_string(),
                icon: None,
                official: false,
                protected: false,
                custom: false,
                support_modes: vec![],
                compatible_with: vec![AiType::Codex],
                validation_endpoint: None,
                category: None,
                website: None,
                regions: vec![],
                env: HashMap::new(),
            },
        );

        let providers_config = ProvidersConfig {
            schema: None,
            providers,
            default_provider: "test".to_string(),
            user_tokens: HashMap::new(),
        };

        let manager = ProviderManager {
            config_path: PathBuf::new(),
            providers_config,
        };

        assert!(manager
            .validate_compatibility("test", AiType::Gemini)
            .is_err());
    }

    #[test]
    fn test_reserved_name_protection() {
        let providers_config = ProvidersConfig::default();
        let mut manager = ProviderManager {
            config_path: PathBuf::new(),
            providers_config,
        };

        let provider = Provider {
            name: "test".to_string(),
            description: "Test".to_string(),
            icon: None,
            official: false,
            protected: false,
            custom: false,
            support_modes: vec![],
            compatible_with: vec![AiType::Codex],
            validation_endpoint: None,
            category: None,
            website: None,
            regions: vec![],
            env: HashMap::new(),
        };

        assert!(manager
            .add_provider("official".to_string(), provider)
            .is_err());
        assert!(manager.remove_provider("official").is_err());
    }
}
