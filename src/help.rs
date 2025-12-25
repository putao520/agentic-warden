//! Help module for aiw
//!
//! Provides comprehensive help information for all commands and features

use std::io::{self, Write};

/// Print general help information
pub fn print_general_help() -> io::Result<()> {
    let help_text = r#"
aiw v0.5.25 - Universal AI CLI Management Platform

USAGE:
    aiw [OPTIONS] <COMMAND>
    aiw [OPTIONS] <AI_CLI> [AI_OPTIONS] "<TASK>"

AI CLI SELECTORS:
    claude | codex | gemini    Single AI agent
    all                         All available agents
    "agent1|agent2"             Multiple agents (quotes required)

MAIN COMMANDS:
    dashboard                   Show Dashboard (default when no args)
    status [--tui]              Show task status
    provider                    Launch Provider Management TUI
    wait                        Wait for all AI CLI tasks to complete
    pwait <PID>                 Wait for specific process tasks
    examples / demo             Show usage examples
    help [COMMAND]              Show help for command
    update                      Update AIW and AI CLI tools
    v                           Show version information

MCP COMMANDS:
    mcp list                    List all MCP servers
    mcp add <name> <cmd> [ARGS]  Add MCP server
    mcp remove <name>           Remove MCP server
    mcp search <query>          Search MCP servers
    mcp install <name>          Install MCP server from registry
    mcp info <name>             Show MCP server details
    mcp update                  Update MCP registry cache
    mcp browse                  Interactive MCP server browser
    mcp get <name>              Get server configuration
    mcp enable <name>           Enable MCP server
    mcp disable <name>          Disable MCP server
    mcp edit                    Edit MCP configuration file
    mcp serve                   Start MCP server (internal use)

ROLE COMMANDS:
    roles list                  List all available role configurations

OPTIONS:
    --help, -h                  Show this help message
    --version, -V               Show version information

EXAMPLES:
    # AI CLI with provider selection
    aiw claude "explain this code"
    aiw claude -p openrouter "explain this code"
    aiw codex -p glm "write tests"

    # AI CLI with parameter forwarding
    aiw claude -p glm --model sonnet --debug api "explain this"
    aiw claude -p glm --print --output-format json "summarize"

    # Multiple AI agents
    aiw all "review this code"
    aiw "claude|gemini" "compare approaches"

    # MCP management
    aiw mcp browse
    aiw mcp install @anthropic/filesystem
    aiw mcp list

    # Task monitoring
    aiw wait
    aiw status

For more detailed information about a specific command:
    aiw help <command>
    aiw help claude
    aiw help mcp

Project home: https://github.com/putao520/agentic-warden
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for specific commands
pub fn print_command_help(command: &str) -> io::Result<()> {
    match command.to_lowercase().as_str() {
        "claude" | "codex" | "gemini" => print_ai_cli_help(command),
        "all" => print_all_agents_help(),
        "wait" => print_wait_help(),
        "pwait" => print_pwait_help(),
        "status" => print_status_help(),
        "provider" => print_provider_help(),
        "dashboard" => print_dashboard_help(),
        "examples" | "demo" => print_examples_help(),
        "update" => print_update_help(),
        "mcp" => print_mcp_help(),
        "roles" => print_roles_help(),
        "version" | "v" => print_version(),
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Use 'aiw help' for general help");
            Ok(())
        }
    }
}

