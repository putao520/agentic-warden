use aiw::commands::parser::{Cli, McpAction};
use aiw::commands::{parse_external_as_ai_cli, Commands, RolesAction};

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
    assert!(args.cli_args.is_empty());
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
fn parses_update_command() {
    match parse(&["update"]) {
        Commands::Update => {}
        other => panic!("expected update command with no tool, got {other:?}"),
    }
}

#[test]
fn captures_unknown_sync_commands_as_external() {
    let push = parse(&["push"]);
    match push {
        Commands::External(tokens) => assert_eq!(tokens, vec!["push"]),
        other => panic!("expected external command for push, got {other:?}"),
    }

    let pull = parse(&["pull"]);
    match pull {
        Commands::External(tokens) => assert_eq!(tokens, vec!["pull"]),
        other => panic!("expected external command for pull, got {other:?}"),
    }
}

#[test]
fn parses_wait_command() {
    match parse(&["wait"]) {
        Commands::Wait => {}
        other => panic!("expected wait command, got {other:?}"),
    }
}

#[test]
fn parses_mcp_search_command() {
    match parse(&["mcp", "search", "filesystem", "--source", "registry", "--limit", "5"]) {
        Commands::Mcp(McpAction::Search {
            query,
            source,
            limit,
        }) => {
            assert_eq!(query, "filesystem");
            assert_eq!(source.as_deref(), Some("registry"));
            assert_eq!(limit, Some(5));
        }
        other => panic!("expected mcp search command, got {other:?}"),
    }
}

#[test]
fn parses_mcp_install_command() {
    match parse(&[
        "mcp",
        "install",
        "@anthropic/filesystem",
        "--env",
        "TOKEN=abc",
        "--skip-env",
    ]) {
        Commands::Mcp(McpAction::Install {
            name,
            source,
            env_vars,
            skip_env,
        }) => {
            assert_eq!(name, "@anthropic/filesystem");
            assert!(source.is_none());
            assert_eq!(env_vars, vec!["TOKEN=abc".to_string()]);
            assert!(skip_env);
        }
        other => panic!("expected mcp install command, got {other:?}"),
    }
}

#[test]
fn parses_mcp_info_command() {
    match parse(&["mcp", "info", "filesystem", "--source", "registry"]) {
        Commands::Mcp(McpAction::Info { name, source }) => {
            assert_eq!(name, "filesystem");
            assert_eq!(source.as_deref(), Some("registry"));
        }
        other => panic!("expected mcp info command, got {other:?}"),
    }
}

#[test]
fn parses_mcp_update_command() {
    match parse(&["mcp", "update"]) {
        Commands::Mcp(McpAction::Update) => {}
        other => panic!("expected mcp update command, got {other:?}"),
    }
}
