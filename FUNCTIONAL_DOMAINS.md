# Agentic-Warden 功能域分析报告

## 📊 项目概况

- **源代码文件**: 116个Rust文件
- **模块数量**: 30+个主要模块
- **CLI命令**: 14个主命令
- **功能域**: 3大核心模块，6个子系统

---

## 🎯 三大核心模块

### **模块 1: 外部AI CLI管理系统**
**职责**: AI CLI工具的生命周期管理

#### 功能组件:
1. **AI CLI进程树追踪** (REQ-001)
   - `src/core/process_tree.rs` - 智能识别AI CLI根进程
   - `src/task_record.rs` - 任务记录管理
   - `src/registry.rs` - 注册表管理
   - 解决传统工具归因问题的核心功能

2. **AI CLI工具检测与状态管理** (REQ-006)
   - `src/cli_manager.rs` - AI CLI工具检测和版本管理
   - 自动检测Claude/Codex/Gemini的安装状态
   - Node.js版本管理和跨平台安装

3. **供应商管理** (REQ-002)
   - `src/provider/` - 多供应商配置管理
   - 统一API提供商配置与环境变量注入
   - 动态Provider选择

4. **AI CLI角色系统** (REQ-014)
   - `src/roles/` - AI角色配置管理
   - Markdown-based角色定义
   - 安全约束和路径遍历保护
   - `aiw roles list` - 角色列表工具

5. **Wait模式与跨进程协调** (REQ-005)
   - `src/wait_mode.rs` - 并发AI CLI任务协调
   - `src/pwait_mode.rs` - 指定进程等待
   - 跨进程任务同步

6. **更新/安装管理** (REQ-011)
   - `src/cli_manager.rs` - 工具更新逻辑
   - `aiw update` - 统一更新命令
   - 支持NPM/系统包管理器

**CLI命令**:
- `aiw status` - 查看任务状态
- `aiw update [tool]` - 更新工具
- `aiw wait` - 等待任务完成
- `aiw pwait <pid>` - 等待指定进程
- `aiw roles list` - 列出角色

**源文件数量**: ~25个文件

---

### **模块 2: CC会话管理系统** (Claude Code集成)
**职责**: 会话历史管理和智能检索

#### 功能组件:
1. **Claude Code Hooks集成** (REQ-010)
   - `src/hooks/` - Hook事件处理
     - `config.rs` - Hook安装/卸载
     - `handler.rs` - Hook事件处理器
     - `parser.rs` - 会话解析器
     - `input.rs` - Hook输入处理
   - 支持SessionEnd、PreCompact等Hook事件
   - 自动从stdin读取并解析会话

2. **会话历史存储与向量索引**
   - `src/memory/` - 双模式内存系统
     - `history.rs` - 会话历史存储
     - `config.rs` - 内存配置管理
   - 基于SahomeDB的持久化存储
   - FastEmbed向量嵌入生成

3. **语义搜索与TODO提取**
   - 语义搜索会话历史
   - 自动提取TODO项
   - 向量相似度匹配

4. **Google Drive同步** (REQ-003)
   - `src/sync/` - Google Drive同步
   - OAuth 2.0设备流认证
   - 选择性配置备份
   - `aiw push/pull/list` - 云同步命令

**CLI命令**:
- `aiw hooks handle` - 处理Hook事件
- `aiw push [dirs]` - 推送到云端
- `aiw pull` - 从云端拉取
- `aiw list` - 列出远程文件

**源文件数量**: ~15个文件

**您提到的功能组**: `CC hooks安装 + 历史会话查询`
- ✅ Hook安装: `src/hooks/config.rs` → `install_hooks()`
- ✅ 会话查询: `src/memory/history.rs` → 搜索API
- ⚠️ 问题: 这两个功能分散在不同模块，缺乏统一的用户界面

---

### **模块 3: MCP代理路由系统**
**职责**: 智能MCP工具管理和动态编排

#### 功能组件:
1. **MCP服务器核心** (REQ-007)
   - `src/mcp/` - MCP协议实现
   - JSON-RPC 2.0协议支持
   - stdio传输
   - 100% Claude Code配置兼容性

2. **智能MCP路由** (REQ-012)
   - `src/mcp_routing/` - 智能路由引擎
     - `mod.rs` - 主路由逻辑
     - `decision.rs` - LLM决策引擎
     - `registry.rs` - 动态工具注册表
   - 98% token减少优化
   - LLM-first + vector搜索fallback
   - Ollama集成进行路由决策
   - 双模式向量数据库

3. **动态JS编排** (REQ-013)
   - `src/mcp_routing/js_orchestrator/` - 完整子系统
     - `engine.rs` - Boa JS引擎沙箱
     - `workflow_planner.rs` - LLM驱动的工作流规划
     - `injector.rs` - MCP函数注入
     - `validator.rs` - 代码验证
     - `schema_*.rs` - Schema验证和修正
   - Boa JS引擎集成（安全沙箱）
   - 内存限制（256MB）
   - 执行时间限制（30分钟）
   - 危险全局变量禁用
   - LLM驱动的代码生成
   - MCP函数统一API

