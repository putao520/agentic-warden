use agentic_warden::provider::config::{Provider, ProvidersConfig};
use std::collections::HashMap;

#[test]
fn test_create_default_provider_template() {
    let config = ProvidersConfig::create_default();
    assert_eq!(config.default_provider, "official");
    assert!(config.providers.contains_key("official"));

    let official = config.providers.get("official").unwrap();
    assert!(official.env.is_empty());
    assert!(official.token.is_none());
}

#[test]
fn test_provider_env_var_generation() {
    let mut env = HashMap::new();
    env.insert("CUSTOM_KEY".to_string(), "value".to_string());

    let provider = Provider {
        token: Some("sk-test-123".to_string()),
        base_url: Some("https://api.example.com".to_string()),
        env,
    };

    let env_map = provider.get_all_env_vars();
    assert_eq!(env_map.get("CUSTOM_KEY"), Some(&"value".to_string()));
    assert!(env_map.contains_key("ANTHROPIC_API_KEY"));
    assert!(env_map.contains_key("ANTHROPIC_BASE_URL"));
}

#[test]
fn test_provider_summary_includes_flags() {
    let provider = Provider {
        token: Some("sk-test-123".to_string()),
        base_url: Some("https://api.example.com".to_string()),
        env: HashMap::from([
            ("EXTRA".to_string(), "1".to_string()),
            ("ANOTHER".to_string(), "2".to_string()),
        ]),
    };

    let summary = provider.summary();
    assert!(summary.contains("token"));
    assert!(summary.contains("url"));
    assert!(summary.contains("env"));
}
