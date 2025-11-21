# MCP CLI 简化设计

**Version**: v5.3.0
**Date**: 2025-11-19
**Principle**: 简单实用，只管理 `~/.aiw/.mcp.json`

## 设计原则

1. **单一配置级别** - 只操作 `~/.aiw/.mcp.json`，不区分机器/用户/项目
2. **参考Claude Code** - 命令风格与Claude Code保持一致
3. **简单直接** - 不做复杂的分阶段、包注册表等功能
4. **即学即用** - 每个命令功能单一明确

---

## 命令列表

### 基础命令（必需）

```bash
aiw mcp list                                    # 列出所有MCP服务器
aiw mcp add <name> <command> [args...]          # 添加MCP服务器
aiw mcp remove <name>                           # 移除MCP服务器
aiw mcp get <name>                              # 查看服务器详细配置
aiw mcp enable <name>                           # 启用服务器
aiw mcp disable <name>                          # 禁用服务器
aiw mcp edit                                    # 用编辑器打开.mcp.json
```

就这7个命令，不再扩展。

---

## 命令详细设计

### 1. `aiw mcp list`

列出所有配置的MCP服务器。

**语法**:
```bash
aiw mcp list
```

**输出示例**:
```
MCP Servers (~/.aiw/.mcp.json)

NAME            COMMAND         STATUS      DESCRIPTION
filesystem      npx             enabled     Filesystem operations
git             uvx             enabled     Git version control
brave-search    npx             disabled    Web search API

Total: 3 servers (2 enabled, 1 disabled)
```

**实现要点**:
- 读取 `~/.aiw/.mcp.json`
- 解析 mcpServers
- 表格形式输出
- 显示 name, command, enabled 状态, description

---

### 2. `aiw mcp add`

添加新的MCP服务器。

**语法**:
```bash
aiw mcp add <name> <command> [args...] [options]
```

**选项**:
```
--description <text>    服务器描述
--category <category>   分类（可选）
--env KEY=VALUE         环境变量（可多次使用）
--disabled              添加但不启用（默认启用）
```

**示例**:
```bash
# 基本用法
aiw mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /home/user

# 带描述和分类
aiw mcp add git uvx mcp-server-git --repository /home/user/project \
  --description "Git version control" \
  --category development

# 带环境变量
aiw mcp add brave-search npx -y @modelcontextprotocol/server-brave-search \
  --env BRAVE_API_KEY=your-key \
  --description "Web search" \
  --disabled
```

**输出**:
```
✅ Added MCP server 'filesystem'

  Command: npx -y @modelcontextprotocol/server-filesystem /home/user
  Status: enabled

Configuration saved to ~/.aiw/.mcp.json
Restart your AI CLI to apply changes.
```

**实现要点**:
- 检查name是否已存在
- 构建McpServerConfig对象
- 写入 `~/.aiw/.mcp.json`
- Pretty-print JSON格式

---

### 3. `aiw mcp remove`

移除MCP服务器。

**语法**:
```bash
aiw mcp remove <name> [-y]
```

**选项**:
```
-y, --yes    跳过确认提示
```

**示例**:
```bash
aiw mcp remove filesystem
```

**输出**:
```
Remove MCP server 'filesystem'?
  Command: npx -y @modelcontextprotocol/server-filesystem /home/user
  Status: enabled

Continue? (y/N): y

✅ Removed MCP server 'filesystem'

Configuration saved to ~/.aiw/.mcp.json
```

**实现要点**:
- 检查服务器是否存在
- 显示确认提示（除非 -y）
- 从mcpServers中删除
- 保存回文件

---

### 4. `aiw mcp get`

查看MCP服务器的详细配置。

**语法**:
```bash
aiw mcp get <name>
```

**输出示例**:
```yaml
name: filesystem
command: npx
args:
  - "-y"
  - "@modelcontextprotocol/server-filesystem"
  - "/home/user"
description: Filesystem operations
category: system
enabled: true
env: {}
```

**实现要点**:
- 读取服务器配置
- YAML格式输出（更易读）
- 如果服务器不存在，提示错误

---

### 5. `aiw mcp enable`

启用已禁用的MCP服务器。

**语法**:
```bash
aiw mcp enable <name>
```

**输出**:
```
✅ Enabled MCP server 'brave-search'

Restart your AI CLI to apply changes.
```

