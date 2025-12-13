use aiw::commands::parse_external_as_ai_cli;
use aiw::commands::parser::{separate_provider_and_cli_args, SeparatedArgs};

#[test]
fn separates_provider_cli_args_and_prompt() {
    let tokens = vec![
        "-p".to_string(),
        "glm".to_string(),
        "--model".to_string(),
        "sonnet".to_string(),
        "--debug".to_string(),
        "api".to_string(),
        "Explain".to_string(),
        "this".to_string(),
        "code".to_string(),
    ];

    let SeparatedArgs {
        provider,
        cli_args,
        prompt,
    } = separate_provider_and_cli_args(&tokens).expect("arguments should parse");

    assert_eq!(provider.as_deref(), Some("glm"));
    assert_eq!(
        cli_args,
        vec![
            "--model".to_string(),
            "sonnet".to_string(),
            "--debug".to_string(),
            "api".to_string()
        ]
    );
    assert_eq!(
        prompt,
        vec!["Explain".to_string(), "this".to_string(), "code".to_string()]
    );
}

#[test]
fn supports_interactive_forwarding_without_prompt() {
    let tokens = vec![
        "-p".to_string(),
        "glm".to_string(),
        "--print".to_string(),
        "--output-format".to_string(),
        "json".to_string(),
    ];

    let parsed = separate_provider_and_cli_args(&tokens).expect("arguments should parse");
    assert_eq!(parsed.provider.as_deref(), Some("glm"));
    assert_eq!(
        parsed.cli_args,
        vec![
            "--print".to_string(),
            "--output-format".to_string(),
            "json".to_string()
        ]
    );
    assert!(parsed.prompt.is_empty());
}

#[test]
fn errors_when_provider_after_cli_args() {
    let tokens = vec![
        "--model".to_string(),
        "sonnet".to_string(),
        "-p".to_string(),
        "glm".to_string(),
        "prompt".to_string(),
    ];

    let err = separate_provider_and_cli_args(&tokens)
        .expect_err("provider after cli args should error");

    assert!(
        err.starts_with("Error: -p/--provider must be specified before other CLI parameters."),
        "unexpected error message: {err}"
    );
}

#[test]
fn parses_external_command_with_forwarded_args() {
    let tokens = vec![
        "claude".to_string(),
        "-p".to_string(),
        "glm".to_string(),
        "--model".to_string(),
        "sonnet".to_string(),
        "summarize".to_string(),
        "text".to_string(),
    ];

    let args = parse_external_as_ai_cli(&tokens).expect("external command should parse");
    assert_eq!(args.selector, "claude");
    assert_eq!(args.provider.as_deref(), Some("glm"));
    assert_eq!(
        args.cli_args,
        vec!["--model".to_string(), "sonnet".to_string()]
    );
    assert_eq!(args.prompt_text(), "summarize text");
}

#[test]
fn keeps_prompt_after_valueless_flag() {
    let tokens = vec![
        "-p".to_string(),
        "glm".to_string(),
        "--print".to_string(),
        "test".to_string(),
    ];

    let parsed = separate_provider_and_cli_args(&tokens).expect("arguments should parse");
    assert_eq!(parsed.provider.as_deref(), Some("glm"));
    assert_eq!(parsed.cli_args, vec!["--print".to_string()]);
    assert_eq!(parsed.prompt, vec!["test".to_string()]);
}

#[test]
fn supports_prompt_without_provider_and_valueless_flag() {
    let tokens = vec!["--print".to_string(), "hello".to_string()];

    let parsed = separate_provider_and_cli_args(&tokens).expect("arguments should parse");
    assert_eq!(parsed.provider, None);
    assert_eq!(parsed.cli_args, vec!["--print".to_string()]);
    assert_eq!(parsed.prompt_text(), "hello");
}
