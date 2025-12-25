# Filesystem MCP Server

Filesystem access server for Model Context Protocol.

## Features

- Read files
- Write files
- Create directories
- List directory contents
- Search files
- File system metadata

## Prerequisites

- Node.js and npm installed

## Installation

```bash
aiw plugin install filesystem-mcp@aiw-official
```

Optionally set a custom base path:

```bash
export AIW_FS_PATH=/path/to/directory
aiw plugin install filesystem-mcp@aiw-official
```

## Usage

The filesystem MCP server provides secure access to a specified directory tree.

### Available Tools

- **read_file**: Read the complete contents of a file
- **read_multiple_files**: Read multiple files
- **write_file**: Write to a file, creating directories if needed
- **create_directory**: Create a new directory
- **list_directory**: List contents of a directory
- **directory_tree**: Get a recursive tree view of a directory
- **search_files**: Search for files and directories by name

### Security

The server is restricted to the base path specified during installation (default: `$HOME`). All file operations are confined to this directory tree.

## Configuration

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "$HOME"
      ]
    }
  }
}
```

## Official Source

- **Repository**: https://github.com/modelcontextprotocol/servers
- **Documentation**: https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem

## License

MIT
