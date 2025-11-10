# MCP 质量升级计划

**日期**: 2025-11-10
**当前状态**: 手动实现的简单stdio处理
**目标状态**: 使用rmcp库的标准MCP服务器

---

## 🔴 当前问题

### 1. 未使用rmcp库
**问题**:
- Cargo.toml已引入 `rmcp = { version = "0.5", features = ["server", "transport-io"] }`
- 但src/mcp.rs完全没有使用，全是手动实现
- 手动实现的stdin/stdout读写和简单JSON-RPC响应

**验证**:
```bash
$ grep -r "use rmcp\|rmcp::" src/mcp.rs
# 无输出 - 完全未使用
```

**当前实现** (src/mcp.rs:45-136):
```rust
pub async fn run_stdio_server(self) -> Result<...> {
    // 手动读取stdin
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut writer = tokio::io::stdout();

    loop {
        reader.read_line(&mut buffer).await?;
        // 手动处理，返回固定JSON
        let response = self.handle_mcp_request(line).await?;
        writer.write_all(response.as_bytes()).await?;
    }
}

async fn handle_mcp_request(&self, request: &str) -> Result<String, ...> {
    // 返回固定的工具列表，不处理实际调用
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": { "message": "...", "tools": [...] }
    });
    Ok(serde_json::to_string(&response)?)
}
```

**问题**:
1. 不是标准的MCP实现
2. 只返回工具列表，无法处理实际工具调用
3. 无JSON-RPC 2.0请求路由
4. 浪费了rmcp依赖

---

## ✅ SPEC要求 (SPEC/MCP-INTEGRATION.md:244-292)

### 使用rmcp库
```rust
use rmcp::prelude::*;

#[derive(Server)]
pub struct AgenticWardenMcpServer {
    provider_manager: Arc<Mutex<ProviderManager>>,
}

impl AgenticWardenMcpServer {
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        self.serve_stdio().await?;
        Ok(())
    }
}

#[tool(description = "Monitor all AI CLI processes")]
pub async fn monitor_processes(
    &self,
    #[arg(description = "Optional filter by AI type")] ai_type: Option<String>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // 工具实现
}
```

**要求**:
- 使用 `#[derive(Server)]` 派生宏
- 使用 `#[tool]` 宏标注工具函数
- 使用 `#[arg]` 宏定义参数
- 使用 `serve_stdio()` 启动服务器
- 返回类型为 `Result<T, E>`，T实现`Serialize`

---

## 📋 升级任务清单

### Phase 1: 研究rmcp API ⏳
- [ ] 查看rmcp 0.5版本文档
- [ ] 确认Server derive宏用法
- [ ] 确认tool宏和arg宏语法
- [ ] 确认serve_stdio()方法

### Phase 2: 重构MCP服务器结构 ⏳
- [ ] 添加 `#[derive(Server)]` 到 `AgenticWardenMcpServer`
- [ ] 更改 `run_stdio_server()` 为使用 `serve_stdio()`
- [ ] 删除手动的stdin/stdout处理代码
- [ ] 删除 `handle_mcp_request()` 方法

### Phase 3: 重构工具函数 ⏳
- [ ] `monitor_processes` - 添加 `#[tool]` 和 `#[arg]` 宏
- [ ] `get_process_tree` - 添加 `#[tool]` 和 `#[arg]` 宏
- [ ] `get_provider_status` - 添加 `#[tool]` 和 `#[arg]` 宏
- [ ] `terminate_process` - 添加 `#[tool]` 和 `#[arg]` 宏
- [ ] `start_concurrent_tasks` - 添加 `#[tool]` 和 `#[arg]` 宏
- [ ] `get_task_command` - 添加 `#[tool]` 和 `#[arg]` 宏

### Phase 4: 测试和验证 ⏳
- [ ] 编译检查
- [ ] MCP协议测试（使用Claude Code）
- [ ] 工具调用测试
- [ ] 错误处理测试

---

## 🎯 预期收益

### 1. 标准化
- ✅ 符合MCP v1.0标准
- ✅ 完整的JSON-RPC 2.0支持
- ✅ 标准的错误处理

### 2. 功能完整
- ✅ 真正的工具调用（当前只返回工具列表）
- ✅ 自动工具发现
- ✅ 参数验证

### 3. 可维护性
- ✅ 使用成熟的库，减少自己维护的代码
- ✅ 更清晰的代码结构
- ✅ 减少bug风险

### 4. 代码量
- ❌ 删除：约150行手动stdio/JSON-RPC处理代码
- ✅ 新增：约50行rmcp宏标注
- 📊 净减少：约100行代码

---

## ⚠️ 注意事项

1. **保持工具函数逻辑不变**
   - 只更改函数签名（添加宏）
   - 保持现有的业务逻辑

2. **保留现有功能**
   - 所有6个工具函数保持可用
   - registry()方法继续使用
   - 进程检测、Provider管理等辅助函数保留

3. **向后兼容**
   - MCP协议不变
   - Claude Code集成不受影响
   - 工具参数保持一致

---

## 🔧 建议的实施方式

由于这是一个较大的重构，建议：

1. **创建新分支**: `feature/mcp-rmcp-upgrade`
2. **增量重构**:
   - 先重构Server结构和run方法
   - 再逐个重构工具函数
   - 每步都编译测试
3. **保留备份**: 保留当前代码作为参考
4. **充分测试**: 使用Claude Code实际测试

---

## 📊 优先级

**P0 - 立即执行**
- 理由：
  1. 当前实现不符合SPEC要求
  2. 功能不完整（无法真正调用工具）
  3. 已经引入了rmcp依赖但未使用

**预计工作量**: 4-6小时
- 研究rmcp API: 1-2h
- 重构代码: 2-3h
- 测试验证: 1h
