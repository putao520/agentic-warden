use aiw::commands::parser::{separate_provider_and_cli_args, SeparatedArgs};

#[test]
fn test_parse_role_parameter() {
    let tokens = vec
!["-r".to_string(), "common".to_string(), "-p".to_string(), "glm".to_string(), "test prompt".to_string()];
    let result = separate_provider_and_cli_args(&tokens).unwrap();
    assert_eq!(result.role, Some("common".to_string()));
    assert_eq!(result.provider, Some("glm".to_string()));
    assert_eq!(result.prompt, vec!["test prompt".to_string()]);
}

#[test]
fn test_role_parameter_order_error() {
    // -r must come before -p
    let tokens = vec
!["-p".to_string(), "glm".to_string(), "-r".to_string(), "common".to_string(), "test".to_string()];
    let result = separate_provider_and_cli_args(&tokens);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("-r/--role must be specified before -p/--provider"));
}

#[test]
fn test_role_only() {
    let tokens = vec
!["-r".to_string(), "debugger".to_string(), "debug this".to_string()];
    let result = separate_provider_and_cli_args(&tokens).unwrap();
    assert_eq!(result.role, Some("debugger".to_string()));
    assert_eq!(result.provider, None);
    assert_eq!(result.prompt, vec!["debug this".to_string()]);
}

#[test]
fn test_role_with_cli_params() {
    let tokens = vec![
        "-r".to_string(),
        "security".to_string(),
        "-p".to_string(),
        "glm".to_string(),
        "--model".to_string(),
        "sonnet".to_string(),
        "review code".to_string(),
    ];
    let result = separate_provider_and_cli_args(&tokens).unwrap();
    assert_eq!(result.role, Some("security".to_string()));
    assert_eq!(result.provider, Some("glm".to_string()));
    assert_eq!(result.cli_args, vec!["--model", "sonnet"]);
    assert_eq!(result.prompt, vec!["review code".to_string()]);
}
