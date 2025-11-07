# Agentic-Warden

<div align="center">

![Agentic-Warden Logo](https://img.shields.io/badge/Agentic--Warden-0.3.0-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey?style=flat-square)

**Unified AI CLI Management and Process Monitoring Platform**

</div>

## 📋 Overview

Agentic-Warden is a unified management tool designed for multi-AI environments, providing intelligent process monitoring, configuration management, and Google Drive integration.

### 🎯 Key Features

- **🚀 Intelligent Process Tree Monitoring**: Automatically identifies AI CLI root processes, avoiding the traditional issue where all processes are attributed to system processes
- **🔧 Unified AI CLI Management**: Support for unified startup and management of multiple AI CLI tools like codex, claude, and gemini
- **⚙️ Provider Management**: Centralized management of third-party API providers (OpenRouter, LiteLLM, Cloudflare AI Gateway)
- **📁 Google Drive Integration**: Cloud sync and backup of configuration files
- **🎨 Modern TUI Interface**: Terminal user interface based on ratatui

## 🚀 Quick Start

### Installation

```bash
# Build from source
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden
cargo build --release
cargo install --path .
```

### Basic Usage

```bash
# Launch TUI interface
agentic-warden

# Check Provider status
agentic-warden status

# Manage Provider configuration
agentic-warden provider

# Upload configuration to Google Drive
agentic-warden push

# Download configuration from Google Drive
agentic-warden pull
```

### AI CLI Startup

```bash
# Launch Claude
agentic-warden claude "Please help me analyze this code"

# Launch Codex
agentic-warden codex "Write a Python script"

# Launch Gemini
agentic-warden gemini "Explain this algorithm"

# Launch multiple AI CLI simultaneously
agentic-warden claude|codex "Compare two implementation approaches"
```

## 📖 Documentation

### Core Features

- [**Process Tree Monitoring**](SPEC/OVERVIEW.md#intelligent-process-tree-monitoring) - Intelligent identification and monitoring of AI CLI processes
- [**Provider Management**](SPEC/CONFIGURATION.md) - Third-party API provider configuration and management
- [**Google Drive Integration**](SPEC/DEPLOYMENT.md) - Cloud sync of configuration files

### Development Documentation

- [**Architecture Design**](SPEC/ARCHITECTURE.md) - System architecture and design patterns
- [**API Documentation**](SPEC/API.md) - CLI commands and interface definitions
- [**Data Models**](SPEC/DATA_MODEL.md) - Core data structure definitions
- [**Module Organization**](SPEC/MODULES.md) - Code module organization structure
- [**Testing Strategy**](SPEC/TESTING.md) - Testing architecture and coverage strategy

## 🛠️ Technical Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Manager    │    │   Task Registry  │    │  TUI Framework   │
│                 │    │                 │    │                 │
│ • CLI Detection │    │ • Shared Memory  │    │ • ratatui UI     │
│ • Tool Management│    │ • Process Tracking│    │ • Event Handling │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Agentic-Warden Core                        │
│                                                                     │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │ Process Tree │  │ Supervisor  │  │ Provider   │  │ Sync Engine │   │
│  │   Manager   │  │   Engine    │  │  Manager    │  │            │   │
│  └─────────────┘  └──────────────┘  └─────────────┘  └─────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Language**: Rust 1.70+
- **TUI Framework**: ratatui (0.24+) + crossterm (0.27+)
- **Process Monitoring**: sysinfo + custom process tree analysis
- **Shared Memory**: shared_hashmap + shared_memory
- **Configuration Management**: serde + JSON
- **Network Requests**: reqwest + tokio
- **Google Drive**: yup-oauth2 + Google Drive API

## 🔧 Configuration

### Provider Configuration

Configuration file location: `~/.agentic-warden/providers.json`

```json
{
  "default_provider": "openrouter",
  "providers": {
    "openrouter": {
      "name": "OpenRouter",
      "description": "OpenRouter API Gateway",
      "icon": "🌐",
      "website": "https://openrouter.ai",
      "modes": {
        "claude": {
          "type": "anthropic",
          "model": "claude-3-5-sonnet-20241022",
          "regions": {
            "us": {
              "endpoint": "https://openrouter.ai/api/v1/chat/completions",
              "auth_env": "OPENROUTER_API_KEY"
            }
          }
        }
      }
    }
  }
}
```

### Environment Variables

```bash
# Claude API
export ANTHROPIC_API_KEY="your-claude-api-key"

# OpenRouter API
export OPENROUTER_API_KEY="your-openrouter-api-key"

# Google Drive (for push/pull)
export GOOGLE_DRIVE_CLIENT_ID="your-client-id"
export GOOGLE_DRIVE_CLIENT_SECRET="your-client-secret"
```

## 📊 Performance Metrics

### Test Coverage

- **Unit Tests**: 69 tests, 90%+ coverage
- **Integration Tests**: 8 tests, completed in 2.21s (fixed from >60s)
- **Concurrent Performance**: Supports 8-thread concurrent testing, no deadlock issues

### Resource Usage

- **Memory Usage**: Base runtime < 100MB
- **Startup Time**: Dashboard startup < 2s
- **Responsiveness**: TUI operation response < 100ms
- **Concurrent Support**: Monitor 100+ AI CLI processes simultaneously

## 🤝 Contributing

We welcome community contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone repository
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden

# Install development dependencies
cargo build

# Run tests
cargo test

# Run specific tests
cargo test --test ai_cli_real_failure

# Check code formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings
```

### Commit Guidelines

We use [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
feat: new feature
fix: bug fix
docs: documentation update
style: code formatting
refactor: code refactoring
test: testing related
chore: build process or auxiliary tool changes
```

## 🐛 Troubleshooting

### Common Issues

#### Q: AI CLI tools not found
**A**: Ensure the AI CLI tools are installed and environment variables are configured:

```bash
# Set Claude
export CLAUDE_BIN="/path/to/claude"

# Set Codex
export CODEX_BIN="/path/to/codex"

# Set Gemini
export GEMINI_BIN="/path/to/gemini"
```

#### Q: Google Drive authorization failed
**A**: Check network connection and API key configuration, ensure proper access permissions.

#### Q: TUI display issues
**A**: Ensure terminal supports ANSI colors and Unicode characters.

### Logging and Debugging

```bash
# Enable debug logging
export RUST_LOG=debug

# Run debug mode
agentic-warden --verbose
```

## 📄 License

This project is licensed under the [MIT License](LICENSE).

## 🔗 Related Links

- **GitHub Repository**: https://github.com/putao520/agentic-warden
- **Documentation Site**: https://docs.agentic-warden.dev
- **Issue Reporting**: [GitHub Issues](https://github.com/putao520/agentic-warden/issues)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)

---

<div align="center">

**Made with ❤️ by the Agentic-Warden Team**

</div>