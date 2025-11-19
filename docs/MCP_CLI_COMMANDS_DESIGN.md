# MCP Management CLI Commands Design

**Version**: v5.3.0 (Planned)
**Date**: 2025-11-19
**Status**: Design Phase
**Related**: [CHANGELOG v5.2.0](../SPEC/05-CHANGELOG.md#v520---配置路径统一与claude-code兼容性增强-🟢-released-2025-11-19)

## Overview

基于对MCPM和Claude Code MCP管理功能的研究，设计一套完整的MCP服务器管理CLI命令。

### Design Principles

1. **简洁优先** - 命令结构清晰，参数明确
2. **Claude Code兼容** - 100%兼容.mcp.json格式
3. **用户友好** - 提供丰富的输出格式和错误提示
4. **健壮性** - 完整的错误处理和验证

### Reference Research

**MCPM** (MCP Package Manager):
- Commands: search, install, add, remove, enable, disable, list, restart
- Registry: Smithery.ai + GitHub MCP Registry
- Features: Package discovery, version management, health checks

**Claude Code**:
- Commands: add, remove, list, get, add-json, test
- Config: ~/.config/claude-code/mcp.json
- Features: Tool inspection, connection testing, schema validation

**Agentic-Warden Approach**:
- Focus: MCP server lifecycle management (不包含package registry)
- Config: ~/.aiw/.mcp.json (global only)
- Priority: Core commands first, optional features later

---

## Command Structure

### Command Hierarchy

```
aiw mcp
├── list                    # List all MCP servers
├── add <name> <command>    # Add new MCP server
├── remove <name>           # Remove MCP server
├── get <name>              # Get server details
├── edit <name>             # Edit server configuration
├── enable <name>           # Enable server
├── disable <name>          # Disable server
├── restart <name>          # Restart server connection
├── test <name>             # Test server connection
├── health [name]           # Check health status
├── tools <name>            # List server tools
├── validate                # Validate .mcp.json
├── export                  # Export configuration
└── import <file>           # Import configuration
```

---

## Core Commands (Phase 1: Must Have)

### 1. `aiw mcp list`

列出所有配置的MCP服务器及其状态。

**Syntax**:
```bash
aiw mcp list [OPTIONS]
```

**Options**:
- `--enabled` - 仅显示已启用的服务器
- `--disabled` - 仅显示已禁用的服务器
- `--format <json|table|yaml>` - 输出格式(默认: table)
- `--verbose`, `-v` - 显示详细信息(包括命令行、环境变量等)

**Output Example** (table format):
```
MCP SERVERS (3 configured, 2 enabled)

NAME            COMMAND    STATUS      CATEGORY      DESCRIPTION
filesystem      npx        enabled     system        Filesystem operations
git             uvx        enabled     development   Git version control
brave-search    npx        disabled    search        Web search API
```

**Output Example** (json format):
```json
{
  "servers": [
    {
      "name": "filesystem",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user"],
      "enabled": true,
      "category": "system",
      "description": "Filesystem operations (read, write, search files)"
    }
  ],
  "summary": {
    "total": 3,
    "enabled": 2,
    "disabled": 1
  }
}
```

**Implementation**:
- Read `~/.aiw/.mcp.json`
- Parse mcpServers configuration
- Display in requested format
- Sort by name alphabetically

---

### 2. `aiw mcp add`

添加新的MCP服务器到配置文件。

**Syntax**:
```bash
aiw mcp add <name> <command> [args...] [OPTIONS]
```

**Arguments**:
- `<name>` - MCP服务器名称(唯一标识符)
- `<command>` - 可执行命令(npx, uvx, node, python等)
- `[args...]` - 命令参数(可选，多个参数用空格分隔)

**Options**:
- `--description <text>` - 服务器描述
- `--category <category>` - 服务器分类(system, development, search等)
- `--enabled <true|false>` - 启用状态(默认: true)
- `--env <KEY=VALUE>` - 环境变量(可多次使用)
- `--health-check` - 启用健康检查(默认: false)
- `--health-interval <seconds>` - 健康检查间隔(默认: 60)
- `--health-timeout <seconds>` - 健康检查超时(默认: 10)

**Examples**:
```bash
# 基本添加
aiw mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /home/user

# 带完整配置
aiw mcp add git uvx mcp-server-git --repository /home/user/project \
  --description "Git version control operations" \
  --category development \
  --health-check \
  --health-interval 60

# 带环境变量
aiw mcp add brave-search npx -y @modelcontextprotocol/server-brave-search \
  --env BRAVE_API_KEY=your-key-here \
  --category search \
  --enabled false
```

**Validation**:
- ✅ Name uniqueness check
- ✅ Command executable validation
- ✅ JSON schema validation
- ✅ Category validation (warn if non-standard)
- ✅ Env var format validation (KEY=VALUE)

**Behavior**:
1. Check if `~/.aiw/.mcp.json` exists, create if not
2. Validate name doesn't exist
3. Build McpServerConfig object
4. Insert into mcpServers
5. Write back to file with pretty formatting
6. Success message with next steps

**Output Example**:
```
✅ MCP server 'filesystem' added successfully

Configuration:
  Command: npx -y @modelcontextprotocol/server-filesystem /home/user
  Category: system
  Status: enabled

Next steps:
  • Test the connection: aiw mcp test filesystem
  • List available tools: aiw mcp tools filesystem
  • Restart AI CLI to apply changes
```

---

### 3. `aiw mcp remove`

从配置中移除MCP服务器。

**Syntax**:
```bash
aiw mcp remove <name> [OPTIONS]
```

**Arguments**:
- `<name>` - MCP服务器名称

**Options**:
- `--yes`, `-y` - 跳过确认提示

**Behavior**:
1. Check if server exists
2. Show server details
3. Prompt for confirmation (unless --yes)
4. Remove from mcpServers
5. Write back to file
6. Success message

**Output Example**:
```
⚠️  You are about to remove MCP server 'filesystem':

  Command: npx -y @modelcontextprotocol/server-filesystem /home/user
  Category: system
  Status: enabled

Are you sure? (y/N): y

✅ MCP server 'filesystem' removed successfully
```

---

### 4. `aiw mcp get`

获取MCP服务器的详细配置信息。

**Syntax**:
```bash
aiw mcp get <name> [OPTIONS]
```

**Arguments**:
- `<name>` - MCP服务器名称

**Options**:
- `--format <json|yaml|table>` - 输出格式(默认: yaml)

**Output Example** (yaml format):
```yaml
name: filesystem
command: npx
args:
  - "-y"
  - "@modelcontextprotocol/server-filesystem"
  - "/home/user"
description: Filesystem operations (read, write, search files)
category: system
enabled: true
env: {}
healthCheck:
  enabled: true
  interval: 60
  timeout: 10
```

---

### 5. `aiw mcp edit`

在默认编辑器中编辑MCP服务器配置。

**Syntax**:
```bash
aiw mcp edit <name>
```

**Arguments**:
- `<name>` - MCP服务器名称(特殊值 "all" 表示编辑整个.mcp.json)

**Behavior**:
1. Check if server exists (or "all")
2. Extract server config to temp YAML file
3. Open in $EDITOR (fallback: vim, then nano)
4. Validate edited content
5. Merge back to .mcp.json
6. Success or error message

**Implementation Notes**:
- Use temp file for editing
- Validate JSON schema after edit
- Rollback on validation failure
- Pretty-print JSON output

---

## Status Control Commands (Phase 1)

### 6. `aiw mcp enable`

启用已禁用的MCP服务器。

**Syntax**:
```bash
aiw mcp enable <name>
```

**Behavior**:
1. Set `enabled: true`
2. Write to .mcp.json
3. Suggest restart

**Output**:
```
✅ MCP server 'brave-search' enabled

Note: Restart your AI CLI to apply changes
```

---

### 7. `aiw mcp disable`

禁用MCP服务器(不删除配置)。

**Syntax**:
```bash
aiw mcp disable <name>
```

**Behavior**:
1. Set `enabled: false`
2. Write to .mcp.json
3. Suggest restart

**Output**:
```
✅ MCP server 'brave-search' disabled

The server configuration is preserved but will not be loaded.
To re-enable: aiw mcp enable brave-search
```

---

### 8. `aiw mcp restart`

重启MCP服务器连接(如果正在运行)。

**Syntax**:
```bash
aiw mcp restart <name>
```

**Behavior**:
1. Check if IntelligentRouter is running
2. Attempt to reconnect to server
3. Report status

**Note**: 此命令需要访问运行中的agentic-warden实例，实现复杂度较高。
**Alternative**: 建议用户重启AI CLI来应用配置更改。

**Simplified Implementation** (Phase 1):
- 输出提示信息，建议重启AI CLI
- 不实际操作运行中的进程

---

## Health & Diagnostics (Phase 2: Should Have)

### 9. `aiw mcp test`

测试MCP服务器连接和响应。

**Syntax**:
```bash
aiw mcp test <name> [OPTIONS]
```

**Options**:
- `--timeout <seconds>` - 超时时间(默认: 10)
- `--verbose`, `-v` - 显示详细测试信息

**Test Steps**:
1. ✅ Configuration validation
2. ✅ Command executable check
3. ✅ Process spawn test
4. ✅ MCP initialize handshake
5. ✅ List tools request
6. ✅ Response time measurement

**Output Example**:
```
Testing MCP server 'filesystem'...

✅ Configuration valid
✅ Command 'npx' found in PATH
✅ Process spawned (PID: 12345)
✅ MCP handshake successful
✅ Tools list received (5 tools)
⏱️  Response time: 234ms

Status: HEALTHY
```

---

### 10. `aiw mcp health`

检查MCP服务器健康状态。

**Syntax**:
```bash
aiw mcp health [name] [OPTIONS]
```

**Arguments**:
- `[name]` - 服务器名称(省略则检查所有)

**Options**:
- `--format <json|table>` - 输出格式

**Output Example**:
```
MCP SERVER HEALTH STATUS

NAME            STATUS      RESPONSE TIME    TOOLS    LAST CHECK
filesystem      healthy     234ms            5        2025-11-19 10:30:15
git             healthy     156ms            8        2025-11-19 10:30:16
brave-search    disabled    -                -        -

Overall: 2/2 enabled servers healthy
```

---

### 11. `aiw mcp tools`

列出MCP服务器提供的所有工具。

**Syntax**:
```bash
aiw mcp tools <name> [OPTIONS]
```

**Arguments**:
- `<name>` - MCP服务器名称

**Options**:
- `--format <json|table|list>` - 输出格式
- `--verbose`, `-v` - 显示工具的详细schema

**Output Example**:
```
MCP TOOLS: filesystem (5 tools)

NAME                DESCRIPTION
read_file           Read content of a file
write_file          Write content to a file
list_directory      List files in directory
search_files        Search for files matching pattern
delete_file         Delete a file

Use --verbose to see detailed schemas
```

---

## Configuration Management (Phase 2)

### 12. `aiw mcp validate`

验证.mcp.json配置文件的正确性。

**Syntax**:
```bash
aiw mcp validate [OPTIONS]
```

**Options**:
- `--strict` - 严格模式(检查command可执行性、env var格式等)
- `--fix` - 自动修复常见问题(如格式化、添加默认值)

**Validation Checks**:
1. ✅ JSON syntax validity
2. ✅ Schema compliance (mcpServers structure)
3. ✅ Name uniqueness
4. ✅ Required fields (command)
5. ✅ Field types (enabled: boolean, etc.)
6. ⚠️ Command executability (--strict)
7. ⚠️ Env var format (--strict)
8. ⚠️ Category standardization

**Output Example**:
```
Validating ~/.aiw/.mcp.json...

✅ JSON syntax valid
✅ Schema compliance passed
✅ 3 servers configured
✅ All names unique
✅ Required fields present

⚠️  Warnings:
  • Server 'filesystem': Command 'npx' not found in PATH
  • Server 'custom-tool': Non-standard category 'custom'

Overall: VALID (2 warnings)
```

---

### 13. `aiw mcp export`

导出MCP配置到文件。

**Syntax**:
```bash
aiw mcp export [OPTIONS]
```

**Options**:
- `--output <file>`, `-o` - 输出文件路径(默认: ./mcp-config-export.json)
- `--format <json|yaml>` - 导出格式(默认: json)
- `--mask-secrets` - 脱敏敏感信息(API keys等)

**Output**:
```
✅ MCP configuration exported to: mcp-config-export.json

Exported:
  • 3 MCP servers
  • Secrets masked: true
```

---

### 14. `aiw mcp import`

从文件导入MCP配置。

**Syntax**:
```bash
aiw mcp import <file> [OPTIONS]
```

**Arguments**:
- `<file>` - 配置文件路径

**Options**:
- `--merge` - 合并模式(保留现有服务器，添加新的)
- `--overwrite` - 覆盖模式(完全替换现有配置)
- `--dry-run` - 预览导入结果，不实际写入

**Behavior**:
1. Parse import file
2. Validate configuration
3. Check for conflicts (name duplicates)
4. Apply merge/overwrite strategy
5. Write to .mcp.json
6. Show summary

**Output Example**:
```
Importing MCP configuration from: backup.json

✅ Configuration valid
⚠️  Conflict detected: Server 'filesystem' already exists

Strategy: --merge
  • Keep existing: filesystem, git
  • Add new: brave-search

Apply changes? (y/N): y

✅ Import successful
  • Preserved: 2 servers
  • Added: 1 server
  • Total: 3 servers
```

---

## Advanced Features (Phase 3: Nice to Have)

### Package Registry Integration

**Commands** (optional, dependent on registry strategy):
```bash
aiw mcp search <query>      # Search MCP packages
aiw mcp install <package>   # Install from registry
aiw mcp upgrade [name]      # Upgrade server package
aiw mcp info <package>      # Show package information
```

**Registry Strategy Options**:

1. **Smithery.ai Integration** (Recommended):
   - API: `GET https://smithery.ai/api/servers?q=<query>`
   - Features: Semantic search, usage metrics, verified packages
   - Implementation: HTTP client with caching

2. **GitHub MCP Registry**:
   - Source: https://github.com/modelcontextprotocol/servers
   - Features: Official packages, version tracking
   - Implementation: GitHub API or git clone

3. **Hybrid Approach**:
   - Primary: Smithery.ai for search
   - Fallback: GitHub Registry for official packages
   - Local cache for offline support

**Decision**: Defer to Phase 3, start with core commands

---

## Implementation Plan

### Phase 1: Core Commands (v5.3.0)
**Priority**: Must Have
**Timeline**: 1-2 weeks

Commands to implement:
- [x] Design specification (this document)
- [ ] `aiw mcp list`
- [ ] `aiw mcp add`
- [ ] `aiw mcp remove`
- [ ] `aiw mcp get`
- [ ] `aiw mcp enable`
- [ ] `aiw mcp disable`
- [ ] `aiw mcp validate`

Implementation tasks:
1. Create `src/commands/mcp.rs` module
2. Implement McpConfigEditor utility
3. Add command parsing in `src/commands/parser.rs`
4. Write unit tests for each command
5. Write integration tests
6. Update README with examples

---

### Phase 2: Diagnostics & Management (v5.4.0)
**Priority**: Should Have
**Timeline**: 1 week

Commands to implement:
- [ ] `aiw mcp test`
- [ ] `aiw mcp health`
- [ ] `aiw mcp tools`
- [ ] `aiw mcp export`
- [ ] `aiw mcp import`
- [ ] `aiw mcp edit`

Implementation tasks:
1. Implement MCP client for testing
2. Add health check protocol
3. Implement YAML serialization
4. Add $EDITOR integration
5. Write comprehensive tests

---

### Phase 3: Advanced Features (v6.0.0)
**Priority**: Nice to Have
**Timeline**: 2-3 weeks

Commands to implement:
- [ ] `aiw mcp search`
- [ ] `aiw mcp install`
- [ ] `aiw mcp upgrade`
- [ ] `aiw mcp info`
- [ ] `aiw mcp restart` (real implementation)

Implementation tasks:
1. Research registry APIs
2. Implement HTTP client with caching
3. Add package version management
4. Implement hot-reload for running instances
5. Comprehensive integration testing

---

## Error Handling Strategy

### Common Errors

1. **Configuration File Not Found**:
   ```
   ❌ Error: MCP configuration not found

   Expected location: ~/.aiw/.mcp.json

   To create a new configuration:
     aiw mcp add <name> <command> [args...]
   ```

2. **Invalid JSON**:
   ```
   ❌ Error: Invalid JSON in ~/.aiw/.mcp.json

   Line 5: Unexpected token '}' at position 123

   To fix:
     aiw mcp validate --fix
   ```

3. **Server Not Found**:
   ```
   ❌ Error: MCP server 'unknown' not found

   Available servers:
     • filesystem
     • git
     • brave-search

   Use 'aiw mcp list' to see all servers
   ```

4. **Name Conflict**:
   ```
   ❌ Error: MCP server 'filesystem' already exists

   To update existing server:
     aiw mcp edit filesystem

   To replace:
     aiw mcp remove filesystem
     aiw mcp add filesystem <command> [args...]
   ```

5. **Command Not Executable**:
   ```
   ⚠️  Warning: Command 'npx' not found in PATH

   The server is configured but may not work.

   To install Node.js and npm:
     https://nodejs.org/

   To test the server:
     aiw mcp test filesystem
   ```

---

## Testing Strategy

### Unit Tests
- Command parsing and validation
- JSON serialization/deserialization
- Config manipulation (add, remove, edit)
- Error handling for edge cases

### Integration Tests
- End-to-end command execution
- File system operations
- MCP server communication
- Cross-command workflows

### Test Coverage Goals
- Core commands: 90%+
- Utilities: 80%+
- Error handling: 100%

---

## Dependencies

### Required Crates
```toml
serde = "1.0"              # JSON serialization
serde_json = "1.0"         # JSON parsing
serde_yaml = "0.9"         # YAML support
colored = "2.0"            # Terminal colors
dialoguer = "0.11"         # Interactive prompts
prettytable-rs = "0.10"    # Table formatting
tempfile = "3.8"           # Temp file handling
which = "5.0"              # Command lookup
```

### Optional (Phase 2+)
```toml
reqwest = "0.11"           # HTTP client (for registry)
tokio = "1.35"             # Async runtime
clap_complete = "4.4"      # Shell completions
```

---

## User Experience Considerations

### Output Formatting
- **Colors**: Use colored output for better readability
  - ✅ Green for success
  - ❌ Red for errors
  - ⚠️ Yellow for warnings
  - 🔵 Blue for info
- **Tables**: Use prettytable for structured output
- **JSON/YAML**: Pretty-print with indentation

### Interactive Prompts
- Confirmation prompts for destructive operations
- Use dialoguer for user-friendly prompts
- Support --yes flag to skip prompts in CI/scripts

### Progress Indicators
- Show spinners for long-running operations
- Display progress bars for multi-step processes
- Provide clear status messages

### Help Messages
- Comprehensive --help for each command
- Examples in help text
- Error messages with actionable suggestions

---

## Backward Compatibility

### Configuration Format
- 100% compatible with Claude Code .mcp.json
- Support optional fields gracefully
- Default values for missing fields
- Preserve unknown fields during edit

### Migration Path
- No breaking changes to existing configs
- Auto-upgrade on first use (if needed)
- Backup before major changes

---

## Security Considerations

### Sensitive Data
- Mask API keys in list/get output
- Option to unmask with --show-secrets flag
- Warn when secrets are in command args
- Support .env file integration

### File Permissions
- Check ~/.aiw/ directory permissions
- Warn if .mcp.json is world-readable
- Set restrictive permissions on creation

### Command Validation
- Validate command executable paths
- Warn about shell injection risks
- Quote arguments properly

---

## Documentation Requirements

### README Updates
- Add "MCP Management" section
- Document all commands with examples
- Troubleshooting guide

### Man Pages
- Generate man pages for commands
- Install with package

### Online Documentation
- Tutorial: Getting started with MCP
- Reference: Complete command list
- FAQ: Common issues and solutions

---

## Success Metrics

### Phase 1 Goals
- [ ] All core commands implemented
- [ ] 90%+ test coverage
- [ ] Zero regression bugs
- [ ] Positive user feedback

### User Experience Goals
- Commands are intuitive and discoverable
- Error messages are helpful
- Documentation is comprehensive
- Performance is acceptable (< 1s for most commands)

---

## Future Enhancements

Ideas for post-v6.0.0:

1. **MCP Templates**:
   ```bash
   aiw mcp add-template filesystem /path
   ```
   - Pre-configured templates for common servers

2. **Batch Operations**:
   ```bash
   aiw mcp enable filesystem git
   aiw mcp disable --category search
   ```

3. **Configuration Profiles**:
   ```bash
   aiw mcp profile save work
   aiw mcp profile load personal
   ```

4. **Dependency Management**:
   - Auto-install npm/pip packages
   - Version pinning
   - Dependency resolution

5. **GUI/TUI Interface**:
   - Interactive MCP management UI
   - Real-time health monitoring
   - Tool browser

---

## Conclusion

This design provides a comprehensive MCP management CLI that:
- ✅ Aligns with Claude Code compatibility
- ✅ Matches industry best practices (MCPM, Claude Code)
- ✅ Provides intuitive user experience
- ✅ Supports phased implementation
- ✅ Maintains backward compatibility
- ✅ Enables future extensibility

**Next Steps**:
1. Review and approve this design
2. Begin Phase 1 implementation
3. Create tracking issues for each command
4. Set up CI/CD for automated testing

---

**Document Status**: Draft for Review
**Last Updated**: 2025-11-19
**Authors**: Claude (AI Assistant)
**Reviewers**: [To be assigned]
