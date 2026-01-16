//! Help module for aiw
//!
//! Provides comprehensive help information for all commands and features

use std::io::{self, Write};

/// Print general help information
pub fn print_general_help() -> io::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let help_text = format!(r#"
aiw v{} - Universal AI CLI Management Platform

USAGE:
    aiw [OPTIONS] <COMMAND>
    aiw [OPTIONS] <AI_CLI> [AI_OPTIONS] "<TASK>"

AI CLI COMMANDS:
    aiw <agent> [-r ROLE] [-mp PROVIDER] [CLI_OPTIONS] ["TASK"]

    Agents:     claude | codex | gemini | all | "agent1|agent2"
    -r ROLE     Inject role prompt (run 'aiw roles list' to see all)
    -mp PROVIDER Use specific provider (e.g., openrouter, glm)
    [CLI_OPTIONS] Forwarded to AI CLI (run 'aiw claude --help' to see all)
    "TASK"      Task description (omit for interactive mode)

    ⚠️  Parameter order: -r and -mp must come BEFORE CLI options!

    Quick start:
        aiw claude "explain this code"           # Simple task
        aiw claude                               # Interactive mode
        aiw claude -r common "write a function"  # With role
        aiw claude -mp glm "help me debug"        # With provider
        aiw claude -r common -mp glm "full example"  # Role + provider

    For detailed AI CLI help:  aiw help claude

MANAGEMENT COMMANDS:
    dashboard                   Show Dashboard (default when no args)
    status [--tui]              Show task status
    provider                    Launch Provider Management TUI
    auto <prompt>               Run auto failover mode
    config cli-order            Manage AI CLI execution order (TUI)
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

PLUGIN COMMANDS:
    plugin browse               Interactive plugin browser (TUI)
    plugin search <query>       Search plugins in marketplace
    plugin install <name>       Install plugin from marketplace
    plugin remove <name>        Remove installed plugin
    plugin info <name>          Show plugin details
    plugin list                 List installed plugins
    plugin enable <name>        Enable installed plugin
    plugin disable <name>       Disable installed plugin
    plugin marketplace ...      Marketplace source management

OPTIONS:
    --help, -h                  Show this help message
    --version, -V               Show version information

EXAMPLES:
    # AI CLI with role injection
    aiw claude -r common "explain this code"
    aiw claude -r debugger -mp glm "debug this issue"
    aiw codex -r security "review this code"

    # AI CLI with provider selection
    aiw claude "explain this code"
    aiw claude -mp openrouter "explain this code"
    aiw codex -mp glm "write tests"

    # AI CLI with role + provider
    aiw claude -r frontend -mp glm "build a React component"

    # AI CLI with working directory
    aiw claude -C /path/to/project "implement feature"
    aiw claude -r common -C ~/myproject "fix bug"

    # AI CLI with parameter forwarding
    aiw claude -r security -mp glm --model sonnet --debug api "explain this"

    # Multiple AI agents
    aiw all "review this code"
    aiw "claude|gemini" "compare approaches"

    # Auto failover mode
    aiw auto "Fix this bug"

    # Configure auto execution order
    aiw config cli-order

    # MCP management
    aiw mcp browse
    aiw mcp install @anthropic/filesystem
    aiw mcp list

    # Task monitoring
    aiw wait
    aiw status

For more detailed information about a specific command:
    aiw help <command>
    aiw help claude     # AI CLI usage and parameters
    aiw help roles      # Role injection system
    aiw help mcp        # MCP server management
    aiw help plugin     # Plugin marketplace

Project home: https://github.com/putao520/agentic-warden
"#, version);
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for specific commands
pub fn print_command_help(command: &str) -> io::Result<()> {
    match command.to_lowercase().as_str() {
        "claude" | "codex" | "gemini" => print_ai_cli_help(command),
        "all" => print_all_agents_help(),
        "auto" => print_auto_help(),
        "wait" => print_wait_help(),
        "pwait" => print_pwait_help(),
        "status" => print_status_help(),
        "provider" => print_provider_help(),
        "dashboard" => print_dashboard_help(),
        "examples" | "demo" => print_examples_help(),
        "update" => print_update_help(),
        "mcp" => print_mcp_help(),
        "config" => print_config_help(),
        "roles" => print_roles_help(),
        "plugin" | "plugins" | "market" => print_plugin_help(),
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
    aiw {} [-r ROLE] [-mp PROVIDER] [-C DIR] [CLI_OPTIONS] ["<TASK>"]
    aiw {} [-r ROLE] [-mp PROVIDER] [-C DIR] [CLI_OPTIONS]

DESCRIPTION:
    Run the {} AI agent with role injection, provider management, and transparent parameter forwarding.

    Agent selector can be: claude, codex, or gemini

ROLE INJECTION:
    -r, --role <ROLE>          Use predefined role (e.g., common, debugger, security)

    Roles are prepended to your prompt to provide context and guidelines.
    22 builtin roles available (run 'aiw roles list' to see all).
    Custom roles can be placed in ~/.aiw/role/*.md

    Language is automatically detected from your system locale:
    - Chinese locales (zh_*) use Chinese role versions
    - All other locales (en_*, ja_*, ko_*, etc.) use English versions

PROVIDER SELECTION:
    -mp, --aiw-provider <PROVIDER>    Use specific provider (e.g., openrouter, glm)

    Without -mp, uses the default provider configured in ~/.aiw/providers.json

WORKING DIRECTORY (optional):
    -C, --cwd <DIR>              Set working directory for AI CLI process

    If specified, the AI CLI will start in the given directory.
    If not specified, uses current working directory.
    - Directory must exist (error if not found)
    - Path must be a directory (error if it's a file)

PARAMETER FORWARDING:
    All other parameters are forwarded directly to {} CLI.

    To see all {} CLI parameters:
        aiw {} --help

    Parameter order: AIW options (-r, -mp) must come BEFORE {} CLI options.

INTERACTIVE MODE:
    aiw {} [-r ROLE] [-mp PROVIDER] [CLI_OPTIONS]

    Start {} in interactive mode (no task specified).
    Useful for extended conversations with the AI.

TASK MODE:
    aiw {} [-r ROLE] [-mp PROVIDER] [CLI_OPTIONS] "your task here"

    Run a single task and exit.

EXAMPLES:

    # Basic usage (default provider, no role)
    aiw {} "write a Rust function"
    aiw {} "explain this code"

    # With role injection
    aiw {} -r common "write a function following coding standards"
    aiw {} -r debugger "help me fix this bug"
    aiw {} -r security -mp glm "review this code for vulnerabilities"

    # With provider selection
    aiw {} -mp openrouter "write python code"
    aiw {} -mp glm "write tests"

    # With role + provider
    aiw {} -r frontend -mp glm "build a React component"
    aiw {} -r database -mp openrouter "design a schema"

    # With working directory
    aiw {} -C /path/to/project "implement this feature"
    aiw {} -r common -C ~/myproject "fix the bug in main.rs"

    # With parameter forwarding (note: prompt must be last)
    aiw {} -r security -mp glm --model sonnet "explain this"
    aiw {} -r ml -mp glm --output-format json "analyze this data"

    # Interactive mode with role (no task = interactive)
    aiw {} -r code-reviewer -mp glm

    # Multi-agent selection
    aiw all "review this code"
    aiw "claude|gemini" "compare implementations"

AVAILABLE ROLES:
    common              General programming standards
    debugger            Code debugging and analysis
    security            Security review and best practices
    frontend-standards  Frontend development standards
    database-standards  Database development standards
    testing-standards   Testing and quality assurance
    deployment          Deployment and DevOps
    devops              DevOps and CI/CD
    quality             Code quality and review
    blockchain          Blockchain development
    ml                  Machine learning
    embedded            Embedded systems
    iot                 IoT development
    mobile-android      Android development
    mobile-ios          iOS development
    game                Game development
    game-unity          Unity game development
    game-unreal         Unreal game development
    graphics            Graphics programming
    multimedia          Multimedia processing
    big-data-standards  Big data processing
    assistant-programmer Assistant programming specialist

CONFIGURATION:
    Providers: ~/.aiw/providers.json
    MCP servers: ~/.aiw/mcp.json
    Custom roles: ~/.aiw/role/*.md

For more information:
    aiw help roles    - Role management and usage
    aiw help mcp      - MCP integration
    aiw roles list    - List all available roles
"#,
        agent.to_uppercase(),
        agent,
        agent,
        agent,
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

fn print_auto_help() -> io::Result<()> {
    let help_text = r#"
AUTO MODE

USAGE:
    aiw auto "<TASK>"

DESCRIPTION:
    Run automatic failover across AI CLIs in the configured order.
    Execution order is stored in ~/.aiw/config.json (cli_execution_order).

EXAMPLES:
    aiw auto "Fix this bug"
    aiw config cli-order
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for "all" agent selector
fn print_all_agents_help() -> io::Result<()> {
    let help_text = r#"
ALL AGENTS

USAGE:
    aiw all [-r ROLE] [-mp PROVIDER] [CLI_OPTIONS] "<TASK>"

DESCRIPTION:
    Send the same task to all available AI agents (claude, codex, gemini).

    Role injection applies to all agents.

EXAMPLES:
    aiw all "review this code and suggest improvements"
    aiw all -r common "explain this algorithm in detail"
    aiw all -mp glm "write comprehensive documentation"

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
        aiw codex -mp glm "explain this function"
        aiw gemini "refactor this code"

    Role Injection:
        aiw claude -r common "write code following standards"
        aiw claude -r debugger "debug this issue"
        aiw codex -r security "review for vulnerabilities"

    Provider Management:
        aiw claude -mp openrouter "write tests"
        aiw codex -mp glm --temperature 0.7 "generate code"

    Parameter Forwarding:
        aiw claude -mp glm --model sonnet "explain this code"
        aiw claude -mp glm --output-format json "summarize this"
        aiw claude -mp glm --allowed-tools Bash,Edit "modify file"

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
    add <name> <cmd> [ARGS] [OPTIONS]
                                Add a new MCP server
        --description <desc>    Server description
        --category <cat>        Server category
        --env KEY=VALUE         Environment variables (repeatable)
        --disabled              Add but don't enable
    remove <name> [-y]          Remove an MCP server
    enable <name>               Enable a disabled server
    disable <name>              Disable an enabled server
    edit                        Edit MCP configuration in editor
    get <name>                  Show server configuration

MCP REGISTRY:
    browse [--source <src>]     Interactive server browser
    search <query> [OPTIONS]    Search MCP registries for servers
        --source <src>          Specify source (registry|smithery)
        --limit <n>             Limit results count
    install <name> [OPTIONS]    Install server from registry
        --source <src>          Specify source
        --env KEY=VALUE         Environment variables (repeatable)
        --skip-env              Skip environment variable configuration
    info <name> [--source <src>]
                                Show detailed server information
    update                      Update registry cache

INTERNAL:
    serve [--transport <type>] [--log-level <level>]
                                Start MCP server (for Claude Code)
        --transport             Transport type (stdio, default: stdio)
        --log-level             Log level (debug|info|warn|error)

EXAMPLES:

    # Server management
    aiw mcp list
    aiw mcp add filesystem npx -- -y @modelcontextprotocol/server-filesystem $HOME
    aiw mcp add myserver node server.js --env API_KEY=xxx --description "My server"
    aiw mcp enable filesystem
    aiw mcp disable filesystem
    aiw mcp remove filesystem -y

    # Registry operations
    aiw mcp browse
    aiw mcp browse --source smithery
    aiw mcp search "filesystem"
    aiw mcp search "database" --source registry --limit 10
    aiw mcp install @anthropic/filesystem
    aiw mcp install myserver --env API_KEY=xxx
    aiw mcp info @anthropic/filesystem
    aiw mcp update

    # Configuration
    aiw mcp get filesystem
    aiw mcp edit

MCP CONFIGURATION:
    Servers: ~/.aiw/mcp.json
    Compatible with Claude Code's mcpServers configuration

REGISTRIES:
    - Official MCP Registry (registry.modelcontextprotocol.io)
    - Smithery (registry.smithery.ai)

For information on using AIW as an MCP server:
    See project documentation at https://github.com/putao520/agentic-warden
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

fn print_config_help() -> io::Result<()> {
    let help_text = r#"
CONFIG COMMANDS

USAGE:
    aiw config cli-order

DESCRIPTION:
    Launch a TUI to configure the AI CLI execution order.

TUI KEYS:
    ↑ / ↓     Move selection
    r         Reset to default order
    q         Save and quit
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

    AIW provides 22 builtin roles that define coding standards, best practices,
    and specialized knowledge areas. These roles are embedded in the binary and
    always available.

    You can also create custom roles in ~/.aiw/role/ directory.

LANGUAGE SUPPORT:
    Roles are available in multiple languages. The system automatically detects
    your locale and selects the appropriate language version:

    - English locales (en_US, en_GB, etc.) → English role versions
    - Chinese and other locales → Chinese role versions (default)

    Currently, only the "common" role has an English translation. Other roles
    will use Chinese as fallback. Additional translations will be added gradually.

EXAMPLES:
    aiw roles list

BUILTIN ROLES:
    common              General programming standards and best practices
    debugger            Code debugging and error analysis
    security            Security review and vulnerability assessment
    frontend-standards  Frontend development guidelines
    database-standards  Database development standards
    testing-standards   Testing and quality assurance standards
    deployment          Deployment strategies and DevOps
    devops              DevOps and CI/CD pipelines
    quality             Code quality standards and review
    blockchain          Blockchain and cryptocurrency development
    ml                  Machine learning engineering
    embedded            Embedded systems programming
    iot                 IoT development and edge computing
    mobile-android      Android development
    mobile-ios          iOS development
    game                Game development fundamentals
    game-unity          Unity game engine development
    game-unreal         Unreal game engine development
    graphics            Graphics and GPU programming
    multimedia          Multimedia and signal processing
    big-data-standards  Big data processing and analytics
    assistant-programmer Assistant programming specialist

CUSTOM ROLES:
    Location: ~/.aiw/role/*.md
    Format: Markdown with first line as description

    Example custom role (~/.aiw/role/my-specialist.md):
        My Domain Expert
        ---
        You are an expert in this specific domain.
        Focus on:
        - Domain-specific patterns
        - Best practices
        - Common pitfalls

ROLE INJECTION:
    Use -r/--role parameter to inject roles into AI CLI sessions.

    Examples:
        aiw claude -r common "write code following standards"
        aiw claude -r debugger -mp glm "help debug this issue"
        aiw codex -r security "review for vulnerabilities"

    Role content is prepended to your prompt with a separator:
        [role content]

        ---

        [your prompt]

ROLE PRIORITY:
    1. Builtin roles (embedded in binary, always available)
    2. Custom roles (from ~/.aiw/role/, if exists)

    If a role name exists in both, builtin takes precedence.

ADVANCED USAGE:
    # Use role with provider and custom parameters
    aiw claude -r security -mp glm --model sonnet "audit this code"

    # Use role in interactive mode
    aiw claude -r code-reviewer -mp glm
    # Then interact with the AI in the review context

    # Use role with multi-agent
    aiw all -r frontend "review the UI code"

For role usage with AI CLI:
    aiw help claude

For role usage with MCP server:
    aiw help mcp
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for plugin command
fn print_plugin_help() -> io::Result<()> {
    let help_text = r#"
PLUGIN COMMANDS

USAGE:
    aiw plugin <SUBCOMMAND> [ARGS]

PLUGIN MANAGEMENT:
    browse [--market <name>] [--category <cat>] [--tags <tags>]
                                Interactive plugin browser (TUI)
    search <query> [--market <name>]
                                Search plugins in marketplace
    install <plugin> [--env KEY=VALUE] [--skip-env]
                                Install plugin from marketplace
    remove <plugin>             Remove installed plugin
    info <plugin>               Show plugin details
    list [--show-disabled]      List installed plugins
    enable <plugin>             Enable installed plugin
    disable <plugin>            Disable installed plugin

MARKETPLACE SOURCE MANAGEMENT:
    marketplace add <repo_url> [--name <alias>]
                                Add marketplace source (GitHub repo/URL)
    marketplace list            List configured marketplace sources
    marketplace remove <name>   Remove marketplace source
    marketplace update [name]   Update marketplace index cache

DESCRIPTION:
    The plugin system extends AIW functionality through a marketplace
    of community-contributed plugins. Plugins can provide additional
    AI providers, tools, integrations, and workflows.

    Plugin names can include marketplace source: <plugin_name>@<market>

EXAMPLES:

    # Browse available plugins
    aiw plugin browse
    aiw plugin browse --category "ai-tools"
    aiw plugin browse --market my-market

    # Search for plugins
    aiw plugin search "openai"
    aiw plugin search "translator" --market official

    # Install a plugin
    aiw plugin install openai-provider
    aiw plugin install my-plugin@my-market
    aiw plugin install my-plugin --env API_KEY=xxx

    # List installed plugins
    aiw plugin list
    aiw plugin list --show-disabled

    # Enable/disable plugins
    aiw plugin enable my-plugin
    aiw plugin disable my-plugin

    # Remove a plugin
    aiw plugin remove openai-provider

    # Marketplace source management
    aiw plugin marketplace add https://github.com/user/plugins --name my-market
    aiw plugin marketplace list
    aiw plugin marketplace update
    aiw plugin marketplace remove my-market

PLUGIN LOCATIONS:
    Plugins: ~/.aiw/plugins/
    Marketplace config: ~/.aiw/marketplaces.json
    Registry cache: ~/.aiw/cache/plugins/

For more information:
    Visit https://github.com/putao520/agentic-warden
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print version information
#[allow(dead_code)]
pub fn print_version() -> io::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    println!("aiw {}", version);
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

3. Role injection:
   aiw claude -r common "Write code following standards"
   aiw claude -r debugger "Help me debug this issue"
   aiw codex -r security "Review for vulnerabilities"

4. Provider selection:
   aiw claude -mp openrouter "write code"
   aiw codex -mp glm "explain this"

5. Parameter forwarding:
   aiw claude -mp glm --model sonnet "explain this code"
   aiw claude -mp glm --output-format json "summarize this"

6. Multiple AI agents:
   aiw all "Review this code"
   aiw "claude|gemini" "Compare these approaches"

7. MCP management:
   aiw mcp browse
   aiw mcp install @anthropic/filesystem
   aiw mcp list

8. Task monitoring:
   aiw wait
   aiw status
   aiw status --tui

9. Dashboard:
   aiw
   aiw dashboard
   aiw provider

10. List available roles:
    aiw roles list

For detailed help on any command:
    aiw help <command>
"#;
    print!("{}", examples);
    io::stdout().flush()
}
