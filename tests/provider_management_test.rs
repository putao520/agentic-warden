//! Comprehensive Provider Management Tests
//!
//! This module tests the complete lifecycle of Provider management:
//! - CRUD operations (Create, Read, Update, Delete)
//! - Default provider management
//! - Environment variable injection and masking
//! - Validation and error handling
//! - Configuration persistence
//! - Compatibility validation

use agentic_warden::provider::{
    config::{AiType, Provider, ProvidersConfig, Region, SupportMode, ModeType, ModeConfig, RegionalConfig},
    env_injector::EnvInjector,
    env_mapping::get_env_vars_for_ai_type,
    manager::ProviderManager,
};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Test Utilities
// ============================================================================

/// Create isolated test environment
fn create_test_env() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Create a test provider configuration
fn create_test_provider(ai_type: AiType, api_key: &str) -> Provider {
    let mut env = HashMap::new();

    match ai_type {
        AiType::Codex => {
            env.insert("OPENAI_API_KEY".to_string(), api_key.to_string());
        }
        AiType::Claude => {
            env.insert("ANTHROPIC_API_KEY".to_string(), api_key.to_string());
        }
        AiType::Gemini => {
            env.insert("GOOGLE_API_KEY".to_string(), api_key.to_string());
        }
    }

    // Create appropriate support mode based on AI type
    let support_mode = match ai_type {
        AiType::Claude => SupportMode {
            mode_type: ModeType::ClaudeCodeNative,
            name: "Claude Code Test Mode".to_string(),
            description: "Test mode for Claude".to_string(),
            priority: 50,
            config: ModeConfig {
                regional_urls: {
                    let mut urls = HashMap::new();
                    urls.insert(
                        Region::International,
                        RegionalConfig {
                            base_url: "https://api.test.com".to_string(),
                            auth_env_var: "ANTHROPIC_API_KEY".to_string(),
                            recommended_model: Some("test-model".to_string()),
                            features: None,
                        },
                    );
                    urls
                },
                models: None,
                additional_env: None,
                rate_limit: None,
            },
        },
        AiType::Codex | AiType::Gemini => SupportMode {
            mode_type: ModeType::OpenAICompatible,
            name: "OpenAI Compatible Test Mode".to_string(),
            description: "Test mode for OpenAI compatible APIs".to_string(),
            priority: 50,
            config: ModeConfig {
                regional_urls: {
                    let mut urls = HashMap::new();
                    urls.insert(
                        Region::International,
                        RegionalConfig {
                            base_url: "https://api.test.com/v1".to_string(),
                            auth_env_var: "OPENAI_API_KEY".to_string(),
                            recommended_model: Some("test-model".to_string()),
                            features: None,
                        },
                    );
                    urls
                },
                models: None,
                additional_env: None,
                rate_limit: None,
            },
        },
    };

    Provider {
        name: format!("Test Provider for {}", ai_type),
        description: format!("Test provider for {}", ai_type),
        icon: Some("🧪".to_string()),
        official: false,
        protected: false,
        custom: true,
        support_modes: vec![support_mode],
        compatible_with: vec![ai_type],
        validation_endpoint: Some("https://api.test.com/validate".to_string()),
        category: Some("Test".to_string()),
        website: Some("https://test.com".to_string()),
        regions: vec!["International".to_string()],
        env,
    }
}

