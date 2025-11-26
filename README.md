# Agentic-Warden

<div align="center">

![Version](https://img.shields.io/badge/version-6.0.1-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![MCP](https://img.shields.io/badge/MCP-2024--11--05-purple?style=flat-square)

**Universal AI CLI Management Platform with Google Drive Sync**

</div>

Agentic-Warden is an intelligent platform for managing AI CLI tools (Claude, Codex, Gemini) with MCP routing, process tracking, and Google Drive configuration synchronization.

## 🎯 Features

### 1. AI CLI Management (`REQ-001`, `REQ-002`, `REQ-006`, `REQ-011`)
- **Process Tree Tracking**: Monitor AI CLI processes and their children
- **Provider Management**: Switch between AI providers (OpenAI, Anthropic, Google, etc.)
- **Capability Detection**: Auto-detect installed AI CLI tools
- **Wait Mode**: Block until AI CLI tasks complete
- **Update Management**: Install and update AI CLI tools

**Usage:**
```bash
# Launch AI CLI with process tracking
aiw claude "explain this code"
aiw codex "write a python script"
aiw gemini "translate to french"

# Wait for all AI CLI tasks to complete
aiw wait --timeout 300

# Manage providers
aiw provider list
aiw provider add openrouter "sk-xxx"
```

### 2. Google Drive Sync (`REQ-003`)
Synchronize AI CLI configurations across devices using Google Drive.

**Setup OAuth:**
```bash
export GOOGLE_CLIENT_ID="your_client_id.apps.googleusercontent.com"
export GOOGLE_CLIENT_SECRET="your_client_secret"
```

**Commands:**
```bash
# Push AI CLI configurations to Google Drive
aiw push [config_name]
# Example: aiw push claude
# Example: aiw push codex
# Example: aiw push gemini

# Pull configurations from Google Drive
aiw pull [config_name]

# List available backups on Google Drive
aiw list
```

**Note:** Currently supports only 3 predefined AI CLI directories:
- `~/.claude`
- `~/.codex`
- `~/.gemini`

**OAuth Flow:**
- Uses OAuth 2.0 Device Flow (RFC 8628)
- Authentication tokens stored in `~/.aiw/auth.json`
- Detailed setup instructions printed during first use

### 3. Intelligent MCP Routing (`REQ-012`, `REQ-013`)
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

# Configure in Claude Code (~/.config/claude-code/mcp.json)
{
  "mcpServers": {
    "agentic-warden": {
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

### 4. MCP Server Management (`REQ-007`)
Manage external MCP servers with hot-reload support.

**Commands:**
```bash
# Add MCP server
aiw mcp add filesystem npx --description "File operations" \
  -- -y @modelcontextprotocol/server-filesystem /home/user

# List servers
aiw mcp list

# Enable/disable servers
aiw mcp enable brave-search
aiw mcp disable filesystem

# Edit configuration
aiw mcp edit

# Hot-reload: Changes apply instantly without restart
```

**Configuration File:** `~/.aiw/.mcp.json`

## 🚀 Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden
cargo build --release

# Install binary
cargo install --path .
```

### Initial Setup

```bash
# Verify installation
aiw --version

# Check available AI CLI tools
aiw status

# Configure MCP servers (optional)
aiw mcp add filesystem npx -- -y @modelcontextprotocol/server-filesystem $HOME

# Set up Google Drive sync (optional)
export GOOGLE_CLIENT_ID="your_client_id.apps.googleusercontent.com"
export GOOGLE_CLIENT_SECRET="your_client_secret"
aiw push claude
```

## 📊 Feature Matrix

| Feature | REQ-ID | Status | Description |
|---------|--------|--------|-------------|
| Process Tree Tracking | 001 | ✅ | Monitor AI CLI processes |
| Provider Management | 002 | ✅ | Switch AI providers |
| Google Drive Sync | 003 | ✅ | Backup/sync configurations |
| Wait Mode | 005 | ✅ | Block until tasks complete |
| Tool Detection | 006 | ✅ | Auto-detect AI CLI tools |
| MCP Servers | 007 | ✅ | External MCP integration |
| Intelligent Routing | 012 | ✅ | LLM-based tool routing |
| JS Orchestration | 013 | ✅ | Auto-generate workflows |
| AI CLI Updates | 011 | ✅ | Install/update tools |
| Role System | 014 | ✅ | AI persona management |

**Overall Status**: ✅ All core features implemented and tested

## 🛠️ Configuration

### Environment Variables

```bash
# AI CLI Configuration
export CLI_TYPE=claude              # claude, codex, or gemini
export CLI_PROVIDER=llmlite         # Any provider from provider.json

# MCP Server Configuration
export MCP_CONFIG_PATH=~/.aiw/.mcp.json

# Google Drive OAuth
export GOOGLE_CLIENT_ID=xxx
export GOOGLE_CLIENT_SECRET=xxx

# LLM Routing (optional)
export OPENAI_TOKEN=sk-xxx          # For Ollama/local LLM mode
export OPENAI_ENDPOINT=http://localhost:11434
export OPENAI_MODEL=qwen2.5:7b
```

### Configuration Files

- **MCP Servers**: `~/.aiw/.mcp.json`
- **Providers**: `~/.aiw/providers.json`
- **AI CLI Auth**: `~/.claude/config.toml`, `~/.codex/config.json`, etc.
- **Google Drive Auth**: `~/.aiw/auth.json`

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test capability_detection
cargo test sync::smart_oauth
cargo test mcp_routing

# Build and check
cargo check
cargo build --release
```

**Test Coverage**: 131/131 unit tests passing

## 📖 Documentation

- **Requirements**: [SPEC/01-REQUIREMENTS.md](./SPEC/01-REQUIREMENTS.md)
- **Architecture**: [SPEC/02-ARCHITECTURE.md](./SPEC/02-ARCHITECTURE.md)
- **API Design**: [SPEC/04-API-DESIGN.md](./SPEC/04-API-DESIGN.md)
- **Testing Strategy**: [SPEC/06-TESTING-STRATEGY.md](./SPEC/06-TESTING-STRATEGY.md)

## 🤝 Contributing

Contributions welcome! Please:

1. Check [GitHub Issues](https://github.com/putao520/agentic-warden/issues)
2. Follow existing code style
3. Add tests for new features
4. Update documentation

## 📜 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **MCP Protocol**: [modelcontextprotocol.io](https://modelcontextprotocol.io)
- **Claude Code**: Anthropic's official CLI for Claude
- **Anthropic Code Execution**: Inspired by [Code Execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp)

---

**Agentic-Warden** - Universal AI CLI Management Platform v6.0.1
