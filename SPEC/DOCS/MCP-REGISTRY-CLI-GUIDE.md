# MCP Registry CLI Development Guide

## Project Context

**Issue**: #20 - feat(mcp): MCP仓库CLI - 多源聚合搜索与安装 [REQ-016]

**SPEC References**:
- `SPEC/01-REQUIREMENTS.md`: REQ-016 (完整验收标准)
- `SPEC/02-ARCHITECTURE.md`: ARCH-015 (架构设计)
- `SPEC/04-API-DESIGN.md`: API-015 (CLI命令设计)

## Directory Structure

```
src/
├── commands/
│   ├── parser.rs          # CLI命令解析 - 需添加新命令
│   └── mcp/
│       ├── mod.rs           # MCP命令入口 - 需添加新命令
│       ├── config_editor.rs # 配置读写 - 可复用
│       ├── list.rs          # 列表显示参考
│       ├── add.rs           # 添加逻辑参考
│       ├── remove.rs        # 移除逻辑参考
│       └── registry/        # 新建：仓库模块
│           ├── mod.rs
│           ├── source.rs    # RegistrySource trait
│           ├── official.rs  # Official Registry实现
│           ├── smithery.rs  # Smithery实现
│           ├── aggregator.rs# 聚合器
│           ├── types.rs     # 数据类型
│           └── interactive.rs # 交互式安装流程
└── lib.rs                 # 模块导出
```

## Existing Code to Reuse

### 1. McpConfigEditor (`src/commands/mcp/config_editor.rs`)
```rust
// 已有功能：
- McpServerConfig struct (command, args, env, description, category, enabled)
- read() / write() 配置文件
- add_server() / remove_server()
- list_servers() / get_server()

// 需扩展：添加 source 字段记录安装来源
```

### 2. CLI Parser (`src/commands/parser.rs`)
```rust
// 已有 McpAction enum:
pub enum McpAction {
    List,
    Add {...},
    Remove {...},
    Get {...},
    Enable {...},
    Disable {...},
    Edit,
    Serve {...},
}

// 需添加:
Search { query: String, source: Option<String>, limit: Option<usize> },
Install { name: String, source: Option<String>, env_vars: Vec<String>, skip_env: bool },
Info { name: String, source: Option<String> },
Update,
```

### 3. Dependencies (Cargo.toml) - 已存在
```toml
dialoguer = "0.11"    # 交互式CLI
indicatif = "0.17"    # 进度条
reqwest = "0.12"      # HTTP客户端
tokio = "1.0"         # 异步运行时
```

## New Code to Implement

### 1. RegistrySource Trait (`src/commands/mcp/registry/source.rs`)
```rust
#[async_trait]
pub trait RegistrySource: Send + Sync {
    fn source_name(&self) -> &'static str;
    fn source_id(&self) -> &'static str;  // "registry" or "smithery"
    fn priority(&self) -> u8;              // 排序优先级

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<McpServerInfo>>;
    async fn get_server(&self, name: &str) -> Result<Option<McpServerDetail>>;
    async fn get_install_config(&self, name: &str) -> Result<McpServerConfig>;
}
```

### 2. Data Types (`src/commands/mcp/registry/types.rs`)
```rust
pub struct McpServerInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub source: String,        // "registry" | "smithery"
    pub server_type: String,   // "npm" | "uvx" | "docker"
    pub package_name: Option<String>,
}

pub struct McpServerDetail {
    pub info: McpServerInfo,
    pub author: Option<String>,
    pub repository: Option<String>,
    pub required_env: Vec<EnvVarSpec>,
}

pub struct EnvVarSpec {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}
```

### 3. Official Registry (`src/commands/mcp/registry/official.rs`)
```rust
// API Endpoint: registry.modelcontextprotocol.io/v0.1
// GET /servers?search={query}&limit=96
// Response: { servers: [...] }
```

### 4. Smithery Registry (`src/commands/mcp/registry/smithery.rs`)
```rust
// API Endpoint: registry.smithery.ai
// GET /servers?q={query}&page=1&pageSize=10
// Header: Authorization: Bearer <optional_token>
// Response: { servers: [...], totalCount: N }
```

### 5. Aggregator (`src/commands/mcp/registry/aggregator.rs`)
```rust
pub struct RegistryAggregator {
    sources: Vec<Box<dyn RegistrySource>>,
}

impl RegistryAggregator {
    pub fn new() -> Self; // 初始化所有源
    pub async fn search(&self, query: &str, source_filter: Option<&str>) -> Result<Vec<McpServerInfo>>;
    pub async fn get_install_config(&self, qualified_name: &str) -> Result<McpServerConfig>;
}
```

### 6. Interactive Installer (`src/commands/mcp/registry/interactive.rs`)
```rust
use dialoguer::{Select, Input, Confirm};
use indicatif::{ProgressBar, ProgressStyle};

pub async fn interactive_search_and_install(query: &str) -> Result<()>;
pub async fn interactive_env_config(server: &McpServerDetail) -> Result<HashMap<String, String>>;
```

## CLI Command Flow

### Search Flow
```
aiw mcp search filesystem
  ↓
1. 并行查询所有源
2. 合并去重（按名称+源）
3. 按优先级+相关度排序
4. 显示编号列表
5. 用户选择编号
6. 调用install流程
```

### Install Flow
```
aiw mcp install @anthropic/filesystem
  ↓
1. 解析qualified name (source:name 或 @scope/name)
2. 获取服务器详情
3. 检测必需环境变量
4. 交互式配置 (除非 --skip-env)
5. 生成McpServerConfig
6. 调用McpConfigEditor.add_server()
7. 显示成功信息
```

## Environment Variable Format

配置文件中使用 `${ENV_VAR}` 引用格式：
```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-github"],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      }
    }
  }
}
```

## Testing Requirements

1. Unit tests for each registry source
2. Integration test for aggregator
3. Mock HTTP responses using `mockito`
4. Test interactive flow with `--skip-env`

## Success Criteria

- [x] REQ-016 所有验收标准通过
- [x] `cargo test` 全部通过
- [x] `cargo clippy` 无警告
- [x] 无TODO/FIXME/stub函数
