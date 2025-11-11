# Agentic Warden - Placeholder

This is a **placeholder NPM package** for the [Agentic Warden](https://github.com/putao520/agentic-warden) project.

## What is Agentic Warden?

A unified AI CLI management tool for multi-account environments, providing intelligent process monitoring, task coordination, and Google Drive integration.

## Installation

Since this is a placeholder package, it doesn't contain the actual binary. Please install agentic-warden using one of these methods:

### Method 1: Using Cargo (Rust Package Manager)

```bash
cargo install --git https://github.com/putao520/agentic-warden
```

### Method 2: Download Pre-built Binaries

Visit the [GitHub Releases](https://github.com/putao520/agentic-warden/releases) page to download the appropriate binary for your platform (Linux, macOS, Windows).

### Method 3: Build from Source

```bash
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden
cargo build --release
```

The binary will be available at `target/release/agentic-warden`.

## Features

- 🚀 **Intelligent Process Tree Monitoring**
- 🔧 **Multi-AI CLI Management** (Claude, Codex, Gemini)
- ⚙️ **Provider Management** (OpenRouter, LiteLLM, Cloudflare AI)
- 📁 **Google Drive Integration**
- 🎨 **Modern TUI Interface** (ratatui)

## Quick Start

After installing:

```bash
# Launch TUI interface
agentic-warden

# Check status
agentic-warden status

# Manage providers
agentic-warden provider

# Sync with Google Drive
agentic-warden push
agentic-warden pull
```

## Documentation

For detailed documentation, visit the [GitHub repository](https://github.com/putao520/agentic-warden).

## License

MIT