**实现要点**:
- 设置 `enabled: true`
- 保存到文件
- 提示重启AI CLI

---

### 6. `aiw mcp disable`

禁用MCP服务器（保留配置）。

**语法**:
```bash
aiw mcp disable <name>
```

**输出**:
```
✅ Disabled MCP server 'brave-search'

The server configuration is preserved.
To re-enable: aiw mcp enable brave-search
```

**实现要点**:
- 设置 `enabled: false`
- 保存到文件
- 说明配置保留

---

### 7. `aiw mcp edit`

直接在编辑器中编辑 `.mcp.json` 文件。

**语法**:
```bash
aiw mcp edit
```

**行为**:
- 使用 `$EDITOR` 环境变量指定的编辑器
- Fallback: vim → nano → 提示错误
- 编辑后验证JSON格式
- 格式错误则不保存，显示错误位置

**输出**:
```
Opening ~/.aiw/.mcp.json in vim...

✅ Configuration saved
   3 servers configured
```

**实现要点**:
- 检测 `$EDITOR` 环境变量
- 调用编辑器打开文件
- 编辑后验证JSON语法
- 如果有错误，提示并回滚

---

## 配置文件格式

只支持一个配置文件：`~/.aiw/.mcp.json`

**示例**:
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user"],
      "description": "Filesystem operations",
      "category": "system",
      "enabled": true
    },
    "git": {
      "command": "uvx",
      "args": ["mcp-server-git", "--repository", "/home/user/project"],
      "description": "Git version control",
      "category": "development",
      "enabled": true
    },
    "brave-search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "description": "Web search",
      "category": "search",
      "enabled": false,
      "env": {
        "BRAVE_API_KEY": "your-key-here"
      }
    }
  }
}
```

**字段说明**:
- `command` (必需) - 可执行命令
- `args` (可选) - 命令参数数组
- `description` (可选) - 服务器描述
- `category` (可选) - 分类标签
- `enabled` (可选) - 是否启用，默认true
- `env` (可选) - 环境变量键值对

---

## 错误处理

### 常见错误提示

**配置文件不存在**:
```
❌ MCP configuration not found: ~/.aiw/.mcp.json

To create your first MCP server:
  aiw mcp add <name> <command> [args...]

Example:
  aiw mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /home/user
```

**服务器已存在**:
```
❌ MCP server 'filesystem' already exists

To update: aiw mcp edit
To remove: aiw mcp remove filesystem
```

**服务器不存在**:
```
❌ MCP server 'unknown' not found

Available servers:
  • filesystem
  • git
  • brave-search

Use 'aiw mcp list' to see all servers
```

**JSON格式错误**:
```
❌ Invalid JSON in ~/.aiw/.mcp.json

  Line 5: Unexpected token '}'

Please fix the JSON syntax or restore from backup.
```

---

## 实现要点

### 代码结构

```
src/commands/
├── mod.rs
├── parser.rs              # 命令解析
└── mcp/
    ├── mod.rs             # MCP命令入口
    ├── list.rs            # list命令实现
    ├── add.rs             # add命令实现
    ├── remove.rs          # remove命令实现
    ├── get.rs             # get命令实现
    ├── enable.rs          # enable命令实现
    ├── disable.rs         # disable命令实现
    ├── edit.rs            # edit命令实现
    └── config_editor.rs   # 配置文件操作工具
```

### 核心工具类

```rust
// src/commands/mcp/config_editor.rs

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

pub struct McpConfigEditor {
    config_path: PathBuf,
}

impl McpConfigEditor {
    pub fn new() -> Result<Self> {
        let config_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot find home directory"))?
            .join(".aiw")
            .join(".mcp.json");
        Ok(Self { config_path })
    }

    pub fn read(&self) -> Result<Value> {
        // 读取并解析.mcp.json
    }

    pub fn write(&self, config: &Value) -> Result<()> {
        // 写入.mcp.json，pretty-print
    }

    pub fn server_exists(&self, name: &str) -> Result<bool> {
        // 检查服务器是否存在
    }

    pub fn add_server(&self, name: &str, config: ServerConfig) -> Result<()> {
        // 添加服务器
    }

    pub fn remove_server(&self, name: &str) -> Result<()> {
        // 移除服务器
    }

