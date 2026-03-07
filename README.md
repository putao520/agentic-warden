# AIW - AI CLI Unified Gateway

<div align="center">

![Version](https://img.shields.io/badge/version-0.5.74-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)

**Unified Router & Proxy for AI CLI Tools**

</div>

## What is AIW?

AIW is a **unified gateway** that acts as an AI CLI proxy router with provider switching, role injection, and transparent parameter forwarding.

```
┌─────────────────────────────────────────────────────────────┐
│                         AIW Gateway                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────┐                                     │
│  │   AI CLI Router     │                                     │
│  │                     │                                     │
│  │  aiw claude ...  ───┼───► Claude CLI                     │
│  │  aiw codex ...   ───┼───► Codex CLI                      │
│  │  aiw gemini ...  ───┼───► Gemini CLI                     │
│  │                     │                                     │
│  │  + Provider Switch  │                                     │
│  │  + Role Injection   │                                     │
│  │  + Tool Search Unlock │                                    │
│  │  + Param Forwarding │                                     │
│  │  + CWD Control      │                                     │
│  └─────────────────────┘                                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Installation

```bash
# Install from crates.io
cargo install aiw

# Verify installation
aiw --version
```

## AI CLI Router

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

### Tool Search Unlock

When using third-party API providers (non-Anthropic official), Claude's Tool Search feature is disabled by default. AIW automatically unlocks this feature via runtime memory patch.

```bash
# Tool Search is automatically enabled when using third-party providers
aiw claude -p glm "use web search to find latest Rust news"
aiw claude -p openrouter "search for documentation"
```

**How it works**: AIW applies a runtime patch to the Claude process, changing `if(O8()==="firstParty"&&!JB())` to `if(O8()!=="firstParty"&&!JB())`, enabling Tool Search for third-party providers.

- Works on Linux/macOS/Windows
- Non-destructive: only affects running process, no system files modified
- Automatic retry with increasing delays (up to 5 attempts)

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

# Built-in roles + custom roles in ~/.aiw/role/*.md
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
# Update AIW itself and all installed AI CLI tools
aiw update
```

The `update` command checks and updates:
- **AIW**: Updates via `cargo install aiw --force` if installed via cargo
- **Claude CLI**: Uses native `claude update` (works for both npm and cargo installations)
- **Codex CLI**: Updates via `npm update -g openai-codex`
- **Gemini CLI**: Updates via `npm update -g gemini-cli`

**Output example:**
```
Checking for updates...
✅ AIW updated successfully!
✅ Claude CLI updated (2.1.71 → 2.1.72)
✅ Codex CLI already up-to-date
⚠️  Gemini CLI not installed
```

## Configuration Files

| File | Purpose |
|------|---------|
| `~/.aiw/config.json` | AIW global configuration |
| `~/.aiw/providers.json` | AI provider configurations |
| `~/.aiw/role/*.md` | Custom role prompts |

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

## Available Roles

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

**AIW** - Unified Gateway for AI CLI | v0.5.74

[GitHub](https://github.com/putao520/agentic-warden) | [crates.io](https://crates.io/crates/aiw)
