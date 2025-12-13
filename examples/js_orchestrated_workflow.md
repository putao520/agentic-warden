# JS Orchestrated Workflow Example

This example walks through the full lifecycle of a dynamically generated JavaScript workflow created by `intelligent_route`.

## 1. User Request
```bash
aiw route "Summarize today's git changes and save REPORT.md"
```

## 2. Route Response (abridged)
```json
{
  "message": "Created orchestrated workflow 'git_daily_summary'. Use this tool to solve your request.",
  "selected_tool": {
    "mcp_server": "orchestrated",
    "tool_name": "git_daily_summary"
  },
  "tool_schema": {
    "type": "object",
    "required": ["repo_path", "since"],
    "properties": {
      "repo_path": {"type": "string", "description": "Repository root"},
      "since": {"type": "string", "description": "ISO timestamp"}
    }
  }
}
```

## 3. Generated JavaScript Workflow
```javascript
async function workflow(input) {
  const status = await mcpGitStatus({ repo: input.repo_path, since: input.since });
  const summary = await mcpWriteReport({
    repo: input.repo_path,
    filename: "REPORT.md",
    block: status.summary,
  });
  return { ok: true, report_path: summary.path };
}
```

### MCP Dependencies
- `mcpGitStatus` → `git::git_status`
- `mcpWriteReport` → `reports::write_report`

## 4. Invoking the Workflow
Once Claude Code refreshes `list_tools`, it can call the workflow directly:
```json
call_tool "git_daily_summary" {
  "repo_path": "/home/user/app",
  "since": "2024-11-10T00:00:00Z"
}
```

The Boa runtime pool injects the MCP functions, runs the sandboxed JS, and returns a structured response:
```json
{
  "ok": true,
  "report_path": "REPORT.md"
}
```
