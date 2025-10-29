# agentic-warden

<div align="center">

![agentic-warden logo](https://img.shields.io/badge/agentic--warden-v0.3.0-blue.svg)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()

**🤖 Universal AI Agent Manager**

Unified interface for Claude, Codex, Gemini and more with batch execution, process management, and cloud sync capabilities.

</div>

## ✨ Features

- 🎯 **Unified Agent Interface** - Single syntax to command multiple AI agents
- 🚀 **Batch Execution** - Send tasks to multiple agents simultaneously
- 📊 **Process Tree Management** - Cross-platform process monitoring and task tracking
- ☁️ **Cloud Configuration Sync** - Google Drive integrated backup and sync
- 🔒 **OAuth 2.0 Authentication** - Secure cloud access
- 📦 **Cross-platform Compression** - TAR.GZ, ZIP, 7Z format support
- 🧹 **Task Monitoring** - Automatic cleanup and status management

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden

# Build the project
cargo build --release

# Add to PATH (optional)
export PATH=$PATH:$(pwd)/target/release
```

### Basic Usage

#### Single Agent Commands

```bash
# Use Claude to write code
agentic-warden claude "write a Rust quicksort algorithm"

# Use Codex to generate code
agentic-warden codex "generate Python data visualization script"

# Use Gemini to explain concepts
agentic-warden gemini "explain microservices architecture"
```

#### Batch Agent Commands

```bash
# Send task to all agents
agentic-warden all "review this code and suggest improvements"

# Send task to specific agent combinations
agentic-warden "claude|gemini" "compare these two programming approaches"
agentic-warden "codex|claude" "write documentation for this project"
```

#### Task Management

```bash
# Monitor task execution status
agentic-warden wait

# Launch CLI management interface
agentic-warden
```

## 📖 Supported AI Agents

| AI Assistant | Command | Description |
|--------------|---------|-------------|
| **Claude** | `claude` | Anthropic Claude coding assistant |
| **Codex** | `codex` | OpenAI Codex code generation tool |
| **Gemini** | `gemini` | Google Gemini AI assistant |

### Agent Selector Syntax

```bash
# Single agent
agentic-warden claude "task description"

# All agents
agentic-warden all "task description"

# Agent combinations (requires quotes)
agentic-warden "claude|gemini" "task description"
agentic-warden "claude|codex|gemini" "task description"
```

## 📋 Command Reference

### Agent Task Execution

```bash
agentic-warden <AGENT_SELECTOR> "<TASK_DESCRIPTION>"
```

**Parameters**:
- `AGENT_SELECTOR`: Agent selector (`claude`, `codex`, `gemini`, `all`, `"agent1|agent2"`)
- `TASK_DESCRIPTION`: Task description text

**Examples**:
```bash
agentic-warden claude "implement a binary search tree"
agentic-warden all "review this rust code"
agentic-warden "claude|gemini" "explain quantum computing"
```

### Configuration Sync Commands

```bash
# Push configuration to cloud storage
agentic-warden push [directory...]

# Pull configuration from cloud storage
agentic-warden pull [directory...]

# View sync status
agentic-warden status

# Reset sync state
agentic-warden reset

# List syncable directories
agentic-warden list
```

### Task Management Commands

```bash
# Wait for tasks to complete and show status
agentic-warden wait

# Launch CLI management interface (no arguments)
agentic-warden
```

## 🔧 Configuration

### Environment Variables

```bash
# Custom agent paths
export CLAUDE_BIN="/path/to/claude"
export CODEX_BIN="/path/to/codex"
export GEMINI_BIN="/path/to/gemini"

# Configuration directory
export AGENTIC_WARDEN_CONFIG_DIR="/custom/config/dir"
```

### Setting up AI Agents

Before using `agentic-warden`, make sure you have the following AI CLI tools installed and accessible:

1. **Claude CLI**: Install from Anthropic or ensure `claude` is in your PATH
2. **Codex CLI**: Install from OpenAI or ensure `codex` is in your PATH  
3. **Gemini CLI**: Install from Google or ensure `gemini` is in your PATH

### Cloud Sync Setup (Optional)

For configuration synchronization:

1. Create OAuth 2.0 credentials in Google Cloud Console
2. Run `agentic-warden status` to initiate authentication flow
3. Follow the browser authentication process

## 🛠️ Development

### Requirements

- Rust 1.70+
- Supported platforms: Windows, Linux, macOS

### Development Setup

```bash
# Clone repository
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden

# Install dependencies
cargo build

# Run tests
cargo test

# Code formatting
cargo fmt

# Code linting
cargo clippy

# Generate documentation
cargo doc --open
```

## 🐛 Troubleshooting

### Common Issues

**Q: Agent CLI not found error**
```bash
Error: 'claude' not found in PATH
```
A: Set environment variables or ensure CLI is in PATH:
```bash
export CLAUDE_BIN="/path/to/claude"
```

**Q: Cloud authentication failed**
```bash
Error: Invalid credentials
```
A: Re-authenticate with `agentic-warden status` or check your OAuth setup

**Q: Shared memory errors**
```bash
Error: Failed to connect to shared memory
```
A: Restart your system or clear shared memory segments

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug agentic-warden claude "test task"
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

### Contribution Process

1. Fork the project
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Claude](https://claude.ai) - AI coding assistant
- [OpenAI Codex](https://openai.com/) - Code generation AI
- [Google Gemini](https://gemini.google.com/) - Multimodal AI assistant
- Rust community - Excellent systems programming language and ecosystem

## 📞 Contact

- Project homepage: [https://github.com/putao520/agentic-warden](https://github.com/putao520/agentic-warden)
- Bug reports: [Issues](https://github.com/putao520/agentic-warden/issues)
- Feature requests: [Discussions](https://github.com/putao520/agentic-warden/discussions)

---

<div align="center">

**⭐ If this project helps you, please give it a star!**

Made with ❤️ by the agentic-warden team

</div>