/// Print help for AI CLI commands (claude, codex, gemini)
fn print_ai_cli_help(agent: &str) -> io::Result<()> {
    let help_text = format!(
        r#"
{} AGENT

USAGE:
    aiw {} [-p PROVIDER] [CLI_OPTIONS] ["<TASK>"]
    aiw {} [-p PROVIDER] [CLI_OPTIONS]

DESCRIPTION:
    Run the {} AI agent with provider management and transparent parameter forwarding.

    Agent selector can be: claude, codex, or gemini

PROVIDER SELECTION:
    -p, --provider <PROVIDER>    Use specific provider (e.g., openrouter, glm)

    Without -p, uses the default provider configured in ~/.aiw/providers.json

TRANSPARENT PARAMETER FORWARDING:
    All CLI parameters after the provider selection are forwarded directly to {}.

    Common parameters:
        --model <name>           Select model (e.g., sonnet, opus, gpt-4)
        --debug <level>          Enable debugging (api, all)
        --print                  Print mode (skip interactive features)
        --output-format <fmt>    Output format (json, stream-json)
        --temperature <n>        Sampling temperature
        --max-tokens <n>         Maximum tokens to generate
        --allowed-tools <list>   Restrict available tools
        --no-session-persistence Disable session persistence

    Parameter order rule: -p/--provider must come BEFORE other CLI parameters

INTERACTIVE MODE:
    aiw {} [-p PROVIDER] [CLI_OPTIONS]

    Start {} in interactive mode (no task specified).
    Useful for extended conversations with the AI.

TASK MODE:
    aiw {} [-p PROVIDER] [CLI_OPTIONS] "your task here"

    Run a single task and exit.

EXAMPLES:

    # Basic usage (default provider)
    aiw {} "write a Rust function"
    aiw {} "explain this code"

    # With provider selection
    aiw {} -p openrouter "write python code"
    aiw {} -p glm "write tests"

    # With parameter forwarding
    aiw {} -p glm --model sonnet --debug api "explain this"
    aiw {} -p glm --print --output-format json "summarize file"
    aiw {} -p glm --temperature 0.3 --max-tokens 500 "translate text"

    # Interactive mode with custom settings
    aiw {} -p glm --model sonnet --debug api

    # Multi-agent selection
    aiw all "review this code"
    aiw "claude|gemini" "compare implementations"

CONFIGURATION:
    Providers: ~/.aiw/providers.json
    MCP servers: ~/.aiw/mcp.json

For more information on MCP integration:
    aiw help mcp
"#,
        agent.to_uppercase(),
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent,
        agent
    );
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for "all" agent selector
fn print_all_agents_help() -> io::Result<()> {
    let help_text = r#"
ALL AGENTS

USAGE:
    aiw all [-p PROVIDER] [CLI_OPTIONS] "<TASK>"

DESCRIPTION:
    Send the same task to all available AI agents (claude, codex, gemini).

EXAMPLES:
    aiw all "review this code and suggest improvements"
    aiw all "explain this algorithm in detail"
    aiw all -p glm "write comprehensive documentation"

Each agent will process the task independently and provide their
unique perspective and approach to the solution.
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for wait command
fn print_wait_help() -> io::Result<()> {
    let help_text = r#"
WAIT COMMAND

USAGE:
    aiw wait

DESCRIPTION:
    Enter monitoring mode to track all AI CLI task completion across processes.

FEATURES:
    - Shows active tasks and their progress
    - Displays completion status for finished tasks
    - Provides cleanup options for completed tasks
    - Monitors shared memory for task registry updates

    Use this when you've launched multiple AI CLI tasks in parallel
    and want to wait for all of them to complete.

EXAMPLES:
    # Terminal 1: Start tasks in background
    aiw claude "task 1" &
    aiw codex "task 2" &
    aiw gemini "task 3" &

    # Terminal 2: Monitor all tasks
    aiw wait
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for pwait command
fn print_pwait_help() -> io::Result<()> {
    let help_text = r#"
PWAIT COMMAND

USAGE:
    aiw pwait <PID>

DESCRIPTION:
    Wait for tasks spawned by a specific process (by PID).

    Unlike 'wait' which monitors all tasks, 'pwait' only monitors
    tasks associated with the specified process ID.

ARGUMENTS:
    <PID>    Process ID to monitor

EXAMPLES:
    # Get PID of a running aiw process
    ps aux | grep "aiw claude"

    # Wait for that process's tasks
    aiw pwait 12345
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for status command
fn print_status_help() -> io::Result<()> {
    let help_text = r#"
STATUS COMMAND

USAGE:
    aiw status [--tui]

DESCRIPTION:
    Display task status and system information.

OPTIONS:
    --tui    Launch TUI interface for detailed status view

TEXT OUTPUT (default):
    - Active AI CLI tasks
    - Task completion status
    - Provider configuration summary
    - MCP server status

TUI OUTPUT (--tui):
    - Interactive task monitoring
    - Real-time status updates
    - Detailed task information

EXAMPLES:
    aiw status
    aiw status --tui
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for provider command
fn print_provider_help() -> io::Result<()> {
    let help_text = r#"
PROVIDER COMMAND

USAGE:
    aiw provider

DESCRIPTION:
    Launch the TUI Provider Management interface.

    This is a shortcut command that directly opens the Provider Management
    screen in the TUI. You can also access it from the Dashboard by pressing 'P'.

FEATURES:
    - List all configured providers
    - Add/edit/remove providers
    - Set default provider
    - Configure provider-specific settings

    Providers are stored in: ~/.aiw/providers.json

TUI CONTROLS:
    ? / h      Show help
    q / Esc    Exit
    ↑ / ↓      Navigate
    Enter      Select/Edit
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for dashboard command
fn print_dashboard_help() -> io::Result<()> {
    let help_text = r#"
DASHBOARD COMMAND

USAGE:
    aiw dashboard
    aiw

DESCRIPTION:
    Launch the AIW TUI Dashboard (default when no command specified).

    The Dashboard provides a comprehensive view of:
    - Active AI CLI tasks
    - Provider status
    - MCP server configuration
    - System status

TUI CONTROLS:
    ? / h      Show help
    q / Esc    Exit
    P          Open Provider Management
    S          Open Status Screen
    R          Refresh

EXAMPLES:
    aiw              # Launch dashboard
    aiw dashboard    # Same as above
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for examples command
fn print_examples_help() -> io::Result<()> {
    let help_text = r#"
EXAMPLES COMMAND

USAGE:
    aiw examples
    aiw demo

DESCRIPTION:
    Display usage examples for common AIW workflows.

EXAMPLES:

    AI CLI Basics:
        aiw claude "write a hello world in Rust"
        aiw codex -p glm "explain this function"
        aiw gemini "refactor this code"

    Provider Management:
        aiw claude -p openrouter "write tests"
        aiw codex -p glm --temperature 0.7 "generate code"

    Parameter Forwarding:
        aiw claude -p glm --model sonnet --debug api "explain"
        aiw claude -p glm --print --output-format json "summarize"
        aiw claude -p glm --allowed-tools Bash,Edit "modify file"

    Multi-Agent:
        aiw all "review this PR"
        aiw "claude|gemini" "compare approaches"

    MCP Management:
        aiw mcp browse
        aiw mcp install @anthropic/filesystem
        aiw mcp list
        aiw mcp enable brave-search

    Task Monitoring:
        aiw wait
        aiw status
        aiw status --tui

For detailed help on specific commands:
    aiw help <command>
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for update command
fn print_update_help() -> io::Result<()> {
    let help_text = r#"
UPDATE COMMAND

USAGE:
    aiw update

DESCRIPTION:
    Update AIW itself and all installed AI CLI tools to latest versions.

    This command will:
    1. Check AIW version from NPM registry
    2. Update AIW if newer version available
    3. Check for updates to installed AI CLI tools
    4. Update AI CLI tools if newer versions available
    5. Display detailed results for all updates

EXAMPLES:
    aiw update

WHAT GETS UPDATED:
    - AIW (from @putao520/aiw NPM package)
    - claude (Anthropic CLI)
    - codex (OpenAI CLI)
    - gemini (Google CLI)
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for mcp command
fn print_mcp_help() -> io::Result<()> {
    let help_text = r#"
MCP COMMANDS

USAGE:
    aiw mcp <SUBCOMMAND> [ARGS]

MCP SERVER MANAGEMENT:
    list                        List all configured MCP servers
    add <name> <cmd> [ARGS]      Add a new MCP server
    remove <name>               Remove an MCP server
    enable <name>               Enable a disabled server
    disable <name>              Disable an enabled server
    edit                        Edit MCP configuration in editor
    get <name>                  Show server configuration

MCP REGISTRY:
    search <query>              Search MCP registries for servers
    install <name>              Install server from registry
    info <name>                 Show detailed server information
    update                      Update registry cache
    browse                      Interactive server browser

INTERNAL:
    serve                       Start MCP server (for Claude Code)

EXAMPLES:

    # Server management
    aiw mcp list
    aiw mcp add filesystem npx -- -y @modelcontextprotocol/server-filesystem $HOME
    aiw mcp enable filesystem
    aiw mcp disable filesystem
    aiw mcp remove filesystem

    # Registry operations
    aiw mcp browse
    aiw mcp search "filesystem"
    aiw mcp install @anthropic/filesystem
    aiw mcp info @anthropic/filesystem
    aiw mcp update

    # Configuration
    aiw mcp get filesystem
    aiw mcp edit

MCP CONFIGURATION:
    Servers are stored in: ~/.aiw/mcp.json
    This file is compatible with Claude Code's mcpServers configuration

REGISTRIES:
    - Official MCP Registry (registry.modelcontextprotocol.io)
    - Smithery (registry.smithery.ai)

For information on using AIW as an MCP server:
    See project documentation at https://github.com/putao520/agentic-warden
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for roles command
fn print_roles_help() -> io::Result<()> {
    let help_text = r#"
ROLES COMMAND

USAGE:
    aiw roles <SUBCOMMAND>

SUBCOMMANDS:
    list    List all available role configurations

DESCRIPTION:
    Manage AI CLI role configurations.

    Roles are markdown files stored in ~/.aiw/role/ that define
    custom system prompts and behaviors for AI CLI tools.

EXAMPLES:
    aiw roles list

ROLE FILES:
    Location: ~/.aiw/role/*.md
    Format: Description followed by delimiter (------------) then content

    Example role file (~/.aiw/role/code-reviewer.md):
        Expert code reviewer for security and performance
        ------------
        You are an expert code reviewer. Focus on:
        - Security vulnerabilities
        - Performance issues
        - Code style and best practices
        - Test coverage

ROLE INJECTION:
    Roles can be injected when using AIW as an MCP server.
    The 'launch_ai_cli_task' MCP tool accepts an optional 'role' parameter
    that prepends the role content to the task prompt.

    Example (via Claude Code MCP):
        Call 'launch_ai_cli_task' with:
        - ai_type: "claude"
        - task: "review this PR"
        - role: "code-reviewer"

    The actual prompt sent to claude will be:
        [role content from code-reviewer.md]

        ---

        review this PR
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print version information
#[allow(dead_code)]
pub fn print_version() -> io::Result<()> {
    println!("aiw 0.5.25");
    println!("Universal AI CLI Management Platform");
    println!();
    println!("Project home: https://github.com/putao520/agentic-warden");
    println!("License: MIT");
    io::stdout().flush()
}

/// Print quick usage examples
pub fn print_quick_examples() -> io::Result<()> {
    let examples = r#"
QUICK START EXAMPLES:

1. Get help:
   aiw help
   aiw help claude
   aiw help mcp

2. Basic AI CLI usage:
   aiw claude "Write a hello world program in Rust"
   aiw codex "Generate Python data visualization code"
   aiw gemini "Explain microservices architecture"

3. Provider selection:
   aiw claude -p openrouter "write code"
   aiw codex -p glm "explain this"

4. Parameter forwarding:
   aiw claude -p glm --model sonnet --debug api "explain this"
   aiw claude -p glm --print --output-format json "summarize"

5. Multiple AI agents:
   aiw all "Review this code"
   aiw "claude|gemini" "Compare these approaches"

6. MCP management:
   aiw mcp browse
   aiw mcp install @anthropic/filesystem
   aiw mcp list

7. Task monitoring:
   aiw wait
   aiw status
   aiw status --tui

8. Dashboard:
   aiw
   aiw dashboard
   aiw provider

For detailed help on any command:
    aiw help <command>
"#;
    print!("{}", examples);
    io::stdout().flush()
}
