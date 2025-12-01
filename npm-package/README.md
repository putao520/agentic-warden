# AIW - AI Workflow Orchestration Tool

## Features

### 🔧 AI Multi-Account Management
- Unified management for Claude, OpenAI, Gemini, and other AI accounts
- Support for OpenRouter, LiteLLM, Cloudflare AI and other providers
- Account switching and configuration management

### 📁 Smart File Synchronization
- Intelligent blacklist mechanism for file filtering
- Push and pull operations support
- Automatic cloud storage synchronization

### 🎨 Modern Terminal Interface
- Interactive TUI interface
- Real-time status monitoring
- Visualized operation panel

### 🌐 Cross-Platform Support
- Linux x64 / Windows x64
- Zero-dependency deployment

## Quick Start

### Installation
```bash
npm install -g aiw
```

### Basic Usage
```bash
# Show help
aiw --help

# Check version
aiw --version

# Launch interactive interface
aiw
```

### AI Account Management
```bash
# List providers
aiw provider list

# Add account
aiw provider add

# Remove account
aiw provider remove
```

### File Synchronization
```bash
# Push files to cloud
aiw push

# Pull files from cloud
aiw pull

# Check sync status
aiw status
```

## License

MIT