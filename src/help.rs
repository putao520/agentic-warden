//! Help module for agentic-warden
//!
//! Provides comprehensive help information for all commands and features

use std::io::{self, Write};

/// Print general help information
pub fn print_general_help() -> io::Result<()> {
    let help_text = r#"
agentic-warden v0.3.0 - Universal AI Agent Manager

USAGE:
    agentic-warden [OPTIONS] <AGENT_SELECTOR> "<TASK_DESCRIPTION>"
    agentic-warden <COMMAND> [COMMAND_ARGS]

AGENT SELECTORS:
    claude, codex, gemini    Single AI agent
    all                      All available agents
    "agent1|agent2"          Multiple agents (quotes required)

COMMANDS:
    wait                     Monitor task completion
    push [DIR...]           Push configurations to cloud
    pull [DIR...]           Pull configurations from cloud
    status                  Show sync status
    reset                   Reset sync state
    list                    List configurable directories

OPTIONS:
    --help, -h              Show this help message
    --version, -v           Show version information

EXAMPLES:
    # Single agent tasks
    agentic-warden claude "Write a Rust quicksort algorithm"
    agentic-warden codex "Generate Python data visualization code"
    agentic-warden gemini "Explain microservices architecture"

    # Multiple agent tasks
    agentic-warden all "Review this code and suggest improvements"
    agentic-warden "claude|gemini" "Compare these two approaches"
    agentic-warden "claude|codex|gemini" "Write comprehensive documentation"

    # Configuration management
    agentic-warden push
    agentic-warden pull ~/.claude ~/.codex
    agentic-warden status

    # Task monitoring
    agentic-warden wait

For more detailed information about a specific command, use:
    agentic-warden --help <command>

Project home: https://github.com/your-username/agentic-warden
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for specific commands
pub fn print_command_help(command: &str) -> io::Result<()> {
    match command.to_lowercase().as_str() {
        "claude" | "codex" | "gemini" => print_agent_help(command),
        "all" => print_all_agents_help(),
        "wait" => print_wait_help(),
        "push" => print_push_help(),
        "pull" => print_pull_help(),
        "status" => print_status_help(),
        "reset" => print_reset_help(),
        "list" => print_list_help(),
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Use 'agentic-warden --help' for general help");
            Ok(())
        }
    }
}

