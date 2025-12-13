use aiw::provider::config::Provider;
use std::collections::HashMap;

#[test]
fn test_provider_env_var_generation() {
    let mut env = HashMap::new();
    env.insert("CUSTOM_KEY".to_string(), "value".to_string());

    let provider = Provider {
        token: Some("sk-test-123".to_string()),
        base_url: Some("https://api.example.com".to_string()),
        scenario: None,
        env,
    };

    let env_map = provider.get_all_env_vars();
    assert_eq!(env_map.get("CUSTOM_KEY"), Some(&"value".to_string()));
    assert!(env_map.contains_key("ANTHROPIC_API_KEY"));
    assert!(env_map.contains_key("ANTHROPIC_BASE_URL"));
}
