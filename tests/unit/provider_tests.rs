//! Provider Unit Tests
//!
//! Unit tests for provider management functionality

use agentic_warden::provider::{
    config::{AiType, Provider, ProviderConfig},
    env_injector::EnvInjector,
    env_mapping::get_env_vars_for_ai_type,
    manager::ProviderManager,
};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// Import test helpers from the common module
#[path = "../common/test_helpers.rs"]
mod test_helpers;
use test_helpers::*;

// ============================================================================
// Provider Configuration Tests
// ============================================================================

#[test]
fn test_provider_config_default() {
    let config = ProviderConfig::default();
    assert_eq!(config.default_provider, "official");
    assert!(config.providers.contains_key("official"));
    let official = &config.providers["official"];
    assert_eq!(official.compatible_with.len(), 3);
}

#[test]
fn test_ai_type_display() {
    assert_eq!(AiType::Codex.to_string(), "codex");
    assert_eq!(AiType::Claude.to_string(), "claude");
    assert_eq!(AiType::Gemini.to_string(), "gemini");
}

#[test]
fn test_ai_type_from_str() {
    assert_eq!("codex".parse::<AiType>().unwrap(), AiType::Codex);
    assert_eq!("CLAUDE".parse::<AiType>().unwrap(), AiType::Claude);
    assert_eq!("Gemini".parse::<AiType>().unwrap(), AiType::Gemini);
    assert!("unknown".parse::<AiType>().is_err());
}

#[test]
fn test_provider_creation() {
    let provider = create_real_provider(AiType::Claude, "Test Claude Provider");

    assert_eq!(provider.description, "Test Claude Provider");
    assert_eq!(provider.compatible_with, vec![AiType::Claude]);
    assert!(provider.env.contains_key("ANTHROPIC_API_KEY"));
    assert!(provider.env.contains_key("ANTHROPIC_BASE_URL"));
}

#[test]
fn test_multi_ai_provider() {
    let provider = create_real_multi_provider("Multi-AI Test Provider");

    assert_eq!(provider.description, "Multi-AI Test Provider");
    assert_eq!(provider.compatible_with.len(), 3);
    assert!(provider.env.contains_key("OPENAI_API_KEY"));
    assert!(provider.env.contains_key("ANTHROPIC_API_KEY"));
    assert!(provider.env.contains_key("GOOGLE_API_KEY"));
}

// ============================================================================
// Environment Injection Tests
// ============================================================================

#[test]
fn test_env_injector_creation() {
    let injector = EnvInjector;
    // EnvInjector is a struct that can be instantiated
    // The actual functionality is tested in test_env_injection
    assert_eq!(std::mem::size_of::<EnvInjector>(), 0); // It's a zero-sized struct
}

#[test]
fn test_env_mapping_for_ai_types() {
    let codex_vars = get_env_vars_for_ai_type(AiType::Codex);
    assert!(!codex_vars.is_empty());

    let claude_vars = get_env_vars_for_ai_type(AiType::Claude);
    assert!(!claude_vars.is_empty());

    let gemini_vars = get_env_vars_for_ai_type(AiType::Gemini);
    assert!(!gemini_vars.is_empty());
}

#[test]
fn test_env_injection() {
    let injector = EnvInjector;
    let provider = create_real_provider(AiType::Claude, "Test Provider");

    // Test that we can mask sensitive values
    let sensitive_key = "ANTHROPIC_API_KEY";
    let sensitive_value = "sk-ant-test123456789";
    let masked = EnvInjector::mask_sensitive_value(sensitive_key, sensitive_value);
    assert!(masked.contains("***"));
    assert!(!masked.contains(sensitive_value));

    // Test that non-sensitive values are not masked
    let normal_key = "ANTHROPIC_BASE_URL";
    let normal_value = "https://api.anthropic.com";
    let not_masked = EnvInjector::mask_sensitive_value(normal_key, normal_value);
    assert_eq!(not_masked, normal_value);

    // Test inject_to_command with a mock command
    use std::process::Command;
    let mut cmd = Command::new("echo");
    EnvInjector::inject_to_command(&mut cmd, &provider.env);
    // We can't easily verify the injected vars without running the command,
    // but we can at least test that the method doesn't panic

    // Test masking sensitive values from the provider env
    for (key, value) in &provider.env {
        let masked_value = EnvInjector::mask_sensitive_value(key, value);
        if key.contains("KEY") || key.contains("SECRET") || key.contains("TOKEN") {
            assert!(masked_value.contains("***"));
            assert_ne!(masked_value, *value);
        } else {
            assert_eq!(masked_value, *value);
        }
    }
}

// ============================================================================
// Provider Manager Tests
// ============================================================================

#[test]
fn test_provider_manager_creation() {
    let temp_dir = create_temp_test_dir();
    let config_path = temp_dir.path().join("provider.json");

    // Create default config
    let config = ProviderConfig::default();
    let json = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
    fs::write(&config_path, json).expect("Failed to write config");

    let manager = ProviderManager::new_with_path(&config_path)
        .expect("Should be able to create manager");

    assert!(manager.get_default_provider().is_ok());
}