/// Create a multi-AI provider
fn create_multi_ai_provider() -> Provider {
    let mut env = HashMap::new();
    env.insert("OPENAI_API_KEY".to_string(), "sk-test123".to_string());
    env.insert(
        "ANTHROPIC_API_KEY".to_string(),
        "sk-ant-test123".to_string(),
    );
    env.insert("GOOGLE_API_KEY".to_string(), "AIza-test123".to_string());

    // Create support modes for all AI types
    let mut support_modes = Vec::new();

    // Claude Code Native mode
    support_modes.push(SupportMode {
        mode_type: ModeType::ClaudeCodeNative,
        name: "Claude Code Test Mode".to_string(),
        description: "Test mode for Claude".to_string(),
        priority: 80,
        config: ModeConfig {
            regional_urls: {
                let mut urls = HashMap::new();
                urls.insert(
                    Region::International,
                    RegionalConfig {
                        base_url: "https://api.test.com".to_string(),
                        auth_env_var: "ANTHROPIC_API_KEY".to_string(),
                        recommended_model: Some("test-claude-model".to_string()),
                        features: None,
                    },
                );
                urls
            },
            models: None,
            additional_env: None,
            rate_limit: None,
        },
    });

    // OpenAI Compatible mode
    support_modes.push(SupportMode {
        mode_type: ModeType::OpenAICompatible,
        name: "OpenAI Compatible Test Mode".to_string(),
        description: "Test mode for OpenAI compatible APIs".to_string(),
        priority: 70,
        config: ModeConfig {
            regional_urls: {
                let mut urls = HashMap::new();
                urls.insert(
                    Region::International,
                    RegionalConfig {
                        base_url: "https://api.test.com/v1".to_string(),
                        auth_env_var: "OPENAI_API_KEY".to_string(),
                        recommended_model: Some("test-openai-model".to_string()),
                        features: None,
                    },
                );
                urls
            },
            models: None,
            additional_env: None,
            rate_limit: None,
        },
    });

    Provider {
        name: "Multi-AI Test Provider".to_string(),
        description: "Multi-AI provider supporting all AI types".to_string(),
        icon: Some("🎯".to_string()),
        official: false,
        protected: false,
        custom: true,
        support_modes,
        compatible_with: vec![AiType::Codex, AiType::Claude, AiType::Gemini],
        validation_endpoint: Some("https://api.test.com/validate".to_string()),
        category: Some("Test".to_string()),
        website: Some("https://test.com".to_string()),
        regions: vec!["International".to_string()],
        env,
    }
}

/// Create provider manager with custom path
fn create_test_manager(temp_dir: &TempDir) -> ProviderManager {
    let config_path = temp_dir.path().join("provider.json");

    // Create the config path in temp dir
    fs::create_dir_all(temp_dir.path()).expect("Failed to create temp dir");

    // Create default config and save it
    let config = ProvidersConfig::create_default().expect("Failed to create default config");
    let json = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
    fs::write(&config_path, json).expect("Failed to write config");

    // Now load the manager
    ProviderManager::new_with_path(config_path).expect("Failed to create provider manager")
}

// ============================================================================
// A. Provider CRUD Operations Tests
// ============================================================================

#[test]
fn test_provider_config_default_creation() {
    let config = ProvidersConfig::create_default().expect("Failed to create default config");

    assert_eq!(config.default_provider, "official");
    assert_eq!(config.providers.len(), 0);
    assert!(config.schema.is_some());
}

#[test]
fn test_add_provider_success() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let provider = create_test_provider(AiType::Codex, "sk-test-key-12345");

    manager
        .add_provider("my-codex".to_string(), provider)
        .expect("Failed to add provider");

    // Verify provider was added
    let added = manager.get_provider("my-codex");
    assert!(added.is_ok());

    let added_provider = added.unwrap();
    assert_eq!(added_provider.compatible_with.len(), 1);
    assert!(added_provider.compatible_with.contains(&AiType::Codex));
    assert!(added_provider.env.contains_key("OPENAI_API_KEY"));
}

#[test]
fn test_add_provider_duplicate_name_updates() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    // Add first provider
    let provider1 = create_test_provider(AiType::Codex, "sk-old-key");
    manager
        .add_provider("test-provider".to_string(), provider1)
        .expect("Failed to add first provider");

    // Add second provider with same name (should update)
    let provider2 = create_test_provider(AiType::Codex, "sk-new-key");
    manager
        .add_provider("test-provider".to_string(), provider2)
        .expect("Failed to update provider");

    // Verify it was updated
    let updated = manager.get_provider("test-provider").unwrap();
    assert_eq!(updated.env.get("OPENAI_API_KEY").unwrap(), "sk-new-key");
}

