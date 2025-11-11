use agentic_warden::commands::parser::Cli;
use agentic_warden::commands::{parse_external_as_ai_cli, Commands};

fn parse(args: &[&str]) -> Commands {
    let argv: Vec<String> = std::iter::once("agentic-warden")
        .chain(args.iter().copied())
        .map(|s| s.to_string())
        .collect();
    Cli::try_parse_command_from(argv).expect("expected command parsing to succeed")
}

#[test]
fn defaults_to_dashboard_when_no_subcommand_given() {
    let command = parse(&[]);
    assert!(matches!(command, Commands::Dashboard));
}

#[test]
fn parses_status_and_provider_commands() {
    match parse(&["status"]) {
        Commands::Status { tui: false } => {}
        other => panic!("expected status command, got {other:?}"),
    }

    match parse(&["status", "--tui"]) {
        Commands::Status { tui: true } => {}
        other => panic!("expected status --tui command, got {other:?}"),
    }

    match parse(&["provider"]) {
        Commands::Provider => {}
        other => panic!("expected provider command, got {other:?}"),
    }
}

#[test]
fn parses_push_with_multiple_directories() {
    let command = parse(&["push", "src", "config", "C:\\tmp\\notes"]);

    let dirs = match command {
        Commands::Push { dirs } => dirs,
        other => panic!("expected push command, got {other:?}"),
    };

    let as_strings: Vec<String> = dirs
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    assert_eq!(as_strings, vec!["src", "config", r"C:\tmp\notes"]);
}

#[test]
fn parses_help_with_optional_topic() {
    let command = parse(&["help", "push"]);
    match command {
        Commands::Help {
            command: Some(topic),
        } => assert_eq!(topic, "push"),
        other => panic!("expected help with topic, got {other:?}"),
    }

    let command = parse(&["help"]);
    match command {
        Commands::Help { command: None } => {}
        other => panic!("expected help without topic, got {other:?}"),
    }
}

#[test]
fn captures_external_subcommands_for_ai_cli() {
    let command = parse(&["claude", "-p", "openrouter", "write", "tests"]);
    let args = match command {
        Commands::External(tokens) => tokens,
        other => panic!("expected external command, got {other:?}"),
    };
    assert_eq!(args, vec!["claude", "-p", "openrouter", "write", "tests"]);
}

#[test]
fn parse_external_ai_cli_arguments() {
    let tokens = vec![
        "claude".to_string(),
        "-p".to_string(),
        "openrouter".to_string(),
        "implement".to_string(),
        "feature".to_string(),
    ];

    let args = parse_external_as_ai_cli(&tokens).expect("external command should parse");
    assert_eq!(args.selector, "claude");
    assert_eq!(args.provider.as_deref(), Some("openrouter"));
    assert_eq!(
        args.prompt,
        vec!["implement".to_string(), "feature".to_string()]
    );
    assert_eq!(args.prompt_text(), "implement feature");
}

#[test]
fn fail_when_provider_flag_missing_value() {
    let tokens = vec!["claude".to_string(), "-p".to_string()];
    let err = parse_external_as_ai_cli(&tokens).expect_err("value is required");
    assert!(err.contains("requires a value"));
}

#[test]
fn returns_error_when_no_external_tokens_are_provided() {
    let err = parse_external_as_ai_cli(&[]).expect_err("no tokens should be rejected");
    assert!(err.contains("No command provided"));
}

#[test]
fn invalid_flag_is_treated_as_prompt_token() {
    // clap handles validation of unknown flags for declared subcommands.
    // For external commands we accept anything as part of the prompt.
    let tokens = vec![
        "claude".to_string(),
        "--style".to_string(),
        "concise".to_string(),
    ];
    let args = parse_external_as_ai_cli(&tokens).expect("external parser should accept flags");
    assert_eq!(args.selector, "claude");
    assert_eq!(
        args.prompt,
        vec!["--style".to_string(), "concise".to_string()]
    );
    assert!(args.provider.is_none());
}

#[test]
fn try_parse_fails_for_unknown_top_level_flag() {
    let result = Cli::try_parse_command_from(["agentic-warden", "--unknown"]);
    assert!(result.is_err(), "unknown flag should produce clap error");
}