/// Print help for specific agents
fn print_agent_help(agent: &str) -> io::Result<()> {
    let help_text = format!(
        r#"
{} AGENT

USAGE:
    agentic-warden {} "<TASK_DESCRIPTION>"

DESCRIPTION:
    Send a task to the {} AI agent with full access permissions.

EXAMPLES:
    agentic-warden {} "Write a sorting algorithm in Rust"
    agentic-warden {} "Explain how neural networks work"
    agentic-warden {} "Review this code for security issues"

The agent will run with full access permissions and automatically
handle file operations, code execution, and other tasks.

CONFIGURATION:
    Custom agent path can be set with environment variable:
    export {}_BIN="/path/to/custom/{}"
"#,
        agent.to_uppercase(),
        agent,
        agent,
        agent,
        agent,
        agent,
        agent.to_uppercase(),
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
    agentic-warden all "<TASK_DESCRIPTION>"

DESCRIPTION:
    Send the same task to all available AI agents (Claude, Codex, Gemini).

EXAMPLES:
    agentic-warden all "Review this code and suggest improvements"
    agentic-warden all "Explain this algorithm in detail"
    agentic-warden all "Write comprehensive documentation"

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
    agentic-warden wait

DESCRIPTION:
    Enter monitoring mode to track task completion and status.

    - Shows active tasks and their progress
    - Displays completion status for finished tasks
    - Provides cleanup options for completed tasks
    - Monitors shared memory for task registry updates

FEATURES:
    - Real-time task status updates
    - Automatic cleanup of completed tasks
    - Detailed task information display
    - Process tree visualization
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for push command
fn print_push_help() -> io::Result<()> {
    let help_text = r#"
PUSH COMMAND

USAGE:
    agentic-warden push [DIRECTORY...]

DESCRIPTION:
    Push configuration directories to Google Drive cloud storage.

ARGUMENTS:
    [DIRECTORY...]    Optional directories to push
                      If not specified, uses default directories

DEFAULT DIRECTORIES:
    - ~/.claude
    - ~/.codex
    - ~/.gemini

FEATURES:
    - Automatic change detection using MD5 hashing
    - Cross-platform compression (TAR.GZ)
    - Secure OAuth 2.0 authentication
    - Incremental sync (only changed files)

SETUP:
    Requires Google Drive OAuth configuration in ~/.agentic-warden/auth.json
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for pull command
fn print_pull_help() -> io::Result<()> {
    let help_text = r#"
PULL COMMAND

USAGE:
    agentic-warden pull [DIRECTORY...]

DESCRIPTION:
    Pull configuration directories from Google Drive cloud storage.

ARGUMENTS:
    [DIRECTORY...]    Optional directories to pull
                      If not specified, uses default directories

FEATURES:
    - Automatic backup of local configurations
    - Conflict resolution options
    - Cross-platform decompression
    - Secure file verification

SETUP:
    Requires Google Drive OAuth configuration in ~/.agentic-warden/auth.json
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for status command
fn print_status_help() -> io::Result<()> {
    let help_text = r#"
STATUS COMMAND

USAGE:
    agentic-warden status

DESCRIPTION:
    Display synchronization status and configuration information.

OUTPUT INCLUDES:
    - Google Drive connection status
    - Last synchronization timestamp
    - Directory sync status
    - Configuration file validation
    - Authentication token status

EXAMPLE OUTPUT:
    ✓ Google Drive: Connected
    ✓ Last sync: 2024-01-15 14:30:25 UTC
    ✓ ~/.claude: Up to date
    ✓ ~/.codex: Changes detected
    ✗ ~/.gemini: Never synced
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for reset command
fn print_reset_help() -> io::Result<()> {
    let help_text = r#"
RESET COMMAND

USAGE:
    agentic-warden reset

DESCRIPTION:
    Reset synchronization state and clear local cache.

WARNING:
    This will clear all local sync history. Your cloud files
    will remain intact, but the next sync will be a full sync.

EFFECTS:
    - Clears local sync state
    - Removes MD5 hash cache
    - Forces full re-sync on next push/pull
    - Preserves authentication tokens

CONFIRMATION:
    You will be prompted to confirm before reset proceeds.
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for list command
fn print_list_help() -> io::Result<()> {
    let help_text = r#"
LIST COMMAND

USAGE:
    agentic-warden list

DESCRIPTION:
    List all configurable directories and their status.

OUTPUT INCLUDES:
    - Directory paths
    - Sync status
    - File counts
    - Total sizes
    - Last modification times

EXAMPLE OUTPUT:
    CONFIGURABLE DIRECTORIES:
    ~/.claude
        Status: Configured for sync
        Files: 15 | Size: 2.3MB | Modified: 2024-01-15 10:30

    ~/.codex
        Status: Not configured
        Files: 8 | Size: 1.1MB | Modified: 2024-01-14 16:45

    ~/.gemini
        Status: Configured for sync
        Files: 12 | Size: 1.8MB | Modified: 2024-01-15 09:15
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print version information
pub fn print_version() -> io::Result<()> {
    println!("agentic-warden v0.3.0");
    println!("Universal AI Agent Manager");
    println!("Built with Rust");
    println!();
    println!("Project home: https://github.com/your-username/agentic-warden");
    println!("License: MIT");
    io::stdout().flush()
}

/// Print quick usage examples
pub fn print_quick_examples() -> io::Result<()> {
    let examples = r#"
QUICK START EXAMPLES:

1. Get help:
   agentic-warden --help

2. Send a task to Claude:
   agentic-warden claude "Write a hello world program in Rust"

3. Get multiple AI perspectives:
   agentic-warden "claude|gemini" "Explain quantum computing"

4. Task all available agents:
   agentic-warden all "Review this Python code for optimizations"

5. Monitor tasks:
   agentic-warden wait

6. Sync configurations:
   agentic-warden push
   agentic-warden pull
   agentic-warden status

For detailed help on any command:
    agentic-warden --help <command>
"#;
    print!("{}", examples);
    io::stdout().flush()
}