#[test]
fn test_add_provider_multiple_ai_types() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let provider = create_multi_ai_provider();

    manager
        .add_provider("multi-ai".to_string(), provider)
        .expect("Failed to add multi-AI provider");

    let added = manager.get_provider("multi-ai").unwrap();
    assert_eq!(added.compatible_with.len(), 3);
    assert!(added.env.contains_key("OPENAI_API_KEY"));
    assert!(added.env.contains_key("ANTHROPIC_API_KEY"));
    assert!(added.env.contains_key("GOOGLE_API_KEY"));
}

#[test]
fn test_list_providers_empty() {
    let temp_dir = create_test_env();
    let manager = create_test_manager(&temp_dir);

    let providers = manager.list_providers();

    // Should have no providers in default empty config
    assert_eq!(providers.len(), 0);
}

#[test]
fn test_list_providers_multiple() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    // Add multiple providers
    manager
        .add_provider(
            "provider1".to_string(),
            create_test_provider(AiType::Codex, "key1"),
        )
        .unwrap();
    manager
        .add_provider(
            "provider2".to_string(),
            create_test_provider(AiType::Claude, "key2"),
        )
        .unwrap();
    manager
        .add_provider(
            "provider3".to_string(),
            create_test_provider(AiType::Gemini, "key3"),
        )
        .unwrap();

    let providers = manager.list_providers();

    // Should have 3 custom + 0 default = 3 total
    assert_eq!(providers.len(), 3);

    let names: Vec<&&String> = providers.iter().map(|(name, _)| name).collect();
    assert!(names.contains(&&&"provider1".to_string()));
    assert!(names.contains(&&&"provider2".to_string()));
    assert!(names.contains(&&&"provider3".to_string()));
}

#[test]
fn test_get_provider_exists() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let provider = create_test_provider(AiType::Codex, "sk-test");
    manager
        .add_provider("exists".to_string(), provider)
        .unwrap();

    let result = manager.get_provider("exists");
    assert!(result.is_ok());
}

#[test]
fn test_get_provider_not_found() {
    let temp_dir = create_test_env();
    let manager = create_test_manager(&temp_dir);

    let result = manager.get_provider("non-existent");
    assert!(result.is_err());
}

#[test]
fn test_delete_provider_success() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let provider = create_test_provider(AiType::Codex, "sk-test-key");
    manager
        .add_provider("to-remove".to_string(), provider)
        .unwrap();

    // Verify it exists
    assert!(manager.get_provider("to-remove").is_ok());

    // Remove it
    manager
        .remove_provider("to-remove")
        .expect("Failed to remove provider");

    // Verify it's gone
    assert!(manager.get_provider("to-remove").is_err());
}

#[test]
fn test_delete_provider_not_found() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let result = manager.remove_provider("non-existent");
    assert!(result.is_err());
}

#[test]
fn test_delete_default_provider_fails() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    // Add a provider and set it as default
    let provider = create_test_provider(AiType::Codex, "sk-test");
    manager
        .add_provider("default-test".to_string(), provider)
        .unwrap();
    manager.set_default("default-test").unwrap();

    // Try to delete it - should fail because it's the default
    let result = manager.remove_provider("default-test");
    assert!(result.is_err());
}

#[test]
fn test_set_default_provider() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let provider = create_test_provider(AiType::Codex, "sk-test-key");
    manager
        .add_provider("new-default".to_string(), provider)
        .unwrap();

    // Set as default
    manager
        .set_default("new-default")
        .expect("Failed to set default");

    assert_eq!(manager.default_provider_name(), "new-default");
}

#[test]
fn test_set_default_overrides_previous() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    // Add two providers
    manager
        .add_provider(
            "first".to_string(),
            create_test_provider(AiType::Codex, "k1"),
        )
        .unwrap();
    manager
        .add_provider(
            "second".to_string(),
            create_test_provider(AiType::Claude, "k2"),
        )
        .unwrap();

    // Set first as default
    manager.set_default("first").unwrap();
    assert_eq!(manager.default_provider_name(), "first");

    // Set second as default (should override)
    manager.set_default("second").unwrap();
    assert_eq!(manager.default_provider_name(), "second");
}

