# AIW - AI CLI & MCP Unified Gateway

<div align="center">

![Version](https://img.shields.io/badge/version-0.5.60-blue?style=flat-square)
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

- Same CLI can be configured with multiple providers (e.g., claude+glm → claude+local → claude+official)
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

### Git Worktree (Isolated Execution)

AIW automatically creates a git worktree for isolated AI CLI execution.

```bash
# AIW automatically creates worktree for git repositories
aiw codex -C /path/to/repo "implement feature"

# After completion, AIW outputs:
# === AIW WORKTREE END ===
# Worktree: /tmp/aiw-worktree-a1b2c3d4
# Branch: main
# Commit: abc123def456
```

The AI CLI works in a temporary worktree at `/tmp/aiw-worktree-<hash>`, keeping your working directory clean. Worktree remains after completion for manual review — merge changes or delete as needed.

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

## Using AIW with Claude Code (CLAUDE.md Guide)

When AIW runs as an MCP server inside Claude Code, it can launch background AI CLI tasks (e.g., `aiw codex`) and manage them via MCP tools. To get the best results, add the following instructions to your project's `CLAUDE.md` or `~/.claude/CLAUDE.md`:

<details>
<summary><strong>Recommended CLAUDE.md snippet (click to expand)</strong></summary>

````markdown
## AIW Background Task Guidelines

AIW is configured as an MCP server and can launch background AI CLI tasks (codex, gemini, claude) via `start_task` / `manage_task` tools.

### When to use AIW background tasks

Use `aiw codex` (via Bash tool, `run_in_background: true`) for tasks that:
- Require compilation, testing, or package installation
- Involve iterative development (write → build → fix → repeat)
- Create new modules with many files
- Need external tools (database, Docker, API calls)
- Can run independently in parallel

### How to launch a task

```bash
# Basic: launch a coding task in the background
aiw codex -r common "implement user authentication module"

# With specialized role
aiw codex -r common,frontend-standards "build the landing page"

# Target a different project directory
aiw codex -r common -C /path/to/project "implement feature X"
```

Key flags:
- `-r <roles>`: Inject role prompts (comma-separated). `common` is recommended as a base role. Run `aiw roles list` to see all available roles.
- `-C <dir>`: Set working directory for the task (defaults to current directory).
- `-p <provider>`: Override API provider (e.g., `openrouter`, `glm`, `auto`).

### Parallel execution

Independent tasks should be launched in parallel using multiple `Bash` calls with `run_in_background: true`:

```
Bash(command="aiw codex -r common 'implement user module'", run_in_background=true, timeout=43200000)
Bash(command="aiw codex -r common 'implement payment module'", run_in_background=true, timeout=43200000)
Bash(command="aiw codex -r common,frontend-standards 'implement frontend pages'", run_in_background=true, timeout=43200000)
```

Dependent tasks must run sequentially — do not parallelize tasks that depend on each other's output.

### Worktree isolation

AIW automatically creates a git worktree (at `/tmp/aiw-worktree-<hash>`) for each task when running inside a git repository. This keeps your working directory clean. After task completion, review the worktree output and merge changes as needed.

### Task lifecycle

1. **Launch**: Use Bash with `run_in_background: true` to start `aiw codex` tasks.
2. **Continue working**: Do not block-wait. Move on to other work after launching.
3. **Check status**: Use `aiw status` or check the background task output when needed.
4. **Collect results**: After tasks complete, review outputs and merge worktree changes.
5. **Fix issues in batch**: If multiple tasks have issues, collect all problems first, then fix them in one pass — do not fix one-by-one.
````

</details>

### Quick Setup

1. Configure AIW as MCP server (see [MCP Router & Proxy](#mcp-router--proxy))
2. Copy the snippet above into your `CLAUDE.md`
3. Customize roles and providers to match your workflow

### Available Roles

Run `aiw roles list` to see all built-in roles. Common ones:

| Role | Use case |
|------|----------|
| `common` | General-purpose coding (recommended as base) |
| `frontend-standards` | Frontend development |
| `database-standards` | Backend / database work |
| `testing-standards` | Test code |
| `security` | Security review |
| `debugger` | Debugging |
| `devops` | DevOps / infrastructure |

Combine roles with commas: `-r common,frontend-standards`

## License

MIT License - see [LICENSE](LICENSE) file for details.

---

**AIW** - Unified Gateway for AI CLI & MCP | v0.5.60

[GitHub](https://github.com/putao520/agentic-warden) | [NPM](https://www.npmjs.com/package/@putao520/aiw)
