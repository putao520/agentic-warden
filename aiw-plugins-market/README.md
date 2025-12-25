# AIW Official Plugin Marketplace

Official plugin marketplace for **AIW (Agentic Warden)** - the universal AI CLI management platform with intelligent MCP routing.

## 📦 About

This marketplace contains **MCP (Model Context Protocol) server plugins** that are compatible with both AIW and Claude Code. All plugins in this marketplace are curated and tested to work seamlessly with AIW's plugin management system.

## 🚀 Quick Start

### Add this marketplace to AIW

```bash
aiw plugin marketplace add putao520/aiw-plugins
```

### Browse available plugins

```bash
aiw plugin browse
```

### Install a plugin

```bash
# Interactive installation with environment variable setup
aiw plugin install github-mcp@aiw-official

# Or search and install interactively
aiw plugin search github
```

## 📚 Available Plugins

### Development Tools

| Plugin | Description | Category |
|--------|-------------|----------|
| **github-mcp** | GitHub integration - repositories, issues, PRs | development |
| **git-mcp** | Git operations - status, commits, branches | development |

### System Utilities

| Plugin | Description | Category |
|--------|-------------|----------|
| **filesystem-mcp** | File system operations - read, write, manage files | system |

### Web & API

| Plugin | Description | Category |
|--------|-------------|----------|
| **brave-search-mcp** | Web search via Brave Search API | utilities |

## 📖 Plugin Format

All plugins in this marketplace follow the Claude Code plugin format standard:

```
plugin-name/
├── .claude-plugin/
│   └── plugin.json          # Plugin manifest
├── .mcp.json                # MCP server configuration
└── README.md                # Documentation
```

## 🔧 Compatibility

### AIW (Primary Target)

AIW automatically filters and installs only the MCP components from these plugins:

```bash
aiw plugin install github-mcp@aiw-official
# → Extracts MCP configuration
# → Writes to ~/.aiw/mcp.json
# → Skips commands/agents/skills/hooks
```

### Claude Code (Full Support)

These plugins are also fully compatible with Claude Code:

```bash
/plugin install github-mcp@putao520/aiw-plugins
```

## 🛠️ Creating Plugins

Want to contribute your own MCP plugin? Follow these steps:

### 1. Plugin Structure

Create a new directory in `plugins/`:

```
plugins/your-mcp-plugin/
├── .claude-plugin/
│   └── plugin.json
├── .mcp.json
└── README.md
```

### 2. Plugin Manifest

```json
{
  "name": "your-mcp-plugin",
  "version": "1.0.0",
  "description": "Brief description",
  "author": {
    "name": "Your Name"
  },
  "mcpServers": "./.mcp.json"
}
```

### 3. MCP Configuration

```json
{
  "mcpServers": {
    "your-server": {
      "command": "npx",
      "args": ["-y", "@your/package"],
      "env": {
        "API_KEY": "${API_KEY}"
      }
    }
  }
}
```

### 4. Update Marketplace

Add your plugin to `.claude-plugin/marketplace.json`:

```json
{
  "plugins": [
    {
      "name": "your-mcp-plugin",
      "source": "./plugins/your-mcp-plugin",
      "description": "Your plugin description",
      "category": "development",
      "strict": false
    }
  ]
}
```

### 5. Submit PR

Create a pull request to this repository with your plugin.

## 📝 Requirements for Plugins

- **MCP Focus**: All plugins must provide MCP server functionality
- **Documentation**: Complete README with installation and usage instructions
- **Environment Variables**: Clearly document all required environment variables
- **License**: Specify an open-source license (MIT recommended)
- **Quality**: Plugins must be tested and working

## 🔗 Official MCP Servers

Most plugins in this marketplace wrap the official MCP servers from [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers). If you want to add a new official server:

1. Check the [official servers list](https://github.com/modelcontextprotocol/servers)
2. Create a plugin wrapper following the existing patterns
3. Submit a PR

## 📄 License

All plugins in this marketplace are released under the MIT License unless otherwise specified in the plugin's own `plugin.json`.

## 🙏 Acknowledgments

- [Model Context Protocol](https://modelcontextprotocol.io/)
- [Anthropic Claude Code](https://code.claude.com/)
- [Official MCP Servers](https://github.com/modelcontextprotocol/servers)

## 📮 Contact

- **Maintainer**: Putao520
- **Issues**: [GitHub Issues](https://github.com/putao520/aiw-plugins/issues)
- **Discussions**: [GitHub Discussions](https://github.com/putao520/aiw-plugins/discussions)

---

**AIW Plugin Marketplace** - Curated MCP plugins for AI CLI management