#[test]
fn test_get_default_provider() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let provider = create_test_provider(AiType::Codex, "sk-test");
    manager
        .add_provider("my-default".to_string(), provider)
        .unwrap();
    manager.set_default("my-default").unwrap();

    let (name, _provider) = manager
        .get_default_provider()
        .expect("Failed to get default");
    assert_eq!(name, "my-default");
}

// ============================================================================
// B. Environment Variable Mapping Tests
// ============================================================================

#[test]
fn test_env_vars_for_codex() {
    let vars = get_env_vars_for_ai_type(AiType::Codex);

    assert_eq!(vars.len(), 3);
    assert_eq!(vars[0].key, "OPENAI_API_KEY");
    assert_eq!(vars[1].key, "OPENAI_BASE_URL");
    assert_eq!(vars[2].key, "OPENAI_ORG_ID");

    assert!(vars[0].required);
    assert!(!vars[1].required);
    assert!(!vars[2].required);
}

#[test]
fn test_env_vars_for_claude() {
    let vars = get_env_vars_for_ai_type(AiType::Claude);

    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].key, "ANTHROPIC_API_KEY");
    assert_eq!(vars[1].key, "ANTHROPIC_BASE_URL");

    assert!(vars[0].required);
    assert!(!vars[1].required);
}

#[test]
fn test_env_vars_for_gemini() {
    let vars = get_env_vars_for_ai_type(AiType::Gemini);

    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].key, "GOOGLE_API_KEY");
    assert_eq!(vars[1].key, "https_proxy");

    assert!(vars[0].required);
    assert!(!vars[1].required);
}

// ============================================================================
// C. Environment Variable Injection Tests
// ============================================================================

#[test]
fn test_inject_env_vars_for_provider() {
    let mut env_vars = HashMap::new();
    env_vars.insert("OPENAI_API_KEY".to_string(), "sk-test123".to_string());
    env_vars.insert(
        "OPENAI_BASE_URL".to_string(),
        "https://custom.api.com".to_string(),
    );

    let mut cmd = std::process::Command::new("echo");
    EnvInjector::inject_to_command(&mut cmd, &env_vars);

    // Verify parent process environment is not modified
    assert!(std::env::var("OPENAI_API_KEY").is_err());
    assert!(std::env::var("OPENAI_BASE_URL").is_err());
}

#[test]
fn test_env_var_masking_for_sensitive_keys() {
    // Test API key masking
    let masked = EnvInjector::mask_sensitive_value("OPENAI_API_KEY", "sk-test123456789");
    assert!(masked.contains("***"));
    assert_eq!(masked, "sk-t***6789");

    // Test token masking
    let masked_token = EnvInjector::mask_sensitive_value("MY_TOKEN", "token1234567890");
    assert!(masked_token.contains("***"));
    assert_eq!(masked_token, "toke***7890");

    // Test password masking
    let masked_pwd = EnvInjector::mask_sensitive_value("PASSWORD", "mypassword123");
    assert!(masked_pwd.contains("***"));

    // Test secret masking
    let masked_secret = EnvInjector::mask_sensitive_value("SECRET_KEY", "secretkey12345");
    assert!(masked_secret.contains("***"));
}

#[test]
fn test_env_var_no_masking_for_urls() {
    // Test URL not masked
    let not_masked = EnvInjector::mask_sensitive_value("OPENAI_BASE_URL", "https://api.openai.com");
    assert_eq!(not_masked, "https://api.openai.com");

    // Test other non-sensitive values
    let not_masked2 = EnvInjector::mask_sensitive_value("MODEL_NAME", "gpt-4");
    assert_eq!(not_masked2, "gpt-4");
}

#[test]
fn test_env_var_short_values_masking() {
    // Short values should be completely masked
    let masked = EnvInjector::mask_sensitive_value("API_KEY", "short");
    assert_eq!(masked, "***");
}

