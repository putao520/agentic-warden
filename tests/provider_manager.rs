use agentic_warden::provider::config::{
    AiType, ModeConfig, ModeType, Provider, ProvidersConfig, Region, RegionalConfig, SupportMode,
};
use agentic_warden::provider::manager::ProviderManager;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn sample_provider() -> Provider {
    let mut regional_urls = HashMap::new();
    regional_urls.insert(
        Region::International,
        RegionalConfig {
            base_url: "https://api.example.com".to_string(),
            auth_env_var: "OPENAI_API_KEY".to_string(),
            recommended_model: Some("gpt-4o-mini".to_string()),
            features: None,
        },
    );

    let mode = SupportMode {
        mode_type: ModeType::OpenAICompatible,
        name: "Default".to_string(),
        description: "OpenAI compatible endpoint".to_string(),
        priority: 1,
        config: ModeConfig {
            regional_urls,
            models: None,
            additional_env: None,
            rate_limit: None,
        },
    };

    let mut env = HashMap::new();
    env.insert("OPENAI_API_KEY".to_string(), "test-key".to_string());

    Provider {
        name: "Custom Provider".to_string(),
        description: "Test provider used for integration verification".to_string(),
        icon: None,
        official: false,
        protected: false,
        custom: true,
        support_modes: vec![mode],
        compatible_with: vec![AiType::Codex],
        validation_endpoint: None,
        category: None,
        website: None,
        regions: vec!["global".to_string()],
        env,
    }
}

#[test]
fn provider_manager_roundtrip_updates() {
    let temp_dir = TempDir::new().expect("temp dir");
    let config_path = temp_dir.path().join("providers.json");

    let config = ProvidersConfig::create_default().expect("default config");
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&config).expect("serialize config"),
    )
    .expect("write config");

    let mut manager = ProviderManager::new_with_path(&config_path).expect("manager");
    manager
        .add_provider("custom".into(), sample_provider())
        .expect("add provider");
    manager.set_default("custom").expect("set default provider");
    manager
        .set_token("custom", Region::International, "secret-token".into())
        .expect("set token");

    manager.save().expect("save config");

    let manager = ProviderManager::new_with_path(&config_path).expect("reload manager");
    let provider = manager.get_provider("custom").expect("provider exists");
    assert_eq!(provider.name, "Custom Provider");
    assert_eq!(manager.default_provider_name(), "custom");
    assert!(manager.has_token("custom", &Region::International));

    // Validate compatibility check executes without error.
    manager
        .validate_compatibility("custom", AiType::Codex)
        .expect("compatibility");
}
