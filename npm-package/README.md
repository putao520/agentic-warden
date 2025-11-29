# AIW - AI Workflow CLI

A high-performance AI workflow orchestration tool written in **pure Rust** with statically-linked Linux binaries.

**AIW** (Agentic Warden) is a unified CLI management tool for multi-account environments, providing intelligent process monitoring, task coordination, and AI integration.

## Installation

### Quick Install (Linux x86_64, ARM64, ARMv7)

```bash
npm install -g aiw
aiw --version
```

This package includes pre-compiled static binaries for Linux:
- **x86_64** - Intel/AMD 64-bit processors
- **arm64** - Apple Silicon, Raspberry Pi 4/5
- **armv7** - Raspberry Pi 3, older ARM devices

### Alternative Installation Methods

**Using Cargo (Rust):**
```bash
cargo install --git https://github.com/putao520/agentic-warden
```

**Download from GitHub Releases:**
Visit [Releases](https://github.com/putao520/agentic-warden/releases) for all platform binaries.

**Build from Source:**
```bash
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden
./build-in-docker.sh x86_64-unknown-linux-musl
# Binary at: target/x86_64-unknown-linux-musl/release/aiw
```

## Features

- 🚀 **Zero-Dependency Deployment** - Fully static Linux binaries
- 🔧 **AI Multi-Account Management** - Claude, OpenAI, Gemini, etc.
- ⚙️ **Provider Abstraction** - Unified interface for OpenRouter, LiteLLM, Cloudflare AI
- 📁 **Smart File Sync** - Intelligent blacklist for cloud sync
- 🎨 **Modern TUI Interface** - Interactive terminal UI with ratatui
- 🌐 **Cross-Platform** - Compiled for x86_64, ARM64, ARMv7 Linux

## Quick Start

```bash
# Show help
aiw --help

# Version info
aiw --version

# Interactive TUI
aiw

# Manage AI providers
aiw provider list
aiw provider add

# Sync files
aiw push
aiw pull
```

## Documentation

For detailed documentation, visit the [GitHub repository](https://github.com/putao520/agentic-warden).

## License

MIT
