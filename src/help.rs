//! Help module for aiw
//!
//! Provides comprehensive help information for all commands and features

use std::io::{self, Write};

/// Print general help information
pub fn print_general_help() -> io::Result<()> {
    let help_text = r#"
aiw v5.0.1 - Universal AI Agent Manager

USAGE:
    aiw [OPTIONS] <AGENT_SELECTOR> "<TASK_DESCRIPTION>"
    aiw <COMMAND> [COMMAND_ARGS]

AGENT SELECTORS:
    claude, codex, gemini    Single AI agent
    all                      All available agents
    "agent1|agent2"          Multiple agents (quotes required)

COMMANDS:
    wait                     Monitor task completion
    push                    Push configurations to cloud (default set)
    pull                    Pull configurations from cloud (default set)
    status                  Show sync status

OPTIONS:
    --help, -h              Show this help message
    --version, -v           Show version information

EXAMPLES:
    # Single agent tasks
    aiw claude "Write a Rust quicksort algorithm"
    aiw codex "Generate Python data visualization code"
    aiw gemini "Explain microservices architecture"

    # Multiple agent tasks
    aiw all "Review this code and suggest improvements"
    aiw "claude|gemini" "Compare these two approaches"
    aiw "claude|codex|gemini" "Write comprehensive documentation"

    # Configuration management
    aiw push
    aiw pull ~/.claude ~/.codex
    aiw status

    # Task monitoring
    aiw wait

For more detailed information about a specific command, use:
    aiw --help <command>

Project home: https://github.com/putao520/agentic-warden
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
        "provider" => print_provider_help(),
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Use 'aiw --help' for general help");
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
    aiw {} "<TASK_DESCRIPTION>"

DESCRIPTION:
    Send a task to the {} AI agent with full access permissions.

EXAMPLES:
    aiw {} "Write a sorting algorithm in Rust"
    aiw {} "Explain how neural networks work"
    aiw {} "Review this code for security issues"

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
    agentic-warden push

DESCRIPTION:
    Save current AI CLI configurations to Google Drive using the default name.

    Scans and uploads only specific configuration files.
    All directories and files are optional - only existing items will be backed up.

    NOTE: If ~/.claude, ~/.codex, and ~/.gemini do not exist,
    a message will be shown and the operation will be cancelled.

    Claude (~/.claude):
    - CLAUDE.md (memory file, if exists)
    - settings.json (config file, if exists)
    - agents/ directory and contents (if exists)
    - skills/ directory and all SKILL.md files (if exists)

    Codex (~/.codex):
    - auth.json (authentication config, if exists)
    - config.toml (config file, if exists)
    - version.json (version info, if exists)
    - agents.md (memory file, if exists)
    - history.jsonl (command history, if exists)

    Gemini (~/.gemini):
    - google_accounts.json (account config, if exists)
    - oauth_creds.json (OAuth credentials, if exists)
    - settings.json (config file, if exists)
    - gemini.md (memory file, if exists)
    - tmp/ directory (if exists)

    EXCLUDED FILES:
    - Cache files (.cache, __pycache__, etc.)
    - Temp files (tmp/, *.tmp, *.temp)
    - Log files (*.log, log/, sessions/)
    - History files (history.jsonl, todos/)
    - Binary files (.exe, .dll, .so)
    - Large files (>10MB)
    - Debug/IDE/shell snapshot files

    All files are saved in a single zip file:
    Google Drive: .aiw/default.zip

    OVERWRITE PROTECTION:
    - Before uploading, the system checks if a configuration with the same name
      already exists in Google Drive
    - If a duplicate is found, you will be prompted to confirm overwrite:
      * Choose 'Y' to overwrite the existing configuration
      * Choose 'N' to cancel the upload operation
    - This prevents accidental overwriting of existing configurations

EXAMPLES:
    agentic-warden push            # Save as default.zip

SETUP:
    Requires Google Drive OAuth configuration in ~/.aiw/auth.json
"#;
    print!("{}", help_text);
    io::stdout().flush()
}

/// Print help for pull command
fn print_pull_help() -> io::Result<()> {
    let help_text = r#"
PULL COMMAND

USAGE:
    agentic-warden pull

DESCRIPTION:
    Download and restore AI CLI configurations from Google Drive using the default name.

    Downloads and extracts:
    - Google Drive: .aiw/default.zip
    - Extracts to: ~/.claude, ~/.codex, ~/.gemini (directories will be created if needed)

    Restores only the specific configuration files that were backed up.
    The backup may contain files from any of the three AI CLI tools:
    - Claude: CLAUDE.md, settings.json, agents/, skills/ (all optional)
    - Codex: auth.json, config.toml, version.json, agents.md, history.jsonl (all optional)
    - Gemini: google_accounts.json, oauth_creds.json, settings.json, gemini.md, tmp/ (all optional)

    Cache and other excluded files are not affected.

EXAMPLES:
    agentic-warden pull            # Restore default.zip

NOTES:
    - Local configurations are overwritten
    - No backup is created before restoring
    - All three AI configs are restored together
    - Uses the default configuration name ("default")

SETUP:
    Requires Google Drive OAuth configuration in ~/.aiw/auth.json
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

/// Print version information
#[allow(dead_code)]
pub fn print_version() -> io::Result<()> {
    println!("aiw v5.0.1");
    println!("Universal AI Agent Manager");
    println!("Built with Rust");
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

/// Print help for provider command
fn print_provider_help() -> io::Result<()> {
    let help_text = r#"
PROVIDER COMMAND

USAGE:
    agentic-warden provider

DESCRIPTION:
    Launch the TUI Provider Management interface.

    This is a shortcut command that directly opens the Provider Management
    screen in the TUI. You can also access it from the Dashboard by pressing 'P'.

    For more help while in the TUI, press '?' or 'H'.
"#;
    print!("{}", help_text);
    io::stdout().flush()
}
