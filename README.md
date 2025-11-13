# AIW (AI Warden)

<div align="center">

![Version](https://img.shields.io/badge/version-5.0.2-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey?style=flat-square)

**Universal AI CLI Management Platform**

</div>

AIW is a unified AI CLI management tool that supports Claude, Codex, Gemini, and other AI assistants with intelligent process tracking and configuration synchronization.

## 🎯 Core Features

- **Multi-AI Management**: Launch and manage different AI CLI tools from a single interface
- **Flexible Providers**: Seamlessly switch between API providers (OpenRouter, LiteLLM, etc.)
- **Process Tracking**: Intelligently identify and track AI CLI process trees
- **Configuration Sync**: Backup and restore configurations via Google Drive

## 🚀 Quick Start

### Installation

```bash
cargo install aiw
```

### Basic Usage

```bash
# Check AI CLI status
aiw status

# Start Claude (interactive mode)
aiw claude

# Execute specific task
aiw codex "write a Python function"

# Specify provider
aiw claude -p openrouter "analyze this code"

# Multi-AI collaboration
aiw claude,codex "compare two algorithms"
```

### Background Task Management

```bash
# Start multiple background tasks
aiw claude "task1" &
aiw gemini "task2" &

# Wait for all tasks to complete
aiw wait
```

## 📋 Working Modes

### CLI Mode (Daily Use)

Direct command-line usage for everyday AI tasks:

```bash
# Interactive chat
aiw claude

# One-time tasks
aiw codex "fix this bug"
aiw gemini "write test cases"

# Provider switching
aiw claude -p litellm "code review"
```

### MCP Mode (Integration Development)

Add AIW as MCP server to Claude Code:

```bash
# Add AIW to Claude Code
claude-code mcp add aiw "aiw mcp"

# Or add with custom name
claude-code mcp add my-ai-assistant "aiw mcp"
```

Once added, AIW tools will be available in your Claude Code conversations for advanced AI task management and coordination.

## ⚙️ Configuration Management

### TUI Interface (Recommended)

```bash
# Launch graphical configuration interface
aiw tui
```

### Configuration File

Configuration location: `~/.aiw/provider.json`

```json
{
  "openrouter": {
    "name": "OpenRouter",
    "env": {
      "OPENAI_API_KEY": "your-key",
      "OPENAI_API_BASE": "https://openrouter.ai/api/v1"
    }
  }
}
```

### Configuration Sync

```bash
# Backup to Google Drive
aiw push

# Restore from Google Drive
aiw pull
```

## 🔧 Advanced Features

### Process Management

```bash
# View detailed status
aiw status --tui

# Wait for specific process
aiw pwait <PID>

# Update AI CLI tools
aiw update
```

### Memory & Search (MCP Mode)

Advanced memory operations available through MCP integration with semantic search and conversation history tracking.

## 📖 Documentation

Complete technical documentation: [SPEC/](./SPEC/) directory

- [Architecture Design](./SPEC/ARCHITECTURE.md)
- [API Documentation](./SPEC/API.md)
- [Configuration Guide](./SPEC/CONFIGURATION.md)

## 🤝 Support

- **Bug Reports**: [GitHub Issues](https://github.com/putao520/agentic-warden/issues)
- **Community Discussions**: [GitHub Discussions](https://github.com/putao520/agentic-warden/discussions)

## 📜 License

MIT License - see [LICENSE](LICENSE) file for details

---

**AIW** - Making AI CLI Management Simple

*(Formerly known as Agentic-Warden)*