    pub fn update_server(&self, name: &str, config: ServerConfig) -> Result<()> {
        // 更新服务器配置
    }

    pub fn list_servers(&self) -> Result<Vec<(String, ServerConfig)>> {
        // 列出所有服务器
    }
}
```

### 依赖项

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"          # 用于get命令的YAML输出
anyhow = "1.0"
dirs = "5.0"
colored = "2.0"             # 终端颜色
prettytable-rs = "0.10"     # 表格输出（list命令）
dialoguer = "0.11"          # 交互提示（remove确认）
```

---

## 实现计划

**时间估算**: 3-5天

### Day 1: 基础设施
- [ ] 创建 `src/commands/mcp/` 模块
- [ ] 实现 `McpConfigEditor` 工具类
- [ ] 编写配置读写测试

### Day 2: 核心命令
- [ ] 实现 `list` 命令
- [ ] 实现 `add` 命令
- [ ] 实现 `remove` 命令
- [ ] 单元测试

### Day 3: 管理命令
- [ ] 实现 `get` 命令
- [ ] 实现 `enable` 命令
- [ ] 实现 `disable` 命令
- [ ] 单元测试

### Day 4: 编辑器集成
- [ ] 实现 `edit` 命令
- [ ] JSON验证和错误提示
- [ ] 集成测试

### Day 5: 完善和测试
- [ ] 错误处理优化
- [ ] 用户体验改进
- [ ] 文档更新
- [ ] 端到端测试

---

## 测试策略

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_server() {
        // 测试添加服务器
    }

    #[test]
    fn test_remove_server() {
        // 测试移除服务器
    }

    #[test]
    fn test_enable_disable() {
        // 测试启用/禁用
    }

    #[test]
    fn test_duplicate_name() {
        // 测试重复名称处理
    }

    #[test]
    fn test_invalid_json() {
        // 测试JSON验证
    }
}
```

### 集成测试

```bash
# 测试完整工作流
aiw mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /home/user
aiw mcp list
aiw mcp get filesystem
aiw mcp disable filesystem
aiw mcp list
aiw mcp enable filesystem
aiw mcp remove filesystem -y
```

---

## 用户文档

### README示例

```markdown
## MCP Server Management

Agentic-Warden provides simple CLI commands to manage your MCP servers in `~/.aiw/.mcp.json`.

### Quick Start

```bash
# Add a filesystem server
aiw mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /home/user \
  --description "Filesystem operations"

# List all servers
aiw mcp list

# Disable a server temporarily
aiw mcp disable filesystem

# Edit configuration directly
aiw mcp edit

# Remove a server
aiw mcp remove filesystem
```

### Available Commands

- `aiw mcp list` - List all MCP servers
- `aiw mcp add <name> <command> [args...]` - Add a new server
- `aiw mcp remove <name>` - Remove a server
- `aiw mcp get <name>` - Show server details
- `aiw mcp enable <name>` - Enable a server
- `aiw mcp disable <name>` - Disable a server
- `aiw mcp edit` - Edit configuration file directly

### Configuration File

All MCP servers are configured in `~/.aiw/.mcp.json`. This file is 100% compatible with Claude Code and other AI tools.
```

---

## 不实现的功能

为了保持简单，以下功能**明确不实现**：

❌ 包注册表搜索和安装 (search, install)
❌ 多配置级别 (机器/用户/项目)
❌ 配置导入导出 (import, export)
❌ 健康检查和测试 (test, health)
❌ 工具列表查询 (tools)
❌ 配置验证 (validate)
❌ 自动重启 (restart)
❌ 配置模板 (templates)
❌ 批量操作 (bulk operations)

用户可以直接使用 `aiw mcp edit` 来做任何高级配置修改。

---

## 总结

这是一个**最小化、实用化**的MCP管理CLI设计：

- ✅ **7个命令** - 覆盖所有基本需求
- ✅ **单一配置** - 只操作 `~/.aiw/.mcp.json`
- ✅ **Claude Code兼容** - 配置格式100%兼容
- ✅ **简单直接** - 无复杂分阶段，无包管理器
- ✅ **快速实现** - 3-5天即可完成

**下一步**: 直接开始实现，无需更多设计。

---

**Document Status**: Final Design
**Last Updated**: 2025-11-19
**Implementation Target**: v5.3.0
**Estimated Effort**: 3-5 days