4. **MCP管理CLI**
   - `aiw mcp list` - 列出MCP服务器
   - `aiw mcp add` - 添加服务器
   - `aiw mcp remove` - 移除服务器
   - `aiw mcp enable/disable` - 启用/禁用
   - `aiw mcp get/edit` - 查看/编辑配置
   - 配置文件热重载

5. **配置观察者与热重载**
   - `src/mcp_routing/config_watcher.rs`
   - 自动检测配置变更
   - 服务生命周期管理
   - 无需重启应用

**CLI命令**:
- `aiw mcp <action>` - 完整MCP管理

**源文件数量**: ~25个文件

---

## 🔧 辅助/基础设施系统

### 1. **核心基础设施**
- `src/error.rs` - 统一错误处理（33KB）
- `src/config.rs` - 配置管理
- `src/logging.rs` - 日志系统
- `src/platform/` - 跨平台支持
- `src/signal.rs` - 信号处理

### 2. **存储系统**
- `src/storage/` - 共享内存存储
  - `src/unified_registry.rs` - 统一注册表
  - `src/registry_factory.rs` - 注册表工厂

### 3. **TUI界面**
- `src/tui/` - 终端用户界面
  - Dashboard（默认界面）
  - Provider管理界面
  - Push/Pull界面
  - 任务状态监控

### 4. **进程管理**
- `src/supervisor/` - 进程监控
- `src/sync/` - 同步协调

**源文件数量**: ~40个文件

---

## 📋 CLI命令完整列表

### AI CLI管理命令
| 命令 | 描述 | 所属模块 |
|------|------|---------|
| `aiw <agent> [task]` | 启动AI CLI执行任务 | 模块1 |
| `aiw status [--tui]` | 查看任务状态 | 模块1 |
| `aiw wait` | 等待任务完成 | 模块1 |
| `aiw pwait <pid>` | 等待指定进程 | 模块1 |
| `aiw update [tool]` | 更新AI CLI工具 | 模块1 |
| `aiw provider` | 管理供应商 | 模块1 |
| `aiw roles list` | 列出AI角色 | 模块1 |

### CC会话管理命令
| 命令 | 描述 | 所属模块 |
|------|------|---------|
| `aiw hooks handle` | 处理Claude Code Hook事件 | 模块2 |
| `aiw push [dirs]` | 推送到Google Drive | 模块2 |
| `aiw pull` | 从Google Drive拉取 | 模块2 |
| `aiw list` | 列出云文件 | 模块2 |

### MCP管理命令
| 命令 | 描述 | 所属模块 |
|------|------|---------|
| `aiw mcp list` | 列出MCP服务器 | 模块3 |
| `aiw mcp add <name> <cmd>` | 添加MCP服务器 | 模块3 |
| `aiw mcp remove <name>` | 移除MCP服务器 | 模块3 |
| `aiw mcp enable <name>` | 启用服务器 | 模块3 |
| `aiw mcp disable <name>` | 禁用服务器 | 模块3 |
| `aiw mcp get <name>` | 查看服务器详情 | 模块3 |
| `aiw mcp edit` | 编辑配置文件 | 模块3 |

### 其他命令
| 命令 | 描述 | 所属 |
|------|------|------|
| `aiw dashboard` | 启动TUI仪表板 | 全模块 |
| `aiw examples` | 显示使用示例 | 通用 |
| `aiw help [topic]` | 显示帮助信息 | 通用 |

---

## 🔍 重要发现

### ✅ **模块边界清晰**
1. **模块1**专注于AI CLI的启动、管理和监控
2. **模块2**专注于会话持久化和云同步
3. **模块3**专注于MCP协议和智能工具路由

### ⚠️ **功能组碎片化问题**
确实存在一些功能分散的情况：

#### 碎片化案例1: CC会话查询
```
Hook安装:
  - 命令: aiw hooks handle
  - 实现: src/hooks/config.rs::install_hooks()

历史查询:
  - 命令: (无直接CLI命令)
  - 实现: src/memory/history.rs::search()
  - 只能通过MCP工具访问

问题:
  - 没有统一的用户界面
  - 无法在CLI直接查询历史
  - MCP-only访问受限
```

#### 碎片化案例2: 配置管理
```
配置文件分布在:
  - ~/.aiw/provider.json         # 供应商配置
  - ~/.aiw/.mcp.json             # MCP配置
  - ~/.aiw/role/                 # 角色配置目录
  - ~/.aiw/sync_state.json       # 同步状态
  - ~/.aiw/auth.json             # OAuth令牌

问题:
  - 没有统一管理入口
  - 用户需要知道每个文件的用途
  - 缺乏配置验证工具
```

### 💡 **功能复杂度分析**

#### 高度复杂功能:
1. **智能MCP路由** (模块3)
   - 多层决策树
   - LLM + 向量搜索双模式
   - 动态工具注册
   - 98% token优化
   - **复杂度**: ⭐⭐⭐⭐⭐

