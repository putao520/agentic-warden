//! Provider configuration manager

use super::config::{AiType, Provider, ProviderConfig, ProvidersConfig, Region};
use super::error::{ProviderError, ProviderResult};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

const NEW_PROVIDER_FILE_NAME: &str = "providers.json";
const AUTH_DIRECTORY: &str = ".agentic-warden";

/// Provider configuration manager supporting both legacy and v2.0 formats
pub struct ProviderManager {
    config_path: PathBuf,
    config: ProviderConfig,
    providers_config: ProvidersConfig,
    use_v2_format: bool,
}

impl ProviderManager {
    /// Create a new ProviderManager (simplified to use only v2.0 format)
    pub fn new() -> ProviderResult<Self> {
        let config_path = Self::get_new_config_path()?;

        // Load or create v2.0 configuration
        let providers_config = if config_path.exists() {
            Self::load_v2_from_file(&config_path)?
        } else {
            let providers_config = ProvidersConfig::create_default()?;
            Self::save_v2_to_file(&config_path, &providers_config)?;
            providers_config
        };

        Ok(Self {
            config_path,
            config: ProviderConfig::default(),
            providers_config,
            use_v2_format: true,
        })
    }

    /// Create a ProviderManager with a custom path (useful for testing)
    pub fn new_with_path<P: Into<PathBuf>>(path: P) -> ProviderResult<Self> {
        let config_path = path.into();
        let providers_config = Self::load_v2_from_file(&config_path)?;

        Ok(Self {
            config_path,
            config: ProviderConfig::default(),
            providers_config,
            use_v2_format: true,
        })
    }

    /// Get configuration file path (legacy)
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

