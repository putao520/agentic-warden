# GitHub MCP Server

GitHub integration server for Model Context Protocol.

## Features

- Repository management
- Issue tracking
- Pull request operations
- File operations
- Branch management

## Prerequisites

- GitHub personal access token with `repo` permissions
- Node.js and npm installed

## Installation

This plugin is installed via AIW plugin marketplace:

```bash
aiw plugin install github-mcp@aiw-official
```

You will be prompted to enter your `GITHUB_TOKEN` during installation.

## Getting GitHub Token

1. Visit https://github.com/settings/tokens
2. Click "Generate new token" → "Generate new token (classic)"
3. Select scopes:
   - `repo` (Full control of private repositories)
   - `read:org` (Read org and team membership, read org projects)
4. Click "Generate token"
5. Copy the token (you won't be able to see it again!)

## Usage

Once installed, the GitHub MCP server provides the following tools:

- **create_or_update_file**: Create or update a file in a repository
- **create_pull_request**: Create a pull request
- **find_files**: Find files by name in a repository
- **push_files**: Push files to a repository
- **create_repository**: Create a new repository
- **fork_repository**: Fork a repository
- **push_to_fork**: Push changes to a fork
- **file_content**: Get the contents of a file
- **list_pull_request_files**: List files in a pull request
- **list_repository_issues**: List issues in a repository
- **list_gists**: List GitHub gists for the authenticated user

## Configuration

The plugin automatically configures the MCP server in `~/.aiw/mcp.json`:

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      }
    }
  }
}
```

## Official Source

- **Repository**: https://github.com/modelcontextprotocol/servers
- **Documentation**: https://github.com/modelcontextprotocol/servers/tree/main/src/github

## License

MIT
