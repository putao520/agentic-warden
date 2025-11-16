use agentic_warden::commands::parser::Cli;
use agentic_warden::commands::{parse_external_as_ai_cli, Commands, RolesAction};

fn parse(args: &[&str]) -> Commands {
    let argv: Vec<String> = std::iter::once("agentic-warden")
        .chain(args.iter().copied())
        .map(|s| s.to_string())
        .collect();
    Cli::try_parse_command_from(argv).expect("expected command parsing to succeed")
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
fn parses_roles_list_command() {
    match parse(&["roles", "list"]) {
        Commands::Roles(RolesAction::List) => {}
        other => panic!("expected roles list command, got {other:?}"),
    }
}

#[test]
fn parses_update_command_with_no_tool() {
    match parse(&["update"]) {
        Commands::Update { tool: None } => {}
        other => panic!("expected update command with no tool, got {other:?}"),
    }
}