    /// Get new v2.0 configuration file path
    fn get_new_config_path() -> ProviderResult<PathBuf> {
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

    /// Load or create configuration file
    fn load_or_create(path: &PathBuf) -> ProviderResult<ProviderConfig> {
        if path.exists() {
            Self::load_from_file(path)
        } else {
            let config = ProviderConfig::default();
            Self::save_to_file(path, &config)?;
            Ok(config)
        }
    }

    /// Load configuration from file
    fn load_from_file(path: &PathBuf) -> ProviderResult<ProviderConfig> {
        let content =
            fs::read_to_string(path).map_err(|e| ProviderError::ConfigLoadError(e.to_string()))?;

        let config: ProviderConfig = serde_json::from_str(&content)
            .map_err(|e| ProviderError::ConfigLoadError(format!("Invalid JSON: {}", e)))?;

        Ok(config)
    }

    /// Save configuration to file
    fn save_to_file(path: &PathBuf, config: &ProviderConfig) -> ProviderResult<()> {
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

    /// Load v2.0 configuration from file
    fn load_v2_from_file(path: &PathBuf) -> ProviderResult<ProvidersConfig> {
        let content =
            fs::read_to_string(path).map_err(|e| ProviderError::ConfigLoadError(e.to_string()))?;

        let config: ProvidersConfig = serde_json::from_str(&content)
            .map_err(|e| ProviderError::ConfigLoadError(format!("Invalid JSON: {}", e)))?;

        Ok(config)
    }

    /// Save v2.0 configuration to file
    fn save_v2_to_file(path: &PathBuf, config: &ProvidersConfig) -> ProviderResult<()> {
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

    /// Save current configuration (simplified for v2.0 only)
    pub fn save(&self) -> ProviderResult<()> {
        let config_path = Self::get_new_config_path()?;
        Self::save_v2_to_file(&config_path, &self.providers_config)?;
        Ok(())
    }

    /// Save v2.0 configuration
    pub fn save_v2(&self) -> ProviderResult<()> {
        let new_config_path = Self::get_new_config_path()?;
        Self::save_v2_to_file(&new_config_path, &self.providers_config)?;
        Ok(())
    }

    /// Get providers configuration (v2.0 format)
    pub fn get_providers_config(&self) -> &ProvidersConfig {
        &self.providers_config
    }

    /// Get mutable providers configuration
    pub fn get_providers_config_mut(&mut self) -> &mut ProvidersConfig {
        &mut self.providers_config
    }

    /// Check if using v2.0 format
    pub fn is_using_v2_format(&self) -> bool {
        self.use_v2_format
    }

    /// Migrate to v2.0 format (no-op since we always use v2.0)
    pub fn migrate_to_v2(&mut self) -> ProviderResult<()> {
        if !self.use_v2_format {
            self.use_v2_format = true;

            // Save v2.0 format
            self.save_v2()?;
        }
        Ok(())
    }

    /// Get provider by name (v2.0 format)
    pub fn get_provider_v2(&self, name: &str) -> Option<&Provider> {
        self.providers_config.providers.get(name)
    }

    /// Get provider by name (legacy format)
    pub fn get_provider(&self, name: &str) -> ProviderResult<&Provider> {
        self.config
            .providers
            .get(name)
            .ok_or_else(|| ProviderError::ProviderNotFound(name.to_string()))
    }

    /// Get default provider (v2.0 format)
    pub fn get_default_provider_v2(&self) -> Option<(String, &Provider)> {
        let name = self.providers_config.default_provider.clone();
        let provider = self.providers_config.providers.get(&name)?;
        Some((name, provider))
    }

    /// Get default provider (legacy format)
    pub fn get_default_provider(&self) -> ProviderResult<(String, &Provider)> {
        let name = self.config.default_provider.clone();
        let provider = self.get_provider(&name)?;
        Ok((name, provider))
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
        if name == "official" {
            return Err(ProviderError::ReservedName(name));
        }

        self.config.providers.insert(name, provider);
        self.save()?;
        Ok(())
    }

    /// Remove provider
    pub fn remove_provider(&mut self, name: &str) -> ProviderResult<()> {
        if name == "official" {
            return Err(ProviderError::ReservedName(name.to_string()));
        }

        if name == self.config.default_provider {
            return Err(ProviderError::InvalidConfig(format!(
                "Cannot remove default provider '{}'. Set another default first.",
                name
            )));
        }

        self.config
            .providers
            .remove(name)
            .ok_or_else(|| ProviderError::ProviderNotFound(name.to_string()))?;

        self.save()?;
        Ok(())
    }

    /// Set default provider
    pub fn set_default(&mut self, name: &str) -> ProviderResult<()> {
        // Verify provider exists
        self.get_provider(name)?;

        self.config.default_provider = name.to_string();
        self.save()?;
        Ok(())
    }

    /// List all providers (v2.0 format)
    pub fn list_providers_v2(&self) -> Vec<(&String, &Provider)> {
        self.providers_config.providers.iter().collect()
    }

    /// List all providers (legacy format)
    pub fn list_providers(&self) -> Vec<(&String, &Provider)> {
        self.config.providers.iter().collect()
    }

    /// Get default provider name (v2.0 format)
    pub fn default_provider_name_v2(&self) -> &str {
        &self.providers_config.default_provider
    }

    /// Get default provider name (legacy format)
    pub fn default_provider_name(&self) -> &str {
        &self.config.default_provider
    }

    // ===== v2.0 Token Management =====

    /// Get token for provider and region
    pub fn get_token(&self, provider_id: &str, region: &Region) -> Option<String> {
        self.providers_config
            .get_token(provider_id, region)
            .cloned()
    }

    /// Set token for provider and region
    pub fn set_token(&mut self, provider_id: &str, region: Region, token: String) -> Result<()> {
        self.providers_config.set_token(provider_id, region, token);
        self.save_v2()?;
        Ok(())
    }

    /// Remove token for provider and region
    pub fn remove_token(&mut self, provider_id: &str, region: &Region) -> Result<()> {
        self.providers_config.remove_token(provider_id, region)?;
        self.save_v2()?;
        Ok(())
    }

    /// Check if provider has token for region
    pub fn has_token(&self, provider_id: &str, region: &Region) -> bool {
        self.providers_config.has_token(provider_id, region)
    }

    /// Set default provider (v2.0 format)
    pub fn set_default_provider(&mut self, provider_id: &str) -> Result<()> {
        if !self.providers_config.providers.contains_key(provider_id) {
            return Err(anyhow::anyhow!("Provider '{}' not found", provider_id));
        }
        self.providers_config.default_provider = provider_id.to_string();
        self.save_v2()?;
        Ok(())
    }

    /// Add custom provider (v2.0 format)
    pub fn add_custom_provider(&mut self, provider_id: String, provider: Provider) -> Result<()> {
        self.providers_config.add_provider(provider_id, provider);
        self.save_v2()?;
        Ok(())
    }

    /// Remove provider (v2.0 format)
    pub fn remove_provider_v2(&mut self, provider_id: &str) -> Result<()> {
        self.providers_config.remove_provider(provider_id);
        self.save_v2()?;
        Ok(())
    }

    /// Get all regional tokens for a provider
    pub fn get_regional_tokens(
        &self,
        provider_id: &str,
    ) -> Option<&crate::provider::config::RegionalTokens> {
        self.providers_config.user_tokens.get(provider_id)
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
        assert!(path.to_string_lossy().contains("provider.json"));
    }

    #[test]
    fn test_validate_compatibility_success() {
        let mut providers = HashMap::new();
        providers.insert(
            "test".to_string(),
            Provider {
                description: "Test".to_string(),
                compatible_with: vec![AiType::Codex],
                env: HashMap::new(),
            },
        );

        let config = ProviderConfig {
            schema: None,
            providers,
            default_provider: "test".to_string(),
        };

        let manager = ProviderManager {
            config_path: PathBuf::new(),
            config,
        };

        assert!(
            manager
                .validate_compatibility("test", AiType::Codex)
                .is_ok()
        );
    }

    #[test]
    fn test_validate_compatibility_failure() {
        let mut providers = HashMap::new();
        providers.insert(
            "test".to_string(),
            Provider {
                description: "Test".to_string(),
                compatible_with: vec![AiType::Codex],
                env: HashMap::new(),
            },
        );

        let config = ProviderConfig {
            schema: None,
            providers,
            default_provider: "test".to_string(),
        };

        let manager = ProviderManager {
            config_path: PathBuf::new(),
            config,
        };

        assert!(
            manager
                .validate_compatibility("test", AiType::Gemini)
                .is_err()
        );
    }

    #[test]
    fn test_reserved_name_protection() {
        let config = ProviderConfig::default();
        let mut manager = ProviderManager {
            config_path: PathBuf::new(),
            config,
        };

        let provider = Provider {
            description: "Test".to_string(),
            compatible_with: vec![AiType::Codex],
            env: HashMap::new(),
        };

        assert!(
            manager
                .add_provider("official".to_string(), provider)
                .is_err()
        );
        assert!(manager.remove_provider("official").is_err());
    }
}
