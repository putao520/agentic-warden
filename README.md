# AIW - AI Workflow Orchestration Tool

<div align="center">

![Version](https://img.shields.io/badge/version-0.5.16-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![MCP](https://img.shields.io/badge/MCP-Supported-purple?style=flat-square)

**Universal AI CLI Management Platform with Intelligent MCP Routing**

</div>

AIW is an intelligent platform for managing AI CLI tools (Claude, Codex, Gemini) with MCP routing, process tracking, and configuration synchronization.

## 🎯 Core Features

### 1. AI CLI Management
- **Process Tree Tracking**: Monitor AI CLI processes and their children
- **Provider Management**: Switch between AI providers (OpenAI, Anthropic, Google, etc.)
- **Capability Detection**: Auto-detect installed AI CLI tools
- **Wait Mode**: Block until AI CLI tasks complete

```bash
# Launch AI CLI with process tracking
aiw claude "explain this code"
aiw codex "write a python script"
aiw gemini "translate to french"

# Wait for all AI CLI tasks to complete
aiw wait

# Manage providers
aiw provider list
aiw provider add openrouter "sk-xxx"
```

### 5. Update (更新AIW和AI CLI工具)
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

# Configure in Claude Code (~/.config/claude-code/mcp.json)
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

### 4. Google Drive Sync (Optional)
Synchronize AI CLI configurations across devices using Google Drive.

```bash
# Push configurations to Google Drive
aiw push           # uses the default configuration set

# Pull configurations from Google Drive
aiw pull           # restores the default configuration set

# List available backups
aiw list
```

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

# Google Drive OAuth (optional)
export GOOGLE_CLIENT_ID=xxx
export GOOGLE_CLIENT_SECRET=xxx
```

### Configuration Files
- **MCP Servers**: `~/.aiw/mcp.json` (standard `mcpServers` schema compatible with Claude Code/Cursor)
- **Providers**: `~/.aiw/providers.json`
- **Google Drive Auth**: `~/.aiw/auth.json`

## 🙏 Acknowledgments

- **Claude Code**: Anthropic's official CLI for Claude
- **Anthropic Code Execution**: Inspired by [Code Execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp)

## 📜 License

MIT License - see [LICENSE](LICENSE) file for details.

---

**AIW** - Universal AI CLI Management Platform with MCP Routing v0.5.16

For full documentation and source code, visit: [https://github.com/putao520/agentic-warden](https://github.com/putao520/agentic-warden)
