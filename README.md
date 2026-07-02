# AIW - AI CLI Unified Gateway

[中文版](README_zh.md)

<div align="center">

![Version](https://img.shields.io/badge/version-0.5.99-blue?style=flat-square)
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
# Interactive mode - launch AI CLI directly
aiw claude    # Launch Claude CLI in interactive mode
aiw codex     # Launch Codex CLI in interactive mode
aiw gemini    # Launch Gemini CLI in interactive mode

# Non-interactive mode - with prompt
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

### Patch Management (File & Runtime)

AIW supports both persistent file patches and runtime memory patches for Claude Code, including **anti-spy / anti-telemetry** patches that blind CC's local environment detection and cut off client reporting to Anthropic.

```bash
# View patch status
aiw patch status

# Apply all patches (one-shot, recommended)
aiw patch apply --max-context-tokens 500000

# Restore original binary from backup
aiw patch restore
```

**Patch Types**:
- **File Patch**: Modifies the Claude CLI binary on disk (persistent across restarts, auto-backup to `.aiw-backup`)
- **Memory Patch**: Applied at runtime when file is unpatched (automatic, best-effort)

**Supported Installations**:
- Native binary (ELF/Mach-O): `~/.local/share/claude/versions/<version>`
- npm installation: `npm install -g @anthropic-ai/claude-code`

**Supported Versions** (cross-version via semantic regex / stable literals):

| Version | Linux x64 | macOS arm64 | Windows x64 |
|---------|-----------|-------------|-------------|
| 2.1.195 | ✅ | ✅ | ✅ |
| 2.1.196 | ✅ | ✅ | ✅ |
| 2.1.197 | ✅ | ✅ | ✅ |
| 2.1.198 | ✅ | ✅ | ✅ |

Run `aiw patch status` to check if your version is supported. Patches use **semantic regex** (wildcarding minified variable names like `Oe`/`Pe`, `g7`/`F7`/`j7`/`dX`) and **stable literals** (API paths, env var names), so they work across versions without a per-version signature database.

**Five Patch Layers** (anti-spy + capability unlock):

| Patch | What it does | Mechanism |
|-------|--------------|-----------|
| **MaxContextTokens** | Unlocks default context window + autoCompact threshold (200000 → configurable, e.g. 500000) | Regex `var \w+=200000,\w+=200000[^;]*;` (tolerates 4/5/6-element blocks) |
| **AntiTelemetry** | Cuts off CC client reporting (`/api/event_logging/v2/batch` → 404), machineID/userID/device fingerprint never sent | Literal `batch`→`xxxxx` (27B equal-length) |
| **AntiSpy (escape hatch)** | Patches `_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL` check so `fu()` always returns true — one-shot silences 30+ call sites: relay-station identity reporting (`custom_base_url` flag), attribution header discrimination (`cch=00000`), tool-set filtering, ToolSearch gating, model-override gating | Semantic regex `if(\w+._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0` → `if(1)` (55B equal-length) |
| **AntiSpy (timezone)** | Blinds timezone detection — `Intl.DateTimeFormat().resolvedOptions().timeZone` always returns `UTC`, real timezone never leaks | Literal → `"UTC"/*...*/` (48B equal-length) |
| **AntiPromptBias** | Eliminates Provider context prompt bias injected to 3rd-party users (`if(dX())n.push("**Provider context:**...")` → `if(0)`) | Semantic regex `if(\w+())` wildcards `g7`/`F7`/`j7`/`dX` (63B equal-length) |

> **Design note**: The escape-hatch patch is based on CC's official escape-hatch env var `_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL` (parsed by `st()` as truthy for `1`/`true`/`yes`/`on`). Patching the check itself (instead of injecting the env var) makes it permanent and covers more call sites. CC v2.1.198 removed the exposed `Hsp()` explicit probe (Asia/Shanghai timezone + base64 host list); identification fell back to `Cot()`/`fu()` host comparison, which the escape-hatch patch neutralizes.

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

## Patch Management

AIW includes a unified patching framework with **five anti-spy / capability-unlock patches** for Claude Code. All patches are cross-version stable (195-198) via semantic regex and stable literals — no per-version signature database needed.

### Available Patches

| Patch | Description | Scope |
|-------|-------------|-------|
| **MaxContextTokens** | Configurable default context window + autoCompact threshold (default 500000) | File + Memory |
| **AntiTelemetry** | Cuts off client reporting (`event_logging` endpoint → 404), fingerprint never sent | File + Memory |
| **AntiSpy** | Blinds local detection: escape-hatch short-circuit (`fu()`→true) + timezone→UTC | File + Memory |
| **AntiPromptBias** | Eliminates Provider context prompt bias for 3rd-party users | File + Memory |

### Patch Commands

```bash
# Show patch status
aiw patch status

# Apply all patches in one shot (recommended)
aiw patch apply --max-context-tokens 500000

# Apply individual patches
aiw patch set-max-tokens 500000        # max-token only
aiw patch disable-telemetry            # anti-telemetry only
aiw patch disable-spy                  # anti-spy only (escape-hatch + timezone)
aiw patch disable-prompt-bias          # anti-prompt-bias only

# Restore original binary from backup
aiw patch restore
```

### Cross-Version Stability

Patches use two mechanisms to stay stable across CC versions (2.1.195-198):
- **Semantic regex**: wildcard minified variable names (`Oe`/`Pe` config object, `g7`/`F7`/`j7`/`dX` condition function)
- **Stable literals**: API paths (`/api/event_logging/v2/batch`), env var names (`_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL`), `Intl.DateTimeFormat().resolvedOptions().timeZone`

All patches follow the **equal-length replacement iron law** (byte-level equal length, no offset shifting). See `SPEC/01-REQUIREMENTS.md` REQ-025~028 and `SPEC/CC-PROMPT-AUDIT.md` for full technical details.

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

[GitHub](https://github.com/putao520/agentic-warden) | [crates.io](https://crates.io/crates/aiw)


---

## For Developers

After cloning the repository, enable project-specific Git hooks:

```bash
git config core.hooksPath .githooks
```

This ensures commits follow project conventions (e.g., README must be updated when bumping version).