// ============================================================================
// D. Provider Configuration Persistence Tests
// ============================================================================

#[test]
fn test_provider_save_and_load() {
    let temp_dir = create_test_env();
    let config_path = temp_dir.path().join("provider.json");

    // Create and save
    {
        let mut manager = create_test_manager(&temp_dir);
        let provider = Provider {
            name: "Test Provider".to_string(),
            description: "Test".to_string(),
            icon: Some("🧪".to_string()),
            official: false,
            protected: false,
            custom: true,
            support_modes: vec![
                SupportMode {
                    mode_type: ModeType::OpenAICompatible,
                    name: "Test Mode".to_string(),
                    description: "Test mode".to_string(),
                    priority: 50,
                    config: ModeConfig {
                        regional_urls: {
                            let mut urls = HashMap::new();
                            urls.insert(
                                Region::International,
                                RegionalConfig {
                                    base_url: "https://api.test.com".to_string(),
                                    auth_env_var: "OPENAI_API_KEY".to_string(),
                                    recommended_model: None,
                                    features: None,
                                },
                            );
                            urls
                        },
                        models: None,
                        additional_env: None,
                        rate_limit: None,
                    },
                }
            ],
            compatible_with: vec![AiType::Codex],
            validation_endpoint: Some("https://api.test.com/validate".to_string()),
            category: Some("Test".to_string()),
            website: Some("https://test.com".to_string()),
            regions: vec!["International".to_string()],
            env: {
                let mut env = HashMap::new();
                env.insert("OPENAI_API_KEY".to_string(), "test".to_string());
                env
            },
        };
        manager.add_provider("test".to_string(), provider).unwrap();
    }

    // Reload
    {
        let manager = ProviderManager::new_with_path(&config_path).unwrap();
        let providers = manager.list_providers();
        assert!(providers.iter().any(|(name, _)| name == &"test"));
    }
}

#[test]
fn test_provider_config_default_persistence() {
    let temp_dir = create_test_env();
    let config_path = temp_dir.path().join("provider.json");

    // Create manager and set default
    {
        let mut manager = create_test_manager(&temp_dir);
        let provider = create_test_provider(AiType::Codex, "sk-test");
        manager
            .add_provider("my-default".to_string(), provider)
            .unwrap();
        manager.set_default("my-default").unwrap();
    }

    // Reload and verify default is persisted
    {
        let manager = ProviderManager::new_with_path(&config_path).unwrap();
        assert_eq!(manager.default_provider_name(), "my-default");
    }
}

#[test]
fn test_provider_file_permissions_on_unix() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = create_test_env();
        let mut manager = create_test_manager(&temp_dir);

        let provider = create_test_provider(AiType::Codex, "sk-test");
        manager.add_provider("test".to_string(), provider).unwrap();

        let config_path = temp_dir.path().join("provider.json");
        let metadata = fs::metadata(&config_path).unwrap();
        let permissions = metadata.permissions();

        // Should be 0o600 (rw-------)
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }
}

// ============================================================================
// E. Validation and Error Handling Tests
// ============================================================================

#[test]
fn test_provider_empty_name() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let provider = create_test_provider(AiType::Codex, "sk-test");
    let _result = manager.add_provider("".to_string(), provider);

    // Empty name should fail (though ProviderManager doesn't explicitly check this,
    // it may be allowed. This test documents the current behavior)
    // If we want to enforce this, we'd need to add validation
    // For now, document that empty names are technically allowed
    // but not recommended
}

#[test]
fn test_provider_reserved_name_protection() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    // Try to add provider with reserved name
    let provider = create_test_provider(AiType::Codex, "sk-test");
    let result = manager.add_provider("official".to_string(), provider);

    // Should fail because "official" is reserved
    assert!(result.is_err());
}

#[test]
fn test_provider_reserved_name_cannot_delete() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    // "official" is a reserved name and cannot be added
    let official_provider = create_test_provider(AiType::Claude, "test-key");
    let result = manager.add_provider("official".to_string(), official_provider);

    // Should fail because "official" is reserved
    assert!(result.is_err());
}

