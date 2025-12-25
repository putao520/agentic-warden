# Contributing to AIW Plugin Marketplace

Thank you for your interest in contributing to the AIW Official Plugin Marketplace!

## 🎯 Contribution Guidelines

### What We Accept

We accept **MCP (Model Context Protocol) server plugins** that:

1. **Provide MCP Functionality**: Plugin must include MCP server configuration
2. **Are Well-Documented**: Complete README with installation and usage
3. **Follow Standards**: Adhere to Claude Code plugin format
4. **Are Tested**: Plugin must be tested and working
5. **Have Open License**: MIT or other permissive license

### What We Don't Accept

- Plugins without MCP servers (commands-only, agents-only, etc.)
- Proprietary or closed-source plugins
- Plugins without documentation
- Broken or non-functional plugins

## 📝 Plugin Submission Process

### 1. Prepare Your Plugin

Create a plugin directory following the standard structure:

```
your-plugin/
├── .claude-plugin/
│   └── plugin.json
├── .mcp.json
└── README.md
```

### 2. Plugin Manifest

Create `.claude-plugin/plugin.json`:

```json
{
  "name": "your-plugin",
  "version": "1.0.0",
  "description": "Brief description of what your plugin does",
  "author": {
    "name": "Your Name",
    "email": "your@email.com"
  },
  "homepage": "https://github.com/your/your-plugin",
  "repository": "https://github.com/your/your-plugin",
  "license": "MIT",
  "keywords": ["mcp", "your-keywords"],
  "category": "development",
  "tags": ["mcp", "your-tags"],
  "strict": false
}
```

### 3. MCP Configuration

Create `.mcp.json`:

```json
{
  "mcpServers": {
    "your-server": {
      "command": "npx",
      "args": ["-y", "@your/package"],
      "env": {
        "REQUIRED_VAR": "${REQUIRED_VAR}"
      }
    }
  },
  "envVars": [
    {
      "name": "REQUIRED_VAR",
      "description": "Description of what this variable does",
      "required": true,
      "link": "https://where.to.get.token"
    }
  ]
}
```

### 4. Documentation

Create `README.md` with:

- Brief description
- Features list
- Prerequisites
- Installation instructions
- Environment variable setup
- Usage examples
- Link to official source repository

### 5. Update Marketplace

Add your plugin to `.claude-plugin/marketplace.json`:

```json
{
  "plugins": [
    {
      "name": "your-plugin",
      "source": "./plugins/your-plugin",
      "description": "Your plugin description",
      "version": "1.0.0",
      "author": {
        "name": "Your Name"
      },
      "category": "development",
      "tags": ["your-tags"],
      "strict": false
    }
  ]
}
```

### 6. Submit PR

1. Fork this repository
2. Create a new branch: `git checkout -b add-your-plugin`
3. Commit your changes: `git commit -am 'Add your-plugin'`
4. Push to branch: `git push origin add-your-plugin`
5. Create a Pull Request

## ✅ Review Process

We will review your submission based on:

- ✅ Completeness of plugin structure
- ✅ Quality of documentation
- ✅ Clarity of environment variables
- ✅ Working MCP server configuration
- ✅ adherence to standards

## 🎨 Plugin Categories

We organize plugins into categories:

- **development**: Development tools (git, github, etc.)
- **system**: System utilities (filesystem, etc.)
- **utilities**: Helper tools (search, api, etc.)
- **integration**: Third-party service integrations

Choose the most appropriate category for your plugin.

## 📋 Plugin Checklist

Before submitting, ensure:

- [ ] Plugin has `.claude-plugin/plugin.json`
- [ ] Plugin has `.mcp.json` with MCP server configuration
- [ ] Plugin has `README.md` with complete documentation
- [ ] Environment variables are documented with links
- [ ] Plugin is tested and working
- [ ] Plugin is added to `marketplace.json`
- [ ] License is specified

## 🆘 Getting Help

If you need help:

- Check existing plugins as examples
- Read the [Claude Code plugin documentation](https://code.claude.com/docs/en/plugin-marketplaces)
- Open an issue for questions
- Start a discussion for ideas

## 📜 Code of Conduct

Be respectful, inclusive, and constructive. We welcome contributors from all backgrounds and experience levels.

---

**Thank you for contributing to AIW Plugin Marketplace!** 🎉
