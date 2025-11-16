//! Provider Injection E2E Tests
//! Tests REQ-002: 第三方Provider管理 (环境变量注入)

mod common;

use agentic_warden::provider::config::{Provider, ProvidersConfig};
use anyhow::{anyhow, Result};
use common::{get_process_env_vars, set_process_env, spawn_ai_cli};
use serial_test::serial;
use std::collections::HashMap;

#[tokio::test]
#[serial]
async fn test_provider_env_injection_to_ai_cli() -> Result<()> {
    let config = ProvidersConfig {
        schema: None,
        providers: HashMap::from([(
            "openrouter".into(),
            Provider {
                token: Some("sk-test-123".into()),
                base_url: Some("https://openrouter.ai/api/v1".into()),
                scenario: None,
                env: HashMap::from([("CUSTOM_KEY".into(), "custom_value".into())]),
            },
        )]),
        default_provider: "openrouter".into(),
        memory: None,
    };

    let provider = config
        .providers
        .get("openrouter")
        .expect("provider exists");
    let env_vars = provider.get_all_env_vars();

    let task = spawn_ai_cli("codex", "test task").await?;
    set_process_env(task.pid, env_vars.clone());

    let process_env = get_process_env_vars(task.pid as i32).await;
    assert_eq!(
        process_env.get("ANTHROPIC_API_KEY"),
        env_vars.get("ANTHROPIC_API_KEY")
    );
    assert_eq!(
        process_env.get("ANTHROPIC_BASE_URL"),
        env_vars.get("ANTHROPIC_BASE_URL")
    );
    assert_eq!(
        process_env.get("CUSTOM_KEY"),
        env_vars.get("CUSTOM_KEY")
    );

    Ok(())
}

fn validate_provider_compatibility(ai_type: &str, provider: &Provider) -> Result<()> {
    if let Some(scenario) = &provider.scenario {
        if !scenario
            .to_lowercase()
            .contains(&ai_type.to_lowercase())
        {
            return Err(anyhow!("provider incompatible with {ai_type}"));
        }
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_provider_compatibility_validation() -> Result<()> {
    let incompatible = Provider {
        token: Some("invalid".into()),
        base_url: None,
        scenario: Some("Only for claude".into()),
        env: HashMap::new(),
    };

    let result = validate_provider_compatibility("codex", &incompatible);
    assert!(result.is_err());

    let compatible = Provider {
        token: Some("valid".into()),
        base_url: None,
        scenario: Some("Works with codex and claude".into()),
        env: HashMap::new(),
    };
    validate_provider_compatibility("codex", &compatible)?;

    Ok(())
}
