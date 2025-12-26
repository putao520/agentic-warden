# AIW - AI Workflow Orchestration Tool

<div align="center">

![Version](https://img.shields.io/badge/version-0.5.26-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![MCP](https://img.shields.io/badge/MCP-Supported-purple?style=flat-square)

**Universal AI CLI Management Platform with Intelligent MCP Routing & Transparent Parameter Forwarding**

</div>

AIW is an intelligent platform for managing AI CLI tools (Claude, Codex, Gemini) with MCP routing, process tracking, provider management, and transparent parameter forwarding.

## 🎯 Core Features

### 1. AI CLI Management
- **Process Tree Tracking**: Monitor AI CLI processes and their children
- **Provider Management**: Switch between AI providers (OpenAI, Anthropic, Google, etc.)
- **Transparent Parameter Forwarding**: Seamlessly pass all CLI parameters while using provider management
- **Capability Detection**: Auto-detect installed AI CLI tools
- **Wait Mode**: Block until AI CLI tasks complete

```bash
# Basic usage with provider management
aiw claude "explain this code"                              # Use default provider
aiw claude -p openrouter "explain this code"                 # Use OpenRouter provider
aiw claude -p glm "explain this code"                        # Use custom GLM provider

# 🆕 Transparent Parameter Forwarding (v0.5.23+)
aiw claude -p glm --model sonnet --debug api "explain this code"
aiw claude -p glm --print --output-format json "get structured response"
aiw codex -p glm --temperature 0.7 --max-tokens 1000 "generate text"

# Interactive mode with parameter forwarding
aiw claude -p glm --model sonnet --debug api               # Interactive session with custom parameters
aiw claude -p glm --print --allowed-tools Bash,Edit         # Interactive with tool restrictions

# Wait for all AI CLI tasks to complete
aiw wait

# Manage providers (TUI interface)
aiw provider
```

### 5. Update
- **AIW Self-Update**: Update AIW itself to latest version from NPM
- **AI CLI Tools Update**: Update all installed AI CLI tools (claude, codex, gemini)
- **Combined Update**: Update both AIW and AI CLI tools in one command

```bash
# Update both AIW and all AI CLI tools
aiw update

# Command will:
# 1. Check and update AIW to latest version
# 2. Check and update all installed AI CLI tools
# 3. Show detailed results for both updates
```

### 2. Intelligent MCP Routing ⭐
Route user requests to the best MCP tool with 98% token reduction.

**How it works:**
- **LLM ReAct Mode**: Uses LLM reasoning to analyze requests
- **Vector Search Mode**: Semantic similarity fallback
- **Dynamic Registration**: Registers tools on-demand (98% token savings)
- **JavaScript Orchestration**: Auto-generates workflows for complex tasks

**Usage:**
```bash
# Start MCP server
aiw mcp

# Configure in Claude Code (~/.claude/settings.json)
{
  "mcpServers": {
    "aiw": {
      "command": "aiw",
      "args": ["mcp"]
    }
  }
}

# Available MCP tools:
# - intelligent_route: Auto-route to best tool
# - get_method_schema: Get tool schemas
# - execute_tool: Execute tools with negotiation
```

### 3. MCP Server Management
Manage external MCP servers with hot-reload support.

```bash
# Add MCP server
aiw mcp add filesystem npx --description "File operations" \
  -- -y @modelcontextprotocol/server-filesystem /home/user

# List servers
aiw mcp list

# Enable/disable servers
aiw mcp enable brave-search
aiw mcp disable filesystem

# Hot-reload: Changes apply instantly without restart
```

### 4. MCP Registry CLI ⭐
Search and install MCP servers from multiple registries with multi-source aggregation.

**Supported Registries:**
- **Official MCP Registry** (registry.modelcontextprotocol.io)
- **Smithery** (registry.smithery.ai)

```bash
# Interactive browse all MCP servers (with fuzzy search)
aiw mcp browse

# Search MCP servers across all registries
aiw mcp search "filesystem"
aiw mcp search "git" --source official  # Search specific registry

# Get detailed server information
aiw mcp info @anthropic/filesystem

# Install MCP server with interactive setup
aiw mcp install @anthropic/filesystem

# Install with environment variables
aiw mcp install @anthropic/filesystem --env API_KEY=xxx

# Update registry cache
aiw mcp update
```

**Features:**
- **Interactive Browse**: Fuzzy search through all servers with ↑↓ navigation
- **Multi-source Aggregation**: Search across Official Registry + Smithery in parallel
- **Deduplication**: Same server from multiple sources shown once with best match
- **Interactive Install**: Configure environment variables during installation
- **Source Tracking**: Track where each server was installed from

### 5. Plugin Marketplace ⭐
Browse and install plugins from Claude Code-compatible plugin marketplaces.

**Default Marketplaces:**
- **Claude Code Official** (anthropics/claude-plugins-official)
- **AIW Official** (putao520/aiw-plugins)

```bash
# Browse MCP plugins interactively
aiw plugin browse

# Search plugins
aiw plugin search "git"
aiw plugin search "playwright" --market claude-code-official

# View plugin details
aiw plugin info playwright@claude-code-official

# Install plugin (interactive setup)
aiw plugin install playwright@claude-code-official

# Install with environment variables
aiw plugin install serena@claude-code-official --env MY_VAR=value

# List installed plugins
aiw plugin list

# Enable/disable plugins
aiw plugin enable playwright
aiw plugin disable serena

# Remove plugin
aiw plugin remove playwright
```

**Marketplace Management:**
```bash
# List marketplaces
aiw plugin marketplace list

# Add custom marketplace
aiw plugin marketplace add my-market https://github.com/user/marketplace

# Remove marketplace
aiw plugin marketplace remove my-market

# Update marketplace indexes
aiw plugin marketplace update
```

**Configuration Files:**
- `~/.aiw/settings.json`: Marketplace sources and plugin states
- `~/.aiw/plugins.json`: Installed plugin records
- `~/.aiw/mcp.json`: Extracted MCP server configurations

**Features:**
- **MCP-Only Filtering**: Only shows plugins with MCP servers (mcpServers field)
- **Claude Code Compatible**: Uses same plugin format as Claude Code
- **JSON Configuration**: Modern JSON format (automatic YAML migration)
- **Interactive Setup**: Configure environment variables during installation
- **Hot-Reload**: Changes apply instantly without restart
- **Transport Type Support**: Supports stdio transport (local executables). HTTP/SSE transports coming soon.

**Environment Variables:**
Plugins can use environment variable placeholders in their MCP configuration:
```json
{
  "env": {
    "API_KEY": "${API_KEY}",
    "GITHUB_TOKEN": "${GITHUB_TOKEN}"
  }
}
```
During installation, AIW will prompt for values or you can provide them via `--env` flag. Placeholders are preserved in the config and expanded at runtime.

## 🔄 Transparent Parameter Forwarding (v0.5.23+)

AIW now supports transparent parameter forwarding, allowing you to use **all** AI CLI features while maintaining provider management capabilities.

### How It Works

```bash
# Parameter parsing logic:
aiw <ai_type> -p <provider> <cli_params...> <prompt>
#           ^^^^^^^^  ^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^
#           AIW args    AIW args       Forward to AI CLI
```

### Rules

- **AIW Consumes**: `-r` / `--role` for role injection, `-p` / `--provider` for provider selection
- **Transparent Forwarding**: All other `-` prefixed parameters are passed directly to the AI CLI
- **Parameter Order**: AIW flags (-r, -p) must come **before** other CLI parameters
- **Full Compatibility**: Maintain complete access to all AI CLI features

### Examples

#### Task Mode
```bash
# Model selection with debugging
aiw claude -p glm --model sonnet --debug api "explain this code"

# Structured output
aiw claude -p glm --print --output-format json "summarize the file"

# Tool restrictions
aiw claude -p glm --allowed-tools Bash,Edit "modify this file"

# Multiple parameters
aiw claude -p glm --model sonnet --max-budget-usd 5 --dangerously-skip-permissions "help me debug"
```

#### Interactive Mode
```bash
# Interactive session with custom model and debugging
aiw claude -p glm --model sonnet --debug api

# Interactive with output formatting
aiw claude -p glm --print --output-format stream-json

# Interactive with specific tools
aiw claude -p glm --tools "Bash,Edit,Read" --no-session-persistence

# Multi-AI with same provider
aiw "claude|codex" -p glm --temperature 0.7 "compare approaches"
```

#### Provider-Specific Examples
```bash
# Claude with structured output
aiw claude -p glm --json-schema '{"type":"object","properties":{"summary":{"type":"string"}}}' "summarize this"

# Codex with custom settings
aiw codex -p glm --temperature 0.3 --max-tokens 500 "write python function"

# Gemini with approval mode
aiw gemini -p glm --approval-mode yolo "translate this text"
```

### Benefits

✅ **Full CLI Access**: Use all AI CLI parameters without limitations
✅ **Provider Flexibility**: Switch providers without changing commands
✅ **Process Tracking**: Maintain AIW's monitoring and task management
✅ **Environment Injection**: Automatic provider configuration injection
✅ **Zero Learning Curve**: Works exactly like native AI CLI with provider prefix

---

## 🚀 Quick Start

### Installation
```bash
# Install from NPM
npm install -g @putao520/aiw

# Or use npx directly
npx @putao520/aiw --help
```

### Initial Setup
```bash
# Verify installation
aiw --version

# Check available AI CLI tools
aiw status

# Configure MCP servers
aiw mcp add filesystem npx -- -y @modelcontextprotocol/server-filesystem $HOME
```

## 📖 Documentation

- **[MCP Usage Guide](./docs/MCP_GUIDE.md)** - Detailed MCP configuration and usage
- **[GitHub Repository](https://github.com/putao520/agentic-warden)** - Full source code and issues

## 🛠️ Configuration

### Environment Variables
```bash
# AI CLI Configuration
export CLI_TYPE=claude              # claude, codex, or gemini
export CLI_PROVIDER=llmlite         # Any provider from provider.json

# MCP Server Configuration
export MCP_CONFIG_PATH=~/.aiw/mcp.json
```

### Configuration Files
- **MCP Servers**: `~/.aiw/mcp.json` (standard `mcpServers` schema compatible with Claude Code)
  - Claude Code users: Add to `~/.claude/settings.json` under the "mcpServers" section
- **Providers**: `~/.aiw/providers.json`

## 🙏 Acknowledgments

- **Claude Code**: Anthropic's official CLI for Claude
- **Anthropic Code Execution**: Inspired by [Code Execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp)

## 📜 License

MIT License - see [LICENSE](LICENSE) file for details.

---

**AIW** - Universal AI CLI Management Platform with MCP Routing v0.5.26

For full documentation and source code, visit: [https://github.com/putao520/agentic-warden](https://github.com/putao520/agentic-warden)