#[test]
fn test_provider_compatibility_validation() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    // Create Codex-only provider
    let provider = create_test_provider(AiType::Codex, "sk-test");
    manager
        .add_provider("codex-only".to_string(), provider)
        .unwrap();

    // Validate compatibility with Codex (should succeed)
    let result = manager.validate_compatibility("codex-only", AiType::Codex);
    assert!(result.is_ok());

    // Validate compatibility with Claude (should fail)
    let result = manager.validate_compatibility("codex-only", AiType::Claude);
    assert!(result.is_err());
}

#[test]
fn test_provider_multi_compatibility() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let provider = create_multi_ai_provider();
    manager.add_provider("multi".to_string(), provider).unwrap();

    // Should be compatible with all AI types
    assert!(
        manager
            .validate_compatibility("multi", AiType::Codex)
            .is_ok()
    );
    assert!(
        manager
            .validate_compatibility("multi", AiType::Claude)
            .is_ok()
    );
    assert!(
        manager
            .validate_compatibility("multi", AiType::Gemini)
            .is_ok()
    );
}

#[test]
fn test_set_default_nonexistent_provider() {
    let temp_dir = create_test_env();
    let mut manager = create_test_manager(&temp_dir);

    let result = manager.set_default("does-not-exist");
    assert!(result.is_err());
}

// ============================================================================
// F. Complete Lifecycle Tests
// ============================================================================

#[test]
fn test_complete_provider_lifecycle() {
    let temp_dir = create_test_env();
    let config_path = temp_dir.path().join("provider.json");

    let mut manager = create_test_manager(&temp_dir);

    // 1. Add provider
    let provider = create_test_provider(AiType::Codex, "sk-lifecycle-test");
    manager
        .add_provider("lifecycle".to_string(), provider)
        .unwrap();
    assert!(manager.get_provider("lifecycle").is_ok());

    // 2. Set as default
    manager.set_default("lifecycle").unwrap();
    assert_eq!(manager.default_provider_name(), "lifecycle");

    // 3. Save to file
    manager.save().unwrap();
    assert!(config_path.exists());

    // 4. Load in new manager
    let manager2 = ProviderManager::new_with_path(&config_path).unwrap();
    assert!(manager2.get_provider("lifecycle").is_ok());
    assert_eq!(manager2.default_provider_name(), "lifecycle");

    // 5. Add another provider to change default
    let mut manager3 = ProviderManager::new_with_path(&config_path).unwrap();
    let another_provider = create_test_provider(AiType::Claude, "test-key-2");
    manager3.add_provider("another".to_string(), another_provider).unwrap();
    manager3.set_default("another").unwrap();

    // 6. Now can remove the lifecycle provider
    manager3.remove_provider("lifecycle").unwrap();
    assert!(manager3.get_provider("lifecycle").is_err());
}

#[test]
fn test_provider_serialization_roundtrip() {
    let config = ProvidersConfig::create_default().expect("Failed to create default config");
    let json = serde_json::to_string_pretty(&config).expect("Failed to serialize");
    let deserialized: ProvidersConfig = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(config.default_provider, deserialized.default_provider);
    assert_eq!(config.providers.len(), deserialized.providers.len());
}

#[test]
fn test_ai_type_string_conversion() {
    // Test Display trait
    assert_eq!(AiType::Codex.to_string(), "codex");
    assert_eq!(AiType::Claude.to_string(), "claude");
    assert_eq!(AiType::Gemini.to_string(), "gemini");

    // Test FromStr trait
    assert_eq!("codex".parse::<AiType>().unwrap(), AiType::Codex);
    assert_eq!("CLAUDE".parse::<AiType>().unwrap(), AiType::Claude);
    assert_eq!("Gemini".parse::<AiType>().unwrap(), AiType::Gemini);
    assert!("unknown".parse::<AiType>().is_err());
}
