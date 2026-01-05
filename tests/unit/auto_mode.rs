use aiw::auto_mode::config::ExecutionOrderConfig;
use aiw::cli_type::CliType;
use aiw::cli_type::parse_cli_type;
use aiw::error::ConfigError;

#[test]
fn validates_default_execution_order() {
    let order = vec![
        "codex".to_string(),
        "gemini".to_string(),
        "claude".to_string(),
    ];

    assert!(ExecutionOrderConfig::validate_order(&order).is_ok());
}

#[test]
fn rejects_invalid_length() {
    let order = vec!["codex".to_string(), "claude".to_string()];
    let err = ExecutionOrderConfig::validate_order(&order).expect_err("should reject length");
    match err {
        ConfigError::InvalidLength { expected, actual } => {
            assert_eq!(expected, 3);
            assert_eq!(actual, 2);
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn rejects_invalid_cli_type() {
    let order = vec![
        "codex".to_string(),
        "gemini".to_string(),
        "dragon".to_string(),
    ];
    let err = ExecutionOrderConfig::validate_order(&order).expect_err("should reject value");
    match err {
        ConfigError::InvalidCliType { value } => assert_eq!(value, "dragon"),
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn rejects_duplicate_cli_types() {
    let order = vec![
        "codex".to_string(),
        "gemini".to_string(),
        "gemini".to_string(),
    ];
    let err = ExecutionOrderConfig::validate_order(&order).expect_err("should reject duplicates");
    assert!(matches!(err, ConfigError::DuplicateCliType));
}

#[test]
fn reset_to_default_order_matches_spec() {
    let order = ExecutionOrderConfig::reset_to_default();
    assert_eq!(order, vec![CliType::Codex, CliType::Gemini, CliType::Claude]);
}

#[test]
fn parses_auto_cli_type() {
    assert_eq!(parse_cli_type("auto"), Some(CliType::Auto));
}
