use aiw::provider::config::Provider;
use std::collections::HashMap;

#[test]
fn test_provider_env_var_generation() {
    let mut env = HashMap::new();
    env.insert("CUSTOM_KEY".to_string(), "value".to_string());
    env.insert("ANTHROPIC_API_KEY".to_string(), "sk-test-123".to_string());
    env.insert("ANTHROPIC_BASE_URL".to_string(), "https://api.example.com".to_string());

    let provider = Provider {
        enabled: true,
        scenario: None,
        compatible_with: None,
        env,
        disabled_until: None,
    };

    assert_eq!(provider.env.get("CUSTOM_KEY"), Some(&"value".to_string()));
    assert!(provider.env.contains_key("ANTHROPIC_API_KEY"));
    assert!(provider.env.contains_key("ANTHROPIC_BASE_URL"));
}
