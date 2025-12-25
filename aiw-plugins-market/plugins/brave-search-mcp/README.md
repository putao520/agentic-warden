# Brave Search MCP Server

Web search integration server using Brave Search API via Model Context Protocol.

## Features

- Web search capabilities
- News search
- Safe search options
- No tracking or personal data collection

## Prerequisites

- Brave Search API key
- Node.js and npm installed

## Installation

```bash
aiw plugin install brave-search-mcp@aiw-official
```

You will be prompted to enter your `BRAVE_API_KEY` during installation.

## Getting Brave API Key

1. Visit https://api.search.brave.com/app/keys
2. Sign up for a free account
3. Generate your API key
4. Copy the key for installation

## Usage

The Brave Search MCP server provides web search capabilities:

### Available Tools

- **brave_web_search**: Search the web using Brave Search API
  - Supports query string, count offset, and time range parameters
  - Optional text extraction for search results

### Search Parameters

- `query`: Search query string (required)
- `count`: Number of results to return (default: 10, max: 20)
- `offset`: Pagination offset (default: 0)
- `textExtractor`: Optional result summarization (choices: `lucid`, `recursive_summary`)

## Configuration

```json
{
  "mcpServers": {
    "brave-search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "env": {
        "BRAVE_API_KEY": "${BRAVE_API_KEY}"
      }
    }
  }
}
```

## Official Source

- **Repository**: https://github.com/modelcontextprotocol/servers
- **Documentation**: https://github.com/modelcontextprotocol/servers/tree/main/src/brave-search

## License

MIT