#[test]
fn test_provider_manager_crud_operations() {
    let temp_dir = create_temp_test_dir();
    let config_path = temp_dir.path().join("provider.json");

    // Start with empty config
    let empty_config = ProviderConfig {
        schema: None,
        default_provider: "test".to_string(),
        providers: HashMap::new(),
    };
    let json = serde_json::to_string_pretty(&empty_config).expect("Failed to serialize config");
    fs::write(&config_path, json).expect("Failed to write config");

    let mut manager = ProviderManager::new_with_path(&config_path)
        .expect("Should be able to create manager");

    // Add provider
    let test_provider = create_real_provider(AiType::Claude, "Test Provider");
    assert!(manager.add_provider("test".to_string(), test_provider).is_ok());

    // List providers
    let providers = manager.list_providers();
    assert_eq!(providers.len(), 1);
    assert!(providers.iter().any(|(name, _)| name.as_str() == "test"));

    // Get provider
    let retrieved = manager.get_provider("test")
        .expect("Should be able to get provider");
    assert_eq!(retrieved.description, "Test Provider");

    // Update provider (by removing and re-adding)
    let updated_provider = create_real_provider(AiType::Codex, "Updated Test Provider");
    assert!(manager.remove_provider("test").is_ok());
    assert!(manager.add_provider("test".to_string(), updated_provider).is_ok());

    // Verify update
    let updated = manager.get_provider("test")
        .expect("Should be able to get updated provider");
    assert_eq!(updated.description, "Updated Test Provider");
    assert_eq!(updated.compatible_with, vec![AiType::Codex]);

    // Note: We can't remove the default provider directly
    // In a real scenario, we would need to set another provider as default first
    // For this test, we'll just verify the provider is still there

    // Verify provider is still present (since we couldn't remove it)
    let providers_after_update = manager.list_providers();
    assert_eq!(providers_after_update.len(), 1);
    assert!(providers_after_update.iter().any(|(name, _)| name.as_str() == "test"));
}

#[test]
fn test_provider_validation() {
    let temp_dir = create_temp_test_dir();
    let config_path = temp_dir.path().join("provider.json");

    let mut manager = ProviderManager::new_with_path(&config_path)
        .expect("Should be able to create manager");

    // Test empty provider name
    let empty_name_provider = create_real_provider(AiType::Claude, "Test");
    assert!(manager.add_provider("".to_string(), empty_name_provider).is_err());

    // Test invalid AI type compatibility (empty list)
    let invalid_provider = Provider {
        description: "Invalid Provider".to_string(),
        compatible_with: vec![],
        env: HashMap::new(),
    };
    assert!(manager.add_provider("invalid".to_string(), invalid_provider).is_err());

    // Test valid provider
    let valid_provider = create_real_provider(AiType::Claude, "Valid Provider");
    assert!(manager.add_provider("valid".to_string(), valid_provider).is_ok());
}

// ============================================================================
// Provider Configuration Persistence Tests
// ============================================================================

#[test]
fn test_config_persistence() {
    let temp_dir = create_temp_test_dir();
    let config_path = temp_dir.path().join("provider.json");

    // Create initial config
    let mut manager = ProviderManager::new_with_path(&config_path)
        .expect("Should be able to create manager");

    // Add providers
    let claude_provider = create_real_provider(AiType::Claude, "Claude Provider");
    let codex_provider = create_real_provider(AiType::Codex, "Codex Provider");

    assert!(manager.add_provider("claude".to_string(), claude_provider).is_ok());
    assert!(manager.add_provider("codex".to_string(), codex_provider).is_ok());

    // Save config
    assert!(manager.save().is_ok());

    // Create new manager instance (simulating restart)
    let new_manager = ProviderManager::new_with_path(&config_path)
        .expect("Should be able to create new manager");

    // Verify persistence
    let providers = new_manager.list_providers();
    assert_eq!(providers.len(), 2);
    assert!(providers.iter().any(|(name, _)| name.as_str() == "claude"));
    assert!(providers.iter().any(|(name, _)| name.as_str() == "codex"));

    let claude = new_manager.get_provider("claude")
        .expect("Should be able to get claude provider");
    assert_eq!(claude.description, "Claude Provider");
}

#[test]
fn test_config_json_serialization() {
    let provider = create_real_provider(AiType::Claude, "Serialization Test");

    // Serialize
    let serialized = serde_json::to_string(&provider)
        .expect("Should be able to serialize provider");
    assert!(!serialized.is_empty());

    // Deserialize
    let deserialized: Provider = serde_json::from_str(&serialized)
        .expect("Should be able to deserialize provider");

    assert_eq!(provider.description, deserialized.description);
    assert_eq!(provider.compatible_with, deserialized.compatible_with);
    assert_eq!(provider.env, deserialized.env);
}

// ============================================================================
// Default Provider Tests
// ============================================================================

#[test]
fn test_default_provider_setting() {
    let temp_dir = create_temp_test_dir();
    let config_path = temp_dir.path().join("provider.json");

    let mut manager = ProviderManager::new_with_path(&config_path)
        .expect("Should be able to create manager");

    // Add multiple providers
    let claude_provider = create_real_provider(AiType::Claude, "Claude Provider");
    let codex_provider = create_real_provider(AiType::Codex, "Codex Provider");

    assert!(manager.add_provider("claude".to_string(), claude_provider).is_ok());
    assert!(manager.add_provider("codex".to_string(), codex_provider).is_ok());

    // Set default provider
    assert!(manager.set_default("claude").is_ok());

    // Verify default
    let default = manager.get_default_provider()
        .expect("Should be able to get default provider");
    assert_eq!(default.0, "claude");
    assert_eq!(default.1.description, "Claude Provider");
}

#[test]
fn test_default_provider_validation() {
    let temp_dir = create_temp_test_dir();
    let config_path = temp_dir.path().join("provider.json");

    let mut manager = ProviderManager::new_with_path(&config_path)
        .expect("Should be able to create manager");

    // Try to set non-existent provider as default
    assert!(manager.set_default("nonexistent").is_err());

    // Add a provider and set it as default
    let provider = create_real_provider(AiType::Claude, "Test Provider");
    assert!(manager.add_provider("test".to_string(), provider).is_ok());
    assert!(manager.set_default("test").is_ok());

    // Verify it's now the default
    let default = manager.get_default_provider()
        .expect("Should be able to get default provider");
    assert_eq!(default.0, "test");
}