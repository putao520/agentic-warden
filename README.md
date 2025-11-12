# Agentic-Warden

<div align="center">

![Version](https://img.shields.io/badge/version-5.0.1-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey?style=flat-square)

**Universal AI CLI Management and Coordination Platform**

</div>

## What is Agentic-Warden?

Agentic-Warden is a command-line tool that solves the chaos of managing multiple AI CLI assistants. It provides unified management, process tracking, and configuration synchronization for Claude, Codex, Gemini, and other AI CLI tools.

## Core Problem It Solves

- **Multi-AI Management**: Switch between different AI CLI tools without losing context
- **Provider Flexibility**: Use any API provider (OpenRouter, LiteLLM, etc.) without modifying AI CLI configurations
- **Process Coordination**: Track and manage multiple AI CLI processes running simultaneously
- **Configuration Sync**: Backup and restore AI CLI configurations across devices via Google Drive

## Key Features

### 🔧 **AI CLI Management**
- Launch and manage Claude, Codex, Gemini from a single interface
- Interactive mode: `agentic-warden claude`
- Task mode: `agentic-warden claude "write Python function"`
- Multi-CLI: `agentic-warden claude,codex "compare algorithms"`

### 🌐 **Provider Management**
- Switch API providers without configuration changes: `agentic-warden claude -p openrouter`
- Support for OpenRouter, LiteLLM, custom endpoints
- Environment variable injection for seamless provider switching

### 📊 **Process Tracking**
- Intelligent process tree identification
- Cross-process task coordination
- `wait` command: `agentic-warden wait` - monitor both CLI and MCP registries, exit when all tasks complete
- Process isolation and namespace management

### 🧠 **Memory & Semantic Search**
- Vector database integration with Qdrant for semantic conversation search
- Ollama embedding service for text vectorization
- Session-based conversation storage and retrieval
- MCP tools for memory operations:
  - `search_history`: Query conversation history with semantic similarity
  - `get_session_todos`: Query incomplete TODOs by session_id
- Automatic TODO extraction and session association

### ☁️ **Google Drive Integration**
- Backup configurations: `agentic-warden push`
- Restore configurations: `agentic-warden pull`
- Selective file packing (no cache/temp files)
- OAuth 2.0 Device Flow for headless environments

### 🛠️ **Utility Commands**
- `agentic-warden status` - Check AI CLI tools and versions
- `agentic-warden update` - Update/install AI CLI tools
- `agentic-warden tui` - Launch terminal interface
- `agentic-warden mcp` - Start MCP server for external integrations

## Installation

### From Cargo (Recommended)
```bash
cargo install agentic-warden
```

### From Source
```bash
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden
cargo install --path .
```

## Quick Start

### 1. Check AI CLI Status
```bash
agentic-warden status
```
Shows which AI CLI tools are installed and their versions.

### 2. Launch AI CLI with Default Provider
```bash
# Interactive mode
agentic-warden claude

# Task mode
agentic-warden codex "debug this Rust code"
```

### 3. Switch Providers
```bash
# Use OpenRouter with Claude
agentic-warden claude -p openrouter "write Python script"

# Use LiteLLM with multiple AI CLIs
agentic-warden claude,codex -p litellm "analyze data"
```

### 4. Manage Background Tasks
```bash
# Start multiple AI CLI tasks in background
agentic-warden codex "task 1" &
agentic-warden gemini "task 2" &
agentic-warden claude "task 3" &

# Wait for all tasks to complete
agentic-warden wait
```

### 5. Memory & Semantic Search
```bash
# Start MCP server for memory operations (run in background)
agentic-warden mcp &

# In Claude Code or other MCP client, search conversation history:
{
  "tool": "search_history",
  "arguments": {
    "query": "python programming best practices",
    "session_id": "session-123",
    "limit": 10
  }
}

# Query TODOs for a specific session:
{
  "tool": "get_session_todos",
  "arguments": {
    "session_id": "session-123",
    "status": "pending"
  }
}
```

### 6. Sync Configurations
```bash
# Backup to Google Drive
agentic-warden push

# Restore from Google Drive
agentic-warden pull
```

## Provider Setup

### Add Custom Provider
```bash
agentic-warden tui
# Navigate to Provider Management
# Add new provider with API keys and endpoints
```

### Example Provider Configuration
```json
{
  "openrouter": {
    "name": "OpenRouter",
    "compatible_with": ["claude", "codex", "gemini"],
    "env": {
      "OPENAI_API_KEY": "sk-or-v1-...",
      "OPENAI_API_BASE": "https://openrouter.ai/api/v1"
    }
  }
}
```

## Use Cases

### For Developers
- **Multi-Model Development**: Test the same prompt across different AI models
- **Provider Testing**: Compare different API providers for cost and quality
- **Task Management**: Run multiple AI tasks concurrently and track progress

### For Teams
- **Configuration Sync**: Share AI CLI configurations across team members
- **Provider Management**: Centralized API key and endpoint management
- **Process Monitoring**: Track team AI usage and task progress

### For Power Users
- **Batch Processing**: Run multiple AI tasks with different providers
- **Configuration Backup**: Never lose your AI CLI setups
- **Advanced Workflows**: Integrate with other tools via MCP server

## Advanced Features

### MCP (Model Context Protocol) Server
```bash
# Start MCP server for external integrations
agentic-warden mcp

# Use with Claude Code or other MCP clients
# Provides tools: monitor_processes, get_provider_status, start_ai_cli
```

### Process Wait Modes
```bash
# Wait for all AI CLI tasks
agentic-warden wait

# Wait for specific process
agentic-warden pwait <PID>

# Wait with timeout
agentic-warden wait --timeout 2h
```

### Update Management
```bash
# Update all AI CLI tools
agentic-warden update

# Update specific tool
agentic-warden update claude

# Install if not present
agentic-warden update gemini
```

## Requirements

- **Rust**: 1.70+ for building from source
- **OS**: Windows 10+, Linux, macOS 10.15+
- **AI CLI Tools**: Claude, Codex, Gemini (optional)
- **Node.js**: 14+ (for npm packages)

## Configuration

Configuration files are stored in `~/.agentic-warden/`:
- `provider.json` - Provider configurations
- `sync_state.json` - Google Drive sync state
- `oauth_tokens.json` - OAuth tokens (encrypted)

## Support

- **GitHub Issues**: [Report bugs and request features](https://github.com/putao520/agentic-warden/issues)
- **Documentation**: [Complete SPEC documentation](./SPEC/)
- **Discussions**: [Community discussions](https://github.com/putao520/agentic-warden/discussions)

## License

MIT License - see [LICENSE](LICENSE) file for details.

---

**Agentic-Warden** - Unified control for your AI CLI ecosystem.