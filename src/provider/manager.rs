//! Provider configuration manager

use super::config::{AiType, Provider, ProvidersConfig};
use super::error::{ProviderError, ProviderResult};
use crate::common::constants::files::PROVIDERS_JSON;
use crate::config::AUTH_DIRECTORY;
use anyhow::Result;
use rand::seq::SliceRandom;
use std::{fs, path::PathBuf};

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
        if provider_id.eq_ignore_ascii_case("official")
            || provider_id.eq_ignore_ascii_case("auto")
        {
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
                "Provider ID cannot be empty".to_string(),
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

        // Validate base_url format if present
        if let Some(base_url) = &provider.base_url {
            if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
                return Err(ProviderError::InvalidConfig(format!(
                    "Base URL for provider '{}' must start with http:// or https://",
                    provider_id
                )));
            }
            // Basic URL validation - check for suspicious patterns
            if base_url.contains("..")
                || base_url.contains("javascript:")
                || base_url.contains("data:")
            {
                return Err(ProviderError::InvalidConfig(format!(
                    "Base URL for provider '{}' contains suspicious patterns",
                    provider_id
                )));
            }
        }

        // Validate environment variable keys and values
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

            // Validate environment variable value length
            const MAX_ENV_VAR_LENGTH: usize = 10000;
            if value.len() > MAX_ENV_VAR_LENGTH {
                return Err(ProviderError::InvalidConfig(format!(
                    "Environment variable '{}' for provider '{}' exceeds maximum length",
                    key, provider_id
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
            let providers_config = ProvidersConfig::create_default();
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

        Ok(config_dir.join(PROVIDERS_JSON))
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
            .map_err(|e| ProviderError::InvalidConfig(e.to_string()))?;
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

    /// Get a random provider compatible with the given AI type
    ///
    /// Returns None if no compatible providers exist (excluding "official" which has no configuration).
    /// When a compatible provider is found, returns the provider name and reference.
    ///
    /// # Arguments
    /// * `ai_type` - The AI CLI type to find a compatible provider for
    ///
    /// # Returns
    /// `Some((name, provider))` if a compatible provider is found, `None` otherwise
    pub fn get_random_compatible_provider(&self, ai_type: &AiType) -> Option<(String, &Provider)> {
        let compatible: Vec<_> = self
            .providers_config
            .providers
            .iter()
            .filter(|(name, provider)| {
                // Exclude "official" as it's an empty placeholder provider
                *name != "official" && provider.is_compatible_with(ai_type)
            })
            .collect();

        if compatible.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        compatible
            .choose(&mut rng)
            .map(|(name, provider)| ((*name).clone(), *provider))
    }

    // ===== Token Management =====
    // Note: Regional token support was removed in favor of simplified design

    /// Add custom provider
    pub fn add_custom_provider(&mut self, provider_id: String, provider: Provider) -> Result<()> {
        self.add_provider(provider_id, provider)?;
        Ok(())
    }

    // ===== SPEC-Compliant Advanced Methods =====

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
    /// use aiw::provider::manager::ProviderManager;
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
    /// use aiw::provider::manager::ProviderManager;
    ///
    /// let mut manager = ProviderManager::new().unwrap();
    /// manager.reset_to_defaults().unwrap();
    /// ```
    pub fn reset_to_defaults(&mut self) -> ProviderResult<()> {
        self.providers_config = ProvidersConfig::create_default();
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
    /// use aiw::provider::manager::ProviderManager;
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
    /// use aiw::provider::manager::ProviderManager;
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
        assert!(path.to_string_lossy().contains(".aiw"));
        assert!(path.to_string_lossy().contains("providers.json"));
    }

    #[test]
    fn test_reserved_name_protection() {
        let providers_config = ProvidersConfig::default();
        let mut manager = ProviderManager {
            config_path: PathBuf::new(),
            providers_config,
        };

        let provider = Provider {
            token: None,
            base_url: None,
            scenario: None,
            compatible_with: None,
            env: HashMap::new(),
        };

        assert!(manager
            .add_provider("official".to_string(), provider)
            .is_err());
        assert!(manager.remove_provider("official").is_err());
    }

    #[test]
    fn test_auto_is_reserved_name() {
        let providers_config = ProvidersConfig::default();
        let mut manager = ProviderManager {
            config_path: PathBuf::new(),
            providers_config,
        };

        let provider = Provider {
            token: None,
            base_url: None,
            scenario: None,
            compatible_with: None,
            env: HashMap::new(),
        };

        // "auto" should be rejected as reserved name (case-insensitive)
        assert!(manager
            .add_provider("auto".to_string(), provider.clone())
            .is_err());
        assert!(manager
            .add_provider("AUTO".to_string(), provider.clone())
            .is_err());
        assert!(manager
            .add_provider("Auto".to_string(), provider)
            .is_err());
    }

    #[test]
    fn test_provider_compatibility() {
        use crate::provider::config::AiType;

        // Provider with no compatible_with (compatible with all)
        let provider_all = Provider {
            token: Some("test".to_string()),
            base_url: None,
            scenario: None,
            compatible_with: None,
            env: HashMap::new(),
        };
        assert!(provider_all.is_compatible_with(&AiType::Claude));
        assert!(provider_all.is_compatible_with(&AiType::Codex));
        assert!(provider_all.is_compatible_with(&AiType::Gemini));

        // Provider with specific compatibility
        let provider_claude = Provider {
            token: Some("test".to_string()),
            base_url: None,
            scenario: None,
            compatible_with: Some(vec![AiType::Claude]),
            env: HashMap::new(),
        };
        assert!(provider_claude.is_compatible_with(&AiType::Claude));
        assert!(!provider_claude.is_compatible_with(&AiType::Codex));
        assert!(!provider_claude.is_compatible_with(&AiType::Gemini));

        // Provider with multiple compatibility
        let provider_multi = Provider {
            token: Some("test".to_string()),
            base_url: None,
            scenario: None,
            compatible_with: Some(vec![AiType::Claude, AiType::Codex]),
            env: HashMap::new(),
        };
        assert!(provider_multi.is_compatible_with(&AiType::Claude));
        assert!(provider_multi.is_compatible_with(&AiType::Codex));
        assert!(!provider_multi.is_compatible_with(&AiType::Gemini));
    }

    #[test]
    fn test_get_random_compatible_provider() {
        use crate::provider::config::AiType;

        let mut providers_config = ProvidersConfig::default();

        // Add a provider compatible with Claude only
        providers_config.providers.insert(
            "claude-only".to_string(),
            Provider {
                token: Some("test-claude".to_string()),
                base_url: None,
                scenario: None,
                compatible_with: Some(vec![AiType::Claude]),
                env: HashMap::new(),
            },
        );

        // Add a provider compatible with all
        providers_config.providers.insert(
            "all-types".to_string(),
            Provider {
                token: Some("test-all".to_string()),
                base_url: None,
                scenario: None,
                compatible_with: None,
                env: HashMap::new(),
            },
        );

        let manager = ProviderManager {
            config_path: PathBuf::new(),
            providers_config,
        };

        // Should find compatible providers for Claude (both claude-only and all-types)
        let result = manager.get_random_compatible_provider(&AiType::Claude);
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert!(name == "claude-only" || name == "all-types");

        // Should find only all-types for Codex
        let result = manager.get_random_compatible_provider(&AiType::Codex);
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "all-types");

        // Should find only all-types for Gemini
        let result = manager.get_random_compatible_provider(&AiType::Gemini);
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "all-types");
    }

    #[test]
    fn test_get_random_compatible_provider_no_match() {
        use crate::provider::config::AiType;

        let mut providers_config = ProvidersConfig::default();

        // Add a provider compatible with Claude only
        providers_config.providers.insert(
            "claude-only".to_string(),
            Provider {
                token: Some("test".to_string()),
                base_url: None,
                scenario: None,
                compatible_with: Some(vec![AiType::Claude]),
                env: HashMap::new(),
            },
        );

        let manager = ProviderManager {
            config_path: PathBuf::new(),
            providers_config,
        };

        // Should not find compatible providers for Codex
        let result = manager.get_random_compatible_provider(&AiType::Codex);
        assert!(result.is_none());
    }
}
