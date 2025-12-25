# Git MCP Server

Git operations server for Model Context Protocol.

## Features

- Git status
- Commit operations
- Branch management
- Diff viewing
- Log history

## Prerequisites

- Git installed
- A git repository
- Node.js and npm installed

## Installation

```bash
aiw plugin install git-mcp@aiw-official
```

Optionally specify a repository path:

```bash
export AIW_GIT_REPO=/path/to/repo
aiw plugin install git-mcp@aiw-official
```

## Usage

The Git MCP server provides comprehensive git operations:

### Available Tools

- **git_status**: Show the working tree status
- **commit**: Create a new commit
- **create_branch**: Create a new branch
- **switch_branch**: Switch to a different branch
- **list_branches**: List all branches
- **read_file**: Read a file from the git repository
- **diff**: Show changes between commits
- **log**: Show commit logs
- **remotes**: List remote repositories
- **commits**: List commits in a branch

### Example Workflow

```bash
# Check git status
mcp_call("git_status", {})

# Create a new feature branch
mcp_call("create_branch", {"branch": "feature/new-feature"})

# Make changes and commit
mcp_call("commit", {
  "message": "Add new feature",
  "files": ["src/main.rs"]
})
```

## Configuration

```json
{
  "mcpServers": {
    "git": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-git",
        "."
      ]
    }
  }
}
```

## Official Source

- **Repository**: https://github.com/modelcontextprotocol/servers
- **Documentation**: https://github.com/modelcontextprotocol/servers/tree/main/src/git

## License

MIT