2. **动态JS编排** (模块3)
   - Boa引擎沙箱
   - LLM代码生成
   - 安全验证多层防护
   - 工作流规划
   - **复杂度**: ⭐⭐⭐⭐⭐

3. **CC Hooks集成** (模块2)
   - 多类型Hook支持
   - 实时会话捕获
   - 向量嵌入生成
   - 语义索引
   - **复杂度**: ⭐⭐⭐⭐

#### 中等复杂功能:
1. **进程树追踪** (模块1) - ⭐⭐⭐⭐
2. **AI CLI管理** (模块1) - ⭐⭐⭐
3. **Google Drive同步** (模块2) - ⭐⭐⭐

---

## 🎯 您提到的功能组分析

### "CC hooks安装 + 历史会话查询"功能组

#### 当前状态:
```
src/hooks/
├── config.rs          # 包含install_hooks() - 安装到~/.claude/hooks/
├── handler.rs         # 处理Hook事件
├── parser.rs          # 解析会话历史
└── input.rs           # 读取stdin输入

src/memory/
├── history.rs         # 会话存储和搜索
└── config.rs          # 内存配置

CLI接口:
✅ aiw hooks handle    # 处理Hook事件
❌ (无直接查询命令)   # 无法直接在CLI查询历史

MCP工具:
✅ search_history      # 通过MCP查询历史
✅ list_conversations  # 列出会话
```

#### 存在的问题:
1. **功能碎片化**
   - Hook安装没有独立命令
   - 历史查询只能通过MCP
   - 缺少统一的用户体验

2. **使用流程不连贯**
   - 安装: 需要手动调用setup
   - 查询: 只能通过Claude Code对话
   - 无法直接在终端查询历史

3. **用户学习成本高**
   - 需要理解Hooks概念
   - 需要理解MCP工具的使用
   - 缺乏"一站式"解决方案

#### 建议改进:
```bash
# 统一的用户界面
aiw cc hooks install              # 安装Hooks到Claude Code
aiw cc hooks uninstall            # 卸载Hooks
aiw cc history search <query>     # 搜索会话历史
aiw cc history list               # 列出所有会话
aiw cc history show <id>          # 显示会话详情
aiw cc history export <id>        # 导出会话

# 辅助功能
aiw cc sync status                # 查看同步状态
aiw cc sync trigger               # 手动触发同步
```

---

## 📊 功能域矩阵

| 功能组 | 技术复杂度 | 用户可见性 | 代码分散度 | 用户反馈 |
|-------|-----------|-----------|-----------|---------|
| AI CLI管理 | 中 | 高 | 低 | ⭐⭐⭐⭐⭐ |
| CC会话管理 | 高 | 低 | 高 | ⭐⭐⭐ |
| MCP路由 | 很高 | 中 | 低 | ⭐⭐⭐⭐ |
| TUI界面 | 中 | 高 | 低 | ⭐⭐⭐⭐ |
| 进程追踪 | 高 | 低 | 中 | ⭐⭐⭐ |

---

## 🚀 优化建议

### 1. **功能域封装**
为每个模块创建统一的命令入口：

```bash
# AI CLI模块
aiw ai list              # 列出AI CLI
aiw ai start <agent>     # 启动AI CLI
aiw ai update            # 更新AI CLI
aiw ai status            # 查看状态

# CC会话模块（您的关注点）
aiw cc hooks install/uninstall  # Hook管理
aiw cc history search/list      # 历史查询
aiw cc sync push/pull/list      # 同步管理

# MCP路由模块
aiw mcp list/add/remove         # 服务器管理
aiw mcp route test              # 路由测试
aiw mcp tools list             # 工具列表
```

### 2. **集中式配置管理**
```bash
aiw config show                 # 显示所有配置
aiw config edit providers       # 编辑供应商配置
aiw config edit mcp            # 编辑MCP配置
aiw config validate            # 验证配置
```

### 3. **用户工作流优化**
```bash
# 设置工作流
aiw setup                      # 交互式设置
aiw cc hooks install           # 自动安装Hooks
aiw cc sync enable             # 启用云同步

# 日常使用
aiw ai start claude "任务"     # 启动AI CLI
aiw cc history search "话题"   # 查询历史
aiw dashboard                  # 图形化界面
```

---

## ✅ 总结

1. **项目功能丰富但不杂乱**，有清晰的三大模块边界
2. **您提到的功能组确实存在碎片化**:
   - Hook安装和历史查询缺乏统一入口
   - 只能通过MCP工具访问历史，CLI支持不足
3. **建议创建`aiw cc`命令组**，统一会话管理体验
4. **技术债务低**，功能相对内聚，主要改进在用户体验层面

**核心优势**: 每个模块都有明确的职责和SPEC需求对应，技术架构清晰。

**改进机会**: 增强用户界面的一致性，特别是CC会话管理功能组。
