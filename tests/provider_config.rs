use agentic_warden::provider::config::{
    AiType, ModeConfig, ModeType, Provider, ProvidersConfig, Region, RegionalConfig, SupportMode,
};
use agentic_warden::provider::env_injector::EnvInjector;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

fn read_json(path: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(path).expect("expected JSON file");
    serde_json::from_str(&raw).expect("expected valid json")
}

#[test]
fn create_default_provider_template_contains_expected_entries() {
    let config = ProvidersConfig::create_default().expect("default template should parse");
    assert_eq!(config.default_provider, "official");
    assert!(config.providers.contains_key("openrouter"));
    assert!(config.providers.contains_key("litellm"));
    assert!(config.providers.contains_key("official"));

    let openrouter = config.providers.get("openrouter").unwrap();
    assert!(openrouter.env.contains_key("OPENAI_API_KEY"));
    assert!(openrouter.compatible_with.contains(&AiType::Codex));
}

#[test]
fn load_from_missing_path_creates_default_file() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let path = temp_dir.path().join("providers.json");

    let config = ProvidersConfig::load_from_path(&path).expect("should create default config");
    assert!(path.exists(), "missing file should be created on load");
    assert_eq!(config.default_provider, "official");

    let json = read_json(&path);
    assert_eq!(json["default_provider"], "official");
}

#[test]
fn load_from_path_returns_error_for_invalid_json() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let path = temp_dir.path().join("providers.json");
    fs::write(&path, "{not-json").expect("write invalid file");

    let err = ProvidersConfig::load_from_path(&path).expect_err("invalid json should fail");
    assert!(
        err.to_string().contains("Failed to parse providers config"),
        "unexpected error: {err}"
    );
}

#[test]
fn token_round_trip_updates_storage_state() {
    let mut config = ProvidersConfig::create_default().expect("default template");
    assert!(
        !config.has_token("openrouter", &Region::International),
        "should start without tokens"
    );

    config.set_token("openrouter", Region::International, "token-123".into());
    assert!(config.has_token("openrouter", &Region::International));
    assert_eq!(
        config.get_token("openrouter", &Region::International),
        Some(&"token-123".to_string())
    );

    config
        .remove_token("openrouter", &Region::International)
        .expect("removal should succeed");
    assert!(
        !config.has_token("openrouter", &Region::International),
        "token should be removed"
    );
}

#[test]
fn select_best_mode_prefers_highest_priority_matching_ai_type() {
    let mut regional_urls = HashMap::new();
    regional_urls.insert(
        Region::International,
        RegionalConfig {
            base_url: "https://example.com".into(),
            auth_env_var: "API_KEY".into(),
            recommended_model: None,
            features: None,
        },
    );

    let config = ProvidersConfig {
        schema: None,
        providers: HashMap::from([(
            "demo".to_string(),
            Provider {
                name: "Demo".into(),
                description: "demo provider".into(),
                icon: None,
                official: false,
                protected: false,
                custom: true,
                support_modes: vec![
                    SupportMode {
                        mode_type: ModeType::OpenAICompatible,
                        name: "generic".into(),
                        description: "generic support".into(),
                        priority: 10,
                        config: ModeConfig {
                            regional_urls: regional_urls.clone(),
                            models: None,
                            additional_env: None,
                            rate_limit: None,
                        },
                    },
                    SupportMode {
                        mode_type: ModeType::ClaudeCodeNative,
                        name: "native".into(),
                        description: "native claude support".into(),
                        priority: 42,
                        config: ModeConfig {
                            regional_urls: regional_urls.clone(),
                            models: None,
                            additional_env: None,
                            rate_limit: None,
                        },
                    },
                ],
                compatible_with: vec![AiType::Codex, AiType::Claude],
                validation_endpoint: None,
                category: None,
                website: None,
                regions: vec!["international".into()],
                env: HashMap::from([("API_KEY".into(), "value".into())]),
            },
        )]),
        default_provider: "demo".into(),
        user_tokens: HashMap::new(),
        memory: None,
    };

    let best = config
        .select_best_mode("demo", &AiType::Claude)
        .expect("mode should exist");
    assert_eq!(best.name, "native");

    let compatible: Vec<_> = config
        .get_compatible_providers(&AiType::Codex)
        .into_iter()
        .map(|(id, _)| id.clone())
        .collect();
    assert_eq!(compatible, vec!["demo"]);
}

#[test]
fn env_injector_sets_environment_and_masks_sensitive_values() {
    let mut cmd = Command::new("echo");
    let env_map = HashMap::from([
        ("OPENAI_API_KEY".to_string(), "sk-test-token".to_string()),
        ("CUSTOM_ENV".to_string(), "value".to_string()),
    ]);

    EnvInjector::inject_to_command(&mut cmd, &env_map);

    let injected: HashMap<String, String> = cmd
        .get_envs()
        .filter_map(|(key, value)| {
            value.map(|val| {
                (
                    key.to_string_lossy().to_string(),
                    val.to_string_lossy().to_string(),
                )
            })
        })
        .collect();

    assert_eq!(injected.get("CUSTOM_ENV"), Some(&"value".to_string()));
    assert_eq!(
        injected.get("OPENAI_API_KEY"),
        Some(&"sk-test-token".to_string())
    );

    let masked = EnvInjector::mask_sensitive_value("OPENAI_API_KEY", "sk-test-token");
    assert_eq!(masked, "sk-t***oken");
    assert_eq!(EnvInjector::mask_api_key("short"), "***");
}
