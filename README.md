# AIW - AI CLI & MCP Unified Gateway

<div align="center">

![Version](https://img.shields.io/badge/version-0.5.48-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![MCP](https://img.shields.io/badge/MCP-Supported-purple?style=flat-square)

**Unified Router & Proxy for AI CLI Tools and MCP Servers**

</div>

## What is AIW?

AIW is a **unified gateway** that acts as:

| Layer | Role | What it does |
|-------|------|--------------|
| **AI CLI Proxy** | Router + Proxy | Route requests to claude/codex/gemini with provider switching, role injection, and transparent parameter forwarding |
| **MCP Proxy** | Router + Proxy | Route tool calls to multiple MCP servers with intelligent selection, plugin marketplace, and hot-reload |

```
┌─────────────────────────────────────────────────────────────┐
│                         AIW Gateway                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────┐    ┌─────────────────────────────┐ │
│  │   AI CLI Router     │    │      MCP Router             │ │
│  │                     │    │                             │ │
│  │  aiw claude ...  ───┼───►│  Claude CLI                 │ │
│  │  aiw codex ...   ───┼───►│  Codex CLI                  │ │
│  │  aiw gemini ...  ───┼───►│  Gemini CLI                 │ │
│  │                     │    │                             │ │
│  │  + Provider Switch  │    │  aiw mcp serve ────────────►│ │
│  │  + Role Injection   │    │    ├─► filesystem server    │ │
│  │  + Param Forwarding │    │    ├─► git server           │ │
│  │  + CWD Control      │    │    ├─► database server      │ │
│  │                     │    │    └─► ... (plugin market)  │ │
│  └─────────────────────┘    └─────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Installation

```bash
# Install from NPM
npm install -g @putao520/aiw

# Verify installation
aiw --version
```

## AI CLI Router & Proxy

### Basic Usage

```bash
# Route to specific AI CLI
aiw claude "explain this code"
aiw codex "write tests"
aiw gemini "translate to Chinese"

# Auto mode: automatic failover across AI CLIs
aiw auto "fix this bug"              # Try CLIs in order, auto-switch on failure
aiw auto -p auto "implement feature" # Auto-switch CLIs + auto-select provider

# Route to multiple AI CLIs
aiw all "review this code"              # All available CLIs
aiw "claude|gemini" "compare approaches" # Specific CLIs
```

### Provider Switching (-p)

```bash
# Switch API provider without changing AI CLI
aiw claude -p openrouter "explain this"
aiw claude -p glm "explain this"
aiw claude -p anthropic "explain this"

# Auto-select compatible provider
aiw claude -p auto "explain this"     # Randomly select compatible provider
aiw auto -p auto "implement feature"  # Auto CLI + auto provider

# Provider config: ~/.aiw/providers.json
```

### Auto Mode (Automatic Failover)

```bash
# Auto mode tries CLI+Provider combinations in configured order, switches on failure
aiw auto "fix this bug"

# Configure CLI+Provider execution order
aiw config cli-order  # TUI to manage order (↑/↓ move, r reset, q save)
```

**How it works**:
1. Reads `auto_execution_order` from `~/.aiw/config.json`
2. Tries each CLI+Provider combination in order (e.g., codex+auto → gemini+auto → claude+auto)
3. Judges success/failure by exit code (0 = success)
4. Switches to next combination if current fails
5. Failed combinations enter 30-second cooldown period

**Configuration** (`~/.aiw/config.json`):
```json
{
  "auto_execution_order": [
    {"cli": "codex", "provider": "auto"},
    {"cli": "gemini", "provider": "auto"},
    {"cli": "claude", "provider": "glm"},
    {"cli": "claude", "provider": "local"},
    {"cli": "claude", "provider": "official"}
  ]
}
```

**Features**:
- Same CLI can be configured with multiple providers (e.g., claude+glm → claude+local → claude+official)
- Each CLI+Provider combination has independent cooldown (30 seconds after failure)
- Provider "auto" means use the CLI's default provider selection
- Order can be fully customized via TUI or direct config editing

### Role Injection (-r)

```bash
# Inject role prompt before task
aiw claude -r common "write a function"
aiw claude -r security "review this code"
aiw claude -r debugger "fix this bug"

# 22 built-in roles + custom roles in ~/.aiw/role/*.md
aiw roles list
```

### Working Directory (-C)

```bash
# Start AI CLI in specific directory
aiw claude -C /path/to/project "implement feature"
aiw claude -r common -C ~/myproject "fix the bug"
```

### Transparent Parameter Forwarding

```bash
# All unknown flags forwarded to AI CLI
aiw claude -p glm --model sonnet --debug api "explain this"
aiw claude -r security --print --output-format json "review"

# Order: aiw flags (-r, -p, -C) → AI CLI flags → prompt
```

### Combined Example

```bash
# Full example with all options
aiw claude -r common -p glm -C ~/project --model sonnet "implement REQ-001"
#          ^^^^^^^^  ^^^^^  ^^^^^^^^^^^  ^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^
#          role      provider  cwd        forwarded     prompt
```

## MCP Router & Proxy

### Start MCP Server

```bash
# Start AIW as MCP server
aiw mcp serve

# Configure in Claude Code (~/.claude/settings.json)
{
  "mcpServers": {
    "aiw": {
      "command": "aiw",
      "args": ["mcp", "serve"]
    }
  }
}
```

### MCP Server Management

```bash
# List configured MCP servers
aiw mcp list

# Add MCP server
aiw mcp add filesystem npx -- -y @modelcontextprotocol/server-filesystem $HOME

# Enable/disable servers (hot-reload)
aiw mcp enable filesystem
aiw mcp disable git

# Edit config directly
aiw mcp edit
```

### MCP Registry (Search & Install)

```bash
# Browse all available MCP servers (interactive TUI)
aiw mcp browse

# Search across registries (Official + Smithery)
aiw mcp search "git"
aiw mcp search "database" --source official

# Get server info
aiw mcp info @anthropic/filesystem

# Install server
aiw mcp install @anthropic/filesystem
aiw mcp install @anthropic/filesystem --env API_KEY=xxx
```

### Plugin Marketplace

```bash
# Browse MCP plugins (interactive TUI)
aiw plugin browse

# Search plugins
aiw plugin search "playwright"

# Install plugin
aiw plugin install playwright@claude-code-official

# List/manage installed plugins
aiw plugin list
aiw plugin enable playwright
aiw plugin disable serena
aiw plugin remove playwright

# Manage marketplace sources
aiw plugin marketplace list
aiw plugin marketplace add my-market https://github.com/user/plugins
```

## Task Monitoring

```bash
# Show task status
aiw status

# Wait for all AI CLI tasks to complete
aiw wait

# Wait for specific process
aiw pwait <PID>
```

## Update

```bash
# Update AIW and all AI CLI tools
aiw update
```

## Configuration Files

| File | Purpose |
|------|---------|
| `~/.aiw/config.json` | AIW global configuration |
| `~/.aiw/providers.json` | AI provider configurations |
| `~/.aiw/mcp.json` | MCP server configurations |
| `~/.aiw/role/*.md` | Custom role prompts |
| `~/.aiw/settings.json` | Plugin marketplace settings |
| `~/.aiw/plugins.json` | Installed plugin records |

### Global Configuration (~/.aiw/config.json)

```json
{
  "user_roles_dir": "~/.claude/roles",
  "auto_execution_order": [
    {"cli": "codex", "provider": "auto"},
    {"cli": "gemini", "provider": "auto"},
    {"cli": "claude", "provider": "auto"}
  ]
}
```

| Option | Type | Description |
|--------|------|-------------|
| `user_roles_dir` | string | Custom directory for user roles (supports `~` expansion). If set, AIW will load user roles from this directory instead of `~/.aiw/role/` |
| `auto_execution_order` | array | CLI+Provider combinations for auto mode. Each entry has `cli` (codex/gemini/claude) and `provider` (provider name or "auto"). Use `aiw config cli-order` TUI to manage |

This allows you to manage all your roles in a single location, such as `~/.claude/roles/`, and share them across different tools.

## Quick Reference

```bash
# AI CLI routing
aiw <cli> [-r role] [-p provider] [-C cwd] [cli-args...] "prompt"
aiw auto [-p provider] "prompt"  # Auto mode with failover

# Configuration
aiw config cli-order        # Manage CLI execution order (TUI)

# MCP commands
aiw mcp serve              # Start MCP server
aiw mcp list|add|remove    # Manage servers
aiw mcp browse|search      # Registry
aiw mcp install|info       # Install servers

# Plugin commands
aiw plugin browse|search   # Discover plugins
aiw plugin install|remove  # Manage plugins
aiw plugin list|enable|disable

# Other commands
aiw status                 # Task status
aiw wait                   # Wait for tasks
aiw update                 # Update tools
aiw roles list             # List roles
aiw help <command>         # Detailed help
```

## License

MIT License - see [LICENSE](LICENSE) file for details.

---

**AIW** - Unified Gateway for AI CLI & MCP | v0.5.48

[GitHub](https://github.com/putao520/agentic-warden) | [NPM](https://www.npmjs.com/package/@putao520/aiw)
